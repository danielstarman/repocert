use std::path::Path;

use serde_json::{Map, Value, json};

use repocert::config::LoadPaths;

pub(crate) fn command_success(
    command: &str,
    paths: &LoadPaths,
    ok: bool,
    command_fields: impl Into<Map<String, Value>>,
) -> Value {
    let mut object = base_object(command, Some(paths));
    object.insert("ok".to_string(), Value::Bool(ok));
    object.insert("error".to_string(), Value::Null);
    object.extend(command_fields.into());
    Value::Object(object)
}

pub(crate) fn command_error(
    command: &str,
    paths: Option<&LoadPaths>,
    category: &str,
    message: String,
    error_details: Option<Map<String, Value>>,
) -> Value {
    let mut object = base_object(command, paths);
    object.insert("ok".to_string(), Value::Bool(false));

    let error = match error_details {
        Some(details) => json!({
            "category": category,
            "message": message,
            "details": details,
        }),
        None => json!({
            "category": category,
            "message": message,
        }),
    };

    object.insert("error".to_string(), error);
    Value::Object(object)
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

fn base_object(command: &str, paths: Option<&LoadPaths>) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("command".to_string(), Value::String(command.to_string()));
    object.insert(
        "repo_root".to_string(),
        paths
            .map(|paths| Value::String(path_string(&paths.repo_root)))
            .unwrap_or(Value::Null),
    );
    object.insert(
        "config_path".to_string(),
        paths
            .map(|paths| Value::String(path_string(&paths.config_path)))
            .unwrap_or(Value::Null),
    );
    object
}
