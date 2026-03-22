use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;
use std::string::FromUtf8Error;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Validation(#[from] ValidationErrors),
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("could not determine the current working directory: {source}")]
    CurrentDir {
        #[source]
        source: std::io::Error,
    },
    #[error("could not find .repocert/config.toml by walking upward from {start_dir}")]
    ConfigNotFound { start_dir: PathBuf },
    #[error("repo root {path} is invalid: {reason}")]
    InvalidRepoRoot { path: PathBuf, reason: String },
    #[error("explicit config path {path} is invalid: {reason}")]
    InvalidExplicitConfigPath { path: PathBuf, reason: String },
    #[error(
        "explicit repo root {repo_root} does not contain required config file at {config_path}"
    )]
    MissingConfigAtRepoRoot {
        repo_root: PathBuf,
        config_path: PathBuf,
    },
    #[error("explicit repo root {repo_root} and config path {config_path} do not match")]
    ExplicitPathsMismatch {
        repo_root: PathBuf,
        config_path: PathBuf,
    },
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub enum ParseError {
    InvalidUtf8 {
        path: PathBuf,
        source: FromUtf8Error,
    },
    InvalidToml {
        path: PathBuf,
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },
}

impl ParseError {
    pub fn from_toml(path: &PathBuf, content: &str, source: toml::de::Error) -> Self {
        let (line, column) = source
            .span()
            .map(|span| offset_to_line_column(content, span.start))
            .map_or((None, None), |(line, column)| (Some(line), Some(column)));

        Self::InvalidToml {
            path: path.clone(),
            message: source.to_string(),
            line,
            column,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 { path, source } => {
                write!(
                    f,
                    "config file {path} is not valid UTF-8: {source}",
                    path = path.display()
                )
            }
            Self::InvalidToml {
                path,
                message,
                line,
                column,
            } => {
                write!(f, "could not parse TOML config at {}", path.display())?;
                if let (Some(line), Some(column)) = (line, column) {
                    write!(f, " (line {}, column {})", line, column)?;
                }
                write!(f, ": {message}")
            }
        }
    }
}

impl StdError for ParseError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::InvalidUtf8 { source, .. } => Some(source),
            Self::InvalidToml { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationErrorKind {
    SchemaVersion,
    EmptyName,
    InvalidCommand,
    UnknownReference,
    ProfileCycle,
    InvalidDefaultProfile,
    InvalidCertifyProfile,
    InvalidProtectedPath,
    InvalidProtectedRef,
    InvalidHookMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationIssue {
    pub kind: ValidationErrorKind,
    pub subject: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationErrors {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationErrors {
    pub fn new(mut issues: Vec<ValidationIssue>) -> Self {
        issues.sort_by(|left, right| {
            left.subject
                .cmp(&right.subject)
                .then_with(|| left.message.cmp(&right.message))
                .then_with(|| format!("{:?}", left.kind).cmp(&format!("{:?}", right.kind)))
        });
        Self { issues }
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "config validation failed with {} issue(s):",
            self.issues.len()
        )?;
        for issue in &self.issues {
            writeln!(
                f,
                "- [{}] {}: {}",
                kind_label(&issue.kind),
                issue.subject,
                issue.message
            )?;
        }
        Ok(())
    }
}

impl StdError for ValidationErrors {}

fn kind_label(kind: &ValidationErrorKind) -> &'static str {
    match kind {
        ValidationErrorKind::SchemaVersion => "schema_version",
        ValidationErrorKind::EmptyName => "empty_name",
        ValidationErrorKind::InvalidCommand => "invalid_command",
        ValidationErrorKind::UnknownReference => "unknown_reference",
        ValidationErrorKind::ProfileCycle => "profile_cycle",
        ValidationErrorKind::InvalidDefaultProfile => "invalid_default_profile",
        ValidationErrorKind::InvalidCertifyProfile => "invalid_certify_profile",
        ValidationErrorKind::InvalidProtectedPath => "invalid_protected_path",
        ValidationErrorKind::InvalidProtectedRef => "invalid_protected_ref",
        ValidationErrorKind::InvalidHookMode => "invalid_hook_mode",
    }
}

fn offset_to_line_column(content: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;

    for (index, ch) in content.char_indices() {
        if index >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}
