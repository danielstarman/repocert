use crate::config::{LoadOptions, LoadPaths};

/// Options for running `repocert fix`.
#[derive(Clone, Debug, Default)]
pub struct FixOptions {
    /// Contract loading options for the target repository.
    pub load_options: LoadOptions,
    /// Optional profile whose fixers should be executed.
    pub profile: Option<String>,
    /// Direct named fixers to execute instead of a profile.
    pub names: Vec<String>,
    /// Whether to emit human progress lines during execution.
    pub emit_progress: bool,
}

/// How fixers were selected for a [`FixReport`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FixSelectionMode {
    /// Fixers came from a selected profile.
    Profile,
    /// Fixers were selected directly by name.
    Fixers,
}

/// Outcome of an individual fixer execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FixOutcome {
    /// The fixer completed successfully.
    Pass,
    /// The fixer failed normally.
    Fail,
    /// The fixer exceeded its timeout.
    Timeout,
}

/// Result for one executed fixer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixItemResult {
    /// Declared fixer name.
    pub name: String,
    /// Fixer outcome.
    pub outcome: FixOutcome,
    /// Process exit code when one was available.
    pub exit_code: Option<i32>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Optional human-readable failure detail.
    pub message: Option<String>,
}

/// Aggregate counters for a `fix` run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixSummary {
    /// Total executed fixers.
    pub total: usize,
    /// Passed fixer count.
    pub pass: usize,
    /// Failed fixer count.
    pub fail: usize,
    /// Timed out fixer count.
    pub timeout: usize,
}

/// Full result of running `repocert fix`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixReport {
    /// Resolved repository/config paths.
    pub paths: LoadPaths,
    /// How fixers were selected.
    pub selection_mode: FixSelectionMode,
    /// Selected profile name, when one was used.
    pub profile: Option<String>,
    /// Effective fixer names that were executed.
    pub fixers: Vec<String>,
    /// Per-fixer execution results.
    pub results: Vec<FixItemResult>,
    /// Aggregate counters.
    pub summary: FixSummary,
}

impl FixReport {
    /// Returns `true` when no fixer failed or timed out.
    pub fn ok(&self) -> bool {
        self.summary.fail == 0 && self.summary.timeout == 0
    }
}
