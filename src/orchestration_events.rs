//! Orchestration stage event ripple — real, live visibility into the
//! 15-stage `/orchestrate` pipeline while a request is in flight.
//!
//! `/orchestrate` is a single blocking HTTP call (per the operator's
//! explicit decision to keep the pipeline's full zero-shot depth exactly as
//! it is — this is not a shortcut around that, just a window into it). The
//! existing `pipeline_progress` WS stream (src/pipeline/executor.rs) already
//! surfaces every subprocess pipeline invocation (TextAnalysisPipeline,
//! WorkspaceTab, the prompt pipeline, ...), but several real stages (Build
//! AMT, Blueprint Assignment, Zero-Shot Simulation) run as direct in-process
//! orchestrator calls with no subprocess execution to track — they had no
//! live signal at all. This hub closes that gap at the one real choke point
//! every stage already passes through: `record_stage`/`record_stage_timed`
//! (src/orchestrator/mod.rs), which already builds the exact `StageResult`
//! that `/task/get` and `OrchestrationResponse.stages_completed` serve after
//! the fact — this just broadcasts the same real data as it happens.
//!
//! Scoping: by `(user_id, device_id)`, both already known to the client
//! that sent the `/orchestrate` request (it's who's asking) — no new
//! per-request id needed for this single-conversation-at-a-time UI.

use std::sync::OnceLock;
use tokio::sync::broadcast;

#[derive(Debug, Clone, serde::Serialize)]
pub struct OrchestrationStageEvent {
    pub user_id: u64,
    pub device_id: u64,
    /// Scope the triggering `/orchestrate` request actually carried
    /// (`OrchestrationRequest.project_id`/`workspace_id`, both already real
    /// fields — "Optional project context (scoped chat)"). None/None means
    /// the prompt came from the global chat, not a project-scoped one. A
    /// live UI consumer needs this to know whether a stage's activity
    /// belongs to a specific project's own view or the global one — the
    /// prior wire frame only carried (user_id, device_id), which can't make
    /// that distinction since both chat scopes share the same user/device.
    pub project_id: Option<u64>,
    pub workspace_id: Option<u64>,
    pub stage: u8,
    pub stage_name: String,
    pub success: bool,
    /// Real dynamic summary text already produced by the stage itself (e.g.
    /// "Chunks: 1, ChunkGraphs: 1, ...", "Methodologies: 3, Categories: 1
    /// (1 created)") — never a fabricated placeholder.
    pub summary: String,
    /// 0 for stages recorded via the untimed `record_stage` path.
    pub duration_ms: u64,
    pub timestamp: u64,
}

static HUB: OnceLock<OrchestrationEventHub> = OnceLock::new();

#[derive(Clone)]
pub struct OrchestrationEventHub {
    tx: broadcast::Sender<std::sync::Arc<OrchestrationStageEvent>>,
}

