use thiserror::Error;

use crate::config::{LoadFailure, LoadPaths};

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
