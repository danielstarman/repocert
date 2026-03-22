use crate::certification::ContractFingerprint;
use crate::config::{LoadOptions, LoadPaths};

#[derive(Clone, Debug, Default)]
pub struct StatusOptions {
    pub load_options: LoadOptions,
    pub commit: Option<String>,
    pub profiles: Vec<String>,
    pub assert_certified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatusProfileState {
    Certified,
    StaleCommit,
    StaleFingerprint,
    Uncertified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusProfileResult {
    pub profile: String,
    pub state: StatusProfileState,
    pub other_certified_commits: Vec<String>,
    pub recorded_fingerprint: Option<ContractFingerprint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtectedRefStatus {
    pub pattern: String,
    pub profile: String,
    pub certified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusSummary {
    pub total_profiles: usize,
    pub certified: usize,
    pub stale_commit: usize,
    pub stale_fingerprint: usize,
    pub uncertified: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusReport {
    pub paths: LoadPaths,
    pub commit: String,
    pub contract_fingerprint: ContractFingerprint,
    pub profiles: Vec<String>,
    pub profile_results: Vec<StatusProfileResult>,
    pub protected_refs: Vec<ProtectedRefStatus>,
    pub assert_certified: bool,
    pub summary: StatusSummary,
}

impl StatusReport {
    pub fn ok(&self) -> bool {
        !self.assert_certified || self.summary.certified == self.summary.total_profiles
    }
}