impl OrchestrationEventHub {
    pub fn global() -> &'static OrchestrationEventHub {
        HUB.get_or_init(|| {
            let (tx, _rx) = broadcast::channel(256);
            OrchestrationEventHub { tx }
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<std::sync::Arc<OrchestrationStageEvent>> {
        self.tx.subscribe()
    }

    fn publish(&self, evt: OrchestrationStageEvent) {
        // No active subscribers is the common case outside an open UI
        // session — not an error, nothing to log.
        let _ = self.tx.send(std::sync::Arc::new(evt));
    }
}

/// Real-location pipeline progress — a DIFFERENT, finer granularity than
/// `OrchestrationStageEvent` above (2026-10-18, additive). `stage_name`
/// there is phase-completion telemetry ("CreatingGraphs", etc.) from only 3
/// call sites in the whole orchestrator; it has no room for "what specific
/// file/object is being touched right now". This event exists to carry
/// exactly that, from a NEW, separate hook point: `executor.rs`'s existing
/// stderr-drain loop (`run_child_bounded`'s `drain()`), which already
/// forwards every complete stderr line from a running pipeline subprocess
/// to `tracing::info!` in real time — this taps the SAME lines, in
/// addition to (never instead of) that existing forwarding, so nothing
/// about current logging behavior changes.
///
/// IMPORTANT — infrastructure only, no real producer yet: as of this
/// addition, no pipeline in this codebase actually emits a structured
/// progress line in the format this expects (checked directly: the `code`
/// modality pipeline is read-only analysis; `code_analysis` has no
/// file-writing action either; no dedicated "code modification" pipeline
/// exists anywhere — code changes in this system do not currently flow
/// through a traceable pipeline call at all). This event type and its
/// executor-side detection are real and ready, but will see zero real
/// traffic until a producer is built — do not treat its mere existence as
/// evidence the gap it targets is closed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineLocationEvent {
    pub execution_id: String,
    pub task_id: Option<u64>,
    pub file_path: String,
    pub operation: String,
    pub timestamp: u64,
}

static LOCATION_HUB: OnceLock<PipelineLocationEventHub> = OnceLock::new();

#[derive(Clone)]
pub struct PipelineLocationEventHub {
    tx: broadcast::Sender<std::sync::Arc<PipelineLocationEvent>>,
}

impl PipelineLocationEventHub {
    pub fn global() -> &'static PipelineLocationEventHub {
        LOCATION_HUB.get_or_init(|| {
            let (tx, _rx) = broadcast::channel(256);
            PipelineLocationEventHub { tx }
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<std::sync::Arc<PipelineLocationEvent>> {
        self.tx.subscribe()
    }
}

/// Called from `executor.rs`'s stderr-drain loop when a line matches the
/// structured progress format — a plain, synchronous fn (not async) since
/// `broadcast::Sender::send` is sync and the drain loop runs on a raw
/// `std::thread`, not inside a tokio task.
pub fn emit_location(execution_id: &str, task_id: Option<u64>, file_path: &str, operation: &str) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let evt = PipelineLocationEvent {
        execution_id: execution_id.to_string(),
        task_id,
        file_path: file_path.to_string(),
        operation: operation.to_string(),
        timestamp,
    };
    // No active subscribers is the common case — not an error, nothing to log.
    let _ = PipelineLocationEventHub::global()
        .tx
        .send(std::sync::Arc::new(evt));
}

/// Real per-node/per-edge graph-CONSTRUCTION visibility — a FOURTH,
/// separate granularity from `OrchestrationStageEvent` (phase-level) and
/// `PipelineLocationEvent` (file-level, no producer yet). Scoped by
/// `project_id` alone, not `(user_id, device_id)`: unlike an `/orchestrate`
/// request, a modality pipeline action (e.g. 3D's `ComputeSpatial`) never
/// receives user_id/device_id in its own input at all — only the
/// `project_id` already stored on the graph itself (confirmed:
/// `ThreeDGraph.project_id`) — and `project_id` alone already matches how
/// this session's `liveEngineActivity` frontend store is scoped, so no
/// user/device identity is needed.
///
/// Emitted over HTTP (`POST /orchestration/node-event`, `src/grpc/mod.rs`)
/// — NOT called in-process like `emit` above — because the real producer
/// runs in a separate OS process: each modality pipeline is a subprocess
/// spawned fresh per call (`src/pipeline/executor.rs`), with no access to
/// this host process's in-memory hub. Same established pattern pipelines
/// already use to reach back into the host for anything they can't do
/// themselves (`file_link`/`url_link`/`package_link`'s `zsei_query()`
/// POSTing to `/zsei/query`).
///
/// First real producer (2026-10-18): 3D's `HeadlessProcess`/`ComputeSpatial`
/// op (`assets/pipelines/modalities/3D/main.rs`) fires one best-effort,
/// fire-and-forget POST per spatial edge it adds, inside its existing
/// per-relationship loop — a genuinely new capability, no existing 3D
/// behavior touched beyond that one additive call.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrchestrationNodeEvent {
    pub project_id: Option<u64>,
    pub modality: String,
    pub graph_id: u64,
    /// The node this event is primarily about.
    pub node_id: u64,
    /// Set when `operation` connects two nodes (e.g. a new edge) — the
    /// other endpoint. `None` for an event about a single node only.
    pub related_node_id: Option<u64>,
    /// The node's real type, OR the edge type name when this event
    /// describes a relationship (e.g. `"SpatialAbove"`).
    pub node_type: String,
    /// e.g. `"node_created"`, `"edge_added"` — real, modality-chosen text,
    /// never a fabricated placeholder.
    pub operation: String,
    pub timestamp: u64,
}

static NODE_HUB: OnceLock<NodeEventHub> = OnceLock::new();

#[derive(Clone)]
pub struct NodeEventHub {
    tx: broadcast::Sender<std::sync::Arc<OrchestrationNodeEvent>>,
}

impl NodeEventHub {
    pub fn global() -> &'static NodeEventHub {
        NODE_HUB.get_or_init(|| {
            let (tx, _rx) = broadcast::channel(256);
            NodeEventHub { tx }
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<std::sync::Arc<OrchestrationNodeEvent>> {
        self.tx.subscribe()
    }
}

/// Called from the new `POST /orchestration/node-event` HTTP handler
/// (`src/grpc/mod.rs`) — the real entry point for pipeline subprocesses,
/// which cannot reach this process's in-memory hub directly.
pub fn emit_node_event(
    project_id: Option<u64>,
    modality: &str,
    graph_id: u64,
    node_id: u64,
    related_node_id: Option<u64>,
    node_type: &str,
    operation: &str,
) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let evt = OrchestrationNodeEvent {
        project_id,
        modality: modality.to_string(),
        graph_id,
        node_id,
        related_node_id,
        node_type: node_type.to_string(),
        operation: operation.to_string(),
        timestamp,
    };
    // No active subscribers is the common case — not an error, nothing to log.
    let _ = NodeEventHub::global().tx.send(std::sync::Arc::new(evt));
}

/// Called from `record_stage`/`record_stage_timed` — the two real,
/// already-existing choke points every stage passes through.
pub fn emit(
    user_id: u64,
    device_id: u64,
    project_id: Option<u64>,
    workspace_id: Option<u64>,
    stage: u8,
    stage_name: &str,
    success: bool,
    summary: &str,
    duration_ms: u64,
) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    OrchestrationEventHub::global().publish(OrchestrationStageEvent {
        user_id,
        device_id,
        project_id,
        workspace_id,
        stage,
        stage_name: stage_name.to_string(),
        success,
        summary: summary.to_string(),
        duration_ms,
        timestamp,
    });
}
