use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use vestige_core::Storage;

pub fn schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "project": {
                "type": "string",
                "description": "Project name to export (matches project: or codebase: tags). Omit for all memories."
            },
            "min_retention": {
                "type": "number",
                "description": "Minimum retention strength to include (default: 0.3)",
                "default": 0.3
            }
        }
    })
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ExportArgs {
    project: Option<String>,
    min_retention: Option<f64>,
}

pub async fn execute(storage: &Arc<Storage>, args: Option<Value>) -> Result<Value, String> {
    let args: ExportArgs = match args {
        Some(v) => serde_json::from_value(v).map_err(|e| format!("Invalid arguments: {}", e))?,
        None => ExportArgs::default(),
    };

    let min_retention = args.min_retention.unwrap_or(0.3);
    let nodes = storage.get_all_nodes(500, 0).map_err(|e| e.to_string())?;

    let filtered: Vec<_> = nodes
        .into_iter()
        .filter(|n| n.retention_strength >= min_retention)
        .filter(|n| {
            if let Some(ref project) = args.project {
                let p_lower = project.to_lowercase();
                n.tags.iter().any(|t| {
                    let t_lower = t.to_lowercase();
                    t_lower == format!("project:{}", p_lower)
                        || t_lower == format!("codebase:{}", p_lower)
                })
            } else {
                true
            }
        })
        .collect();

    if filtered.is_empty() {
        return Ok(serde_json::json!({
            "success": true,
            "markdown": "# CLAUDE.md\n\nNo memories found matching the criteria.\n",
            "memoryCount": 0,
        }));
    }

    let mut decisions: Vec<&str> = Vec::new();
    let mut patterns: Vec<&str> = Vec::new();
    let mut facts: Vec<&str> = Vec::new();
    let mut preferences: Vec<&str> = Vec::new();
    let mut other: Vec<(&str, &str)> = Vec::new();

    for n in &filtered {
        match n.node_type.as_str() {
            "decision" => decisions.push(&n.content),
            "pattern" => patterns.push(&n.content),
            "fact" => facts.push(&n.content),
            "note" if n.tags.iter().any(|t| t.contains("preference") || t.contains("user")) => {
                preferences.push(&n.content)
            }
            _ => other.push((&n.node_type, &n.content)),
        }
    }

    let mut md = String::new();
    let title = args.project.as_deref().unwrap_or("Project");
    md.push_str(&format!("# {} — CLAUDE.md\n\n", title));
    md.push_str(&format!(
        "_Generated from Vestige memory ({} memories, min retention {:.0}%)_\n\n",
        filtered.len(),
        min_retention * 100.0
    ));

    if !decisions.is_empty() {
        md.push_str("## Decisions\n\n");
        for d in &decisions {
            md.push_str(&format!("- {}\n", first_line(d)));
        }
        md.push('\n');
    }

    if !patterns.is_empty() {
        md.push_str("## Patterns & Conventions\n\n");
        for p in &patterns {
            md.push_str(&format!("- {}\n", first_line(p)));
        }
        md.push('\n');
    }

    if !preferences.is_empty() {
        md.push_str("## Preferences\n\n");
        for p in &preferences {
            md.push_str(&format!("- {}\n", first_line(p)));
        }
        md.push('\n');
    }

    if !facts.is_empty() {
        md.push_str("## Knowledge\n\n");
        for f in &facts {
            md.push_str(&format!("- {}\n", first_line(f)));
        }
        md.push('\n');
    }

    if !other.is_empty() {
        md.push_str("## Other\n\n");
        for (node_type, content) in &other {
            md.push_str(&format!("- [{}] {}\n", node_type, first_line(content)));
        }
        md.push('\n');
    }

    Ok(serde_json::json!({
        "success": true,
        "markdown": md,
        "memoryCount": filtered.len(),
        "sections": {
            "decisions": decisions.len(),
            "patterns": patterns.len(),
            "preferences": preferences.len(),
            "facts": facts.len(),
            "other": other.len(),
        }
    }))
}

fn first_line(content: &str) -> &str {
    let content = content.trim();
    let end = content.find('\n').unwrap_or(content.len()).min(200);
    let end = content.floor_char_boundary(end);
    &content[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use vestige_core::IngestInput;

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
    async fn test_export_empty_db() {
        let (storage, _dir) = test_storage().await;
        let result = execute(&storage, None).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["memoryCount"], 0);
    }

    #[tokio::test]
    async fn test_export_with_memories() {
        let (storage, _dir) = test_storage().await;
        storage
            .ingest(IngestInput {
                content: "Use Result<T, E> for error handling".to_string(),
                node_type: "pattern".to_string(),
                source: None,
                sentiment_score: 0.0,
                sentiment_magnitude: 0.0,
                tags: vec!["codebase:myapp".to_string()],
                valid_from: None,
                valid_until: None,
            })
            .unwrap();

        let args = serde_json::json!({ "project": "myapp" });
        let result = execute(&storage, Some(args)).await.unwrap();
        assert_eq!(result["memoryCount"], 1);
        let md = result["markdown"].as_str().unwrap();
        assert!(md.contains("## Patterns"));
    }
}
