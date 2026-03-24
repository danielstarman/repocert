use std::path::PathBuf;

use crate::config::LoadOptions;

/// Generated git hook entrypoints understood by `repocert`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GeneratedHook {
    /// Commit hook used for local checkout policy enforcement.
    PreCommit,
    /// Merge-commit hook used for local checkout policy enforcement.
    PreMergeCommit,
    /// Push hook used for protected ref authorization.
    PrePush,
    /// Server-style update hook used for protected ref authorization.
    Update,
}

impl GeneratedHook {
    /// Parse a git hook name into a generated hook variant.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pre-commit" => Some(Self::PreCommit),
            "pre-merge-commit" => Some(Self::PreMergeCommit),
            "pre-push" => Some(Self::PrePush),
            "update" => Some(Self::Update),
            _ => None,
        }
    }

    /// Return the canonical git hook name for this generated hook.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreCommit => "pre-commit",
            Self::PreMergeCommit => "pre-merge-commit",
            Self::PrePush => "pre-push",
            Self::Update => "update",
        }
    }
}

/// Options for generated hook installation.
#[derive(Clone, Debug)]
pub struct InstallHooksOptions {
    /// Contract loading options for the target repository.
    pub load_options: LoadOptions,
    /// Absolute path to the current `repocert` executable.
    pub executable_path: PathBuf,
}

/// Installed hook mode reported by [`InstallHooksReport`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookInstallMode {
    /// Generated wrappers managed by `repocert`.
    Generated,
}

/// Result of a successful `install-hooks` run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallHooksReport {
    /// Resolved repository/config paths.
    pub paths: crate::config::LoadPaths,
    /// Installed hook mode.
    pub mode: HookInstallMode,
    /// Effective `core.hooksPath` target after installation.
    pub hooks_path: PathBuf,
    /// Whether any files or git config entries were changed.
    pub changed: bool,
    /// Human-readable list of repaired/generated items.
    pub repaired_items: Vec<String>,
}

impl InstallHooksReport {
    /// Returns `true` for any successful installation report.
    pub fn ok(&self) -> bool {
        true
    }
}
