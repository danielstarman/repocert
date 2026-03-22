use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

use super::error::{LoadError, ValidationErrorKind, ValidationErrors, ValidationIssue};
use super::model::{
    CommandSpec, Contract, FixerSpec, HookMode, HooksConfig, Profile, ProtectedRef, RepoPath,
};
use super::raw::{RawCommand, RawConfig, RawFixer, RawHooks, RawProfile};

const SUPPORTED_SCHEMA_VERSION: u64 = 1;

pub(super) fn validate(raw: RawConfig, repo_root: &Path) -> Result<Contract, LoadError> {
    let mut issues = Vec::new();

    if raw.schema_version != SUPPORTED_SCHEMA_VERSION {
        issues.push(issue(
            ValidationErrorKind::SchemaVersion,
            "schema_version",
            format!(
                "expected schema_version = {SUPPORTED_SCHEMA_VERSION}, found {}",
                raw.schema_version
            ),
        ));
    }

    let checks = raw
        .checks
        .iter()
        .map(|(name, command)| (name.clone(), validate_command(name, command, &mut issues)))
        .collect::<BTreeMap<_, _>>();

    let fixers = raw
        .fixers
        .iter()
        .map(|(name, fixer)| (name.clone(), validate_fixer(name, fixer, &mut issues)))
        .collect::<BTreeMap<_, _>>();

    validate_profile_names(&raw.profiles, &mut issues);
    validate_profile_references(&raw, &mut issues);

    let resolved_profiles = resolve_profiles(&raw.profiles, &mut issues);

    let default_profile = validate_default_profile(&raw.profiles, &mut issues);
    validate_certifiable_profiles(&raw.profiles, &resolved_profiles, &mut issues);

    let declared_protected_paths =
        validate_protected_paths(&raw.protected_paths, repo_root, &mut issues);
    let protected_refs = validate_protected_refs(&raw, &mut issues);
    let hooks = validate_hooks(raw.hooks.as_ref(), repo_root, &mut issues);

    if !issues.is_empty() {
        return Err(LoadError::Validation(ValidationErrors::new(issues)));
    }

    let profiles = raw
        .profiles
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
                    default: default_profile.as_deref() == Some(name.as_str()),
                    certify: profile.certify,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    Ok(Contract {
        schema_version: raw.schema_version,
        checks,
        fixers,
        profiles,
        default_profile,
        built_in_protected_dir: RepoPath::new(".repocert".to_string()),
        declared_protected_paths,
        protected_refs,
        hooks,
    })
}

fn validate_command(
    name: &str,
    command: &RawCommand,
    issues: &mut Vec<ValidationIssue>,
) -> CommandSpec {
    validate_name("checks", name, issues);
    validate_argv(&format!("checks.{name}.argv"), &command.argv, issues);
    validate_env_keys(&format!("checks.{name}.env"), &command.env, issues);
    validate_timeout(
        &format!("checks.{name}.timeout_ms"),
        command.timeout_ms,
        issues,
    );

    CommandSpec {
        argv: command.argv.clone(),
        env: command.env.clone(),
        timeout_ms: command.timeout_ms,
    }
}

fn validate_fixer(name: &str, fixer: &RawFixer, issues: &mut Vec<ValidationIssue>) -> FixerSpec {
    validate_name("fixers", name, issues);
    validate_argv(&format!("fixers.{name}.argv"), &fixer.argv, issues);
    if let Some(probe_argv) = &fixer.probe_argv {
        validate_argv(&format!("fixers.{name}.probe_argv"), probe_argv, issues);
    }
    validate_env_keys(&format!("fixers.{name}.env"), &fixer.env, issues);
    validate_timeout(
        &format!("fixers.{name}.timeout_ms"),
        fixer.timeout_ms,
        issues,
    );
    validate_timeout(
        &format!("fixers.{name}.probe_timeout_ms"),
        fixer.probe_timeout_ms,
        issues,
    );

    FixerSpec {
        argv: fixer.argv.clone(),
        probe_argv: fixer.probe_argv.clone(),
        env: fixer.env.clone(),
        timeout_ms: fixer.timeout_ms,
        probe_timeout_ms: fixer.probe_timeout_ms,
    }
}

fn validate_profile_names(
    profiles: &BTreeMap<String, RawProfile>,
    issues: &mut Vec<ValidationIssue>,
) {
    for name in profiles.keys() {
        validate_name("profiles", name, issues);
    }
}

fn validate_profile_references(raw: &RawConfig, issues: &mut Vec<ValidationIssue>) {
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

#[derive(Clone, Debug)]
struct ResolvedProfile {
    effective_checks: Vec<String>,
    effective_fixers: Vec<String>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum VisitState {
    Visiting,
    Resolved,
}

fn resolve_profiles(
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

fn validate_default_profile(
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

fn validate_certifiable_profiles(
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

fn validate_protected_paths(
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

fn validate_protected_refs(
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

fn validate_hooks(
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

fn validate_name(section: &str, name: &str, issues: &mut Vec<ValidationIssue>) {
    if name.trim().is_empty() {
        issues.push(issue(
            ValidationErrorKind::EmptyName,
            section.to_string(),
            "names must not be empty".to_string(),
        ));
    }
}

fn validate_argv(subject: &str, argv: &[String], issues: &mut Vec<ValidationIssue>) {
    if argv.is_empty() {
        issues.push(issue(
            ValidationErrorKind::InvalidCommand,
            subject.to_string(),
            "argv must contain at least one element".to_string(),
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

fn validate_env_keys(
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

fn validate_timeout(subject: &str, timeout_ms: Option<u64>, issues: &mut Vec<ValidationIssue>) {
    if timeout_ms == Some(0) {
        issues.push(issue(
            ValidationErrorKind::InvalidCommand,
            subject.to_string(),
            "timeout_ms must be greater than zero when provided".to_string(),
        ));
    }
}

fn normalize_repo_path(raw_path: &str, repo_root: &Path) -> Result<RepoPath, String> {
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

fn issue(
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::normalize_repo_path;

    #[test]
    fn normalizes_repo_relative_paths() {
        let repo_root = Path::new("/tmp/example");
        let path = normalize_repo_path("./docs/./spec.md", repo_root).unwrap();
        assert_eq!(path.as_str(), "docs/spec.md");
    }

    #[test]
    fn rejects_parent_directory_escape() {
        let repo_root = Path::new("/tmp/example");
        let error = normalize_repo_path("../secret", repo_root).unwrap_err();
        assert!(error.contains("escape"));
    }
}
