use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use vestige_core::Storage;

pub fn schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "prefix": {
                "type": "string",
                "description": "Filter by tag prefix (e.g. 'tool', 'domain', 'lang', 'project'). Omit to list all."
            }
        }
    })
}

#[derive(Debug, Deserialize, Default)]
struct TagsArgs {
    prefix: Option<String>,
}

pub async fn execute(storage: &Arc<Storage>, args: Option<Value>) -> Result<Value, String> {
    let args: TagsArgs = match args {
        Some(v) => serde_json::from_value(v).map_err(|e| format!("Invalid arguments: {}", e))?,
        None => TagsArgs::default(),
    };

    let all_tags = storage.list_tags().map_err(|e| e.to_string())?;

    let filtered: Vec<&(String, i64)> = match &args.prefix {
        Some(prefix) => {
            let prefix_colon = format!("{}:", prefix);
            all_tags
                .iter()
                .filter(|(tag, _)| tag.starts_with(&prefix_colon))
                .collect()
        }
        None => all_tags.iter().collect(),
    };

    let categories: std::collections::HashMap<String, Vec<Value>> = {
        let mut map: std::collections::HashMap<String, Vec<Value>> =
            std::collections::HashMap::new();
        for (tag, count) in &filtered {
            let (cat, name) = tag
                .split_once(':')
                .unwrap_or(("uncategorized", tag.as_str()));
            map.entry(cat.to_string())
                .or_default()
                .push(serde_json::json!({ "tag": name, "count": count }));
        }
        map
    };

    Ok(serde_json::json!({
        "totalTags": filtered.len(),
        "categories": categories,
        "tags": filtered.iter().map(|item| {
            serde_json::json!({ "tag": item.0, "count": item.1 })
        }).collect::<Vec<_>>(),
    }))
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
    async fn test_list_tags_empty_db() {
        let (storage, _dir) = test_storage().await;
        let result = execute(&storage, None).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["totalTags"], 0);
    }

    #[tokio::test]
    async fn test_list_tags_with_data() {
        let (storage, _dir) = test_storage().await;
        storage
            .ingest(IngestInput {
                content: "Docker containers are great".to_string(),
                node_type: "fact".to_string(),
                source: None,
                sentiment_score: 0.0,
                sentiment_magnitude: 0.0,
                tags: vec!["tool:docker".to_string(), "domain:devops".to_string()],
                valid_from: None,
                valid_until: None,
            })
            .unwrap();

        let result = execute(&storage, None).await.unwrap();
        assert_eq!(result["totalTags"], 2);
    }

    #[tokio::test]
    async fn test_list_tags_with_prefix_filter() {
        let (storage, _dir) = test_storage().await;
        storage
            .ingest(IngestInput {
                content: "Kubernetes on AWS".to_string(),
                node_type: "fact".to_string(),
                source: None,
                sentiment_score: 0.0,
                sentiment_magnitude: 0.0,
                tags: vec![
                    "tool:kubernetes".to_string(),
                    "tool:aws".to_string(),
                    "domain:devops".to_string(),
                ],
                valid_from: None,
                valid_until: None,
            })
            .unwrap();

        let args = serde_json::json!({ "prefix": "tool" });
        let result = execute(&storage, Some(args)).await.unwrap();
        assert_eq!(result["totalTags"], 2);
    }
}
