use crate::config::{LoadOptions, LoadPaths};

#[derive(Clone, Debug, Default)]
pub struct FixOptions {
    pub load_options: LoadOptions,
    pub profile: Option<String>,
    pub names: Vec<String>,
    pub emit_progress: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FixSelectionMode {
    Profile,
    Fixers,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FixOutcome {
    Pass,
    Fail,
    Timeout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixItemResult {
    pub name: String,
    pub outcome: FixOutcome,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixSummary {
    pub total: usize,
    pub pass: usize,
    pub fail: usize,
    pub timeout: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixReport {
    pub paths: LoadPaths,
    pub selection_mode: FixSelectionMode,
    pub profile: Option<String>,
    pub fixers: Vec<String>,
    pub results: Vec<FixItemResult>,
    pub summary: FixSummary,
}

impl FixReport {
    pub fn ok(&self) -> bool {
        self.summary.fail == 0 && self.summary.timeout == 0
    }
}
