use thiserror::Error;

use crate::config::{LoadFailure, LoadPaths};
use crate::contract::SelectionError;

/// Errors returned while running `repocert fix`.
#[derive(Debug, Error)]
pub enum FixError {
    /// Contract discovery, parsing, or validation failed before execution.
    #[error(transparent)]
    Load(#[from] LoadFailure),
    /// Profile or named-fixer selection failed.
    #[error("{error}")]
    Selection {
        /// Resolved repository/config paths.
        paths: LoadPaths,
        /// Underlying selection error.
        #[source]
        error: FixSelectionError,
    },
}

impl FixError {
    /// Return resolved paths when they were available for this failure.
    pub fn paths(&self) -> Option<&LoadPaths> {
        match self {
            Self::Load(error) => error.paths.as_ref(),
            Self::Selection { paths, .. } => Some(paths),
        }
    }
}

/// Selection errors specific to `repocert fix`.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum FixSelectionError {
    /// `--profile` and `--name` selectors were used together.
    #[error("`fix` selector modes are mutually exclusive; use either `--profile` or `--name`")]
    ConflictingSelectors,
    /// No explicit profile was selected and no default profile exists.
    #[error(
        "no profile selector was provided and no implicit or explicit default profile is available"
    )]
    NoDefaultProfile,
    /// One or more selected profiles were not found.
    #[error("unknown profile selector(s): {0}")]
    UnknownProfiles(String),
    /// One or more selected named fixers were not found.
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
