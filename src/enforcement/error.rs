use thiserror::Error;

use crate::certification::{FingerprintError, StorageError};
use crate::config::{LoadFailure, LoadPaths};
use crate::git::GitCommitError;

#[derive(Debug, Error)]
pub enum AuthorizeError {
    #[error(transparent)]
    Load(#[from] LoadFailure),
    #[error("ref-update deletions with an all-zero new OID are unsupported in v1")]
    UnsupportedDeletion { paths: LoadPaths },
    #[error("{error}")]
    GitCommit {
        paths: LoadPaths,
        #[source]
        error: GitCommitError,
    },
    #[error("{error}")]
    Fingerprint {
        paths: LoadPaths,
        #[source]
        error: FingerprintError,
    },
    #[error("{error}")]
    Storage {
        paths: LoadPaths,
        #[source]
        error: StorageError,
    },
    #[error("invalid protected-ref pattern {pattern:?}: {message}")]
    InvalidPattern {
        paths: LoadPaths,
        pattern: String,
        message: String,
    },
}

impl AuthorizeError {
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
