use std::process::ExitCode;

use serde_json::{Map, json};

use repocert::config::LoadPaths;
use repocert::hooks::{HookInstallMode, InstallHooksError, InstallHooksOptions, install_hooks};

use super::app::{InstallHooksArgs, OutputFormat};
use super::json::command_success;
use super::session::CommandRuntime;

pub(super) fn run(args: InstallHooksArgs) -> ExitCode {
    let executable_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(source) => {
            CommandRuntime::render_without_session(
                "install-hooks",
                args.format,
                "executable",
                &format!("failed to determine the current repocert executable path: {source}"),
                None,
            );
            return ExitCode::from(1);
        }
    };

    let options = InstallHooksOptions { executable_path };

    let runtime = match CommandRuntime::load(
        "install-hooks",
        args.format,
        args.repo_root,
        args.config_path,
    ) {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };

    match install_hooks(runtime.session(), options) {
        Ok(report) => {
            match runtime.format() {
                OutputFormat::Human => render_human_success(runtime.paths(), &report),
                OutputFormat::Json => render_json_success(runtime.paths(), &report),
            }
            ExitCode::SUCCESS
        }
        Err(error) => runtime.fail(error_category(&error), &error.to_string(), None),
    }
}

fn render_human_success(paths: &LoadPaths, report: &repocert::hooks::InstallHooksReport) {
    println!("PASS install-hooks");
    println!("repo_root: {}", paths.repo_root.display());
    println!("config_path: {}", paths.config_path.display());
    println!("mode: {}", mode_label(&report.mode));
    println!("hooks_path: {}", report.hooks_path.display());
    println!("changed: {}", report.changed);
    if !report.repaired_items.is_empty() {
        println!("repaired_items: {}", report.repaired_items.join(", "));
    }
}

fn render_json_success(paths: &LoadPaths, report: &repocert::hooks::InstallHooksReport) {
    let mut command_fields = Map::new();
    command_fields.insert("mode".to_string(), json!(mode_label(&report.mode)));
    command_fields.insert(
        "hooks_path".to_string(),
        json!(report.hooks_path.display().to_string()),
    );
    command_fields.insert("changed".to_string(), json!(report.changed));
    command_fields.insert("repaired_items".to_string(), json!(report.repaired_items));

    let output = command_success("install-hooks", paths, true, command_fields);
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn error_category(error: &InstallHooksError) -> &'static str {
    match error {
        InstallHooksError::MissingHooksConfig => "hooks",
        InstallHooksError::GitHooksPath(_) => "git",
        InstallHooksError::GitDir(_) => "git",
        InstallHooksError::GeneratedHookWrite { .. } => "hooks",
        InstallHooksError::GeneratedHookPrune { .. } => "hooks",
    }
}

fn mode_label(mode: &HookInstallMode) -> &'static str {
    match mode {
        HookInstallMode::Generated => "generated",
    }
}
