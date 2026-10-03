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

/// Called from `record_stage`/`record_stage_timed` — the two real,
/// already-existing choke points every stage passes through.
pub fn emit(
    user_id: u64,
    device_id: u64,
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
        stage,
        stage_name: stage_name.to_string(),
        success,
        summary: summary.to_string(),
        duration_ms,
        timestamp,
    });
}
