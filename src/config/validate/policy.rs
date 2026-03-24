use std::collections::BTreeSet;
use std::path::Path;

use crate::config::error::{ValidationErrorKind, ValidationIssue};
use crate::config::model::{HookMode, HooksConfig, LocalPolicy, ProtectedRef, RepoPath};
use crate::config::raw::{RawConfig, RawHooks, RawLocalPolicy};
use crate::contract::validate_pattern;

use super::common::{issue, normalize_repo_path};

pub(super) fn validate_protected_paths(
    paths: &[String],
    repo_root: &Path,
    issues: &mut Vec<ValidationIssue>,
) -> BTreeSet<RepoPath> {
    let mut normalized = BTreeSet::new();

    for raw_path in paths {
        match normalize_repo_path(raw_path, repo_root) {
            Ok(path) => {
                if !normalized.insert(path.clone()) {
                    issues.push(issue(
                        ValidationErrorKind::InvalidProtectedPath,
                        "protected_paths".to_string(),
                        format!(
                            "duplicate protected path after normalization: {:?}",
                            path.as_str()
                        ),
                    ));
                }
            }
            Err(message) => issues.push(issue(
                ValidationErrorKind::InvalidProtectedPath,
                "protected_paths".to_string(),
                format!("{raw_path:?}: {message}"),
            )),
        }
    }

    normalized
}

pub(super) fn validate_protected_refs(
    raw: &RawConfig,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<ProtectedRef> {
    let mut validated = Vec::new();

    for rule in &raw.protected_refs {
        let subject = format!("protected_refs[pattern={}]", rule.pattern);

        if rule.pattern.trim().is_empty() {
            issues.push(issue(
                ValidationErrorKind::InvalidProtectedRef,
                subject.clone(),
                "protected ref pattern must not be empty".to_string(),
            ));
        } else if let Err(message) = validate_pattern(&rule.pattern) {
            issues.push(issue(
                ValidationErrorKind::InvalidProtectedRef,
                subject.clone(),
                format!("invalid protected ref pattern: {message}"),
            ));
        }

        match raw.profiles.get(&rule.profile) {
            Some(profile) if profile.certify => {}
            Some(_) => issues.push(issue(
                ValidationErrorKind::InvalidProtectedRef,
                subject.clone(),
                format!(
                    "protected ref requires non-certification-eligible profile {:?}",
                    rule.profile
                ),
            )),
            None => issues.push(issue(
                ValidationErrorKind::InvalidProtectedRef,
                subject.clone(),
                format!(
                    "protected ref references unknown profile {:?}",
                    rule.profile
                ),
            )),
        }

        validated.push(ProtectedRef {
            pattern: rule.pattern.clone(),
            profile: rule.profile.clone(),
        });
    }

    validated
}

pub(super) fn validate_local_policy(
    raw: Option<&RawLocalPolicy>,
    issues: &mut Vec<ValidationIssue>,
) -> Option<LocalPolicy> {
    let raw = raw?;

    if raw.protected_branches.is_empty() {
        issues.push(issue(
            ValidationErrorKind::InvalidLocalPolicy,
            "local_policy.protected_branches".to_string(),
            "local protected policy requires at least one protected branch pattern".to_string(),
        ));
    }

    for pattern in &raw.protected_branches {
        if pattern.trim().is_empty() {
            issues.push(issue(
                ValidationErrorKind::InvalidLocalPolicy,
                "local_policy.protected_branches".to_string(),
                "protected branch patterns must not be empty".to_string(),
            ));
            continue;
        }
        if !pattern.starts_with("refs/heads/") {
            issues.push(issue(
                ValidationErrorKind::InvalidLocalPolicy,
                "local_policy.protected_branches".to_string(),
                format!("local protected branch pattern {pattern:?} must target refs/heads/*"),
            ));
            continue;
        }
        if let Err(message) = validate_pattern(pattern) {
            issues.push(issue(
                ValidationErrorKind::InvalidLocalPolicy,
                "local_policy.protected_branches".to_string(),
                format!("invalid protected branch pattern {pattern:?}: {message}"),
            ));
        }
    }

    Some(LocalPolicy {
        protected_branches: raw.protected_branches.clone(),
        require_clean_primary_checkout: raw.require_clean_primary_checkout,
    })
}

pub(super) fn validate_hooks(
    hooks: Option<&RawHooks>,
    local_policy: Option<&LocalPolicy>,
    protected_refs: &[ProtectedRef],
    issues: &mut Vec<ValidationIssue>,
) -> Option<HooksConfig> {
    let Some(hooks) = hooks else {
        if local_policy.is_some() {
            issues.push(issue(
                ValidationErrorKind::InvalidLocalPolicy,
                "local_policy".to_string(),
                "local protected policy requires hooks configuration so it can be enforced"
                    .to_string(),
            ));
        }
        return None;
    };

    match hooks.mode.as_str() {
        "generated" => {
            if let Some(generated) = hooks.generated.as_ref() {
                issues.push(issue(
                    ValidationErrorKind::InvalidHookMode,
                    if generated.hooks.is_some() {
                        "hooks.generated.hooks".to_string()
                    } else {
                        "hooks.generated".to_string()
                    },
                    "generated hook mode derives managed hooks from protected_refs and local_policy; [hooks.generated] is not allowed"
                        .to_string(),
                ));
            }

            if local_policy.is_none() && protected_refs.is_empty() {
                issues.push(issue(
                    ValidationErrorKind::InvalidHookMode,
                    "hooks.mode".to_string(),
                    "generated hook mode requires protected_refs and/or local_policy".to_string(),
                ));
            }

            Some(HooksConfig {
                mode: HookMode::Generated,
            })
        }
        other => {
            issues.push(issue(
                ValidationErrorKind::InvalidHookMode,
                "hooks.mode".to_string(),
                format!("unsupported hook mode {other:?}; expected \"generated\""),
            ));
            None
        }
    }
}
