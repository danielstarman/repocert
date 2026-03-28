use std::fs;

use repocert::config::{
    LoadError, LoadOptions, LoadPaths, RepoSession, load_repo_session, resolve_paths,
};
use tempfile::TempDir;

#[path = "config/error.rs"]
mod config_error;
#[path = "config/loading.rs"]
mod config_loading;
#[path = "config/validate.rs"]
mod config_validate;

#[derive(Debug)]
pub(crate) struct LoadContractFailure {
    paths: Option<LoadPaths>,
    error: LoadError,
}

impl LoadContractFailure {
    pub(crate) fn paths(&self) -> Option<&LoadPaths> {
        self.paths.as_ref()
    }

    pub(crate) fn error(&self) -> &LoadError {
        &self.error
    }
}

pub(crate) fn load_contract(options: LoadOptions) -> Result<RepoSession, Box<LoadContractFailure>> {
    let paths = resolve_paths(options).map_err(|error| {
        Box::new(LoadContractFailure {
            paths: None,
            error: LoadError::Discovery(error),
        })
    })?;
    load_repo_session(paths.clone()).map_err(|error| {
        Box::new(LoadContractFailure {
            paths: Some(paths),
            error,
        })
    })
}

pub(crate) fn write_repo_file(repo: &TempDir, relative_path: &str, contents: &str) {
    let path = repo.path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}
