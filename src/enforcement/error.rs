use thiserror::Error;

use crate::certification::{FingerprintError, StorageError};
use crate::config::{LoadFailure, LoadPaths};
use crate::git::GitCommitError;

/// Errors returned while authorizing a ref update.
#[derive(Debug, Error)]
pub enum AuthorizeError {
    /// Contract discovery, parsing, or validation failed before authorization.
    #[error(transparent)]
    Load(#[from] LoadFailure),
    /// The requested update is a deletion, which `repocert` does not authorize in v1.
    #[error("ref-update deletions with an all-zero new OID are unsupported in v1")]
    UnsupportedDeletion {
        /// Resolved repository/config paths.
        paths: LoadPaths,
    },
    /// Resolving the target commit or checking referenced commit objects failed.
    #[error("{error}")]
    GitCommit {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying git commit lookup error.
        #[source]
        error: GitCommitError,
    },
    /// Computing the current contract fingerprint failed.
    #[error("{error}")]
    Fingerprint {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying fingerprinting error.
        #[source]
        error: FingerprintError,
    },
    /// Reading or verifying certification storage failed.
    #[error("{error}")]
    Storage {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying storage or signing error.
        #[source]
        error: StorageError,
    },
    /// A configured protected-ref pattern was invalid.
    #[error("invalid protected-ref pattern {pattern:?}: {message}")]
    InvalidPattern {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Invalid configured pattern.
        pattern: String,
        /// Pattern validation error detail.
        message: String,
    },
}

impl AuthorizeError {
    /// Return resolved paths when they were available for this failure.
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::UnsupportedDeletion { paths }
            | Self::GitCommit { paths, .. }
            | Self::Fingerprint { paths, .. }
            | Self::Storage { paths, .. }
            | Self::InvalidPattern { paths, .. } => Some(paths),
        }
    }
}
