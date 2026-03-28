use thiserror::Error;

use crate::certification::{FingerprintError, ProfileCertificationError, StorageError};
use crate::contract::SelectionError;
use crate::git::{GitCheckoutError, GitCommitError};

/// Errors returned while running `repocert status`.
#[derive(Debug, Error)]
pub enum StatusError {
    /// Profile selection for status inspection failed.
    #[error(transparent)]
    Selection(#[from] StatusSelectionError),
    /// Inspecting the current checkout/ref failed while inferring assertion scope.
    #[error(transparent)]
    GitCheckout(#[from] GitCheckoutError),
    /// Resolving the inspected commit or checking referenced commit objects failed.
    #[error(transparent)]
    GitCommit(#[from] GitCommitError),
    /// Computing the current contract fingerprint failed.
    #[error(transparent)]
    Fingerprint(#[from] FingerprintError),
    /// Reading or verifying certification storage failed.
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Profile-selection errors specific to `repocert status`.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum StatusSelectionError {
    /// Assertion mode needed an inferred profile scope, but none could be determined.
    #[error(
        "status assertion requires an explicit --profile because no protected-ref match or default profile could be inferred"
    )]
    NoAssertionScope,
    /// One or more selected profiles were not found.
    #[error("unknown profile selector(s): {0}")]
    UnknownProfiles(String),
    /// One or more selected profiles are not certification-eligible.
    #[error("selected profile(s) are not certification-eligible: {0}")]
    NonCertifiableProfiles(String),
}

impl From<SelectionError> for StatusSelectionError {
    fn from(error: SelectionError) -> Self {
        match error {
            SelectionError::NoDefaultProfile => Self::NoAssertionScope,
            SelectionError::UnknownProfiles(names) => Self::UnknownProfiles(names),
            SelectionError::ConflictingSelectors
            | SelectionError::UnknownChecks(_)
            | SelectionError::UnknownFixers(_) => {
                unreachable!("status only uses explicit or all-profile selection")
            }
        }
    }
}

impl From<ProfileCertificationError> for StatusError {
    fn from(error: ProfileCertificationError) -> Self {
        match error {
            ProfileCertificationError::Storage(error) => Self::Storage(error),
            ProfileCertificationError::GitCommit(error) => Self::GitCommit(error),
        }
    }
}
