use thiserror::Error;

use crate::certification::{FingerprintError, StorageError};
use crate::config::{LoadFailure, LoadPaths};
use crate::contract::SelectionError;
use crate::git::GitCommitError;

#[derive(Debug, Error)]
pub enum StatusError {
    #[error(transparent)]
    Load(#[from] LoadFailure),
    #[error("{error}")]
    Selection {
        paths: LoadPaths,
        #[source]
        error: StatusSelectionError,
    },
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
}

impl StatusError {
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::Selection { paths, .. }
            | Self::GitCommit { paths, .. }
            | Self::Fingerprint { paths, .. }
            | Self::Storage { paths, .. } => Some(paths),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum StatusSelectionError {
    #[error("unknown profile selector(s): {0}")]
    UnknownProfiles(String),
    #[error("selected profile(s) are not certification-eligible: {0}")]
    NonCertifiableProfiles(String),
}

impl From<SelectionError> for StatusSelectionError {
    fn from(error: SelectionError) -> Self {
        match error {
            SelectionError::UnknownProfiles(names) => Self::UnknownProfiles(names),
            SelectionError::ConflictingSelectors
            | SelectionError::NoDefaultProfile
            | SelectionError::UnknownChecks(_)
            | SelectionError::UnknownFixers(_) => {
                unreachable!("status only uses explicit or all-profile selection")
            }
        }
    }
}
