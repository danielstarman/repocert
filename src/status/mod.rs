mod error;
mod service;
mod types;

/// Errors returned by [`run_status`].
pub use error::{StatusError, StatusSelectionError};
/// Inspect certification state for a commit and any configured protected refs in a session.
pub use service::run_status;
/// Types used to configure and inspect `status` results.
pub use types::{
    ProtectedRefStatus, StatusOptions, StatusProfileResult, StatusProfileState, StatusReport,
    StatusSummary,
};
