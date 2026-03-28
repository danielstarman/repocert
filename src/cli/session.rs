use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::{Map, Value};

use repocert::config::{
    LoadError, LoadOptions, LoadPaths, RepoSession, load_repo_session, resolve_paths,
};

use super::app::OutputFormat;
use super::json::command_error;

struct SessionLoadFailure {
    paths: Option<LoadPaths>,
    error: LoadError,
}

pub(super) struct CommandRuntime {
    name: &'static str,
    format: OutputFormat,
    session: RepoSession,
}

impl CommandRuntime {
    pub(super) fn load(
        name: &'static str,
        format: OutputFormat,
        repo_root: Option<PathBuf>,
        config_path: Option<PathBuf>,
    ) -> Result<Self, ExitCode> {
        match load_session(repo_root, config_path) {
            Ok(session) => Ok(Self {
                name,
                format,
                session,
            }),
            Err(failure) => {
                render_error(
                    name,
                    format,
                    failure.paths.as_ref(),
                    load_error_category(&failure.error),
                    &failure.error.to_string(),
                    None,
                );
                Err(ExitCode::from(1))
            }
        }
    }

    pub(super) fn session(&self) -> &RepoSession {
        &self.session
    }

    pub(super) fn paths(&self) -> &LoadPaths {
        self.session.paths()
    }

    pub(super) fn format(&self) -> OutputFormat {
        self.format
    }

    pub(super) fn fail(
        &self,
        category: &str,
        message: &str,
        error_details: Option<Map<String, Value>>,
    ) -> ExitCode {
        self.render_error(category, message, error_details);
        ExitCode::from(1)
    }

    pub(super) fn render_error(
        &self,
        category: &str,
        message: &str,
        error_details: Option<Map<String, Value>>,
    ) {
        render_error(
            self.name,
            self.format,
            Some(self.paths()),
            category,
            message,
            error_details,
        );
    }

    pub(super) fn render_without_session(
        name: &'static str,
        format: OutputFormat,
        category: &str,
        message: &str,
        error_details: Option<Map<String, Value>>,
    ) {
        render_error(name, format, None, category, message, error_details);
    }
}

fn load_session(
    repo_root: Option<PathBuf>,
    config_path: Option<PathBuf>,
) -> Result<RepoSession, Box<SessionLoadFailure>> {
    let load_options = LoadOptions {
        start_dir: None,
        repo_root,
        config_path,
    };
    let paths = resolve_paths(load_options).map_err(|error| {
        Box::new(SessionLoadFailure {
            paths: None,
            error: LoadError::Discovery(error),
        })
    })?;
    load_repo_session(paths.clone()).map_err(|error| {
        Box::new(SessionLoadFailure {
            paths: Some(paths),
            error,
        })
    })
}

fn load_error_category(error: &LoadError) -> &'static str {
    match error {
        LoadError::Discovery(_) => "discovery",
        LoadError::Parse(_) => "parse",
        LoadError::Validation(_) => "validation",
    }
}

fn render_error(
    command: &str,
    format: OutputFormat,
    paths: Option<&LoadPaths>,
    category: &str,
    message: &str,
    error_details: Option<Map<String, Value>>,
) {
    match format {
        OutputFormat::Human => {
            eprintln!("FAIL {command} [{category}]");
            eprintln!("{message}");
        }
        OutputFormat::Json => {
            let output =
                command_error(command, paths, category, message.to_string(), error_details);
            println!(
                "{}",
                serde_json::to_string(&output).expect("JSON serialization should succeed")
            );
        }
    }
}
