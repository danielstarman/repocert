use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CertificationKey {
    pub commit: String,
    pub profile: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractFingerprint([u8; 32]);

impl ContractFingerprint {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex_encode(&self.0)
    }

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CertificationRecord {
    pub key: CertificationKey,
    pub contract_fingerprint: ContractFingerprint,
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(nibble_to_hex(byte >> 4));
        encoded.push(nibble_to_hex(byte & 0x0f));
    }
    encoded
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if value.len() != 64 {
        return Err("contract fingerprint must be 64 lowercase hex characters".to_string());
    }

    let mut bytes = Vec::with_capacity(value.len() / 2);
    let mut chars = value.bytes();
    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        let high = hex_value(high)?;
        let low = hex_value(low)?;
        bytes.push((high << 4) | low);
    }

    Ok(bytes)
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + (value - 10)),
        _ => unreachable!("nibbles must stay within 0..=15"),
    }
}

fn hex_value(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err("contract fingerprint must be 64 lowercase hex characters".to_string()),
    }
}
