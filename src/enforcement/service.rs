use std::collections::BTreeSet;

use crate::certification::{
    CertificationStore, ProfileCertificationState, compute_contract_fingerprint,
    inspect_profile_certification,
};
use crate::config::load_contract;
use crate::contract::matches_pattern;
use crate::git::resolve_commit;

use super::error::AuthorizeError;
use super::types::{
    AuthorizeOptions, AuthorizeProfileResult, AuthorizeProfileState, AuthorizeReport, MatchedRule,
};

/// Authorize a proposed ref update against the current contract and certification store.
pub fn authorize_ref_update(options: AuthorizeOptions) -> Result<AuthorizeReport, AuthorizeError> {
    let AuthorizeOptions {
        load_options,
        old,
        new,
        reference,
    } = options;

    let loaded = load_contract(load_options)?;
    if is_zero_oid(&new) {
        return Err(AuthorizeError::UnsupportedDeletion {
            paths: loaded.paths.clone(),
        });
    }

    let target_commit = resolve_commit(&loaded.paths.repo_root, &new).map_err(|error| {
        AuthorizeError::GitCommit {
            paths: loaded.paths.clone(),
            error,
        }
    })?;
    let contract_fingerprint =
        compute_contract_fingerprint(&loaded).map_err(|error| AuthorizeError::Fingerprint {
            paths: loaded.paths.clone(),
            error,
        })?;
    let store = CertificationStore::open(&loaded.paths.repo_root).map_err(|error| {
        AuthorizeError::Storage {
            paths: loaded.paths.clone(),
            error,
        }
    })?;
    let certification = loaded.contract.certification.as_ref();

    let matched_rules = loaded
        .contract
        .protected_refs
        .iter()
        .filter_map(|rule| match matches_pattern(&rule.pattern, &reference) {
            Ok(true) => Some(Ok(MatchedRule {
                pattern: rule.pattern.clone(),
                profile: rule.profile.clone(),
            })),
            Ok(false) => None,
            Err(message) => Some(Err(AuthorizeError::InvalidPattern {
                paths: loaded.paths.clone(),
                pattern: rule.pattern.clone(),
                message,
            })),
        })
        .collect::<Result<Vec<_>, _>>()?;

    let required_profiles = dedupe_profiles(&matched_rules);
    let profile_results = required_profiles
        .iter()
        .map(|profile| {
            let certification = certification
                .as_ref()
                .expect("certification-eligible profiles require certification config");
            inspect_profile_certification(
                &store,
                &target_commit,
                profile,
                &contract_fingerprint,
                certification,
            )
            .map(|inspection| AuthorizeProfileResult {
                profile: inspection.profile,
                state: map_profile_state(inspection.state),
                signer_name: inspection.signer_name,
            })
            .map_err(|error| AuthorizeError::Storage {
                paths: loaded.paths.clone(),
                error,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let allowed = profile_results
        .iter()
        .all(|result| result.state == AuthorizeProfileState::Certified);

    Ok(AuthorizeReport {
        paths: loaded.paths,
        old,
        new,
        reference,
        target_commit,
        contract_fingerprint,
        matched_rules,
        required_profiles,
        profile_results,
        allowed,
    })
}

fn dedupe_profiles(matched_rules: &[MatchedRule]) -> Vec<String> {
    let mut profiles = Vec::new();
    let mut seen = BTreeSet::new();
    for rule in matched_rules {
        if seen.insert(rule.profile.clone()) {
            profiles.push(rule.profile.clone());
        }
    }
    profiles
}

fn map_profile_state(state: ProfileCertificationState) -> AuthorizeProfileState {
    match state {
        ProfileCertificationState::Certified => AuthorizeProfileState::Certified,
        ProfileCertificationState::UntrustedSigner => AuthorizeProfileState::UntrustedSigner,
        ProfileCertificationState::InvalidSignature => AuthorizeProfileState::InvalidSignature,
        ProfileCertificationState::StaleCommit => AuthorizeProfileState::StaleCommit,
        ProfileCertificationState::StaleFingerprint => AuthorizeProfileState::StaleFingerprint,
        ProfileCertificationState::Uncertified => AuthorizeProfileState::Uncertified,
    }
}

fn is_zero_oid(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte == b'0')
}
