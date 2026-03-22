use std::fs;
use std::path::PathBuf;

use super::discovery;
use super::error::{DiscoveryError, LoadError, ParseError};
use super::model::LoadedContract;
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

pub fn load_contract(options: LoadOptions) -> Result<LoadedContract, LoadError> {
    let resolved = discovery::resolve(options)?;
    let config_bytes = fs::read(&resolved.config_path).map_err(|source| {
        LoadError::Discovery(DiscoveryError::Io {
            path: resolved.config_path.clone(),
            source,
        })
    })?;
    let config_text = String::from_utf8(config_bytes.clone()).map_err(|source| {
        LoadError::Parse(ParseError::InvalidUtf8 {
            path: resolved.config_path.clone(),
            source,
        })
    })?;
    let raw: RawConfig = toml::from_str(&config_text).map_err(|source| {
        LoadError::Parse(ParseError::from_toml(
            &resolved.config_path,
            &config_text,
            source,
        ))
    })?;
    let contract = validate::validate(raw, &resolved.repo_root)?;

    Ok(LoadedContract {
        repo_root: resolved.repo_root,
        config_path: resolved.config_path,
        config_bytes,
        contract,
    })
}
