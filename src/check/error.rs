use thiserror::Error;

use crate::config::{LoadFailure, LoadPaths};
use crate::contract::SelectionError;

#[derive(Debug, Error)]
pub enum CheckError {
    #[error(transparent)]
    Load(#[from] LoadFailure),
    #[error("{error}")]
    Selection {
        paths: LoadPaths,
        #[source]
        error: CheckSelectionError,
    },
}

impl CheckError {
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::Selection { paths, .. } => Some(paths),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CheckSelectionError {
    #[error("`check` selector modes are mutually exclusive; use either `--profile` or `--name`")]
    ConflictingSelectors,
    #[error(
        "no profile selector was provided and no implicit or explicit default profile is available"
    )]
    NoDefaultProfile,
    #[error("unknown profile selector(s): {0}")]
    UnknownProfiles(String),
    #[error("unknown named check selector(s): {0}")]
    UnknownChecks(String),
}

impl From<SelectionError> for CheckSelectionError {
    fn from(error: SelectionError) -> Self {
        match error {
            SelectionError::ConflictingSelectors => Self::ConflictingSelectors,
            SelectionError::NoDefaultProfile => Self::NoDefaultProfile,
            SelectionError::UnknownProfiles(names) => Self::UnknownProfiles(names),
            SelectionError::UnknownChecks(names) => Self::UnknownChecks(names),
            SelectionError::UnknownFixers(_) => {
                unreachable!("fixer selection errors should not map into check")
            }
        }
    }
}
