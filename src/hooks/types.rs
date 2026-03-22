use std::path::PathBuf;

use crate::config::LoadOptions;

#[derive(Clone, Debug)]
pub struct InstallHooksOptions {
    pub load_options: LoadOptions,
    pub executable_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookInstallMode {
    RepoOwned,
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
