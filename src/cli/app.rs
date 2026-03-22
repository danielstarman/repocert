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
    Validate(ValidateArgs),
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
