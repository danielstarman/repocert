use thiserror::Error;

use crate::certification::{FingerprintError, StorageError};
use crate::config::{LoadFailure, LoadPaths};
use crate::contract::SelectionError;
use crate::git::{GitCheckoutError, GitCommitError};

/// Errors returned while running `repocert status`.
#[derive(Debug, Error)]
pub enum StatusError {
    /// Contract discovery, parsing, or validation failed before inspection.
    #[error(transparent)]
    Load(#[from] LoadFailure),
    /// Profile selection for status inspection failed.
    #[error("{error}")]
    Selection {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying selection error.
        #[source]
        error: StatusSelectionError,
    },
    /// Inspecting the current checkout/ref failed while inferring assertion scope.
    #[error("{error}")]
    GitCheckout {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying checkout inspection error.
        #[source]
        error: GitCheckoutError,
    },
    /// Resolving the commit to inspect failed.
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
    /// Reading or verifying certification storage failed.
    #[error("{error}")]
    Storage {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying storage or signing error.
        #[source]
        error: StorageError,
    },
}

impl StatusError {
    /// Return resolved paths when they were available for this failure.
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::Selection { paths, .. }
            | Self::GitCheckout { paths, .. }
            | Self::GitCommit { paths, .. }
            | Self::Fingerprint { paths, .. }
            | Self::Storage { paths, .. } => Some(paths),
        }
    }
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
