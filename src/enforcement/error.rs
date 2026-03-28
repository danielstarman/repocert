use thiserror::Error;

use crate::certification::{FingerprintError, ProfileCertificationError, StorageError};
use crate::git::GitCommitError;

/// Errors returned while authorizing a ref update.
#[derive(Debug, Error)]
pub enum AuthorizeError {
    /// The requested update is a deletion, which `repocert` does not authorize in v1.
    #[error("ref-update deletions with an all-zero new OID are unsupported in v1")]
    UnsupportedDeletion,
    /// Resolving the target commit or checking referenced commit objects failed.
    #[error(transparent)]
    GitCommit(#[from] GitCommitError),
    /// Computing the current contract fingerprint failed.
    #[error(transparent)]
    Fingerprint(#[from] FingerprintError),
    /// Reading or verifying certification storage failed.
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// A configured protected-ref pattern was invalid.
    #[error("invalid protected-ref pattern {pattern:?}: {message}")]
    InvalidPattern {
        /// Invalid configured pattern.
        pattern: String,
        /// Pattern validation error detail.
        message: String,
    },
}

impl From<ProfileCertificationError> for AuthorizeError {
    fn from(error: ProfileCertificationError) -> Self {
        match error {
            ProfileCertificationError::Storage(error) => Self::Storage(error),
            ProfileCertificationError::GitCommit(error) => Self::GitCommit(error),
        }
    }
}
