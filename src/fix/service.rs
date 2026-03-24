use std::path::PathBuf;

use crate::config::load_contract;

use super::error::FixError;
use super::execute::{protected_roots, run_planned_fixer};
use super::plan::{FixPlan, build_fix_plan};
use super::types::{FixItemResult, FixOptions, FixOutcome, FixReport, FixSummary};

/// Run mutating fixers for a selected profile or explicit fixer list.
pub fn run_fix(options: FixOptions) -> Result<FixReport, FixError> {
    let FixOptions {
        load_options,
        profile,
        names,
        emit_progress,
    } = options;

    let loaded = load_contract(load_options)?;
    let plan = build_fix_plan(&loaded.contract, &profile, &names).map_err(|error| {
        FixError::Selection {
            paths: loaded.paths.clone(),
            error,
        }
    })?;
    let protected_roots = protected_roots(&loaded.contract);
    let results = execute_plan(
        &loaded.paths.repo_root,
        &plan,
        &protected_roots,
        emit_progress,
    );
    let summary = summarize(&results);

    Ok(FixReport {
        paths: loaded.paths,
        selection_mode: plan.selection_mode,
        profile: plan.profile,
        fixers: plan.fixers,
        results,
        summary,
    })
}

fn execute_plan(
    repo_root: &PathBuf,
    plan: &FixPlan,
    protected_roots: &[String],
    emit_progress: bool,
) -> Vec<FixItemResult> {
    let mut results = Vec::new();

    for item in &plan.items {
        if emit_progress {
            eprintln!("RUN fixer {}", item.name);
        }

        let result = run_planned_fixer(repo_root, protected_roots, item);
        let should_stop = result.outcome != FixOutcome::Pass;
        results.push(result);
        if should_stop {
            break;
        }
    }

    results
}

fn summarize(results: &[FixItemResult]) -> FixSummary {
    let mut summary = FixSummary {
        total: results.len(),
        pass: 0,
        fail: 0,
        timeout: 0,
    };

    for result in results {
        match result.outcome {
            FixOutcome::Pass => summary.pass += 1,
            FixOutcome::Fail => summary.fail += 1,
            FixOutcome::Timeout => summary.timeout += 1,
        }
    }

    summary
}
