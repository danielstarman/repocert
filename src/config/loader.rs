use std::fs;

use super::LoadOptions;
use super::discovery;
use super::error::{DiscoveryError, LoadError, ParseError};
use super::model::LoadedContract;
use super::raw::RawConfig;
use super::validate;

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
