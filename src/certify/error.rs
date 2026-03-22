use thiserror::Error;

use crate::certification::{FingerprintError, StorageError};
use crate::config::{LoadFailure, LoadPaths};
use crate::contract::SelectionError;
use crate::git::{GitHeadError, GitWorktreeError};

#[derive(Debug, Error)]
pub enum CertifyError {
    #[error(transparent)]
    Load(#[from] LoadFailure),
    #[error("{error}")]
    Selection {
        paths: LoadPaths,
        #[source]
        error: CertifySelectionError,
    },
    #[error("worktree must be clean before certification; dirty path(s): {dirty_paths}")]
    DirtyWorktree {
        paths: LoadPaths,
        dirty_paths: String,
    },
    #[error("{error}")]
    GitStatus {
        paths: LoadPaths,
        #[source]
        error: GitWorktreeError,
    },
    #[error("{error}")]
    GitHead {
        paths: LoadPaths,
        #[source]
        error: GitHeadError,
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

impl CertifyError {
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::Selection { paths, .. }
            | Self::DirtyWorktree { paths, .. }
            | Self::GitStatus { paths, .. }
            | Self::GitHead { paths, .. }
            | Self::Fingerprint { paths, .. }
            | Self::Storage { paths, .. } => Some(paths),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CertifySelectionError {
    #[error(
        "no profile selector was provided and no implicit or explicit default profile is available"
    )]
    NoDefaultProfile,
    #[error("unknown profile selector(s): {0}")]
    UnknownProfiles(String),
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
