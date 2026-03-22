use std::process::ExitCode;

use serde_json::Map;

use repocert::config::{LoadError, LoadOptions, load_contract};

use super::app::{OutputFormat, ValidateArgs};
use super::json::{command_error, command_success};

pub(super) fn run(args: ValidateArgs) -> ExitCode {
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
                    println!("repo_root: {}", loaded.paths.repo_root.display());
                    println!("config_path: {}", loaded.paths.config_path.display());
                }
                OutputFormat::Json => {
                    let output = command_success("validate", &loaded.paths, Map::new());
                    println!(
                        "{}",
                        serde_json::to_string(&output).expect("JSON serialization should succeed")
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(failure) => {
            let category = error_category(&failure.error);

            match args.format {
                OutputFormat::Human => {
                    eprintln!("FAIL validate [{category}]");
                    eprintln!("{}", failure.error);
                }
                OutputFormat::Json => {
                    let output = command_error(
                        "validate",
                        failure.paths.as_ref(),
                        category,
                        failure.error.to_string(),
                        None,
                    );
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
