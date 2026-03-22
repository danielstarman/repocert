use std::process::ExitCode;

use serde_json::{Map, Value, json};

use repocert::certification::ContractFingerprint;
use repocert::certify::{
    CertifyError, CertifyItemKind, CertifyItemOutcome, CertifyOptions, CertifyProfileOutcome,
    CertifyReport, run_certify,
};
use repocert::config::LoadError;

use super::app::{CertifyArgs, OutputFormat};
use super::json::{command_error, command_success};

pub(super) fn run(args: CertifyArgs) -> ExitCode {
    let options = CertifyOptions {
        load_options: repocert::config::LoadOptions {
            start_dir: None,
            repo_root: args.repo_root,
            config_path: args.config_path,
        },
        profiles: args.profile,
        emit_progress: true,
    };

    match run_certify(options) {
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

fn render_human_success(report: &CertifyReport) {
    let overall = if report.ok() { "PASS" } else { "FAIL" };
    println!("{overall} certify");
    println!("repo_root: {}", report.paths.repo_root.display());
    println!("config_path: {}", report.paths.config_path.display());
    println!("commit: {}", report.commit);
    println!(
        "contract_fingerprint: {}",
        fingerprint_string(&report.contract_fingerprint)
    );
    println!("profiles: {}", report.profiles.join(", "));

    for profile in &report.profile_results {
        println!(
            "- profile {} {} record_written={}",
            profile.profile,
            outcome_label(&profile.outcome),
            profile.record_written
        );

        for result in &profile.item_results {
            let detail = result
                .exit_code
                .map(|code| format!(" exit_code={code}"))
                .or_else(|| result.message.as_ref().map(|message| format!(" {message}")))
                .unwrap_or_default();
            println!(
                "  {} {} {} ({} ms){}",
                item_kind_label(&result.kind),
                result.name,
                item_outcome_label(&result.outcome),
                result.duration_ms,
                detail
            );
        }
    }

    println!(
        "summary: total_profiles={} certified={} failed={} timeout={} repair_needed={}",
        report.summary.total_profiles,
        report.summary.certified,
        report.summary.failed,
        report.summary.timeout,
        report.summary.repair_needed
    );
}

fn render_json_success(report: &CertifyReport) {
    let mut command_fields = Map::new();
    command_fields.insert("commit".to_string(), json!(report.commit));
    command_fields.insert(
        "contract_fingerprint".to_string(),
        json!(fingerprint_string(&report.contract_fingerprint)),
    );
    command_fields.insert("profiles".to_string(), json!(report.profiles));
    command_fields.insert(
        "profile_results".to_string(),
        Value::Array(
            report
                .profile_results
                .iter()
                .map(|profile| {
                    json!({
                        "profile": profile.profile,
                        "checks": profile.checks,
                        "outcome": outcome_label(&profile.outcome),
                        "record_written": profile.record_written,
                        "item_results": profile
                            .item_results
                            .iter()
                            .map(|result| json!({
                                "name": result.name,
                                "kind": item_kind_label(&result.kind),
                                "outcome": item_outcome_label(&result.outcome),
                                "exit_code": result.exit_code,
                                "duration_ms": result.duration_ms,
                                "message": result.message,
                            }))
                            .collect::<Vec<_>>(),
                    })
                })
                .collect(),
        ),
    );
    command_fields.insert(
        "summary".to_string(),
        json!({
            "total_profiles": report.summary.total_profiles,
            "certified": report.summary.certified,
            "failed": report.summary.failed,
            "timeout": report.summary.timeout,
            "repair_needed": report.summary.repair_needed,
        }),
    );
    command_fields.insert("error".to_string(), Value::Null);

    let output = command_success("certify", &report.paths, report.ok(), command_fields);
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn render_human_error(error: &CertifyError) {
    eprintln!("FAIL certify [{}]", error_category(error));
    eprintln!("{error}");
}

fn render_json_error(error: &CertifyError) {
    let command_fields = match error {
        CertifyError::DirtyWorktree { dirty_paths, .. } => Some({
            let mut fields = Map::new();
            fields.insert(
                "dirty_paths".to_string(),
                Value::Array(
                    dirty_paths
                        .split(", ")
                        .map(|path| Value::String(path.to_string()))
                        .collect(),
                ),
            );
            fields
        }),
        _ => None,
    };
    let output = command_error(
        "certify",
        error.paths(),
        error_category(error),
        error.to_string(),
        command_fields,
    );
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn error_category(error: &CertifyError) -> &'static str {
    match error {
        CertifyError::Load(failure) => match &failure.error {
            LoadError::Discovery(_) => "discovery",
            LoadError::Parse(_) => "parse",
            LoadError::Validation(_) => "validation",
        },
        CertifyError::Selection { .. } => "selection",
        CertifyError::DirtyWorktree { .. } => "worktree",
        CertifyError::GitStatus { .. } | CertifyError::GitHead { .. } => "git",
        CertifyError::Fingerprint { .. } => "fingerprint",
        CertifyError::Storage { .. } => "storage",
    }
}

fn fingerprint_string(fingerprint: &ContractFingerprint) -> String {
    fingerprint.to_hex()
}

fn item_kind_label(kind: &CertifyItemKind) -> &'static str {
    match kind {
        CertifyItemKind::Check => "check",
        CertifyItemKind::FixerProbe => "fixer_probe",
    }
}

fn outcome_label(outcome: &CertifyProfileOutcome) -> &'static str {
    match outcome {
        CertifyProfileOutcome::Certified => "certified",
        CertifyProfileOutcome::Failed => "failed",
        CertifyProfileOutcome::TimedOut => "timeout",
        CertifyProfileOutcome::RepairNeeded => "repair_needed",
    }
}

fn item_outcome_label(outcome: &CertifyItemOutcome) -> &'static str {
    match outcome {
        CertifyItemOutcome::Pass => "pass",
        CertifyItemOutcome::Fail => "fail",
        CertifyItemOutcome::Timeout => "timeout",
        CertifyItemOutcome::RepairNeeded => "repair_needed",
    }
}
