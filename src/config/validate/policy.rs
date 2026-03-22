use std::collections::BTreeSet;
use std::path::Path;

use crate::config::error::{ValidationErrorKind, ValidationIssue};
use crate::config::model::{HookMode, HooksConfig, ProtectedRef, RepoPath};
use crate::config::raw::{RawConfig, RawHooks};

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

pub(super) fn validate_hooks(
    hooks: Option<&RawHooks>,
    repo_root: &Path,
    issues: &mut Vec<ValidationIssue>,
) -> Option<HooksConfig> {
    let hooks = hooks?;

    match hooks.mode.as_str() {
        "repo-owned" => {
            if hooks.generated.is_some() {
                issues.push(issue(
                    ValidationErrorKind::InvalidHookMode,
                    "hooks.generated".to_string(),
                    "generated hook configuration is not allowed when hooks.mode = \"repo-owned\""
                        .to_string(),
                ));
            }
            let Some(repo_owned) = hooks.repo_owned.as_ref() else {
                issues.push(issue(
                    ValidationErrorKind::InvalidHookMode,
                    "hooks.repo_owned".to_string(),
                    "repo-owned hook mode requires a [hooks.repo_owned] table".to_string(),
                ));
                return None;
            };
            let path = match normalize_repo_path(&repo_owned.path, repo_root) {
                Ok(path) => path,
                Err(message) => {
                    issues.push(issue(
                        ValidationErrorKind::InvalidHookMode,
                        "hooks.repo_owned.path".to_string(),
                        message,
                    ));
                    return None;
                }
            };

            Some(HooksConfig {
                mode: HookMode::RepoOwned { path },
            })
        }
        "generated" => {
            if hooks.repo_owned.is_some() {
                issues.push(issue(
                    ValidationErrorKind::InvalidHookMode,
                    "hooks.repo_owned".to_string(),
                    "repo-owned hook configuration is not allowed when hooks.mode = \"generated\""
                        .to_string(),
                ));
            }
            let Some(generated) = hooks.generated.as_ref() else {
                issues.push(issue(
                    ValidationErrorKind::InvalidHookMode,
                    "hooks.generated".to_string(),
                    "generated hook mode requires a [hooks.generated] table".to_string(),
                ));
                return None;
            };
            if generated.hooks.is_empty() {
                issues.push(issue(
                    ValidationErrorKind::InvalidHookMode,
                    "hooks.generated.hooks".to_string(),
                    "generated hook mode requires at least one hook name".to_string(),
                ));
            }
            for hook in &generated.hooks {
                if hook.trim().is_empty() {
                    issues.push(issue(
                        ValidationErrorKind::InvalidHookMode,
                        "hooks.generated.hooks".to_string(),
                        "hook names must not be empty".to_string(),
                    ));
                }
            }

            Some(HooksConfig {
                mode: HookMode::Generated {
                    hooks: generated.hooks.clone(),
                },
            })
        }
        other => {
            issues.push(issue(
                ValidationErrorKind::InvalidHookMode,
                "hooks.mode".to_string(),
                format!(
                    "unsupported hook mode {other:?}; expected \"repo-owned\" or \"generated\""
                ),
            ));
            None
        }
    }
}
