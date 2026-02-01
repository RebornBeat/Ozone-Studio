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
        .route("/task/get", post(get_task))
        .route("/task/list", post(list_tasks))
        .route("/zsei/query", post(query_zsei))
        .route("/config/get", post(get_config))
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
