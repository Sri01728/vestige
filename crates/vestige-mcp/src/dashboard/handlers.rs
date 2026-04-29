//! Dashboard API endpoint handlers
//!
//! v2.0: Adds cognitive operation endpoints (dream, explore, predict, importance, consolidation)

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Json, Redirect};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::Value;

use super::events::VestigeEvent;
use super::state::AppState;

/// Redirect root to the SvelteKit dashboard
pub async fn serve_dashboard() -> Redirect {
    Redirect::permanent("/dashboard")
}

#[derive(Debug, Deserialize)]
pub struct MemoryListParams {
    pub q: Option<String>,
    pub node_type: Option<String>,
    pub tag: Option<String>,
    pub min_retention: Option<f64>,
    pub sort: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// List memories with optional search
pub async fn list_memories(
    State(state): State<AppState>,
    Query(params): Query<MemoryListParams>,
) -> Result<Json<Value>, StatusCode> {
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let offset = params.offset.unwrap_or(0).max(0);

    if let Some(query) = params.q.as_ref().filter(|q| !q.trim().is_empty()) {
        // Use hybrid search
        let results = state
            .storage
            .hybrid_search(query, limit, 0.3, 0.7)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let formatted: Vec<Value> = results
            .into_iter()
            .filter(|r| {
                if let Some(min_ret) = params.min_retention {
                    r.node.retention_strength >= min_ret
                } else {
                    true
                }
            })
            .map(|r| {
                serde_json::json!({
                    "id": r.node.id,
                    "content": r.node.content,
                    "nodeType": r.node.node_type,
                    "tags": r.node.tags,
                    "retentionStrength": r.node.retention_strength,
                    "storageStrength": r.node.storage_strength,
                    "retrievalStrength": r.node.retrieval_strength,
                    "createdAt": r.node.created_at.to_rfc3339(),
                    "updatedAt": r.node.updated_at.to_rfc3339(),
                    "combinedScore": r.combined_score,
                    "source": r.node.source,
                    "reviewCount": r.node.reps,
                })
            })
            .collect();

        return Ok(Json(serde_json::json!({
            "total": formatted.len(),
            "memories": formatted,
        })));
    }

    // No search query — list all memories
    let mut nodes = state
        .storage
        .get_all_nodes(limit, offset)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Apply filters
    if let Some(ref node_type) = params.node_type {
        nodes.retain(|n| n.node_type == *node_type);
    }
    if let Some(ref tag) = params.tag {
        nodes.retain(|n| n.tags.iter().any(|t| t == tag));
    }
    if let Some(min_ret) = params.min_retention {
        nodes.retain(|n| n.retention_strength >= min_ret);
    }

    let formatted: Vec<Value> = nodes
        .iter()
        .map(|n| {
            serde_json::json!({
                "id": n.id,
                "content": n.content,
                "nodeType": n.node_type,
                "tags": n.tags,
                "retentionStrength": n.retention_strength,
                "storageStrength": n.storage_strength,
                "retrievalStrength": n.retrieval_strength,
                "createdAt": n.created_at.to_rfc3339(),
                "updatedAt": n.updated_at.to_rfc3339(),
                "source": n.source,
                "reviewCount": n.reps,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "total": formatted.len(),
        "memories": formatted,
    })))
}

/// Get a single memory by ID
pub async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let node = state
        .storage
        .get_node(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "id": node.id,
        "content": node.content,
        "nodeType": node.node_type,
        "tags": node.tags,
        "retentionStrength": node.retention_strength,
        "storageStrength": node.storage_strength,
        "retrievalStrength": node.retrieval_strength,
        "sentimentScore": node.sentiment_score,
        "sentimentMagnitude": node.sentiment_magnitude,
        "source": node.source,
        "createdAt": node.created_at.to_rfc3339(),
        "updatedAt": node.updated_at.to_rfc3339(),
        "lastAccessedAt": node.last_accessed.to_rfc3339(),
        "nextReviewAt": node.next_review.map(|dt| dt.to_rfc3339()),
        "reviewCount": node.reps,
        "validFrom": node.valid_from.map(|dt| dt.to_rfc3339()),
        "validUntil": node.valid_until.map(|dt| dt.to_rfc3339()),
    })))
}

