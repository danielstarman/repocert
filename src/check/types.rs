use crate::config::{LoadOptions, LoadPaths};

/// Options for running `repocert check`.
#[derive(Clone, Debug, Default)]
pub struct CheckOptions {
    /// Contract loading options for the target repository.
    pub load_options: LoadOptions,
    /// Profile names to evaluate.
    pub profiles: Vec<String>,
    /// Direct named checks to evaluate instead of profiles.
    pub names: Vec<String>,
    /// Whether to emit human progress lines during execution.
    pub emit_progress: bool,
}

/// How checks were selected for a [`CheckReport`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckSelectionMode {
    /// Checks came from one or more profiles.
    Profiles,
    /// Checks were selected directly by name.
    Checks,
}

/// Kind of item executed during `check`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckItemKind {
    /// A normal repo-declared check command.
    Check,
    /// A fixer probe executed in non-mutating mode.
    FixerProbe,
}

/// Outcome of an individual `check` item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckOutcome {
    /// The item passed.
    Pass,
    /// The item failed normally.
    Fail,
    /// The item exceeded its timeout.
    Timeout,
    /// A fixer probe reported that repair is needed.
    RepairNeeded,
}

/// Result for one executed check or fixer probe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckItemResult {
    /// Declared check or fixer name.
    pub name: String,
    /// Item kind.
    pub kind: CheckItemKind,
    /// Item outcome.
    pub outcome: CheckOutcome,
    /// Process exit code when one was available.
    pub exit_code: Option<i32>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Optional human-readable failure detail.
    pub message: Option<String>,
}

/// Aggregate counters for a `check` run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckSummary {
    /// Total executed items.
    pub total: usize,
    /// Passed item count.
    pub pass: usize,
    /// Failed item count.
    pub fail: usize,
    /// Timed out item count.
    pub timeout: usize,
    /// Fixer probes that reported repair-needed.
    pub repair_needed: usize,
}

/// Full result of running `repocert check`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckReport {
    /// Resolved repository/config paths.
    pub paths: LoadPaths,
    /// How the checks were selected.
    pub selection_mode: CheckSelectionMode,
    /// Evaluated profile names.
    pub profiles: Vec<String>,
    /// Effective check names that were executed.
    pub checks: Vec<String>,
    /// Per-item execution results.
    pub results: Vec<CheckItemResult>,
    /// Aggregate counters.
    pub summary: CheckSummary,
}

impl CheckReport {
    /// Returns `true` when no item failed, timed out, or reported repair-needed.
    pub fn ok(&self) -> bool {
        self.summary.fail == 0 && self.summary.timeout == 0 && self.summary.repair_needed == 0
    }
}
