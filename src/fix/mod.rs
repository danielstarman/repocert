mod error;
mod execute;
mod plan;
mod service;
mod types;

/// Errors returned by [`run_fix`].
pub use error::{FixError, FixSelectionError};
/// Run mutating fixers declared by a loaded repository session.
pub use service::run_fix;
/// Types used to configure and inspect `fix` execution.
pub use types::{FixItemResult, FixOptions, FixOutcome, FixReport, FixSelectionMode, FixSummary};
