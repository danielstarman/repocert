use std::process::ExitCode;

use serde_json::{Map, Value, json};

use repocert::check::{
    CheckError, CheckItemKind, CheckOptions, CheckOutcome, CheckReport, CheckSelectionMode,
    run_check,
};
use repocert::config::LoadPaths;

use super::app::{CheckArgs, OutputFormat};
use super::json::{command_success, execution_result};
use super::session::CommandRuntime;

pub(super) fn run(args: CheckArgs) -> ExitCode {
    let options = CheckOptions {
        profiles: args.profile,
        names: args.name,
        emit_progress: true,
    };

    let runtime = match CommandRuntime::load("check", args.format, args.repo_root, args.config_path)
    {
        Ok(runtime) => runtime,
        Err(code) => return code,
    };

    match run_check(runtime.session(), options) {
        Ok(report) => {
            match runtime.format() {
                OutputFormat::Human => render_human_success(runtime.paths(), &report),
                OutputFormat::Json => render_json_success(runtime.paths(), &report),
            }

            if report.ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => runtime.fail(error_category(&error), &error.to_string(), None),
    }
}

fn render_human_success(paths: &LoadPaths, report: &CheckReport) {
    let overall = if report.ok() { "PASS" } else { "FAIL" };
    println!("{overall} check");
    println!("repo_root: {}", paths.repo_root.display());
    println!("config_path: {}", paths.config_path.display());
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

fn render_json_success(paths: &LoadPaths, report: &CheckReport) {
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
                    execution_result(
                        &result.name,
                        item_kind_label(&result.kind),
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
            "repair_needed": report.summary.repair_needed,
        }),
    );

    let output = command_success("check", paths, report.ok(), command_fields);
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn error_category(error: &CheckError) -> &'static str {
    match error {
        CheckError::Selection(_) => "selection",
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
