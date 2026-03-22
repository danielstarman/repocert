use std::path::PathBuf;

use crate::config::load_contract;

use super::error::CheckError;
use super::execute::run_planned_item;
use super::plan::{SelectionPlan, build_selection_plan};
use super::types::{
    CheckItemKind, CheckItemResult, CheckOptions, CheckOutcome, CheckReport, CheckSummary,
};

pub fn run_check(options: CheckOptions) -> Result<CheckReport, CheckError> {
    let CheckOptions {
        load_options,
        profiles,
        names,
        emit_progress,
    } = options;

    let loaded = load_contract(load_options)?;
    let plan = build_selection_plan(&loaded.contract, &profiles, &names).map_err(|error| {
        CheckError::Selection {
            paths: loaded.paths.clone(),
            error,
        }
    })?;
    let results = execute_plan(&loaded.paths.repo_root, &plan, emit_progress);
    let summary = summarize(&results);

    Ok(CheckReport {
        paths: loaded.paths,
        selection_mode: plan.selection_mode,
        profiles: plan.profiles,
        checks: plan.checks,
        results,
        summary,
    })
}

fn execute_plan(
    repo_root: &PathBuf,
    plan: &SelectionPlan,
    emit_progress: bool,
) -> Vec<CheckItemResult> {
    plan.items
        .iter()
        .map(|item| {
            if emit_progress {
                eprintln!("RUN {} {}", item_kind_label(&item.kind), item.name);
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

fn item_kind_label(kind: &CheckItemKind) -> &'static str {
    match kind {
        CheckItemKind::Check => "check",
        CheckItemKind::FixerProbe => "probe",
    }
}
