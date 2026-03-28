use thiserror::Error;

use crate::git::{GitCheckoutError, GitWorktreeError};

/// Errors returned while checking local checkout policy.
#[derive(Debug, Error)]
pub enum LocalPolicyError {
    /// Inspecting checkout identity or branch information failed.
    #[error(transparent)]
    GitCheckout(#[from] GitCheckoutError),
    /// Capturing worktree dirtiness failed.
    #[error(transparent)]
    GitWorktree(#[from] GitWorktreeError),
    /// A configured protected-branch pattern was invalid.
    #[error("invalid local protected branch pattern {pattern:?}: {message}")]
    InvalidPattern {
        /// Invalid configured pattern.
        pattern: String,
        /// Pattern validation error detail.
        message: String,
    },
}
