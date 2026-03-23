use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadPaths {
    pub repo_root: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedContract {
    pub paths: LoadPaths,
    pub config_bytes: Vec<u8>,
    pub contract: Contract,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contract {
    pub schema_version: u64,
    pub checks: BTreeMap<String, CommandSpec>,
    pub fixers: BTreeMap<String, FixerSpec>,
    pub profiles: BTreeMap<String, Profile>,
    pub default_profile: Option<String>,
    pub built_in_protected_dir: RepoPath,
    pub declared_protected_paths: BTreeSet<RepoPath>,
    pub protected_refs: Vec<ProtectedRef>,
    pub local_policy: Option<LocalPolicy>,
    pub hooks: Option<HooksConfig>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub argv: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixerSpec {
    pub command: CommandSpec,
    pub probe_argv: Option<Vec<String>>,
    pub probe_timeout_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    pub name: String,
    pub declared_checks: Vec<String>,
    pub declared_fixers: Vec<String>,
    pub declared_includes: Vec<String>,
    pub effective_checks: Vec<String>,
    pub effective_fixers: Vec<String>,
    pub default: bool,
    pub certify: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtectedRef {
    pub pattern: String,
    pub profile: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalPolicy {
    pub protected_branches: Vec<String>,
    pub require_clean_primary_checkout: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HooksConfig {
    pub mode: HookMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookMode {
    RepoOwned { path: RepoPath },
    Generated { hooks: Vec<String> },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RepoPath(String);

impl RepoPath {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
