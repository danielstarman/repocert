mod error;
mod execute;
mod plan;
mod service;
mod types;

/// Errors returned by [`run_check`].
pub use error::{CheckError, CheckSelectionError};
/// Run checks and fixer probes selected from a loaded repository session.
pub use service::run_check;
/// Types used to configure and inspect `check` execution.
pub use types::{
    CheckItemKind, CheckItemResult, CheckOptions, CheckOutcome, CheckReport, CheckSelectionMode,
    CheckSummary,
};
