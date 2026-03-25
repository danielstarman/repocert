use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::{NamedTempFile, TempDir};

use super::{CertificationBackend, CertificationPayload, SignedCertificationRecord, SigningError};
use crate::config::TrustedSigner;

/// SSH signature namespace used for authenticated certification records.
pub const SIGNING_NAMESPACE: &str = "repocert-certification";
pub const SIGNED_RECORD_VERSION: u64 = 1;

/// Encode the signed certification payload into deterministic bytes for signing.
pub fn encode_payload_for_signing(payload: &CertificationPayload) -> Vec<u8> {
    let mut encoded = String::new();
    encoded.push_str("repocert-certification-v1\n");
    encoded.push_str("commit=");
    encoded.push_str(&payload.key.commit);
    encoded.push('\n');
    encoded.push_str("profile_hex=");
    encoded.push_str(&hex_encode(payload.key.profile.as_bytes()));
    encoded.push('\n');
    encoded.push_str("contract_fingerprint=");
    encoded.push_str(&payload.contract_fingerprint.to_hex());
    encoded.push('\n');
    encoded.into_bytes()
}

/// Compute the SHA-256 fingerprint for an SSH public key file.
pub fn compute_ssh_key_fingerprint(key_path: &Path) -> Result<String, SigningError> {
    ensure_key_file(key_path)?;

    let output = run_ssh_keygen(
        Command::new("ssh-keygen")
            .args(["-lf"])
            .arg(key_path)
            .args(["-E", "sha256"]),
    )?;

    parse_fingerprint(&output.stdout)
}

/// Produce an SSH-signed certification envelope for the given payload.
pub fn sign_payload_with_ssh(
    signing_key: &Path,
    payload: &CertificationPayload,
) -> Result<SignedCertificationRecord, SigningError> {
    ensure_key_file(signing_key)?;

    let temp_dir = TempDir::new().map_err(|source| SigningError::TempFile { source })?;
    let payload_path = temp_dir.path().join("payload.txt");
    let payload_bytes = encode_payload_for_signing(payload);
    fs::write(&payload_path, &payload_bytes).map_err(|source| SigningError::TempFile { source })?;

    run_ssh_keygen(
        Command::new("ssh-keygen")
            .args(["-Y", "sign", "-f"])
            .arg(signing_key)
            .args(["-n", SIGNING_NAMESPACE])
            .arg(&payload_path),
    )?;

    let signature_path = payload_path.with_extension("txt.sig");
    let signature =
        fs::read_to_string(&signature_path).map_err(|source| SigningError::Io { source })?;
    let signer_fingerprint = compute_ssh_key_fingerprint(signing_key)?;

    Ok(SignedCertificationRecord {
        version: SIGNED_RECORD_VERSION,
        backend: CertificationBackend::Ssh,
        payload: payload.clone(),
        signer_fingerprint,
        signature,
    })
}

/// Verify an SSH-signed certification record against repo-trusted signer keys.
pub fn verify_payload_with_ssh(
    record: &SignedCertificationRecord,
    trusted_signer: &[TrustedSigner],
) -> Result<(), SigningError> {
    validate_signed_record(record)?;

    let Some(index) = trusted_signer
        .iter()
        .position(|signer| signer.fingerprint == record.signer_fingerprint)
    else {
        return Err(SigningError::UntrustedSigner {
            fingerprint: record.signer_fingerprint.clone(),
        });
    };
    let trusted_signer = &trusted_signer[index].public_key;

    let payload_file = NamedTempFile::new().map_err(|source| SigningError::TempFile { source })?;
    let payload_bytes = encode_payload_for_signing(&record.payload);
    fs::write(payload_file.path(), &payload_bytes)
        .map_err(|source| SigningError::TempFile { source })?;

    let signature_file =
        NamedTempFile::new().map_err(|source| SigningError::TempFile { source })?;
    fs::write(signature_file.path(), &record.signature)
        .map_err(|source| SigningError::TempFile { source })?;

    let allowed_signers =
        NamedTempFile::new().map_err(|source| SigningError::TempFile { source })?;
    fs::write(
        allowed_signers.path(),
        allowed_signers_entry(&record.signer_fingerprint, trusted_signer),
    )
    .map_err(|source| SigningError::TempFile { source })?;

    let output = run_ssh_keygen_with_stdin(
        Command::new("ssh-keygen")
            .args(["-Y", "verify", "-f"])
            .arg(allowed_signers.path())
            .args([
                "-I",
                &record.signer_fingerprint,
                "-n",
                SIGNING_NAMESPACE,
                "-s",
            ])
            .arg(signature_file.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
        &payload_bytes,
    )?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SigningError::InvalidSignature {
            fingerprint: record.signer_fingerprint.clone(),
        })
    }
}

