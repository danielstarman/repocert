use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use crate::config::error::{ValidationErrorKind, ValidationIssue};
use crate::config::model::RepoPath;

pub(super) fn validate_name(section: &str, name: &str, issues: &mut Vec<ValidationIssue>) {
    if name.trim().is_empty() {
        issues.push(issue(
            ValidationErrorKind::EmptyName,
            section.to_string(),
            "names must not be empty".to_string(),
        ));
    }
}

pub(super) fn validate_argv(subject: &str, argv: &[String], issues: &mut Vec<ValidationIssue>) {
    if argv.is_empty() {
        issues.push(issue(
            ValidationErrorKind::InvalidCommand,
            subject.to_string(),
            "argv must include the executable as the first element".to_string(),
        ));
        return;
    }

    for arg in argv {
        if arg.is_empty() {
            issues.push(issue(
                ValidationErrorKind::InvalidCommand,
                subject.to_string(),
                "argv must not contain empty strings".to_string(),
            ));
        }
    }
}

pub(super) fn validate_env_keys(
    subject: &str,
    env: &BTreeMap<String, String>,
    issues: &mut Vec<ValidationIssue>,
) {
    for key in env.keys() {
        if key.trim().is_empty() {
            issues.push(issue(
                ValidationErrorKind::InvalidCommand,
                subject.to_string(),
                "environment variable names must not be empty".to_string(),
            ));
        }
        if key.contains('=') {
            issues.push(issue(
                ValidationErrorKind::InvalidCommand,
                subject.to_string(),
                format!("environment variable name {:?} must not contain '='", key),
            ));
        }
    }
}

pub(super) fn validate_timeout(
    subject: &str,
    timeout_ms: Option<u64>,
    issues: &mut Vec<ValidationIssue>,
) {
    if timeout_ms == Some(0) {
        issues.push(issue(
            ValidationErrorKind::InvalidCommand,
            subject.to_string(),
            "timeout_ms must be greater than zero when provided".to_string(),
        ));
    }
}

pub(super) fn normalize_repo_path(raw_path: &str, repo_root: &Path) -> Result<RepoPath, String> {
    let path = Path::new(raw_path);

    if raw_path.trim().is_empty() {
        return Err("path must not be empty".to_string());
    }
    if path.is_absolute() {
        return Err("path must be relative to the repo root".to_string());
    }

    let mut normalized = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err("path must not escape the repo root".to_string());
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("path must be relative to the repo root".to_string());
            }
        }
    }

    if normalized.is_empty() {
        return Err("path must not normalize to the repo root".to_string());
    }

    let normalized_path = normalized.join("/");
    let full_path = repo_root.join(PathBuf::from(normalized_path.clone()));
    if !full_path.starts_with(repo_root) {
        return Err("path must stay within the repo root".to_string());
    }

    Ok(RepoPath::new(normalized_path))
}

pub(super) fn issue(
    kind: ValidationErrorKind,
    subject: impl Into<String>,
    message: impl Into<String>,
) -> ValidationIssue {
    ValidationIssue {
        kind,
        subject: subject.into(),
        message: message.into(),
    }
}