/// Delete a memory by ID
pub async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let deleted = state
        .storage
        .delete_node(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        state.emit(VestigeEvent::MemoryDeleted {
            id: id.clone(),
            timestamp: chrono::Utc::now(),
        });
        Ok(Json(serde_json::json!({ "deleted": true, "id": id })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Promote a memory
pub async fn promote_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let node = state
        .storage
        .promote_memory(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.emit(VestigeEvent::MemoryPromoted {
        id: node.id.clone(),
        new_retention: node.retention_strength,
        timestamp: chrono::Utc::now(),
    });

    Ok(Json(serde_json::json!({
        "promoted": true,
        "id": node.id,
        "retentionStrength": node.retention_strength,
    })))
}

/// Demote a memory
pub async fn demote_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let node = state
        .storage
        .demote_memory(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.emit(VestigeEvent::MemoryDemoted {
        id: node.id.clone(),
        new_retention: node.retention_strength,
        timestamp: chrono::Utc::now(),
    });

    Ok(Json(serde_json::json!({
        "demoted": true,
        "id": node.id,
        "retentionStrength": node.retention_strength,
    })))
}

/// Get system stats
pub async fn get_stats(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let stats = state
        .storage
        .get_stats()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let embedding_coverage = if stats.total_nodes > 0 {
        (stats.nodes_with_embeddings as f64 / stats.total_nodes as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(serde_json::json!({
        "totalMemories": stats.total_nodes,
        "dueForReview": stats.nodes_due_for_review,
        "averageRetention": stats.average_retention,
        "averageStorageStrength": stats.average_storage_strength,
        "averageRetrievalStrength": stats.average_retrieval_strength,
        "withEmbeddings": stats.nodes_with_embeddings,
        "embeddingCoverage": embedding_coverage,
        "embeddingModel": stats.embedding_model,
        "oldestMemory": stats.oldest_memory.map(|dt| dt.to_rfc3339()),
        "newestMemory": stats.newest_memory.map(|dt| dt.to_rfc3339()),
    })))
}

#[derive(Debug, Deserialize)]
pub struct TimelineParams {
    pub days: Option<i64>,
    pub limit: Option<i32>,
}

/// Get timeline data
pub async fn get_timeline(
    State(state): State<AppState>,
    Query(params): Query<TimelineParams>,
) -> Result<Json<Value>, StatusCode> {
    let days = params.days.unwrap_or(7).clamp(1, 90);
    let limit = params.limit.unwrap_or(200).clamp(1, 500);

    let start = Utc::now() - Duration::days(days);
    let nodes = state
        .storage
        .query_time_range(Some(start), Some(Utc::now()), limit, None, None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Group by day
    let mut by_day: std::collections::BTreeMap<String, Vec<Value>> =
        std::collections::BTreeMap::new();
    for node in &nodes {
        let date = node.created_at.format("%Y-%m-%d").to_string();
        let content_preview: String = {
            let preview: String = node.content.chars().take(100).collect();
            if preview.len() < node.content.len() {
                format!("{}...", preview)
            } else {
                preview
            }
        };
        by_day.entry(date).or_default().push(serde_json::json!({
            "id": node.id,
            "content": content_preview,
            "nodeType": node.node_type,
            "retentionStrength": node.retention_strength,
            "createdAt": node.created_at.to_rfc3339(),
        }));
    }

    let timeline: Vec<Value> = by_day
        .into_iter()
        .rev()
        .map(|(date, memories)| {
            serde_json::json!({
                "date": date,
                "count": memories.len(),
                "memories": memories,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "days": days,
        "totalMemories": nodes.len(),
        "timeline": timeline,
    })))
}

/// Health check
pub async fn health_check(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let stats = state
        .storage
        .get_stats()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let status = if stats.total_nodes == 0 {
        "empty"
    } else if stats.average_retention < 0.3 {
        "critical"
    } else if stats.average_retention < 0.5 {
        "degraded"
    } else {
        "healthy"
    };

    Ok(Json(serde_json::json!({
        "status": status,
        "totalMemories": stats.total_nodes,
        "averageRetention": stats.average_retention,
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

// ============================================================================
// MEMORY GRAPH
// ============================================================================

/// Redirect legacy graph to SvelteKit dashboard graph page
pub async fn serve_graph() -> Redirect {
    Redirect::permanent("/dashboard/graph")
}

#[derive(Debug, Deserialize)]
pub struct GraphParams {
    pub query: Option<String>,
    pub center_id: Option<String>,
    pub depth: Option<u32>,
    pub max_nodes: Option<usize>,
    pub workspace: Option<String>,
}

/// Get memory graph data (nodes + edges with layout positions)
pub async fn get_graph(
    State(state): State<AppState>,
    Query(params): Query<GraphParams>,
) -> Result<Json<Value>, StatusCode> {
    let depth = params.depth.unwrap_or(2).clamp(1, 3);
    let max_nodes = params.max_nodes.unwrap_or(50).clamp(1, 200);

    // Determine graph data: if center_id/query specified, do subgraph BFS.
    // Otherwise load ALL nodes so every cluster is visible.
    let (nodes, edges, center_id) = if let Some(ref id) = params.center_id {
        let (n, e) = state
            .storage
            .get_memory_subgraph(id, depth, max_nodes)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        (n, e, id.clone())
    } else if let Some(ref query) = params.query {
        let results = state
            .storage
            .search(query, 1)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let cid = results
            .first()
            .map(|n| n.id.clone())
            .ok_or(StatusCode::NOT_FOUND)?;
        let (n, e) = state
            .storage
            .get_memory_subgraph(&cid, depth, max_nodes)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        (n, e, cid)
    } else {
        let mut all_nodes = state
            .storage
            .get_all_nodes(max_nodes as i32, 0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        // Workspace filter: only show nodes with matching project:/codebase: tag
        if let Some(ref ws) = params.workspace {
            let ws_lower = ws.to_lowercase();
            let project_tag = format!("project:{}", ws_lower);
            let codebase_tag = format!("codebase:{}", ws_lower);
            all_nodes.retain(|n| {
                n.tags.iter().any(|t| {
                    let tl = t.to_lowercase();
                    tl == project_tag || tl == codebase_tag
                })
            });
        }
        let node_ids: std::collections::HashSet<String> =
            all_nodes.iter().map(|n| n.id.clone()).collect();
        let mut all_connections = state
            .storage
            .get_all_connections()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        // Filter edges to only include nodes in the workspace
        if params.workspace.is_some() {
            all_connections.retain(|c| {
                node_ids.contains(&c.source_id) && node_ids.contains(&c.target_id)
            });
        }
        let cid = all_nodes.first().map(|n| n.id.clone()).unwrap_or_default();
        (all_nodes, all_connections, cid)
    };

    if nodes.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Build nodes JSON with timestamps for recency calculation
    let nodes_json: Vec<Value> = nodes
        .iter()
        .map(|n| {
            let label = if n.content.chars().count() > 80 {
                format!("{}...", n.content.chars().take(77).collect::<String>())
            } else {
                n.content.clone()
            };
            serde_json::json!({
                "id": n.id,
                "label": label,
                "type": n.node_type,
                "retention": n.retention_strength,
                "tags": n.tags,
                "createdAt": n.created_at.to_rfc3339(),
                "updatedAt": n.updated_at.to_rfc3339(),
                "isCenter": n.id == center_id,
            })
        })
        .collect();

    let edges_json: Vec<Value> = edges
        .iter()
        .map(|e| {
            serde_json::json!({
                "source": e.source_id,
                "target": e.target_id,
                "weight": e.strength,
                "type": e.link_type,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "nodes": nodes_json,
        "edges": edges_json,
        "center_id": center_id,
        "depth": depth,
        "nodeCount": nodes.len(),
        "edgeCount": edges.len(),
    })))
}

// ============================================================================
// SEARCH (dedicated endpoint)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<i32>,
    pub min_retention: Option<f64>,
}

/// Search memories with hybrid search
pub async fn search_memories(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, StatusCode> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let start = std::time::Instant::now();

    let results = state
        .storage
        .hybrid_search(&params.q, limit, 0.3, 0.7)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let duration_ms = start.elapsed().as_millis() as u64;

    let result_ids: Vec<String> = results.iter().map(|r| r.node.id.clone()).collect();

    // Emit search event
    state.emit(VestigeEvent::SearchPerformed {
        query: params.q.clone(),
        result_count: results.len(),
        result_ids: result_ids.clone(),
        duration_ms,
        timestamp: Utc::now(),
    });

    let formatted: Vec<Value> = results
        .into_iter()
        .filter(|r| {
            params
                .min_retention
                .is_none_or(|min| r.node.retention_strength >= min)
        })
        .map(|r| {
            serde_json::json!({
                "id": r.node.id,
                "content": r.node.content,
                "nodeType": r.node.node_type,
                "tags": r.node.tags,
                "retentionStrength": r.node.retention_strength,
                "combinedScore": r.combined_score,
                "createdAt": r.node.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "query": params.q,
        "total": formatted.len(),
        "durationMs": duration_ms,
        "results": formatted,
    })))
}

// ============================================================================
// COGNITIVE OPERATIONS (v2.0)
// ============================================================================

/// Trigger a dream cycle via CognitiveEngine
pub async fn trigger_dream(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let cognitive = state
        .cognitive
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let start = std::time::Instant::now();
    let memory_count: usize = 50;

    // Load memories for dreaming
    let all_nodes = state
        .storage
        .get_all_nodes(memory_count as i32, 0)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if all_nodes.len() < 5 {
        return Ok(Json(serde_json::json!({
            "status": "insufficient_memories",
            "message": format!("Need at least 5 memories. Current: {}", all_nodes.len()),
        })));
    }

    // Emit start event
    state.emit(VestigeEvent::DreamStarted {
        memory_count: all_nodes.len(),
        timestamp: Utc::now(),
    });

    // Build dream memories
    let dream_memories: Vec<vestige_core::DreamMemory> = all_nodes
        .iter()
        .map(|n| vestige_core::DreamMemory {
            id: n.id.clone(),
            content: n.content.clone(),
            embedding: state.storage.get_node_embedding(&n.id).ok().flatten(),
            tags: n.tags.clone(),
            created_at: n.created_at,
            access_count: n.reps as u32,
        })
        .collect();

    // Run dream through CognitiveEngine
    let cog = cognitive.lock().await;
    // Capture start time before the dream — composite-score eviction in store_connections
    // reorders the buffer, making positional slicing (pre_dream_count..) unreliable.
    let dream_start = Utc::now();
    let dream_result = cog.dreamer.dream(&dream_memories).await;
    let insights = cog.dreamer.synthesize_insights(&dream_memories);
    let all_connections = cog.dreamer.get_connections();
    drop(cog);

    // Persist new connections
    // Filter by timestamp — same approach as dream.rs to avoid positional index issues.
    let new_connections: Vec<&vestige_core::DiscoveredConnection> = all_connections
        .iter()
        .filter(|c| c.discovered_at >= dream_start)
        .collect();
    let mut connections_persisted = 0u64;
    let now = Utc::now();
    for conn in new_connections.iter() {
        // Skip noisy edge types
        let link_type = match conn.connection_type {
            vestige_core::DiscoveredConnectionType::Semantic => "semantic",
            vestige_core::DiscoveredConnectionType::SharedConcept => "shared_concepts",
            vestige_core::DiscoveredConnectionType::CausalChain => "caused_by",
            vestige_core::DiscoveredConnectionType::Temporal
            | vestige_core::DiscoveredConnectionType::Complementary => continue,
        };
        let record = vestige_core::ConnectionRecord {
            source_id: conn.from_id.clone(),
            target_id: conn.to_id.clone(),
            strength: conn.similarity,
            link_type: link_type.to_string(),
            created_at: now,
            last_activated: now,
            activation_count: 1,
        };
        if state.storage.save_connection(&record).is_ok() {
            connections_persisted += 1;
        }

        // Emit connection events
        state.emit(VestigeEvent::ConnectionDiscovered {
            source_id: conn.from_id.clone(),
            target_id: conn.to_id.clone(),
            connection_type: link_type.to_string(),
            weight: conn.similarity,
            timestamp: now,
        });
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    // Emit completion event
    state.emit(VestigeEvent::DreamCompleted {
        memories_replayed: dream_memories.len(),
        connections_found: connections_persisted as usize,
        insights_generated: insights.len(),
        duration_ms,
        timestamp: Utc::now(),
    });

    Ok(Json(serde_json::json!({
        "status": "dreamed",
        "memoriesReplayed": dream_memories.len(),
        "connectionsPersisted": connections_persisted,
        "insights": insights.iter().map(|i| serde_json::json!({
            "type": format!("{:?}", i.insight_type),
            "insight": i.insight,
            "sourceMemories": i.source_memories,
            "confidence": i.confidence,
            "noveltyScore": i.novelty_score,
        })).collect::<Vec<Value>>(),
        "stats": {
            "newConnectionsFound": dream_result.new_connections_found,
            "connectionsPersisted": connections_persisted,
            "memoriesStrengthened": dream_result.memories_strengthened,
            "memoriesCompressed": dream_result.memories_compressed,
            "insightsGenerated": dream_result.insights_generated.len(),
            "durationMs": duration_ms,
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct ExploreRequest {
    pub from_id: String,
    pub to_id: Option<String>,
    pub action: Option<String>, // "associations", "chains", "bridges", "deep"
    pub limit: Option<usize>,
    pub max_hops: Option<usize>,
}

/// Explore connections between memories
pub async fn explore_connections(
    State(state): State<AppState>,
    Json(req): Json<ExploreRequest>,
) -> Result<Json<Value>, StatusCode> {
    let action = req.action.as_deref().unwrap_or("associations");
    let limit = req.limit.unwrap_or(10).clamp(1, 50);

    match action {
        "associations" => {
            // First: get actual graph connections (high quality, from dream)
            let connections = state.storage
                .get_connections_for_memory(&req.from_id)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let mut seen_ids = std::collections::HashSet::new();
            seen_ids.insert(req.from_id.clone());
            let mut formatted: Vec<Value> = Vec::new();

            // Add real graph connections first (skip temporal — they're just co-ingested, not meaningful)
            for conn in connections.iter().filter(|c| c.link_type != "temporal" && c.link_type != "complementary") {
                let neighbor_id = if conn.source_id == req.from_id {
                    &conn.target_id
                } else {
                    &conn.source_id
                };
                if !seen_ids.insert(neighbor_id.clone()) { continue; }
                if let Ok(Some(node)) = state.storage.get_node(neighbor_id) {
                    formatted.push(serde_json::json!({
                        "id": node.id,
                        "content": node.content,
                        "nodeType": node.node_type,
                        "score": conn.strength,
                        "retention": node.retention_strength,
                        "connectionType": conn.link_type,
                        "source": "graph",
                    }));
                }
            }

            // Then: fill remaining slots with semantic search (only high similarity)
            if formatted.len() < limit {
                let source_node = state.storage
                    .get_node(&req.from_id)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .ok_or(StatusCode::NOT_FOUND)?;

                let results = state.storage
                    .hybrid_search(&source_node.content, (limit * 2) as i32, 0.3, 0.7)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                for r in &results {
                    if formatted.len() >= limit { break; }
                    if !seen_ids.insert(r.node.id.clone()) { continue; }
                    if r.combined_score < 0.65 { continue; }
                    formatted.push(serde_json::json!({
                        "id": r.node.id,
                        "content": r.node.content,
                        "nodeType": r.node.node_type,
                        "score": r.combined_score,
                        "retention": r.node.retention_strength,
                        "source": "semantic",
                    }));
                }
            }

            // Sort by score descending
            formatted.sort_by(|a, b| {
                let sa = a["score"].as_f64().unwrap_or(0.0);
                let sb = b["score"].as_f64().unwrap_or(0.0);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });
            formatted.truncate(limit);

            Ok(Json(serde_json::json!({
                "action": "associations",
                "fromId": req.from_id,
                "results": formatted,
            })))
        }
        "chains" | "bridges" => {
            let to_id = req.to_id.as_deref().ok_or(StatusCode::BAD_REQUEST)?;

            let (nodes, edges) = state
                .storage
                .get_memory_subgraph(&req.from_id, 2, limit)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let nodes_json: Vec<Value> = nodes
                .iter()
                .map(|n| {
                    serde_json::json!({
                        "id": n.id,
                        "content": n.content.chars().take(100).collect::<String>(),
                        "nodeType": n.node_type,
                        "retention": n.retention_strength,
                    })
                })
                .collect();

            let edges_json: Vec<Value> = edges
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "source": e.source_id,
                        "target": e.target_id,
                        "weight": e.strength,
                        "type": e.link_type,
                    })
                })
                .collect();

            Ok(Json(serde_json::json!({
                "action": action,
                "fromId": req.from_id,
                "toId": to_id,
                "nodes": nodes_json,
                "edges": edges_json,
            })))
        }
        "deep" => {
            let max_hops = req.max_hops.unwrap_or(3).clamp(1, 5);
            let mut visited = std::collections::HashMap::<String, usize>::new(); // id -> hop
            let mut queue = std::collections::VecDeque::new();
            queue.push_back((req.from_id.clone(), 0usize));
            visited.insert(req.from_id.clone(), 0);

            while let Some((node_id, hop)) = queue.pop_front() {
                if hop >= max_hops { continue; }
                let conns = state.storage
                    .get_connections_for_memory(&node_id)
                    .unwrap_or_default();
                for conn in &conns {
                    if conn.link_type == "temporal" || conn.link_type == "complementary" { continue; }
                    let neighbor = if conn.source_id == node_id { &conn.target_id } else { &conn.source_id };
                    if !visited.contains_key(neighbor) {
                        visited.insert(neighbor.clone(), hop + 1);
                        queue.push_back((neighbor.clone(), hop + 1));
                    }
                }
            }

            // Build results grouped by hop
            let mut results: Vec<Value> = Vec::new();
            let mut by_hop: std::collections::BTreeMap<usize, Vec<Value>> = std::collections::BTreeMap::new();

            for (id, hop) in &visited {
                if *hop == 0 { continue; } // skip origin
                if let Ok(Some(node)) = state.storage.get_node(id) {
                    let entry = serde_json::json!({
                        "id": node.id,
                        "content": node.content,
                        "nodeType": node.node_type,
                        "retention": node.retention_strength,
                        "hop": hop,
                        "tags": node.tags,
                    });
                    results.push(entry.clone());
                    by_hop.entry(*hop).or_default().push(entry);
                }
            }

            results.sort_by(|a, b| {
                let ha = a["hop"].as_u64().unwrap_or(99);
                let hb = b["hop"].as_u64().unwrap_or(99);
                ha.cmp(&hb)
            });

            Ok(Json(serde_json::json!({
                "action": "deep",
                "fromId": req.from_id,
                "maxHops": max_hops,
                "totalFound": results.len(),
                "byHop": by_hop,
                "results": results,
            })))
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Predict which memories will be needed
pub async fn predict_memories(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    // Get recent memories as predictions based on activity
    let recent = state
        .storage
        .get_all_nodes(10, 0)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let predictions: Vec<Value> = recent
        .iter()
        .map(|n| {
            serde_json::json!({
                "id": n.id,
                "content": n.content.chars().take(100).collect::<String>(),
                "nodeType": n.node_type,
                "retention": n.retention_strength,
                "predictedNeed": "high",
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "predictions": predictions,
        "basedOn": "recent_activity",
    })))
}

#[derive(Debug, Deserialize)]
pub struct ImportanceRequest {
    pub content: String,
}

/// Score content importance using 4-channel model
pub async fn score_importance(
    State(state): State<AppState>,
    Json(req): Json<ImportanceRequest>,
) -> Result<Json<Value>, StatusCode> {
    if let Some(ref cognitive) = state.cognitive {
        let context = vestige_core::ImportanceContext::current();
        let cog = cognitive.lock().await;
        let score = cog
            .importance_signals
            .compute_importance(&req.content, &context);
        drop(cog);

        let composite = score.composite;
        let novelty = score.novelty;
        let arousal = score.arousal;
        let reward = score.reward;
        let attention = score.attention;

        state.emit(VestigeEvent::ImportanceScored {
            memory_id: None,
            content_preview: req.content.chars().take(80).collect(),
            composite_score: composite,
            novelty,
            arousal,
            reward,
            attention,
            timestamp: Utc::now(),
        });

        Ok(Json(serde_json::json!({
            "composite": composite,
            "channels": {
                "novelty": novelty,
                "arousal": arousal,
                "reward": reward,
                "attention": attention,
            },
            "recommendation": if composite > 0.6 { "save" } else { "skip" },
        })))
    } else {
        // Fallback: basic heuristic scoring
        let word_count = req.content.split_whitespace().count();
        let has_code = req.content.contains("```") || req.content.contains("fn ");
        let composite = if has_code {
            0.7
        } else {
            (word_count as f64 / 100.0).min(0.8)
        };

        Ok(Json(serde_json::json!({
            "composite": composite,
            "channels": {
                "novelty": composite,
                "arousal": 0.5,
                "reward": 0.5,
                "attention": composite,
            },
            "recommendation": if composite > 0.6 { "save" } else { "skip" },
        })))
    }
}

/// Trigger consolidation
pub async fn trigger_consolidation(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    state.emit(VestigeEvent::ConsolidationStarted {
        timestamp: Utc::now(),
    });

    let start = std::time::Instant::now();

    let result = state
        .storage
        .run_consolidation()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let duration_ms = start.elapsed().as_millis() as u64;

    state.emit(VestigeEvent::ConsolidationCompleted {
        nodes_processed: result.nodes_processed as usize,
        decay_applied: result.decay_applied as usize,
        embeddings_generated: result.embeddings_generated as usize,
        duration_ms,
        timestamp: Utc::now(),
    });

    Ok(Json(serde_json::json!({
        "nodesProcessed": result.nodes_processed,
        "decayApplied": result.decay_applied,
        "embeddingsGenerated": result.embeddings_generated,
        "duplicatesMerged": result.duplicates_merged,
        "activationsComputed": result.activations_computed,
        "durationMs": duration_ms,
    })))
}

/// Get retention distribution (for histogram visualization)
pub async fn retention_distribution(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    // Cap at 1000 to prevent excessive memory usage on large databases
    let nodes = state
        .storage
        .get_all_nodes(1000, 0)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build distribution buckets
    let mut buckets = [0u32; 10]; // 0-10%, 10-20%, ..., 90-100%
    let mut by_type: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut endangered = Vec::new();

    for node in &nodes {
        let bucket = ((node.retention_strength * 10.0).floor() as usize).min(9);
        buckets[bucket] += 1;
        *by_type.entry(node.node_type.clone()).or_default() += 1;

        // Endangered: retention below 30%
        if node.retention_strength < 0.3 {
            endangered.push(serde_json::json!({
                "id": node.id,
                "content": node.content.chars().take(60).collect::<String>(),
                "retention": node.retention_strength,
                "nodeType": node.node_type,
            }));
        }
    }

    let distribution: Vec<Value> = buckets
        .iter()
        .enumerate()
        .map(|(i, &count)| {
            serde_json::json!({
                "range": format!("{}-{}%", i * 10, (i + 1) * 10),
                "count": count,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "distribution": distribution,
        "byType": by_type,
        "endangered": endangered,
        "total": nodes.len(),
    })))
}

// ============================================================================
// INTENTIONS (v2.0)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct IntentionListParams {
    pub status: Option<String>,
}

/// List intentions
pub async fn list_intentions(
    State(state): State<AppState>,
    Query(params): Query<IntentionListParams>,
) -> Result<Json<Value>, StatusCode> {
    let status_filter = params.status.unwrap_or_else(|| "active".to_string());

    let intentions = if status_filter == "all" {
        // Get all statuses
        let mut all = state.storage.get_active_intentions().unwrap_or_default();
        all.extend(
            state
                .storage
                .get_intentions_by_status("fulfilled")
                .unwrap_or_default(),
        );
        all.extend(
            state
                .storage
                .get_intentions_by_status("cancelled")
                .unwrap_or_default(),
        );
        all.extend(
            state
                .storage
                .get_intentions_by_status("snoozed")
                .unwrap_or_default(),
        );
        all
    } else if status_filter == "active" {
        state
            .storage
            .get_active_intentions()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        state
            .storage
            .get_intentions_by_status(&status_filter)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let count = intentions.len();
    Ok(Json(serde_json::json!({
        "intentions": intentions,
        "total": count,
        "filter": status_filter,
    })))
}

// ============================================================================
// TAGS / WORKSPACES
// ============================================================================

pub async fn list_tags(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let all_tags = state
        .storage
        .list_tags()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut workspaces: Vec<String> = Vec::new();
    let mut tags_json: Vec<Value> = Vec::new();

    for (tag, count) in &all_tags {
        tags_json.push(serde_json::json!({ "tag": tag, "count": count }));
        if tag.starts_with("project:") || tag.starts_with("codebase:") {
            let name = tag.split_once(':').map(|(_, v)| v).unwrap_or(tag);
            if !workspaces.contains(&name.to_string()) {
                workspaces.push(name.to_string());
            }
        }
    }

    Ok(Json(serde_json::json!({
        "tags": tags_json,
        "workspaces": workspaces,
        "total": all_tags.len(),
    })))
}

// ============================================================================
// INTENTION ACTIONS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateIntentionBody {
    pub content: String,
    pub trigger_type: Option<String>,
    pub trigger_data: Option<String>,
    pub priority: Option<i32>,
    pub deadline: Option<String>,
}

pub async fn create_intention(
    State(state): State<AppState>,
    Json(body): Json<CreateIntentionBody>,
) -> Result<Json<Value>, StatusCode> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let trigger_type = body.trigger_type.unwrap_or_else(|| "context".to_string());
    let trigger_data = body.trigger_data.unwrap_or_else(|| "{}".to_string());
    let priority = body.priority.unwrap_or(2);
    let deadline = body.deadline.and_then(|d| chrono::DateTime::parse_from_rfc3339(&d).ok().map(|dt| dt.with_timezone(&Utc)));

    let record = vestige_core::IntentionRecord {
        id: id.clone(),
        content: body.content.clone(),
        trigger_type,
        trigger_data,
        priority,
        status: "active".to_string(),
        created_at: now,
        deadline,
        fulfilled_at: None,
        reminder_count: 0,
        last_reminded_at: None,
        notes: None,
        tags: vec![],
        related_memories: vec![],
        snoozed_until: None,
        source_type: "dashboard".to_string(),
        source_data: None,
    };

    state.storage.save_intention(&record).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "id": id,
        "content": body.content,
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateIntentionBody {
    pub status: String,
}

pub async fn update_intention(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateIntentionBody>,
) -> Result<Json<Value>, StatusCode> {
    let valid = ["fulfilled", "cancelled", "snoozed", "active"];
    if !valid.contains(&body.status.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let updated = state.storage
        .update_intention_status(&id, &body.status)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "success": updated,
        "id": id,
        "status": body.status,
    })))
}

// ============================================================================
// ANALYTICS — Command Center
// ============================================================================

pub async fn get_analytics(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let storage = &state.storage;

    // 1. Memory growth: count per day for last 30 days
    let all_nodes = storage.get_all_nodes(5000, 0).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let now = Utc::now();
    let mut daily_created: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    let mut type_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut workspace_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut tier_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut decay_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut retention_buckets = [0i64; 5]; // 0-20, 20-40, 40-60, 60-80, 80-100

    for node in &all_nodes {
        // Daily created
        let day = node.created_at.format("%Y-%m-%d").to_string();
        *daily_created.entry(day).or_insert(0) += 1;

        // Type distribution
        *type_counts.entry(node.node_type.clone()).or_insert(0) += 1;

        // Workspace distribution
        for tag in &node.tags {
            if tag.starts_with("project:") || tag.starts_with("codebase:") {
                let ws = tag.split_once(':').map(|(_, v)| v).unwrap_or(tag);
                *workspace_counts.entry(ws.to_string()).or_insert(0) += 1;
            }
        }

        // Tier distribution
        *tier_counts.entry(node.tier.clone()).or_insert(0) += 1;

        // Decay class distribution
        *decay_counts.entry(node.decay_class.clone()).or_insert(0) += 1;

        // Retention distribution
        let bucket = ((node.retention_strength * 5.0).floor() as usize).min(4);
        retention_buckets[bucket] += 1;
    }

    // 2. Access log activity (last 30 days)
    let access_log = {
        let reader = storage.reader_lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let cutoff = (now - Duration::days(30)).to_rfc3339();
        let mut stmt = reader.prepare(
            "SELECT DATE(accessed_at) as day, access_type, COUNT(*) as cnt
             FROM memory_access_log
             WHERE accessed_at >= ?1
             GROUP BY day, access_type
             ORDER BY day"
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let rows = stmt.query_map(rusqlite::params![cutoff], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut result: Vec<Value> = Vec::new();
        for row in rows {
            if let Ok((day, access_type, count)) = row {
                result.push(serde_json::json!({ "date": day, "type": access_type, "count": count }));
            }
        }
        result
    };

    // 3. Dream history
    let dream_history = {
        let reader = storage.reader_lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut stmt = reader.prepare(
            "SELECT dreamed_at, memories_replayed, connections_found, insights_generated
             FROM dream_history ORDER BY dreamed_at DESC LIMIT 20"
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "date": row.get::<_, String>(0)?,
                "memoriesReplayed": row.get::<_, i64>(1)?,
                "connectionsFound": row.get::<_, i64>(2)?,
                "insightsGenerated": row.get::<_, i64>(3)?,
            }))
        }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    // 4. Retention trend from snapshots
    let retention_trend = {
        let reader = storage.reader_lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut stmt = reader.prepare(
            "SELECT snapshot_at, avg_retention, total_memories
             FROM retention_snapshots ORDER BY snapshot_at DESC LIMIT 30"
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "date": row.get::<_, String>(0)?,
                "avgRetention": row.get::<_, f64>(1)?,
                "totalMemories": row.get::<_, i64>(2)?,
            }))
        }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    // 5. Connection stats
    let connection_stats = {
        let reader = storage.reader_lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let total: i64 = reader.query_row("SELECT COUNT(*) FROM memory_connections", [], |r| r.get(0))
            .unwrap_or(0);
        let mut type_stmt = reader.prepare(
            "SELECT link_type, COUNT(*) FROM memory_connections GROUP BY link_type ORDER BY COUNT(*) DESC"
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let types = type_stmt.query_map([], |row| {
            Ok(serde_json::json!({ "type": row.get::<_, String>(0)?, "count": row.get::<_, i64>(1)? }))
        }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let type_list: Vec<Value> = types.filter_map(|r| r.ok()).collect();
        serde_json::json!({ "total": total, "byType": type_list })
    };

    // 6. Summary scores
    let total_memories = all_nodes.len() as i64;
    let avg_retention = if total_memories > 0 {
        all_nodes.iter().map(|n| n.retention_strength).sum::<f64>() / total_memories as f64
    } else { 0.0 };
    let core_count = *tier_counts.get("core").unwrap_or(&0);
    let permanent_count = *decay_counts.get("permanent").unwrap_or(&0);

    Ok(Json(serde_json::json!({
        "summary": {
            "totalMemories": total_memories,
            "avgRetention": (avg_retention * 100.0).round() / 100.0,
            "coreMemories": core_count,
            "permanentMemories": permanent_count,
            "totalConnections": connection_stats["total"],
            "totalDreams": dream_history.len(),
        },
        "dailyCreated": daily_created,
        "typeDistribution": type_counts,
        "workspaceDistribution": workspace_counts,
        "tierDistribution": tier_counts,
        "decayDistribution": decay_counts,
        "retentionBuckets": {
            "0-20%": retention_buckets[0],
            "20-40%": retention_buckets[1],
            "40-60%": retention_buckets[2],
            "60-80%": retention_buckets[3],
            "80-100%": retention_buckets[4],
        },
        "accessLog": access_log,
        "dreamHistory": dream_history,
        "retentionTrend": retention_trend,
        "connectionStats": connection_stats,
    })))
}
