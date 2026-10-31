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
    /// MCP call metering per agent/day — the MCP usage budget (src/mcp.rs).
    pub mcp_usage: Arc<crate::mcp::UsageLedger>,
    /// Real process start/stop/crash-restart for registered MCP tools
    /// (see src/mcp_supervisor.rs) — registration alone never started
    /// anything; this is the part that actually does.
    pub mcp_supervisor: Arc<crate::mcp_supervisor::McpProcessSupervisor>,
    /// Real ZSEI data dir, passed to tool processes that need it
    /// (git-mcp's `OZONE_ZSEI_DATA_DIR`) — same value used for the MCP
    /// registry's own persistence path.
    pub zsei_data_dir: String,
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
    /// Coordination metadata (from inputs for /task/create tasks): what the
    /// task IS, who it's routed to, who created it — so the API record is
    /// self-describing without needing out-of-band context.
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
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
    /// Which project this task is scoped to, if any — real field on
    /// TaskData (`project_id: Option<u64>`) that was previously silently
    /// dropped at every TaskInfo construction site, so the UI never had a
    /// way to show or navigate to a task's owning project. `#[serde(default)]`
    /// so older persisted records without this field still deserialize.
    #[serde(default)]
    pub project_id: Option<u64>,
    /// Final answer text for a completed task — the exact same text
    /// OrchestrateResponse.response carries on the fast synchronous path,
    /// read from the one place the orchestrator itself already writes it on
    /// completion: TaskData.outputs.response (set by
    /// TaskManager::complete_task, written from orchestrator/stages.rs's
    /// stage_9_result_collection `state.final_response`). Was previously
    /// computed by the orchestrator but silently dropped at this API
    /// boundary — get_task never copied `task.outputs` into the response at
    /// all, so a client that had to recover a slow-but-successful task via
    /// /task/get had no way to read back the real answer. None until the
    /// task reaches a terminal status, or if it failed before producing one.
    #[serde(default)]
    pub response: Option<String>,
    /// Same derivation the orchestrator itself uses for its own top-level
    /// OrchestrateResponse.model_used (orchestrator/mod.rs:
    /// `state.step_results.last()...model_used`): the last executed step's
    /// model, read from the already-populated per-step `steps[].model_used`
    /// above — not a new data source, just exposed at the top level too.
    #[serde(default)]
    pub model_used: Option<String>,
    /// Same split OrchestrateResponse applies via `attempt_trail_and_refusals`
    /// below, run here over `thinking_log` above — so a recovered task shows
    /// the identical fallback-walk trail/refusal lists the fast path does.
    #[serde(default)]
    pub attempt_trail: Vec<String>,
    #[serde(default)]
    pub refusals: Vec<String>,
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
    /// This instance's own real libp2p PeerId (2026-10-21, additive — was
    /// computed by `get_status()` already but never surfaced over HTTP,
    /// so the only way to learn a remote instance's peer id was to read
    /// its log by eye, "Local peer ID: ..." — confirmed by grep, no route
    /// exposed it). Lets a tool that's tunneled into a remote instance's
    /// loopback-only API discover its peer id and build a real multiaddr
    /// without any log-scraping step.
    pub peer_id: Option<String>,
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
    /// Per-request consciousness decision gate. Server-side default TRUE:
    /// with [consciousness].enabled in config, the gate participates in
    /// every orchestration unless a caller explicitly opts out (found
    /// hardcoded false in the CLI — the gate never ran).
    #[serde(default = "default_consciousness_enabled")]
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

fn default_consciousness_enabled() -> bool {
    true
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
    /// One line per model attempt the fallback walk made, in order (model,
    /// outcome, cause, latency, next step), plus the walk's final error when
    /// it failed. Empty when no walk ran. Read from the thinking log.
    #[serde(default)]
    pub attempt_trail: Vec<String>,
    /// Paid or unknown model overrides the orchestrator refused, each with
    /// its reason. The refused step ran on the free chain instead. Empty when
    /// none. Read from the thinking log.
    #[serde(default)]
    pub refusals: Vec<String>,
}

/// Split the thinking log into the attempt trail and the refusals. Both are
/// ordinary thinking-log entries, so the response shows them without a new
/// channel: `walk_trail` and `walk_last_attempt` (the walk's own lines) feed
/// the trail in order; `model_override_refused:*` feeds the refusals.
fn attempt_trail_and_refusals(thinking_log: &[serde_json::Value]) -> (Vec<String>, Vec<String>) {
    let mut trail = Vec::new();
    let mut refusals = Vec::new();
    for entry in thinking_log {
        let stage = entry.get("stage").and_then(|s| s.as_str()).unwrap_or("");
        let raw = entry
            .get("raw_response")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        if stage == "walk_trail" || stage == "walk_last_attempt" {
            trail.push(raw);
        } else if stage.starts_with("model_override_refused") {
            refusals.push(raw);
        }
    }
    (trail, refusals)
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
        peer_id: net_status.peer_id,
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


/// Local-process internal token (2026-10-27). With OZONE_REQUIRE_SESSION=1 the
/// strict gate 401'd every local helper that has no paired session — notably
/// the shared-context MCP server's `/context/mirror` and `/task/create`
/// (coordination events silently stopped mirroring). The host now keeps a
/// random token in `<data_dir>/.internal_token` (created 0600, REUSED across
/// restarts so long-lived local agents keep working) and the gate accepts it
/// as `x-oz-internal`. It is a file-permission boundary: a browser page cannot
/// read it, a same-user local process can — the same trust a paired session
/// would give that process, without requiring a pairing it cannot perform.
static INTERNAL_TOKEN: std::sync::OnceLock<String> = std::sync::OnceLock::new();

fn init_internal_token(data_dir: &str) {
    let path = std::path::Path::new(data_dir).join(".internal_token");
    let existing = std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() >= 32);
    let token = match existing {
        Some(t) => t,
        None => {
            use rand::RngCore;
            let mut bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut bytes);
            let token = hex::encode(bytes);
            let _ = std::fs::create_dir_all(data_dir);
            #[cfg(unix)]
            {
                use std::io::Write;
                use std::os::unix::fs::OpenOptionsExt;
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(&path)
                {
                    Ok(mut f) => {
                        let _ = f.write_all(token.as_bytes());
                    }
                    Err(e) => tracing::warn!(path = %path.display(), error = %e, "could not write internal token file"),
                }
            }
            #[cfg(not(unix))]
            {
                let _ = std::fs::write(&path, &token);
            }
            token
        }
    };
    let _ = INTERNAL_TOKEN.set(token);
}

/// STRICT SESSION MODE (perimeter review 2026-10-23): with
/// `OZONE_REQUIRE_SESSION=1`, the mutating route families (`/config/`,
/// `/mcp/tools`, `/network/`, `/task/`, `/context/mirror`,
/// `/consciousness/reviews/resolve`, `/pipelines/register`) require a valid
/// `Authorization: Bearer <token>` or `x-oz-session` header. Default OFF
/// preserves the documented loopback posture; the operator flips it when
/// their clients send headers. The routes that validate body tokens
/// themselves (/orchestrate, /pipeline/execute) already do so and are not
/// double-gated here.
async fn session_gate(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    const PROTECTED: &[&str] = &[
        "/config/set",
        "/mcp/tools",
        "/network/",
        "/task/",
        "/context/mirror",
        "/consciousness/reviews/resolve",
        "/capture/ingest_image",
        "/capture/ingest_uidocument",
        "/pipelines/register",
    ];
    // FIRST-RUN EXEMPTION (2026-10-27): while setup is incomplete the strict
    // gate stands down entirely — the setup wizard must be able to write
    // config BEFORE any pairing/session exists (previously the gate 401'd
    // the wizard's own confirm, which left user_setup_complete never
    // persisted and re-showed the wizard on every launch). Completing setup
    // arms the gate.
    let first_run = {
        let rt = state.runtime.read().await;
        !(rt.config.general.setup_complete && rt.config.general.user_setup_complete)
    };
    if std::env::var("OZONE_REQUIRE_SESSION").as_deref() == Ok("1")
        && !first_run
        && PROTECTED.iter().any(|p| req.uri().path().starts_with(p))
    {
        // Read headers into OWNED strings BEFORE any await: a closure holding
        // &Request across an await makes the future !Send (the body is not
        // Sync) — exactly the FromFn Service failure the first compile hit.
        let bearer = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer ").map(|t| t.to_string()));
        let alt = req
            .headers()
            .get("x-oz-session")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let internal = req
            .headers()
            .get("x-oz-internal")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let internal_ok = !internal.is_empty()
            && INTERNAL_TOKEN.get().map_or(false, |t| *t == internal);
        let token = bearer.or(alt).unwrap_or_default();
        let ok = internal_ok || {
            let runtime = state.runtime.read().await;
            let auth = runtime.auth.read().await;
            auth.validate_session(&hex::decode(&token).unwrap_or_default())
                .await
                .is_ok()
        };
        if !ok {
            return axum::http::StatusCode::UNAUTHORIZED.into_response();
        }
    }
    next.run(req).await
}

/// GET /consciousness/reviews/pending — the operator's visibility half of the
/// ReviewPending contract (the resolve half is POST /consciousness/reviews/
/// resolve). "Needs a human" finally has a door: this lists every final S10
/// row still sitting in review_pending / review-failed, newest first.
async fn list_pending_reviews_route(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let data_dir = runtime.config.general.data_dir.clone();
    let rows = crate::orchestrator::decision_review::list_pending_reviews(&data_dir);
    let count = rows.len();
    Json(serde_json::json!({ "rows": rows, "count": count }))
}

#[derive(Debug, serde::Deserialize)]
pub struct ResolveReviewRequest {
    pub review_id: String,
    /// "Proceed" or "Decline" (case-insensitive; anything else is refused).
    pub decision: String,
    #[serde(default)]
    pub note: String,
}

