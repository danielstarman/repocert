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
/// Load a repository contract from disk.
pub use loader::{LoadFailure, LoadOptions, load_contract};
/// The validated repository contract model.
pub use model::{
    CertificationConfig, CertificationMode, CommandSpec, Contract, FixerSpec, HookMode,
    HooksConfig, LoadPaths, LoadedContract, Profile, ProtectedRef, RepoPath, TrustedSigner,
};
