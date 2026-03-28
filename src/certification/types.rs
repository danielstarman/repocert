use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::hex;

/// Unique certification key for a `(commit, profile)` pair.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CertificationKey {
    /// Certified commit SHA.
    pub commit: String,
    /// Certified profile name.
    pub profile: String,
}

/// Exact-byte SHA-256 fingerprint of the current repository contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractFingerprint([u8; 32]);

impl ContractFingerprint {
    /// Construct a fingerprint from raw SHA-256 bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw fingerprint bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Encode the fingerprint as lowercase hexadecimal.
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    /// Decode a lowercase hexadecimal fingerprint string.
    pub fn from_hex(value: &str) -> Result<Self, String> {
        let bytes = hex_decode(value)?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "contract fingerprint must decode to 32 bytes".to_string())?;
        Ok(Self(bytes))
    }
}

impl Serialize for ContractFingerprint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for ContractFingerprint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContractFingerprintVisitor;

        impl<'de> Visitor<'de> for ContractFingerprintVisitor {
            type Value = ContractFingerprint;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a lowercase hexadecimal SHA-256 fingerprint")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: DeError,
            {
                ContractFingerprint::from_hex(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(ContractFingerprintVisitor)
    }
}

/// Stored certification state for one `(commit, profile)` pair.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CertificationPayload {
    /// Commit/profile key this record certifies.
    pub key: CertificationKey,
    /// Contract fingerprint that was current when the certification was written.
    pub contract_fingerprint: ContractFingerprint,
}

/// Supported authenticated certification backends.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CertificationBackend {
    /// SSH signature produced and verified via `ssh-keygen -Y`.
    Ssh,
}

/// Versioned signed certification envelope stored on disk.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CertificationRecord {
    /// Envelope version for future format evolution.
    pub version: u64,
    /// Signature backend used for this record.
    pub backend: CertificationBackend,
    /// Signed certification payload.
    pub payload: CertificationPayload,
    /// Fingerprint of the signer public key used to produce the signature.
    pub signer_fingerprint: String,
    /// Signature blob as produced by the signing backend.
    pub signature: String,
}

impl CertificationRecord {
    /// Borrow the logical certification payload.
    pub fn payload(&self) -> &CertificationPayload {
        &self.payload
    }

    /// Borrow the `(commit, profile)` key for this record.
    pub fn key(&self) -> &CertificationKey {
        &self.payload.key
    }

    /// Borrow the contract fingerprint carried by this record.
    pub fn contract_fingerprint(&self) -> &ContractFingerprint {
        &self.payload.contract_fingerprint
    }
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if value.len() != 64 {
        return Err("contract fingerprint must be 64 lowercase hex characters".to_string());
    }

    hex::decode(value)
        .ok_or_else(|| "contract fingerprint must be 64 lowercase hex characters".to_string())
}