fn validate_signed_record(record: &SignedCertificationRecord) -> Result<(), SigningError> {
    if record.version != SIGNED_RECORD_VERSION {
        return Err(SigningError::UnsupportedRecordVersion {
            version: record.version,
        });
    }
    match record.backend {
        CertificationBackend::Ssh => Ok(()),
    }
}

fn ensure_key_file(key_path: &Path) -> Result<(), SigningError> {
    if key_path.is_file() {
        Ok(())
    } else {
        Err(SigningError::MissingSigningKey {
            path: key_path.to_path_buf(),
        })
    }
}

fn run_ssh_keygen(command: &mut Command) -> Result<Output, SigningError> {
    let output = command
        .output()
        .map_err(|source| SigningError::Io { source })?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(SigningError::CommandFailed {
            message: command_message(&output),
        })
    }
}

fn run_ssh_keygen_with_stdin(command: &mut Command, input: &[u8]) -> Result<Output, SigningError> {
    let mut child = command
        .spawn()
        .map_err(|source| SigningError::Io { source })?;
    use std::io::Write as _;
    child
        .stdin
        .as_mut()
        .expect("child stdin should be available")
        .write_all(input)
        .map_err(|source| SigningError::Io { source })?;
    child
        .wait_with_output()
        .map_err(|source| SigningError::Io { source })
}

fn allowed_signers_entry(identity: &str, trusted_signer: &str) -> String {
    format!("{identity} {trusted_signer}\n")
}

fn parse_fingerprint(output: &[u8]) -> Result<String, SigningError> {
    let text = String::from_utf8_lossy(output);
    let fingerprint = text
        .split_whitespace()
        .nth(1)
        .ok_or(SigningError::MissingFingerprint)?;
    if fingerprint.starts_with("SHA256:") {
        Ok(fingerprint.to_string())
    } else {
        Err(SigningError::MissingFingerprint)
    }
}

fn command_message(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        stderr
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(nibble_to_hex(byte >> 4));
        encoded.push(nibble_to_hex(byte & 0x0f));
    }
    encoded
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + (value - 10)),
        _ => unreachable!("nibbles must stay within 0..=15"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;

    use tempfile::TempDir;

    use super::{
        CertificationBackend, CertificationPayload, SIGNED_RECORD_VERSION,
        SignedCertificationRecord, compute_ssh_key_fingerprint, encode_payload_for_signing,
        sign_payload_with_ssh, verify_payload_with_ssh,
    };
    use crate::certification::{CertificationKey, ContractFingerprint, SigningError};
    use crate::config::TrustedSigner;

    #[test]
    fn encode_payload_for_signing_is_deterministic() {
        let payload = payload();

        let first = encode_payload_for_signing(&payload);
        let second = encode_payload_for_signing(&payload);

        assert_eq!(first, second);
    }

    #[test]
    fn verify_payload_with_ssh_rejects_wrong_record_version() {
        let (_dir, public_key_path, public_key) = generate_ssh_signer();
        let signed = SignedCertificationRecord {
            version: SIGNED_RECORD_VERSION + 1,
            backend: CertificationBackend::Ssh,
            payload: payload(),
            signer_fingerprint: compute_ssh_key_fingerprint(&public_key_path).unwrap(),
            signature: "-----BEGIN SSH SIGNATURE-----\ninvalid\n-----END SSH SIGNATURE-----\n"
                .to_string(),
        };

        let error = verify_payload_with_ssh(
            &signed,
            &[TrustedSigner {
                name: "test".to_string(),
                public_key: public_key.clone(),
                fingerprint: signed.signer_fingerprint.clone(),
            }],
        )
        .unwrap_err();

        assert!(matches!(
            error,
            SigningError::UnsupportedRecordVersion { .. }
        ));
    }

    #[test]
    fn sign_and_verify_round_trip_with_ssh() {
        let (_dir, public_key_path, public_key) = generate_ssh_signer();
        let fingerprint = compute_ssh_key_fingerprint(&public_key_path).unwrap();
        let signed = sign_payload_with_ssh(&public_key_path, &payload()).unwrap();

        assert_eq!(signed.signer_fingerprint, fingerprint);
        verify_payload_with_ssh(
            &signed,
            &[TrustedSigner {
                name: "test".to_string(),
                public_key,
                fingerprint,
            }],
        )
        .unwrap();
    }

    fn generate_ssh_signer() -> (TempDir, PathBuf, String) {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("signer");
        let output = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let public_key_path = PathBuf::from(format!("{}.pub", key_path.display()));
        let public_key = std::fs::read_to_string(&public_key_path).unwrap();
        (dir, public_key_path, public_key.trim().to_string())
    }

    fn payload() -> CertificationPayload {
        CertificationPayload {
            key: CertificationKey {
                commit: "abc123".to_string(),
                profile: "default".to_string(),
            },
            contract_fingerprint: ContractFingerprint::from_bytes([7; 32]),
        }
    }
}
