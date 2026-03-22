use std::process::ExitCode;

use serde_json::{Map, Value, json};

use repocert::check::{
    CheckError, CheckItemKind, CheckOptions, CheckOutcome, CheckReport, CheckSelectionMode,
    run_check,
};
use repocert::config::LoadError;

use super::app::{CheckArgs, OutputFormat};
use super::json::{command_error, command_success};

pub(super) fn run(args: CheckArgs) -> ExitCode {
    let options = CheckOptions {
        load_options: repocert::config::LoadOptions {
            start_dir: None,
            repo_root: args.repo_root,
            config_path: args.config_path,
        },
        profiles: args.profile,
        names: args.name,
        emit_progress: true,
    };

    match run_check(options) {
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

fn render_human_success(report: &CheckReport) {
    let overall = if report.ok() { "PASS" } else { "FAIL" };
    println!("{overall} check");
    println!("repo_root: {}", report.paths.repo_root.display());
    println!("config_path: {}", report.paths.config_path.display());
    println!(
        "selection_mode: {}",
        selection_mode_label(&report.selection_mode)
    );
    if !report.profiles.is_empty() {
        println!("profiles: {}", report.profiles.join(", "));
    }
    if !report.checks.is_empty() {
        println!("checks: {}", report.checks.join(", "));
    }

    for result in &report.results {
        let detail = result
            .exit_code
            .map(|code| format!(" exit_code={code}"))
            .or_else(|| result.message.as_ref().map(|message| format!(" {message}")))
            .unwrap_or_default();
        println!(
            "- {} {} {} ({} ms){}",
            item_kind_label(&result.kind),
            result.name,
            outcome_label(&result.outcome),
            result.duration_ms,
            detail
        );
    }

    println!(
        "summary: total={} pass={} fail={} timeout={} repair_needed={}",
        report.summary.total,
        report.summary.pass,
        report.summary.fail,
        report.summary.timeout,
        report.summary.repair_needed
    );
}

fn render_json_success(report: &CheckReport) {
    let mut command_fields = Map::new();
    command_fields.insert(
        "selection_mode".to_string(),
        Value::String(selection_mode_label(&report.selection_mode).to_string()),
    );
    command_fields.insert("profiles".to_string(), json!(report.profiles));
    command_fields.insert("checks".to_string(), json!(report.checks));
    command_fields.insert(
        "results".to_string(),
        Value::Array(
            report
                .results
                .iter()
                .map(|result| {
                    json!({
                        "name": result.name,
                        "kind": item_kind_label(&result.kind),
                        "outcome": outcome_label(&result.outcome),
                        "exit_code": result.exit_code,
                        "duration_ms": result.duration_ms,
                        "message": result.message,
                    })
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
            "repair_needed": report.summary.repair_needed,
        }),
    );

    let output = command_success("check", &report.paths, command_fields);
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn render_human_error(error: &CheckError) {
    eprintln!("FAIL check [{}]", error_category(error));
    eprintln!("{error}");
}

fn render_json_error(error: &CheckError) {
    let output = command_error(
        "check",
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

fn error_category(error: &CheckError) -> &'static str {
    match error {
        CheckError::Load(failure) => match &failure.error {
            LoadError::Discovery(_) => "discovery",
            LoadError::Parse(_) => "parse",
            LoadError::Validation(_) => "validation",
        },
        CheckError::Selection { .. } => "selection",
    }
}

fn selection_mode_label(mode: &CheckSelectionMode) -> &'static str {
    match mode {
        CheckSelectionMode::Profiles => "profiles",
        CheckSelectionMode::Checks => "checks",
    }
}

fn item_kind_label(kind: &CheckItemKind) -> &'static str {
    match kind {
        CheckItemKind::Check => "check",
        CheckItemKind::FixerProbe => "fixer_probe",
    }
}

fn outcome_label(outcome: &CheckOutcome) -> &'static str {
    match outcome {
        CheckOutcome::Pass => "pass",
        CheckOutcome::Fail => "fail",
        CheckOutcome::Timeout => "timeout",
        CheckOutcome::RepairNeeded => "repair_needed",
    }
}
