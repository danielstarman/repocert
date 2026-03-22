use std::fs;
use std::path::PathBuf;

use super::discovery;
use super::error::{DiscoveryError, LoadError, ParseError};
use super::model::{LoadPaths, LoadedContract};
use super::raw::RawConfig;
use super::validate;

#[derive(Clone, Debug, Default)]
pub struct LoadOptions {
    pub start_dir: Option<PathBuf>,
    pub repo_root: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
}

impl LoadOptions {
    pub fn discover_from(path: impl Into<PathBuf>) -> Self {
        Self {
            start_dir: Some(path.into()),
            repo_root: None,
            config_path: None,
        }
    }

    pub fn from_repo_root(path: impl Into<PathBuf>) -> Self {
        Self {
            start_dir: None,
            repo_root: Some(path.into()),
            config_path: None,
        }
    }

    pub fn from_config_path(path: impl Into<PathBuf>) -> Self {
        Self {
            start_dir: None,
            repo_root: None,
            config_path: Some(path.into()),
        }
    }
}

#[derive(Debug)]
pub struct LoadFailure {
    pub paths: Option<LoadPaths>,
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
