use crate::certification::ContractFingerprint;
use crate::config::{LoadOptions, LoadPaths};

#[derive(Clone, Debug, Default)]
pub struct CertifyOptions {
    pub load_options: LoadOptions,
    pub profiles: Vec<String>,
    pub emit_progress: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertifyItemKind {
    Check,
    FixerProbe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertifyItemOutcome {
    Pass,
    Fail,
    Timeout,
    RepairNeeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifyItemResult {
    pub name: String,
    pub kind: CertifyItemKind,
    pub outcome: CertifyItemOutcome,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertifyProfileOutcome {
    Certified,
    Failed,
    TimedOut,
    RepairNeeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifyProfileResult {
    pub profile: String,
    pub checks: Vec<String>,
    pub item_results: Vec<CertifyItemResult>,
    pub outcome: CertifyProfileOutcome,
    pub record_written: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifySummary {
    pub total_profiles: usize,
    pub certified: usize,
    pub failed: usize,
    pub timeout: usize,
    pub repair_needed: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifyReport {
    pub paths: LoadPaths,
    pub commit: String,
    pub contract_fingerprint: ContractFingerprint,
    pub profiles: Vec<String>,
    pub profile_results: Vec<CertifyProfileResult>,
    pub summary: CertifySummary,
}

impl CertifyReport {
    pub fn ok(&self) -> bool {
        self.summary.failed == 0 && self.summary.timeout == 0 && self.summary.repair_needed == 0
    }
}
