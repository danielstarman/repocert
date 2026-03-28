use crate::certification::{
    CertificationStore, ProfileCertificationError, ProfileCertificationState,
    compute_contract_fingerprint, inspect_profile_certification,
};
use crate::config::load_contract;
use crate::contract::matches_pattern;
use crate::git::inspect_checkout;
use crate::git::{resolve_commit, resolve_head_commit};

use super::error::{StatusError, StatusSelectionError};
use super::types::{
    ProtectedRefStatus, StatusOptions, StatusProfileResult, StatusProfileState, StatusReport,
    StatusSummary,
};

/// Inspect certification state for a commit and any matching protected refs.
pub fn run_status(options: StatusOptions) -> Result<StatusReport, StatusError> {
    let StatusOptions {
        load_options,
        commit,
        profiles,
        assert_certified,
    } = options;

    let loaded = load_contract(load_options)?;
    let current_ref = if assert_certified && profiles.is_empty() && commit.is_none() {
        inspect_checkout(&loaded.paths.repo_root)
            .map_err(|error| StatusError::GitCheckout {
                paths: loaded.paths.clone(),
                error,
            })?
            .head_ref
    } else {
        None
    };
    let selected_profiles = resolve_status_profiles(
        &loaded.contract,
        &profiles,
        assert_certified,
        current_ref.as_deref(),
    )
    .map_err(|error| StatusError::Selection {
        paths: loaded.paths.clone(),
        error,
    })?;
    let commit = match commit {
        Some(commit) => resolve_commit(&loaded.paths.repo_root, &commit).map_err(|error| {
            StatusError::GitCommit {
                paths: loaded.paths.clone(),
                error,
            }
        })?,
        None => resolve_head_commit(&loaded.paths.repo_root).map_err(|error| {
            StatusError::GitCommit {
                paths: loaded.paths.clone(),
                error,
            }
        })?,
    };
    let contract_fingerprint =
        compute_contract_fingerprint(&loaded).map_err(|error| StatusError::Fingerprint {
            paths: loaded.paths.clone(),
            error,
        })?;
    let store = CertificationStore::open(&loaded.paths.repo_root).map_err(|error| {
        StatusError::Storage {
            paths: loaded.paths.clone(),
            error,
        }
    })?;
    let certification = loaded.contract.certification.as_ref();

    let profile_results = selected_profiles
        .iter()
        .map(|profile| {
            let certification = certification
                .as_ref()
                .expect("certification-eligible profiles require certification config");
            inspect_profile_certification(
                &store,
                &loaded.paths.repo_root,
                &commit,
                profile,
                &contract_fingerprint,
                certification,
            )
            .map(|inspection| StatusProfileResult {
                profile: inspection.profile,
                state: map_profile_state(&inspection.state),
                signer_name: inspection.signer_name,
                other_certified_commits: inspection.other_certified_commits,
                recorded_fingerprint: inspection.recorded_fingerprint,
            })
            .map_err(|error| match error {
                ProfileCertificationError::Storage(error) => StatusError::Storage {
                    paths: loaded.paths.clone(),
                    error,
                },
                ProfileCertificationError::GitCommit(error) => StatusError::GitCommit {
                    paths: loaded.paths.clone(),
                    error,
                },
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let protected_refs = loaded
        .contract
        .protected_refs
        .iter()
        .map(|rule| ProtectedRefStatus {
            pattern: rule.pattern.clone(),
            profile: rule.profile.clone(),
            certified: profile_results
                .iter()
                .find(|result| result.profile == rule.profile)
                .map(|result| result.state == StatusProfileState::Certified)
                .unwrap_or(false),
        })
        .collect();
    let summary = summarize(&profile_results);

    Ok(StatusReport {
        paths: loaded.paths,
        commit,
        contract_fingerprint,
        profiles: selected_profiles,
        profile_results,
        protected_refs,
        assert_certified,
        summary,
    })
}

fn resolve_status_profiles(
    contract: &crate::config::Contract,
    requested_profiles: &[String],
    assert_certified: bool,
    current_ref: Option<&str>,
) -> Result<Vec<String>, StatusSelectionError> {
    let profiles = if requested_profiles.is_empty() {
        if assert_certified {
            inferred_assertion_profiles(contract, current_ref)?
        } else {
            contract
                .profiles
                .values()
                .filter(|profile| profile.certify)
                .map(|profile| profile.name.clone())
                .collect::<Vec<_>>()
        }
    } else {
        let profiles = crate::contract::resolve_profiles(contract, requested_profiles)
            .map_err(StatusSelectionError::from)?;
        profiles
    };

    let non_certifiable = profiles
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
        Ok(profiles)
    } else {
        Err(StatusSelectionError::NonCertifiableProfiles(
            non_certifiable.join(", "),
        ))
    }
}

fn inferred_assertion_profiles(
    contract: &crate::config::Contract,
    current_ref: Option<&str>,
) -> Result<Vec<String>, StatusSelectionError> {
    if let Some(current_ref) = current_ref {
        let mut protected_profiles = Vec::new();
        for rule in &contract.protected_refs {
            if matches_pattern(&rule.pattern, current_ref).unwrap_or(false)
                && !protected_profiles
                    .iter()
                    .any(|profile| profile == &rule.profile)
            {
                protected_profiles.push(rule.profile.clone());
            }
        }
        if !protected_profiles.is_empty() {
            return Ok(protected_profiles);
        }
    }

    if let Some(default_profile) = contract.default_profile.as_ref() {
        return Ok(vec![default_profile.clone()]);
    }

    Err(StatusSelectionError::NoAssertionScope)
}

fn summarize(results: &[StatusProfileResult]) -> StatusSummary {
    let mut summary = StatusSummary {
        total_profiles: results.len(),
        certified: 0,
        untrusted_signer: 0,
        invalid_signature: 0,
        stale_commit: 0,
        stale_fingerprint: 0,
        uncertified: 0,
    };

    for result in results {
        match result.state {
            StatusProfileState::Certified => summary.certified += 1,
            StatusProfileState::UntrustedSigner => summary.untrusted_signer += 1,
            StatusProfileState::InvalidSignature => summary.invalid_signature += 1,
            StatusProfileState::StaleCommit => summary.stale_commit += 1,
            StatusProfileState::StaleFingerprint => summary.stale_fingerprint += 1,
            StatusProfileState::Uncertified => summary.uncertified += 1,
        }
    }

    summary
}

fn map_profile_state(state: &ProfileCertificationState) -> StatusProfileState {
    match state {
        ProfileCertificationState::Certified => StatusProfileState::Certified,
        ProfileCertificationState::UntrustedSigner => StatusProfileState::UntrustedSigner,
        ProfileCertificationState::InvalidSignature => StatusProfileState::InvalidSignature,
        ProfileCertificationState::StaleCommit => StatusProfileState::StaleCommit,
        ProfileCertificationState::StaleFingerprint => StatusProfileState::StaleFingerprint,
        ProfileCertificationState::Uncertified => StatusProfileState::Uncertified,
    }
}
