mod error;
mod fingerprint;
mod signing;
mod state;
mod store;
mod types;

/// Errors produced while computing a contract fingerprint or reading certification storage.
pub use error::{FingerprintError, SigningError, StorageError};
/// Compute the exact-byte fingerprint for a loaded contract.
pub use fingerprint::compute_contract_fingerprint;
pub use signing::{
    SIGNING_NAMESPACE, compute_ssh_key_fingerprint, encode_payload_for_signing,
    find_trusted_signer, sign_payload_with_ssh, verify_payload_with_ssh,
};
/// Git-local certification record store.
pub use store::CertificationStore;
/// Core certification key, record, and fingerprint types.
pub use types::{
    CertificationBackend, CertificationKey, CertificationPayload, CertificationRecord,
    ContractFingerprint, SignedCertificationRecord,
};

pub(crate) use state::{ProfileCertificationState, inspect_profile_certification};
