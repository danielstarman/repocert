use std::path::Path;
use std::process::ExitCode;

use serde_json::json;

use repocert::config::LoadError;
use repocert::fix::{FixError, FixOptions, FixOutcome, FixReport, FixSelectionMode, run_fix};

use super::app::{FixArgs, OutputFormat};

pub(super) fn run(args: FixArgs) -> ExitCode {
    let options = FixOptions {
        load_options: repocert::config::LoadOptions {
            start_dir: None,
            repo_root: args.repo_root,
            config_path: args.config_path,
        },
        profile: args.profile,
        names: args.name,
        emit_progress: true,
    };

    match run_fix(options) {
        Ok(report) => {
            match args.format {
                OutputFormat::Human => render_human_success(&report),
                OutputFormat::Json => render_json_success(&report),
            }

            if report.ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
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

fn render_human_success(report: &FixReport) {
    let overall = if report.ok() { "PASS" } else { "FAIL" };
    println!("{overall} fix");
    println!("repo_root: {}", report.paths.repo_root.display());
    println!("config_path: {}", report.paths.config_path.display());
    println!(
        "selection_mode: {}",
        selection_mode_label(&report.selection_mode)
    );
    if let Some(profile) = &report.profile {
        println!("profile: {profile}");
    }
    if !report.fixers.is_empty() {
        println!("fixers: {}", report.fixers.join(", "));
    }

    for result in &report.results {
        let detail = result
            .exit_code
            .map(|code| format!(" exit_code={code}"))
            .or_else(|| result.message.as_ref().map(|message| format!(" {message}")))
            .unwrap_or_default();
        println!(
            "- fixer {} {} ({} ms){}",
            result.name,
            outcome_label(&result.outcome),
            result.duration_ms,
            detail
        );
    }

    println!(
        "summary: total={} pass={} fail={} timeout={}",
        report.summary.total, report.summary.pass, report.summary.fail, report.summary.timeout
    );
}

fn render_json_success(report: &FixReport) {
    let output = json!({
        "ok": report.ok(),
        "command": "fix",
        "repo_root": path_string(&report.paths.repo_root),
        "config_path": path_string(&report.paths.config_path),
        "selection_mode": selection_mode_label(&report.selection_mode),
        "profile": report.profile,
        "fixers": report.fixers,
        "results": report.results.iter().map(|result| {
            json!({
                "name": result.name,
                "outcome": outcome_label(&result.outcome),
                "exit_code": result.exit_code,
                "duration_ms": result.duration_ms,
                "message": result.message,
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "total": report.summary.total,
            "pass": report.summary.pass,
            "fail": report.summary.fail,
            "timeout": report.summary.timeout,
        },
        "error": serde_json::Value::Null,
    });
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn render_human_error(error: &FixError) {
    eprintln!("FAIL fix [{}]", error_category(error));
    eprintln!("{error}");
}

fn render_json_error(error: &FixError) {
    let paths = error.paths();
    let output = json!({
        "ok": false,
        "command": "fix",
        "repo_root": paths.map(|paths| path_string(&paths.repo_root)),
        "config_path": paths.map(|paths| path_string(&paths.config_path)),
        "error": {
            "category": error_category(error),
            "message": error.to_string(),
        },
    });
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn error_category(error: &FixError) -> &'static str {
    match error {
        FixError::Load(failure) => match &failure.error {
            LoadError::Discovery(_) => "discovery",
            LoadError::Parse(_) => "parse",
            LoadError::Validation(_) => "validation",
        },
        FixError::Selection { .. } => "selection",
    }
}

fn selection_mode_label(mode: &FixSelectionMode) -> &'static str {
    match mode {
        FixSelectionMode::Profile => "profile",
        FixSelectionMode::Fixers => "fixers",
    }
}

fn outcome_label(outcome: &FixOutcome) -> &'static str {
    match outcome {
        FixOutcome::Pass => "pass",
        FixOutcome::Fail => "fail",
        FixOutcome::Timeout => "timeout",
    }
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
