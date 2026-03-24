use crate::certification::ContractFingerprint;
use crate::config::{LoadOptions, LoadPaths};

/// Options for running `repocert certify`.
#[derive(Clone, Debug, Default)]
pub struct CertifyOptions {
    /// Contract loading options for the target repository.
    pub load_options: LoadOptions,
    /// Certification-eligible profile names to run.
    pub profiles: Vec<String>,
    /// Whether to emit human progress lines during execution.
    pub emit_progress: bool,
}

/// Kind of item executed during `certify`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertifyItemKind {
    /// A normal repo-declared check command.
    Check,
    /// A fixer probe executed in non-mutating mode.
    FixerProbe,
}

/// Outcome of an individual `certify` item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertifyItemOutcome {
    /// The item passed.
    Pass,
    /// The item failed normally.
    Fail,
    /// The item exceeded its timeout.
    Timeout,
    /// A fixer probe reported that repair is needed.
    RepairNeeded,
}

/// Result for one executed check or fixer probe during certification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifyItemResult {
    /// Declared check or fixer name.
    pub name: String,
    /// Item kind.
    pub kind: CertifyItemKind,
    /// Item outcome.
    pub outcome: CertifyItemOutcome,
    /// Process exit code when one was available.
    pub exit_code: Option<i32>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Optional human-readable failure detail.
    pub message: Option<String>,
}

/// Outcome of a profile certification attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertifyProfileOutcome {
    /// The profile was certified successfully.
    Certified,
    /// One or more items failed.
    Failed,
    /// One or more items timed out.
    TimedOut,
    /// A fixer probe reported that repair is needed.
    RepairNeeded,
}

/// Result for one certified profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifyProfileResult {
    /// Profile name.
    pub profile: String,
    /// Effective check names for the profile.
    pub checks: Vec<String>,
    /// Per-item execution results.
    pub item_results: Vec<CertifyItemResult>,
    /// Overall profile outcome.
    pub outcome: CertifyProfileOutcome,
    /// Whether a certification record was written.
    pub record_written: bool,
}

/// Aggregate counters for a `certify` run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifySummary {
    /// Total profiles attempted.
    pub total_profiles: usize,
    /// Successfully certified profile count.
    pub certified: usize,
    /// Failed profile count.
    pub failed: usize,
    /// Timed-out profile count.
    pub timeout: usize,
    /// Repair-needed profile count.
    pub repair_needed: usize,
}

/// Full result of running `repocert certify`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifyReport {
    /// Resolved repository/config paths.
    pub paths: LoadPaths,
    /// Certified commit SHA.
    pub commit: String,
    /// Contract fingerprint used for certification.
    pub contract_fingerprint: ContractFingerprint,
    /// Requested profile names.
    pub profiles: Vec<String>,
    /// Per-profile certification results.
    pub profile_results: Vec<CertifyProfileResult>,
    /// Aggregate counters.
    pub summary: CertifySummary,
}

impl CertifyReport {
    /// Returns `true` when all requested profiles were certified.
    pub fn ok(&self) -> bool {
        self.summary.failed == 0 && self.summary.timeout == 0 && self.summary.repair_needed == 0
    }
}
