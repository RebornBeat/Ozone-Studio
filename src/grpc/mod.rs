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

    Json(HealthResponse {
        healthy: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: state.start_time.elapsed().as_secs(),
        active_tasks: task_mgr.active_count().await as u32,
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

    let mut runtime = state.runtime.write().await;

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
    let active_tasks = task_mgr.active_count().await as u32;

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
            })
            .collect(),
        total,
    })
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
                    _ => model_config.api_endpoint, // already Option<String>
                };
            }
            if let Some(v) = models.get("api_key").and_then(|v| v.as_str()) {
                // you probably want to store it in env or a secure place, but for now:
                model_config.api_key_env = Some(v.to_string()); // or however you store it
            }
            if let Some(v) = models.get("local_model_path").and_then(|v| v.as_str()) {
                model_config.local_model_path = Some(v.to_string());
            }
            if let Some(v) = models.get("local_model_type").and_then(|v| v.as_str()) {
                model_config.local_model_type = Some(v.to_string());
            }

            runtime.config.models = model_config;
        }

        // Handle consciousness updates
        if let Some(consciousness) = updates.get("consciousness") {
            if let Some(enabled) = consciousness.get("enabled").and_then(|v| v.as_bool()) {
                runtime.config.consciousness.enabled = enabled;
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
            // Optional: add more fields if your wizard ever sends them
            // e.g. backend type, model size preference, etc.

            runtime.config.voice = voice_config;
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
    if let Some(model_cfg) = &req.model_config {
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
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => crate::types::Value::String(s),
                    serde_json::Value::Bool(b) => crate::types::Value::Bool(b),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            crate::types::Value::Int(i)
                        } else {
                            crate::types::Value::Float(n.as_f64().unwrap_or(0.0))
                        }
                    }
                    other => crate::types::Value::String(other.to_string()),
                };
                (k, val)
            })
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
                error: None,
                execution_time_ms,
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
            category: info.category.clone(),
            has_ui: info.has_ui,
            is_tab: info.is_tab,
            description: info.description.clone(),
        })
        .collect()
}

/// Get pipeline UI component.js content
async fn get_pipeline_ui_component(
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

    // Try to load component.js from pipeline's ui folder
    let pipelines_path =
        std::env::var("OZONE_PIPELINES_PATH").unwrap_or_else(|_| "./pipelines".to_string());

    let possible_paths = [
        format!(
            "{}/{}/{}/ui/component.js",
            pipelines_path, category, folder_name
        ),
        format!("./pipelines/{}/{}/ui/component.js", category, folder_name),
    ];

    for path in &possible_paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return Json(PipelineUIComponentResponse {
                success: true,
                component_js: Some(content),
                error: None,
            });
        }
    }

    Json(PipelineUIComponentResponse {
        success: false,
        component_js: None,
        error: Some(format!(
            "No UI component found for pipeline {}",
            pipeline_id
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
        r.pipeline_registry.read().await.executor().progress_map()
    };

    let state = Arc::new(AppState {
        runtime,
        start_time: std::time::Instant::now(),
        executor_progress: progress_map,
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
        .route("/zsei/query", post(query_zsei))
        .route("/config/get", post(get_config))
        .route("/config/set", post(set_config))
        .route("/ws", get(websocket_handler))
        .route("/pipeline/progress", post(get_pipeline_progress))
        .route("/pipeline/cancel", post(cancel_pipeline))
        .route("/orchestrate", post(orchestrate))
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
    let was_running = registry.executor().cancel(&req.execution_id).await;

    Json(PipelineCancelResponse {
        success: true,
        was_running,
        error: None,
    })
}
