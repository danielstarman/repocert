use std::fs;
use std::path::PathBuf;

use super::discovery;
use super::error::{DiscoveryError, LoadError, ParseError};
use super::model::{LoadPaths, LoadedContract};
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

/// Wraps a [`LoadError`] together with any resolved paths available at failure time.
#[derive(Debug)]
pub struct LoadFailure {
    /// Resolved repository/config paths, when discovery reached that point.
    pub paths: Option<LoadPaths>,
    /// The underlying discovery, parse, or validation error.
    pub error: LoadError,
}

impl std::fmt::Display for LoadFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for LoadFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Discover, parse, and validate a repository contract from disk.
pub fn load_contract(options: LoadOptions) -> Result<LoadedContract, LoadFailure> {
    let paths = discovery::resolve(options).map_err(|error| LoadFailure {
        paths: None,
        error: LoadError::Discovery(error),
    })?;
    let config_bytes = fs::read(&paths.config_path).map_err(|source| LoadFailure {
        paths: Some(paths.clone()),
        error: LoadError::Discovery(DiscoveryError::Io {
            path: paths.config_path.clone(),
            source,
        }),
    })?;
    let config_text = String::from_utf8(config_bytes.clone()).map_err(|source| LoadFailure {
        paths: Some(paths.clone()),
        error: LoadError::Parse(ParseError::InvalidUtf8 {
            path: paths.config_path.clone(),
            source,
        }),
    })?;
    let raw: RawConfig = toml::from_str(&config_text).map_err(|source| LoadFailure {
        paths: Some(paths.clone()),
        error: LoadError::Parse(ParseError::from_toml(
            &paths.config_path,
            &config_text,
            source,
        )),
    })?;
    let contract = validate::validate(raw, &paths.repo_root).map_err(|error| LoadFailure {
        paths: Some(paths.clone()),
        error,
    })?;

    Ok(LoadedContract {
        paths,
        config_bytes,
        contract,
    })
}
