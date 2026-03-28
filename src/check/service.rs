use std::path::Path;

use crate::config::RepoSession;
use crate::contract::progress_label;

use super::error::CheckError;
use super::execute::run_planned_item;
use super::plan::{SelectionPlan, build_selection_plan};
use super::types::{CheckItemResult, CheckOptions, CheckOutcome, CheckReport, CheckSummary};

/// Run contract checks and fixer probes for the selected profiles or named checks.
pub fn run_check(session: &RepoSession, options: CheckOptions) -> Result<CheckReport, CheckError> {
    let CheckOptions {
        profiles,
        names,
        emit_progress,
    } = options;

    let plan = build_selection_plan(&session.contract, &profiles, &names)?;
    let results = execute_plan(&session.paths.repo_root, &plan, emit_progress);
    let summary = summarize(&results);

    Ok(CheckReport {
        selection_mode: plan.selection_mode,
        profiles: plan.profiles,
        checks: plan.checks,
        results,
        summary,
    })
}

fn execute_plan(
    repo_root: &Path,
    plan: &SelectionPlan,
    emit_progress: bool,
) -> Vec<CheckItemResult> {
    plan.items
        .iter()
        .map(|item| {
            if emit_progress {
                eprintln!("RUN {} {}", progress_label(&item.kind), item.name);
            }
            run_planned_item(repo_root, item)
        })
        .collect()
}

fn summarize(results: &[CheckItemResult]) -> CheckSummary {
    let mut summary = CheckSummary {
        total: results.len(),
        pass: 0,
        fail: 0,
        timeout: 0,
        repair_needed: 0,
    };

    for result in results {
        match result.outcome {
            CheckOutcome::Pass => summary.pass += 1,
            CheckOutcome::Fail => summary.fail += 1,
            CheckOutcome::Timeout => summary.timeout += 1,
            CheckOutcome::RepairNeeded => summary.repair_needed += 1,
        }
    }

    summary
}
