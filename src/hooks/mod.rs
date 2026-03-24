mod error;
mod generated;
mod service;
mod types;

/// Errors returned by [`install_hooks`].
pub use error::InstallHooksError;
/// Install generated git hooks for the current repository/worktree.
pub use service::install_hooks;
/// Types used to configure and inspect generated hook installation.
pub use types::{GeneratedHook, HookInstallMode, InstallHooksOptions, InstallHooksReport};
