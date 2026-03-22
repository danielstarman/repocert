mod discovery;
mod error;
mod loader;
mod model;
mod raw;
mod validate;

use std::path::PathBuf;

pub use error::{
    DiscoveryError, LoadError, ParseError, ValidationErrorKind, ValidationErrors, ValidationIssue,
};
pub use loader::load_contract;
pub use model::{
    CommandSpec, Contract, FixerSpec, HookMode, HooksConfig, LoadedContract, Profile, ProtectedRef,
    RepoPath,
};

#[derive(Clone, Debug, Default)]
pub struct LoadOptions {
    pub start_dir: Option<PathBuf>,
    pub repo_root: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
}

impl LoadOptions {
    pub fn discover_from(path: impl Into<PathBuf>) -> Self {
        Self {
            start_dir: Some(path.into()),
            repo_root: None,
            config_path: None,
        }
    }

    pub fn from_repo_root(path: impl Into<PathBuf>) -> Self {
        Self {
            start_dir: None,
            repo_root: Some(path.into()),
            config_path: None,
        }
    }

    pub fn from_config_path(path: impl Into<PathBuf>) -> Self {
        Self {
            start_dir: None,
            repo_root: None,
            config_path: Some(path.into()),
        }
    }
}
