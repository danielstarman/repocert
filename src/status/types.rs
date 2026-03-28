use crate::certification::ContractFingerprint;
use crate::config::{LoadOptions, LoadPaths};

/// Options for running `repocert status`.
#[derive(Clone, Debug, Default)]
pub struct StatusOptions {
    /// Contract loading options for the target repository.
    pub load_options: LoadOptions,
    /// Optional explicit commit to inspect instead of `HEAD`.
    pub commit: Option<String>,
    /// Optional profile names to inspect.
    pub profiles: Vec<String>,
    /// Whether the result should fail unless all selected profiles are certified.
    pub assert_certified: bool,
}

/// Certification state for one profile on the inspected commit.
///
/// Only [`StatusProfileState::Certified`] satisfies certification requirements.
/// All other states are explanatory non-certification states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatusProfileState {
    /// The commit is certified for the profile under the current fingerprint.
    Certified,
    /// A signed certification exists, but the signer is not trusted by the repo.
    UntrustedSigner,
    /// A signed certification exists, but the signature does not verify.
    InvalidSignature,
    /// A certification exists for the profile, but on a different commit.
    StaleCommit,
    /// A certification exists for the commit/profile, but under a different fingerprint.
    StaleFingerprint,
    /// No certification exists for the commit/profile.
    Uncertified,
}

/// Status result for one profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusProfileResult {
    /// Profile name.
    pub profile: String,
    /// Certification state for the profile.
    pub state: StatusProfileState,
    /// Repo-trusted signer name for a matching signed record, when available.
    pub signer_name: Option<String>,
    /// Other commits that are certified for the same profile.
    pub other_certified_commits: Vec<String>,
    /// Recorded fingerprint for the matching record, if one exists.
    pub recorded_fingerprint: Option<ContractFingerprint>,
}

/// Protected ref status derived from the inspected commit and current rules.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtectedRefStatus {
    /// Protected ref pattern.
    pub pattern: String,
    /// Required certification profile.
    pub profile: String,
    /// Whether the inspected commit satisfies the protected ref rule.
    pub certified: bool,
}

/// Aggregate counters for a `status` run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusSummary {
    /// Total inspected profiles.
    pub total_profiles: usize,
    /// Certified profile count.
    pub certified: usize,
    /// Untrusted signer profile count.
    pub untrusted_signer: usize,
    /// Invalid signature profile count.
    pub invalid_signature: usize,
    /// Stale-commit profile count.
    pub stale_commit: usize,
    /// Stale-fingerprint profile count.
    pub stale_fingerprint: usize,
    /// Uncertified profile count.
    pub uncertified: usize,
}

/// Full result of running `repocert status`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusReport {
    /// Resolved repository/config paths.
    pub paths: LoadPaths,
    /// Inspected commit SHA.
    pub commit: String,
    /// Current contract fingerprint.
    pub contract_fingerprint: ContractFingerprint,
    /// Requested or inferred profile names.
    pub profiles: Vec<String>,
    /// Per-profile certification results.
    pub profile_results: Vec<StatusProfileResult>,
    /// Current protected ref rule status for the inspected commit.
    pub protected_refs: Vec<ProtectedRefStatus>,
    /// Whether uncertified profiles should make the result fail.
    pub assert_certified: bool,
    /// Aggregate counters.
    pub summary: StatusSummary,
}

impl StatusReport {
    /// Returns `true` unless `assert_certified` is set and some profile is not certified.
    pub fn ok(&self) -> bool {
        !self.assert_certified || self.summary.certified == self.summary.total_profiles
    }
}
