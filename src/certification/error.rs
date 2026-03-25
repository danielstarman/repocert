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
pub enum SigningError {
    #[error("ssh signing key path {path:?} does not exist or is not a file")]
    MissingSigningKey { path: PathBuf },
    #[error("signed certification record version {version} is unsupported")]
    UnsupportedRecordVersion { version: u64 },
    #[error("failed to create temporary signing files")]
    TempFile {
        #[source]
        source: io::Error,
    },
    #[error("failed to run ssh-keygen for signing or verification")]
    Io {
        #[source]
        source: io::Error,
    },
    #[error("ssh-keygen failed: {message}")]
    CommandFailed { message: String },
    #[error("ssh-keygen output did not contain a SHA256 fingerprint")]
    MissingFingerprint,
    #[error("trusted signer public key {index} is invalid")]
    InvalidTrustedSigner { index: usize },
    #[error("trusted signer fingerprint {fingerprint} is not allowed by the repository contract")]
    UntrustedSigner { fingerprint: String },
    #[error("signature verification failed for signer {fingerprint}")]
    InvalidSignature { fingerprint: String },
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    GitMetadata(#[from] crate::git::GitCommonDirError),
    #[error(transparent)]
    Signing(#[from] SigningError),
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
