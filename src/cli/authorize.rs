use std::process::ExitCode;

use serde_json::{Map, Value, json};

use repocert::config::LoadError;
use repocert::enforcement::{
    AuthorizeError, AuthorizeOptions, AuthorizeProfileResult, AuthorizeProfileState,
    AuthorizeReport, MatchedRule, authorize_ref_update,
};

use super::app::{AuthorizeArgs, OutputFormat};
use super::json::{command_error, command_success, matched_rule_result, profile_state_result};

pub(super) fn run(args: AuthorizeArgs) -> ExitCode {
    let options = AuthorizeOptions {
        load_options: repocert::config::LoadOptions {
            start_dir: None,
            repo_root: args.repo_root,
            config_path: args.config_path,
        },
        old: args.old,
        new: args.new,
        reference: args.reference,
    };

    match authorize_ref_update(options) {
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

fn render_human_success(report: &AuthorizeReport) {
    let overall = if report.allowed { "PASS" } else { "FAIL" };
    println!("{overall} authorize");
    println!("repo_root: {}", report.paths.repo_root.display());
    println!("config_path: {}", report.paths.config_path.display());
    println!("old: {}", report.old);
    println!("new: {}", report.new);
    println!("ref: {}", report.reference);
    println!("target_commit: {}", report.target_commit);
    println!(
        "contract_fingerprint: {}",
        report.contract_fingerprint.to_hex()
    );

    if !report.matched_rules.is_empty() {
        println!("matched_rules:");
        for rule in &report.matched_rules {
            println!("- {} -> {}", rule.pattern, rule.profile);
        }
    }

    if !report.required_profiles.is_empty() {
        println!("required_profiles: {}", report.required_profiles.join(", "));
    }

    for result in &report.profile_results {
        println!(
            "- profile {} {}",
            result.profile,
            state_label(&result.state)
        );
    }

    println!("allowed: {}", report.allowed);
}

fn render_json_success(report: &AuthorizeReport) {
    let mut command_fields = Map::new();
    command_fields.insert("old".to_string(), json!(report.old));
    command_fields.insert("new".to_string(), json!(report.new));
    command_fields.insert("ref".to_string(), json!(report.reference));
    command_fields.insert("target_commit".to_string(), json!(report.target_commit));
    command_fields.insert(
        "contract_fingerprint".to_string(),
        json!(report.contract_fingerprint.to_hex()),
    );
    command_fields.insert(
        "matched_rules".to_string(),
        Value::Array(report.matched_rules.iter().map(matched_rule_json).collect()),
    );
    command_fields.insert(
        "required_profiles".to_string(),
        Value::Array(
            report
                .required_profiles
                .iter()
                .map(|profile| Value::String(profile.clone()))
                .collect(),
        ),
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
    command_fields.insert("allowed".to_string(), Value::Bool(report.allowed));
    let output = command_success("authorize", &report.paths, report.ok(), command_fields);
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn render_human_error(error: &AuthorizeError) {
    eprintln!("FAIL authorize [{}]", error_category(error));
    eprintln!("{error}");
}

fn render_json_error(error: &AuthorizeError) {
    let output = command_error(
        "authorize",
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

fn error_category(error: &AuthorizeError) -> &'static str {
    match error {
        AuthorizeError::Load(failure) => match &failure.error {
            LoadError::Discovery(_) => "discovery",
            LoadError::Parse(_) => "parse",
            LoadError::Validation(_) => "validation",
        },
        AuthorizeError::UnsupportedDeletion { .. } => "input",
        AuthorizeError::GitCommit { .. } => "git",
        AuthorizeError::Fingerprint { .. } => "fingerprint",
        AuthorizeError::Storage { .. } => "storage",
        AuthorizeError::InvalidPattern { .. } => "pattern",
    }
}

fn matched_rule_json(rule: &MatchedRule) -> Value {
    matched_rule_result(&rule.pattern, &rule.profile)
}

fn profile_result_json(result: &AuthorizeProfileResult) -> Value {
    profile_state_result(&result.profile, state_label(&result.state), Map::new())
}

fn state_label(state: &AuthorizeProfileState) -> &'static str {
    match state {
        AuthorizeProfileState::Certified => "certified",
        AuthorizeProfileState::StaleCommit => "stale_commit",
        AuthorizeProfileState::StaleFingerprint => "stale_fingerprint",
        AuthorizeProfileState::Uncertified => "uncertified",
    }
}
