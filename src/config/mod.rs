mod discovery;
mod error;
mod loader;
mod model;
mod raw;
mod validate;

/// Errors produced while discovering, parsing, or validating a contract.
pub use error::{
    DiscoveryError, LoadError, ParseError, ValidationErrorKind, ValidationErrors, ValidationIssue,
};
/// Resolve and load repository contract state from disk.
pub use loader::{LoadOptions, load_repo_session, resolve_paths};
/// The validated repository contract model.
pub use model::{
    CertificationConfig, CertificationMode, CommandSpec, Contract, FixerSpec, HookMode,
    HooksConfig, LoadPaths, Profile, ProtectedRef, RepoPath, RepoSession, TrustedSigner,
};
