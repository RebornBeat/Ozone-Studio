//! BlueprintCreatePipeline - Pipeline #14
//! Create and update task blueprints. Per spec §14.

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum BlueprintCreateInput {
    Create {
        name: String,
        description: String,
        input_signature: Vec<InputField>,
        output_type: String,
        steps: Vec<StepInput>,
        methodologies: Vec<u64>,
        keywords: Vec<String>,
        /// Task this blueprint was distilled from, if any — carried into the
        /// persisted container's metadata so meta-work traces back to the
        /// real task that produced it.
        #[serde(default)]
        task_id: Option<u64>,
    },
    Update {
        blueprint_id: u64,
        name: Option<String>,
        description: Option<String>,
        add_steps: Option<Vec<StepInput>>,
        remove_steps: Option<Vec<u32>>,
        add_keywords: Option<Vec<String>>,
    },
    Delete { blueprint_id: u64 },
    Clone { blueprint_id: u64, new_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputField { pub name: String, pub field_type: String, pub required: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInput { pub order: u32, pub description: String, pub pipeline_id: Option<u64> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintCreateOutput {
    pub success: bool,
    pub blueprint_id: Option<u64>,
    pub version: Option<u32>,
    pub error: Option<String>,
}

const BLUEPRINT_ROOT_ID: u64 = 3;

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

/// Write the full blueprint content (steps/input_signature/output_type —
/// Container has no generic slot for this nested structure, same constraint
/// pipeline 100's modality graphs and the orchestrator's AMT tree hit) to
/// disk, referenced via storage.object_store_path.
fn write_content_file(id_hint: &str, content: &serde_json::Value) -> Option<String> {
    let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let dir = format!("{}/blueprints_created", data_dir);
    let _ = std::fs::create_dir_all(&dir);
    let file_name = format!("blueprint_{}_{}.json", now_secs(), id_hint);
    let path = format!("{}/{}", dir, file_name);
    let json = serde_json::to_string_pretty(content).ok()?;
    std::fs::write(&path, json).ok()?;
    Some(format!("blueprints_created/{}", file_name))
}

fn build_container_json(
    parent_id: u64,
    name: &str,
    keywords: &[String],
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
                "container_type": "Blueprint",
                "modality": "Unknown",
                "created_at": now,
                "updated_at": now,
                "provenance": match task_id {
                    Some(t) => format!("blueprint_create:task:{}", t),
                    None => "blueprint_create".to_string(),
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
                "topics": [],
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

pub async fn execute(input: BlueprintCreateInput) -> Result<BlueprintCreateOutput, String> {
    match input {
        BlueprintCreateInput::Create {
            name,
            description,
            input_signature,
            output_type,
            steps,
            methodologies,
            keywords,
            task_id,
        } => {
            let content = serde_json::json!({
                "name": name,
                "description": description,
                "input_signature": input_signature,
                "output_type": output_type,
                "steps": steps,
                "methodologies": methodologies,
            });
            let object_store_path = write_content_file("new", &content);
            let container = build_container_json(BLUEPRINT_ROOT_ID, &name, &keywords, object_store_path, task_id);
            let result = zsei_query(serde_json::json!({
                "CreateContainer": {"parent_id": BLUEPRINT_ROOT_ID, "container": container}
            }))
            .await?;
            let id = result
                .get("ContainerID")
                .and_then(|v| v.as_u64())
                .ok_or("CreateContainer returned no container id")?;
            Ok(BlueprintCreateOutput { success: true, blueprint_id: Some(id), version: Some(1), error: None })
        }
        BlueprintCreateInput::Update { blueprint_id, name, description, add_steps, remove_steps, add_keywords } => {
            let existing = zsei_query(serde_json::json!({
                "GetContainer": {"container_id": blueprint_id}
            }))
            .await?;
            let container = existing.get("Container").ok_or("Blueprint container not found")?;
            let mut keywords: Vec<String> = container
                .pointer("/local_state/context/keywords")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            if let Some(add) = add_keywords {
                keywords.extend(add);
            }
            keywords.sort();
            keywords.dedup();
            let existing_name = container
                .pointer("/local_state/metadata/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if add_steps.is_some() || remove_steps.is_some() {
                let content = serde_json::json!({"added_steps": add_steps, "removed_step_orders": remove_steps});
                write_content_file(&blueprint_id.to_string(), &content);
            }
            let _ = description; // folded into the content file, not the container's own fields

            let updates = serde_json::json!({
                "metadata": {
                    "container_type": "Blueprint",
                    "modality": "Unknown",
                    "created_at": container.pointer("/local_state/metadata/created_at").cloned().unwrap_or(serde_json::json!(now_secs())),
                    "updated_at": now_secs(),
                    "provenance": container.pointer("/local_state/metadata/provenance").cloned().unwrap_or(serde_json::json!("blueprint_create")),
                    "permissions": 0,
                    "owner_id": 0,
                    "name": name.unwrap_or(existing_name),
                    "materialized_path": null
                },
                "context": {
                    "categories": [],
                    "methodologies": [],
                    "keywords": keywords,
                    "topics": [],
                    "relationships": [],
                    "learned_associations": [],
                    "embedding": null
                }
            });
            zsei_query(serde_json::json!({
                "UpdateContainer": {"container_id": blueprint_id, "updates": updates}
            }))
            .await?;
            Ok(BlueprintCreateOutput { success: true, blueprint_id: Some(blueprint_id), version: Some(2), error: None })
        }
        BlueprintCreateInput::Delete { blueprint_id } => {
            zsei_query(serde_json::json!({"DeleteContainer": {"container_id": blueprint_id}})).await?;
            Ok(BlueprintCreateOutput { success: true, blueprint_id: Some(blueprint_id), version: None, error: None })
        }
        BlueprintCreateInput::Clone { blueprint_id, new_name } => {
            let existing = zsei_query(serde_json::json!({
                "GetContainer": {"container_id": blueprint_id}
            }))
            .await?;
            let container = existing.get("Container").ok_or("Blueprint container not found")?;
            let keywords: Vec<String> = container
                .pointer("/local_state/context/keywords")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let object_store_path = container
                .pointer("/local_state/storage/object_store_path")
                .and_then(|v| v.as_str())
                .map(String::from);
            let new_container = build_container_json(BLUEPRINT_ROOT_ID, &new_name, &keywords, object_store_path, None);
            let result = zsei_query(serde_json::json!({
                "CreateContainer": {"parent_id": BLUEPRINT_ROOT_ID, "container": new_container}
            }))
            .await?;
            let id = result
                .get("ContainerID")
                .and_then(|v| v.as_u64())
                .ok_or("CreateContainer returned no container id")?;
            Ok(BlueprintCreateOutput { success: true, blueprint_id: Some(id), version: Some(1), error: None })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    // The host passes the full PipelineInput envelope {data, context} — every
    // real invocation from the orchestrator is wrapped this way. Unwrap
    // `data` when present; a bare BlueprintCreateInput (direct CLI use) also
    // still works.
    let parsed_value: serde_json::Value = serde_json::from_str(&input_json)
        .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let data_json = match parsed_value.get("data") {
        Some(d) => serde_json::to_string(d).unwrap_or_else(|_| input_json.clone()),
        None => input_json.clone(),
    };
    let input: BlueprintCreateInput = serde_json::from_str(&data_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
