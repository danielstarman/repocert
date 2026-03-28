use std::path::Path;

use crate::certification::{
    CertificationKey, CertificationPayload, CertificationStore, compute_contract_fingerprint,
    sign_payload_with_ssh, verify_payload_with_ssh,
};
use crate::config::{CertificationConfig, CertificationMode, RepoSession};
use crate::contract::{
    EvaluationItemKind, EvaluationItemResult, EvaluationOutcome, build_profile_evaluation_plan,
    progress_label, resolve_profiles, run_evaluation_item,
};
use crate::git::{capture_worktree_snapshot, resolve_head_commit};

use super::error::{CertifyError, CertifySelectionError};
use super::types::{
    CertifyItemKind, CertifyItemOutcome, CertifyItemResult, CertifyOptions, CertifyProfileOutcome,
    CertifyProfileResult, CertifyReport, CertifySummary,
};

/// Certify the current `HEAD` commit for one or more certification-eligible profiles.
pub fn run_certify(
    session: &RepoSession,
    options: CertifyOptions,
) -> Result<CertifyReport, CertifyError> {
    let CertifyOptions {
        profiles,
        signing_key,
        emit_progress,
    } = options;

    let selected_profiles =
        resolve_profiles(&session.contract, &profiles).map_err(CertifySelectionError::from)?;
    validate_certifiable_profiles(&session.contract, &selected_profiles)?;

    let worktree = capture_worktree_snapshot(&session.paths.repo_root)?;
    if !worktree.is_clean() {
        return Err(CertifyError::DirtyWorktree {
            dirty_paths: worktree.paths().join(", "),
        });
    }

    let commit = resolve_head_commit(&session.paths.repo_root)?;
    let contract_fingerprint = compute_contract_fingerprint(session)?;
    let CertificationConfig {
        mode: certification_mode,
    } = session
        .contract
        .certification
        .as_ref()
        .expect("certification-eligible profiles require certification config");
    let signing_key = signing_key.ok_or(CertifyError::MissingSigningKeySelection)?;
    let store = CertificationStore::open(&session.paths.repo_root)?;

    let context = CertifyExecutionContext {
        session,
        store: &store,
        commit: &commit,
        contract_fingerprint: &contract_fingerprint,
        certification_mode,
        signing_key: &signing_key,
        emit_progress,
    };
    let profile_results = execute_profiles(&context, &selected_profiles)?;
    let summary = summarize(&profile_results);

    Ok(CertifyReport {
        commit,
        contract_fingerprint,
        profiles: selected_profiles,
        profile_results,
        summary,
    })
}

fn validate_certifiable_profiles(
    contract: &crate::config::Contract,
    selected_profiles: &[String],
) -> Result<(), CertifyError> {
    let non_certifiable = selected_profiles
        .iter()
        .filter(|profile| {
            !contract
                .profiles
                .get(profile.as_str())
                .expect("selected profile should exist")
                .certify
        })
        .cloned()
        .collect::<Vec<_>>();

    if non_certifiable.is_empty() {
        Ok(())
    } else {
        Err(CertifySelectionError::NonCertifiableProfiles(non_certifiable.join(", ")).into())
    }
}

struct CertifyExecutionContext<'a> {
    session: &'a RepoSession,
    store: &'a CertificationStore,
    commit: &'a str,
    contract_fingerprint: &'a crate::certification::ContractFingerprint,
    certification_mode: &'a CertificationMode,
    signing_key: &'a Path,
    emit_progress: bool,
}

fn execute_profiles(
    context: &CertifyExecutionContext<'_>,
    selected_profiles: &[String],
) -> Result<Vec<CertifyProfileResult>, CertifyError> {
    let mut results = Vec::new();

    for profile in selected_profiles {
        let plan = build_profile_evaluation_plan(&context.session.contract, profile);
        let mut item_results = Vec::new();

        for item in &plan.items {
            if context.emit_progress {
                eprintln!(
                    "RUN {} {} [{}]",
                    progress_label(&item.kind),
                    item.name,
                    plan.profile
                );
            }
            item_results.push(map_item_result(run_evaluation_item(
                &context.session.paths.repo_root,
                item,
            )));
        }

        let outcome = summarize_profile_outcome(&item_results);
        let record_written = if outcome == CertifyProfileOutcome::Certified {
            let payload = CertificationPayload {
                key: CertificationKey {
                    commit: context.commit.to_string(),
                    profile: plan.profile.clone(),
                },
                contract_fingerprint: context.contract_fingerprint.clone(),
            };
            let record = match context.certification_mode {
                CertificationMode::SshSigned { trusted_signer } => {
                    let record =
                        sign_payload_with_ssh(context.signing_key, &payload).map_err(|error| {
                            CertifyError::Signing {
                                signing_key: context.signing_key.to_path_buf(),
                                error,
                            }
                        })?;
                    verify_payload_with_ssh(&record, trusted_signer).map_err(|error| {
                        CertifyError::Signing {
                            signing_key: context.signing_key.to_path_buf(),
                            error,
                        }
                    })?;
                    record
                }
            };
            context
                .store
                .write(&record)
                .map_err(CertifyError::Storage)?;
            true
        } else {
            false
        };

        results.push(CertifyProfileResult {
            profile: plan.profile,
            checks: plan.checks,
            item_results,
            outcome,
            record_written,
        });
    }

    Ok(results)
}

fn map_item_result(result: EvaluationItemResult) -> CertifyItemResult {
    CertifyItemResult {
        name: result.name,
        kind: match result.kind {
            EvaluationItemKind::Check => CertifyItemKind::Check,
            EvaluationItemKind::FixerProbe => CertifyItemKind::FixerProbe,
        },
        outcome: match result.outcome {
            EvaluationOutcome::Pass => CertifyItemOutcome::Pass,
            EvaluationOutcome::Fail => CertifyItemOutcome::Fail,
            EvaluationOutcome::Timeout => CertifyItemOutcome::Timeout,
            EvaluationOutcome::RepairNeeded => CertifyItemOutcome::RepairNeeded,
        },
        exit_code: result.exit_code,
        duration_ms: result.duration_ms,
        message: result.message,
    }
}

fn summarize_profile_outcome(results: &[CertifyItemResult]) -> CertifyProfileOutcome {
    if results
        .iter()
        .any(|result| result.outcome == CertifyItemOutcome::Timeout)
    {
        CertifyProfileOutcome::TimedOut
    } else if results
        .iter()
        .any(|result| result.outcome == CertifyItemOutcome::RepairNeeded)
    {
        CertifyProfileOutcome::RepairNeeded
    } else if results
        .iter()
        .any(|result| result.outcome == CertifyItemOutcome::Fail)
    {
        CertifyProfileOutcome::Failed
    } else {
        CertifyProfileOutcome::Certified
    }
}

fn summarize(results: &[CertifyProfileResult]) -> CertifySummary {
    let mut summary = CertifySummary {
        total_profiles: results.len(),
        certified: 0,
        failed: 0,
        timeout: 0,
        repair_needed: 0,
    };

    for result in results {
        match result.outcome {
            CertifyProfileOutcome::Certified => summary.certified += 1,
            CertifyProfileOutcome::Failed => summary.failed += 1,
            CertifyProfileOutcome::TimedOut => summary.timeout += 1,
            CertifyProfileOutcome::RepairNeeded => summary.repair_needed += 1,
        }
    }

    summary
}
