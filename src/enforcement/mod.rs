mod error;
mod service;
mod types;

/// Errors returned while authorizing a ref update.
pub use error::AuthorizeError;
/// Decide whether a proposed ref update is allowed by the repository session contract.
pub use service::authorize_ref_update;
/// Types used to configure and inspect authorization decisions.
pub use types::{
    AuthorizeOptions, AuthorizeProfileResult, AuthorizeProfileState, AuthorizeReport, MatchedRule,
};
