use std::process::ExitCode;

use serde_json::{Map, Value, json};

use repocert::config::LoadError;
use repocert::fix::{FixError, FixOptions, FixOutcome, FixReport, FixSelectionMode, run_fix};

use super::app::{FixArgs, OutputFormat};
use super::json::{command_error, command_success, execution_result};

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
    let mut command_fields = Map::new();
    command_fields.insert(
        "selection_mode".to_string(),
        Value::String(selection_mode_label(&report.selection_mode).to_string()),
    );
    command_fields.insert("profile".to_string(), json!(report.profile));
    command_fields.insert("fixers".to_string(), json!(report.fixers));
    command_fields.insert(
        "results".to_string(),
        Value::Array(
            report
                .results
                .iter()
                .map(|result| {
                    execution_result(
                        &result.name,
                        "fixer",
                        outcome_label(&result.outcome),
                        result.exit_code,
                        result.duration_ms,
                        result.message.as_deref(),
                    )
                })
                .collect(),
        ),
    );
    command_fields.insert(
        "summary".to_string(),
        json!({
            "total": report.summary.total,
            "pass": report.summary.pass,
            "fail": report.summary.fail,
            "timeout": report.summary.timeout,
        }),
    );
    let output = command_success("fix", &report.paths, report.ok(), command_fields);
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
    let output = command_error(
        "fix",
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