/// POST /consciousness/reviews/resolve — record the operator's resolution of
/// a pending review. The DecisionReviewExecutor consults the resolutions
/// store before judging, so re-submitting the same request returns the
/// operator's decision instead of halting again.
async fn resolve_review_route(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResolveReviewRequest>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let data_dir = runtime.config.general.data_dir.clone();
    // IDEMPOTENT (UNCOMPILED pending the build gate): a second submit of the
    // same decision for an already-resolved review reuses the recorded
    // resolution instead of appending a duplicate row; proposals' execution is
    // itself idempotent (proposals::execute_resolved returns the recorded
    // outcome with already_resolved:true).
    let prior = crate::orchestrator::decision_review::latest_resolution(&data_dir, &req.review_id)
        .filter(|r| {
            r.get("decision").and_then(|d| d.as_str()).map(|d| d.eq_ignore_ascii_case(req.decision.trim())) == Some(true)
        });
    let resolved = match prior {
        Some(r) => Ok(r),
        None => crate::orchestrator::decision_review::resolve_review(
            &data_dir,
            &req.review_id,
            &req.decision,
            &req.note,
        ),
    };
    match resolved {
        Ok(record) => {
            // ASSISTANT-PROPOSAL EXECUTION (the acting chain's completion):
            // a proposal is an S10 review row; the operator's "Proceed" IS
            // the approval — execute the proposed action through the same
            // metered, jurisdiction-gated call_global door every MCP call
            // takes, and append the honest executed/failed row. "Decline"
            // needs nothing extra — the resolution row IS the record.
            let execution = if record["decision"] == "Proceed" {
                let kind = crate::orchestrator::decision_review::review_row_kind(
                    &data_dir,
                    &req.review_id,
                );
                if kind.as_deref() == Some(crate::consciousness::proposals::PROPOSAL_REVIEW_KIND) {
                    // Always HTTP 200 with an honest body: the host drops the
                    // body of non-2xx answers, so Err becomes success:false.
                    Some(
                        match crate::consciousness::proposals::execute_resolved(
                            &data_dir,
                            &req.review_id,
                        )
                        .await
                        {
                            Ok(v) => v,
                            Err(e) => serde_json::json!({ "success": false, "error": e }),
                        },
                    )
                } else {
                    None
                }
            } else {
                None
            };
            Json(serde_json::json!({
                "success": true,
                "resolution": record,
                "proposal_execution": execution,
            }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "error": e })),
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

    // CONSCIOUSNESS GATE (operator security directive): consciousness-category
    // pipelines are internal meta — never callable through the external
    // /pipeline/execute contract, whatever the session's role.
    if crate::pipeline::registry::category_of(req.pipeline_id) == Some("consciousness") {
        return Json(PipelineResponse {
            success: false,
            task_id: Some(0),
            output: None,
            error: Some(format!(
                "pipeline {} is consciousness-category: internal meta, not callable via external contracts",
                req.pipeline_id
            )),
        });
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
        Some(task) => {
            // Computed from borrows BEFORE the fields below move task.outputs
            // / task.steps / task.thinking_log out of `task`.
            let response = task
                .outputs
                .as_ref()
                .and_then(|o| o.get("response"))
                .and_then(|r| r.as_str())
                .map(String::from);
            let model_used = task.steps.iter().rev().find_map(|s| s.model_used.clone());
            let (attempt_trail, refusals) = attempt_trail_and_refusals(&task.thinking_log);

            Json(Some(TaskInfo {
                task_id: task.task_id,
                blueprint_id: task.blueprint_id,
                blueprint_name: format!("Blueprint #{}", task.blueprint_id.unwrap_or(0)),
                // task.status is already a String (src/task/mod.rs) — Debug-
                // formatting it here wrapped every real status value in literal
                // quote characters (e.g. the JSON value was "\"interrupted\""
                // instead of "interrupted"), confirmed live by the UI fork
                // working around it defensively. Fixed at the source instead.
                status: task.status.clone(),
                progress: task.progress,
                created_at: task.created_at,
                started_at: task.started_at,
                completed_at: task.completed_at,
                error: task.error.map(|e| format!("{:?}", e)),
                description: task
                    .inputs
                    .as_ref()
                    .and_then(|i| i.get("prompt"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
                assignee: task
                    .inputs
                    .as_ref()
                    .and_then(|i| i.get("assignee"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
                created_by: task
                    .inputs
                    .as_ref()
                    .and_then(|i| i.get("source"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
                steps: task.steps,
                thinking_log: task.thinking_log,
                amt_summary: task.amt_summary,
                project_id: task.project_id,
                response,
                model_used,
                attempt_trail,
                refusals,
            }))
        }
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
            .map(|t| {
                // Coordination metadata surfaces straight from inputs so the
                // task record is self-describing over the API.
                let inputs = t.inputs.as_ref().cloned().unwrap_or(serde_json::Value::Null);
                let coord = |key: &str| {
                    inputs
                        .get(key)
                        .and_then(|v| v.as_str())
                        .map(String::from)
                };
                let description = coord("prompt");
                let assignee = coord("assignee");
                let created_by = coord("source").or_else(|| coord("created_by"));
                TaskInfo {
                    task_id: t.task_id,
                    blueprint_id: t.blueprint_id,
                    blueprint_name: format!("Blueprint #{}", t.blueprint_id.unwrap_or(0)),
                    status: t.status.clone(),
                    progress: t.progress,
                    created_at: t.created_at,
                    started_at: t.started_at,
                    completed_at: t.completed_at,
                    error: t.error.map(|e| format!("{:?}", e)),
                    description,
                    assignee,
                    created_by,
                    steps: Vec::new(),
                    thinking_log: Vec::new(),
                    amt_summary: None,
                    project_id: t.project_id,
                    // Same lightweight-list-view convention as steps/
                    // thinking_log/amt_summary above: populated on get_task,
                    // left empty here to keep the list view cheap.
                    response: None,
                    model_used: None,
                    attempt_trail: Vec::new(),
                    refusals: Vec::new(),
                }
            })
            .collect(),
        total,
    })
}

#[derive(Debug, Deserialize)]
pub struct GlobalOrderQuery {
    pub workspace_id: Option<u64>,
    pub project_id: Option<u64>,
    /// Universal Order Stage 2 (guide §2): kind filter ("todo", "meeting",
    /// ...) and time view ("overdue" | "today" | "week" | "upcoming" |
    /// "someday"). Time buckets are COMPUTED from due_at at read — never
    /// stored (guide §2 rules).
    pub kind: Option<String>,
    pub due: Option<String>,
}

/// GET /assistant/feed — the Personal Assistant's derived feed
/// (docs/PERSONAL_ASSISTANT_GUIDE.md §4.2). The FREE half of the
/// assistant: findings are COMPUTED at read by the SAME
/// `consciousness::assistant::compute_findings` the check-up loop uses —
/// one canonical implementation, no second store, no LLM cost on this
/// route. Scope: "global" (default) | "ws:<id>" | "proj:<id>" (the same
/// scope-keyword convention emit_task_ripple produces).
#[derive(Debug, Deserialize)]
pub struct AssistantFeedQuery {
    pub scope: Option<String>,
}

async fn get_assistant_feed(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<AssistantFeedQuery>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let task_mgr = runtime.task_manager.read().await;
    let all = task_mgr.list_tasks(None, None, 10_000, 0).await;
    drop(task_mgr);
    drop(runtime);

    // Scope keyword parse — mirror emit_task_ripple's "ws:"/"proj:" forms.
    let mut ws_filter: Option<u64> = None;
    let mut proj_filter: Option<u64> = None;
    let scope_label = q.scope.clone().unwrap_or_else(|| "global".to_string());
    if let Some(w) = scope_label.strip_prefix("ws:") {
        ws_filter = w.parse::<u64>().ok();
    } else if let Some(p) = scope_label.strip_prefix("proj:") {
        proj_filter = p.parse::<u64>().ok();
    }

    let filtered: Vec<crate::task::TaskData> = all
        .into_iter()
        .filter(|t| ws_filter.map_or(true, |w| t.workspace_id == Some(w)))
        .filter(|t| proj_filter.map_or(true, |p| t.project_id == Some(p)))
        .collect();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let findings = crate::consciousness::assistant::compute_findings(&filtered, now);

    let mut counts: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let entries: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            *counts.entry(f.class.to_string()).or_insert(0) += 1;
            f.to_json()
        })
        .collect();

    Json(serde_json::json!({
        "scope": scope_label,
        "generated_at": now,
        "counts": counts,
        "findings": entries,
    }))
}

/// GET /order/global — the universal native task order (guide §9, Phase
/// 2b, operator architecture). A DERIVED VIEW, never a copy: joins the
/// real task store (every task + its real steps) and groups by native
/// state — live / paused / queued / interrupted / done — ordered oldest
/// first within each group (sequence = creation order). Blueprint steps
/// ARE the checklist; this route is "what's live, what's paused, what's
/// next" across ALL of them at once, scoped filterable by workspace and
/// project. Ripple-invalidated consumers just re-fetch; nothing here
/// caches.
async fn get_global_order(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<GlobalOrderQuery>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let task_mgr = runtime.task_manager.read().await;
    let all = task_mgr.list_tasks(None, None, 10_000, 0).await;

    fn state_of(status: &str) -> &'static str {
        match status {
            "running" => "live",
            "paused" => "paused",
            "queued" => "queued",
            "interrupted" => "interrupted",
            "completed" | "failed" | "cancelled" => "done",
            _ => "other",
        }
    }

    let mut groups: std::collections::HashMap<&'static str, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    let mut counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let day = 86_400u64;
    let due_bucket = |due: Option<u64>| -> &'static str {
        match due {
            Some(d) if d < now.saturating_sub(day) => "overdue",
            Some(d) if d <= now + day => "today",
            Some(d) if d <= now + 7 * day => "week",
            _ => "upcoming",
        }
    };

    for t in all {
        if let Some(w) = q.workspace_id {
            if t.workspace_id != Some(w) {
                continue;
            }
        }
        if let Some(p) = q.project_id {
            if t.project_id != Some(p) {
                continue;
            }
        }
        // Stage 2 filters: kind + computed due bucket
        let kind = t
            .inputs
            .as_ref()
            .and_then(|i| i.get("kind"))
            .and_then(|k| k.as_str())
            .unwrap_or("todo")
            .to_string();
        if let Some(want) = &q.kind {
            if &kind != want {
                continue;
            }
        }
        let due_at = t.due_at;
        if let Some(due) = q.due.as_deref() {
            let bucket = match due_at {
                Some(_) => due_bucket(due_at),
                None => "someday",
            };
            if bucket != due {
                continue;
            }
        }
        *counts.entry(state_of(&t.status).to_string()).or_insert(0) += 1;

        let inputs = t.inputs.as_ref().cloned().unwrap_or(serde_json::Value::Null);
        let coord = |key: &str| {
            inputs.get(key).and_then(|v| v.as_str()).map(String::from)
        };
        // Next step = the FIRST step not yet finished, in blueprint order.
        let next_step = t
            .steps
            .iter()
            .find(|s| s.status != "completed" && s.status != "failed")
            .map(|s| serde_json::json!({
                "step_index": s.step_index,
                "action": s.action,
                "status": s.status,
            }));
        let done_steps = t
            .steps
            .iter()
            .filter(|s| s.status == "completed" || s.status == "failed")
            .count();

        let entry = serde_json::json!({
            "task_id": t.task_id,
            "name": coord("name")
                .or_else(|| coord("prompt").map(|p| p.chars().take(80).collect::<String>()))
                .unwrap_or_else(|| format!("Task {}", t.task_id)),
            "source": coord("source"),
            "assignee": coord("assignee"),
            "kind": coord("kind").unwrap_or_else(|| "todo".to_string()),
            "due_at": t.due_at,
            "remind_at": t.remind_at,
            "meeting_url": coord("meeting_url"),
            "note_body": coord("note"),
            "blueprint_id": t.blueprint_id,
            "workspace_id": t.workspace_id,
            "project_id": t.project_id,
            "status": t.status,
            "progress": t.progress,
            "created_at": t.created_at,
            "steps_total": t.steps.len(),
            "steps_done": done_steps,
            "next_step": next_step,
        });

        groups.entry(state_of(&t.status)).or_default().push(entry);
    }

    // Order key within every group (stable sort, so equal keys keep input
    // order): dated before undated; dated items by due_at ascending
    // (overdue naturally first, then soonest); then created_at (creation
    // order is the final sequence). The stored task record carries no
    // priority, so priority only orders the live task QUEUE (task/mod.rs),
    // not this view; the `priority_rank` slot below is 0 until a response
    // field exists for it.
    for list in groups.values_mut() {
        list.sort_by_key(|e| {
            let due = e["due_at"].as_u64();
            (
                due.is_none(),
                due.unwrap_or(0),
                std::cmp::Reverse(e["priority_rank"].as_u64().unwrap_or(0)),
                e["created_at"].as_u64().unwrap_or(0),
            )
        });
    }

    Json(serde_json::json!({
        "live": groups.get("live").cloned().unwrap_or_default(),
        "paused": groups.get("paused").cloned().unwrap_or_default(),
        "queued": groups.get("queued").cloned().unwrap_or_default(),
        "interrupted": groups.get("interrupted").cloned().unwrap_or_default(),
        "done": groups.get("done").cloned().unwrap_or_default(),
        "other": groups.get("other").cloned().unwrap_or_default(),
        "counts": counts,
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsciousnessReviewPassResponse {
    pub insight_container_ids: Vec<u64>,
}

/// Manual trigger for the consciousness review pass (TOP_DOWN_REVIEW_
/// GUIDE.md §8, item 3) — reads the real decision-review capture store +
/// recent tasks, traverses the graph for context, and persists any
/// genuine finding as a real container under CONSCIOUSNESS_METACOGNITION_
/// ROOT_ID. Manually triggered for now (not a background loop) until a
/// real cadence decision is made; empty `insight_container_ids` is a
/// correct, honest outcome when nothing genuinely warranted flagging.
async fn consciousness_review_pass(
    State(state): State<Arc<AppState>>,
) -> Json<ConsciousnessReviewPassResponse> {
    let runtime = state.runtime.read().await;
    let store: Arc<dyn crate::orchestrator::StoreAccess> =
        Arc::new(crate::orchestrator::ZseiStoreAdapter {
            zsei: runtime.zsei.clone(),
        });
    let data_dir = runtime.config.general.data_dir.clone();
    let task_manager = runtime.task_manager.read().await;
    let created = crate::consciousness::review::run_review_pass(store, &task_manager, &data_dir).await;
    Json(ConsciousnessReviewPassResponse {
        insight_container_ids: created,
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
    // ENTRY GATE: a rerun is an external dispatch too — trace root (or
    // joined child) + one external-pipeline span row; dispatch unchanged.
    let result =
        crate::orchestrator::entry_gate::gated(pipeline_id, adapter.execute(pipeline_id, exec_input))
            .await;

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
                    response_text.as_ref().map(|r| r.chars().take(200).collect::<String>()),
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

// ZSEI variants that mutate the store. Pipelines and the UI write through this
// route without session tokens today, so writes are AUDITED by default; set
// OZONE_ZSEI_REQUIRE_SESSION=1 to refuse writes that lack a valid session_token.
const ZSEI_WRITE_VARIANTS: &[&str] = &[
    "CreateContainer",
    "UpdateContainer",
    "DeleteContainer",
    "LinkFile",
    "LinkURL",
    "LinkPackage",
    "UnlinkFile",
    "Rollback",
];

fn append_zsei_write_audit(data_dir: &str, row: &serde_json::Value) {
    use std::io::Write;
    let dir = format!("{}/model_calls", data_dir);
    let outcome = std::fs::create_dir_all(&dir).and_then(|_| {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{}/zsei_writes.jsonl", dir))?;
        writeln!(f, "{}", row)
    });
    if let Err(e) = outcome {
        tracing::warn!("ZSEI write audit row not recorded: {e}");
    }
}

async fn query_zsei(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ZseiQueryRequest>,
) -> Json<ZseiResponse> {
    let variant = req
        .query
        .as_object()
        .and_then(|o| o.keys().next().cloned())
        .unwrap_or_default();
    let is_write = ZSEI_WRITE_VARIANTS.contains(&variant.as_str());
    let container_id = req
        .query
        .get(&variant)
        .and_then(|body| body.get("container_id"))
        .and_then(|v| v.as_u64());
    let session_token = req.session_token.clone();

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

    let identity_validated = if session_token.trim().is_empty() {
        false
    } else {
        let auth = runtime.auth.read().await;
        let token_bytes = hex::decode(&session_token).unwrap_or_default();
        auth.validate_session(&token_bytes).await.is_ok()
    };

    let require_session = std::env::var("OZONE_ZSEI_REQUIRE_SESSION")
        .map(|v| v == "1")
        .unwrap_or(false);
    if is_write && require_session && !identity_validated {
        let row = serde_json::json!({
            "ts": chrono::Utc::now().to_rfc3339(),
            "variant": variant,
            "identity_validated": identity_validated,
            "container_id": container_id,
            "success": false,
            "refused": true,
            "error": "session_token required (OZONE_ZSEI_REQUIRE_SESSION=1)",
        });
        append_zsei_write_audit(&runtime.config.general.data_dir, &row);
        return Json(ZseiResponse {
            success: false,
            result: None,
            error: Some(format!(
                "ZSEI write '{variant}' refused: valid session_token required (OZONE_ZSEI_REQUIRE_SESSION=1)"
            )),
        });
    }

    let outcome = runtime.query_zsei(query).await;
    if is_write {
        let row = serde_json::json!({
            "ts": chrono::Utc::now().to_rfc3339(),
            "variant": variant,
            "identity_validated": identity_validated,
            "container_id": container_id,
            "success": outcome.is_ok(),
            "error": outcome.as_ref().err().map(|e| e.to_string()),
        });
        append_zsei_write_audit(&runtime.config.general.data_dir, &row);
    }

    match outcome {
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

// ============================================================================
// CAPTURE STORES — decision-review / zero-shot-call JSONL, read-only
// (Batch B: B4/B5). Flat append-only files under `{general.data_dir}/
// model_calls/`, structurally outside the ZSEI container system — no
// container, no object_store_path, not a ZSEIQuery shape (confirmed,
// docs/UI_UX_FORK_PLAN.md "Batch B audit results"). Same honesty discipline
// as the private reader these mirror (`read_capture_store`,
// src/consciousness/review.rs:74): missing file = no calls captured yet
// (honest empty, not an error), a malformed line is skipped rather than
// failing the whole read. File is small/append-only — full-read-then-slice
// pagination, no seek-based paging needed.
// ============================================================================

const CAPTURE_DEFAULT_LIMIT: usize = 100;
const CAPTURE_MAX_LIMIT: usize = 500;

/// Read a JSONL file into `Vec<T>`. Missing file → empty vec (not an
/// error). Each line parsed independently — a malformed/unparseable line
/// is skipped, not fatal to the rest of the file.
fn read_jsonl<T: serde::de::DeserializeOwned>(path: &str) -> Vec<T> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<T>(line).ok())
        .collect()
}

/// One row from `{data_dir}/model_calls/decision_review.jsonl`, as
/// `DecisionReviewExecutor::capture` actually writes it
/// (src/orchestrator/decision_review.rs:317-345). `raw_response_preview`
/// is genuinely optional — it postdates older lines written before the
/// field existed, so it must deserialize as absent, not fail the line.
#[derive(Debug, Serialize, Deserialize)]
pub struct DecisionReviewRow {
    pub ts: String,
    pub model_used: String,
    pub tokens_used: u64,
    pub decision: String,
    #[serde(default)]
    pub confidence: Option<f32>,
    pub task_summary_preview: String,
    pub reasoning_preview: String,
    #[serde(default)]
    pub raw_response_preview: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CaptureQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct NodeEventRequest {
    pub project_id: Option<u64>,
    pub modality: String,
    pub graph_id: u64,
    pub node_id: u64,
    pub related_node_id: Option<u64>,
    pub node_type: String,
    pub operation: String,
}

/// POST /orchestration/node-event — the real entry point a modality
/// pipeline subprocess calls to report real per-node/per-edge
/// graph-construction activity (2026-10-18, additive; see
/// `orchestration_events::OrchestrationNodeEvent`'s doc comment for why
/// this HTTP bridge exists at all — the producer runs in a separate OS
/// process with no access to this host's in-memory hub). Fire-and-forget
/// from the caller's side by design: this just republishes to the hub and
/// returns; no validation beyond deserialization, matching `query_zsei`'s
/// own tolerance for a best-effort internal signal rather than a
/// security-sensitive write path.
async fn report_node_event(Json(req): Json<NodeEventRequest>) -> Json<serde_json::Value> {
    crate::orchestration_events::emit_node_event(
        req.project_id,
        &req.modality,
        req.graph_id,
        req.node_id,
        req.related_node_id,
        &req.node_type,
        &req.operation,
    );
    Json(serde_json::json!({ "success": true }))
}

#[derive(Debug, serde::Deserialize)]
pub struct ConnectPeerRequest {
    /// A full libp2p multiaddr, e.g.
    /// "/ip4/161.97.114.105/tcp/9090/p2p/<PeerId>". Validated by
    /// `NetworkManager::connect_peer` itself (real `Multiaddr::from_str`
    /// parse) — this route adds no parsing of its own.
    pub peer_addr: String,
}

/// POST /network/connect-peer — the real, previously-missing entry point
/// to dial a NEW P2P peer (2026-10-19). `NetworkManager::connect_peer` was
/// always real (genuine libp2p swarm dial), but nothing in the HTTP API
/// ever called it with a fresh address — `initialize()` only ever
/// reconnects to peers already present in `known_peers`, which starts
/// empty on a fresh instance. Explicit, operator-triggered only: never
/// called automatically by anything in this codebase. Does not itself
/// enable broadcasting anything to the new peer — that's gated separately
/// by `NetworkConfig::auto_broadcast_sync` (see
/// `orchestrator::amt_loop::spawn_network_hook_bridge`'s call site in
/// `lib.rs`).
async fn connect_peer_route(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConnectPeerRequest>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let mut network = runtime.network.write().await;
    match network.connect_peer(&req.peer_addr).await {
        Ok(()) => Json(serde_json::json!({ "success": true, "peer_addr": req.peer_addr })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct PeerIdRequest {
    /// Base58 libp2p PeerId string (as printed in this host's own logs,
    /// e.g. "P2P: connection established with peer <this>").
    pub peer_id: String,
}

fn parse_peer_id(raw: &str) -> Result<libp2p::PeerId, Json<serde_json::Value>> {
    raw.parse::<libp2p::PeerId>().map_err(|e| {
        Json(serde_json::json!({ "success": false, "error": format!("invalid peer id: {}", e) }))
    })
}

/// POST /network/approve-peer — the real trust gate (2026-10-20). A
/// successful `connect-peer`/inbound dial never implied trust — see
/// `NetworkManager::paired_peers`'s own doc comment. Explicit,
/// operator-triggered only, same loopback-only security boundary as
/// `connect-peer` itself; never called automatically.
async fn approve_peer_route(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PeerIdRequest>,
) -> Json<serde_json::Value> {
    let peer_id = match parse_peer_id(&req.peer_id) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let runtime = state.runtime.read().await;
    let network = runtime.network.read().await;
    match network.approve_peer(peer_id).await {
        Ok(()) => Json(serde_json::json!({ "success": true, "peer_id": req.peer_id })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

/// POST /network/revoke-peer — undo an approval. Added alongside
/// approve-peer, not as an afterthought: an approve-only trust gate with
/// no way back out would itself be a real gap.
async fn revoke_peer_route(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PeerIdRequest>,
) -> Json<serde_json::Value> {
    let peer_id = match parse_peer_id(&req.peer_id) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let runtime = state.runtime.read().await;
    let network = runtime.network.read().await;
    match network.revoke_peer(&peer_id).await {
        Ok(()) => Json(serde_json::json!({ "success": true, "peer_id": req.peer_id })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

/// GET /network/paired-peers — list currently-approved (trusted) peer ids.
async fn paired_peers_route(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let network = runtime.network.read().await;
    Json(serde_json::json!({ "paired_peers": network.paired_peer_ids().await }))
}

/// GET /quota — OpenRouter free-model quota, read from the host's own
/// snapshot (src/openrouter_quota.rs persists this on its own poll cycle;
/// this route never calls OpenRouter itself, so it spends nothing). Uses
/// OZONE_ZSEI_DATA_DIR directly, matching the writer's own path
/// construction (openrouter_quota.rs::snapshot_path) rather than
/// config.general.data_dir, since that's the directory the snapshot is
/// actually written under and the two are not guaranteed to be the same
/// value. No reading becomes a guessed number: a missing or unparsable
/// snapshot returns `{"known": false, "reason": ...}`, never a fabricated
/// used/limit/remaining.
async fn get_quota() -> Json<serde_json::Value> {
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let path = format!("{}/model_calls/openrouter_quota.json", data_dir);
    match tokio::fs::read_to_string(&path).await {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(mut v) => {
                // Same tier rule openrouter_quota.rs itself uses: the
                // reported limit decides the tier, not any other field.
                if let Some(limit) = v.get("limit").and_then(|l| l.as_u64()) {
                    let tier = if limit == 50 { "unfunded" } else { "funded" };
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("known".to_string(), serde_json::json!(true));
                        obj.insert("tier".to_string(), serde_json::json!(tier));
                    }
                }
                Json(v)
            }
            Err(e) => Json(serde_json::json!({
                "known": false,
                "reason": format!("quota snapshot is not valid JSON: {e}")
            })),
        },
        Err(_) => Json(serde_json::json!({
            "known": false,
            "reason": "no quota snapshot yet (host has not completed a quota poll)"
        })),
    }
}

/// GET /model-ledger-summary — per-model outcome counts from the ledger
/// (src/model_ledger.rs `full_summary`, all history: the in-memory table is
/// replayed from model_calls/model_ledger.jsonl on first use) plus today's
/// attributed-cause breakdown (`cause_summary_today`). Read-only, serves the
/// UI's ledger panel, and spends nothing: it never calls OpenRouter.
async fn get_model_ledger_summary() -> Json<serde_json::Value> {
    let models: Vec<serde_json::Value> = crate::model_ledger::full_summary()
        .into_iter()
        .map(|(model, s)| {
            serde_json::json!({
                "model": model,
                "attempts": s.attempts,
                "successes": s.successes,
                "empties": s.empties,
                "http_errors": s.http_errors,
                "timeouts": s.timeouts,
                "other_errors": s.other_errors,
                "median_latency_ms": s.median_latency_ms,
                "last_outcome": s.last_outcome,
                "last_seen_ms": s.last_seen_ms,
            })
        })
        .collect();
    let causes_today: Vec<serde_json::Value> = crate::model_ledger::cause_summary_today()
        .into_iter()
        .map(|(tag, count)| serde_json::json!({ "tag": tag, "count": count }))
        .collect();
    Json(serde_json::json!({
        "models": models,
        "causes_today": causes_today,
    }))
}

/// GET /cross-modal-pairs — the CrossModalIndex derived read-model (2026-10-13):
/// per-request rebuild FROM context.relationships (the only written truth),
/// so the answer is always fresh and there is no second writer. Optional
/// `src`/`dst` query params filter by case-insensitive modality label
/// (e.g. ?src=Text&dst=Code). Read-only, spends nothing.
#[derive(serde::Deserialize)]
struct CrossModalPairsQuery {
    src: Option<String>,
    dst: Option<String>,
}

async fn get_cross_modal_pairs(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<CrossModalPairsQuery>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let outcome = runtime
        .query_zsei(crate::types::zsei::ZSEIQuery::GetCrossModalPairs {
            src: q.src,
            dst: q.dst,
        })
        .await;
    let json = match outcome {
        Ok(crate::types::zsei::ZSEIQueryResult::Content { json: Some(j), .. }) => j,
        Ok(other) => serde_json::json!({
            "error": "unexpected result variant",
            "variant": format!("{:?}", std::mem::discriminant(&other)),
        }),
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    };
    Json(json)
}

/// POST /capture/ingest_image — DESIGNED_NOT_BUILT_REGISTRY §1: turn a host-side
/// screenshot into a REAL Image-modality graph container by running pipeline 102
/// through the SAME entry every execution takes (entry gate trace, jurisdiction,
/// S12 capture, graph ripple). Strict-session protected; LOCAL files under the
/// host data dir ONLY (screenshots live in {data_dir}/.ozone-context/…) — no
/// arbitrary filesystem reads, no URLs. Surface analysis + text detection by
/// default so the call returns in seconds; object/face detection is opt-in.
#[derive(Debug, Deserialize)]
pub struct IngestImageRequest {
    pub path: String,
    #[serde(default)]
    pub project_id: Option<u64>,
    #[serde(default)]
    pub workspace_id: Option<u64>,
    #[serde(default)]
    pub session_token: String,
    #[serde(default)]
    pub detect_objects: bool,
    #[serde(default)]
    pub detect_faces: bool,
}

async fn ingest_image(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IngestImageRequest>,
) -> Json<serde_json::Value> {
    let data_dir = {
        let runtime = state.runtime.read().await;
        runtime.config.general.data_dir.clone()
    };
    // LOCAL-UNDER-DATA-DIR ONLY: canonicalize both sides so `..` games die.
    let (canon, canon_dir) = match (
        std::fs::canonicalize(&req.path),
        std::fs::canonicalize(&data_dir),
    ) {
        (Ok(p), Ok(d)) => (p, d),
        (Err(e), _) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("path unreadable: {}", e)
            }))
        }
        (_, Err(e)) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("data dir unreadable: {}", e)
            }))
        }
    };
    // LOCAL-TO-THE-HOST: data_dir (zsei_data) OR the process working dir
    // (.ozone-context browser-shots live there). Both checks canonicalized;
    // anything else (tmp, home-wide paths) is refused.
    let cwd_ok = std::env::current_dir()
        .ok()
        .and_then(|c| std::fs::canonicalize(c).ok())
        .map_or(false, |c| canon.starts_with(&c));
    if !canon.starts_with(&canon_dir) && !cwd_ok {
        return Json(serde_json::json!({
            "success": false,
            "error": "refused: only files under the host data dir or working dir can be ingested"
        }));
    }
    if !canon.is_file() {
        return Json(serde_json::json!({
            "success": false,
            "error": "refused: path is not a file"
        }));
    }

    // Pipeline 102 contract (assets/pipelines/modalities/image/main.rs): the
    // full PipelineInput envelope with data.action = ImageAction::Analyze and
    // ImageData::FilePath — the binary unwraps `data` on stdin.
    let input = crate::types::pipeline::PipelineInput {
        data: vec![(
            "action".to_string(),
            json_to_typed_value(serde_json::json!({
                "type": "Analyze",
                "image_data": { "FilePath": canon.to_string_lossy() },
                "depth": "Surface",
                "detect_objects": req.detect_objects,
                "detect_faces": req.detect_faces,
                "detect_text": true,
            })),
        )]
        .into_iter()
        .collect(),
        context: crate::types::pipeline::ExecutionContext {
            user_id: 0,
            device_id: 0,
            workspace_id: req.workspace_id,
            project_id: req.project_id,
            task_context_id: None,
            // ExecutionContext.metadata is HashMap<String, String>.
            metadata: [("source".to_string(), "screenshot-ingest".to_string())]
                .into_iter()
                .collect(),
        },
    };

    let runtime = state.runtime.read().await;
    // Registry + entry gate DIRECTLY (not runtime.execute_pipeline — that
    // demands the legacy app-session, which is None on a restarted host;
    // this route is ALREADY operator-authenticated by the strict gate, and
    // entry_gate::gated provides the same trace root + span row every
    // external dispatch takes).
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    match crate::orchestrator::entry_gate::gated(
        102u64,
        registry.execute(102, input, None),
    )
    .await
    {
        Ok(out) => Json(serde_json::json!({
            "success": out.success,
            "error": out.error,
            "output": out.data.get("result").cloned().unwrap_or(serde_json::Value::Null),
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

/// POST /capture/ingest_uidocument — UNCOMPILED pending the build gate.
/// Hands a UI-element document (HTML / CSS / live-DOM batch / Android XML /
/// a11y JSON) to the UiDocument modality (pipeline 127) through the SAME entry
/// every execution takes (strict-session gate, `entry_gate::gated` trace root,
/// registry gate), then persists the returned `graph` block with the SAME
/// function an MCP `graph` block uses (`mcp_graph`), scoped to the caller's
/// workspace/project. Modeled on `ingest_image`.
///
/// Body: `{"flavor"?: "html"|"css"|"dom-live"|"android-xml"|"a11y-json" (default
/// "html"), "url"?, EXACTLY ONE of "path" | "html" | "batch", "project_id"?,
/// "workspace_id"?}`. Inline `html` / `batch` are capped at 5 MB; `path` must
/// be a regular file under the host data dir or working dir (no URLs, no
/// arbitrary reads).
///
/// Pipeline 127 input contract (for the pipeline author): `data.action =
/// {"type":"Analyze","flavor","url","document_data": {"Html": str} |
/// {"FilePath": str} | {"Batch": json}}`; on success `result.graph =
/// {"nodes":[{key,kind,label,parent?,attributes{geom?}}],"edges":[...]}`
/// (nodes carrying `attributes.geom` get spatial relations computed centrally
/// by `mcp_graph::compute_spatial_relations`).
#[derive(Debug, Deserialize)]
pub struct IngestUiDocumentRequest {
    #[serde(default)]
    pub flavor: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub html: Option<String>,
    #[serde(default)]
    pub batch: Option<serde_json::Value>,
    #[serde(default)]
    pub project_id: Option<u64>,
    #[serde(default)]
    pub workspace_id: Option<u64>,
}

const UIDOC_MAX_INLINE_BYTES: usize = 5 * 1024 * 1024;
const UIDOC_FLAVORS: [&str; 6] = ["html", "css", "dom-live", "svg", "android-xml", "a11y-json"];

fn uidoc_refuse(msg: &str) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "success": false, "error": msg }))
}

async fn ingest_uidocument(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IngestUiDocumentRequest>,
) -> Json<serde_json::Value> {
    let flavor = req
        .flavor
        .as_deref()
        .unwrap_or("html")
        .to_ascii_lowercase();
    if !UIDOC_FLAVORS.contains(&flavor.as_str()) {
        return uidoc_refuse(&format!(
            "unknown flavor '{}' (html | css | dom-live | android-xml | a11y-json)",
            flavor
        ));
    }
    let provided = [req.path.is_some(), req.html.is_some(), req.batch.is_some()]
        .iter()
        .filter(|b| **b)
        .count();
    if provided != 1 {
        return uidoc_refuse("provide exactly one of path, html, batch");
    }

    let document_data: serde_json::Value = if let Some(html) = req.html.as_ref() {
        if html.len() > UIDOC_MAX_INLINE_BYTES {
            return uidoc_refuse("refused: inline html exceeds the 5 MB cap");
        }
        serde_json::json!({ "Html": html })
    } else if let Some(batch) = req.batch.as_ref() {
        let size = serde_json::to_vec(batch).map(|v| v.len()).unwrap_or(usize::MAX);
        if size > UIDOC_MAX_INLINE_BYTES {
            return uidoc_refuse("refused: inline batch exceeds the 5 MB cap");
        }
        serde_json::json!({ "Batch": batch })
    } else {
        let path = req.path.as_deref().unwrap_or("");
        let data_dir = {
            let runtime = state.runtime.read().await;
            runtime.config.general.data_dir.clone()
        };
        // Same locality rule as ingest_image: canonicalize both sides so `..`
        // games die; data dir OR the process working dir; regular files only.
        let (canon, canon_dir) = match (
            std::fs::canonicalize(path),
            std::fs::canonicalize(&data_dir),
        ) {
            (Ok(p), Ok(d)) => (p, d),
            (Err(e), _) => return uidoc_refuse(&format!("path unreadable: {}", e)),
            (_, Err(e)) => return uidoc_refuse(&format!("data dir unreadable: {}", e)),
        };
        let cwd_ok = std::env::current_dir()
            .ok()
            .and_then(|c| std::fs::canonicalize(c).ok())
            .map_or(false, |c| canon.starts_with(&c));
        if !canon.starts_with(&canon_dir) && !cwd_ok {
            return uidoc_refuse(
                "refused: only files under the host data dir or working dir can be ingested",
            );
        }
        if !canon.is_file() {
            return uidoc_refuse("refused: path is not a file");
        }
        serde_json::json!({ "FilePath": canon.to_string_lossy() })
    };

    let input = crate::types::pipeline::PipelineInput {
        data: vec![(
            "action".to_string(),
            json_to_typed_value(serde_json::json!({
                "type": "Analyze",
                "flavor": flavor,
                "url": req.url,
                "document_data": document_data,
            })),
        )]
        .into_iter()
        .collect(),
        context: crate::types::pipeline::ExecutionContext {
            user_id: 0,
            device_id: 0,
            workspace_id: req.workspace_id,
            project_id: req.project_id,
            task_context_id: None,
            metadata: [("source".to_string(), "uidocument-ingest".to_string())]
                .into_iter()
                .collect(),
        },
    };

    // Registry + entry gate DIRECTLY (same reasoning as ingest_image: the strict
    // gate already authenticated this route; entry_gate::gated adds the trace
    // root + span row every external dispatch takes).
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    match crate::orchestrator::entry_gate::gated(127u64, registry.execute(127, input, None)).await {
        Ok(out) => {
            // SHAPE TOLERANCE (2026-10-31, found live): the adapter splits the
            // binary's stdout keys into data DIRECTLY (no "result" wrapper) —
            // so when "result" is absent, the whole map IS the output.
            let output = out.data.get("result").cloned().unwrap_or_else(|| {
                serde_json::to_value(&out.data).unwrap_or(serde_json::Value::Null)
            });
            let graph = if out.success {
                let zsei = runtime.zsei.read().await;
                let data_dir = runtime.config.general.data_dir.clone();
                match crate::mcp_graph::persist_from_output_scoped(
                    &zsei,
                    &data_dir,
                    "uidocument_ingest",
                    "uidocument-ingest",
                    &output,
                    req.workspace_id,
                    req.project_id,
                )
                .await
                {
                    Some(Ok(summary)) => serde_json::json!({ "persisted": true, "summary": summary }),
                    Some(Err(e)) => serde_json::json!({ "persisted": false, "error": e }),
                    None => serde_json::Value::Null,
                }
            } else {
                serde_json::Value::Null
            };
            Json(serde_json::json!({
                "success": out.success,
                "error": out.error,
                "flavor": flavor,
                "output": output,
                "graph": graph,
            }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

/// GET /capture/decision-reviews — B4.
async fn get_decision_reviews(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<CaptureQuery>,
) -> Json<serde_json::Value> {
    let data_dir = {
        let runtime = state.runtime.read().await;
        runtime.config.general.data_dir.clone()
    };
    let path = format!("{}/model_calls/decision_review.jsonl", data_dir);
    let rows: Vec<DecisionReviewRow> = read_jsonl(&path);
    let total = rows.len();
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(CAPTURE_DEFAULT_LIMIT).min(CAPTURE_MAX_LIMIT);
    let page: Vec<&DecisionReviewRow> = rows.iter().skip(offset).take(limit).collect();
    Json(serde_json::json!({ "rows": page, "total": total, "offset": offset, "limit": limit }))
}

/// One row from `{data_dir}/model_calls/zero_shot_calls.jsonl`, as
/// `capture_zero_shot_call` actually writes it (src/orchestrator/mod.rs:
/// 2592-2638) — 12 fields, all present on every line (re-verified directly
/// against that function, not assumed).
#[derive(Debug, Serialize, Deserialize)]
pub struct ZeroShotCallRow {
    pub ts: String,
    pub call_site: String,
    pub model_used: String,
    pub tokens_used: u64,
    pub retry_count: u32,
    pub used_fallback: bool,
    pub success: bool,
    pub response_preview: String,
    #[serde(default)]
    pub amt_container_id: Option<u64>,
    #[serde(default)]
    pub blueprint_id: Option<u64>,
    #[serde(default)]
    pub project_id: Option<u64>,
    pub prompt_preview: String,
}

/// One row of `pipeline_zero_shot_calls.jsonl` — the modality pipelines'
/// own capture store (C6-minimal, `assets/pipelines/shared/capture.rs`).
/// Same append/parse honesty as every capture reader: missing file →
/// empty vec, malformed line → skipped, absent fields → defaults (older
/// rows and non-capturing call shapes must not kill the read).
#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineZeroShotCallRow {
    pub ts: String,
    #[serde(default)]
    pub pipeline: String,
    pub call_site: String,
    #[serde(default)]
    pub model_used: String,
    #[serde(default)]
    pub tokens_used: u64,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub response_preview: String,
    #[serde(default)]
    pub prompt_preview: String,
}

#[derive(Debug, Deserialize)]
pub struct PipelineZeroShotQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub pipeline: Option<String>,
    pub call_site: Option<String>,
    pub model_used: Option<String>,
    /// The invisible-failure view is the primary use: `success=false` rows
    /// are exactly the extraction failures that used to vanish.
    pub success: Option<bool>,
}

/// GET /capture/pipeline-zero-shot-calls — the read side of the pipelines'
/// capture store (same flat-file situation as B4/B5: NOT a ZSEIQuery
/// shape). Resolution nuance documented in capture.rs: the file lives
/// where the PIPELINE resolved its data dir (`OZONE_ZSEI_DATA_DIR` →
/// `"zsei_data"`); under the normal layout (host cwd = target/release,
/// `general.data_dir = "zsei_data"`) that is the same directory the
/// host's other model_calls stores use, so `general.data_dir` resolves it.
#[derive(Debug, Deserialize)]
pub struct ToolCallsQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub tool: Option<String>,
    pub agent: Option<String>,
}

/// GET /capture/tool-calls — S13 read route (2026-09-29): the /mcp/call
/// capture store (tool_calls.jsonl). One truthful row per tool call —
/// tool/agent/success/error/input_preview/identity_validated/transport.
/// Same append/parse honesty as every capture reader: missing file ->
/// empty vec, malformed line -> skipped.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCallRow {
    pub ts: String,
    pub tool: String,
    pub agent: String,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub input_preview: String,
    #[serde(default)]
    pub identity_validated: bool,
    #[serde(default)]
    pub transport: String,
}

async fn get_tool_calls(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<ToolCallsQuery>,
) -> Json<serde_json::Value> {
    let data_dir = {
        let runtime = state.runtime.read().await;
        runtime.config.general.data_dir.clone()
    };
    let path = format!("{}/model_calls/tool_calls.jsonl", data_dir);
    let rows: Vec<ToolCallRow> = read_jsonl(&path);
    let filtered: Vec<&ToolCallRow> = rows
        .iter()
        .filter(|r| q.tool.as_deref().map_or(true, |v| r.tool == v))
        .filter(|r| q.agent.as_deref().map_or(true, |v| r.agent == v))
        .collect();
    let total = filtered.len();
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(CAPTURE_DEFAULT_LIMIT).min(CAPTURE_MAX_LIMIT);
    let page: Vec<&ToolCallRow> = filtered.into_iter().skip(offset).take(limit).collect();
    Json(serde_json::json!({ "rows": page, "total": total, "offset": offset, "limit": limit }))
}

async fn get_pipeline_zero_shot_calls(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<PipelineZeroShotQuery>,
) -> Json<serde_json::Value> {
    let data_dir = {
        let runtime = state.runtime.read().await;
        runtime.config.general.data_dir.clone()
    };
    let path = format!("{}/model_calls/pipeline_zero_shot_calls.jsonl", data_dir);
    let rows: Vec<PipelineZeroShotCallRow> = read_jsonl(&path);
    let filtered: Vec<&PipelineZeroShotCallRow> = rows
        .iter()
        .filter(|r| q.pipeline.as_deref().map_or(true, |v| r.pipeline == v))
        .filter(|r| q.call_site.as_deref().map_or(true, |v| r.call_site == v))
        .filter(|r| q.model_used.as_deref().map_or(true, |v| r.model_used == v))
        .filter(|r| q.success.map_or(true, |v| r.success == v))
        .collect();
    let total = filtered.len();
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(CAPTURE_DEFAULT_LIMIT).min(CAPTURE_MAX_LIMIT);
    let page: Vec<&PipelineZeroShotCallRow> =
        filtered.into_iter().skip(offset).take(limit).collect();
    Json(serde_json::json!({ "rows": page, "total": total, "offset": offset, "limit": limit }))
}

#[derive(Debug, Deserialize)]
pub struct ZeroShotQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub amt_container_id: Option<u64>,
    pub blueprint_id: Option<u64>,
    pub project_id: Option<u64>,
    pub call_site: Option<String>,
    pub model_used: Option<String>,
}

/// GET /capture/zero-shot-calls — B5. Filter predicates apply before
/// offset/limit slicing (filter first, then paginate the filtered set).
async fn get_zero_shot_calls(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<ZeroShotQuery>,
) -> Json<serde_json::Value> {
    let data_dir = {
        let runtime = state.runtime.read().await;
        runtime.config.general.data_dir.clone()
    };
    let path = format!("{}/model_calls/zero_shot_calls.jsonl", data_dir);
    let rows: Vec<ZeroShotCallRow> = read_jsonl(&path);
    let filtered: Vec<&ZeroShotCallRow> = rows
        .iter()
        .filter(|r| q.amt_container_id.map_or(true, |v| r.amt_container_id == Some(v)))
        .filter(|r| q.blueprint_id.map_or(true, |v| r.blueprint_id == Some(v)))
        .filter(|r| q.project_id.map_or(true, |v| r.project_id == Some(v)))
        .filter(|r| q.call_site.as_deref().map_or(true, |v| r.call_site == v))
        .filter(|r| q.model_used.as_deref().map_or(true, |v| r.model_used == v))
        .collect();
    let total = filtered.len();
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(CAPTURE_DEFAULT_LIMIT).min(CAPTURE_MAX_LIMIT);
    let page: Vec<&ZeroShotCallRow> = filtered.into_iter().skip(offset).take(limit).collect();
    Json(serde_json::json!({ "rows": page, "total": total, "offset": offset, "limit": limit }))
}

/// Redaction sentinel for serialized secrets. `/config/get` replaces every
/// non-empty `api_key` value with this string; `/config/set` treats an
/// incoming value of exactly this string as "unchanged", so a UI
/// read-modify-write round-trip can never overwrite a real key with the mask.
const CONFIG_SECRET_MASK: &str = "[redacted]";

/// Mask secret fields in a serialized config value. Recursive because the
/// full-config branch nests models behind section objects. `api_key_env` is
/// NOT masked — it is a variable NAME, not a secret.
fn redact_config_secrets(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, val) in map.iter_mut() {
                if k == "api_key" {
                    if let Some(s) = val.as_str() {
                        if !s.is_empty() {
                            *val = serde_json::Value::String(CONFIG_SECRET_MASK.to_string());
                        }
                    }
                } else {
                    redact_config_secrets(val);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_config_secrets(item);
            }
        }
        _ => {}
    }
}

async fn get_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConfigRequest>,
) -> Json<ConfigResponse> {
    let runtime = state.runtime.read().await;

    let mut config = match req.section.as_deref() {
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

    // No secret leaves via this route: mask api_key values in whatever
    // section was requested (perimeter review 2026-10-23 — /config/get
    // previously serialized the raw key to any caller).
    if let Some(ref mut v) = config {
        redact_config_secrets(v);
    }

    Json(ConfigResponse {
        success: config.is_some(),
        config,
        error: None,
    })
}

/// Operator policy: paid models are never selectable. Returns the refusal
/// reason when a `models` update names a paid direct provider, a paid direct
/// endpoint, or an OpenRouter model the live catalog does not mark free (or
/// does not list at all). None means the update may proceed. A refusal applies
/// nothing: set_config returns success=false with this text as the error.
fn paid_policy_refusal(models: &serde_json::Value) -> Option<String> {
    const POLICY: &str = "policy: only free OpenRouter models and local BitNet may be used";
    if let Some(p) = models.get("api_provider").and_then(|v| v.as_str()) {
        if matches!(p, "anthropic" | "openai" | "google") {
            return Some(format!("refused: api_provider '{p}' is a paid direct provider; {POLICY}"));
        }
    }
    if let Some(ep) = models.get("api_endpoint").and_then(|v| v.as_str()) {
        for paid_host in ["api.anthropic.com", "api.openai.com", "generativelanguage.googleapis.com"] {
            if ep.contains(paid_host) {
                return Some(format!("refused: api_endpoint '{ep}' is a paid direct provider; {POLICY}"));
            }
        }
    }
    if let Some(model) = models.get("api_model").and_then(|v| v.as_str()) {
        match crate::model_windows::catalog_entry(model) {
            Some(e) if e.is_free => {}
            Some(_) => {
                return Some(format!("refused: api_model '{model}' is a paid OpenRouter model; {POLICY}"))
            }
            // The free router is allowed by name: it is the policy's own free
            // entry, and it must stay selectable while the catalog is not
            // loaded (network down, no persisted copy yet).
            None if model == "openrouter/free" => {}
            None => {
                return Some(format!(
                    "refused: api_model '{model}' is not in the live OpenRouter free catalog (unknown, paid, or the catalog is not loaded yet); {POLICY}"
                ))
            }
        }
    }
    // add_model: parse it here, so a malformed entry is refused with a reason
    // rather than dropped by the silent `if let Ok` further down, and check the
    // entry's provider, endpoint and (for API entries) its OpenRouter identifier.
    if let Some(v) = models.get("add_model") {
        match serde_json::from_value::<crate::config::AvailableModel>(v.clone()) {
            Err(e) => {
                return Some(format!("refused: add_model is malformed ({e}); nothing was applied"));
            }
            Ok(m) => {
                let as_update = serde_json::json!({
                    "api_provider": m.provider,
                    "api_endpoint": m.api_endpoint,
                    "api_model": if m.model_type == "api" { Some(m.identifier.clone()) } else { None },
                });
                if let Some(reason) = paid_policy_refusal(&as_update) {
                    return Some(reason);
                }
            }
        }
    }
    None
}

async fn set_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConfigSetRequest>,
) -> Json<ConfigSetResponse> {
    // Per-user policy: refuse paid selections only while this user's config
    // has allow_paid_models = false (the default). Nothing is silently changed.
    let allow_paid = state.runtime.read().await.config.models.allow_paid_models;
    if let Some(refusal) = (!allow_paid)
        .then(|| req.updates.get("models").and_then(paid_policy_refusal))
        .flatten()
    {
        tracing::warn!(reason = %refusal, "set_config refused a paid model selection");
        return Json(ConfigSetResponse { success: false, error: Some(refusal) });
    }

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
                // Paid providers (anthropic, openai, google) never reach this
                // point: paid_policy_refusal rejects them before any update.
                model_config.api_endpoint = match v {
                    "openrouter" => {
                        Some("https://openrouter.ai/api/v1/chat/completions".to_string())
                    }
                    _ => model_config.api_endpoint, // already Option<String>
                };
                if v == "openrouter" {
                    model_config.wire_protocol = Some("chat_completions".to_string());
                }
            }
            if let Some(v) = models.get("api_key").and_then(|v| v.as_str()) {
                // Redaction sentinel = "unchanged" (get masked it on the way
                // out; a UI round-trip posts the mask back and must NOT
                // overwrite the real key with it).
                if v != CONFIG_SECRET_MASK {
                    // Raw secret value — belongs in api_key (persisted, gitignored
                    // config.toml), never in api_key_env (that field is a NAME,
                    // e.g. "ANTHROPIC_API_KEY", not a value).
                    model_config.api_key = Some(v.to_string());
                }
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

    // Real sections touched — derived directly from the request's own
    // top-level keys, never guessed at which fields within them actually
    // changed value (that level of detail isn't worth the complexity for
    // an audit-trail ripple; "these sections were part of this config-set
    // call" is the honest claim this can make).
    let sections_touched: Vec<String> = req
        .updates
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();

    // Save config to file
    let result = match toml::to_string_pretty(&runtime.config) {
        Ok(config_str) => match std::fs::write(&config_path, &config_str) {
            Ok(_) => ConfigSetResponse {
                success: true,
                error: None,
            },
            Err(e) => ConfigSetResponse {
                success: false,
                error: Some(format!("Failed to write config: {}", e)),
            },
        },
        Err(e) => ConfigSetResponse {
            success: false,
            error: Some(format!("Failed to serialize config: {}", e)),
        },
    };

    // Real ripple for real config changes — this was a genuine gap (R5):
    // every settings change (model switch, consciousness toggle, network
    // config) left zero trace in the graph. Only fires on an actual
    // successful save, and only claims what's real: which top-level
    // sections were touched, never fabricated per-field diffs. Same
    // reused mirror() mechanism as the mcp_call tool_call ripple —
    // genuinely goes through ZSEI::query's CreateContainer choke point.
    if result.success && !sections_touched.is_empty() {
        let data_dir = runtime.config.general.data_dir.clone();
        let zsei = runtime.zsei.read().await;
        let req_mirror = crate::context_mirror::MirrorRequest {
            kind: "config_change".to_string(),
            agent: "host-config".to_string(),
            title: format!("Config updated: {}", sections_touched.join(", ")),
            body: String::new(),
            files: Vec::new(),
            detail: Some(serde_json::json!({ "sections": sections_touched })),
            scope: Some("global".to_string()),
            workspace_id: None,
            project_id: None,
        };
        let _ = crate::context_mirror::mirror(&zsei, &data_dir, &req_mirror).await;
    }

    Json(result)
}

// ============================================================================
// Orchestration Handler — full 14-stage AMT flow
// ============================================================================

async fn orchestrate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OrchestrateRequest>,
) -> Json<OrchestrateResponse> {
    let start = std::time::Instant::now();

    // SESSION VALIDATION (CC's long-standing finding, closed 2026-09-29):
    // /orchestrate never read session_token — every caller was implicitly
    // trusted. Now: a PROVIDED token is validated against the real
    // AuthSystem; an INVALID token is rejected outright. A MISSING token
    // remains allowed under the documented localhost-only trust boundary
    // (the Electron UI does not yet attach sessions to /orchestrate —
    // wiring that is the remaining half, tracked in CHECKLIST).
    if let Some(token) = &req.session_token {
        if !token.trim().is_empty() {
            let auth_guard = state.runtime.read().await;
            let auth = auth_guard.auth.read().await;
            let token_bytes = hex::decode(token).unwrap_or_default();
            if auth.validate_session(&token_bytes).await.is_err() {
                return Json(OrchestrateResponse {
                    success: false,
                    response: None,
                    task_id: None,
                    blueprint_id: None,
                    stages_completed: vec![],
                    needs_clarification: false,
                    clarification_points: vec![],
                    error: Some("Invalid session token".into()),
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    model_used: None,
                    total_tokens_used: None,
                    amt_summary: None,
                    thinking_log: vec![],
                    attempt_trail: vec![],
                    refusals: vec![],
                });
            }
        }
    }

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

    // Run through the orchestrator — full 14-stage AMT flow:
    // Stage 1-3: Intent capture (IntentCapture, BranchCapture, DetailCapture)
    // Stage 4-5: Context aggregation + cross-reference
    // Stage 6-7: Blueprint search + selection
    // Stage 8-12: Pipeline execution per blueprint step
    // Stage 13-14: Response synthesis + consciousness post-hook
    //
    // DETACHED: the walk runs on its own task. Axum drops this handler
    // future when the client disconnects (curl -m, UI reload), and before
    // this the drop cancelled the walk mid-stage. The spawned task owns its
    // runtime handle and runs to completion regardless; the completion log
    // fires from inside the task, so a dropped client still leaves the
    // outcome in the logs. The walk's task and stage records are written as
    // it goes. A panic inside the walk becomes an error response, not silence.
    let user_id = req.user_id;
    let device_id = req.device_id;
    let runtime_handle = state.runtime.clone();
    let walk = tokio::spawn(async move {
        let runtime = runtime_handle.read().await;
        let outcome = runtime.orchestrate(pipeline_input, user_id, device_id).await;
        match &outcome {
            Ok(result) => tracing::info!(
                "Orchestration complete: task={:?}, blueprint={:?}, {}ms",
                result.task_id,
                result.blueprint_id,
                start.elapsed().as_millis() as u64
            ),
            Err(e) => tracing::error!("Orchestration failed: {}", e),
        }
        outcome
    });
    let outcome = match walk.await {
        Ok(outcome) => outcome,
        Err(join_err) => {
            tracing::error!("Orchestration task aborted: {}", join_err);
            Err(crate::types::OzoneError::TaskError(format!(
                "orchestration task aborted: {join_err}"
            )))
        }
    };

    match outcome {
        Ok(result) => {
            let execution_time_ms = start.elapsed().as_millis() as u64;
            // A failed walk still arrives here (success: false, log intact),
            // so the trail is read on both success and failure.
            let (attempt_trail, refusals) = attempt_trail_and_refusals(&result.thinking_log);
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
                attempt_trail,
                refusals,
            })
        }
        Err(e) => {
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
                // An Err here means the request was rejected before any walk
                // (empty prompt) or the task aborted; no walk state exists to
                // read a trail from, so both lists are empty by construction.
                attempt_trail: vec![],
                refusals: vec![],
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
    // "general"), so search every known category dir.
    //
    // SOURCE-FIRST (2026-10-18, found live): the data-dir copy
    // ({data_dir}/pipelines/... — where bootstrap places a component.js at
    // first boot) used to come FIRST in this search, so the Workspace and
    // Tasks tabs were served a FROZEN bootstrap-era copy of component.js
    // while every fix landed in assets/pipelines (the source of truth) and
    // never reached the UI no matter how many times anything was rebuilt.
    // The CARGO_MANIFEST_DIR-anchored assets dir is now the FIRST candidate
    // (same convention as the executor's binary search); the data-dir copy
    // remains a fallback for deployments without the source tree.
    let source_pipelines = format!("{}/assets/pipelines", env!("CARGO_MANIFEST_DIR"));
    let pipelines_path =
        std::env::var("OZONE_PIPELINES_PATH").unwrap_or_else(|_| "./pipelines".to_string());
    let data_pipelines = {
        let r = state.runtime.read().await;
        format!("{}/pipelines", r.config.general.data_dir)
    };

    let category_variants = [category, "general", "consciousness", "modalities", "shared"];
    let bases = [
        source_pipelines,
        data_pipelines,
        pipelines_path,
        "./pipelines".to_string(),
    ];
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

/// T-G4 wire contract — the EXACT frame a connected WebSocket receives for
/// a graph event. Extracted from handle_websocket so the UI/agent-facing
/// shape is pinned by test, not by convention.
pub(crate) fn graph_event_frame(evt: &crate::graph_events::GraphEvent) -> String {
    let wire = serde_json::json!({
        "action": "graph_event",
        "event": evt.event,
        "container_id": evt.container_id,
        "parent_id": evt.parent_id,
        "container_type": evt.container_type,
        "source": evt.source,
        "scope_keywords": evt.scope_keywords,
        "timestamp": evt.timestamp,
    });
    serde_json::to_string(&wire).unwrap_or_default()
}

async fn handle_websocket(mut socket: WebSocket, state: Arc<AppState>) {
    // Start progress broadcast task
    let progress_map = state.executor_progress.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);

    // GRAPH RIPPLE — subscribe this socket to the living-graph event hub:
    // every graph write anywhere (containers, links, coordination mirrors)
    // pushes to connected UIs and agents in real time. Scope filtering is
    // client-side for now (events carry their scope keywords).
    {
        let tx = tx.clone();
        let mut graph_rx = crate::graph_events::GraphEventHub::global().subscribe();
        tokio::spawn(async move {
            loop {
                match graph_rx.recv().await {
                    Ok(evt) => {
                        let frame = graph_event_frame(&evt);
                        if tx.send(frame).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        let _ = tx
                            .send(format!(
                                "{{\"action\":\"graph_event\",\"lagged\":{n}}}"
                            ))
                            .await;
                    }
                    Err(_) => break,
                }
            }
        });
    }

    // ORCHESTRATION STAGE RIPPLE — the same wire, a sibling hub. Every real
    // stage of an in-flight /orchestrate call (all 15, including the ones
    // with no subprocess pipeline for `pipeline_progress` to track: Build
    // AMT, Blueprint Assignment, Zero-Shot Simulation) reaches connected UIs
    // live instead of only after the whole blocking call returns.
    {
        let tx = tx.clone();
        let mut orch_rx = crate::orchestration_events::OrchestrationEventHub::global().subscribe();
        tokio::spawn(async move {
            loop {
                match orch_rx.recv().await {
                    Ok(evt) => {
                        let wire = serde_json::json!({
                            "action": "orchestration_stage",
                            "user_id": evt.user_id,
                            "device_id": evt.device_id,
                            "project_id": evt.project_id,
                            "workspace_id": evt.workspace_id,
                            "stage": evt.stage,
                            "stage_name": evt.stage_name,
                            "success": evt.success,
                            "summary": evt.summary,
                            "duration_ms": evt.duration_ms,
                            "timestamp": evt.timestamp,
                        });
                        if tx
                            .send(serde_json::to_string(&wire).unwrap_or_default())
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        let _ = tx
                            .send(format!(
                                "{{\"action\":\"orchestration_stage\",\"lagged\":{n}}}"
                            ))
                            .await;
                    }
                    Err(_) => break,
                }
            }
        });
    }

    // NODE RIPPLE — a third sibling hub (2026-10-18, additive), finer than
    // `orchestration_stage` above: real per-node/per-edge construction
    // activity reported by a modality pipeline subprocess via `POST
    // /orchestration/node-event` (no in-process access to this hub from a
    // separate OS process — see `OrchestrationNodeEvent`'s doc comment).
    {
        let tx = tx.clone();
        let mut node_rx = crate::orchestration_events::NodeEventHub::global().subscribe();
        tokio::spawn(async move {
            loop {
                match node_rx.recv().await {
                    Ok(evt) => {
                        let wire = serde_json::json!({
                            "action": "node_event",
                            "project_id": evt.project_id,
                            "modality": evt.modality,
                            "graph_id": evt.graph_id,
                            "node_id": evt.node_id,
                            "related_node_id": evt.related_node_id,
                            "node_type": evt.node_type,
                            "operation": evt.operation,
                            "timestamp": evt.timestamp,
                        });
                        if tx
                            .send(serde_json::to_string(&wire).unwrap_or_default())
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        let _ = tx
                            .send(format!("{{\"action\":\"node_event\",\"lagged\":{n}}}"))
                            .await;
                    }
                    Err(_) => break,
                }
            }
        });
    }

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

    {
        let data_dir = {
            let r = runtime.read().await;
            r.config.general.data_dir.clone()
        };
        init_internal_token(&data_dir);
    }

    let progress_map = {
        let r = runtime.read().await;
        let map = r.pipeline_registry.read().await.progress_map();
        map
    };

    // PERSISTED MCP TOOL REGISTRY (registry-persistence fix, 2026-09-28):
    // registrations survive restarts — found live when the 22:41 restart
    // wiped the 69 bridge/terminal registrations.
    let mcp_persist_path = {
        let r = runtime.read().await;
        format!("{}/mcp_tool_registry.json", r.config.general.data_dir)
    };
    let mcp_registry = Arc::new(
        crate::mcp::McpRegistry::new().with_persistence(mcp_persist_path.clone()),
    );
    let restored = mcp_registry.load_persisted().await;
    if restored > 0 {
        tracing::info!("MCP tool registry: restored {} persisted tool(s) from {}", restored, mcp_persist_path);
    }
    let mcp_usage = Arc::new(crate::mcp::UsageLedger::new());
    // Process-global MCP handles — orchestrator stages call
    // crate::mcp::call_global without needing AppState threaded through.
    crate::mcp::install_global(mcp_registry.clone(), mcp_usage.clone());

    // P2P-BRIDGE REGISTRATION (2026-10-23 — closes the "supervisor-known but
    // registry-absent" two-books gap): when P2P is enabled, the operator's
    // bridge tool gets its registry entry so /mcp/call's on-demand start can
    // reach it. auto_start stays FALSE (operator opt-in per tool, per the
    // standing ruling); the supervisor's requires_p2p_enabled gate still
    // applies to every start path.
    let enable_p2p = runtime.read().await.config.network.enable_p2p;
    if enable_p2p {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = mcp_registry
            .register(crate::mcp::McpTool {
                name: "p2p-bridge".to_string(),
                transport: crate::mcp::McpTransport::Http,
                endpoint: "http://127.0.0.1:3278/call".to_string(),
                capabilities: vec![
                    "p2p".to_string(),
                    "tunnel".to_string(),
                    "connect-peer".to_string(),
                ],
                server_version: Some("0.1.0".to_string()),
                registered_at: now,
                auto_start: false,
            })
            .await;
        tracing::info!("p2p-bridge MCP registered (auto_start=false — on-demand start per supervisor gates)");
    }

    // MCP TOOL-SERVER SUPERVISOR (2026-10-18): registration survives
    // restarts, but nothing ever started the backing processes — the
    // boot-time autostart pass + 30s crash-restart loop lives here,
    // gated entirely on the operator's own per-tool `auto_start` choice
    // (default false for all 134 persisted entries; never flipped by
    // this code). See src/mcp_supervisor.rs's module doc for the real
    // device/credential requirements this pass honors.
    let (zsei_data_dir, enable_p2p) = {
        let r = runtime.read().await;
        (r.config.general.data_dir.clone(), r.config.network.enable_p2p)
    };
    let mcp_supervisor = crate::mcp_supervisor::spawn_autostart_supervisor(
        mcp_registry.clone(),
        zsei_data_dir.clone(),
        enable_p2p,
    );

    let state = Arc::new(AppState {
        runtime,
        start_time: std::time::Instant::now(),
        executor_progress: progress_map,
        pairing: Arc::new(crate::pairing::PairingHub::new()),
        mcp: mcp_registry,
        mcp_usage,
        mcp_supervisor,
        zsei_data_dir,
    });

    // GRAPH RIPPLE → monitor feed: graph writes surface in the same feed
    // every agent and the dashboard already watches.
    {
        let state_for_graph = state.clone();
        let mut graph_rx = crate::graph_events::GraphEventHub::global().subscribe();
        tokio::spawn(async move {
            loop {
                match graph_rx.recv().await {
                    Ok(evt) => {
                        let runtime = state_for_graph.runtime.read().await;
                        let registry = runtime.pipeline_registry.read().await;
                        registry.activity_hub().record(
                            crate::monitor::ActivityKind::Bridge,
                            crate::monitor::ActivityLevel::Info,
                            &evt.source,
                            format!("graph {}: {} ({})", evt.event, evt.container_type, evt.container_id),
                            Some(serde_json::json!({
                                "container_id": evt.container_id,
                                "parent_id": evt.parent_id,
                                "scope_keywords": evt.scope_keywords,
                            })),
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        });
    }

    // CORS (perimeter review 2026-10-23 — `allow_origin(Any)` meant any web
    // page could read every loopback route from the operator's browser):
    // only loopback browser contexts (localhost / 127.0.0.1 / [::1]) and the
    // Electron file:// origin ("null") may read responses. Non-browser
    // clients (curl, agents, the paired devices) send no Origin at all and
    // are unaffected — CORS is a browser control.
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(|origin, _| {
            let o = match origin.to_str() {
                Ok(s) => s,
                Err(_) => return false,
            };
            o == "null"
                || o.contains("localhost")
                || o.contains("127.0.0.1")
                || o.contains("[::1]")
        }))
        .allow_methods(Any)
        .allow_headers(Any);

    // 20MB body limit (raised 2026-09-29 from axum's 2MB default): image
    // ingests and YOLO detection payloads are multi-MB base64 — the visual
    // MCP accepts 20MB, the call surface must match.
    let app = Router::new().layer(axum::extract::DefaultBodyLimit::max(20 * 1024 * 1024))
        .route("/health", get(health))
        .route("/auth/challenge", post(request_challenge))
        .route("/auth/authenticate", post(authenticate))
        .route("/pipeline/execute", post(execute_pipeline))
        .route("/consciousness/reviews/pending", get(list_pending_reviews_route))
        .route("/consciousness/reviews/resolve", post(resolve_review_route))
        .route("/pipeline/registry", post(get_pipeline_registry))
        .route("/pipeline/ui-component", post(get_pipeline_ui_component))
        .route("/task/get", post(get_task))
        .route("/task/list", post(list_tasks))
        .route("/order/global", get(get_global_order))
        .route("/assistant/feed", get(get_assistant_feed))
        .route("/consciousness/review_pass", post(consciousness_review_pass))
        .route("/task/cancel", post(cancel_task))
        .route("/task/step/rerun", post(rerun_step))
        .route("/zsei/query", post(query_zsei))
        .route("/orchestration/node-event", post(report_node_event))
        // Real, explicit, operator-triggered P2P dial — see
        // connect_peer_route's doc comment (2026-10-19).
        .route("/network/connect-peer", post(connect_peer_route))
        .route("/network/approve-peer", post(approve_peer_route))
        .route("/network/revoke-peer", post(revoke_peer_route))
        .route("/network/paired-peers", get(paired_peers_route))
        // OpenRouter quota status, read-only, spends nothing (2026-10-07).
        .route("/quota", get(get_quota))
        .route("/model-ledger-summary", get(get_model_ledger_summary))
        .route("/cross-modal-pairs", get(get_cross_modal_pairs))
        // CAPTURE STORES — decision-review / zero-shot-call JSONL reads,
        // outside the ZSEI container system (Batch B: B4/B5).
        .route("/capture/decision-reviews", get(get_decision_reviews))
        .route("/capture/ingest_image", post(ingest_image))
        .route("/capture/ingest_uidocument", post(ingest_uidocument))
        .route("/capture/zero-shot-calls", get(get_zero_shot_calls))
        .route(
            "/capture/pipeline-zero-shot-calls",
            get(get_pipeline_zero_shot_calls),
        )
        .route("/capture/tool-calls", get(get_tool_calls))
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
        .route("/pair/:pairing_id", get(pair_page))
        .route("/pairing/status", get(pairing_status))
        .route("/pairing/approve", post(approve_pairing))
        .route("/devices", get(list_devices))
        // MCP TOOLS — external tool registration (the 90+ tool surface).
        .route("/mcp/tools", get(mcp_list_tools))
        .route("/mcp/tools/register", post(mcp_register_tool))
        .route("/mcp/tools/unregister", post(mcp_unregister_tool))
        // AUTO-START CONFIG + process control (2026-10-18) — operator
        // opt-in per tool, real start/stop against the real supervisor.
        .route("/mcp/tools/:name/autostart", post(mcp_set_autostart))
        .route("/mcp/tools/:name/start", post(mcp_start_tool))
        .route("/mcp/tools/:name/stop", post(mcp_stop_tool))
        // MCP USAGE — the per-agent daily budget, enforced host-side.
        .route("/mcp/usage", get(mcp_usage_snapshot))
        .route("/mcp/usage", post(mcp_usage_record))
        // THE standardized abstract MCP call — one shape for every agent
        // and every tool.
        .route("/mcp/call", post(mcp_call))
        // CONTEXT MIRROR — coordination events as ZSEI graph containers.
        .route("/context/mirror", post(mirror_context))
        // COORDINATION — read-only cross-process peek at the Node MCP
        // server's .ozone-context/state.json (Batch B: B10/B11).
        .route("/coordination/presence", get(get_coordination_presence))
        .route("/coordination/claims", get(get_coordination_claims))
        // TASK CREATE — coordination tasks from the shared-context tool;
        // real TaskManager records, listed with every other task.
        .route("/task/create", post(create_coordination_task))
        // TASK UPDATE — coordination-task lifecycle (routed agent marks
        // its own work completed/failed).
        .route("/task/update", post(update_coordination_task))
        .layer(cors)
        .layer(axum::middleware::from_fn_with_state(state.clone(), session_gate))
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
    /// Reserved-id reconciliation (2026-10-23): registering a BUILT-IN id is
    /// refused by default (remote dispatch wins at the executor, so an
    /// unauthenticated override = hijack). An authenticated operator MAY
    /// override deliberately: this flag set AND a valid `session_token`.
    /// Non-built-in ids ignore this flag entirely.
    #[serde(default)]
    pub allow_builtin_override: bool,
    #[serde(default)]
    pub session_token: String,
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
/// Bridge graph state upsert (guide §4): find-or-create a container under
/// /External (keyword `bridge:<pipeline_id>` = idempotence key, same
/// scan-based pattern context_mirror uses for claim dedupe), then rewrite
/// its state JSON on every heartbeat. The container anchors the bridge in
/// the graph; the state file is the live connection truth.
async fn upsert_bridge_container(
    zsei: &crate::zsei::ZSEI,
    data_dir: &str,
    pipeline_id: u64,
    name: &str,
    execute_url: &str,
) -> Result<u64, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dedupe_keyword = format!("bridge:{}", pipeline_id);

    // Find-or-create the container under the External root.
    let existing = {
        let root = zsei
            .get_container(crate::types::container::EXTERNAL_ROOT_ID)
            .await
            .map_err(|e| e.to_string())?;
        let mut found = None;
        if let Some(root) = root {
            for child_id in &root.global_state.child_ids {
                if let Ok(Some(child)) = zsei.get_container(*child_id).await {
                    if child
                        .local_state
                        .context
                        .keywords
                        .iter()
                        .any(|k| k == &dedupe_keyword)
                    {
                        found = Some(*child_id);
                        break;
                    }
                }
            }
        }
        found
    };

    let container_id = match existing {
        Some(id) => id,
        None => {
            let container = crate::types::container::Container {
                global_state: crate::types::container::GlobalState {
                    container_id: 0, // allocated by CreateContainer
                    parent_id: crate::types::container::EXTERNAL_ROOT_ID,
                    child_ids: vec![],
                    child_count: 0,
                    version: 1,
                },
                local_state: crate::types::container::LocalState {
                    metadata: crate::types::container::Metadata {
                        container_type: crate::types::container::ContainerType::Pipeline,
                        modality: crate::types::container::Modality::Unknown,
                        created_at: now,
                        updated_at: now,
                        provenance: "ozone-bridge".to_string(),
                        permissions: 0,
                        owner_id: 0,
                        name: Some(format!("Bridge: {} ({})", name, pipeline_id)),
                        materialized_path: Some(format!("/External/Bridges/{}", name)),
                    },
                    context: crate::types::container::Context {
                        categories: vec![],
                        methodologies: vec![],
                        keywords: vec![
                            "bridge".to_string(),
                            dedupe_keyword.clone(),
                            name.to_lowercase(),
                        ],
                        topics: vec!["bridge".to_string()],
                        relationships: vec![],
                        learned_associations: vec![],
                        embedding: None,
                    },
                    storage: crate::types::container::StoragePointers {
                        db_shard_id: None,
                        vector_index_ref: None,
                        object_store_path: Some(format!("bridges/bridge_{}.json", pipeline_id)),
                        compression_type: crate::types::container::CompressionType::None,
                    },
                    hints: crate::types::container::TraversalHints::default(),
                    integrity: crate::types::container::IntegrityData::default(),
                    file_context: None,
                    code_context: None,
                    text_context: None,
                    external_ref: None,
                },
            };
            match zsei
                .query(crate::types::zsei::ZSEIQuery::CreateContainer {
                    parent_id: crate::types::container::EXTERNAL_ROOT_ID,
                    container,
                })
                .await
            {
                Ok(crate::types::zsei::ZSEIQueryResult::ContainerID(id)) => id,
                Ok(_) => return Err("unexpected CreateContainer result".to_string()),
                Err(e) => return Err(e.to_string()),
            }
        }
    };

    // Live connection state — rewritten on every heartbeat.
    let dir = std::path::PathBuf::from(data_dir).join("bridges");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let state = serde_json::json!({
        "pipeline_id": pipeline_id,
        "name": name,
        "execute_url": execute_url,
        "connected": true,
        "last_heartbeat": now,
    });
    std::fs::write(
        dir.join(format!("bridge_{}.json", pipeline_id)),
        serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(container_id)
}

async fn register_remote_pipeline(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RemotePipelineRegisterRequest>,
) -> Json<RemotePipelineRegisterResponse> {
    // RESERVED-ID GUARD, reconciled (2026-10-23): remote dispatch wins over
    // built-ins at the executor, so an override of a built-in id redirects
    // that pipeline's every call — a hijack in unauthenticated hands, but a
    // DELIBERATE FEATURE for an authenticated operator (e.g. serving
    // pipeline 9 from a registered agent). Reconciliation: refuse by
    // default; allow ONLY when the request sets `allow_builtin_override`
    // AND carries a valid session token. Non-built-in ids are unaffected.
    let runtime = state.runtime.read().await;
    let is_builtin = crate::pipeline::registry::category_of(req.pipeline_id).is_some();
    if is_builtin {
        let authenticated = {
            let auth = runtime.auth.read().await;
            !req.session_token.is_empty()
                && auth
                    .validate_session(&hex::decode(&req.session_token).unwrap_or_default())
                    .await
                    .is_ok()
        };
        if !req.allow_builtin_override || !authenticated {
            tracing::warn!(
                pipeline_id = req.pipeline_id,
                name = %req.name,
                override_flag = req.allow_builtin_override,
                authenticated,
                "remote pipeline registration REFUSED: built-in id (override requires allow_builtin_override + a valid session)"
            );
            return Json(RemotePipelineRegisterResponse {
                success: false,
                error: Some(format!(
                    "pipeline id {} is a built-in: override requires allow_builtin_override=true AND a valid session token",
                    req.pipeline_id
                )),
            });
        }
        tracing::warn!(
            pipeline_id = req.pipeline_id,
            name = %req.name,
            "built-in pipeline id OVERRIDE by authenticated operator — remote dispatch will shadow the built-in until deregistered"
        );
    }
    let registry = runtime.pipeline_registry.read().await;
    let roles = req
        .roles
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| vec!["agent".to_string()]);
    let execute_url = req.execute_url.clone();
    let entry = match registry
        .remote_pipelines()
        .register_guarded(
            req.pipeline_id,
            req.name,
            execute_url.clone(),
            roles,
            is_builtin && req.allow_builtin_override,
        )
        .await
    {
        Ok(entry) => entry,
        Err(e) => {
            return Json(RemotePipelineRegisterResponse { success: false, error: Some(e) });
        }
    };
    // BRIDGE GRAPH STATE (guide §4, Phase 2 — operator-approved): bridge-
    // range registrations (200-299) get a real container under /External
    // (keyword `bridge:<id>` for idempotent find-or-create) whose state
    // JSON (execute_url, last heartbeat) is rewritten on EVERY heartbeat.
    // Bridges are long-lived external participants: their connection state
    // is graph state, rippled like every other write.
    if (200..300).contains(&req.pipeline_id) {
        let zsei = runtime.zsei.read().await;
        let data_dir = runtime.config.general.data_dir.clone();
        if let Err(e) = upsert_bridge_container(
            &zsei,
            &data_dir,
            req.pipeline_id,
            &entry.name,
            &execute_url,
        )
        .await
        {
            tracing::warn!(
                pipeline_id = req.pipeline_id,
                error = %e,
                "bridge container upsert failed (registration still valid)"
            );
        }
    }
    // Seed the execution gate (registry.blueprints) so this id can actually
    // be dispatched to: RegistryExecutorAdapter::execute and
    // PipelineRegistry::execute both refuse any pipeline_id absent from that
    // map BEFORE remote dispatch is ever attempted, and it's normally only
    // populated at boot from the compile-time PIPELINE_INFO table (ids
    // 1-55) — a fresh remote id like 9001 would otherwise 404 here even
    // though RemotePipelines itself is ready to serve it.
    // BUILT-IN OVERRIDE: skip the blueprint seed — the compile-time map
    // already holds the real blueprint for this id, and overwriting it with
    // an empty remote stub would corrupt built-in dispatch. The override
    // works at the EXECUTOR (remote-first check) which needs only the
    // RemotePipelines entry above. Fresh (non-built-in) ids still seed.
    let skip_seed = is_builtin;
    if !skip_seed {
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
    } // end !skip_seed
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

    // OUT-OF-BAND APPROVAL (perimeter review 2026-10-23 — the pairing flow
    // used to hand its own approval code to whoever called /pairing/start,
    // making sessions self-issuable): the API response now carries NO code.
    // The QR/URL encodes only the pairing id; the CODE is delivered on the
    // HOST — console log + a file under <data_dir>/pairing_pending/ — and
    // the /pair/<pairing_id> page asks the operator to type what the host
    // shows. Whoever can reach the API can START pairings; only whoever can
    // read the host console can APPROVE them.
    let config = {
        let r = state.runtime.read().await;
        r.config.grpc.clone()
    };
    let host = crate::pairing::pairing_public_host(&config.address);
    let port = config.port;
    let approve_url = format!("http://{}:{}/pair/{}", host, port, pairing_id);

    // Host-side code delivery: console + file (the two out-of-band surfaces).
    tracing::info!(
        pairing_id = %pairing_id,
        "PAIRING STARTED — approval code: {} (enter it on the /pair/{} page to approve; never share it over the API)",
        code, pairing_id
    );
    {
        let runtime = state.runtime.read().await;
        let dir = std::path::PathBuf::from(&runtime.config.general.data_dir)
            .join("pairing_pending");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(
            dir.join(format!("{}.code", pairing_id)),
            format!("{}\n", code),
        );
    }

    {
        let runtime = state.runtime.read().await;
        let registry = runtime.pipeline_registry.read().await;
        registry.activity_hub().record(
            crate::monitor::ActivityKind::Bridge,
            crate::monitor::ActivityLevel::Info,
            "pairing",
            format!("Pairing started (device: waiting for scan + host-console code)"),
            None,
        );
    }

    Json(serde_json::json!({
        "pairing_id": pairing_id,
        "code_delivery": "host-console",
        "approve_url": approve_url,
        "qr_payload": approve_url,
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
    axum::extract::Path(_pairing_id): axum::extract::Path<String>,
) -> axum::response::Html<&'static str> {
    // OUT-OF-BAND APPROVAL PAGE (2026-10-23): the URL carries ONLY the
    // pairing id — never the code. The operator types the code the HOST
    // shows (console log / pairing_pending/<pairing_id>.code). Standalone
    // inline HTML so a phone scanning the QR needs no app build.
    axum::response::Html(r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Ozone-Studio — approve device</title>
<style>body{font-family:system-ui,sans-serif;background:#0d1117;color:#e6edf3;display:flex;min-height:100vh;align-items:center;justify-content:center;margin:0}
.box{background:#161b22;border:1px solid #30363d;border-radius:12px;padding:28px;max-width:380px;width:90%}
h1{font-size:17px;margin:0 0 6px}p{font-size:13px;color:#8b949e;margin:4px 0 14px}
input{width:100%;box-sizing:border-box;background:#0d1117;color:#e6edf3;border:1px solid #30363d;border-radius:8px;padding:10px;font-size:16px;letter-spacing:2px;text-transform:uppercase}
button{width:100%;margin-top:10px;background:#238636;color:#fff;border:none;border-radius:8px;padding:11px;font-size:15px;cursor:pointer}
button:disabled{opacity:.5}
#out{margin-top:12px;font-size:12.5px;white-space:pre-wrap;word-break:break-all}</style></head>
<body><div class="box"><h1>Approve a device</h1>
<p>Type the approval code shown on the Ozone-Studio <b>host console</b> (it is never sent over the API). Approving grants this device a session token.</p>
<input id="code" placeholder="approval code" autocomplete="off">
<button id="go" onclick="go()">Approve device</button>
<div id="out"></div></div>
<script>
async function go(){
  const code=document.getElementById('code').value.trim();
  const out=document.getElementById('out'); const btn=document.getElementById('go');
  if(!code){out.textContent='Enter the code from the host console.';return}
  btn.disabled=true; out.textContent='Approving…';
  try{
    const r=await fetch('/pairing/approve',{method:'POST',headers:{'Content-Type':'application/json'},
      body:JSON.stringify({code:code,device_name:'paired device ('+new Date().toLocaleString()+')'})});
    const j=await r.json();
    if(j && j.success){out.textContent='APPROVED — device '+(j.device_id||'')+'. Return to the app: pairing is complete (the app fetches its session via pairing status).';}
    else{out.textContent='Not approved: '+((j && (j.error||j.status))||'unknown response');}
  }catch(e){out.textContent='Request failed: '+e;}
  btn.disabled=false;
}
document.getElementById('code').addEventListener('keydown',e=>{if(e.key==='Enter')go()});
</script></body></html>"#)
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
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(tools.len());
    for t in &tools {
        // real, computed facts (not persisted) — see mcp_supervisor's
        // doc comment for why only android/gcal/zoom have either.
        let device_requirement = crate::mcp_supervisor::device_requirement_for(&state.mcp, &t.name).await;
        let credential_requirement = crate::mcp_supervisor::credential_requirement_for(&state.mcp, &t.name).await;
        let running = match crate::mcp_supervisor::extract_port(&t.endpoint) {
            Some(port) => state.mcp_supervisor.is_running(port).await,
            None => false,
        };
        out.push(serde_json::json!({
            "name": t.name,
            "transport": t.transport.as_str(),
            "endpoint": t.endpoint,
            "capabilities": t.capabilities,
            "server_version": t.server_version,
            "registered_at": t.registered_at,
            "auto_start": t.auto_start,
            "device_requirement": device_requirement,
            "credential_requirement": credential_requirement,
            "running": running,
        }));
    }
    Json(serde_json::json!({ "tools": out }))
}

#[derive(Debug, Deserialize)]
pub struct McpAutoStartRequest {
    pub enabled: bool,
}

/// Toggle `auto_start` for the (transport, endpoint) GROUP `name`
/// belongs to — real persisted config, honored by the boot-time/30s
/// supervisor loop, never actioned synchronously by this route itself
/// (it only flips the flag; the next supervisor tick does the work).
async fn mcp_set_autostart(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(req): Json<McpAutoStartRequest>,
) -> Json<serde_json::Value> {
    match state.mcp.set_auto_start(&name, req.enabled).await {
        Some(affected) => Json(serde_json::json!({
            "success": true,
            "enabled": req.enabled,
            "affected": affected,
        })),
        None => Json(serde_json::json!({
            "success": false,
            "error": format!("tool '{}' not registered", name),
        })),
    }
}

/// Start the process backing `name`'s port right now, bypassing the
/// supervisor's auto_start gate — an explicit operator action (e.g. a
/// "Start now" UI button), still honoring the same real device/
/// credential requirement check so this can't be used to route around
/// an honestly-unmet requirement.
async fn mcp_start_tool(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let Some(tool) = state.mcp.get(&name).await else {
        return Json(serde_json::json!({ "success": false, "error": format!("tool '{}' not registered", name) }));
    };
    let Some(port) = crate::mcp_supervisor::extract_port(&tool.endpoint) else {
        return Json(serde_json::json!({ "success": false, "error": "not an http tool — no port to start" }));
    };
    let Some(facts) = crate::mcp_supervisor::facts_for_port(port) else {
        return Json(serde_json::json!({ "success": false, "error": format!("no known process facts for port {}", port) }));
    };
    if let Some(req) = facts.device_requirement {
        if !crate::mcp_supervisor::device_available(req) {
            return Json(serde_json::json!({ "success": false, "error": format!("device requirement '{}' not met", req) }));
        }
    }
    if let Some(req) = facts.credential_requirement {
        if !crate::mcp_supervisor::credential_available(req) {
            return Json(serde_json::json!({ "success": false, "error": format!("credential requirement '{}' not met", req) }));
        }
    }
    match state.mcp_supervisor.start_port(port, &state.zsei_data_dir).await {
        Ok(started) => Json(serde_json::json!({ "success": true, "started": started })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e })),
    }
}

async fn mcp_stop_tool(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let Some(tool) = state.mcp.get(&name).await else {
        return Json(serde_json::json!({ "success": false, "error": format!("tool '{}' not registered", name) }));
    };
    let Some(port) = crate::mcp_supervisor::extract_port(&tool.endpoint) else {
        return Json(serde_json::json!({ "success": false, "error": "not an http tool — nothing to stop" }));
    };
    let stopped = state.mcp_supervisor.stop_port(port).await;
    Json(serde_json::json!({ "success": true, "stopped": stopped }))
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
            // CONTRACT UNIFICATION (2026-10-18): new tools default to
            // auto_start=false — operator opt-in via /mcp/tools/:name/autostart,
            // never pre-decided by a registration call.
            auto_start: false,
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

// ============================================================================
// MCP USAGE — per-agent daily metering (the MCP budget, enforced host-side)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct McpUsageRecordRequest {
    pub agent: String,
    pub tool: String,
}

/// Record one MCP tool call. Returns the gate decision — an over-limit
/// agent is refused here, not by convention.
async fn mcp_usage_record(
    State(state): State<Arc<AppState>>,
    Json(req): Json<McpUsageRecordRequest>,
) -> Json<serde_json::Value> {
    let (allowed, total_today, limit) = state.mcp_usage.record(&req.agent, &req.tool).await;
    Json(serde_json::json!({
        "allowed": allowed,
        "agent": req.agent,
        "tool": req.tool,
        "total_today": total_today,
        "daily_limit": limit,
    }))
}

/// Per-agent usage snapshot: today's total, limit, remaining, per-tool.
async fn mcp_usage_snapshot(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<McpUsageQuery>,
) -> Json<serde_json::Value> {
    Json(state.mcp_usage.snapshot(&q.agent).await)
}

#[derive(Debug, Deserialize)]
pub struct McpUsageQuery {
    pub agent: String,
}

// UNCOMPILED (fork 6, 2026-10-31): edits in this /mcp/call region are
// pending the build gate — scope mirroring (mcp_call_scope + the
// MirrorRequest/S13 fields below).
/// Workspace/project the CALLER declared in `McpCall.context`
/// (`workspace_id` / `project_id`, JSON number or numeric string — the same
/// u64 ids the rest of the host uses). A project is only meaningful inside
/// a workspace (context_mirror::resolve_scope's rule), so a project with no
/// workspace is dropped here and the call stays global — never a guess.
/// Returns (scope, workspace_id, project_id).
fn mcp_call_scope(
    ctx: Option<&serde_json::Value>,
) -> (&'static str, Option<u64>, Option<u64>) {
    fn id_of(ctx: Option<&serde_json::Value>, key: &str) -> Option<u64> {
        let v = ctx?.get(key)?;
        v.as_u64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse::<u64>().ok()))
    }
    let ws = id_of(ctx, "workspace_id");
    let proj = id_of(ctx, "project_id");
    match (ws, proj) {
        (Some(w), Some(p)) => ("project", Some(w), Some(p)),
        (Some(w), None) => ("workspace", Some(w), None),
        _ => ("global", None, None),
    }
}

/// THE standardized abstract MCP call — every agent, every transport, one
/// shape (see src/mcp.rs McpCall). Metered, gated, monitor-recorded.
async fn mcp_call(
    State(state): State<Arc<AppState>>,
    Json(call): Json<crate::mcp::McpCall>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let registry = runtime.pipeline_registry.read().await;
    let hub = registry.activity_hub();
    let call_tool = call.tool.clone();
    let call_agent = call.agent.clone();
    // CALLER IDENTITY (2026-09-29): validate the session token when
    // provided — the agent name becomes a REAL authenticated identity, and
    // per-role authorization (terminal allowlists, tool gating) keys on it.
    // Absent token = documented localhost posture, identity_validated:false.
    let identity_validated = if let Some(tok) = &call.session_token {
        if !tok.trim().is_empty() {
            let auth_guard = state.runtime.read().await;
            let auth = auth_guard.auth.read().await;
            let token_bytes = hex::decode(tok).unwrap_or_default();
            auth.validate_session(&token_bytes).await.is_ok()
        } else {
            false
        }
    } else {
        false
    };
    let call_input_preview = serde_json::to_string(&call.input)
        .map(|s| s.chars().take(300).collect::<String>())
        .unwrap_or_default();
    let call_input = call.input.clone();
    // Caller-declared scope, read BEFORE `call` is moved into invoke().
    let (call_scope, call_ws, call_proj) = mcp_call_scope(call.context.as_ref());

    // REAL jurisdiction gate for MCP calls — same rules, same matching
    // logic, same RequireConfirmation model review the /orchestrate flow
    // uses (src/orchestrator/jurisdiction.rs), not a fabricated claim and
    // not a second drifting copy of the logic. Built from a real, cheap
    // PromptOrchestrator instance (the same adapters AppRuntime::orchestrate
    // constructs, src/lib.rs) so load_jurisdiction_rules/categorize_
    // jurisdiction_matches/resolve_confirmation_reviews are the identical
    // real functions, not reimplemented here. Runs BEFORE invoke() — a Block
    // match refuses the tool call outright, mirroring Stage 0's own
    // block-before-processing behavior.
    let jurisdiction_result: crate::orchestrator::jurisdiction::JurisdictionGateResult = {
        let base_executor_adapter = Arc::new(crate::orchestrator::RegistryExecutorAdapter {
            registry: runtime.pipeline_registry.clone(),
        });
        let executor_adapter: Arc<dyn crate::orchestrator::PipelineExecutor> =
            Arc::new(crate::orchestrator::decision_review::DecisionReviewExecutor {
                inner: base_executor_adapter,
                available_models: runtime.config.models.available_models.clone(),
                fallback_order: runtime.config.models.fallback.order.clone(),
                fallback_free_only: runtime.config.models.fallback.free_only,
                data_dir: runtime.config.general.data_dir.clone(),
            });
        let zsei_adapter = Arc::new(crate::orchestrator::ZseiStoreAdapter {
            zsei: runtime.zsei.clone(),
        });
        let orchestrator = crate::orchestrator::PromptOrchestrator::new(
            executor_adapter.clone(),
            zsei_adapter,
            runtime.task_manager.clone(),
            Arc::new(tokio::sync::RwLock::new(None)),
            runtime.config.models.context_length as u32,
            runtime.config.jurisdiction.clone(),
            runtime.config.general.data_dir.clone(),
        );

        let region = if runtime.config.jurisdiction.enabled {
            runtime.config.jurisdiction.instance_region.clone()
        } else {
            None
        };
        let rules = orchestrator.load_jurisdiction_rules(region.as_deref()).await;

        // Real haystack: the tool name, agent, and real call input — the
        // same "match real request content against real rule conditions"
        // contract Stage 0 uses on the chat prompt, applied here to what an
        // MCP call actually carries.
        let haystack = format!(
            "{} {} {}",
            call_tool,
            call_agent,
            serde_json::to_string(&call_input).unwrap_or_default()
        )
        .to_lowercase();

        let mut jr = crate::orchestrator::jurisdiction::JurisdictionGateResult {
            rules_loaded: rules.len(),
            ..Default::default()
        };
        let confirmation_matches =
            crate::orchestrator::jurisdiction::categorize_jurisdiction_matches(&rules, &haystack, &mut jr);
        if !confirmation_matches.is_empty() {
            crate::orchestrator::jurisdiction::resolve_confirmation_reviews(
                &executor_adapter,
                &confirmation_matches,
                &haystack,
                0,
                0,
                &mut jr,
                "MCP tool call — no project standing context (not a chat request)",
                "mcp-call",
            )
            .await;
        }
        jr
    };

    if jurisdiction_result.blocked {
        let reason = jurisdiction_result
            .confirmations
            .iter()
            .find(|(_, r)| r.decision == "Decline")
            .map(|(rule, r)| format!("confirmation review declined for \"{}\": {}", rule.condition, r.reasoning))
            .unwrap_or_else(|| "a Block-action jurisdiction rule matched".to_string());
        return Json(serde_json::json!({
            "success": false,
            "output": null,
            "error": format!("MCP call blocked by jurisdiction gate: {}", reason),
            "usage": null,
            "jurisdiction_gate": jurisdiction_result,
        }));
    }

    // ON-DEMAND START (2026-10-21, operator directive: "AMT blueprints
    // should be able to call MCPs on demand"). Before this, `invoke()`
    // just failed outright with "tool unreachable" whenever a registered
    // tool's backing process wasn't already running — confirmed by
    // reading `McpRegistry::invoke()` directly, there was no third
    // pattern beyond `auto_start`'s boot-time pass and the manual
    // "start now" route. This widens WHEN an already-opted-in tool
    // starts (right now, on first use, instead of waiting up to 30s for
    // the periodic sweep or needing a manual action) — it deliberately
    // does NOT widen WHETHER a tool is allowed to start at all. A tool
    // the operator has not set `auto_start=true` for (terminal-mcp as of
    // this writing — a real, presumably deliberate choice to keep an
    // actual command-execution capability off the standing attack
    // surface) stays off exactly as before; a blueprint calling it still
    // gets the same "tool unreachable" failure until the operator opts
    // it in. Honors the exact same device/credential/p2p requirement
    // gates the periodic auto-start sweep already enforces — on-demand
    // start never bypasses a real unmet requirement.
    if let Some(tool) = state.mcp.get(&call.tool).await {
        if tool.auto_start
            && matches!(tool.transport, crate::mcp::McpTransport::Http | crate::mcp::McpTransport::Sse)
        {
            if let Some(port) = crate::mcp_supervisor::extract_port(&tool.endpoint) {
                if !state.mcp_supervisor.is_running(port).await {
                    if let Some(facts) = crate::mcp_supervisor::facts_for_port(port) {
                        let mut requirement_met = true;
                        if let Some(req) = facts.device_requirement {
                            requirement_met &= crate::mcp_supervisor::device_available(req);
                        }
                        if let Some(req) = facts.credential_requirement {
                            requirement_met &= crate::mcp_supervisor::credential_available(req);
                        }
                        if facts.requires_p2p_enabled {
                            requirement_met &= runtime.config.network.enable_p2p;
                        }
                        if requirement_met {
                            match state
                                .mcp_supervisor
                                .start_port(port, &runtime.config.general.data_dir)
                                .await
                            {
                                Ok(_) => {
                                    // Brief best-effort readiness wait — Node
                                    // start is fast but not instant. Not every
                                    // tool implements GET /status; a timeout
                                    // here just falls through to the real
                                    // call below, which will fail honestly if
                                    // the process genuinely isn't up yet.
                                    let status_url = tool.endpoint.replace("/call", "/status");
                                    for _ in 0..25 {
                                        if reqwest::Client::new()
                                            .get(&status_url)
                                            .timeout(std::time::Duration::from_millis(300))
                                            .send()
                                            .await
                                            .is_ok()
                                        {
                                            break;
                                        }
                                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                                    }
                                }
                                Err(e) => tracing::warn!(
                                    "on-demand start of {} (port {port}) failed: {e}",
                                    facts.dir
                                ),
                            }
                        } else {
                            tracing::info!(
                                "on-demand start of {} (port {port}) skipped — a real requirement isn't met",
                                facts.dir
                            );
                        }
                    }
                }
            }
        }
    }

    let result = state
        .mcp
        .invoke(call, &state.mcp_usage, Some(&*hub))
        .await;

    // Real graph ripple for the call — this used to be a bare claim in the
    // "captured" list below with no emission anywhere behind it (found live,
    // 2026-09-28: grepped invoke() end to end, confirmed no ripple ever
    // fired). Mirrors the same way notes/decisions/handoffs/claims already
    // do (context_mirror::mirror → a real CoordinationEvent container →
    // ZSEI::query's CreateContainer choke point → graph_events::emit fires
    // for real) — so this is also now a real, queryable, I1-feed-visible
    // event, not just an honest-but-empty admission.
    let ripple_emitted = {
        let data_dir = runtime.config.general.data_dir.clone();
        let zsei = runtime.zsei.read().await;
        let req = crate::context_mirror::MirrorRequest {
            kind: "tool_call".to_string(),
            agent: call_agent.clone(),
            title: format!("MCP tool call: {}", call_tool),
            body: String::new(),
            files: Vec::new(),
            detail: Some(serde_json::json!({
                "tool": call_tool,
                "success": result.success,
                "error": result.error,
            })),
            // Caller's declared scope (global when none was passed).
            scope: Some(call_scope.to_string()),
            workspace_id: call_ws,
            project_id: call_proj,
        };
        crate::context_mirror::mirror(&zsei, &data_dir, &req).await.is_ok()
    };

    // Graph-native MCP output: a tool's `graph` block becomes real ZSEI
    // containers (containment + typed relations), not just a ripple event.
    let graph_persisted = match result.output.as_ref() {
        Some(out) if result.success => {
            let zsei = runtime.zsei.read().await;
            let data_dir = runtime.config.general.data_dir.clone();
            crate::mcp_graph::persist_from_output(&zsei, &data_dir, &call_tool, &call_agent, out)
                .await
        }
        _ => None,
    };

    // S13 TOOL-CALL CAPTURE (capture unification, 2026-09-28 — operator
    // directive: tool calls are captured alongside pipeline calls, same
    // discipline, same store family): every /mcp/call appends one truthful
    // row — what was called, by whom, the outcome, the input preview.
    // Tool calls had a ledger row (counts) and a graph mirror (event) but
    // no per-call capture store; this closes that asymmetry so "show me
    // every tool call" is one jsonl, same as S10/S11/S12.
    {
        let data_dir = runtime.config.general.data_dir.clone();
        let dir = std::path::PathBuf::from(&data_dir).join("model_calls");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("tool_calls.jsonl");
        let ts = chrono::Utc::now().to_rfc3339();
        let row = serde_json::json!({
            "ts": ts,
            "tool": call_tool,
            "agent": call_agent.clone(),
            "success": result.success,
            "error": result.error.clone().unwrap_or_default(),
            "input_preview": call_input_preview,
            "identity_validated": identity_validated,
            "transport": "mcp/call",
            "scope": call_scope,
            "workspace_id": call_ws,
            "project_id": call_proj,
        });
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            use std::io::Write;
            let _ = writeln!(f, "{}", row);
        }
    }

    // ORDER-LAYER REVIEW (guide §3, Phase 1 — operator-approved): every
    // tool call gets a state-of-the-world reply assembled from real
    // stores, not just an ack. Every field is measured or explicitly
    // absent — the no-fabrication rule, now on the tool surface.
    let review = {
        let data_dir = runtime.config.general.data_dir.clone();

        // maybe_missed: aging decision-review failures (S10) + live claims.
        let dr_path = format!("{}/model_calls/decision_review.jsonl", data_dir);
        let rows: Vec<serde_json::Value> = read_jsonl(&dr_path);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let aging_reviews = rows
            .iter()
            .filter(|r| {
                r.get("decision").and_then(|d| d.as_str()) == Some("review-failed")
                    && r.get("ts")
                        .and_then(|t| t.as_str())
                        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                        .map(|t| now.saturating_sub(t.timestamp() as u64) > 86_400)
                        .unwrap_or(false)
            })
            .count();
        let oz_state = read_ozone_context_state();
        let claims = oz_state.claims.len();
        let _presence = oz_state.sessions.len();

        // order: task queue by native state (same buckets as /order/global).
        let tasks = runtime
            .task_manager
            .read()
            .await
            .list_tasks(None, None, 10_000, 0)
            .await;
        let count = |want: &[&str]| {
            tasks.iter().filter(|t| want.contains(&t.status.as_str())).count()
        };
        let mut maybe_missed = Vec::new();
        if aging_reviews > 0 {
            maybe_missed.push(format!(
                "{} decision review(s) failed and unresolved for over 24h",
                aging_reviews
            ));
        }
        // "jurisdiction gate applied" was a bare claim with nothing behind
        // it — grepped src/mcp.rs and this handler end to end, confirmed
        // zero jurisdiction-related code touched an MCP call. Real as of
        // 2026-09-28: a genuine JurisdictionGateResult is computed above,
        // through the identical real rule-loading/matching/confirmation-
        // review path Stage 0 of /orchestrate uses — so this claim is only
        // made when rules were genuinely loaded and evaluated (rules_loaded
        // is always a real, even-if-zero count; the claim itself only
        // appears once real evaluation happened, which it always does
        // above before this point is reached).
        let mut captured = vec![
            "usage ledger row recorded".to_string(),
            "jurisdiction gate applied".to_string(),
        ];
        // Real, not assumed: only claim the ripple when mirror() really
        // returned Ok (a real container id) above.
        if ripple_emitted {
            captured.push("graph ripple emitted for the call".to_string());
        }
        match &graph_persisted {
            Some(Ok(g)) => captured.push(format!(
                "graph persisted: root {}, {} entities, {} relations",
                g.root_id, g.entities, g.relations
            )),
            Some(Err(e)) => captured.push(format!("graph persistence failed: {e}")),
            None => {}
        }
        serde_json::json!({
            "captured": captured,
            "maybe_missed": maybe_missed,
            "open_claims": claims,
            "identity_validated": identity_validated,
            "order": {
                "live": count(&["running"]),
                "paused": count(&["paused"]),
                "queued": count(&["queued"]),
                "interrupted": count(&["interrupted"]),
            },
        })
    };

    Json(serde_json::json!({
        "success": result.success,
        "output": result.output,
        "error": result.error,
        "usage": result.usage,
        "review": review,
        "jurisdiction_gate": jurisdiction_result,
        "persisted_graph": graph_persisted.as_ref().and_then(|r| r.as_ref().ok()),
    }))
}

// ============================================================================
// CONTEXT MIRROR — coordination events as real ZSEI containers (task 42)
// ============================================================================

/// Mirror one coordination event (note / decision / handoff / finding /
/// file claim) into the /SharedContext graph root. Idempotent for claims
/// (one container per claimed path).
async fn mirror_context(
    State(state): State<Arc<AppState>>,
    Json(req): Json<crate::context_mirror::MirrorRequest>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.read().await;
    let data_dir = runtime.config.general.data_dir.clone();
    let zsei = runtime.zsei.read().await;
    match crate::context_mirror::mirror(&zsei, &data_dir, &req).await {
        Ok(container_id) => {
            let registry = runtime.pipeline_registry.read().await;
            registry.activity_hub().record(
                crate::monitor::ActivityKind::Bridge,
                crate::monitor::ActivityLevel::Info,
                &req.agent,
                format!("mirrored {} into graph: {}", req.kind, req.title),
                Some(serde_json::json!({ "container_id": container_id })),
            );
            Json(serde_json::json!({ "success": true, "container_id": container_id }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "error": e })),
    }
}

// ============================================================================
// COORDINATION — read-only cross-process peek at `.ozone-context/state.json`
// (Batch B: B10/B11). That file is NOT owned by this process — it's written
// by the separate long-running Node MCP server (`tools/ozone-shared-context/
// server.js`) that backs this session's own `mcp__ozone-shared-context__*`
// tools. These two routes are a read-only view of its state, not a
// replacement for it (writes still go through that server's own tools).
// Path resolution mirrors server.js's own exactly (server.js:32): env
// `OZONE_CONTEXT_DIR` if set, else `<process cwd>/.ozone-context`, then
// `/state.json`. Missing/malformed file → honest empty result, not an
// error (the coordination server may not be running yet — a real, valid
// state, not a fault).
// ============================================================================

const OZONE_PRESENCE_TTL_MS: u64 = 5 * 60 * 1000; // server.js PRESENCE_TTL_MS

/// Resolution order: `OZONE_CONTEXT_DIR`, then the nearest ancestor of the
/// host's cwd that actually holds `.ozone-context/state.json`, then
/// `<cwd>/.ozone-context`. The ancestor walk matters because the host is
/// launched from `target/release` while server.js runs from the repo root —
/// resolving against cwd alone made both routes silently return empty.
fn ozone_context_state_path() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("OZONE_CONTEXT_DIR") {
        return std::path::PathBuf::from(dir).join("state.json");
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    for ancestor in cwd.ancestors() {
        let candidate = ancestor.join(".ozone-context").join("state.json");
        if candidate.is_file() {
            return candidate;
        }
    }
    cwd.join(".ozone-context").join("state.json")
}

#[derive(Debug, Deserialize, Default)]
struct OzoneContextSession {
    #[serde(default)]
    role: String,
    #[serde(default)]
    current_files: Vec<String>,
    #[serde(default)]
    task: String,
    #[serde(default)]
    last_seen: u64,
}

#[derive(Debug, Deserialize, Default)]
struct OzoneContextClaim {
    #[serde(default)]
    agent: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    at: u64,
}

#[derive(Debug, Deserialize, Default)]
struct OzoneContextState {
    #[serde(default)]
    sessions: std::collections::HashMap<String, OzoneContextSession>,
    #[serde(default)]
    claims: std::collections::HashMap<String, OzoneContextClaim>,
}

/// Missing file (coordination server never started/hasn't written yet) or
/// malformed JSON both yield an honest default (empty sessions/claims),
/// not an error.
fn read_ozone_context_state() -> OzoneContextState {
    match std::fs::read_to_string(ozone_context_state_path()) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => OzoneContextState::default(),
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Serialize)]
pub struct PresenceEntry {
    pub agent: String,
    pub role: String,
    pub current_files: Vec<String>,
    pub task: String,
    pub last_seen_age_s: u64,
}

/// GET /coordination/presence — B10. Same shape and 300s live-filter as
/// server.js's own `presenceList()` (server.js:214-223).
async fn get_coordination_presence() -> Json<serde_json::Value> {
    let state = read_ozone_context_state();
    let now = now_ms();
    let live: Vec<PresenceEntry> = state
        .sessions
        .into_iter()
        .filter(|(_, s)| now.saturating_sub(s.last_seen) < OZONE_PRESENCE_TTL_MS)
        .map(|(agent, s)| {
            let age_ms = now.saturating_sub(s.last_seen);
            PresenceEntry {
                agent,
                role: s.role,
                current_files: s.current_files,
                task: s.task,
                last_seen_age_s: (age_ms + 500) / 1000, // matches server.js's Math.round
            }
        })
        .collect();
    Json(serde_json::json!({ "live": live, "ttl_seconds": OZONE_PRESENCE_TTL_MS / 1000 }))
}

#[derive(Debug, Serialize)]
pub struct ClaimEntry {
    pub file: String,
    pub agent: String,
    pub reason: String,
    pub age_min: u64,
}

/// GET /coordination/claims — B11. Same shape as server.js's own
/// `fileClaims()` (server.js:276-283). Reads the real current `claims`
/// map from state.json directly — always correctly maintained locally by
/// `file_claim`/`file_release` (the ZSEI-mirror side-channel bug fixed
/// earlier today was in a separate, unrelated code path).
async fn get_coordination_claims() -> Json<serde_json::Value> {
    let state = read_ozone_context_state();
    let now = now_ms();
    let claims: Vec<ClaimEntry> = state
        .claims
        .into_iter()
        .map(|(file, c)| {
            let age_ms = now.saturating_sub(c.at);
            ClaimEntry {
                file,
                agent: c.agent,
                reason: c.reason,
                age_min: (age_ms + 30_000) / 60_000, // matches server.js's Math.round
            }
        })
        .collect();
    Json(serde_json::json!({ "claims": claims }))
}

// ============================================================================
// TASK CREATE — lightweight coordination tasks (shared-context handoffs)
// Real TaskManager records: routed, tracked, listed with everything else.
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CoordinationTaskRequest {
    /// What the assigned agent should do — or, for a personal Universal
    /// Order item (Stage 4 quick-capture: no assignee), the item's name.
    pub prompt: String,
    /// Which agent this is routed to ("zcode", "claude-code", …).
    #[serde(default)]
    pub assignee: Option<String>,
    /// Who created it.
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    pub session_token: String,
    // Universal Order Stage-1 fields (docs/UNIVERSAL_ORDER_GUIDE.md §2) —
    // additive, all optional. Widens this endpoint from agent-coordination-
    // only to also serve the quick-capture UI's personal todo/note/meeting
    // items, reusing the same enqueue_task→inputs path Stage 1 already
    // reads these keys from (src/task/mod.rs:982-985) rather than adding a
    // parallel creation path.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub due_at: Option<u64>,
    #[serde(default)]
    pub remind_at: Option<u64>,
    #[serde(default)]
    pub recurrence: Option<String>,
    #[serde(default)]
    pub meeting_url: Option<String>,
    #[serde(default)]
    pub note_body: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<u64>,
    #[serde(default)]
    pub project_id: Option<u64>,
}

async fn create_coordination_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CoordinationTaskRequest>,
) -> Json<serde_json::Value> {
    let priority = match req.priority.as_deref() {
        Some("high") => crate::task::TaskPriority::High,
        Some("low") => crate::task::TaskPriority::Low,
        _ => crate::task::TaskPriority::Normal,
    };

    let mut inputs = std::collections::HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!(req.prompt));
    inputs.insert(
        "source".to_string(),
        serde_json::json!(req.created_by.as_deref().unwrap_or("shared-context")),
    );
    if let Some(a) = &req.assignee {
        inputs.insert("assignee".to_string(), serde_json::json!(a));
    }
    if let Some(k) = &req.kind {
        inputs.insert("kind".to_string(), serde_json::json!(k));
    }
    if let Some(d) = req.due_at {
        inputs.insert("due_at".to_string(), serde_json::json!(d));
    }
    if let Some(r) = req.remind_at {
        inputs.insert("remind_at".to_string(), serde_json::json!(r));
    }
    if let Some(r) = &req.recurrence {
        inputs.insert("recurrence".to_string(), serde_json::json!(r));
    }
    if let Some(m) = &req.meeting_url {
        inputs.insert("meeting_url".to_string(), serde_json::json!(m));
    }
    if let Some(n) = &req.note_body {
        inputs.insert("note_body".to_string(), serde_json::json!(n));
    }

    let enqueue_result = {
        let runtime = state.runtime.write().await;
        let task_mgr = runtime.task_manager.read().await;
        task_mgr
            .enqueue_task(
                None, // no blueprint — coordination task, picked up by an agent
                inputs,
                0, // system user
                0, // system device
                req.workspace_id,
                req.project_id,
                priority,
            )
            .await
    }; // write guard dropped here — the feed record below takes a read lock

    match enqueue_result {
        Ok(task_id) => {
            let runtime = state.runtime.read().await;
            let registry = runtime.pipeline_registry.read().await;
            registry.activity_hub().record(
                crate::monitor::ActivityKind::Job,
                crate::monitor::ActivityLevel::Info,
                req.created_by.as_deref().unwrap_or("shared-context"),
                format!(
                    "Task {} created for {} — {}",
                    task_id,
                    req.assignee.as_deref().unwrap_or("any agent"),
                    truncate_for_feed(&req.prompt, 90)
                ),
                None,
            );
            Json(serde_json::json!({
                "success": true,
                "task_id": task_id,
                "assignee": req.assignee,
                "status": "queued",
            }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

fn truncate_for_feed(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(max)])
    }
}

/// Coordination-task lifecycle: the routed agent marks its own work
/// queued → completed/failed. Only touches /task/create tasks (source-
/// tagged) — host-executed orchestration tasks keep their own lifecycle.
async fn update_coordination_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CoordinationTaskUpdateRequest>,
) -> Json<serde_json::Value> {
    let runtime = state.runtime.write().await;
    let result = runtime
        .task_manager
        .read()
        .await
        .update_coordination_status(&req.task_id, &req.status, req.error.clone())
        .await;
    drop(runtime);

    match result {
        Ok(true) => {
            let runtime = state.runtime.read().await;
            let registry = runtime.pipeline_registry.read().await;
            registry.activity_hub().record(
                crate::monitor::ActivityKind::Job,
                crate::monitor::ActivityLevel::Info,
                req.agent.as_deref().unwrap_or("agent"),
                format!("Task {} → {}", req.task_id, req.status),
                None,
            );
            Json(serde_json::json!({ "success": true, "task_id": req.task_id, "status": req.status }))
        }
        Ok(false) => Json(serde_json::json!({
            "success": false,
            "error": "task not found or not a coordination task",
        })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
pub struct CoordinationTaskUpdateRequest {
    pub task_id: u64,
    /// completed | failed | queued (requeue)
    pub status: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
}
