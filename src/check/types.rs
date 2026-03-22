use crate::config::{LoadOptions, LoadPaths};

#[derive(Clone, Debug, Default)]
pub struct CheckOptions {
    pub load_options: LoadOptions,
    pub profiles: Vec<String>,
    pub names: Vec<String>,
    pub emit_progress: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckSelectionMode {
    Profiles,
    Checks,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckItemKind {
    Check,
    FixerProbe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckOutcome {
    Pass,
    Fail,
    Timeout,
    RepairNeeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckItemResult {
    pub name: String,
    pub kind: CheckItemKind,
    pub outcome: CheckOutcome,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckSummary {
    pub total: usize,
    pub pass: usize,
    pub fail: usize,
    pub timeout: usize,
    pub repair_needed: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckReport {
    pub paths: LoadPaths,
    pub selection_mode: CheckSelectionMode,
    pub profiles: Vec<String>,
    pub checks: Vec<String>,
    pub results: Vec<CheckItemResult>,
    pub summary: CheckSummary,
}

impl CheckReport {
    pub fn ok(&self) -> bool {
        self.summary.fail == 0 && self.summary.timeout == 0 && self.summary.repair_needed == 0
    }
}
