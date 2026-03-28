use std::path::Path;

use crate::config::RepoSession;

use super::error::FixError;
use super::execute::{protected_roots, run_planned_fixer};
use super::plan::{FixPlan, build_fix_plan};
use super::types::{FixItemResult, FixOptions, FixOutcome, FixReport, FixSummary};

/// Run mutating fixers for a selected profile or explicit fixer list.
pub fn run_fix(session: &RepoSession, options: FixOptions) -> Result<FixReport, FixError> {
    let FixOptions {
        profile,
        names,
        emit_progress,
    } = options;

    let plan = build_fix_plan(&session.contract, &profile, &names)?;
    let protected_roots = protected_roots(&session.contract);
    let results = execute_plan(
        &session.paths.repo_root,
        &plan,
        &protected_roots,
        emit_progress,
    );
    let summary = summarize(&results);

    Ok(FixReport {
        selection_mode: plan.selection_mode,
        profile: plan.profile,
        fixers: plan.fixers,
        results,
        summary,
    })
}

fn execute_plan(
    repo_root: &Path,
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
