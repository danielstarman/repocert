use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::config::RepoSession;

use super::{ContractFingerprint, FingerprintError};

const CONFIG_REPO_PATH: &str = ".repocert/config.toml";
const DOMAIN_SEPARATOR: &[u8] = b"repocert:contract-fingerprint:v1\0";

/// Compute the deterministic fingerprint for a loaded contract.
///
/// The fingerprint includes the exact bytes of `.repocert/config.toml` plus any
/// additional protected contract paths declared by the repository.
pub fn compute_contract_fingerprint(
    loaded: &RepoSession,
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

fn collect_additional_paths(loaded: &RepoSession) -> BTreeSet<String> {
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use tempfile::TempDir;

    use crate::certification::compute_contract_fingerprint;
    use crate::config::{Contract, LoadPaths, RepoPath, RepoSession};

    #[test]
    fn declared_paths_in_different_insertion_order_return_same_fingerprint() {
        let repo = TempDir::new().unwrap();
        std::fs::write(repo.path().join("a.txt"), "alpha\n").unwrap();
        std::fs::write(repo.path().join("b.txt"), "beta\n").unwrap();
        let paths = LoadPaths {
            repo_root: repo.path().canonicalize().unwrap(),
            config_path: repo.path().join(".repocert/config.toml"),
        };
        let config_bytes = b"schema_version = 1\n".to_vec();

        let first = RepoSession {
            paths: paths.clone(),
            config_bytes: config_bytes.clone(),
            contract: minimal_contract(["a.txt", "b.txt"]),
        };
        let second = RepoSession {
            paths,
            config_bytes,
            contract: minimal_contract(["b.txt", "a.txt"]),
        };

        let first = compute_contract_fingerprint(&first).unwrap();
        let second = compute_contract_fingerprint(&second).unwrap();

        assert_eq!(first, second);
    }

    fn minimal_contract<const N: usize>(paths: [&str; N]) -> Contract {
        let mut declared_protected_paths = BTreeSet::new();
        for path in paths {
            declared_protected_paths.insert(RepoPath::new(path.to_string()));
        }

        Contract {
            schema_version: 1,
            checks: BTreeMap::new(),
            fixers: BTreeMap::new(),
            profiles: BTreeMap::new(),
            default_profile: None,
            built_in_protected_dir: RepoPath::new(".repocert".to_string()),
            declared_protected_paths,
            protected_refs: Vec::new(),
            certification: None,
            local_policy: None,
            hooks: None,
        }
    }
}
