use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::LoadOptions;
use super::error::DiscoveryError;

const CONFIG_DIR: &str = ".repocert";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug)]
pub(super) struct ResolvedPaths {
    pub repo_root: PathBuf,
    pub config_path: PathBuf,
}

pub(super) fn resolve(options: LoadOptions) -> Result<ResolvedPaths, DiscoveryError> {
    match (options.repo_root, options.config_path) {
        (Some(repo_root), Some(config_path)) => resolve_both(&repo_root, &config_path),
        (Some(repo_root), None) => resolve_from_repo_root(&repo_root),
        (None, Some(config_path)) => resolve_from_config_path(&config_path),
        (None, None) => {
            let start_dir = canonicalize_dir(options.start_dir.unwrap_or_else(|| {
                std::env::current_dir().expect("current_dir should be available")
            }))?;
            discover_from(&start_dir)
        }
    }
}

fn resolve_both(repo_root: &Path, config_path: &Path) -> Result<ResolvedPaths, DiscoveryError> {
    let resolved_repo_root = canonicalize_dir(repo_root)?;
    let resolved_config_path = canonicalize_file(config_path)?;
    let expected_config_path = resolved_repo_root.join(CONFIG_DIR).join(CONFIG_FILE);

    if resolved_config_path != expected_config_path {
        return Err(DiscoveryError::ExplicitPathsMismatch {
            repo_root: resolved_repo_root,
            config_path: resolved_config_path,
        });
    }

    Ok(ResolvedPaths {
        repo_root: resolved_repo_root,
        config_path: expected_config_path,
    })
}

fn resolve_from_repo_root(repo_root: &Path) -> Result<ResolvedPaths, DiscoveryError> {
    let resolved_repo_root = canonicalize_dir(repo_root)?;
    let config_path = resolved_repo_root.join(CONFIG_DIR).join(CONFIG_FILE);

    if !config_path.is_file() {
        return Err(DiscoveryError::MissingConfigAtRepoRoot {
            repo_root: resolved_repo_root,
            config_path,
        });
    }

    Ok(ResolvedPaths {
        repo_root: resolved_repo_root,
        config_path,
    })
}

fn resolve_from_config_path(config_path: &Path) -> Result<ResolvedPaths, DiscoveryError> {
    let resolved_config_path = canonicalize_file(config_path)?;

    if resolved_config_path
        .file_name()
        .and_then(|name| name.to_str())
        != Some(CONFIG_FILE)
    {
        return Err(DiscoveryError::InvalidExplicitConfigPath {
            path: resolved_config_path,
            reason: "config file must be named .repocert/config.toml".to_string(),
        });
    }

    let config_dir =
        resolved_config_path
            .parent()
            .ok_or_else(|| DiscoveryError::InvalidExplicitConfigPath {
                path: resolved_config_path.clone(),
                reason: "config file must have a parent directory".to_string(),
            })?;

    if config_dir.file_name().and_then(|name| name.to_str()) != Some(CONFIG_DIR) {
        return Err(DiscoveryError::InvalidExplicitConfigPath {
            path: resolved_config_path,
            reason: "config file must live under a .repocert directory".to_string(),
        });
    }

    let repo_root = canonicalize_dir(config_dir.parent().ok_or_else(|| {
        DiscoveryError::InvalidExplicitConfigPath {
            path: resolved_config_path.clone(),
            reason: "config file must have a repo root parent".to_string(),
        }
    })?)?;

    Ok(ResolvedPaths {
        repo_root,
        config_path: resolved_config_path,
    })
}

fn discover_from(start_dir: &Path) -> Result<ResolvedPaths, DiscoveryError> {
    for candidate_root in start_dir.ancestors() {
        let config_path = candidate_root.join(CONFIG_DIR).join(CONFIG_FILE);
        if config_path.is_file() {
            return Ok(ResolvedPaths {
                repo_root: candidate_root.to_path_buf(),
                config_path,
            });
        }
    }

    Err(DiscoveryError::ConfigNotFound {
        start_dir: start_dir.to_path_buf(),
    })
}

fn canonicalize_dir(path: impl AsRef<Path>) -> Result<PathBuf, DiscoveryError> {
    let path = path.as_ref();
    let resolved = fs::canonicalize(path).map_err(|source| discovery_io_error(path, source))?;
    if !resolved.is_dir() {
        return Err(DiscoveryError::InvalidRepoRoot {
            path: resolved,
            reason: "path is not a directory".to_string(),
        });
    }

    Ok(resolved)
}

fn canonicalize_file(path: impl AsRef<Path>) -> Result<PathBuf, DiscoveryError> {
    let path = path.as_ref();
    let resolved = fs::canonicalize(path).map_err(|source| discovery_io_error(path, source))?;
    if !resolved.is_file() {
        return Err(DiscoveryError::InvalidExplicitConfigPath {
            path: resolved,
            reason: "path is not a file".to_string(),
        });
    }

    Ok(resolved)
}

fn discovery_io_error(path: &Path, source: io::Error) -> DiscoveryError {
    DiscoveryError::Io {
        path: path.to_path_buf(),
        source,
    }
}
