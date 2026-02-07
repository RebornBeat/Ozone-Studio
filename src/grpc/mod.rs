//! Service layer for UI communication
//!
//! Provides HTTP/WebSocket endpoints for Electron UI.
//! Uses axum for HTTP and WebSocket support.

use crate::types::{OzoneError, OzoneResult};
use crate::types::pipeline::PipelineInput;
use crate::types::zsei::ZSEIQuery;
use crate::OzoneRuntime;
use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
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
    pub task_id: u64,
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
    pub pipeline_id: u64,
    pub pipeline_name: String,
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

// ============================================================================
// v0.4.0 - Pipeline Registry Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineRegistryRequest {
    pub session_token: Option<String>,  // Optional - registry is semi-public
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
        active_tasks: task_mgr.active_count() as u32,
    })
}

async fn request_challenge(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, (StatusCode, String)> {
    let public_key = hex::decode(&req.public_key)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid public key: {}", e)))?;
    
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
        Err(e) => return Json(AuthResponse {
            success: false,
            session_token: None,
            user_id: None,
            device_id: None,
            expires_at: None,
            error: Some(format!("Invalid public key: {}", e)),
        }),
    };
    
    let signature = match hex::decode(&req.signature) {
        Ok(s) => s,
        Err(e) => return Json(AuthResponse {
            success: false,
            session_token: None,
            user_id: None,
            device_id: None,
            expires_at: None,
            error: Some(format!("Invalid signature: {}", e)),
        }),
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
            Ok(_) => {},
            Err(_) => return Json(PipelineResponse {
                success: false,
                task_id: 0,
                output: None,
                error: Some("Invalid session".into()),
            }),
        }
    }
    
    let input: crate::types::pipeline::PipelineInput = match serde_json::from_value(req.input) {
        Ok(i) => i,
        Err(e) => return Json(PipelineResponse {
            success: false,
            task_id: 0,
            output: None,
            error: Some(format!("Invalid input: {}", e)),
        }),
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
            task_id: 0,
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
    
    match task_mgr.get_task(req.task_id).await {
        Some(task) => Json(Some(TaskInfo {
            task_id: task.task_id,
            pipeline_id: task.pipeline_used,
            pipeline_name: format!("Pipeline #{}", task.pipeline_used),
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
    
    let tasks = task_mgr.list_tasks(status_filter, limit, offset).await;
    let total = tasks.len() as u32;
    
    Json(TaskListResponse {
        tasks: tasks.into_iter().map(|t| TaskInfo {
            task_id: t.task_id,
            pipeline_id: t.pipeline_used,
            pipeline_name: format!("Pipeline #{}", t.pipeline_used),
            status: format!("{:?}", t.status),
            progress: t.progress,
            created_at: t.created_at,
            started_at: t.started_at,
            completed_at: t.completed_at,
            error: t.error.map(|e| format!("{:?}", e)),
        }).collect(),
        total,
    })
}

async fn query_zsei(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ZseiQueryRequest>,
) -> Json<ZseiResponse> {
    let query: ZSEIQuery = match serde_json::from_value(req.query) {
        Ok(q) => q,
        Err(e) => return Json(ZseiResponse {
            success: false,
            result: None,
            error: Some(format!("Invalid query: {}", e)),
        }),
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
        Some(s) => return Json(ConfigResponse {
            success: false,
            config: None,
            error: Some(format!("Unknown config section: {}", s)),
        }),
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
    let config_path = std::env::var("OZONE_CONFIG")
        .unwrap_or_else(|_| "config.toml".to_string());
    
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
        
        // Handle model updates
        if let Some(models) = updates.get("models") {
            if let Ok(model_config) = serde_json::from_value(models.clone()) {
                runtime.config.models = model_config;
            }
        }
        
        // Handle consciousness updates
        if let Some(consciousness) = updates.get("consciousness") {
            if let Some(enabled) = consciousness.get("enabled").and_then(|v| v.as_bool()) {
                runtime.config.consciousness.enabled = enabled;
            }
        }
        
        // Handle voice updates
        if let Some(voice) = updates.get("voice") {
            if let Ok(voice_config) = serde_json::from_value(voice.clone()) {
                runtime.config.voice = voice_config;
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
        Ok(config_str) => {
            match std::fs::write(&config_path, &config_str) {
                Ok(_) => Json(ConfigSetResponse {
                    success: true,
                    error: None,
                }),
                Err(e) => Json(ConfigSetResponse {
                    success: false,
                    error: Some(format!("Failed to write config: {}", e)),
                }),
            }
        }
        Err(e) => Json(ConfigSetResponse {
            success: false,
            error: Some(format!("Failed to serialize config: {}", e)),
        }),
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
/// MUST stay in sync with src/pipeline/registry.rs PIPELINE_INFO
fn build_pipeline_registry() -> Vec<PipelineRegistryEntry> {
    vec![
        // Core System Pipelines (1-38)
        PipelineRegistryEntry { id: 1, name: "Auth".into(), folder_name: "auth".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Ed25519 challenge-response authentication".into() },
        PipelineRegistryEntry { id: 2, name: "ThemeLoader".into(), folder_name: "theme_loader".into(), category: "core".into(), has_ui: false, is_tab: false, description: "UI theme management and pipeline UI loading".into() },
        PipelineRegistryEntry { id: 3, name: "ZSEIQuery".into(), folder_name: "zsei_query".into(), category: "core".into(), has_ui: false, is_tab: false, description: "ZSEI container queries".into() },
        PipelineRegistryEntry { id: 4, name: "ZSEIWrite".into(), folder_name: "zsei_write".into(), category: "core".into(), has_ui: false, is_tab: false, description: "ZSEI container writes".into() },
        PipelineRegistryEntry { id: 5, name: "TaskManager".into(), folder_name: "task_manager".into(), category: "core".into(), has_ui: true, is_tab: true, description: "Task lifecycle management".into() },
        PipelineRegistryEntry { id: 6, name: "WorkspaceTab".into(), folder_name: "workspace_tab".into(), category: "core".into(), has_ui: true, is_tab: true, description: "Workspace and project management".into() },
        PipelineRegistryEntry { id: 7, name: "LibraryTab".into(), folder_name: "library_tab".into(), category: "core".into(), has_ui: true, is_tab: true, description: "Pipeline/methodology/blueprint browser".into() },
        PipelineRegistryEntry { id: 8, name: "SettingsTab".into(), folder_name: "settings_tab".into(), category: "core".into(), has_ui: true, is_tab: true, description: "Application settings".into() },
        PipelineRegistryEntry { id: 9, name: "Prompt".into(), folder_name: "prompt".into(), category: "core".into(), has_ui: false, is_tab: false, description: "LLM interface (API/GGUF/ONNX)".into() },
        PipelineRegistryEntry { id: 10, name: "Voice".into(), folder_name: "voice".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Voice input/output".into() },
        PipelineRegistryEntry { id: 11, name: "MethodologyFetch".into(), folder_name: "methodology_fetch".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Methodology retrieval".into() },
        PipelineRegistryEntry { id: 12, name: "MethodologyCreate".into(), folder_name: "methodology_create".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Methodology creation".into() },
        PipelineRegistryEntry { id: 13, name: "BlueprintSearch".into(), folder_name: "blueprint_search".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Blueprint search".into() },
        PipelineRegistryEntry { id: 14, name: "BlueprintCreate".into(), folder_name: "blueprint_create".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Blueprint creation".into() },
        PipelineRegistryEntry { id: 15, name: "PipelineCreation".into(), folder_name: "pipeline_creation".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Pipeline bootstrapping".into() },
        PipelineRegistryEntry { id: 16, name: "ZeroShotSimulation".into(), folder_name: "zero_shot_simulation".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Zero-shot task validation".into() },
        PipelineRegistryEntry { id: 17, name: "TraversalML".into(), folder_name: "traversal_ml".into(), category: "core".into(), has_ui: false, is_tab: false, description: "ML-guided ZSEI traversal".into() },
        PipelineRegistryEntry { id: 18, name: "CodeAnalysis".into(), folder_name: "code_analysis".into(), category: "core".into(), has_ui: true, is_tab: false, description: "Code analysis with LLM".into() },
        PipelineRegistryEntry { id: 19, name: "PackageContext".into(), folder_name: "package_context".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Package context extraction".into() },
        PipelineRegistryEntry { id: 20, name: "TextAnalysis".into(), folder_name: "text_analysis".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Text analysis with LLM".into() },
        PipelineRegistryEntry { id: 21, name: "ContextAggregation".into(), folder_name: "context_aggregation".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Context aggregation for prompts".into() },
        PipelineRegistryEntry { id: 22, name: "GraphVisualization".into(), folder_name: "graph_visualization".into(), category: "core".into(), has_ui: true, is_tab: false, description: "Dependency graph visualization".into() },
        PipelineRegistryEntry { id: 23, name: "TaskRecommendation".into(), folder_name: "task_recommendation".into(), category: "core".into(), has_ui: false, is_tab: false, description: "ML-based task suggestions".into() },
        PipelineRegistryEntry { id: 24, name: "Reordering".into(), folder_name: "reordering".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Blueprint step reordering".into() },
        PipelineRegistryEntry { id: 25, name: "BrowserNavigation".into(), folder_name: "browser_navigation".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Web browser automation".into() },
        PipelineRegistryEntry { id: 26, name: "IntegrityCheck".into(), folder_name: "integrity_check".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Blake3 hash verification".into() },
        PipelineRegistryEntry { id: 27, name: "Consensus".into(), folder_name: "consensus".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Distributed consensus".into() },
        PipelineRegistryEntry { id: 28, name: "ExternalReference".into(), folder_name: "external_reference".into(), category: "core".into(), has_ui: false, is_tab: false, description: "External reference tracking".into() },
        PipelineRegistryEntry { id: 29, name: "PackageRelationship".into(), folder_name: "package_relationship".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Package dependency graphs".into() },
        PipelineRegistryEntry { id: 30, name: "FileLink".into(), folder_name: "file_link".into(), category: "core".into(), has_ui: true, is_tab: false, description: "File reference linking".into() },
        PipelineRegistryEntry { id: 31, name: "URLLink".into(), folder_name: "url_link".into(), category: "core".into(), has_ui: true, is_tab: false, description: "URL reference linking".into() },
        PipelineRegistryEntry { id: 32, name: "PackageLink".into(), folder_name: "package_link".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Package reference linking".into() },
        PipelineRegistryEntry { id: 33, name: "Sync".into(), folder_name: "sync".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Distributed sync".into() },
        PipelineRegistryEntry { id: 34, name: "DeviceRegister".into(), folder_name: "device_register".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Device registration".into() },
        PipelineRegistryEntry { id: 35, name: "HomeReturn".into(), folder_name: "home_return".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Home navigation".into() },
        PipelineRegistryEntry { id: 36, name: "TaskViewer".into(), folder_name: "task_viewer".into(), category: "core".into(), has_ui: false, is_tab: false, description: "DEPRECATED - merged into TaskManager".into() },
        PipelineRegistryEntry { id: 37, name: "LogViewer".into(), folder_name: "log_viewer".into(), category: "core".into(), has_ui: true, is_tab: false, description: "Log viewer UI".into() },
        PipelineRegistryEntry { id: 38, name: "DeviceStatus".into(), folder_name: "device_status".into(), category: "core".into(), has_ui: false, is_tab: false, description: "Device monitoring".into() },
        // Consciousness Pipelines (39-54)
        PipelineRegistryEntry { id: 39, name: "DecisionGate".into(), folder_name: "consciousness_decision_gate".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Ethical decision gating".into() },
        PipelineRegistryEntry { id: 40, name: "ExperienceCategorization".into(), folder_name: "experience_categorization".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Experience categorization".into() },
        PipelineRegistryEntry { id: 41, name: "CoreMemoryFormation".into(), folder_name: "core_memory_formation".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Core memory formation".into() },
        PipelineRegistryEntry { id: 42, name: "ExperienceRetrieval".into(), folder_name: "experience_retrieval".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Experience retrieval".into() },
        PipelineRegistryEntry { id: 43, name: "EmotionalBaselineUpdate".into(), folder_name: "emotional_baseline_update".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Emotional baseline calibration".into() },
        PipelineRegistryEntry { id: 44, name: "ILoop".into(), folder_name: "i_loop".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "I-Loop self-reflection".into() },
        PipelineRegistryEntry { id: 45, name: "InternalLanguage".into(), folder_name: "internal_language".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Internal language processing".into() },
        PipelineRegistryEntry { id: 46, name: "NarrativeConstruction".into(), folder_name: "narrative_construction".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Narrative construction".into() },
        PipelineRegistryEntry { id: 47, name: "RelationshipDevelopment".into(), folder_name: "relationship_development".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Relationship development".into() },
        PipelineRegistryEntry { id: 48, name: "EthicalAssessment".into(), folder_name: "ethical_assessment".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Ethical assessment".into() },
        PipelineRegistryEntry { id: 49, name: "EthicalSimulation".into(), folder_name: "ethical_simulation".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Ethical simulation".into() },
        PipelineRegistryEntry { id: 50, name: "PlaybackReview".into(), folder_name: "playback_review".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Experience playback review".into() },
        PipelineRegistryEntry { id: 51, name: "UserFeedback".into(), folder_name: "user_feedback".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "User feedback processing".into() },
        PipelineRegistryEntry { id: 52, name: "CollectiveConsciousness".into(), folder_name: "collective_consciousness".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Collective consciousness sync".into() },
        PipelineRegistryEntry { id: 53, name: "VoiceIdentity".into(), folder_name: "voice_identity".into(), category: "consciousness".into(), has_ui: false, is_tab: false, description: "Voice identity development".into() },
        PipelineRegistryEntry { id: 54, name: "MetaPortionConsciousness".into(), folder_name: "meta_portion_consciousness".into(), category: "consciousness".into(), has_ui: true, is_tab: false, description: "Meta Portion consciousness UI".into() },
    ]
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
    let pipelines_path = std::env::var("OZONE_PIPELINES_PATH")
        .unwrap_or_else(|_| "./pipelines".to_string());
    
    let possible_paths = [
        format!("{}/{}/{}/ui/component.js", pipelines_path, category, folder_name),
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
        error: Some(format!("No UI component found for pipeline {}", pipeline_id)),
    })
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: Arc<AppState>) {
    while let Some(msg) = socket.recv().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(request) = serde_json::from_str::<serde_json::Value>(&text) {
                let response = handle_ws_message(request, &state).await;
                let _ = socket.send(Message::Text(
                    serde_json::to_string(&response).unwrap_or_default()
                )).await;
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
    
    let state = Arc::new(AppState {
        runtime,
        start_time: std::time::Instant::now(),
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
        .layer(cors)
        .with_state(state);
    
    tracing::info!("Starting HTTP server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await
        .map_err(|e| OzoneError::ServerError(format!("Failed to bind: {}", e)))?;
    
    axum::serve(listener, app).await
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
