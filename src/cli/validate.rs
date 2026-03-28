use std::process::ExitCode;

use serde_json::Map;

use repocert::config::LoadPaths;

use super::app::{OutputFormat, ValidateArgs};
use super::json::command_success;
use super::session::CommandRuntime;

pub(super) fn run(args: ValidateArgs) -> ExitCode {
    let runtime =
        match CommandRuntime::load("validate", args.format, args.repo_root, args.config_path) {
            Ok(runtime) => runtime,
            Err(code) => return code,
        };

    match runtime.format() {
        OutputFormat::Human => render_human_success(runtime.paths()),
        OutputFormat::Json => render_json_success(runtime.paths()),
    }
    ExitCode::SUCCESS
}

fn render_human_success(paths: &LoadPaths) {
    println!("PASS validate");
    println!("repo_root: {}", paths.repo_root.display());
    println!("config_path: {}", paths.config_path.display());
}

fn render_json_success(paths: &LoadPaths) {
    let output = command_success("validate", paths, true, Map::new());
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}
