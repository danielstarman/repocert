use crate::certification::ContractFingerprint;
use crate::config::LoadOptions;

#[derive(Clone, Debug)]
pub struct AuthorizeOptions {
    pub load_options: LoadOptions,
    pub old: String,
    pub new: String,
    pub reference: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchedRule {
    pub pattern: String,
    pub profile: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorizeProfileState {
    Certified,
    StaleCommit,
    StaleFingerprint,
    Uncertified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizeProfileResult {
    pub profile: String,
    pub state: AuthorizeProfileState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizeReport {
    pub paths: crate::config::LoadPaths,
    pub old: String,
    pub new: String,
    pub reference: String,
    pub target_commit: String,
    pub contract_fingerprint: ContractFingerprint,
    pub matched_rules: Vec<MatchedRule>,
    pub required_profiles: Vec<String>,
    pub profile_results: Vec<AuthorizeProfileResult>,
    pub allowed: bool,
}

impl AuthorizeReport {
    pub fn ok(&self) -> bool {
        self.allowed
    }
}
