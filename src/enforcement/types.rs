use crate::certification::ContractFingerprint;

/// Options for authorizing one proposed ref update.
#[derive(Clone, Debug)]
pub struct AuthorizeOptions {
    /// Old object id from the ref update request.
    pub old: String,
    /// New object id from the ref update request.
    pub new: String,
    /// Fully qualified ref name being updated.
    pub reference: String,
}

/// One protected ref rule that matched the requested update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchedRule {
    /// Matched protected ref pattern.
    pub pattern: String,
    /// Certification profile required by the rule.
    pub profile: String,
}

/// Certification state for a required profile during authorization.
///
/// Only [`AuthorizeProfileState::Certified`] allows a protected ref update.
/// All other states are explanatory deny states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorizeProfileState {
    /// The target commit is certified for the profile.
    Certified,
    /// A signed certification exists, but the signer is not trusted by the repo.
    UntrustedSigner,
    /// A signed certification exists, but the signature does not verify.
    InvalidSignature,
    /// A certification exists for the profile, but on a different commit.
    StaleCommit,
    /// A certification exists for the commit/profile, but under a different fingerprint.
    StaleFingerprint,
    /// No matching certification exists.
    Uncertified,
}

/// Authorization result for one required profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizeProfileResult {
    /// Profile name.
    pub profile: String,
    /// Certification state for the profile.
    pub state: AuthorizeProfileState,
    /// Repo-trusted signer name for a matching signed record, when available.
    pub signer_name: Option<String>,
}

/// Full decision for one attempted ref update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizeReport {
    /// Old object id from the input request.
    pub old: String,
    /// New object id from the input request.
    pub new: String,
    /// Fully qualified ref name being updated.
    pub reference: String,
    /// Commit that must be authorized for the update to proceed.
    pub target_commit: String,
    /// Current contract fingerprint.
    pub contract_fingerprint: ContractFingerprint,
    /// Protected ref rules that matched the update.
    pub matched_rules: Vec<MatchedRule>,
    /// Distinct required profiles derived from the matched rules.
    pub required_profiles: Vec<String>,
    /// Per-profile certification states.
    pub profile_results: Vec<AuthorizeProfileResult>,
    /// Whether the ref update is allowed.
    pub allowed: bool,
}

impl AuthorizeReport {
    /// Returns `true` when the update is allowed.
    pub fn ok(&self) -> bool {
        self.allowed
    }
}
