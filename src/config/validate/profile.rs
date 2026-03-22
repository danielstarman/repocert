use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::config::error::{ValidationErrorKind, ValidationIssue};
use crate::config::model::Profile;
use crate::config::raw::{RawConfig, RawProfile};

use super::common::{issue, validate_name};

#[derive(Clone, Debug)]
pub(super) struct ResolvedProfile {
    pub effective_checks: Vec<String>,
    pub effective_fixers: Vec<String>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum VisitState {
    Visiting,
    Resolved,
}

pub(super) fn validate_profile_names(
    profiles: &BTreeMap<String, RawProfile>,
    issues: &mut Vec<ValidationIssue>,
) {
    for name in profiles.keys() {
        validate_name("profiles", name, issues);
    }
}

pub(super) fn validate_profile_references(raw: &RawConfig, issues: &mut Vec<ValidationIssue>) {
    for (profile_name, profile) in &raw.profiles {
        for check in &profile.checks {
            if !raw.checks.contains_key(check) {
                issues.push(issue(
                    ValidationErrorKind::UnknownReference,
                    format!("profiles.{profile_name}.checks"),
                    format!("unknown check reference {check:?}"),
                ));
            }
        }

        for fixer in &profile.fixers {
            if !raw.fixers.contains_key(fixer) {
                issues.push(issue(
                    ValidationErrorKind::UnknownReference,
                    format!("profiles.{profile_name}.fixers"),
                    format!("unknown fixer reference {fixer:?}"),
                ));
                continue;
            }

            if raw
                .fixers
                .get(fixer)
                .and_then(|spec| spec.probe_argv.as_ref())
                .is_none()
            {
                issues.push(issue(
                    ValidationErrorKind::InvalidCertifyProfile,
                    format!("profiles.{profile_name}.fixers"),
                    format!("fixer {fixer:?} is used by a profile but does not declare probe_argv"),
                ));
            }
        }

        for include in &profile.includes {
            if !raw.profiles.contains_key(include) {
                issues.push(issue(
                    ValidationErrorKind::UnknownReference,
                    format!("profiles.{profile_name}.includes"),
                    format!("unknown included profile {include:?}"),
                ));
            }
        }
    }
}

pub(super) fn resolve_profiles(
    profiles: &BTreeMap<String, RawProfile>,
    issues: &mut Vec<ValidationIssue>,
) -> HashMap<String, ResolvedProfile> {
    let mut states = HashMap::<String, VisitState>::new();
    let mut resolved = HashMap::<String, ResolvedProfile>::new();
    let mut stack = Vec::<String>::new();

    for name in profiles.keys() {
        resolve_profile(
            name,
            profiles,
            &mut states,
            &mut resolved,
            &mut stack,
            issues,
        );
    }

    resolved
}

fn resolve_profile(
    name: &str,
    profiles: &BTreeMap<String, RawProfile>,
    states: &mut HashMap<String, VisitState>,
    resolved: &mut HashMap<String, ResolvedProfile>,
    stack: &mut Vec<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    if resolved.contains_key(name) {
        return;
    }

    if matches!(states.get(name), Some(VisitState::Visiting)) {
        let cycle_start = stack.iter().position(|entry| entry == name).unwrap_or(0);
        let cycle = stack[cycle_start..]
            .iter()
            .cloned()
            .chain(std::iter::once(name.to_string()))
            .collect::<Vec<_>>()
            .join(" -> ");
        issues.push(issue(
            ValidationErrorKind::ProfileCycle,
            format!("profiles.{name}.includes"),
            format!("profile include cycle detected: {cycle}"),
        ));
        return;
    }

    let Some(profile) = profiles.get(name) else {
        return;
    };

    states.insert(name.to_string(), VisitState::Visiting);
    stack.push(name.to_string());

    let mut checks = Vec::new();
    let mut seen_checks = BTreeSet::new();
    let mut fixers = Vec::new();
    let mut seen_fixers = BTreeSet::new();

    for include in &profile.includes {
        if !profiles.contains_key(include) {
            continue;
        }
        resolve_profile(include, profiles, states, resolved, stack, issues);
        if let Some(included) = resolved.get(include) {
            for check in &included.effective_checks {
                if seen_checks.insert(check.clone()) {
                    checks.push(check.clone());
                }
            }
            for fixer in &included.effective_fixers {
                if seen_fixers.insert(fixer.clone()) {
                    fixers.push(fixer.clone());
                }
            }
        }
    }

    for check in &profile.checks {
        if seen_checks.insert(check.clone()) {
            checks.push(check.clone());
        }
    }
    for fixer in &profile.fixers {
        if seen_fixers.insert(fixer.clone()) {
            fixers.push(fixer.clone());
        }
    }

    stack.pop();
    states.insert(name.to_string(), VisitState::Resolved);
    resolved.insert(
        name.to_string(),
        ResolvedProfile {
            effective_checks: checks,
            effective_fixers: fixers,
        },
    );
}

pub(super) fn validate_default_profile(
    profiles: &BTreeMap<String, RawProfile>,
    issues: &mut Vec<ValidationIssue>,
) -> Option<String> {
    let explicit_defaults = profiles
        .iter()
        .filter_map(|(name, profile)| profile.default.then_some(name.clone()))
        .collect::<Vec<_>>();

    if explicit_defaults.len() > 1 {
        issues.push(issue(
            ValidationErrorKind::InvalidDefaultProfile,
            "profiles".to_string(),
            format!(
                "multiple default profiles declared: {}",
                explicit_defaults.join(", ")
            ),
        ));
        return None;
    }

    if let Some(name) = explicit_defaults.into_iter().next() {
        return Some(name);
    }

    (profiles.len() == 1)
        .then(|| profiles.keys().next().cloned())
        .flatten()
}

pub(super) fn validate_certifiable_profiles(
    profiles: &BTreeMap<String, RawProfile>,
    resolved: &HashMap<String, ResolvedProfile>,
    issues: &mut Vec<ValidationIssue>,
) {
    for (name, profile) in profiles {
        if !profile.certify {
            continue;
        }

        let effective_check_count = resolved
            .get(name)
            .map(|profile| profile.effective_checks.len())
            .unwrap_or(0);

        if effective_check_count == 0 {
            issues.push(issue(
                ValidationErrorKind::InvalidCertifyProfile,
                format!("profiles.{name}"),
                "certification-eligible profiles must include at least one check after include expansion"
                    .to_string(),
            ));
        }
    }
}

pub(super) fn build_profiles(
    profiles: &BTreeMap<String, RawProfile>,
    resolved_profiles: &HashMap<String, ResolvedProfile>,
    default_profile: Option<&str>,
) -> BTreeMap<String, Profile> {
    profiles
        .iter()
        .map(|(name, profile)| {
            let resolved = resolved_profiles
                .get(name)
                .expect("resolved profiles should exist when validation succeeds");

            (
                name.clone(),
                Profile {
                    name: name.clone(),
                    declared_checks: profile.checks.clone(),
                    declared_fixers: profile.fixers.clone(),
                    declared_includes: profile.includes.clone(),
                    effective_checks: resolved.effective_checks.clone(),
                    effective_fixers: resolved.effective_fixers.clone(),
                    default: default_profile == Some(name.as_str()),
                    certify: profile.certify,
                },
            )
        })
        .collect()
}
