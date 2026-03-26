mod command;
mod common;
mod policy;
mod profile;

use std::path::Path;

use super::error::{LoadError, ValidationErrorKind, ValidationErrors};
use super::model::{Contract, RepoPath};
use super::raw::RawConfig;

use command::{validate_checks, validate_fixers};
use common::issue;
use policy::{
    validate_certification, validate_hooks, validate_local_policy, validate_protected_paths,
    validate_protected_refs,
};
use profile::{
    build_profiles, resolve_profiles, validate_certifiable_profiles, validate_default_profile,
    validate_profile_names, validate_profile_references,
};

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

    let checks = validate_checks(&raw.checks, &mut issues);
    let fixers = validate_fixers(&raw.fixers, &mut issues);

    validate_profile_names(&raw.profiles, &mut issues);
    validate_profile_references(&raw, &mut issues);

    let resolved_profiles = resolve_profiles(&raw.profiles, &mut issues);

    let default_profile = validate_default_profile(&raw.profiles, &mut issues);
    validate_certifiable_profiles(&raw.profiles, &resolved_profiles, &mut issues);

    let declared_protected_paths =
        validate_protected_paths(&raw.protected_paths, repo_root, &mut issues);
    let protected_refs = validate_protected_refs(&raw, &mut issues);
    let certification =
        validate_certification(raw.certification.as_ref(), &raw.profiles, &mut issues);
    let local_policy = validate_local_policy(raw.local_policy.as_ref(), &mut issues);
    let hooks = validate_hooks(
        raw.hooks.as_ref(),
        local_policy.as_ref(),
        &protected_refs,
        &mut issues,
    );

    if !issues.is_empty() {
        return Err(LoadError::Validation(ValidationErrors::new(issues)));
    }

    let profiles = build_profiles(
        &raw.profiles,
        &resolved_profiles,
        default_profile.as_deref(),
    );

    Ok(Contract {
        schema_version: raw.schema_version,
        checks,
        fixers,
        profiles,
        default_profile,
        built_in_protected_dir: RepoPath::new(".repocert".to_string()),
        declared_protected_paths,
        protected_refs,
        certification,
        local_policy,
        hooks,
    })
}
