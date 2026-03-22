use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::config::LoadedContract;

use super::{ContractFingerprint, FingerprintError};

const CONFIG_REPO_PATH: &str = ".repocert/config.toml";
const DOMAIN_SEPARATOR: &[u8] = b"repocert:contract-fingerprint:v1\0";

pub fn compute_contract_fingerprint(
    loaded: &LoadedContract,
) -> Result<ContractFingerprint, FingerprintError> {
    let additional_paths = collect_additional_paths(loaded);
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_SEPARATOR);
    update_hash_with_entry(&mut hasher, CONFIG_REPO_PATH, &loaded.config_bytes);

    for path in additional_paths {
        let full_path = loaded.paths.repo_root.join(Path::new(path.as_str()));
        let bytes = read_protected_contract_file(&full_path)?;
        update_hash_with_entry(&mut hasher, path.as_str(), &bytes);
    }

    Ok(ContractFingerprint::from_bytes(hasher.finalize().into()))
}

fn collect_additional_paths(loaded: &LoadedContract) -> BTreeSet<String> {
    loaded
        .contract
        .declared_protected_paths
        .iter()
        .filter(|path| path.as_str() != CONFIG_REPO_PATH)
        .map(|path| path.as_str().to_string())
        .collect()
}

fn read_protected_contract_file(path: &Path) -> Result<Vec<u8>, FingerprintError> {
    let metadata = fs::metadata(path).map_err(|source| FingerprintError::ProtectedPathIo {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(FingerprintError::ProtectedPathNotFile {
            path: path.to_path_buf(),
        });
    }

    fs::read(path).map_err(|source| FingerprintError::ProtectedPathIo {
        path: path.to_path_buf(),
        source,
    })
}

fn update_hash_with_entry(hasher: &mut Sha256, repo_path: &str, bytes: &[u8]) {
    let path_bytes = repo_path.as_bytes();
    hasher.update((path_bytes.len() as u64).to_be_bytes());
    hasher.update(path_bytes);
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}
