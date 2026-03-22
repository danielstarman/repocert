use std::path::Path;

use crate::check::types::{CheckItemKind, CheckItemResult, CheckOutcome};
use crate::contract::{
    EvaluationItem, EvaluationItemKind, EvaluationItemResult, EvaluationOutcome,
    run_evaluation_item,
};

pub(super) fn run_planned_item(repo_root: &Path, item: &EvaluationItem) -> CheckItemResult {
    map_result(run_evaluation_item(repo_root, item))
}

fn map_result(result: EvaluationItemResult) -> CheckItemResult {
    CheckItemResult {
        name: result.name,
        kind: map_kind(result.kind),
        outcome: map_outcome(result.outcome),
        exit_code: result.exit_code,
        duration_ms: result.duration_ms,
        message: result.message,
    }
}

fn map_kind(kind: EvaluationItemKind) -> CheckItemKind {
    match kind {
        EvaluationItemKind::Check => CheckItemKind::Check,
        EvaluationItemKind::FixerProbe => CheckItemKind::FixerProbe,
    }
}

fn map_outcome(outcome: EvaluationOutcome) -> CheckOutcome {
    match outcome {
        EvaluationOutcome::Pass => CheckOutcome::Pass,
        EvaluationOutcome::Fail => CheckOutcome::Fail,
        EvaluationOutcome::Timeout => CheckOutcome::Timeout,
        EvaluationOutcome::RepairNeeded => CheckOutcome::RepairNeeded,
    }
}
