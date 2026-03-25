use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Errors produced while computing a contract fingerprint.
#[derive(Debug, Error)]
pub enum FingerprintError {
    /// A protected contract path resolved to something other than a regular file.
    #[error("protected contract path {path:?} must be a regular file")]
    ProtectedPathNotFile {
        /// Path that resolved to a non-file entry.
        path: PathBuf,
    },
    /// Reading a protected contract file failed.
    #[error("failed to read protected contract path {path:?}")]
    ProtectedPathIo {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
}

/// Errors produced while signing or verifying authenticated certification records.
#[derive(Debug, Error)]
pub enum SigningError {
    /// The configured SSH public key path does not point to a readable file.
    #[error("ssh signing key path {path:?} does not exist or is not a file")]
    MissingSigningKey {
        /// SSH public key path that could not be used.
        path: PathBuf,
    },
    /// The signed certification record version is not recognized by this binary.
    #[error("signed certification record version {version} is unsupported")]
    UnsupportedRecordVersion {
        /// Unrecognized version number found in the stored record.
        version: u64,
    },
    /// Temporary file creation or writes for SSH signing/verification failed.
    #[error("failed to create temporary signing files")]
    TempFile {
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// Launching or waiting on `ssh-keygen` failed.
    #[error("failed to run ssh-keygen for signing or verification")]
    Io {
        /// Underlying process I/O error.
        #[source]
        source: io::Error,
    },
    /// `ssh-keygen` exited unsuccessfully.
    #[error("ssh-keygen failed: {message}")]
    CommandFailed {
        /// Best-effort stderr/stdout message from the failed command.
        message: String,
    },
    /// SSH key fingerprint output did not contain the expected SHA-256 form.
    #[error("ssh-keygen output did not contain a SHA256 fingerprint")]
    MissingFingerprint,
    /// A trusted signer entry in repo config is not a valid SSH public key.
    #[error("trusted signer public key {index} is invalid")]
    InvalidTrustedSigner {
        /// Index of the invalid trusted signer entry in repo config.
        index: usize,
    },
    /// The record signer fingerprint does not match any repo-trusted signer.
    #[error("trusted signer fingerprint {fingerprint} is not allowed by the repository contract")]
    UntrustedSigner {
        /// Signer fingerprint found in the record.
        fingerprint: String,
    },
    /// The signature did not verify for the signed payload.
    #[error("signature verification failed for signer {fingerprint}")]
    InvalidSignature {
        /// Signer fingerprint associated with the failed verification.
        fingerprint: String,
    },
}

/// Errors produced while reading or writing certification storage.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Resolving the shared git metadata directory failed.
    #[error(transparent)]
    GitMetadata(#[from] crate::git::GitCommonDirError),
    /// Authentication-related signing or verification failed.
    #[error(transparent)]
    Signing(#[from] SigningError),
    /// The requested commit identifier is not a valid hex object id string.
    #[error("commit id {commit:?} must be non-empty lowercase or uppercase hex")]
    InvalidCommitId {
        /// Invalid commit identifier supplied to the store.
        commit: String,
    },
    /// Reading a certification record from disk failed.
    #[error("failed to read certification record {path:?}")]
    Io {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// Parsing a stored certification record failed.
    #[error("failed to parse certification record {path:?}")]
    Json {
        /// Path that could not be parsed.
        path: PathBuf,
        /// Underlying JSON parse error.
        #[source]
        source: serde_json::Error,
    },
    /// A stored profile filename could not be decoded back into a profile name.
    #[error("stored certification filename {path:?} is not a valid encoded profile")]
    InvalidStoredProfileName {
        /// Path whose filename could not be decoded into a profile.
        path: PathBuf,
    },
    /// A stored certification record does not match its derived `(commit, profile)` key.
    #[error("stored certification record {path:?} does not match its derived key")]
    InvalidStoredRecordKey {
        /// Path whose stored record key did not match its location-derived key.
        path: PathBuf,
    },
    /// Atomically persisting a certification record failed.
    #[error("failed to persist certification record {path:?}")]
    Persist {
        /// Target path that could not be persisted.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
}
