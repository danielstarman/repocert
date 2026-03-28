use thiserror::Error;

use std::path::PathBuf;

use crate::certification::{FingerprintError, SigningError, StorageError};
use crate::contract::SelectionError;
use crate::git::{GitCommitError, GitWorktreeError};

/// Errors returned while running `repocert certify`.
#[derive(Debug, Error)]
pub enum CertifyError {
    /// Profile selection for certification failed.
    #[error(transparent)]
    Selection(#[from] CertifySelectionError),
    /// The worktree was dirty when certification required a clean checkout.
    #[error("worktree must be clean before certification; dirty path(s): {dirty_paths}")]
    DirtyWorktree {
        /// Dirty paths visible in the worktree snapshot.
        dirty_paths: String,
    },
    /// Capturing worktree state failed.
    #[error(transparent)]
    GitStatus(#[from] GitWorktreeError),
    /// Resolving the commit to certify failed.
    #[error(transparent)]
    GitCommit(#[from] GitCommitError),
    /// Computing the current contract fingerprint failed.
    #[error(transparent)]
    Fingerprint(#[from] FingerprintError),
    /// Authenticated certification required a local signing key, but none was selected.
    #[error(
        "authenticated certification requires a local signing key; pass --signing-key or set REPOCERT_SIGNING_KEY"
    )]
    MissingSigningKeySelection,
    /// SSH signing or signed-record verification failed during certification.
    #[error("{error}")]
    Signing {
        /// Local public-key path used for signing.
        signing_key: PathBuf,
        /// Underlying signing or verification error.
        #[source]
        error: SigningError,
    },
    /// Reading or writing certification storage failed.
    #[error(transparent)]
    Storage(#[from] StorageError),
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
