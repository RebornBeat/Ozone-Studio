//! MCP graph persistence: a tool output that carries a `graph` block becomes
//! real ZSEI containers. Containment is the native parent/child link
//! (`parent_id` / `child_ids`); typed relations (Above, NearTo, InFrontOf,
//! Overlaps, ...) are `Relation`s in `context.relationships`, which Structural
//! traversal follows. Entity attributes sit behind `object_store_path`, the
//! same content-pointer pattern context_mirror uses.
//!
//! Block shape (all keys validated here, never trusted):
//!   { "nodes": [{"key", "kind", "label", "parent"?, "attributes"?}],
//!     "edges": [{"from", "to", "relation", "confidence"?}] }

use std::collections::HashMap;

use serde_json::Value;

use crate::types::container::{
    CompressionType, Container, ContainerType, Context, DiscoveryMethod, GlobalState,
    IntegrityData, LocalState, Metadata, Modality, Relation, RelationType, StoragePointers,
    TraversalHints,
};
use crate::types::zsei::{ContainerUpdate, ZSEIQuery, ZSEIQueryResult};
use crate::zsei::ZSEI;

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphSummary {
    pub root_id: u64,
    pub entities: usize,
    pub relations: usize,
}

pub async fn persist_from_output(
    zsei: &ZSEI,
    data_dir: &str,
    tool: &str,
    agent: &str,
    output: &Value,
) -> Option<Result<GraphSummary, String>> {
    let graph = output
        .get("graph")
        .or_else(|| output.get("output").and_then(|o| o.get("graph")))?;
    Some(persist(zsei, data_dir, tool, agent, graph).await)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 80
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn relation_from_name(name: &str) -> Option<RelationType> {
    Some(match name {
        "Contains" => RelationType::Contains,
        "PartOf" => RelationType::PartOf,
        "RelatedTo" => RelationType::RelatedTo,
        "References" => RelationType::References,
        "ImportsFrom" => RelationType::ImportsFrom,
        "CallsTo" => RelationType::CallsTo,
        "Above" => RelationType::Above,
        "Below" => RelationType::Below,
        "NearTo" => RelationType::NearTo,
        "InFrontOf" => RelationType::InFrontOf,
        "Overlaps" => RelationType::Overlaps,
        _ => return None,
    })
}

fn modality_for(tool: &str) -> Modality {
    if tool.starts_with("visual_")
        || tool.starts_with("yolo_")
        || tool.starts_with("depth_")
        || tool.starts_with("shape_")
        || tool.starts_with("pose_")
    {
        Modality::Image
    } else {
        Modality::Unknown
    }
}

fn build_container(
    container_type: ContainerType,
    modality: Modality,
    name: &str,
    path: String,
    keywords: Vec<String>,
    agent: &str,
    object_store_path: Option<String>,
    parent_id: u64,
) -> Container {
    let now = now_secs();
    Container {
        global_state: GlobalState {
            container_id: 0,
            parent_id,
            child_ids: vec![],
            child_count: 0,
            version: 1,
        },
        local_state: LocalState {
            metadata: Metadata {
                container_type,
                modality,
                created_at: now,
                updated_at: now,
                provenance: agent.to_string(),
                permissions: 0,
                owner_id: 0,
                name: Some(name.to_string()),
                materialized_path: Some(path),
            },
            context: Context {
                categories: vec![],
                methodologies: vec![],
                keywords,
                topics: vec!["mcp".to_string()],
                relationships: vec![],
                learned_associations: vec![],
                embedding: None,
            },
            storage: StoragePointers {
                db_shard_id: None,
                vector_index_ref: None,
                object_store_path,
                compression_type: CompressionType::None,
            },
            hints: TraversalHints::default(),
            integrity: IntegrityData::default(),
            file_context: None,
            code_context: None,
            text_context: None,
            external_ref: None,
        },
    }
}

async fn create(zsei: &ZSEI, parent_id: u64, container: Container) -> Result<u64, String> {
    match zsei
        .query(ZSEIQuery::CreateContainer { parent_id, container })
        .await
    {
        Ok(ZSEIQueryResult::ContainerID(id)) => Ok(id),
        Ok(_) => Err("unexpected CreateContainer result".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

async fn persist(
    zsei: &ZSEI,
    data_dir: &str,
    tool: &str,
    agent: &str,
    graph: &Value,
) -> Result<GraphSummary, String> {
    let nodes = graph
        .get("nodes")
        .and_then(|n| n.as_array())
        .ok_or("graph.nodes must be an array")?;
    let edges = graph
        .get("edges")
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default();
    if nodes.is_empty() {
        return Err("graph has no nodes".to_string());
    }

    let mut keys: HashMap<&str, Option<&str>> = HashMap::new();
    for node in nodes {
        let key = node
            .get("key")
            .and_then(|k| k.as_str())
            .ok_or("every node needs a string key")?;
        if !valid_key(key) {
            return Err(format!("node key '{key}' must be [A-Za-z0-9_-], max 80 chars"));
        }
        let parent = node.get("parent").and_then(|p| p.as_str());
        if keys.insert(key, parent).is_some() {
            return Err(format!("duplicate node key '{key}'"));
        }
    }
    for (key, parent) in &keys {
        if let Some(pk) = *parent {
            if !keys.contains_key(pk) {
                return Err(format!("node '{key}' has unknown parent '{pk}'"));
            }
        }
        let mut cur: Option<&str> = *parent;
        let mut steps = 0usize;
        while let Some(p) = cur {
            steps += 1;
            if steps > keys.len() {
                return Err(format!("parent cycle through node '{key}'"));
            }
            cur = keys.get(p).copied().flatten();
        }
    }
    for edge in &edges {
        let from = edge.get("from").and_then(|v| v.as_str()).unwrap_or("");
        let to = edge.get("to").and_then(|v| v.as_str()).unwrap_or("");
        let name = edge.get("relation").and_then(|v| v.as_str()).unwrap_or("");
        if !keys.contains_key(from) {
            return Err(format!("edge source '{from}' is not a node"));
        }
        if !keys.contains_key(to) {
            return Err(format!("edge target '{to}' is not a node"));
        }
        if relation_from_name(name).is_none() {
            return Err(format!("unknown relation '{name}'"));
        }
    }

    let root_container = build_container(
        ContainerType::McpResult,
        modality_for(tool),
        &format!("{tool} result"),
        format!("/mcp/{tool}"),
        vec![tool.to_string(), "mcp_result".to_string()],
        agent,
        None,
        zsei.root_id(),
    );
    let root_id = create(zsei, zsei.root_id(), root_container).await?;
    let dir = format!("{}/mcp_graph/{}", data_dir, root_id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("create graph dir: {e}"))?;

    let mut key_to_id: HashMap<String, u64> = HashMap::new();
    let mut pending: Vec<&Value> = nodes.iter().collect();
    while !pending.is_empty() {
        let batch = std::mem::take(&mut pending);
        let before = batch.len();
        for node in batch {
            let key = node
                .get("key")
                .and_then(|k| k.as_str())
                .ok_or("every node needs a string key")?;
            if !valid_key(key) {
                return Err(format!("node key '{key}' must be [A-Za-z0-9_-], max 80 chars"));
            }
            let kind = node.get("kind").and_then(|k| k.as_str()).unwrap_or("entity");
            let label = node.get("label").and_then(|l| l.as_str()).unwrap_or(key);
            let parent_id = match node.get("parent").and_then(|p| p.as_str()) {
                None => root_id,
                Some(pk) => match key_to_id.get(pk) {
                    Some(id) => *id,
                    None => {
                        pending.push(node);
                        continue;
                    }
                },
            };

            let rel_path = format!("mcp_graph/{}/{}.json", root_id, key);
            let attrs = serde_json::json!({
                "key": key,
                "kind": kind,
                "label": label,
                "tool": tool,
                "attributes": node.get("attributes").cloned().unwrap_or(Value::Null),
            });
            tokio::fs::write(
                format!("{}/{}", data_dir, rel_path),
                serde_json::to_vec_pretty(&attrs).map_err(|e| e.to_string())?,
            )
            .await
            .map_err(|e| format!("write entity content {key}: {e}"))?;

            let container = build_container(
                ContainerType::McpEntity,
                modality_for(tool),
                label,
                format!("/mcp/{tool}/{root_id}/{key}"),
                vec![tool.to_string(), kind.to_string()],
                agent,
                Some(rel_path),
                parent_id,
            );
            let id = create(zsei, parent_id, container).await?;
            key_to_id.insert(key.to_string(), id);
        }
        if pending.len() == before {
            return Err("graph has a missing parent key or a parent cycle".to_string());
        }
    }

    let mut relations = 0usize;
    for edge in &edges {
        let from = edge.get("from").and_then(|v| v.as_str()).unwrap_or("");
        let to = edge.get("to").and_then(|v| v.as_str()).unwrap_or("");
        let name = edge.get("relation").and_then(|v| v.as_str()).unwrap_or("");
        let confidence = edge
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let from_id = *key_to_id
            .get(from)
            .ok_or_else(|| format!("edge source '{from}' is not a node"))?;
        let to_id = *key_to_id
            .get(to)
            .ok_or_else(|| format!("edge target '{to}' is not a node"))?;
        let relation_type = relation_from_name(name)
            .ok_or_else(|| format!("unknown relation '{name}'"))?;

        let source = zsei
            .get_container(from_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("edge source container vanished")?;
        let mut context = source.local_state.context.clone();
        let already = context
            .relationships
            .iter()
            .any(|r| r.target_id == to_id && r.relation_type == relation_type);
        if already {
            continue;
        }
        context.relationships.push(Relation {
            target_id: to_id,
            relation_type,
            confidence,
            discovered_via: DiscoveryMethod::ToolOutput,
            graph_hops: None,
        });
        zsei.query(ZSEIQuery::UpdateContainer {
            container_id: from_id,
            updates: ContainerUpdate {
                context: Some(context),
                ..Default::default()
            },
        })
        .await
        .map_err(|e| e.to_string())?;
        relations += 1;
    }

    Ok(GraphSummary {
        root_id,
        entities: key_to_id.len(),
        relations,
    })
}
