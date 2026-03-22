use std::path::Path;

use crate::exec::{CommandRunnerOptions, CommandRunnerStatus, run_command};
use crate::git::{capture_pathspec_snapshot, protected_pathspecs};

use super::plan::PlannedFixer;
use super::types::{FixItemResult, FixOutcome};

pub(super) fn run_planned_fixer(
    repo_root: &Path,
    protected_roots: &[String],
    item: &PlannedFixer,
) -> FixItemResult {
    let before = match capture_pathspec_snapshot(repo_root, protected_roots) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return FixItemResult {
                name: item.name.clone(),
                outcome: FixOutcome::Fail,
                exit_code: None,
                duration_ms: 0,
                message: Some(format!("failed to capture pre-run git status: {error}")),
            };
        }
    };

    let execution = run_command(
        repo_root,
        &CommandRunnerOptions {
            argv: item.command.argv.clone(),
            env: item.command.env.clone(),
            timeout_ms: item.command.timeout_ms,
        },
    );

    let after = match capture_pathspec_snapshot(repo_root, protected_roots) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return FixItemResult {
                name: item.name.clone(),
                outcome: FixOutcome::Fail,
                exit_code: status_exit_code(&execution.status),
                duration_ms: execution.duration_ms,
                message: Some(format!("failed to capture post-run git status: {error}")),
            };
        }
    };

    let changed_protected_paths = before.changed_paths(&after);
    if !changed_protected_paths.is_empty() {
        return FixItemResult {
            name: item.name.clone(),
            outcome: FixOutcome::Fail,
            exit_code: status_exit_code(&execution.status),
            duration_ms: execution.duration_ms,
            message: Some(format!(
                "fixer modified protected contract path(s): {}",
                changed_protected_paths.join(", ")
            )),
        };
    }

    FixItemResult {
        name: item.name.clone(),
        outcome: classify_status(&execution.status),
        exit_code: status_exit_code(&execution.status),
        duration_ms: execution.duration_ms,
        message: execution.message,
    }
}

pub(super) fn protected_roots(contract: &crate::config::Contract) -> Vec<String> {
    protected_pathspecs(contract)
}

fn classify_status(status: &CommandRunnerStatus) -> FixOutcome {
    match status {
        CommandRunnerStatus::TimedOut => FixOutcome::Timeout,
        CommandRunnerStatus::Exited { exit_code: Some(0) } => FixOutcome::Pass,
        CommandRunnerStatus::Exited { .. } => FixOutcome::Fail,
    }
}

fn status_exit_code(status: &CommandRunnerStatus) -> Option<i32> {
    match status {
        CommandRunnerStatus::TimedOut => None,
        CommandRunnerStatus::Exited { exit_code } => *exit_code,
    }
}
