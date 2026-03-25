use thiserror::Error;

use crate::config::{LoadFailure, LoadPaths};
use crate::git::{GitCheckoutError, GitWorktreeError};

/// Errors returned while checking local checkout policy.
#[derive(Debug, Error)]
pub enum LocalPolicyError {
    /// Contract discovery, parsing, or validation failed before policy checks ran.
    #[error(transparent)]
    Load(#[from] LoadFailure),
    /// Inspecting checkout identity or branch information failed.
    #[error("{error}")]
    GitCheckout {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying checkout inspection error.
        #[source]
        error: GitCheckoutError,
    },
    /// Capturing worktree dirtiness failed.
    #[error("{error}")]
    GitWorktree {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying worktree snapshot error.
        #[source]
        error: GitWorktreeError,
    },
    /// A configured protected-branch pattern was invalid.
    #[error("invalid local protected branch pattern {pattern:?}: {message}")]
    InvalidPattern {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Invalid configured pattern.
        pattern: String,
        /// Pattern validation error detail.
        message: String,
    },
}

impl LocalPolicyError {
    /// Return resolved paths when they were available for this failure.
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::GitCheckout { paths, .. }
            | Self::GitWorktree { paths, .. }
            | Self::InvalidPattern { paths, .. } => Some(paths),
        }
    }
}
