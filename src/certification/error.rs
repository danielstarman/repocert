use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FingerprintError {
    #[error("protected contract path {path:?} must be a regular file")]
    ProtectedPathNotFile { path: PathBuf },
    #[error("failed to read protected contract path {path:?}")]
    ProtectedPathIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    GitMetadata(#[from] crate::git::GitCommonDirError),
    #[error("commit id {commit:?} must be non-empty lowercase or uppercase hex")]
    InvalidCommitId { commit: String },
    #[error("failed to read certification record {path:?}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse certification record {path:?}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("stored certification filename {path:?} is not a valid encoded profile")]
    InvalidStoredProfileName { path: PathBuf },
    #[error("stored certification record {path:?} does not match its derived key")]
    InvalidStoredRecordKey { path: PathBuf },
    #[error("failed to persist certification record {path:?}")]
    Persist {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}
