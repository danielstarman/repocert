use std::path::PathBuf;

use crate::config::LoadOptions;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GeneratedHook {
    PreCommit,
    PreMergeCommit,
    PrePush,
    Update,
}

impl GeneratedHook {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pre-commit" => Some(Self::PreCommit),
            "pre-merge-commit" => Some(Self::PreMergeCommit),
            "pre-push" => Some(Self::PrePush),
            "update" => Some(Self::Update),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreCommit => "pre-commit",
            Self::PreMergeCommit => "pre-merge-commit",
            Self::PrePush => "pre-push",
            Self::Update => "update",
        }
    }
}

#[derive(Clone, Debug)]
pub struct InstallHooksOptions {
    pub load_options: LoadOptions,
    pub executable_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookInstallMode {
    Generated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallHooksReport {
    pub paths: crate::config::LoadPaths,
    pub mode: HookInstallMode,
    pub hooks_path: PathBuf,
    pub changed: bool,
    pub repaired_items: Vec<String>,
}

impl InstallHooksReport {
    pub fn ok(&self) -> bool {
        true
    }
}
