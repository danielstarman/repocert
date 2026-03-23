use serde_json::{Map, Value, json};

pub(crate) fn execution_result(
    name: &str,
    kind: &str,
    outcome: &str,
    exit_code: Option<i32>,
    duration_ms: u64,
    message: Option<&str>,
) -> Value {
    json!({
        "name": name,
        "kind": kind,
        "outcome": outcome,
        "exit_code": exit_code,
        "duration_ms": duration_ms,
        "message": message,
    })
}

pub(crate) fn profile_outcome_result(
    profile: &str,
    outcome: &str,
    extra_fields: impl Into<Map<String, Value>>,
) -> Value {
    let mut object = Map::new();
    object.insert("profile".to_string(), Value::String(profile.to_string()));
    object.insert("outcome".to_string(), Value::String(outcome.to_string()));
    object.extend(extra_fields.into());
    Value::Object(object)
}

pub(crate) fn profile_state_result(
    profile: &str,
    state: &str,
    extra_fields: impl Into<Map<String, Value>>,
) -> Value {
    let mut object = Map::new();
    object.insert("profile".to_string(), Value::String(profile.to_string()));
    object.insert("state".to_string(), Value::String(state.to_string()));
    object.extend(extra_fields.into());
    Value::Object(object)
}

pub(crate) fn matched_rule_result(pattern: &str, profile: &str) -> Value {
    json!({
        "pattern": pattern,
        "profile": profile,
    })
}

pub(crate) fn protected_ref_result(pattern: &str, profile: &str, certified: bool) -> Value {
    json!({
        "pattern": pattern,
        "profile": profile,
        "certified": certified,
    })
}
