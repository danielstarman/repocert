use std::collections::BTreeSet;

use thiserror::Error;

use crate::config::Contract;

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub(crate) enum SelectionError {
    #[error("selector modes are mutually exclusive")]
    ConflictingSelectors,
    #[error("no implicit or explicit default profile is available")]
    NoDefaultProfile,
    #[error("unknown profile selector(s): {0}")]
    UnknownProfiles(String),
    #[error("unknown named check selector(s): {0}")]
    UnknownChecks(String),
    #[error("unknown named fixer selector(s): {0}")]
    UnknownFixers(String),
}

pub(crate) fn resolve_profiles(
    contract: &Contract,
    requested_profiles: &[String],
) -> Result<Vec<String>, SelectionError> {
    let profiles = if requested_profiles.is_empty() {
        vec![
            contract
                .default_profile
                .clone()
                .ok_or(SelectionError::NoDefaultProfile)?,
        ]
    } else {
        dedupe_preserving_order(requested_profiles)
    };

    let unknown = profiles
        .iter()
        .filter(|name| !contract.profiles.contains_key(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if !unknown.is_empty() {
        return Err(SelectionError::UnknownProfiles(unknown.join(", ")));
    }

    Ok(profiles)
}

pub(crate) fn resolve_named_checks(
    contract: &Contract,
    names: &[String],
) -> Result<Vec<String>, SelectionError> {
    let checks = dedupe_preserving_order(names);
    let unknown = checks
        .iter()
        .filter(|name| !contract.checks.contains_key(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if !unknown.is_empty() {
        return Err(SelectionError::UnknownChecks(unknown.join(", ")));
    }

    Ok(checks)
}

pub(crate) fn resolve_named_fixers(
    contract: &Contract,
    names: &[String],
) -> Result<Vec<String>, SelectionError> {
    let fixers = dedupe_preserving_order(names);
    let unknown = fixers
        .iter()
        .filter(|name| !contract.fixers.contains_key(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if !unknown.is_empty() {
        return Err(SelectionError::UnknownFixers(unknown.join(", ")));
    }

    Ok(fixers)
}

pub(crate) fn collect_effective_fixers(contract: &Contract, profiles: &[String]) -> Vec<String> {
    let mut fixers = Vec::new();
    let mut seen_fixers = BTreeSet::new();

    for profile_name in profiles {
        let profile = contract
            .profiles
            .get(profile_name)
            .expect("profile should exist after validation");

        for fixer in &profile.effective_fixers {
            if seen_fixers.insert(fixer.clone()) {
                fixers.push(fixer.clone());
            }
        }
    }

    fixers
}

fn dedupe_preserving_order(names: &[String]) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();

    for name in names {
        if seen.insert(name.clone()) {
            ordered.push(name.clone());
        }
    }

    ordered
}
