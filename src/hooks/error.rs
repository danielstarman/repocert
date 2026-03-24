use std::path::PathBuf;

use thiserror::Error;

use crate::config::{LoadFailure, LoadPaths};
use crate::git::{GitDirError, GitHooksPathError};

#[derive(Debug, Error)]
pub enum InstallHooksError {
    #[error(transparent)]
    Load(#[from] LoadFailure),
    #[error("hooks configuration is required to install hooks")]
    MissingHooksConfig { paths: LoadPaths },
    #[error("{error}")]
    GitHooksPath {
        paths: LoadPaths,
        #[source]
        error: GitHooksPathError,
    },
    #[error("{error}")]
    GitDir {
        paths: LoadPaths,
        #[source]
        error: GitDirError,
    },
    #[error("failed to determine the current repocert executable path")]
    CurrentExecutable {
        paths: Option<LoadPaths>,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write generated hook {hook:?} at {path:?}")]
    GeneratedHookWrite {
        paths: LoadPaths,
        hook: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to remove stale generated hook at {path:?}")]
    GeneratedHookPrune {
        paths: LoadPaths,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl InstallHooksError {
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::MissingHooksConfig { paths }
            | Self::GitHooksPath { paths, .. }
            | Self::GitDir { paths, .. }
            | Self::GeneratedHookWrite { paths, .. }
            | Self::GeneratedHookPrune { paths, .. } => Some(paths),
            Self::CurrentExecutable { paths, .. } => paths.as_ref(),
        }
    }
}
