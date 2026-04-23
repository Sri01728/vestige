use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use vestige_core::Storage;

const CONFIG_KEY: &str = "active_workspace";

pub fn schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["set", "get", "list", "new", "clear"],
                "description": "Action: 'set' activates a workspace (all future ingests auto-tagged), 'get' returns current workspace, 'list' shows all workspaces, 'new' creates and activates a workspace, 'clear' deactivates workspace (back to global)"
            },
            "name": {
                "type": "string",
                "description": "Workspace name (for set/new actions). Used as project:<name> tag."
            }
        },
        "required": ["action"]
    })
}

#[derive(Debug, Deserialize)]
struct WorkspaceArgs {
    action: String,
    name: Option<String>,
}

pub async fn execute(storage: &Arc<Storage>, args: Option<Value>) -> Result<Value, String> {
    let args: WorkspaceArgs = match args {
        Some(v) => serde_json::from_value(v).map_err(|e| format!("Invalid arguments: {}", e))?,
        None => return Err("Missing arguments".to_string()),
    };

    match args.action.as_str() {
        "get" => {
            let ws = storage.get_config(CONFIG_KEY).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "action": "get",
                "workspace": ws,
                "message": match &ws {
                    Some(w) => format!("Active workspace: {}", w),
                    None => "No active workspace (global mode)".to_string(),
                }
            }))
        }
        "set" => {
            let name = args.name.ok_or("Missing 'name' for set action")?;
            let name = name.to_lowercase().trim().to_string();
            if name.is_empty() {
                return Err("Workspace name cannot be empty".to_string());
            }
            storage.set_config(CONFIG_KEY, &name).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "action": "set",
                "workspace": name,
                "message": format!("Workspace set to '{}'. All new memories will be auto-tagged with project:{}.", name, name),
            }))
        }
        "new" => {
            let name = args.name.ok_or("Missing 'name' for new action")?;
            let name = name.to_lowercase().trim().to_string();
            if name.is_empty() {
                return Err("Workspace name cannot be empty".to_string());
            }
            storage.set_config(CONFIG_KEY, &name).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "action": "new",
                "workspace": name,
                "message": format!("Created and activated workspace '{}'. All new memories will be auto-tagged with project:{}.", name, name),
            }))
        }
        "clear" => {
            storage.set_config(CONFIG_KEY, "").map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "action": "clear",
                "workspace": null,
                "message": "Workspace cleared. Back to global mode — memories won't be auto-tagged with a project.",
            }))
        }
        "list" => {
            let all_tags = storage.list_tags().map_err(|e| e.to_string())?;
            let mut workspaces: Vec<(String, i64)> = Vec::new();
            for (tag, count) in &all_tags {
                if let Some(name) = tag.strip_prefix("project:").or_else(|| tag.strip_prefix("codebase:")) {
                    if !workspaces.iter().any(|(n, _)| n == name) {
                        workspaces.push((name.to_string(), *count));
                    }
                }
            }
            let active = storage.get_config(CONFIG_KEY).map_err(|e| e.to_string())?.unwrap_or_default();
            Ok(serde_json::json!({
                "action": "list",
                "active": if active.is_empty() { Value::Null } else { Value::String(active) },
                "workspaces": workspaces.iter().map(|(name, count)| {
                    serde_json::json!({ "name": name, "memories": count })
                }).collect::<Vec<_>>(),
                "total": workspaces.len(),
            }))
        }
        _ => Err(format!("Invalid action '{}'. Must be: set, get, list, new, clear", args.action)),
    }
}

pub fn get_active_workspace(storage: &Storage) -> Option<String> {
    storage.get_config(CONFIG_KEY).ok().flatten().filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn test_storage() -> (Arc<Storage>, TempDir) {
        let dir = TempDir::new().unwrap();
        let storage = Storage::new(Some(dir.path().join("test.db"))).unwrap();
        (Arc::new(storage), dir)
    }

    #[test]
    fn test_schema_is_valid() {
        let s = schema();
        assert_eq!(s["type"], "object");
    }

    #[tokio::test]
    async fn test_get_no_workspace() {
        let (storage, _dir) = test_storage().await;
        let result = execute(&storage, Some(serde_json::json!({"action": "get"}))).await;
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v["workspace"].is_null());
    }

    #[tokio::test]
    async fn test_set_and_get() {
        let (storage, _dir) = test_storage().await;
        execute(&storage, Some(serde_json::json!({"action": "set", "name": "myproject"}))).await.unwrap();
        let v = execute(&storage, Some(serde_json::json!({"action": "get"}))).await.unwrap();
        assert_eq!(v["workspace"], "myproject");
    }

    #[tokio::test]
    async fn test_clear() {
        let (storage, _dir) = test_storage().await;
        execute(&storage, Some(serde_json::json!({"action": "set", "name": "myproject"}))).await.unwrap();
        execute(&storage, Some(serde_json::json!({"action": "clear"}))).await.unwrap();
        let v = execute(&storage, Some(serde_json::json!({"action": "get"}))).await.unwrap();
        assert!(v["workspace"].is_null() || v["workspace"] == "");
    }
}
