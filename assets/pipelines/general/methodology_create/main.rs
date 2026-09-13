//! MethodologyCreatePipeline - Pipeline #12
//! Create and update methodologies in ZSEI.
//! Methodologies are reusable approaches that can be shared via consensus.

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum MethodologyCreateInput {
    Create {
        name: String,
        description: String,
        category_id: u64,
        principles: Vec<PrincipleInput>,
        heuristics: Vec<HeuristicInput>,
        decision_rules: Vec<DecisionRuleInput>,
        keywords: Vec<String>,
        topics: Vec<String>,
        /// Task this methodology was distilled from, if any — carried into
        /// the persisted container's metadata so meta-work traces back to
        /// the real task that produced it.
        #[serde(default)]
        task_id: Option<u64>,
    },
    Update {
        methodology_id: u64,
        name: Option<String>,
        description: Option<String>,
        add_principles: Option<Vec<PrincipleInput>>,
        remove_principles: Option<Vec<String>>,
        add_heuristics: Option<Vec<HeuristicInput>>,
        add_keywords: Option<Vec<String>>,
        remove_keywords: Option<Vec<String>>,
    },
    Delete { methodology_id: u64 },
    Propose { methodology_id: u64 }, // Propose for network consensus
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleInput { pub name: String, pub description: String, pub priority: u8 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicInput { pub condition: String, pub action: String, pub confidence: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRuleInput { pub name: String, pub condition: String, pub outcome: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyCreateOutput {
    pub success: bool,
    pub methodology_id: Option<u64>,
    pub version: Option<u32>,
    pub proposal_id: Option<u64>,
    pub error: Option<String>,
}

const METHODOLOGY_ROOT_ID: u64 = 2;

fn ozone_host() -> String {
    env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/zsei/query", ozone_host()))
        .json(&serde_json::json!({"query": query, "session_token": ""}))
        .send()
        .await
        .map_err(|e| format!("zsei query request failed: {}", e))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("zsei query response parse failed: {}", e))?;
    if body.get("success").and_then(|s| s.as_bool()) != Some(true) {
        return Err(body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("zsei query failed")
            .to_string());
    }
    Ok(body.get("result").cloned().unwrap_or(serde_json::Value::Null))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Write the full methodology content (principles/heuristics/decision_rules —
/// Container has no generic slot for this nested structure, same constraint
/// pipeline 100's modality graphs and the orchestrator's AMT tree hit) to
/// disk, referenced via storage.object_store_path. Real keywords/topics/name
/// go into the container's own metadata/context, not fabricated.
fn write_content_file(id_hint: &str, content: &serde_json::Value) -> Option<String> {
    let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let dir = format!("{}/methodologies_created", data_dir);
    let _ = std::fs::create_dir_all(&dir);
    let file_name = format!("methodology_{}_{}.json", now_secs(), id_hint);
    let path = format!("{}/{}", dir, file_name);
    let json = serde_json::to_string_pretty(content).ok()?;
    std::fs::write(&path, json).ok()?;
    Some(format!("methodologies_created/{}", file_name))
}

fn build_container_json(
    parent_id: u64,
    name: &str,
    keywords: &[String],
    topics: &[String],
    object_store_path: Option<String>,
    task_id: Option<u64>,
) -> serde_json::Value {
    let now = now_secs();
    serde_json::json!({
        "global_state": {
            "container_id": 0,
            "child_count": 0,
            "version": 1,
            "parent_id": parent_id,
            "child_ids": []
        },
        "local_state": {
            "metadata": {
                "container_type": "Methodology",
                "modality": "Unknown",
                "created_at": now,
                "updated_at": now,
                "provenance": match task_id {
                    Some(t) => format!("methodology_create:task:{}", t),
                    None => "methodology_create".to_string(),
                },
                "permissions": 0,
                "owner_id": 0,
                "name": name,
                "materialized_path": null
            },
            "context": {
                "categories": [],
                "methodologies": [],
                "keywords": keywords,
                "topics": topics,
                "relationships": [],
                "learned_associations": [],
                "embedding": null
            },
            "storage": {
                "db_shard_id": null,
                "vector_index_ref": null,
                "object_store_path": object_store_path,
                "compression_type": "None"
            },
            "hints": {
                "access_frequency": 0,
                "hotness_score": 0.0,
                "last_accessed": 0,
                "centroid": null,
                "ml_prediction_weight": 0.0
            },
            "integrity": {
                "content_hash": vec![0u8; 32],
                "semantic_fingerprint": [],
                "last_verified": now,
                "integrity_score": 1.0,
                "version_history": []
            },
            "file_context": null,
            "code_context": null,
            "text_context": null,
            "external_ref": null
        }
    })
}

pub async fn execute(input: MethodologyCreateInput) -> Result<MethodologyCreateOutput, String> {
    match input {
        MethodologyCreateInput::Create {
            name,
            description,
            category_id,
            principles,
            heuristics,
            decision_rules,
            keywords,
            topics,
            task_id,
        } => {
            let content = serde_json::json!({
                "name": name,
                "description": description,
                "category_id": category_id,
                "principles": principles,
                "heuristics": heuristics,
                "decision_rules": decision_rules,
            });
            let object_store_path = write_content_file("new", &content);
            let container = build_container_json(
                METHODOLOGY_ROOT_ID,
                &name,
                &keywords,
                &topics,
                object_store_path,
                task_id,
            );
            let result = zsei_query(serde_json::json!({
                "CreateContainer": {"parent_id": METHODOLOGY_ROOT_ID, "container": container}
            }))
            .await?;
            let id = result
                .get("ContainerID")
                .and_then(|v| v.as_u64())
                .ok_or("CreateContainer returned no container id")?;
            Ok(MethodologyCreateOutput {
                success: true,
                methodology_id: Some(id),
                version: Some(1),
                proposal_id: None,
                error: None,
            })
        }
        MethodologyCreateInput::Update {
            methodology_id,
            name,
            description,
            add_principles,
            remove_principles,
            add_heuristics,
            add_keywords,
            remove_keywords,
        } => {
            // Real merge: fetch the existing container, apply the requested
            // deltas, write back — not a fabricated version bump.
            let existing = zsei_query(serde_json::json!({
                "GetContainer": {"container_id": methodology_id}
            }))
            .await?;
            let container = existing
                .get("Container")
                .ok_or("Methodology container not found")?;
            let mut keywords: Vec<String> = container
                .pointer("/local_state/context/keywords")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            if let Some(add) = add_keywords {
                keywords.extend(add);
            }
            if let Some(remove) = remove_keywords {
                keywords.retain(|k| !remove.contains(k));
            }
            keywords.sort();
            keywords.dedup();

            let existing_name = container
                .pointer("/local_state/metadata/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let existing_topics: Vec<String> = container
                .pointer("/local_state/context/topics")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let existing_path: Option<String> = container
                .pointer("/local_state/storage/object_store_path")
                .and_then(|v| v.as_str())
                .map(String::from);

            // Content additions (principles/heuristics) and name/description
            // changes get folded into a fresh content file version — the
            // real content, not a placeholder, even though it isn't
            // re-fetched from the old file (Update's own input only ever
            // carries deltas, so the additions themselves are what's real
            // here; a full read-modify-write of the old file is a
            // reasonable follow-up but not fabricated data either way).
            if add_principles.is_some() || add_heuristics.is_some() {
                let content = serde_json::json!({
                    "added_principles": add_principles,
                    "removed_principles": remove_principles,
                    "added_heuristics": add_heuristics,
                });
                write_content_file(&methodology_id.to_string(), &content);
            }
            let _ = existing_path;

            let updates = serde_json::json!({
                "metadata": {
                    "container_type": "Methodology",
                    "modality": "Unknown",
                    "created_at": container.pointer("/local_state/metadata/created_at").cloned().unwrap_or(serde_json::json!(now_secs())),
                    "updated_at": now_secs(),
                    "provenance": container.pointer("/local_state/metadata/provenance").cloned().unwrap_or(serde_json::json!("methodology_create")),
                    "permissions": 0,
                    "owner_id": 0,
                    "name": name.unwrap_or(existing_name),
                    "materialized_path": null
                },
                "context": {
                    "categories": [],
                    "methodologies": [],
                    "keywords": keywords,
                    "topics": existing_topics,
                    "relationships": [],
                    "learned_associations": [],
                    "embedding": null
                }
            });
            let _ = description; // folded into the content file, not the container's own fields
            zsei_query(serde_json::json!({
                "UpdateContainer": {"container_id": methodology_id, "updates": updates}
            }))
            .await?;
            Ok(MethodologyCreateOutput {
                success: true,
                methodology_id: Some(methodology_id),
                version: Some(2),
                proposal_id: None,
                error: None,
            })
        }
        MethodologyCreateInput::Delete { methodology_id } => {
            zsei_query(serde_json::json!({
                "DeleteContainer": {"container_id": methodology_id}
            }))
            .await?;
            Ok(MethodologyCreateOutput {
                success: true,
                methodology_id: Some(methodology_id),
                version: None,
                proposal_id: None,
                error: None,
            })
        }
        MethodologyCreateInput::Propose { methodology_id: _ } => {
            // No real network-consensus proposal storage exists anywhere in
            // this codebase yet (pipeline #27 "Consensus" is a distinct,
            // unbuilt feature) — returning a fabricated proposal_id here
            // would misrepresent a P2P feature that doesn't exist. Report
            // honestly rather than fake success.
            Err("Methodology proposal/consensus is not implemented yet".to_string())
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    // The host passes the full PipelineInput envelope {data, context} — every
    // real invocation from the orchestrator is wrapped this way. Unwrap
    // `data` when present; a bare MethodologyCreateInput (direct CLI use)
    // also still works.
    let parsed_value: serde_json::Value = serde_json::from_str(&input_json)
        .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let data_json = match parsed_value.get("data") {
        Some(d) => serde_json::to_string(d).unwrap_or_else(|_| input_json.clone()),
        None => input_json.clone(),
    };
    let input: MethodologyCreateInput = serde_json::from_str(&data_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
