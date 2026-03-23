use std::process::ExitCode;

use serde_json::{Map, json};

use repocert::config::LoadError;
use repocert::hooks::{HookInstallMode, InstallHooksError, InstallHooksOptions, install_hooks};

use super::app::{InstallHooksArgs, OutputFormat};
use super::json::{command_error, command_success};

pub(super) fn run(args: InstallHooksArgs) -> ExitCode {
    let executable_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(source) => {
            let error = InstallHooksError::CurrentExecutable {
                paths: None,
                source,
            };
            match args.format {
                OutputFormat::Human => render_human_error(&error),
                OutputFormat::Json => render_json_error(&error),
            }
            return ExitCode::from(1);
        }
    };

    let options = InstallHooksOptions {
        load_options: repocert::config::LoadOptions {
            start_dir: None,
            repo_root: args.repo_root,
            config_path: args.config_path,
        },
        executable_path,
    };

    match install_hooks(options) {
        Ok(report) => {
            match args.format {
                OutputFormat::Human => render_human_success(&report),
                OutputFormat::Json => render_json_success(&report),
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            match args.format {
                OutputFormat::Human => render_human_error(&error),
                OutputFormat::Json => render_json_error(&error),
            }
            ExitCode::from(1)
        }
    }
}

fn render_human_success(report: &repocert::hooks::InstallHooksReport) {
    println!("PASS install-hooks");
    println!("repo_root: {}", report.paths.repo_root.display());
    println!("config_path: {}", report.paths.config_path.display());
    println!("mode: {}", mode_label(&report.mode));
    println!("hooks_path: {}", report.hooks_path.display());
    println!("changed: {}", report.changed);
    if !report.repaired_items.is_empty() {
        println!("repaired_items: {}", report.repaired_items.join(", "));
    }
}

fn render_json_success(report: &repocert::hooks::InstallHooksReport) {
    let mut command_fields = Map::new();
    command_fields.insert("mode".to_string(), json!(mode_label(&report.mode)));
    command_fields.insert(
        "hooks_path".to_string(),
        json!(report.hooks_path.display().to_string()),
    );
    command_fields.insert("changed".to_string(), json!(report.changed));
    command_fields.insert("repaired_items".to_string(), json!(report.repaired_items));

    let output = command_success("install-hooks", &report.paths, true, command_fields);
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn render_human_error(error: &InstallHooksError) {
    eprintln!("FAIL install-hooks [{}]", error_category(error));
    eprintln!("{error}");
}

fn render_json_error(error: &InstallHooksError) {
    let output = command_error(
        "install-hooks",
        error.paths(),
        error_category(error),
        error.to_string(),
        None,
    );
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn error_category(error: &InstallHooksError) -> &'static str {
    match error {
        InstallHooksError::Load(failure) => match &failure.error {
            LoadError::Discovery(_) => "discovery",
            LoadError::Parse(_) => "parse",
            LoadError::Validation(_) => "validation",
        },
        InstallHooksError::MissingHooksConfig { .. } => "hooks",
        InstallHooksError::GitHooksPath { .. } => "git",
        InstallHooksError::GitDir { .. } => "git",
        InstallHooksError::MissingRepoOwnedHookDir { .. } => "hooks",
        InstallHooksError::UnsupportedGeneratedHook { .. } => "hooks",
        InstallHooksError::CurrentExecutable { .. } => "executable",
        InstallHooksError::GeneratedHookWrite { .. } => "hooks",
        InstallHooksError::GeneratedHookPrune { .. } => "hooks",
    }
}

fn mode_label(mode: &HookInstallMode) -> &'static str {
    match mode {
        HookInstallMode::RepoOwned => "repo-owned",
        HookInstallMode::Generated => "generated",
    }
}
