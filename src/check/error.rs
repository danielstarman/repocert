use thiserror::Error;

use crate::contract::SelectionError;

/// Errors returned while running `repocert check`.
#[derive(Debug, Error)]
pub enum CheckError {
    /// Profile or named-check selection failed.
    #[error(transparent)]
    Selection(#[from] CheckSelectionError),
}

/// Selection errors specific to `repocert check`.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum CheckSelectionError {
    /// `--profile` and `--name` selectors were used together.
    #[error("`check` selector modes are mutually exclusive; use either `--profile` or `--name`")]
    ConflictingSelectors,
    /// No explicit profile was selected and no default profile exists.
    #[error(
        "no profile selector was provided and no implicit or explicit default profile is available"
    )]
    NoDefaultProfile,
    /// One or more selected profiles were not found.
    #[error("unknown profile selector(s): {0}")]
    UnknownProfiles(String),
    /// One or more selected named checks were not found.
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
