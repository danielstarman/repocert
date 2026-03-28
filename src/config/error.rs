use std::error::Error as StdError;
use std::fmt;
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;

use thiserror::Error;

/// High-level contract loading failures.
#[derive(Debug, Error)]
pub enum LoadError {
    /// Config discovery failed before a config file was fully resolved.
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    /// Parsing raw config bytes into TOML failed.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Structural contract validation failed.
    #[error(transparent)]
    Validation(#[from] ValidationErrors),
}

/// Errors produced while discovering the repository root and config file.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// The current working directory could not be determined.
    #[error("could not determine the current working directory: {source}")]
    CurrentDir {
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// Upward config discovery did not find `.repocert/config.toml`.
    #[error("could not find .repocert/config.toml by walking upward from {start_dir}")]
    ConfigNotFound {
        /// Directory discovery started from.
        start_dir: PathBuf,
    },
    /// An explicitly supplied repository root was invalid.
    #[error("repo root {path} is invalid: {reason}")]
    InvalidRepoRoot {
        /// Invalid repository root path.
        path: PathBuf,
        /// Human-readable reason the path was rejected.
        reason: String,
    },
    /// An explicitly supplied config path was invalid.
    #[error("explicit config path {path} is invalid: {reason}")]
    InvalidExplicitConfigPath {
        /// Invalid config path.
        path: PathBuf,
        /// Human-readable reason the path was rejected.
        reason: String,
    },
    /// The explicit repo root did not contain the required default config path.
    #[error(
        "explicit repo root {repo_root} does not contain required config file at {config_path}"
    )]
    MissingConfigAtRepoRoot {
        /// Explicit repository root path.
        repo_root: PathBuf,
        /// Expected config path under that repository root.
        config_path: PathBuf,
    },
    /// Explicit repo root and config path did not refer to the same repository.
    #[error("explicit repo root {repo_root} and config path {config_path} do not match")]
    ExplicitPathsMismatch {
        /// Explicit repository root path.
        repo_root: PathBuf,
        /// Explicit config path.
        config_path: PathBuf,
    },
    /// A filesystem I/O error occurred during discovery.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// Path associated with the I/O failure.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
}

/// Errors produced while parsing the raw config file.
#[derive(Debug)]
pub enum ParseError {
    /// The config file bytes were not valid UTF-8.
    InvalidUtf8 {
        /// Config file path.
        path: PathBuf,
        /// Underlying UTF-8 conversion error.
        source: FromUtf8Error,
    },
    /// TOML parsing failed.
    InvalidToml {
        /// Config file path.
        path: PathBuf,
        /// Parser-provided error message.
        message: String,
        /// 1-based line number, when available.
        line: Option<usize>,
        /// 1-based column number, when available.
        column: Option<usize>,
    },
}

impl ParseError {
    /// Build a TOML parse error with resolved line/column information.
    pub fn from_toml(path: &Path, content: &str, source: toml::de::Error) -> Self {
        let (line, column) = source
            .span()
            .map(|span| offset_to_line_column(content, span.start))
            .map_or((None, None), |(line, column)| (Some(line), Some(column)));

        Self::InvalidToml {
            path: path.to_path_buf(),
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

/// Kinds of structural contract validation issues.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationErrorKind {
    /// `schema_version` did not match the supported version.
    SchemaVersion,
    /// A named item used an empty or whitespace-only name.
    EmptyName,
    /// A declared command or fixer command was structurally invalid.
    InvalidCommand,
    /// A profile, check, or fixer reference pointed at an unknown item.
    UnknownReference,
    /// Profile includes formed a cycle.
    ProfileCycle,
    /// Default-profile configuration was invalid.
    InvalidDefaultProfile,
    /// Certification-profile configuration was invalid.
    InvalidCertifyProfile,
    /// A protected contract path was invalid.
    InvalidProtectedPath,
    /// A protected-ref rule was invalid.
    InvalidProtectedRef,
    /// Certification signing/trust config was invalid.
    InvalidCertificationConfig,
    /// Local policy config was invalid.
    InvalidLocalPolicy,
    /// Hook installation config was invalid.
    InvalidHookMode,
}

/// One structural validation issue found while loading the contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationIssue {
    /// Issue kind.
    pub kind: ValidationErrorKind,
    /// Machine-readable subject describing where the issue occurred.
    pub subject: String,
    /// Human-readable issue message.
    pub message: String,
}

/// Collection of structural contract validation issues.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationErrors {
    /// Sorted validation issues.
    pub issues: Vec<ValidationIssue>,
}

impl ValidationErrors {
    /// Create a sorted validation error collection.
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
        ValidationErrorKind::InvalidCertificationConfig => "invalid_certification_config",
        ValidationErrorKind::InvalidLocalPolicy => "invalid_local_policy",
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
