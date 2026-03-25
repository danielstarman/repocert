use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Canonical filesystem locations used while loading a contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadPaths {
    /// Canonical repository root.
    pub repo_root: PathBuf,
    /// Canonical config file path.
    pub config_path: PathBuf,
}

/// A fully loaded contract plus the exact config bytes used to produce it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedContract {
    /// Resolved repository/config paths.
    pub paths: LoadPaths,
    /// Raw config bytes used for fingerprinting.
    pub config_bytes: Vec<u8>,
    /// Validated contract model.
    pub contract: Contract,
}

/// Validated repository contract model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contract {
    /// Supported schema version declared by the config.
    pub schema_version: u64,
    /// Named checks available to profiles and direct execution.
    pub checks: BTreeMap<String, CommandSpec>,
    /// Named fixers available to profiles and direct execution.
    pub fixers: BTreeMap<String, FixerSpec>,
    /// Validated and flattened profiles keyed by name.
    pub profiles: BTreeMap<String, Profile>,
    /// Default profile name when one is designated.
    pub default_profile: Option<String>,
    /// The built-in protected contract directory, currently `.repocert`.
    pub built_in_protected_dir: RepoPath,
    /// Additional protected contract paths declared by the repo.
    pub declared_protected_paths: BTreeSet<RepoPath>,
    /// Protected ref rules keyed by glob pattern and required profile.
    pub protected_refs: Vec<ProtectedRef>,
    /// Optional certification authenticity configuration.
    pub certification: Option<CertificationConfig>,
    /// Optional local checkout policy enforced by generated commit hooks.
    pub local_policy: Option<LocalPolicy>,
    /// Optional git hook installation configuration.
    pub hooks: Option<HooksConfig>,
}

/// Opaque external command specification used for checks and fixers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    /// Executable and arguments to run.
    pub argv: Vec<String>,
    /// Extra environment variables supplied to the command.
    pub env: BTreeMap<String, String>,
    /// Optional timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

/// Mutating fixer declaration plus its non-mutating probe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixerSpec {
    /// Command used to perform the repair.
    pub command: CommandSpec,
    /// Optional command used to detect whether repair is needed.
    pub probe_argv: Option<Vec<String>>,
    /// Optional timeout for the probe in milliseconds.
    pub probe_timeout_ms: Option<u64>,
}

/// Validated profile definition with resolved effective members.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    /// Profile name.
    pub name: String,
    /// Checks declared directly on the profile.
    pub declared_checks: Vec<String>,
    /// Fixers declared directly on the profile.
    pub declared_fixers: Vec<String>,
    /// Included profiles declared directly on the profile.
    pub declared_includes: Vec<String>,
    /// Flattened effective check list after includes and deduplication.
    pub effective_checks: Vec<String>,
    /// Flattened effective fixer list after includes and deduplication.
    pub effective_fixers: Vec<String>,
    /// Whether this profile is the designated default.
    pub default: bool,
    /// Whether this profile may be used for certification.
    pub certify: bool,
}

/// Protected ref rule mapping a glob pattern to a certification profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtectedRef {
    /// Glob-like ref pattern, such as `refs/heads/main`.
    pub pattern: String,
    /// Certification-eligible profile required for matching refs.
    pub profile: String,
}

/// Certification authenticity configuration for the repository.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertificationConfig {
    /// Selected certification authenticity mode.
    pub mode: CertificationMode,
}

/// Supported certification authenticity modes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertificationMode {
    /// Require SSH-signed certification records verified against trusted signer keys.
    SshSigned {
        /// Repo-wide allowlist of trusted SSH public keys.
        trusted_signers: Vec<String>,
        /// Precomputed SHA-256 fingerprints for the trusted signer allowlist.
        trusted_signer_fingerprints: Vec<String>,
    },
}

/// Local checkout policy enforced by generated commit hooks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalPolicy {
    /// Branch patterns on which local commits are blocked.
    pub protected_branches: Vec<String>,
    /// Whether the primary checkout must remain clean.
    pub require_clean_primary_checkout: bool,
}

/// Hook installation configuration for the repository.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HooksConfig {
    /// Selected hook installation mode.
    pub mode: HookMode,
}

/// Supported hook installation modes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookMode {
    /// Generate and manage hook wrappers derived from contract semantics.
    Generated,
}

/// Normalized repository-relative path stored in the validated contract.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RepoPath(String);

impl RepoPath {
    /// Create a normalized repository-relative path wrapper.
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Borrow the normalized repository-relative path as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
