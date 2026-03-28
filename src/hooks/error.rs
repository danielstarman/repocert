use std::path::PathBuf;

use thiserror::Error;

use crate::git::{GitDirError, GitHooksPathError};

/// Errors returned while running `repocert install-hooks`.
#[derive(Debug, Error)]
pub enum InstallHooksError {
    /// The repository contract does not declare hook installation config.
    #[error("hooks configuration is required to install hooks")]
    MissingHooksConfig,
    /// Reading or writing git hook path config failed.
    #[error(transparent)]
    GitHooksPath(#[from] GitHooksPathError),
    /// Resolving the per-worktree git dir failed.
    #[error(transparent)]
    GitDir(#[from] GitDirError),
    /// Writing a generated hook wrapper failed.
    #[error("failed to write generated hook {hook:?} at {path:?}")]
    GeneratedHookWrite {
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
        /// Path that could not be removed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}
