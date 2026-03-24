use crate::certification::{
    CertificationKey, CertificationRecord, CertificationStore, compute_contract_fingerprint,
};
use crate::config::load_contract;
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
pub fn run_certify(options: CertifyOptions) -> Result<CertifyReport, CertifyError> {
    let CertifyOptions {
        load_options,
        profiles,
        emit_progress,
    } = options;

    let loaded = load_contract(load_options)?;
    let selected_profiles =
        resolve_profiles(&loaded.contract, &profiles).map_err(|error| CertifyError::Selection {
            paths: loaded.paths.clone(),
            error: error.into(),
        })?;
    validate_certifiable_profiles(&loaded.contract, &loaded.paths, &selected_profiles)?;

    let worktree = capture_worktree_snapshot(&loaded.paths.repo_root).map_err(|error| {
        CertifyError::GitStatus {
            paths: loaded.paths.clone(),
            error,
        }
    })?;
    if !worktree.is_clean() {
        return Err(CertifyError::DirtyWorktree {
            paths: loaded.paths.clone(),
            dirty_paths: worktree.paths().join(", "),
        });
    }

    let commit =
        resolve_head_commit(&loaded.paths.repo_root).map_err(|error| CertifyError::GitCommit {
            paths: loaded.paths.clone(),
            error,
        })?;
    let contract_fingerprint =
        compute_contract_fingerprint(&loaded).map_err(|error| CertifyError::Fingerprint {
            paths: loaded.paths.clone(),
            error,
        })?;
    let store = CertificationStore::open(&loaded.paths.repo_root).map_err(|error| {
        CertifyError::Storage {
            paths: loaded.paths.clone(),
            error,
        }
    })?;

    let profile_results = execute_profiles(
        &loaded.paths,
        &loaded.contract,
        &store,
        &commit,
        &contract_fingerprint,
        &selected_profiles,
        emit_progress,
    )?;
    let summary = summarize(&profile_results);

    Ok(CertifyReport {
        paths: loaded.paths,
        commit,
        contract_fingerprint,
        profiles: selected_profiles,
        profile_results,
        summary,
    })
}

fn validate_certifiable_profiles(
    contract: &crate::config::Contract,
    paths: &crate::config::LoadPaths,
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
        Err(CertifyError::Selection {
            paths: paths.clone(),
            error: CertifySelectionError::NonCertifiableProfiles(non_certifiable.join(", ")),
        })
    }
}

fn execute_profiles(
    paths: &crate::config::LoadPaths,
    contract: &crate::config::Contract,
    store: &CertificationStore,
    commit: &str,
    contract_fingerprint: &crate::certification::ContractFingerprint,
    selected_profiles: &[String],
    emit_progress: bool,
) -> Result<Vec<CertifyProfileResult>, CertifyError> {
    let mut results = Vec::new();

    for profile in selected_profiles {
        let plan = build_profile_evaluation_plan(contract, profile);
        let mut item_results = Vec::new();

        for item in &plan.items {
            if emit_progress {
                eprintln!(
                    "RUN {} {} [{}]",
                    progress_label(&item.kind),
                    item.name,
                    plan.profile
                );
            }
            item_results.push(map_item_result(run_evaluation_item(&paths.repo_root, item)));
        }

        let outcome = summarize_profile_outcome(&item_results);
        let record_written = if outcome == CertifyProfileOutcome::Certified {
            let record = CertificationRecord {
                key: CertificationKey {
                    commit: commit.to_string(),
                    profile: plan.profile.clone(),
                },
                contract_fingerprint: contract_fingerprint.clone(),
            };
            store
                .write(&record)
                .map_err(|error| CertifyError::Storage {
                    paths: paths.clone(),
                    error,
                })?;
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
