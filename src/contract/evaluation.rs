use std::path::Path;

use crate::config::{CommandSpec, Contract, FixerSpec};
use crate::exec::{CommandRunnerOptions, CommandRunnerStatus, run_command};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EvaluationItemKind {
    Check,
    FixerProbe,
}

#[derive(Clone, Debug)]
pub(crate) struct EvaluationItem {
    pub name: String,
    pub kind: EvaluationItemKind,
    pub command: CommandSpec,
}

#[derive(Clone, Debug)]
pub(crate) struct ProfileEvaluationPlan {
    pub profile: String,
    pub checks: Vec<String>,
    pub items: Vec<EvaluationItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EvaluationOutcome {
    Pass,
    Fail,
    Timeout,
    RepairNeeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EvaluationItemResult {
    pub name: String,
    pub kind: EvaluationItemKind,
    pub outcome: EvaluationOutcome,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub message: Option<String>,
}

pub(crate) fn build_profile_evaluation_plan(
    contract: &Contract,
    profile_name: &str,
) -> ProfileEvaluationPlan {
    let profile = contract
        .profiles
        .get(profile_name)
        .expect("selected profile should exist after validation");
    let checks = profile.effective_checks.clone();
    let fixer_names = profile.effective_fixers.clone();

    let mut items = checks
        .iter()
        .map(|name| EvaluationItem {
            name: name.clone(),
            kind: EvaluationItemKind::Check,
            command: contract
                .checks
                .get(name)
                .expect("selected check should exist after validation")
                .clone(),
        })
        .collect::<Vec<_>>();

    items.extend(
        fixer_names
            .iter()
            .map(|name| build_probe_item(contract, name)),
    );

    ProfileEvaluationPlan {
        profile: profile_name.to_string(),
        checks,
        items,
    }
}

pub(crate) fn run_evaluation_item(repo_root: &Path, item: &EvaluationItem) -> EvaluationItemResult {
    let execution = run_command(
        repo_root,
        &CommandRunnerOptions {
            argv: item.command.argv.clone(),
            env: item.command.env.clone(),
            timeout_ms: item.command.timeout_ms,
        },
    );
    let (outcome, exit_code) = classify_execution(&item.kind, &execution.status);

    EvaluationItemResult {
        name: item.name.clone(),
        kind: item.kind.clone(),
        outcome,
        exit_code,
        duration_ms: execution.duration_ms,
        message: execution.message,
    }
}

pub(crate) fn progress_label(kind: &EvaluationItemKind) -> &'static str {
    match kind {
        EvaluationItemKind::Check => "check",
        EvaluationItemKind::FixerProbe => "probe",
    }
}

fn build_probe_item(contract: &Contract, name: &str) -> EvaluationItem {
    let fixer = contract
        .fixers
        .get(name)
        .expect("selected fixer should exist after validation");

    EvaluationItem {
        name: name.to_string(),
        kind: EvaluationItemKind::FixerProbe,
        command: probe_command_spec(fixer),
    }
}

fn probe_command_spec(fixer: &FixerSpec) -> CommandSpec {
    CommandSpec {
        argv: fixer
            .probe_argv
            .clone()
            .expect("profile-selected fixer probes are validated"),
        env: fixer.command.env.clone(),
        timeout_ms: fixer.probe_timeout_ms.or(fixer.command.timeout_ms),
    }
}

fn classify_execution(
    kind: &EvaluationItemKind,
    status: &CommandRunnerStatus,
) -> (EvaluationOutcome, Option<i32>) {
    match status {
        CommandRunnerStatus::TimedOut => (EvaluationOutcome::Timeout, None),
        CommandRunnerStatus::Exited { exit_code } => (classify_exit(kind, *exit_code), *exit_code),
    }
}

fn classify_exit(kind: &EvaluationItemKind, exit_code: Option<i32>) -> EvaluationOutcome {
    match kind {
        EvaluationItemKind::Check => match exit_code {
            Some(0) => EvaluationOutcome::Pass,
            _ => EvaluationOutcome::Fail,
        },
        EvaluationItemKind::FixerProbe => match exit_code {
            Some(0) => EvaluationOutcome::Pass,
            Some(1) => EvaluationOutcome::RepairNeeded,
            _ => EvaluationOutcome::Fail,
        },
    }
}
