use std::process::ExitCode;

use serde_json::{Map, Value, json};

use repocert::certification::ContractFingerprint;
use repocert::certify::{
    CertifyError, CertifyItemKind, CertifyItemOutcome, CertifyOptions, CertifyProfileOutcome,
    CertifyReport, run_certify,
};
use repocert::config::LoadPaths;

use super::app::{CertifyArgs, OutputFormat};
use super::json::{command_success, execution_result, profile_outcome_result};
use super::session::CommandRuntime;

pub(super) fn run(args: CertifyArgs) -> ExitCode {
    let signing_key = args
        .signing_key
        .or_else(|| std::env::var_os("REPOCERT_SIGNING_KEY").map(Into::into));
    let options = CertifyOptions {
        profiles: args.profile,
        signing_key,
        emit_progress: true,
    };

    let runtime =
        match CommandRuntime::load("certify", args.format, args.repo_root, args.config_path) {
            Ok(runtime) => runtime,
            Err(code) => return code,
        };

    match run_certify(runtime.session(), options) {
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
        Err(error) => {
            let details = match &error {
                CertifyError::DirtyWorktree { dirty_paths } => {
                    let mut detail_fields = Map::new();
                    detail_fields.insert(
                        "dirty_paths".to_string(),
                        Value::Array(
                            dirty_paths
                                .split(", ")
                                .map(|path| Value::String(path.to_string()))
                                .collect(),
                        ),
                    );
                    Some(detail_fields)
                }
                _ => None,
            };
            runtime.fail(error_category(&error), &error.to_string(), details)
        }
    }
}

fn render_human_success(paths: &LoadPaths, report: &CertifyReport) {
    let overall = if report.ok() { "PASS" } else { "FAIL" };
    println!("{overall} certify");
    println!("repo_root: {}", paths.repo_root.display());
    println!("config_path: {}", paths.config_path.display());
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

fn render_json_success(paths: &LoadPaths, report: &CertifyReport) {
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
                    let mut extra_fields = Map::new();
                    extra_fields.insert("checks".to_string(), json!(profile.checks));
                    extra_fields
                        .insert("record_written".to_string(), json!(profile.record_written));
                    extra_fields.insert(
                        "item_results".to_string(),
                        Value::Array(
                            profile
                                .item_results
                                .iter()
                                .map(|result| {
                                    execution_result(
                                        &result.name,
                                        item_kind_label(&result.kind),
                                        item_outcome_label(&result.outcome),
                                        result.exit_code,
                                        result.duration_ms,
                                        result.message.as_deref(),
                                    )
                                })
                                .collect(),
                        ),
                    );
                    profile_outcome_result(
                        &profile.profile,
                        outcome_label(&profile.outcome),
                        extra_fields,
                    )
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
    let output = command_success("certify", paths, report.ok(), command_fields);
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn error_category(error: &CertifyError) -> &'static str {
    match error {
        CertifyError::Selection(_) => "selection",
        CertifyError::DirtyWorktree { .. } => "worktree",
        CertifyError::GitStatus(_) | CertifyError::GitCommit(_) => "git",
        CertifyError::Fingerprint(_) => "fingerprint",
        CertifyError::MissingSigningKeySelection | CertifyError::Signing { .. } => "signing",
        CertifyError::Storage(_) => "storage",
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
