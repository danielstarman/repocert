mod error;
mod service;
mod types;

/// Errors returned by [`run_certify`].
pub use error::{CertifyError, CertifySelectionError};
/// Certify the current `HEAD` commit for one or more profiles in a session.
pub use service::run_certify;
/// Types used to configure and inspect `certify` execution.
pub use types::{
    CertifyItemKind, CertifyItemOutcome, CertifyItemResult, CertifyOptions, CertifyProfileOutcome,
    CertifyProfileResult, CertifyReport, CertifySummary,
};
