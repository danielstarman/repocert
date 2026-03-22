use std::path::Path;

use crate::check::types::{CheckItemKind, CheckItemResult, CheckOutcome};
use crate::exec::{CommandRunnerOptions, CommandRunnerStatus, run_command};

use super::plan::PlannedItem;

pub(super) fn run_planned_item(repo_root: &Path, item: &PlannedItem) -> CheckItemResult {
    let execution = run_command(
        repo_root,
        &CommandRunnerOptions {
            argv: item.command.argv.clone(),
            env: item.command.env.clone(),
            timeout_ms: item.command.timeout_ms,
        },
    );

    let (outcome, exit_code) = classify_execution(&item.kind, &execution.status);

    CheckItemResult {
        name: item.name.clone(),
        kind: item.kind.clone(),
        outcome,
        exit_code,
        duration_ms: execution.duration_ms,
        message: execution.message,
    }
}

fn classify_execution(
    kind: &CheckItemKind,
    status: &CommandRunnerStatus,
) -> (CheckOutcome, Option<i32>) {
    match status {
        CommandRunnerStatus::TimedOut => (CheckOutcome::Timeout, None),
        CommandRunnerStatus::Exited { exit_code } => (classify_exit(kind, *exit_code), *exit_code),
    }
}

fn classify_exit(kind: &CheckItemKind, exit_code: Option<i32>) -> CheckOutcome {
    match kind {
        CheckItemKind::Check => match exit_code {
            Some(0) => CheckOutcome::Pass,
            _ => CheckOutcome::Fail,
        },
        CheckItemKind::FixerProbe => match exit_code {
            Some(0) => CheckOutcome::Pass,
            Some(1) => CheckOutcome::RepairNeeded,
            _ => CheckOutcome::Fail,
        },
    }
}
