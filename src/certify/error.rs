use thiserror::Error;

use std::path::PathBuf;

use crate::certification::{FingerprintError, SigningError, StorageError};
use crate::config::{LoadFailure, LoadPaths};
use crate::contract::SelectionError;
use crate::git::{GitCommitError, GitWorktreeError};

/// Errors returned while running `repocert certify`.
#[derive(Debug, Error)]
pub enum CertifyError {
    /// Contract discovery, parsing, or validation failed before certification ran.
    #[error(transparent)]
    Load(#[from] LoadFailure),
    /// Profile selection for certification failed.
    #[error("{error}")]
    Selection {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying profile selection error.
        #[source]
        error: CertifySelectionError,
    },
    /// The worktree was dirty when certification required a clean checkout.
    #[error("worktree must be clean before certification; dirty path(s): {dirty_paths}")]
    DirtyWorktree {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Dirty paths visible in the worktree snapshot.
        dirty_paths: String,
    },
    /// Capturing worktree state failed.
    #[error("{error}")]
    GitStatus {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying git worktree error.
        #[source]
        error: GitWorktreeError,
    },
    /// Resolving the commit to certify failed.
    #[error("{error}")]
    GitCommit {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying git commit resolution error.
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
    /// Authenticated certification required a local signing key, but none was selected.
    #[error(
        "authenticated certification requires a local signing key; pass --signing-key or set REPOCERT_SIGNING_KEY"
    )]
    MissingSigningKeySelection {
        /// Resolved repository/config paths.
        paths: LoadPaths,
    },
    /// SSH signing or signed-record verification failed during certification.
    #[error("{error}")]
    Signing {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Local public-key path used for signing.
        signing_key: PathBuf,
        /// Underlying signing or verification error.
        #[source]
        error: SigningError,
    },
    /// Reading or writing certification storage failed.
    #[error("{error}")]
    Storage {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying storage error.
        #[source]
        error: StorageError,
    },
}

impl CertifyError {
    /// Return resolved paths when they were available for this failure.
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::Selection { paths, .. }
            | Self::DirtyWorktree { paths, .. }
            | Self::GitStatus { paths, .. }
            | Self::GitCommit { paths, .. }
            | Self::Fingerprint { paths, .. }
            | Self::MissingSigningKeySelection { paths }
            | Self::Signing { paths, .. }
            | Self::Storage { paths, .. } => Some(paths),
        }
    }
}

/// Profile-selection errors specific to `repocert certify`.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CertifySelectionError {
    /// No explicit profile was selected and no default certification profile exists.
    #[error(
        "no profile selector was provided and no implicit or explicit default profile is available"
    )]
    NoDefaultProfile,
    /// One or more selected profiles were not found.
    #[error("unknown profile selector(s): {0}")]
    UnknownProfiles(String),
    /// One or more selected profiles are not certification-eligible.
    #[error("selected profile(s) are not certification-eligible: {0}")]
    NonCertifiableProfiles(String),
}

impl From<SelectionError> for CertifySelectionError {
    fn from(error: SelectionError) -> Self {
        match error {
            SelectionError::NoDefaultProfile => Self::NoDefaultProfile,
            SelectionError::UnknownProfiles(names) => Self::UnknownProfiles(names),
            SelectionError::ConflictingSelectors => {
                unreachable!("certify only supports profile selection")
            }
            SelectionError::UnknownChecks(_) | SelectionError::UnknownFixers(_) => {
                unreachable!("check/fix selector errors should not map into certify")
            }
        }
    }
}
