use std::fs;
use std::path::PathBuf;

use super::discovery;
use super::error::{DiscoveryError, LoadError, ParseError};
use super::model::{LoadPaths, RepoSession};
use super::raw::RawConfig;
use super::validate;

/// Controls how `.repocert/config.toml` is located before loading.
#[derive(Clone, Debug, Default)]
pub struct LoadOptions {
    /// Starting directory for upward config discovery.
    pub start_dir: Option<PathBuf>,
    /// Explicit repository root to load from.
    pub repo_root: Option<PathBuf>,
    /// Explicit config file path to load.
    pub config_path: Option<PathBuf>,
}

impl LoadOptions {
    /// Discover a config by walking upward from `path`.
    pub fn discover_from(path: impl Into<PathBuf>) -> Self {
        Self {
            start_dir: Some(path.into()),
            repo_root: None,
            config_path: None,
        }
    }

    /// Load the default config from an explicit repository root.
    pub fn from_repo_root(path: impl Into<PathBuf>) -> Self {
        Self {
            start_dir: None,
            repo_root: Some(path.into()),
            config_path: None,
        }
    }

    /// Load a config from an explicit path.
    pub fn from_config_path(path: impl Into<PathBuf>) -> Self {
        Self {
            start_dir: None,
            repo_root: None,
            config_path: Some(path.into()),
        }
    }
}

/// Resolve repository and contract file paths from load options.
pub fn resolve_paths(options: LoadOptions) -> Result<LoadPaths, DiscoveryError> {
    discovery::resolve(options)
}

/// Load and validate a repository contract at resolved paths.
pub fn load_repo_session(paths: LoadPaths) -> Result<RepoSession, LoadError> {
    let paths = resolve_paths(LoadOptions {
        start_dir: None,
        repo_root: Some(paths.repo_root),
        config_path: Some(paths.config_path),
    })
    .map_err(LoadError::Discovery)?;

    let config_bytes = fs::read(&paths.config_path).map_err(|source| {
        LoadError::Discovery(DiscoveryError::Io {
            path: paths.config_path.clone(),
            source,
        })
    })?;
    let config_text = String::from_utf8(config_bytes.clone()).map_err(|source| {
        LoadError::Parse(ParseError::InvalidUtf8 {
            path: paths.config_path.clone(),
            source,
        })
    })?;
    let raw: RawConfig = toml::from_str(&config_text).map_err(|source| {
        LoadError::Parse(ParseError::from_toml(
            &paths.config_path,
            &config_text,
            source,
        ))
    })?;
    let contract = validate::validate(raw, &paths.repo_root)?;

    Ok(RepoSession {
        paths,
        config_bytes,
        contract,
    })
}
