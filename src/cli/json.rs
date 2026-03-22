use std::path::Path;

use serde_json::{Map, Value, json};

use repocert::config::LoadPaths;

pub(super) fn command_success(
    command: &str,
    paths: &LoadPaths,
    command_fields: impl Into<Map<String, Value>>,
) -> Value {
    let mut object = base_object(command, Some(paths));
    object.insert("ok".to_string(), Value::Bool(true));
    object.extend(command_fields.into());
    Value::Object(object)
}

pub(super) fn command_error(
    command: &str,
    paths: Option<&LoadPaths>,
    category: &str,
    message: String,
    command_fields: Option<Map<String, Value>>,
) -> Value {
    let mut object = base_object(command, paths);
    object.insert("ok".to_string(), Value::Bool(false));
    object.insert(
        "error".to_string(),
        json!({
            "category": category,
            "message": message,
        }),
    );
    if let Some(command_fields) = command_fields {
        object.extend(command_fields);
    }
    Value::Object(object)
}

pub(super) fn path_string(path: &Path) -> String {
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
