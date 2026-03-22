use std::collections::BTreeMap;

use crate::config::error::ValidationIssue;
use crate::config::model::{CommandSpec, FixerSpec};
use crate::config::raw::{RawCommand, RawFixer};

use super::common::{validate_argv, validate_env_keys, validate_name, validate_timeout};

pub(super) fn validate_checks(
    checks: &BTreeMap<String, RawCommand>,
    issues: &mut Vec<ValidationIssue>,
) -> BTreeMap<String, CommandSpec> {
    checks
        .iter()
        .map(|(name, command)| (name.clone(), validate_command(name, command, issues)))
        .collect()
}

pub(super) fn validate_fixers(
    fixers: &BTreeMap<String, RawFixer>,
    issues: &mut Vec<ValidationIssue>,
) -> BTreeMap<String, FixerSpec> {
    fixers
        .iter()
        .map(|(name, fixer)| (name.clone(), validate_fixer(name, fixer, issues)))
        .collect()
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
