mod error;
mod service;
mod types;

/// Errors returned by [`check_local_commit_policy`].
pub use error::LocalPolicyError;
/// Check local protected-branch and primary-checkout policy for the current session.
pub use service::check_local_commit_policy;
/// Types used to configure and inspect local policy decisions.
pub use types::{LocalPolicyDecision, LocalPolicyViolation};
