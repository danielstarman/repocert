use std::process::ExitCode;

use serde_json::{Map, Value, json};

use repocert::config::LoadPaths;
use repocert::enforcement::{
    AuthorizeError, AuthorizeOptions, AuthorizeProfileResult, AuthorizeProfileState,
    AuthorizeReport, MatchedRule, authorize_ref_update,
};

use super::app::{AuthorizeArgs, OutputFormat};
use super::json::{command_success, matched_rule_result, profile_state_result};
use super::session::CommandRuntime;

pub(super) fn run(args: AuthorizeArgs) -> ExitCode {
    let options = AuthorizeOptions {
        old: args.old,
        new: args.new,
        reference: args.reference,
    };

    let runtime =
        match CommandRuntime::load("authorize", args.format, args.repo_root, args.config_path) {
            Ok(runtime) => runtime,
            Err(code) => return code,
        };

    match authorize_ref_update(runtime.session(), options) {
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

fn render_human_success(paths: &LoadPaths, report: &AuthorizeReport) {
    let overall = if report.allowed { "PASS" } else { "FAIL" };
    println!("{overall} authorize");
    println!("repo_root: {}", paths.repo_root.display());
    println!("config_path: {}", paths.config_path.display());
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
        if let Some(signer_name) = &result.signer_name {
            println!("  signer_name: {}", signer_name);
        }
    }

    println!("allowed: {}", report.allowed);
}

fn render_json_success(paths: &LoadPaths, report: &AuthorizeReport) {
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
    let output = command_success("authorize", paths, report.ok(), command_fields);
    println!(
        "{}",
        serde_json::to_string(&output).expect("JSON serialization should succeed")
    );
}

fn error_category(error: &AuthorizeError) -> &'static str {
    match error {
        AuthorizeError::UnsupportedDeletion => "input",
        AuthorizeError::GitCommit(_) => "git",
        AuthorizeError::Fingerprint(_) => "fingerprint",
        AuthorizeError::Storage(_) => "storage",
        AuthorizeError::InvalidPattern { .. } => "pattern",
    }
}

fn matched_rule_json(rule: &MatchedRule) -> Value {
    matched_rule_result(&rule.pattern, &rule.profile)
}

fn profile_result_json(result: &AuthorizeProfileResult) -> Value {
    let mut extra_fields = Map::new();
    extra_fields.insert("signer_name".to_string(), json!(result.signer_name));
    profile_state_result(&result.profile, state_label(&result.state), extra_fields)
}

fn state_label(state: &AuthorizeProfileState) -> &'static str {
    match state {
        AuthorizeProfileState::Certified => "certified",
        AuthorizeProfileState::UntrustedSigner => "untrusted_signer",
        AuthorizeProfileState::InvalidSignature => "invalid_signature",
        AuthorizeProfileState::StaleCommit => "stale_commit",
        AuthorizeProfileState::StaleFingerprint => "stale_fingerprint",
        AuthorizeProfileState::Uncertified => "uncertified",
    }
}
