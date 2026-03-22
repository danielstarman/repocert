use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "repocert")]
pub(super) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(super) enum Commands {
    Authorize(AuthorizeArgs),
    Certify(CertifyArgs),
    Check(CheckArgs),
    Fix(FixArgs),
    Status(StatusArgs),
    Validate(ValidateArgs),
}

#[derive(Debug, Args)]
pub(super) struct AuthorizeArgs {
    #[arg(long = "repo-root")]
    pub repo_root: Option<PathBuf>,
    #[arg(long = "config-path")]
    pub config_path: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
    pub old: String,
    pub new: String,
    pub reference: String,
}

#[derive(Debug, Args)]
pub(super) struct CertifyArgs {
    #[arg(long = "repo-root")]
    pub repo_root: Option<PathBuf>,
    #[arg(long = "config-path")]
    pub config_path: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
    #[arg(long = "profile")]
    pub profile: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct CheckArgs {
    #[arg(long = "repo-root")]
    pub repo_root: Option<PathBuf>,
    #[arg(long = "config-path")]
    pub config_path: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
    #[arg(long = "profile")]
    pub profile: Vec<String>,
    #[arg(long = "name")]
    pub name: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct FixArgs {
    #[arg(long = "repo-root")]
    pub repo_root: Option<PathBuf>,
    #[arg(long = "config-path")]
    pub config_path: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
    #[arg(long = "profile")]
    pub profile: Option<String>,
    #[arg(long = "name")]
    pub name: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct StatusArgs {
    #[arg(long = "repo-root")]
    pub repo_root: Option<PathBuf>,
    #[arg(long = "config-path")]
    pub config_path: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
    #[arg(long = "commit")]
    pub commit: Option<String>,
    #[arg(long = "profile")]
    pub profile: Vec<String>,
    #[arg(long = "assert-certified")]
    pub assert_certified: bool,
}

#[derive(Debug, Args)]
pub(super) struct ValidateArgs {
    #[arg(long = "repo-root")]
    pub repo_root: Option<PathBuf>,
    #[arg(long = "config-path")]
    pub config_path: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub(super) enum OutputFormat {
    #[default]
    Human,
    Json,
}
