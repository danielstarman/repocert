use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::json;

use repocert::config::{DiscoveryError, LoadError, LoadOptions, ParseError, load_contract};

#[derive(Debug, Parser)]
#[command(name = "repocert")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Validate(ValidateArgs),
}

#[derive(Debug, Args)]
struct ValidateArgs {
    #[arg(long = "repo-root")]
    repo_root: Option<PathBuf>,
    #[arg(long = "config-path")]
    config_path: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    #[default]
    Human,
    Json,
}

pub fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return ExitCode::from(error.exit_code() as u8);
        }
    };

    match cli.command {
        Commands::Validate(args) => run_validate(args),
    }
}

fn run_validate(args: ValidateArgs) -> ExitCode {
    let load_options = LoadOptions {
        start_dir: None,
        repo_root: args.repo_root,
        config_path: args.config_path,
    };

    match load_contract(load_options) {
        Ok(loaded) => {
            match args.format {
                OutputFormat::Human => {
                    println!("PASS validate");
                    println!("repo_root: {}", loaded.repo_root.display());
                    println!("config_path: {}", loaded.config_path.display());
                }
                OutputFormat::Json => {
                    let output = json!({
                        "ok": true,
                        "command": "validate",
                        "repo_root": path_string(&loaded.repo_root),
                        "config_path": path_string(&loaded.config_path),
                    });
                    println!(
                        "{}",
                        serde_json::to_string(&output).expect("JSON serialization should succeed")
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            let category = error_category(&error);
            let (repo_root, config_path) = error_paths(&error);

            match args.format {
                OutputFormat::Human => {
                    eprintln!("FAIL validate [{category}]");
                    eprintln!("{error}");
                }
                OutputFormat::Json => {
                    let output = json!({
                        "ok": false,
                        "command": "validate",
                        "repo_root": repo_root.as_deref(),
                        "config_path": config_path.as_deref(),
                        "error": {
                            "category": category,
                            "message": error.to_string(),
                        },
                    });
                    println!(
                        "{}",
                        serde_json::to_string(&output).expect("JSON serialization should succeed")
                    );
                }
            }

            ExitCode::from(1)
        }
    }
}

fn error_category(error: &LoadError) -> &'static str {
    match error {
        LoadError::Discovery(_) => "discovery",
        LoadError::Parse(_) => "parse",
        LoadError::Validation(_) => "validation",
    }
}

fn error_paths(error: &LoadError) -> (Option<String>, Option<String>) {
    match error {
        LoadError::Discovery(error) => discovery_error_paths(error),
        LoadError::Parse(error) => parse_error_paths(error),
        LoadError::Validation(_) => (None, None),
    }
}

fn discovery_error_paths(error: &DiscoveryError) -> (Option<String>, Option<String>) {
    match error {
        DiscoveryError::MissingConfigAtRepoRoot {
            repo_root,
            config_path,
        } => (Some(path_string(repo_root)), Some(path_string(config_path))),
        DiscoveryError::ExplicitPathsMismatch {
            repo_root,
            config_path,
        } => (Some(path_string(repo_root)), Some(path_string(config_path))),
        DiscoveryError::InvalidRepoRoot { path, .. } => (Some(path_string(path)), None),
        DiscoveryError::InvalidExplicitConfigPath { path, .. } => (None, Some(path_string(path))),
        DiscoveryError::Io { path, .. } => (None, Some(path_string(path))),
        DiscoveryError::ConfigNotFound { .. } | DiscoveryError::CurrentDir { .. } => (None, None),
    }
}

fn parse_error_paths(error: &ParseError) -> (Option<String>, Option<String>) {
    match error {
        ParseError::InvalidUtf8 { path, .. } | ParseError::InvalidToml { path, .. } => {
            (None, Some(path_string(path)))
        }
    }
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
