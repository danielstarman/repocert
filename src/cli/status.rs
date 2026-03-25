use std::process::ExitCode;

use serde_json::{Map, Value, json};

use repocert::certification::ContractFingerprint;
use repocert::config::LoadError;
use repocert::status::{
    ProtectedRefStatus, StatusError, StatusOptions, StatusProfileResult, StatusProfileState,
    StatusReport, run_status,
};

use super::app::{OutputFormat, StatusArgs};
use super::json::{command_error, command_success, profile_state_result, protected_ref_result};

pub(super) fn run(args: StatusArgs) -> ExitCode {
    let options = StatusOptions {
        load_options: repocert::config::LoadOptions {
            start_dir: None,
            repo_root: args.repo_root,
            config_path: args.config_path,
        },
        commit: args.commit,
        profiles: args.profile,
        assert_certified: args.assert_certified,
    };

    match run_status(options) {
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

fn render_human_success(report: &StatusReport) {
    if report.assert_certified {
        let overall = if report.ok() { "PASS" } else { "FAIL" };
        println!("{overall} status");
    } else {
        println!("STATUS");
    }
    println!("repo_root: {}", report.paths.repo_root.display());
    println!("config_path: {}", report.paths.config_path.display());
    println!("commit: {}", report.commit);
    println!(
        "contract_fingerprint: {}",
        fingerprint_string(&report.contract_fingerprint)
    );
    println!("assert_certified: {}", report.assert_certified);
    println!("profiles: {}", report.profiles.join(", "));

    for result in &report.profile_results {
        println!(
            "- profile {} {}",
            result.profile,
            profile_state_label(&result.state)
        );
        if !result.other_certified_commits.is_empty() {
            println!(
                "  other_certified_commits: {}",
                result.other_certified_commits.join(", ")
            );
        }
        if let Some(signer_name) = &result.signer_name {
            println!("  signer_name: {}", signer_name);
        }
        if let Some(fingerprint) = &result.recorded_fingerprint {
            println!(
                "  recorded_fingerprint: {}",
                fingerprint_string(fingerprint)
            );
        }
    }

    if !report.protected_refs.is_empty() {
        println!("protected_refs:");
        for rule in &report.protected_refs {
            println!(
                "- {} -> {} certified={}",
                rule.pattern, rule.profile, rule.certified
            );
        }
    }

    println!(
        "summary: total_profiles={} certified={} legacy_unsigned={} untrusted_signer={} invalid_signature={} stale_commit={} stale_fingerprint={} uncertified={}",
        report.summary.total_profiles,
        report.summary.certified,
        report.summary.legacy_unsigned,
        report.summary.untrusted_signer,
        report.summary.invalid_signature,
        report.summary.stale_commit,
        report.summary.stale_fingerprint,
        report.summary.uncertified
    );
}

fn render_json_success(report: &StatusReport) {
    let mut command_fields = Map::new();
    command_fields.insert("commit".to_string(), json!(report.commit));
    command_fields.insert(
        "contract_fingerprint".to_string(),
        json!(fingerprint_string(&report.contract_fingerprint)),
    );
    command_fields.insert("profiles".to_string(), json!(report.profiles));
    command_fields.insert(
        "assert_certified".to_string(),
        json!(report.assert_certified),
    );
    command_fields.insert(
        "profile_results".to_string(),
        Value::Array(
            report
                .profile_results
                .iter()
                .map(profile_result_json)
                .collect(),
        ),
    );
    command_fields.insert(
        "protected_refs".to_string(),
        Value::Array(
            report
                .protected_refs
                .iter()
                .map(protected_ref_json)
                .collect(),
        ),
    );
    command_fields.insert(
        "summary".to_string(),
        json!({
            "total_profiles": report.summary.total_profiles,
            "certified": report.summary.certified,
            "legacy_unsigned": report.summary.legacy_unsigned,
            "untrusted_signer": report.summary.untrusted_signer,
            "invalid_signature": report.summary.invalid_signature,
            "stale_commit": report.summary.stale_commit,
            "stale_fingerprint": report.summary.stale_fingerprint,
            "uncertified": report.summary.uncertified,
        }),
    );
    let output = command_success("status", &report.paths, report.ok(), command_fields);
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn render_human_error(error: &StatusError) {
    eprintln!("FAIL status [{}]", error_category(error));
    eprintln!("{error}");
}

fn render_json_error(error: &StatusError) {
    let output = command_error(
        "status",
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

fn error_category(error: &StatusError) -> &'static str {
    match error {
        StatusError::Load(failure) => match &failure.error {
            LoadError::Discovery(_) => "discovery",
            LoadError::Parse(_) => "parse",
            LoadError::Validation(_) => "validation",
        },
        StatusError::Selection { .. } => "selection",
        StatusError::GitCommit { .. } => "git",
        StatusError::Fingerprint { .. } => "fingerprint",
        StatusError::Storage { .. } => "storage",
    }
}

fn profile_result_json(result: &StatusProfileResult) -> Value {
    let mut extra_fields = Map::new();
    extra_fields.insert(
        "other_certified_commits".to_string(),
        json!(result.other_certified_commits),
    );
    extra_fields.insert("signer_name".to_string(), json!(result.signer_name));
    extra_fields.insert(
        "recorded_fingerprint".to_string(),
        json!(result.recorded_fingerprint.as_ref().map(fingerprint_string)),
    );
    profile_state_result(
        &result.profile,
        profile_state_label(&result.state),
        extra_fields,
    )
}

fn protected_ref_json(result: &ProtectedRefStatus) -> Value {
    protected_ref_result(&result.pattern, &result.profile, result.certified)
}

fn fingerprint_string(fingerprint: &ContractFingerprint) -> String {
    fingerprint.to_hex()
}

fn profile_state_label(state: &StatusProfileState) -> &'static str {
    match state {
        StatusProfileState::Certified => "certified",
        StatusProfileState::LegacyUnsigned => "legacy_unsigned",
        StatusProfileState::UntrustedSigner => "untrusted_signer",
        StatusProfileState::InvalidSignature => "invalid_signature",
        StatusProfileState::StaleCommit => "stale_commit",
        StatusProfileState::StaleFingerprint => "stale_fingerprint",
        StatusProfileState::Uncertified => "uncertified",
    }
}
