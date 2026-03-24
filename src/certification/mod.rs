mod error;
mod fingerprint;
mod state;
mod store;
mod types;

/// Errors produced while computing a contract fingerprint or reading certification storage.
pub use error::{FingerprintError, StorageError};
/// Compute the exact-byte fingerprint for a loaded contract.
pub use fingerprint::compute_contract_fingerprint;
/// Git-local certification record store.
pub use store::CertificationStore;
/// Core certification key, record, and fingerprint types.
pub use types::{CertificationKey, CertificationRecord, ContractFingerprint};

pub(crate) use state::{ProfileCertificationState, inspect_profile_certification};
