mod error;
mod generated;
mod service;
mod types;

pub use error::InstallHooksError;
pub use service::install_hooks;
pub use types::{GeneratedHook, HookInstallMode, InstallHooksOptions, InstallHooksReport};
