//! Service layer for UI communication
//!
//! Provides HTTP/WebSocket endpoints for Electron UI.
//! Uses axum for HTTP and WebSocket support.

use crate::types::zsei::ZSEIQuery;
use crate::types::{OzoneError, OzoneResult};
use crate::OzoneRuntime;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

/// Shared application state
pub struct AppState {
    pub runtime: Arc<RwLock<OzoneRuntime>>,
    pub start_time: std::time::Instant,
    pub executor_progress: Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, crate::pipeline::PipelineProgress>>,
    >,
    /// QR device pairing — the phone as authenticator (see src/pairing.rs).
    pub pairing: Arc<crate::pairing::PairingHub>,
    /// Registered external tools — MCP connection surface (see src/mcp.rs).
    pub mcp: Arc<crate::mcp::McpRegistry>,
}

/// Recursively convert a `serde_json::Value` into `crate::types::Value`,
/// preserving nested objects/arrays as `Value::Map`/`Value::Array` instead of
/// flattening them into a stringified fallback (the previous inline match in
/// `orchestrate()` stringified any object/array, which silently broke
/// round-tripping structured fields like `model_config` through
/// `PipelineInput.data`).
fn json_to_typed_value(v: serde_json::Value) -> crate::types::Value {
    match v {
        serde_json::Value::Null => crate::types::Value::Null,
        serde_json::Value::Bool(b) => crate::types::Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                crate::types::Value::Int(i)
            } else {
                crate::types::Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => crate::types::Value::String(s),
        serde_json::Value::Array(arr) => {
            crate::types::Value::Array(arr.into_iter().map(json_to_typed_value).collect())
        }
        serde_json::Value::Object(map) => crate::types::Value::Map(
            map.into_iter()
                .map(|(k, v)| (k, json_to_typed_value(v)))
                .collect(),
        ),
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ChallengeRequest {
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRequest {
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub session_token: Option<String>,
    pub user_id: Option<u64>,
    pub device_id: Option<u64>,
    pub expires_at: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineRequest {
    pub pipeline_id: u64,
    pub input: serde_json::Value,
    pub session_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineResponse {
    pub success: bool,
    pub task_id: Option<u64>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_id: u64,
    pub session_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskListRequest {
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub session_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskInfo {
    pub task_id: u64,
    pub blueprint_id: Option<u64>,
    pub blueprint_name: String,
    pub status: String,
    pub progress: f32,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub error: Option<String>,
    /// Per-step detail (action, status, tokens, model_used) — populated on
    /// the single-task lookup (get_task) for the task-detail/rewind UI;
    /// left empty on list_tasks to keep the list view lightweight.
    #[serde(default)]
    pub steps: Vec<crate::task::TaskStepData>,
    /// Full "thinking cycle" for this task — see TaskData.thinking_log.
    /// Same lightweight-list-view convention as `steps`: populated on
    /// get_task, left empty on list_tasks.
    #[serde(default)]
    pub thinking_log: Vec<serde_json::Value>,
    /// Real AMT structure (branches/relationships/verification) for this
    /// task — see orchestrator::AMTSummary. Same lightweight-list-view
    /// convention as `steps`/`thinking_log`: populated on get_task, left
    /// empty on list_tasks.
    #[serde(default)]
    pub amt_summary: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskInfo>,
    pub total: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZseiQueryRequest {
    pub query: serde_json::Value,
    pub session_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZseiResponse {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub version: String,
    pub uptime_secs: u64,
    pub active_tasks: u32,
    /// Real libp2p connected-peer count (NetworkManager::get_status) — was
    /// previously only faked in the UI layer via a nonexistent config field.
    pub peer_count: usize,
    pub p2p_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigRequest {
    pub section: Option<String>,
    pub session_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub success: bool,
    pub config: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSetRequest {
    pub updates: serde_json::Value,
    pub session_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSetResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineProgressRequest {
    pub execution_id: String,
    pub session_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineProgressResponse {
    pub success: bool,
    pub execution_id: String,
    pub pipeline_id: Option<u64>,
    pub pipeline_name: Option<String>,
    pub task_id: Option<u64>,
    pub step_index: Option<u32>,
    pub status: String,
    pub progress_percent: u8,
    pub tokens_used: Option<u32>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineCancelRequest {
    pub execution_id: String,
    pub session_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineCancelResponse {
    pub success: bool,
    pub was_running: bool,
    pub error: Option<String>,
}

// Request/Response types (add with other types)
#[derive(Debug, Serialize, Deserialize)]
pub struct OrchestrateRequest {
    pub prompt: String,
    pub project_id: Option<u64>,
    pub workspace_id: Option<u64>,
    pub user_id: u64,
    pub device_id: u64,
    pub consciousness_enabled: bool,
    pub token_budget: Option<u32>,
    pub model_config: Option<serde_json::Value>,
    pub session_token: Option<String>,
    /// Files attached to this prompt — confirmed missing live: AppRuntime::
    /// orchestrate (lib.rs) already extracts an "attached_files" key from
    /// PipelineInput.data into OrchestrationRequest.attached_files, but this
    /// HTTP request struct had no field to receive it from a client at all,
    /// so every attached file was silently dropped before reaching the
    /// orchestrator regardless of what a caller sent.
    #[serde(default)]
    pub attached_files: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrchestrateResponse {
    pub success: bool,
    pub response: Option<String>,
    pub task_id: Option<u64>,
    pub blueprint_id: Option<u64>,
    pub stages_completed: Vec<serde_json::Value>,
    pub needs_clarification: bool,
    pub clarification_points: Vec<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    /// Which model actually produced `response` — lets the chat UI show
    /// "handled by X" and reflect a per-step model switch.
    pub model_used: Option<String>,
    pub total_tokens_used: Option<u32>,
    pub amt_summary: Option<serde_json::Value>,
    /// Full "thinking cycle" — one entry per real LLM call made this run
    /// (AMT-building passes, blueprint drafting, zero-shot simulation, step
    /// execution), each carrying the FULL raw response text, not the
    /// truncated summaries `stages_completed` carries. Lets the chat UI show
    /// the reasoning that produced the final response, not just the answer.
    #[serde(default)]
    pub thinking_log: Vec<serde_json::Value>,
}

// ============================================================================
// v0.4.0 - Pipeline Registry Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineRegistryRequest {
    pub session_token: Option<String>, // Optional - registry is semi-public
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRegistryEntry {
    pub id: u64,
    pub name: String,
    pub folder_name: String,
    pub category: String,
    pub has_ui: bool,
    pub is_tab: bool,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineRegistryResponse {
    pub success: bool,
    pub registry: Option<Vec<PipelineRegistryEntry>>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineUIComponentRequest {
    pub pipeline_id: u64,
    pub session_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineUIComponentResponse {
    pub success: bool,
    pub component_js: Option<String>,
    pub error: Option<String>,
}

// ============================================================================
// Route Handlers
// ============================================================================

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let runtime = state.runtime.read().await;
    let task_mgr = runtime.task_manager.read().await;
    let net_status = runtime.network.read().await.get_status().await;

    Json(HealthResponse {
        healthy: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: state.start_time.elapsed().as_secs(),
        active_tasks: task_mgr.active_count().await as u32,
        peer_count: net_status.connected_peers,
        p2p_enabled: net_status.enabled,
    })
}

async fn request_challenge(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, (StatusCode, String)> {
    let public_key = hex::decode(&req.public_key).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid public key: {}", e),
        )
    })?;

    let runtime = state.runtime.read().await;
    let auth = runtime.auth.read().await;

    match auth.create_challenge(&public_key).await {
        Ok(challenge) => Ok(Json(ChallengeResponse {
            challenge: hex::encode(&challenge.challenge),
            expires_at: challenge.expires_at,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn authenticate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AuthRequest>,
) -> Json<AuthResponse> {
    let public_key = match hex::decode(&req.public_key) {
        Ok(k) => k,
        Err(e) => {
            return Json(AuthResponse {
                success: false,
                session_token: None,
                user_id: None,
                device_id: None,
                expires_at: None,
                error: Some(format!("Invalid public key: {}", e)),
            })
        }
    };

    let signature = match hex::decode(&req.signature) {
        Ok(s) => s,
        Err(e) => {
            return Json(AuthResponse {
                success: false,
                session_token: None,
                user_id: None,
                device_id: None,
                expires_at: None,
                error: Some(format!("Invalid signature: {}", e)),
            })
        }
    };

    let runtime = state.runtime.write().await;

    match runtime.authenticate(&public_key, &signature).await {
        Ok(session) => Json(AuthResponse {
            success: true,
            session_token: Some(hex::encode(&session.session_token)),
            user_id: Some(session.user_id),
            device_id: Some(session.device_id),
            expires_at: Some(session.expires_at),
            error: None,
        }),
        Err(e) => Json(AuthResponse {
            success: false,
            session_token: None,
            user_id: None,
            device_id: None,
            expires_at: None,
            error: Some(e.to_string()),
        }),
    }
}

async fn execute_pipeline(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PipelineRequest>,
) -> Json<PipelineResponse> {
    let runtime = state.runtime.read().await;
    {
        let auth = runtime.auth.read().await;
        let token_bytes = hex::decode(&req.session_token).unwrap_or_default();
        match auth.validate_session(&token_bytes).await {
            Ok(_) => {}
            Err(_) => {
                return Json(PipelineResponse {
                    success: false,
                    task_id: Some(0),
                    output: None,
                    error: Some("Invalid session".into()),
                })
            }
        }
    }

    let input: crate::types::pipeline::PipelineInput = match serde_json::from_value(req.input) {
        Ok(i) => i,
        Err(e) => {
            return Json(PipelineResponse {
                success: false,
                task_id: Some(0),
                output: None,
                error: Some(format!("Invalid input: {}", e)),
            })
        }
    };

    match runtime.execute_pipeline(req.pipeline_id, input).await {
        Ok(output) => Json(PipelineResponse {
            success: output.success,
            task_id: output.task_id,
            output: Some(serde_json::to_value(&output.data).unwrap_or_default()),
            error: output.error,
        }),
        Err(e) => Json(PipelineResponse {
            success: false,
            task_id: Some(0),
            output: None,
            error: Some(e.to_string()),
        }),
    }
}

async fn get_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TaskRequest>,
) -> Json<Option<TaskInfo>> {
    let runtime = state.runtime.read().await;
    let task_mgr = runtime.task_manager.read().await;
    let _active_tasks = task_mgr.active_count().await as u32;

    match task_mgr.get_task(req.task_id).await {
        Some(task) => Json(Some(TaskInfo {
            task_id: task.task_id,
            blueprint_id: task.blueprint_id,
            blueprint_name: format!("Blueprint #{}", task.blueprint_id.unwrap_or(0)),
            status: format!("{:?}", task.status),
            progress: task.progress,
            created_at: task.created_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
            error: task.error.map(|e| format!("{:?}", e)),
            steps: task.steps,
            thinking_log: task.thinking_log,
            amt_summary: task.amt_summary,
        })),
        None => Json(None),
    }
}

async fn list_tasks(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TaskListRequest>,
) -> Json<TaskListResponse> {
    let runtime = state.runtime.read().await;
    let task_mgr = runtime.task_manager.read().await;

    let status_filter = req.status.as_deref();
    let limit = req.limit.unwrap_or(50) as usize;
    let offset = req.offset.unwrap_or(0) as usize;

    let tasks = task_mgr
        .list_tasks(status_filter, None, limit, offset)
        .await;
    let total = tasks.len() as u32;

    Json(TaskListResponse {
        tasks: tasks
            .into_iter()
            .map(|t| TaskInfo {
                task_id: t.task_id,
                blueprint_id: t.blueprint_id,
                blueprint_name: format!("Blueprint #{}", t.blueprint_id.unwrap_or(0)),
                status: format!("{:?}", t.status),
                progress: t.progress,
                created_at: t.created_at,
                started_at: t.started_at,
                completed_at: t.completed_at,
                error: t.error.map(|e| format!("{:?}", e)),
                steps: Vec::new(),
                thinking_log: Vec::new(),
                amt_summary: None,
            })
            .collect(),
        total,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskCancelResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Cooperative cancel: marks the task "cancelled" immediately; the
/// orchestrator's step loop (PromptOrchestrator::stage_6_to_8_execute_steps)
/// checks this between steps and stops issuing further ones. A step already
/// in flight when cancel lands still runs to completion — there is no
/// mid-step interrupt, only "don't start the next one."
async fn cancel_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TaskRequest>,
) -> Json<TaskCancelResponse> {
    let runtime = state.runtime.read().await;
    let task_mgr = runtime.task_manager.read().await;
    match task_mgr.cancel_task(req.task_id).await {
        Ok(()) => Json(TaskCancelResponse {
            success: true,
            error: None,
        }),
        Err(e) => Json(TaskCancelResponse {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

#[derive(Debug, Deserialize)]
pub struct StepRerunRequest {
    pub task_id: u64,
    pub step_index: u32,
    /// Same shape as orchestrator::ModelConfigOverride / pipeline 9's
    /// ModelOverrideConfig — forwarded through opaquely as JSON so this
    /// endpoint doesn't need to depend on either crate's exact type.
    pub model_override: Option<serde_json::Value>,
    /// true: build this step's context from the ORIGINAL recorded outputs
    /// of earlier steps (steps before step_index). false: run this step
    /// fresh with no prior-step context, standing alone under the new model.
    #[serde(default)]
    pub carry_forward_context: bool,
    #[serde(default)]
    pub session_token: String,
}

#[derive(Debug, Serialize)]
pub struct StepRerunResponse {
    pub success: bool,
    pub response: Option<String>,
    pub tokens_used: Option<u32>,
    pub model_used: Option<String>,
    pub error: Option<String>,
}

/// Rewind + rerun a single step of a completed/failed/in-progress task with
/// a different model, picking up exactly where the original left off rather
/// than replaying the whole orchestration. Operates directly against
/// PipelineRegistry (via the same RegistryExecutorAdapter the orchestrator
/// uses) — deliberately outside PromptOrchestrator's 14-stage state machine,
/// which has no "resume at step N" entry point.
async fn rerun_step(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StepRerunRequest>,
) -> Json<StepRerunResponse> {
    fn err(msg: impl Into<String>) -> Json<StepRerunResponse> {
        Json(StepRerunResponse {
            success: false,
            response: None,
            tokens_used: None,
            model_used: None,
            error: Some(msg.into()),
        })
    }

    let runtime = state.runtime.read().await;
    let task_mgr = runtime.task_manager.read().await;

    let task = match task_mgr.get_task(req.task_id).await {
        Some(t) => t,
        None => return err(format!("Task {} not found", req.task_id)),
    };

    let Some(blueprint_id) = task.blueprint_id else {
        return err("Task has no blueprint to re-derive the step from");
    };

    let container = match runtime.zsei.read().await.get_container(blueprint_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return err(format!("Blueprint {} not found", blueprint_id)),
        Err(e) => return err(format!("Blueprint lookup failed: {}", e)),
    };
    let container_json = serde_json::to_value(&container).unwrap_or_default();
    let steps = container_json
        .get("local_state")
        .and_then(|ls| ls.get("storage"))
        .and_then(|s| s.get("steps"))
        .and_then(|s| s.as_array().cloned())
        .unwrap_or_default();

    let Some(step_json) = steps
        .iter()
        .find(|s| s.get("step_index").and_then(|i| i.as_u64()) == Some(req.step_index as u64))
    else {
        return err(format!(
            "Step {} not found in blueprint {}",
            req.step_index, blueprint_id
        ));
    };

    let description = step_json
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("");
    let action = step_json
        .get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("execute");
    let pipeline_id = step_json
        .get("pipeline_id")
        .and_then(|p| p.as_u64())
        .unwrap_or(9);

    let context_text = if req.carry_forward_context {
        task.steps
            .iter()
            .filter(|s| s.step_index < req.step_index)
            .filter_map(|s| s.output_summary.clone())
            .collect::<Vec<_>>()
            .join("\n\n")
    } else {
        String::new()
    };

    let prompt = format!(
        "Step {}: {}\n\nContext:\n{}",
        req.step_index + 1,
        description,
        context_text
    );

    let mut exec_input = serde_json::json!({
        "prompt": prompt,
        "max_tokens": 2048,
        "temperature": 0.7,
        "action": action,
        // Rerun under the ORIGINAL task's context (was previously always a
        // default context — the step ran, but scoped to nobody in
        // particular). See RegistryExecutorAdapter::execute's
        // "_execution_context" handling.
        "_execution_context": {
            "user_id": task.user_id,
            "device_id": task.device_id,
            "workspace_id": task.workspace_id,
            "project_id": task.project_id,
            "task_context_id": null,
            "metadata": {},
        },
    });
    if let Some(mo) = &req.model_override {
        exec_input["model_override_config"] = mo.clone();
    }

    let adapter = crate::orchestrator::RegistryExecutorAdapter {
        registry: runtime.pipeline_registry.clone(),
    };
    use crate::orchestrator::PipelineExecutor;
    let result = adapter.execute(pipeline_id, exec_input).await;

    match result {
        Ok(output) => {
            let response_text = output
                .get("response")
                .and_then(|v| v.as_str())
                .map(String::from);
            let tokens_used = output.get("tokens_used").and_then(|v| v.as_u64()).map(|v| v as u32);
            let model_used = output
                .get("model_used")
                .and_then(|v| v.as_str())
                .map(String::from);

            let _ = task_mgr
                .update_step(
                    req.task_id,
                    req.step_index,
                    "completed",
                    tokens_used,
                    response_text.as_ref().map(|r| r[..200.min(r.len())].to_string()),
                    None,
                    action,
                    Some(format!(
                        "Rerun with model override (carry_forward_context={})",
                        req.carry_forward_context
                    )),
                    Vec::new(),
                    Vec::new(),
                    None,
                    Vec::new(),
                    Vec::new(),
                    model_used.clone(),
                )
                .await;

            Json(StepRerunResponse {
                success: true,
                response: response_text,
                tokens_used,
                model_used,
                error: None,
            })
        }
        Err(e) => err(e),
    }
}

async fn query_zsei(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ZseiQueryRequest>,
) -> Json<ZseiResponse> {
    let query: ZSEIQuery = match serde_json::from_value(req.query) {
        Ok(q) => q,
        Err(e) => {
            return Json(ZseiResponse {
                success: false,
                result: None,
                error: Some(format!("Invalid query: {}", e)),
            })
        }
    };

    let runtime = state.runtime.read().await;

    match runtime.query_zsei(query).await {
        Ok(result) => Json(ZseiResponse {
            success: true,
            result: Some(serde_json::to_value(&result).unwrap_or_default()),
            error: None,
        }),
        Err(e) => Json(ZseiResponse {
            success: false,
            result: None,
            error: Some(e.to_string()),
        }),
    }
}

async fn get_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConfigRequest>,
) -> Json<ConfigResponse> {
    let runtime = state.runtime.read().await;

    let config = match req.section.as_deref() {
        None | Some("") => serde_json::to_value(&runtime.config).ok(),
        Some("zsei") => serde_json::to_value(&runtime.config.zsei).ok(),
        Some("pipelines") => serde_json::to_value(&runtime.config.pipelines).ok(),
        Some("ui") => serde_json::to_value(&runtime.config.ui).ok(),
        Some("model") | Some("models") => serde_json::to_value(&runtime.config.models).ok(),
        Some("consciousness") => serde_json::to_value(&runtime.config.consciousness).ok(),
        Some("voice") => serde_json::to_value(&runtime.config.voice).ok(),
        Some("network") => serde_json::to_value(&runtime.config.network).ok(),
        Some("jurisdiction") => serde_json::to_value(&runtime.config.jurisdiction).ok(),
        Some("general") => serde_json::to_value(&runtime.config.general).ok(),
        Some("auth") => serde_json::to_value(&runtime.config.auth).ok(),
        Some("integrity") => serde_json::to_value(&runtime.config.integrity).ok(),
        Some("tasks") => serde_json::to_value(&runtime.config.tasks).ok(),
        Some("grpc") => serde_json::to_value(&runtime.config.grpc).ok(),
        Some("k_algorithms") => {
            // Report the live registry's actual current defaults (not just
            // the stored config) plus available options, so the UI never
            // shows a stale value after a runtime change.
            let k = crate::k_registry::KAlgorithms::global();
            let (convergence_preset, pairwise_preset) = k.current_presets();
            let (convergence_options, pairwise_options) = k.available_presets();
            serde_json::to_value(serde_json::json!({
                "convergence_preset": convergence_preset,
                "pairwise_preset": pairwise_preset,
                "convergence_options": convergence_options,
                "pairwise_options": pairwise_options,
            }))
            .ok()
        }
        Some(s) => {
            return Json(ConfigResponse {
                success: false,
                config: None,
                error: Some(format!("Unknown config section: {}", s)),
            })
        }
    };

    Json(ConfigResponse {
        success: config.is_some(),
        config,
        error: None,
    })
}

async fn set_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConfigSetRequest>,
) -> Json<ConfigSetResponse> {
    // Get config path
    let config_path = std::env::var("OZONE_CONFIG").unwrap_or_else(|_| "config.toml".to_string());

    // Read current config
    let mut runtime = state.runtime.write().await;

    // Apply updates from request
    if let Some(updates) = req.updates.as_object() {
        // Handle setup_complete flag
        if let Some(setup_complete) = updates.get("setup_complete") {
            if let Some(val) = setup_complete.as_bool() {
                runtime.config.general.setup_complete = val;
            }
        }

        // Handle user_setup_complete flag
        if let Some(user_setup) = updates.get("user_setup_complete") {
            if let Some(val) = user_setup.as_bool() {
                runtime.config.general.user_setup_complete = val;
            }
        }

        // Handle model updates
        if let Some(models) = updates.get("models") {
            let mut model_config = runtime.config.models.clone();

            if let Some(v) = models.get("model_type").and_then(|v| v.as_str()) {
                model_config.model_type = v.to_string();
            }
            if let Some(v) = models.get("api_provider").and_then(|v| v.as_str()) {
                // map to your actual fields
                model_config.api_endpoint = match v {
                    "anthropic" => Some("https://api.anthropic.com/v1/messages".to_string()),
                    "openai" => Some("https://api.openai.com/v1/chat/completions".to_string()),
                    "google" => {
                        Some("https://generativelanguage.googleapis.com/v1beta".to_string())
                    }
                    "openrouter" => {
                        Some("https://openrouter.ai/api/v1/chat/completions".to_string())
                    }
                    _ => model_config.api_endpoint, // already Option<String>
                };
                // Anthropic uses its own wire format; every other canned
                // provider here speaks OpenAI-style chat completions.
                if matches!(v, "openai" | "google" | "openrouter") {
                    model_config.wire_protocol = Some("chat_completions".to_string());
                } else if v == "anthropic" {
                    model_config.wire_protocol = Some("anthropic".to_string());
                }
            }
            if let Some(v) = models.get("api_key").and_then(|v| v.as_str()) {
                // Raw secret value — belongs in api_key (persisted, gitignored
                // config.toml), never in api_key_env (that field is a NAME,
                // e.g. "ANTHROPIC_API_KEY", not a value).
                model_config.api_key = Some(v.to_string());
            }
            if let Some(v) = models.get("api_key_env").and_then(|v| v.as_str()) {
                model_config.api_key_env = Some(v.to_string());
            }
            if let Some(v) = models.get("api_endpoint").and_then(|v| v.as_str()) {
                model_config.api_endpoint = Some(v.to_string());
            }
            if let Some(v) = models.get("api_model").and_then(|v| v.as_str()) {
                model_config.api_model = Some(v.to_string());
            }
            if let Some(v) = models.get("context_length").and_then(|v| v.as_u64()) {
                model_config.context_length = v as usize;
            }
            if let Some(v) = models.get("gpu_layers") {
                model_config.gpu_layers = v.as_u64().map(|n| n as u32);
            }
            if let Some(v) = models.get("allow_user_selection").and_then(|v| v.as_bool()) {
                model_config.allow_user_selection = v;
            }
            if let Some(v) = models.get("local_model_path").and_then(|v| v.as_str()) {
                model_config.local_model_path = Some(v.to_string());
            }
            if let Some(v) = models.get("local_model_type").and_then(|v| v.as_str()) {
                model_config.local_model_type = Some(v.to_string());
            }
            if let Some(v) = models.get("wire_protocol").and_then(|v| v.as_str()) {
                model_config.wire_protocol = Some(v.to_string());
            }
            if let Some(v) = models.get("bitnet_cli_path").and_then(|v| v.as_str()) {
                model_config.bitnet_cli_path = Some(v.to_string());
            }

            // available_models: discrete add/remove rather than whole-list
            // replace — Settings and the setup wizard both write this list,
            // and a wholesale replace risks one clobbering the other's
            // concurrent edit on a stale fetch-then-save.
            if let Some(v) = models.get("add_model") {
                if let Ok(m) = serde_json::from_value::<crate::config::AvailableModel>(v.clone()) {
                    model_config.available_models.retain(|existing| existing.identifier != m.identifier);
                    model_config.available_models.push(m);
                }
            }
            if let Some(v) = models.get("remove_model").and_then(|v| v.as_str()) {
                model_config.available_models.retain(|m| m.identifier != v);
            }

            // Multi-provider fallback chain — user-defined order + free-only
            // gate (see ModelFallbackConfig, PromptOrchestrator::try_fallback_chain).
            if let Some(fallback) = models.get("fallback") {
                if let Some(order) = fallback.get("order").and_then(|v| v.as_array()) {
                    model_config.fallback.order = order
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }
                if let Some(v) = fallback.get("free_only").and_then(|v| v.as_bool()) {
                    model_config.fallback.free_only = v;
                }
            }

            // Re-export env BEFORE moving into runtime config, so spawned or
            // connected pipeline-9 instances pick up new settings immediately.
            for (k, v) in model_config.to_pipeline_env() {
                std::env::set_var(&k, &v);
            }
            runtime.config.models = model_config;
        }

        // Handle consciousness updates
        if let Some(consciousness) = updates.get("consciousness") {
            let c = &mut runtime.config.consciousness;
            if let Some(v) = consciousness.get("enabled").and_then(|v| v.as_bool()) {
                c.enabled = v;
            }
            if let Some(v) = consciousness.get("emotional_system_enabled").and_then(|v| v.as_bool()) {
                c.emotional_system_enabled = v;
            }
            if let Some(v) = consciousness.get("experience_memory_enabled").and_then(|v| v.as_bool()) {
                c.experience_memory_enabled = v;
            }
            if let Some(v) = consciousness.get("identity_system_enabled").and_then(|v| v.as_bool()) {
                c.identity_system_enabled = v;
            }
            if let Some(v) = consciousness.get("relationship_system_enabled").and_then(|v| v.as_bool()) {
                c.relationship_system_enabled = v;
            }
            if let Some(v) = consciousness.get("ethical_system_enabled").and_then(|v| v.as_bool()) {
                c.ethical_system_enabled = v;
            }
            if let Some(v) = consciousness.get("collective_enabled").and_then(|v| v.as_bool()) {
                c.collective_enabled = v;
            }
            if let Some(v) = consciousness.get("show_emotional_state").and_then(|v| v.as_bool()) {
                c.show_emotional_state = v;
            }
            if let Some(v) = consciousness.get("show_decision_reasoning").and_then(|v| v.as_bool()) {
                c.show_decision_reasoning = v;
            }
            if let Some(v) = consciousness.get("i_loop_interval_ms").and_then(|v| v.as_u64()) {
                c.i_loop_interval_ms = v;
            }
            if let Some(v) = consciousness.get("playback_enabled").and_then(|v| v.as_bool()) {
                c.playback_enabled = v;
            }
        }

        if let Some(voice) = updates.get("voice") {
            let mut voice_config = runtime.config.voice.clone();

            if let Some(enabled) = voice.get("enabled").and_then(|v| v.as_bool()) {
                voice_config.enabled = enabled;
            }
            if let Some(path) = voice.get("whisper_model_path").and_then(|v| v.as_str()) {
                voice_config.whisper_model_path = Some(path.to_string());
            }
            if let Some(v) = voice.get("backend").and_then(|v| v.as_str()) {
                voice_config.backend = v.to_string();
            }
            if let Some(v) = voice.get("whisper_cpp_path").and_then(|v| v.as_str()) {
                voice_config.whisper_cpp_path = Some(v.to_string());
            }
            if let Some(v) = voice.get("api_endpoint").and_then(|v| v.as_str()) {
                voice_config.api_endpoint = Some(v.to_string());
            }
            if let Some(v) = voice.get("api_key").and_then(|v| v.as_str()) {
                voice_config.api_key = Some(v.to_string());
            }
            if let Some(v) = voice.get("api_key_env").and_then(|v| v.as_str()) {
                voice_config.api_key_env = Some(v.to_string());
            }
            if let Some(v) = voice.get("language").and_then(|v| v.as_str()) {
                voice_config.language = Some(v.to_string());
            }
            if let Some(v) = voice.get("ffmpeg_path").and_then(|v| v.as_str()) {
                voice_config.ffmpeg_path = v.to_string();
            }

            runtime.config.voice = voice_config.clone();

            // Voice pipeline (#10) children inherit these — no per-pipeline
            // host code needed (inherited environment).
            for (k, v) in voice_config.to_pipeline_env() {
                std::env::set_var(&k, &v);
            }
        }

        // Handle network updates — persisted only; NetworkManager is
        // initialized once at boot (OzoneRuntime::new) and isn't live-
        // reconfigured, so these need a restart to take effect (same
        // behavior as every other section in this handler).
        if let Some(network) = updates.get("network") {
            let n = &mut runtime.config.network;
            if let Some(v) = network.get("enable_p2p").and_then(|v| v.as_bool()) {
                n.enable_p2p = v;
            }
            if let Some(v) = network.get("enable_cloud_sync").and_then(|v| v.as_bool()) {
                n.enable_cloud_sync = v;
            }
            if let Some(v) = network.get("p2p_port").and_then(|v| v.as_u64()) {
                n.p2p_port = v as u16;
            }
            if let Some(v) = network.get("max_peers").and_then(|v| v.as_u64()) {
                n.max_peers = v as u32;
            }
            if let Some(v) = network.get("enable_mdns").and_then(|v| v.as_bool()) {
                n.enable_mdns = v;
            }
            if let Some(v) = network.get("batch_sync_interval_secs").and_then(|v| v.as_u64()) {
                n.batch_sync_interval_secs = v;
            }
        }

        // Jurisdiction gate config (src/orchestrator/jurisdiction.rs) — the
        // gate itself always runs; this only controls whether it's allowed
        // to enforce anything, and which region's ruleset it looks for.
        // Ships with zero real rule content regardless of these values.
        if let Some(jurisdiction) = updates.get("jurisdiction") {
            let j = &mut runtime.config.jurisdiction;
            if let Some(v) = jurisdiction.get("enabled").and_then(|v| v.as_bool()) {
                j.enabled = v;
            }
            if let Some(v) = jurisdiction.get("instance_region").and_then(|v| v.as_str()) {
                j.instance_region = if v.trim().is_empty() { None } else { Some(v.to_string()) };
            }
        }

        // Handle K-ALGORITHM preset updates — applies to both the persisted
        // config (survives restart) and the live global registry
        // (orchestrator/amt.rs picks it up on its very next call, no
        // restart needed). set_*_preset silently no-ops on an unknown name
        // rather than erroring, so an invalid value here just keeps the
        // previous default.
        if let Some(k_alg) = updates.get("k_algorithms") {
            let k = crate::k_registry::KAlgorithms::global();
            if let Some(v) = k_alg.get("convergence_preset").and_then(|v| v.as_str()) {
                if k.set_convergence_preset(v) {
                    runtime.config.k_algorithms.convergence_preset = v.to_string();
                }
            }
            if let Some(v) = k_alg.get("pairwise_preset").and_then(|v| v.as_str()) {
                if k.set_pairwise_preset(v) {
                    runtime.config.k_algorithms.pairwise_preset = v.to_string();
                }
            }
        }

        // Handle UI updates
        if let Some(ui) = updates.get("ui") {
            if let Ok(ui_config) = serde_json::from_value(ui.clone()) {
                runtime.config.ui = ui_config;
            }
        }
    }

    // Save config to file
    match toml::to_string_pretty(&runtime.config) {
        Ok(config_str) => match std::fs::write(&config_path, &config_str) {
            Ok(_) => Json(ConfigSetResponse {
                success: true,
                error: None,
            }),
            Err(e) => Json(ConfigSetResponse {
                success: false,
                error: Some(format!("Failed to write config: {}", e)),
            }),
        },
        Err(e) => Json(ConfigSetResponse {
            success: false,
            error: Some(format!("Failed to serialize config: {}", e)),
        }),
    }
}

// ============================================================================
// Orchestration Handler — full 14-stage AMT flow
// ============================================================================

async fn orchestrate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OrchestrateRequest>,
) -> Json<OrchestrateResponse> {
    let start = std::time::Instant::now();

    // Build the pipeline input that the orchestrator understands
    let mut data = std::collections::HashMap::new();
    data.insert(
        "prompt".to_string(),
        serde_json::Value::String(req.prompt.clone()),
    );
    data.insert(
        "consciousness_enabled".to_string(),
        serde_json::Value::Bool(req.consciousness_enabled),
    );
    data.insert(
        "token_budget".to_string(),
        serde_json::json!(req.token_budget.unwrap_or(100_000)),
    );
    if let Some(proj_id) = req.project_id {
        data.insert("project_id".to_string(), serde_json::json!(proj_id));
    }
    if let Some(ws_id) = req.workspace_id {
        data.insert("workspace_id".to_string(), serde_json::json!(ws_id));
    }
    if !req.attached_files.is_empty() {
        data.insert(
            "attached_files".to_string(),
            serde_json::Value::Array(req.attached_files.clone()),
        );
    }
    if let Some(model_cfg) = &req.model_config {
        // Full object under "model_config" — AppRuntime::orchestrate (lib.rs)
        // deserializes this key into OrchestrationRequest.model_config
        // (ModelConfigOverride). Previously only "model_identifier" was
        // flattened out here into a key nothing else read, so per-request
        // model overrides never actually reached the orchestrator.
        data.insert("model_config".to_string(), model_cfg.clone());
        if let Some(model_id) = model_cfg.get("model_identifier").and_then(|v| v.as_str()) {
            data.insert(
                "model_identifier".to_string(),
                serde_json::Value::String(model_id.to_string()),
            );
        }
    }

    let pipeline_input = crate::types::pipeline::PipelineInput {
        data: data
            .into_iter()
            .map(|(k, v)| (k, json_to_typed_value(v)))
            .collect(),
        context: crate::types::pipeline::ExecutionContext {
            user_id: req.user_id,
            device_id: req.device_id,
            workspace_id: req.workspace_id,
            project_id: req.project_id,
            task_context_id: None,
            metadata: std::collections::HashMap::new(),
        },
    };

    let runtime = state.runtime.read().await;

    // Run through the orchestrator — full 14-stage AMT flow:
    // Stage 1-3: Intent capture (IntentCapture, BranchCapture, DetailCapture)
    // Stage 4-5: Context aggregation + cross-reference
    // Stage 6-7: Blueprint search + selection
    // Stage 8-12: Pipeline execution per blueprint step
    // Stage 13-14: Response synthesis + consciousness post-hook
    match runtime
        .orchestrate(pipeline_input, req.user_id, req.device_id)
        .await
    {
        Ok(result) => {
            let execution_time_ms = start.elapsed().as_millis() as u64;
            tracing::info!(
                "Orchestration complete: task={:?}, blueprint={:?}, {}ms",
                result.task_id,
                result.blueprint_id,
                execution_time_ms
            );
            Json(OrchestrateResponse {
                success: result.success,
                response: result.response_text,
                task_id: result.task_id,
                blueprint_id: result.blueprint_id,
                stages_completed: result
                    .stages_completed
                    .into_iter()
                    .map(|s| serde_json::to_value(s).unwrap_or_default())
                    .collect(),
                needs_clarification: result.needs_clarification,
                clarification_points: result.clarification_points,
                error: result.error,
                execution_time_ms,
                model_used: result.model_used,
                total_tokens_used: result.total_tokens_used,
                amt_summary: result.amt_summary,
                thinking_log: result.thinking_log,
            })
        }
        Err(e) => {
            tracing::error!("Orchestration failed: {}", e);
            Json(OrchestrateResponse {
                success: false,
                response: None,
                task_id: None,
                blueprint_id: None,
                stages_completed: vec![],
                needs_clarification: false,
                clarification_points: vec![],
                error: Some(e.to_string()),
                execution_time_ms: start.elapsed().as_millis() as u64,
                model_used: None,
                total_tokens_used: None,
                amt_summary: None,
                thinking_log: vec![],
            })
        }
    }
}

// ============================================================================
// v0.4.0 - Pipeline Registry Handlers
// ============================================================================

/// Get full pipeline registry - THE SINGLE SOURCE OF TRUTH
async fn get_pipeline_registry(
    Json(_req): Json<PipelineRegistryRequest>,
) -> Json<PipelineRegistryResponse> {
    // Build registry from the authoritative source
    // This mirrors what's in src/pipeline/registry.rs PIPELINE_INFO
    let registry = build_pipeline_registry();

    Json(PipelineRegistryResponse {
        success: true,
        registry: Some(registry),
        error: None,
    })
}

/// Build the full pipeline registry
fn build_pipeline_registry() -> Vec<PipelineRegistryEntry> {
    let index_path = std::env::var("OZONE_PIPELINES_INDEX")
        .unwrap_or_else(|_| "./zsei_data/pipelines/index.json".to_string());

    if let Ok(content) = std::fs::read_to_string(&index_path) {
        if let Ok(index) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(pipelines) = index.get("pipelines").and_then(|p| p.as_array()) {
                let entries: Vec<PipelineRegistryEntry> = pipelines
                    .iter()
                    .filter_map(|p| {
                        let id = p.get("pipeline_id")?.as_u64()?;
                        let name = p.get("name")?.as_str()?.to_string();
                        let folder_name = p.get("folder_name")?.as_str()?.to_string();
                        let category = p.get("category")?.as_str()?.to_string();
                        let has_ui = p.get("has_ui").and_then(|h| h.as_bool()).unwrap_or(false);
                        let is_tab = p.get("is_tab").and_then(|t| t.as_bool()).unwrap_or(false);
                        let description = p
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some(PipelineRegistryEntry {
                            id,
                            name,
                            folder_name,
                            category,
                            has_ui,
                            is_tab,
                            description,
                        })
                    })
                    .collect();

                if !entries.is_empty() {
                    return entries;
                }
            }
        }
    }

    // Fallback to compile-time PIPELINE_INFO when index.json is missing/invalid
    tracing::warn!("Pipeline index not found, falling back to compile-time registry");
    crate::pipeline::PIPELINE_INFO
        .iter()
        .map(|(id, info)| PipelineRegistryEntry {
            id: *id,
            name: info.name.clone(),
            folder_name: info.folder_name.clone(),
            category: info.category.to_string(),
            has_ui: info.has_ui,
            is_tab: info.is_tab,
            description: info.description.clone(),
        })
        .collect()
}

/// Get pipeline UI component.js content
async fn get_pipeline_ui_component(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PipelineUIComponentRequest>,
) -> Json<PipelineUIComponentResponse> {
    let pipeline_id = req.pipeline_id;

    // Get folder name from registry
    let registry = build_pipeline_registry();
    let entry = registry.iter().find(|e| e.id == pipeline_id);

    let (category, folder_name) = match entry {
        Some(e) => (e.category.as_str(), e.folder_name.as_str()),
        None => {
            return Json(PipelineUIComponentResponse {
                success: false,
                component_js: None,
                error: Some(format!("Pipeline {} not found in registry", pipeline_id)),
            });
        }
    };

    // Try to load component.js from the pipeline's ui folder. The index's
    // category names don't always match on-disk asset folders ("core" vs
    // "general"), so search every known category dir; the data-dir copy
    // (where bootstrap places it) comes first.
    let pipelines_path =
        std::env::var("OZONE_PIPELINES_PATH").unwrap_or_else(|_| "./pipelines".to_string());
    let data_pipelines = {
        let r = state.runtime.read().await;
        format!("{}/pipelines", r.config.general.data_dir)
    };

    let category_variants = [category, "general", "consciousness", "modalities", "shared"];
    let bases = [data_pipelines, pipelines_path, "./pipelines".to_string()];
    let mut tried: Vec<String> = Vec::new();
    for base in &bases {
        for cat in category_variants {
            let path = format!("{}/{}/{}/ui/component.js", base, cat, folder_name);
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    return Json(PipelineUIComponentResponse {
                        success: true,
                        component_js: Some(content),
                        error: None,
                    });
                }
                Err(_) => tried.push(path),
            }
        }
    }

    Json(PipelineUIComponentResponse {
        success: false,
        component_js: None,
        error: Some(format!(
            "No UI component found for pipeline {} (tried {} paths, folder {:?})",
            pipeline_id,
            tried.len(),
            folder_name
        )),
    })
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: Arc<AppState>) {
    // Start progress broadcast task
    let progress_map = state.executor_progress.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);

    tokio::spawn(async move {
        let mut last_snapshot: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let map = progress_map.read().await;
            for (id, progress) in map.iter() {
                let status = format!("{:?}", progress.status);
                if last_snapshot.get(id) != Some(&status) {
                    last_snapshot.insert(id.clone(), status.clone());
                    let event = serde_json::json!({
                        "action": "pipeline_progress",
                        "execution_id": id,
                        "status": status,
                        "progress_percent": progress.progress_percent,
                        "pipeline_id": progress.pipeline_id,
                        "pipeline_name": progress.pipeline_name,
                        "task_id": progress.task_id,
                        "step_index": progress.step_index,
                        "tokens_used": progress.tokens_used,
                    });
                    if tx
                        .send(serde_json::to_string(&event).unwrap_or_default())
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    // Main message loop
    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(request) = serde_json::from_str::<serde_json::Value>(&text) {
                            let response = handle_ws_message(request, &state).await;
                            let _ = socket.send(Message::Text(
                                serde_json::to_string(&response).unwrap_or_default()
                            )).await;
                        }
                    }
                    None | Some(Ok(Message::Close(_))) | Some(Err(_)) => break,
                    _ => {}
                }
            }
            event = rx.recv() => {
                if let Some(event_str) = event {
                    let _ = socket.send(Message::Text(event_str)).await;
                }
            }
        }
    }
}

async fn handle_ws_message(
    request: serde_json::Value,
    _state: &Arc<AppState>,
) -> serde_json::Value {
    let action = request.get("action").and_then(|v| v.as_str()).unwrap_or("");

    match action {
        "ping" => serde_json::json!({"action": "pong"}),
        "subscribe_tasks" => serde_json::json!({"action": "subscribed", "channel": "tasks"}),
        "subscribe_pipeline_progress" => {
            let execution_id = request
                .get("execution_id")
                .and_then(|e| e.as_str())
                .unwrap_or("")
                .to_string();
            serde_json::json!({
                "action": "subscribed",
                "channel": "pipeline_progress",
                "execution_id": execution_id
            })
        }
        "cancel_pipeline" => {
            let execution_id = request
                .get("execution_id")
                .and_then(|e| e.as_str())
                .unwrap_or("")
                .to_string();
            serde_json::json!({
                "action": "cancel_requested",
                "execution_id": execution_id
            })
        }
        _ => serde_json::json!({"error": "Unknown action"}),
    }
}

// ============================================================================
// Server Startup
// ============================================================================

/// Start the HTTP/WebSocket server
pub async fn start_server(runtime: Arc<RwLock<OzoneRuntime>>) -> OzoneResult<()> {
    let config = {
        let r = runtime.read().await;
        r.config.grpc.clone()
    };
    let addr = format!("{}:{}", config.address, config.port);

    let progress_map = {
        let r = runtime.read().await;
        let map = r.pipeline_registry.read().await.progress_map();
        map
    };

    let state = Arc::new(AppState {
        runtime,
        start_time: std::time::Instant::now(),
        executor_progress: progress_map,
        pairing: Arc::new(crate::pairing::PairingHub::new()),
        mcp: Arc::new(crate::mcp::McpRegistry::new()),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/challenge", post(request_challenge))
        .route("/auth/authenticate", post(authenticate))
        .route("/pipeline/execute", post(execute_pipeline))
        .route("/pipeline/registry", post(get_pipeline_registry))
        .route("/pipeline/ui-component", post(get_pipeline_ui_component))
        .route("/task/get", post(get_task))
        .route("/task/list", post(list_tasks))
        .route("/task/cancel", post(cancel_task))
        .route("/task/step/rerun", post(rerun_step))
        .route("/zsei/query", post(query_zsei))
        .route("/config/get", post(get_config))
        .route("/config/set", post(set_config))
        .route("/ws", get(websocket_handler))
        .route("/pipeline/progress", post(get_pipeline_progress))
        .route("/pipeline/cancel", post(cancel_pipeline))
        .route("/orchestrate", post(orchestrate))
        // CONNECT MODEL — pipelines register themselves here on boot; the
        // executor dispatches to registered remotes before any spawn.
        .route("/pipelines/remote", get(list_remote_pipelines))
        .route("/pipelines/register", post(register_remote_pipeline))
        .route("/pipelines/unregister", post(unregister_remote_pipeline))
        // MONITOR — activity hub (dashboard feed, browser plugin, ZCode).
        .route("/monitor/activity", get(list_activity))
        .route("/monitor/activity", post(push_activity))
        .route("/monitor/summary", get(monitor_summary))
        // PAIRING — QR multi-device onboarding, the phone as authenticator.
        .route("/pairing/start", post(start_pairing))
        .route("/pairing/status", get(pairing_status))
        .route("/pairing/approve", post(approve_pairing))
        .route("/pair/:code", get(pair_page))
        .route("/devices", get(list_devices))
        // MCP TOOLS — external tool registration (the 90+ tool surface).
        .route("/mcp/tools", get(mcp_list_tools))
        .route("/mcp/tools/register", post(mcp_register_tool))
        .route("/mcp/tools/unregister", post(mcp_unregister_tool))
        .layer(cors)
        .with_state(state);

    tracing::info!("Starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| OzoneError::ServerError(format!("Failed to bind: {}", e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| OzoneError::ServerError(format!("Server error: {}", e)))?;

    Ok(())
}

pub fn to_status(error: OzoneError) -> StatusCode {
    match error {
        OzoneError::AuthError(_) => StatusCode::UNAUTHORIZED,
        OzoneError::NotFound(_) => StatusCode::NOT_FOUND,
        OzoneError::PermissionDenied(_) => StatusCode::FORBIDDEN,
        OzoneError::ValidationError(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn get_pipeline_progress(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PipelineProgressRequest>,
) -> Json<PipelineProgressResponse> {
    let map = state.executor_progress.read().await;
    match map.get(&req.execution_id) {
        Some(progress) => Json(PipelineProgressResponse {
            success: true,
            execution_id: req.execution_id,
            pipeline_id: Some(progress.pipeline_id),
            pipeline_name: Some(progress.pipeline_name.clone()),
            task_id: progress.task_id,
            step_index: progress.step_index,
            status: format!("{:?}", progress.status),
            progress_percent: progress.progress_percent,
            tokens_used: progress.tokens_used,
            started_at: Some(progress.started_at),
            completed_at: progress.completed_at,
            error: progress.error.clone(),
        }),
        None => Json(PipelineProgressResponse {
            success: false,
            execution_id: req.execution_id,
            pipeline_id: None,
            pipeline_name: None,
            task_id: None,
            step_index: None,
            status: "NotFound".to_string(),
            progress_percent: 0,
            tokens_used: None,
            started_at: None,
            completed_at: None,
            error: Some("Execution not found".to_string()),
        }),
    }
}

async fn cancel_pipeline(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PipelineCancelRequest>,
) -> Json<PipelineCancelResponse> {
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    let was_running = registry.cancel_execution(&req.execution_id).await;

    Json(PipelineCancelResponse {
        success: true,
        was_running,
        error: None,
    })
}

// ============================================================================
// CONNECT MODEL — pipeline self-registration (see src/pipeline/remote.rs)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RemotePipelineRegisterRequest {
    pub pipeline_id: u64,
    pub name: String,
    /// Endpoint accepting POST <PipelineInput JSON> → one JSON output object.
    pub execute_url: String,
    /// "agent" | "model" | "observer" — models serve pipeline-9 model calls;
    /// observers only push/consume monitor activity. Absent = ["agent"].
    #[serde(default)]
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct RemotePipelineRegisterResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RemotePipelineUnregisterRequest {
    pub pipeline_id: u64,
}

#[derive(Debug, Serialize)]
pub struct RemotePipelineUnregisterResponse {
    pub success: bool,
    pub was_registered: bool,
}

#[derive(Debug, Serialize)]
pub struct RemotePipelineListResponse {
    pub pipelines: Vec<crate::pipeline::RemotePipelineInfo>,
}

/// A pipeline booting elsewhere announces itself here. Latest registration
/// for an id wins (reconnect / replacement). Re-registration is the
/// heartbeat — the dashboard reads `registered_at` as "last seen".
async fn register_remote_pipeline(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RemotePipelineRegisterRequest>,
) -> Json<RemotePipelineRegisterResponse> {
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    let roles = req
        .roles
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| vec!["agent".to_string()]);
    let entry = registry
        .remote_pipelines()
        .register(req.pipeline_id, req.name, req.execute_url, roles)
        .await;
    // Seed the execution gate (registry.blueprints) so this id can actually
    // be dispatched to: RegistryExecutorAdapter::execute and
    // PipelineRegistry::execute both refuse any pipeline_id absent from that
    // map BEFORE remote dispatch is ever attempted, and it's normally only
    // populated at boot from the compile-time PIPELINE_INFO table (ids
    // 1-55) — a fresh remote id like 9001 would otherwise 404 here even
    // though RemotePipelines itself is ready to serve it.
    let _ = registry
        .register_custom(crate::types::pipeline::PipelineBlueprint {
            pipeline_id: entry.pipeline_id,
            name: entry.name.clone(),
            version: crate::types::SemVer::default(),
            author: Vec::new(),
            description: format!(
                "Remote-registered agent: {} (roles: {})",
                entry.name,
                entry.roles.join("+")
            ),
            specification: crate::types::pipeline::BlueprintSpec {
                input_schema: crate::types::pipeline::Schema::default(),
                output_schema: crate::types::pipeline::Schema::default(),
                dependencies: Vec::new(),
                sub_pipelines: Vec::new(),
                execution_flow: crate::types::pipeline::ExecutionFlow::Sequential(Vec::new()),
            },
            implementations: Vec::new(),
            content_hash: [0u8; 32],
            peers: Vec::new(),
            consensus_status: crate::types::pipeline::ConsensusStatus::Accepted,
            verified_by: 0,
        })
        .await;
    // Capture the landing in the monitor feed — one observable registry.
    registry.activity_hub().record(
        crate::monitor::ActivityKind::Agent,
        crate::monitor::ActivityLevel::Ok,
        &entry.name,
        format!(
            "Agent connected: {} (roles: {})",
            entry.name,
            entry.roles.join(", ")
        ),
        Some(serde_json::json!({
            "pipeline_id": entry.pipeline_id,
            "execute_url": entry.execute_url,
        })),
    );
    tracing::info!(
        pipeline_id = entry.pipeline_id,
        name = %entry.name,
        url = %entry.execute_url,
        "Remote pipeline registered"
    );
    Json(RemotePipelineRegisterResponse {
        success: true,
        error: None,
    })
}

async fn unregister_remote_pipeline(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RemotePipelineUnregisterRequest>,
) -> Json<RemotePipelineUnregisterResponse> {
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    let was_registered = registry.remote_pipelines().deregister(req.pipeline_id).await;
    let _ = registry.unregister_custom(req.pipeline_id).await;
    Json(RemotePipelineUnregisterResponse {
        success: true,
        was_registered,
    })
}

async fn list_remote_pipelines(State(state): State<Arc<AppState>>) -> Json<RemotePipelineListResponse> {
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    Json(RemotePipelineListResponse {
        pipelines: registry.remote_pipelines().list().await,
    })
}

// ============================================================================
// MONITOR — activity hub endpoints (dashboard + browser plugin + ZCode)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    pub limit: Option<usize>,
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ActivityPush {
    pub kind: String,     // log | agent | tool | job | bridge | external
    pub level: String,    // info | ok | warn | error
    pub source: String,   // who recorded it ("browser-plugin", "zcode", …)
    pub message: String,
    #[serde(default)]
    pub detail: Option<serde_json::Value>,
}

fn parse_kind(s: &str) -> Option<crate::monitor::ActivityKind> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

async fn list_activity(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<ActivityQuery>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    let hub = registry.activity_hub();
    let kind = q.kind.as_deref().and_then(parse_kind);
    let events = hub.recent(q.limit.unwrap_or(100), kind);
    Json(serde_json::json!({ "events": events }))
}

/// External observers (browser plugin, ZCode connector) push activity here.
async fn push_activity(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActivityPush>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    let hub = registry.activity_hub();
    let kind = parse_kind(&req.kind)
        .unwrap_or(crate::monitor::ActivityKind::External);
    let level = serde_json::from_value::<crate::monitor::ActivityLevel>(
        serde_json::Value::String(req.level),
    )
    .unwrap_or(crate::monitor::ActivityLevel::Info);
    let event = hub.record(kind, level, &req.source, req.message, req.detail);
    Json(serde_json::json!({ "success": true, "id": event.id }))
}

/// One-shot summary: agents + latest activity in a single call for dashboards.
async fn monitor_summary(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    let agents = registry.remote_pipelines().list().await;
    let events = registry.activity_hub().recent(50, None);
    Json(serde_json::json!({
        "agents": agents,
        "activity": events,
    }))
}

// ============================================================================
// PAIRING — QR multi-device onboarding, the phone as authenticator
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PairingStartRequest {
    /// What is asking to pair, shown to the approver ("Desktop UI", "Web …").
    #[serde(default)]
    pub device_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PairingApproveRequest {
    pub code: String,
    #[serde(default)]
    pub device_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PairingStatusQuery {
    pub pairing_id: String,
}

async fn start_pairing(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PairingStartRequest>,
) -> Json<serde_json::Value> {
    let (pairing_id, code, expires_at) = state
        .pairing
        .start(req.device_hint.unwrap_or_else(|| "Device".into()))
        .await;

    // The QR encodes the phone-reachable approve URL on the host itself.
    let config = {
        let r = state.runtime.read().await;
        r.config.grpc.clone()
    };
    let host = crate::pairing::pairing_public_host(&config.address);
    let port = config.port;
    let approve_url = format!("http://{}:{}/pair/{}", host, port, code);
    let qr_payload = format!("ozone://pair/v1?host={}&code={}", host, code);

    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    registry.activity_hub().record(
        crate::monitor::ActivityKind::Bridge,
        crate::monitor::ActivityLevel::Info,
        "pairing",
        format!("Pairing started — code {} (device: waiting for scan)", code),
        None,
    );

    Json(serde_json::json!({
        "pairing_id": pairing_id,
        "code": code,
        "approve_url": approve_url,
        "qr_payload": qr_payload,
        "expires_at": expires_at,
    }))
}

async fn pairing_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<PairingStatusQuery>,
) -> Json<serde_json::Value> {
    let (status, token, device_id, expires_at) = state.pairing.status(&q.pairing_id).await;
    Json(serde_json::json!({
        "status": status,
        "session_token": token,
        "device_id": device_id,
        "expires_at": expires_at,
    }))
}

async fn approve_pairing(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PairingApproveRequest>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let auth = runtime.auth.read().await;
    match state
        .pairing
        .approve(
            &req.code,
            req.device_name.unwrap_or_else(|| "Paired device".into()),
            crate::types::auth::DeviceType::Mobile,
            &auth,
        )
        .await
    {
        Ok(device_id) => {
            let runtime = state.runtime.read().await;
            let registry = runtime.pipeline_registry.read().await;
            registry.activity_hub().record(
                crate::monitor::ActivityKind::Bridge,
                crate::monitor::ActivityLevel::Ok,
                "pairing",
                format!("Device {} approved by phone — session issued", device_id),
                None,
            );
            Json(serde_json::json!({ "success": true, "device_id": device_id }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "error": e })),
    }
}

/// Phone-facing approve page (scanned from the QR).
async fn pair_page(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(code): axum::extract::Path<String>,
) -> axum::response::Html<String> {
    let config = {
        let r = state.runtime.read().await;
        r.config.grpc.clone()
    };
    axum::response::Html(crate::pairing::approve_page_html(
        &code,
        &format!("{}:{}", config.address, config.port),
    ))
}

/// Multi-device registry — every paired device on this host.
async fn list_devices(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    use crate::types::auth::DeviceStatus;
    let runtime = state.runtime.read().await;
    let auth = runtime.auth.read().await;
    let mut devices = Vec::new();
    for user_id in 1..1000u64 {
        if let Some(user) = auth.get_user(user_id).await {
            for d in &user.registered_devices {
                devices.push(serde_json::json!({
                    "device_id": d.device_id,
                    "device_name": d.device_name,
                    "device_type": format!("{:?}", d.device_type),
                    "registered_at": d.registered_at,
                    "last_seen": d.last_seen,
                    "online": matches!(d.status, DeviceStatus::Online),
                }));
            }
        }
    }
    Json(serde_json::json!({ "devices": devices }))
}

// ============================================================================
// MCP TOOLS — external tool registration (see src/mcp.rs)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct McpToolRegisterRequest {
    pub name: String,
    /// "stdio" | "http" | "sse"
    pub transport: String,
    /// Command+args (stdio) or URL (http/sse).
    pub endpoint: String,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    #[serde(default)]
    pub server_version: Option<String>,
}

async fn mcp_list_tools(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let tools = state.mcp.list().await;
    let tools: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "transport": t.transport.as_str(),
                "endpoint": t.endpoint,
                "capabilities": t.capabilities,
                "server_version": t.server_version,
                "registered_at": t.registered_at,
            })
        })
        .collect();
    Json(serde_json::json!({ "tools": tools }))
}

async fn mcp_register_tool(
    State(state): State<Arc<AppState>>,
    Json(req): Json<McpToolRegisterRequest>,
) -> Json<serde_json::Value> {
    let transport = match crate::mcp::McpTransport::parse(&req.transport) {
        Some(t) => t,
        None => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("unknown transport {} (stdio | http | sse)", req.transport),
            }))
        }
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let replaced = state
        .mcp
        .register(crate::mcp::McpTool {
            name: req.name.clone(),
            transport,
            endpoint: req.endpoint.clone(),
            capabilities: req.capabilities.unwrap_or_default(),
            server_version: req.server_version,
            registered_at: now,
        })
        .await;

    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    registry.activity_hub().record(
        crate::monitor::ActivityKind::Tool,
        crate::monitor::ActivityLevel::Ok,
        "mcp",
        format!(
            "Tool {} registered ({}, {})",
            req.name, req.transport, req.endpoint
        ),
        None,
    );

    Json(serde_json::json!({ "success": true, "replaced": replaced }))
}

#[derive(Debug, Deserialize)]
pub struct McpToolUnregisterRequest {
    pub name: String,
}

async fn mcp_unregister_tool(
    State(state): State<Arc<AppState>>,
    Json(req): Json<McpToolUnregisterRequest>,
) -> Json<serde_json::Value> {
    let removed = state.mcp.unregister(&req.name).await;
    Json(serde_json::json!({ "success": true, "was_registered": removed }))
}
