use std::path::PathBuf;

use thiserror::Error;

use crate::config::{LoadFailure, LoadPaths};
use crate::git::{GitDirError, GitHooksPathError};

/// Errors returned while running `repocert install-hooks`.
#[derive(Debug, Error)]
pub enum InstallHooksError {
    /// Contract discovery, parsing, or validation failed before installation.
    #[error(transparent)]
    Load(#[from] LoadFailure),
    /// The repository contract does not declare hook installation config.
    #[error("hooks configuration is required to install hooks")]
    MissingHooksConfig {
        /// Resolved repository/config paths.
        paths: LoadPaths,
    },
    /// Reading or writing git hook path config failed.
    #[error("{error}")]
    GitHooksPath {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying git config error.
        #[source]
        error: GitHooksPathError,
    },
    /// Resolving the per-worktree git dir failed.
    #[error("{error}")]
    GitDir {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying git dir resolution error.
        #[source]
        error: GitDirError,
    },
    /// Determining the current `repocert` executable path failed.
    #[error("failed to determine the current repocert executable path")]
    CurrentExecutable {
        /// Resolved repository/config paths, when available.
        paths: Option<LoadPaths>,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Writing a generated hook wrapper failed.
    #[error("failed to write generated hook {hook:?} at {path:?}")]
    GeneratedHookWrite {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Hook name or synthetic directory label.
        hook: String,
        /// Path that could not be written.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Removing a stale generated hook wrapper failed.
    #[error("failed to remove stale generated hook at {path:?}")]
    GeneratedHookPrune {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Path that could not be removed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

impl InstallHooksError {
    /// Return resolved paths when they were available for this failure.
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
