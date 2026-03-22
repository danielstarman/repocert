use thiserror::Error;

use crate::config::{LoadFailure, LoadPaths};
use crate::selection::SelectionError;

#[derive(Debug, Error)]
pub enum FixError {
    #[error(transparent)]
    Load(#[from] LoadFailure),
    #[error("{error}")]
    Selection {
        paths: LoadPaths,
        #[source]
        error: FixSelectionError,
    },
}

impl FixError {
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::Selection { paths, .. } => Some(paths),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum FixSelectionError {
    #[error("`fix` selector modes are mutually exclusive; use either `--profile` or `--name`")]
    ConflictingSelectors,
    #[error(
        "no profile selector was provided and no implicit or explicit default profile is available"
    )]
    NoDefaultProfile,
    #[error("unknown profile selector(s): {0}")]
    UnknownProfiles(String),
    #[error("unknown named fixer selector(s): {0}")]
    UnknownFixers(String),
}

impl From<SelectionError> for FixSelectionError {
    fn from(error: SelectionError) -> Self {
        match error {
            SelectionError::ConflictingSelectors => Self::ConflictingSelectors,
            SelectionError::NoDefaultProfile => Self::NoDefaultProfile,
            SelectionError::UnknownProfiles(names) => Self::UnknownProfiles(names),
            SelectionError::UnknownFixers(names) => Self::UnknownFixers(names),
            SelectionError::UnknownChecks(_) => {
                unreachable!("check selection errors should not map into fix")
            }
        }
    }
}
