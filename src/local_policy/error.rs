use thiserror::Error;

use crate::config::{LoadFailure, LoadPaths};
use crate::git::{GitCheckoutError, GitWorktreeError};

#[derive(Debug, Error)]
pub enum LocalPolicyError {
    #[error(transparent)]
    Load(#[from] LoadFailure),
    #[error("{error}")]
    GitCheckout {
        paths: LoadPaths,
        #[source]
        error: GitCheckoutError,
    },
    #[error("{error}")]
    GitWorktree {
        paths: LoadPaths,
        #[source]
        error: GitWorktreeError,
    },
    #[error("invalid local protected branch pattern {pattern:?}: {message}")]
    InvalidPattern {
        paths: LoadPaths,
        pattern: String,
        message: String,
    },
}

impl LocalPolicyError {
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::GitCheckout { paths, .. }
            | Self::GitWorktree { paths, .. }
            | Self::InvalidPattern { paths, .. } => Some(paths),
        }
    }
}
