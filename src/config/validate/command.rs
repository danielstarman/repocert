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
    validate_exec_command("checks", name, command, issues)
}

fn validate_fixer(name: &str, fixer: &RawFixer, issues: &mut Vec<ValidationIssue>) -> FixerSpec {
    let command = validate_exec_command("fixers", name, &fixer.command, issues);
    if let Some(probe_argv) = &fixer.probe_argv {
        validate_argv(&format!("fixers.{name}.probe_argv"), probe_argv, issues);
    }
    validate_timeout(
        &format!("fixers.{name}.probe_timeout_ms"),
        fixer.probe_timeout_ms,
        issues,
    );

    FixerSpec {
        command,
        probe_argv: fixer.probe_argv.clone(),
        probe_timeout_ms: fixer.probe_timeout_ms,
    }
}

fn validate_exec_command(
    section: &str,
    name: &str,
    command: &RawCommand,
    issues: &mut Vec<ValidationIssue>,
) -> CommandSpec {
    validate_name(section, name, issues);
    validate_argv(&format!("{section}.{name}.argv"), &command.argv, issues);
    validate_env_keys(&format!("{section}.{name}.env"), &command.env, issues);
    validate_timeout(
        &format!("{section}.{name}.timeout_ms"),
        command.timeout_ms,
        issues,
    );

    CommandSpec {
        argv: command.argv.clone(),
        env: command.env.clone(),
        timeout_ms: command.timeout_ms,
    }
}
