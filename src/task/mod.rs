//! Task Management System - v0.4.0
//!
//! Based on Sections 11-12 of the specification.
//!
//! All pipeline executions are tracked as tasks. Tasks provide:
//! - Execution state tracking
//! - Progress monitoring
//! - Error handling
//! - History and logging
//! - Queue-based execution
//! - Consciousness integration
//! - I-Loop protection
//! - Refinement daemon (meta loop)
//!
//! CRITICAL: Tasks MUST wait for I-Loop to complete before starting.
//! I-Loop is NOT front-run by tasks.
//!
//! QUEUE ARCHITECTURE:
//! - Tasks are added to queue via `enqueue_task()`
//! - Queue processor runs continuously
//! - Tasks execute in order with priority support
//! - Concurrent execution limit is configurable
//!
//! REFINEMENT DAEMON (Meta Loop):
//! - Runs every 24 hours (configurable)
//! - Decomposes complex methodologies
//! - Identifies new sub-categories
//! - Detects emerging modalities
//! - Cross-references and deduplicates

use crate::types::{
    DeviceID, LogEntry, LogLevel, OzoneError, OzoneResult, PipelineID, TaskExecutionState, TaskID, UserID,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Task queue configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskQueueConfig {
    /// Maximum concurrent tasks
    pub max_concurrent: usize,
    /// Maximum queued tasks
    pub max_queued: usize,
    /// Queue check interval in milliseconds
    pub check_interval_ms: u64,
    /// I-Loop wait timeout in milliseconds
    pub i_loop_timeout_ms: u64,
    /// DEPRECATED / IGNORED (C1, 2026-10-23): the task manager now observes
    /// the static `CONSCIOUSNESS_STORE`, which is gated by its own
    /// `config.enabled` (copied from config.toml at boot). The field is kept
    /// only so existing constructors (lib.rs) and tests still compile.
    pub consciousness_enabled: bool,
    /// DEPRECATED / IGNORED: the store uses `OZONE_CONSCIOUSNESS_PATH`
    /// (default `./zsei_data/consciousness`). Kept for compatibility.
    pub consciousness_path: String,
    /// Task storage path
    pub storage_path: String,
    /// Task persistence backend name ("json_file" default; swappable — see
    /// task/store.rs, the K-registry store family).
    #[serde(default)]
    pub store_backend: String,
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            max_queued: 100,
            check_interval_ms: 100,
            i_loop_timeout_ms: 30000,
            consciousness_enabled: false,
            consciousness_path: "./zsei_data/consciousness".to_string(),
            storage_path: "./zsei_data/tasks".to_string(),
            store_backend: "json_file".to_string(),
        }
    }
}

/// Refinement daemon configuration
#[derive(Debug, Clone)]
pub struct RefinementConfig {
    /// Interval between runs in seconds (default: 24 hours)
    pub interval_secs: u64,
    /// Maximum methodologies per category before split
    pub max_methodologies_per_category: usize,
    /// Maximum principles per methodology before split
    pub max_principles_per_methodology: usize,
    /// Enable automatic refinement
    pub enabled: bool,
}

impl Default for RefinementConfig {
    fn default() -> Self {
        Self {
            interval_secs: 86400, // 24 hours
            max_methodologies_per_category: 10,
            max_principles_per_methodology: 5,
            enabled: true,
        }
    }
}

// ============================================================================
// CONSCIOUSNESS HOOKS
// ============================================================================

pub mod store;

/// Task-manager view of consciousness. HISTORY: this module used to be a
/// complete second consciousness implementation (file-based gate, experience
/// log, emotional state and perception window under `consciousness_path`),
/// separate from `crate::consciousness::store::CONSCIOUSNESS_STORE` — two
/// implementations writing the same directory in incompatible formats, with
/// nothing feeding either. OPERATOR DECISION (C1, 2026-10-23): the static
/// store is THE implementation and the task manager is an OBSERVER of it
/// ("consciousness is the window from the task manager into the whole
/// system"). Every unique behavior of the old hooks was ported into the store
/// first (sensitive-data gate terms, persisted perception window, gate-decision
/// persistence, I-Loop wait, token count on experiences, one-time migration of
/// the old file formats); what remains here are thin delegates with the old
/// signatures (so nothing that called them breaks) plus `GateDecision`, which
/// is persisted in task records and exposed through `TaskData::gate_result`.
///
/// OBSERVE-ONLY: nothing here blocks or fails a task. The old
/// `enqueue_task` path REJECTED a task when the hook gate declined; that
/// blocking is intentionally gone from the task manager — the stage-8 model
/// gate and jurisdiction own blocking, and a decline at enqueue made
/// `amt_loop` (which uses `.ok()`) silently lose its task.
///
/// The `consciousness_path` arguments are kept for signature compatibility and
/// ignored; the store's own directory (`OZONE_CONSCIOUSNESS_PATH`) is used.
pub mod consciousness_hooks {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GateDecision {
        pub proceed: bool,
        pub decision: String, // "Proceed", "Modify", "Decline", "Defer"
        pub confidence: f32,
        pub reasoning: String,
        pub modifications: Vec<String>,
    }

    /// True while the I-Loop is mid-cycle (in-process flag or the compat file).
    pub fn is_i_loop_active(_consciousness_path: &str) -> bool {
        crate::consciousness::store::i_loop_busy()
    }

    /// Wait (bounded) for the I-Loop to finish before registering work.
    pub async fn wait_for_i_loop_completion(_consciousness_path: &str, timeout_ms: u64) {
        crate::consciousness::store::wait_for_i_loop_completion(timeout_ms).await;
    }

    /// Evaluate a task through the store's gate. INFORMATIONAL for the task
    /// manager (it records the result on the task, never acts on a decline).
    pub fn pre_task_gate(
        _consciousness_path: &str,
        task_summary: &str,
        user_id: u64,
        _blueprint_id: Option<u64>,
    ) -> GateDecision {
        gate_for_task(0, task_summary, user_id)
    }

    /// Same as `pre_task_gate`, but records the REAL task id on the store's
    /// gate-decision history (the old hook record carried no task id).
    pub fn gate_for_task(task_id: u64, task_summary: &str, user_id: u64) -> GateDecision {
        let rec = crate::consciousness::store::pre_task_gate(task_id, task_summary, user_id);
        let proceed = rec.decision == "proceed";
        GateDecision {
            proceed,
            decision: if proceed { "Proceed" } else { "Decline" }.to_string(),
            confidence: rec.confidence,
            reasoning: rec.reasoning,
            modifications: vec![],
        }
    }

    /// Record a task outcome in the store (experience + emotion). `error` is
    /// the failure cause; `update_emotion = false` for loop-generated tasks.
    pub fn post_task_experience(
        _consciousness_path: &str,
        task_id: u64,
        task_summary: &str,
        success: bool,
        user_id: Option<u64>,
        tokens_used: Option<u32>,
        error: Option<&str>,
        update_emotion: bool,
    ) {
        let _ = crate::consciousness::store::post_task_experience_with(
            task_id,
            task_summary,
            success,
            user_id,
            error,
            tokens_used,
            update_emotion,
        );
    }

    /// Report a task lifecycle event to the store's perception window.
    pub fn add_to_perception_window(
        _consciousness_path: &str,
        task_id: u64,
        task_type: &str,
        status: &str,
    ) {
        crate::consciousness::store::observe_task_event(task_id, task_type, status);
    }

    /// Loop-generated bookkeeping tasks (amt_loop registers one per candidate;
    /// 472 of 550 failed on the local host). They are still recorded as
    /// experiences, but must not move the emotional state — otherwise loop
    /// noise pins valence at -1.0 permanently — and they get no per-task gate.
    pub fn is_loop_task_inputs(inputs: &HashMap<String, serde_json::Value>) -> bool {
        inputs.get("source").and_then(|v| v.as_str()) == Some("amt-loop")
            || inputs.get("kind").and_then(|v| v.as_str()) == Some("loop-pass")
    }

    /// Same test for a stored task's `inputs` value.
    pub fn is_loop_task_value(inputs: Option<&serde_json::Value>) -> bool {
        match inputs {
            Some(i) => {
                i.get("source").and_then(|v| v.as_str()) == Some("amt-loop")
                    || i.get("kind").and_then(|v| v.as_str()) == Some("loop-pass")
            }
            None => false,
        }
    }

    /// Human-readable summary of a task's inputs: prompt, else description,
    /// else name (the old lookup read only `prompt`, so tasks created with a
    /// description were all recorded as "Task").
    pub fn summarize_inputs(inputs: Option<&serde_json::Value>) -> String {
        inputs
            .and_then(|i| {
                i.get("prompt")
                    .and_then(|p| p.as_str())
                    .or_else(|| i.get("description").and_then(|p| p.as_str()))
                    .or_else(|| i.get("name").and_then(|p| p.as_str()))
            })
            .unwrap_or("Task")
            .to_string()
    }
}

// ============================================================================
// TASK TYPES
// ============================================================================

/// Queued task entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedTask {
    pub task_id: TaskID,
    pub priority: TaskPriority,
    pub created_at: u64,
    pub blueprint_id: Option<u64>,
    pub user_id: UserID,
    pub device_id: DeviceID,
    pub workspace_id: Option<u64>,
    pub project_id: Option<u64>,
    pub inputs: HashMap<String, serde_json::Value>,
    pub task_summary: String,
    /// Copy of the task's `due_at` (unix secs) — queue tie-break only.
    /// `serde(default)` keeps previously persisted queues loadable.
    #[serde(default)]
    pub due_at: Option<u64>,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// TASK-LIFECYCLE RIPPLE (R5 top gap, fixed 2026-09-28 — operator
/// architecture: the universal task order is ALIVE, so task state changes
/// move the live graph like every other write). Zero graph coverage
/// existed before: create/complete/fail/pause were invisible to ws
/// subscribers, the monitor feed, and AMT ripple-sync. Scopes follow the
/// one vocabulary: ws:<id> / proj:<id>, global fallback.
fn emit_task_ripple(
    event: &'static str,
    task_id: TaskID,
    workspace_id: Option<u64>,
    project_id: Option<u64>,
) {
    let mut scope = Vec::new();
    if let Some(w) = workspace_id {
        scope.push(format!("ws:{}", w));
    }
    if let Some(p) = project_id {
        scope.push(format!("proj:{}", p));
    }
    if scope.is_empty() {
        scope.push("scope:global".to_string());
    }
    crate::graph_events::emit(
        event,
        task_id,
        workspace_id.unwrap_or(0),
        "Task".to_string(),
        "task-system",
        scope,
    );
    // Personal assistant wake (docs/PERSONAL_ASSISTANT_GUIDE.md §4): a real
    // task lifecycle event is exactly what its check-up pass exists for —
    // wake it instantly, interval is the fallback (amt_loop's convention).
    crate::consciousness::assistant::notify_wake();
}

/// Extended task data with full details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskData {
    pub task_id: TaskID,
    pub blueprint_id: Option<u64>,
    pub status: String,
    pub progress: f32,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub user_id: UserID,
    pub device_id: DeviceID,
    pub workspace_id: Option<u64>,
    pub project_id: Option<u64>,
    pub parent_task_id: Option<TaskID>,
    pub child_count: u32,
    pub error: Option<String>,
    pub inputs: Option<serde_json::Value>,
    pub outputs: Option<serde_json::Value>,
    pub steps: Vec<TaskStepData>,
    pub total_tokens: u32,
    pub gate_result: Option<consciousness_hooks::GateDecision>,
    #[serde(default)]
    pub thinking_log: Vec<serde_json::Value>,
    /// Real AMT structure (branches/relationships/verification) for this
    /// task — see orchestrator::AMTSummary. Set once via `set_amt_summary`
    /// after orchestration completes, same convention as thinking_log.
    #[serde(default)]
    pub amt_summary: Option<serde_json::Value>,
    // ── UNIVERSAL ORDER Stage 1 (docs/UNIVERSAL_ORDER_GUIDE.md §2) ──
    // Additive item-model fields: everything-is-one (todos, meetings,
    // follow-ups, notes, code work, external syncs) in the same order.
    // All serde-defaulted — zero migration, every existing record loads.
    /// What kind of order-item this is: todo | meeting | followup | note |
    /// code | milestone | external. Default "todo".
    #[serde(default)]
    pub kind: String,
    /// The WHEN axis — unix secs. Drives computed time buckets (overdue /
    /// today / week / upcoming) at read time; never a stored bucket.
    #[serde(default)]
    pub due_at: Option<u64>,
    /// Follow-up nudge time (unix secs).
    #[serde(default)]
    pub remind_at: Option<u64>,
    /// Recurrence rule for repeating items ("daily"/"weekly"/... simple
    /// vocabulary; expansion to concrete due_at is derived at read).
    #[serde(default)]
    pub recurrence: Option<String>,
    /// Meeting join link (zoom/meet/jitsi) for meeting-kind items.
    #[serde(default)]
    pub meeting_url: Option<String>,
    /// External provenance when synced from a connector (google-calendar,
    /// zoom, …): {provider, external_id, last_synced}. Idempotent sync
    /// keys on external_id; the order never stores foreign schemas.
    #[serde(default)]
    pub external_ref: Option<serde_json::Value>,
    /// Sub-items on any task: [{text, done}].
    #[serde(default)]
    pub checklist: Vec<serde_json::Value>,
    /// Note body for kind "note" — captured thoughts; promote by adding
    /// due_at / converting to todo.
    #[serde(default)]
    pub note_body: Option<String>,
}

/// Task step data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStepData {
    pub step_index: u32,
    pub action: String,
    pub pipeline_id: PipelineID,
    pub status: String,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub tokens_used: u32,
    pub output_summary: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub stages_completed: Vec<String>,
    #[serde(default)]
    pub stages_pending: Vec<String>,
    #[serde(default)]
    pub current_stage: Option<String>,
    #[serde(default)]
    pub graph_ids_read: Vec<String>,
    #[serde(default)]
    pub graph_ids_updated: Vec<String>,
    #[serde(default = "default_step_version")]
    pub version: u32,
    #[serde(default)]
    pub version_notes: Vec<StepVersionNote>,
    #[serde(default)]
    pub methodology_ids_applied: Vec<u64>,
    /// Which model actually served this step (e.g. "bitnet:ggml-model-i2_s",
    /// "claude-3.5-sonnet") — lets the UI show a switch marker when
    /// consecutive steps used different backends.
    #[serde(default)]
    pub model_used: Option<String>,
    /// Context-object mirror (see StoredTaskStep.context_assembled) — the
    /// assembled context this step received and how it was gathered.
    #[serde(default)]
    pub context_assembled: Option<String>,
    #[serde(default)]
    pub context_sources: Vec<String>,
}

/// Timeline event for task history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub details: Option<String>,
}

/// Task comparison entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskComparison {
    pub task_id: TaskID,
    pub blueprint_id: Option<u64>,
    pub status: String,
    pub duration_secs: Option<u64>,
    pub total_tokens: u32,
    pub created_at: u64,
}

// ============================================================================
// STORED TASK (PERSISTENCE)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredTask {
    task_id: TaskID,
    blueprint_id: Option<u64>,
    status: String,
    progress: f32,
    created_at: u64,
    started_at: Option<u64>,
    completed_at: Option<u64>,
    user_id: UserID,
    device_id: DeviceID,
    workspace_id: Option<u64>,
    project_id: Option<u64>,
    parent_task_id: Option<TaskID>,
    inputs: Option<serde_json::Value>,
    outputs: Option<serde_json::Value>,
    steps: Vec<StoredTaskStep>,
    total_tokens: u32,
    error: Option<String>,
    gate_result: Option<consciousness_hooks::GateDecision>,
    /// Full "thinking cycle" for this task — one entry per real LLM call the
    /// run made (AMT-building passes, blueprint drafting, zero-shot
    /// simulation, step execution), each carrying the FULL raw response
    /// text. Set once via `set_thinking_log` after orchestration completes
    /// (task creation happens mid-run, before the log is complete) — see
    /// PromptOrchestrator::orchestrate in src/orchestrator/mod.rs. Empty for
    /// tasks created before this existed.
    #[serde(default)]
    thinking_log: Vec<serde_json::Value>,
    /// Real AMT structure for this task — see TaskData's field of the same
    /// name for the full explanation. Same set-once-after-completion
    /// convention as thinking_log. Empty for tasks created before this
    /// existed.
    #[serde(default)]
    amt_summary: Option<serde_json::Value>,
    // ── UNIVERSAL ORDER Stage 1 (docs/UNIVERSAL_ORDER_GUIDE.md §2) ──
    // Additive, all defaulted: tasks created before these existed load
    // with the documented defaults (kind "todo", no time, no externals).
    #[serde(default)]
    kind: String,
    #[serde(default)]
    due_at: Option<u64>,
    #[serde(default)]
    remind_at: Option<u64>,
    #[serde(default)]
    recurrence: Option<String>,
    #[serde(default)]
    meeting_url: Option<String>,
    #[serde(default)]
    external_ref: Option<serde_json::Value>,
    #[serde(default)]
    checklist: Vec<serde_json::Value>,
    #[serde(default)]
    note_body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTaskStep {
    step_index: u32,
    action: String,
    pipeline_id: PipelineID,
    status: String,
    started_at: Option<u64>,
    completed_at: Option<u64>,
    tokens_used: u32,
    output_summary: Option<String>,
    error: Option<String>,
    // ── Extended step metadata (orchestrator step model) ──
    #[serde(default)]
    stages_completed: Vec<String>,
    #[serde(default)]
    stages_pending: Vec<String>,
    #[serde(default)]
    current_stage: Option<String>,
    #[serde(default)]
    graph_ids_read: Vec<String>,
    #[serde(default)]
    graph_ids_updated: Vec<String>,
    #[serde(default = "default_step_version")]
    version: u32,
    #[serde(default)]
    version_notes: Vec<StepVersionNote>,
    #[serde(default)]
    methodology_ids_applied: Vec<u64>,
    #[serde(default)]
    model_used: Option<String>,
    /// CONTEXT OBJECT (the "what did this step actually see" record —
    /// persistent, loggable, API-obtainable; never write-only):
    /// the assembled context text this step was given, plus provenance of
    /// how it was gathered ("keyword-scan" today; "traversal:<mode>" once
    /// the TraversalEngine is wired into assembly).
    #[serde(default)]
    context_assembled: Option<String>,
    #[serde(default)]
    context_sources: Vec<String>,
}

/// One versioned change record on a step (metrics, not fabricated scores —
/// notes carry measured facts like token counts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepVersionNote {
    pub version: u32,
    pub note: String,
    pub timestamp: u64,
    pub change_type: String,
}

fn default_step_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskStoreData {
    tasks: HashMap<TaskID, StoredTask>,
    logs: HashMap<TaskID, Vec<LogEntry>>,
    next_id: TaskID,
}

// ============================================================================
// TASK MANAGER
// ============================================================================

/// Task manager - tracks all task executions with queue support
pub struct TaskManager {
    /// Configuration
    config: TaskQueueConfig,

    /// Refinement daemon configuration
    refinement_config: RefinementConfig,

    /// Task storage
    tasks: Arc<RwLock<HashMap<TaskID, StoredTask>>>,

    /// Task logs
    logs: Arc<RwLock<HashMap<TaskID, Vec<LogEntry>>>>,

    /// Task execution states
    execution_states: Arc<RwLock<HashMap<TaskID, TaskExecutionState>>>,

    /// Task queue (priority queue)
    queue: Arc<RwLock<VecDeque<QueuedTask>>>,

    /// Currently running tasks
    running: Arc<RwLock<Vec<TaskID>>>,

    /// Next task ID
    next_id: Arc<RwLock<TaskID>>,

    /// Storage path
    storage_path: String,

    /// Persistence backend (K-registry store family — swappable).
    backend: std::sync::Arc<dyn store::TaskStoreBackend>,

    /// Queue processor running flag
    queue_running: Arc<RwLock<bool>>,

    /// Refinement daemon running flag
    refinement_running: Arc<RwLock<bool>>,

    /// Last refinement run timestamp
    last_refinement: Arc<RwLock<u64>>,
}

impl TaskManager {
    /// Create new task manager
    pub fn new(config: TaskQueueConfig, refinement_config: RefinementConfig) -> OzoneResult<Self> {
        let storage_path = config.storage_path.clone();
        let backend = store::select_backend(&config.store_backend, &storage_path);

        let manager = Self {
            config,
            refinement_config,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            logs: Arc::new(RwLock::new(HashMap::new())),
            execution_states: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(RwLock::new(VecDeque::new())),
            running: Arc::new(RwLock::new(Vec::new())),
            next_id: Arc::new(RwLock::new(1)),
            storage_path,
            backend,
            queue_running: Arc::new(RwLock::new(false)),
            refinement_running: Arc::new(RwLock::new(false)),
            last_refinement: Arc::new(RwLock::new(0)),
        };

        // Load from disk
        manager.load_from_disk_sync();

        Ok(manager)
    }

    pub async fn active_count(&self) -> usize {
        self.running.read().await.len()
    }

    /// Load tasks through the persistence backend (sync, initialization).
    fn load_from_disk_sync(&self) {
        if let Ok(Some(snapshot)) = self.backend.load() {
            if let Ok(mut tasks) = self.tasks.try_write() {
                *tasks = snapshot.tasks;

                // Boot-time reconciliation: a task still in a non-terminal
                // status ("queued" or "running") when this snapshot was
                // written can only mean the process died before it reached
                // complete_task/fail_task/cancel_task (the only places that
                // ever set a terminal status) — there is no clean-shutdown
                // path that leaves a task queued/running. Mark these
                // honestly as "interrupted" rather than silently leaving
                // them claiming to still be in flight forever (nothing
                // will ever resume or complete them otherwise). Real
                // per-step progress up to the crash point is preserved
                // (update_step/set_thinking_log/set_amt_summary now
                // checkpoint to disk after every step, not just at the
                // end), so an interrupted task's `steps` array honestly
                // reflects how far it actually got.
                let mut reconciled = 0u32;
                for task in tasks.values_mut() {
                    // An "awaiting" task is parked INSIDE a live walk (stage 8 loops
                    // in-process waiting for meta). A restart ends that walk and
                    // nothing re-enters it, so leaving the task "awaiting" would be a
                    // phantom — it becomes "interrupted" like any stranded run, and
                    // the boot driver cancels its gate item (resubmit the prompt).
                    if task.status == "queued" || task.status == "running" || task.status == "awaiting" {
                        // Coordination tasks (created via /task/create — they
                        // carry a "source" input from the API wrapper) are
                        // AGENT-routed work, not in-flight host execution: a
                        // restart doesn't interrupt them. Assignee-bearing
                        // tasks and source-tagged tasks both qualify —
                        // task 56 (unassigned architecture work) exposed the
                        // assignee-only gap. Everything else queued/running
                        // at snapshot time honestly becomes "interrupted".
                        let inputs = task.inputs.as_ref().cloned().unwrap_or(serde_json::Value::Null);
                        let is_coordination = inputs.get("source").and_then(|v| v.as_str()).is_some()
                            || inputs.get("assignee").map(|a| !a.is_null()).unwrap_or(false);
                        if is_coordination && task.status == "queued" {
                            continue;
                        }
                        task.status = "interrupted".to_string();
                        reconciled += 1;
                    }
                }
                if reconciled > 0 {
                    tracing::warn!(
                        count = reconciled,
                        "Boot reconciliation: marked stranded queued/running tasks as interrupted \
                         (process ended without a clean completion) — their recorded steps reflect \
                         real progress up to the interruption point"
                    );
                }
            }
            if let Ok(mut logs) = self.logs.try_write() {
                *logs = snapshot.logs;
            }
            if let Ok(mut next_id) = self.next_id.try_write() {
                *next_id = snapshot.next_id;
            }
        }
    }

    /// Save tasks through the persistence backend.
    async fn save_to_disk(&self) -> OzoneResult<()> {
        let tasks = self.tasks.read().await;
        let logs = self.logs.read().await;
        let next_id = *self.next_id.read().await;

        let snapshot = store::TaskSnapshot {
            tasks: tasks.clone(),
            logs: logs.clone(),
            next_id,
        };

        self.backend.save(&snapshot)
    }

    // ========================================================================
    // QUEUE MANAGEMENT
    // ========================================================================

    /// Enqueue a new task
    pub async fn enqueue_task(
        &self,
        blueprint_id: Option<u64>,
        mut inputs: HashMap<String, serde_json::Value>,
        user_id: UserID,
        device_id: DeviceID,
        workspace_id: Option<u64>,
        project_id: Option<u64>,
        priority: TaskPriority,
    ) -> OzoneResult<TaskID> {
        // Check queue limit. The queue holds CONCURRENTLY queued work:
        // completed / failed / cancelled tasks leave it (see
        // `remove_from_queue`), so the bound no longer fills permanently
        // after `max_queued` lifetime enqueues (the queue processor is never
        // started, which used to make every entry permanent).
        // TODO(operator-gated wiring): `config.tasks.max_queued_tasks` (config/mod.rs)
        // is not reachable here — lib.rs builds `TaskQueueConfig::default()`
        // and passes only the refinement settings. Honoring it needs a
        // one-line change at the construction site in lib.rs.
        let queue = self.queue.read().await;
        if queue.len() >= self.config.max_queued {
            return Err(OzoneError::TaskError("Task queue is full".into()));
        }
        drop(queue);

        // Trace spine: record which run created this task so the task (the
        // user-visible anchor) and the run's capture rows can be joined. Only
        // inserted when a trace is active and the caller didn't set one.
        if !inputs.contains_key("trace_id") {
            if let Some(t) = crate::trace::current_trace() {
                inputs.insert("trace_id".to_string(), serde_json::Value::String(t.trace_id));
            }
        }

        // Allocate task ID
        let mut next_id = self.next_id.write().await;
        let task_id = *next_id;
        *next_id += 1;
        drop(next_id);

        // Get task summary for consciousness
        let task_summary = inputs
            .get("prompt")
            .or(inputs.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("Task execution")
            .to_string();

        // Consciousness OBSERVER (C1: the store is the one implementation;
        // `config.consciousness_enabled` / `consciousness_path` are
        // deprecated and ignored — the store's own `config.enabled`, copied
        // from config.toml at boot, decides). Never blocks or fails the
        // enqueue: a gate decline used to reject the task here, which made
        // amt_loop (`.ok()`) silently lose its task; blocking now belongs to
        // the stage-8 model gate and jurisdiction. The store's verdict is
        // still RECORDED on the task (`gate_result`) for audit. Loop-generated
        // tasks get no per-task gate (bookkeeping, not user intent).
        let is_loop_task = consciousness_hooks::is_loop_task_inputs(&inputs);
        let gate_result = if !is_loop_task && crate::consciousness::store::is_consciousness_enabled() {
            // Do not front-run an active I-Loop cycle (bounded wait; only
            // waits when the in-process flag / compat file says it is busy).
            consciousness_hooks::wait_for_i_loop_completion(
                &self.config.consciousness_path,
                self.config.i_loop_timeout_ms,
            )
            .await;

            Some(consciousness_hooks::gate_for_task(task_id, &task_summary, user_id))
        } else {
            None
        };

        // Create stored task
        let stored_task = StoredTask {
            task_id,
            blueprint_id,
            status: "queued".to_string(),
            progress: 0.0,
            created_at: now(),
            started_at: None,
            completed_at: None,
            user_id,
            device_id,
            workspace_id,
            project_id,
            parent_task_id: inputs.get("parent_task_id").and_then(|v| v.as_u64()),
            inputs: Some(serde_json::to_value(&inputs).unwrap_or_default()),
            outputs: None,
            steps: Vec::new(),
            total_tokens: 0,
            error: None,
            gate_result,
            thinking_log: Vec::new(),
            amt_summary: None,
            // UNIVERSAL ORDER Stage 1: kind may arrive via inputs (a
            // /task/create caller sets inputs.kind); everything else defaults.
            kind: inputs
                .get("kind")
                .and_then(|k| k.as_str())
                .unwrap_or("todo")
                .to_string(),
            due_at: inputs.get("due_at").and_then(|d| d.as_u64()),
            remind_at: inputs.get("remind_at").and_then(|d| d.as_u64()),
            recurrence: inputs
                .get("recurrence")
                .and_then(|r| r.as_str())
                .map(String::from),
            meeting_url: inputs
                .get("meeting_url")
                .and_then(|m| m.as_str())
                .map(String::from),
            external_ref: inputs.get("external_ref").cloned(),
            // Checklist items arrive as [{text, done}] (or bare strings) on
            // /task/create; normalised to {text, done} objects.
            checklist: inputs
                .get("checklist")
                .and_then(|c| c.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|it| {
                            if let Some(t) = it.as_str() {
                                Some(serde_json::json!({"text": t, "done": false}))
                            } else if it.get("text").and_then(|t| t.as_str()).is_some() {
                                Some(serde_json::json!({
                                    "text": it["text"].clone(),
                                    "done": it.get("done").and_then(|d| d.as_bool()).unwrap_or(false),
                                }))
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
            note_body: inputs
                .get("note")
                .and_then(|n| n.as_str())
                .map(String::from),
        };

        // Store task
        self.tasks.write().await.insert(task_id, stored_task);
        self.logs.write().await.insert(task_id, Vec::new());

        // Create queued task entry
        let queued_due_at = inputs.get("due_at").and_then(|d| d.as_u64());
        let queued = QueuedTask {
            task_id,
            priority,
            created_at: now(),
            blueprint_id,
            user_id,
            device_id,
            workspace_id,
            project_id,
            inputs,
            task_summary: task_summary.clone(),
            due_at: queued_due_at,
        };

        // Add to queue. Order key: priority desc, then (within equal
        // priority) earlier due_at first, undated after dated, then FIFO
        // (a new item goes after existing equals).
        let mut queue = self.queue.write().await;
        let insert_pos = queue
            .iter()
            .position(|t| {
                t.priority < priority
                    || (t.priority == priority
                        && match (queued_due_at, t.due_at) {
                            (Some(new_d), Some(old_d)) => new_d < old_d,
                            (Some(_), None) => true,
                            _ => false,
                        })
            })
            .unwrap_or(queue.len());
        queue.insert(insert_pos, queued);

        // Add to perception window (observer; no-op when the store is disabled)
        consciousness_hooks::add_to_perception_window(
            &self.config.consciousness_path,
            task_id,
            "task",
            "queued",
        );

        // Save to disk
        let _ = self.save_to_disk().await;

        tracing::info!("Enqueued task {} with priority {:?}", task_id, priority);

                // TASK-LIFECYCLE RIPPLE (R5): creation moves the live graph.
        emit_task_ripple("created", task_id, workspace_id, project_id);
Ok(task_id)
    }

    /// Start the queue processor.
    ///
    /// INTENTIONALLY NOT STARTED anywhere (verified: no caller): this loop
    /// only pops the queue and flips a task to "running" — it never executes
    /// anything ("actual execution is handled by the orchestrator"). Walk
    /// tasks are executed by `/orchestrate` itself and complete via
    /// `complete_task`/`fail_task`; queued coordination tasks are agent-routed
    /// records. A real task executor (queued task -> `orchestrate`) is
    /// deliberately not built until an approval gate for what it may run is
    /// designed — wiring this processor without one would become the newest
    /// guardrail bypass. Kept (nothing dropped) for that future wiring.
    pub async fn start_queue_processor(&self) {
        let mut running = self.queue_running.write().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let tasks = Arc::clone(&self.tasks);
        let logs = Arc::clone(&self.logs);
        let queue = Arc::clone(&self.queue);
        let running_tasks = Arc::clone(&self.running);
        let queue_running = Arc::clone(&self.queue_running);
        let config = self.config.clone();

        tokio::spawn(async move {
            loop {
                // Check if still running
                if !*queue_running.read().await {
                    break;
                }

                // Check for available slots
                let running_count = running_tasks.read().await.len();
                if running_count >= config.max_concurrent {
                    tokio::time::sleep(Duration::from_millis(config.check_interval_ms)).await;
                    continue;
                }

                // Get next task from queue
                let next_task = {
                    let mut q = queue.write().await;
                    q.pop_front()
                };

                if let Some(queued_task) = next_task {
                    let task_id = queued_task.task_id;

                    // Mark as running
                    running_tasks.write().await.push(task_id);

                    // Update task status
                    if let Some(task) = tasks.write().await.get_mut(&task_id) {
                        task.status = "running".to_string();
                        task.started_at = Some(now());
                    }

                    // Add log entry
                    if let Some(task_logs) = logs.write().await.get_mut(&task_id) {
                        task_logs.push(LogEntry {
                            timestamp: now(),
                            level: LogLevel::Info,
                            message: "Task started".to_string(),
                            metadata: std::collections::HashMap::new(),
                        });
                    }

                    tracing::info!("Started task {}", task_id);

                    // Note: Actual task execution is handled by the orchestrator
                    // The queue processor just manages the queue state
                }

                tokio::time::sleep(Duration::from_millis(config.check_interval_ms)).await;
            }
        });
    }

    /// Stop the queue processor
    pub async fn stop_queue_processor(&self) {
        *self.queue_running.write().await = false;
    }

    /// Mark a task as completed
    pub async fn complete_task(
        &self,
        task_id: TaskID,
        outputs: Option<serde_json::Value>,
        total_tokens: u32,
    ) -> OzoneResult<()> {
        // Update task
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.status = "completed".to_string();
                task.completed_at = Some(now());
                task.outputs = outputs;
                task.total_tokens = total_tokens;
                task.progress = 1.0;
            }
        }
        self.after_completion(task_id, total_tokens).await?;
        // a parent walk parked as `awaiting_children` may have been waiting for exactly this task (and the work-order nodes above it)
        self.release_awaiting(task_id).await;
        Ok(())
    }

    /// Everything that follows a task becoming "completed": running/queue bookkeeping, log, consciousness observer, save, ripple. Shared by
    /// `complete_task` and the deferred release of a parked task (`release_awaiting`).
    async fn after_completion(&self, task_id: TaskID, total_tokens: u32) -> OzoneResult<()> {
        // Remove from running, and from the queue (the queue processor is
        // never started, so without this a finished task would hold a queue
        // slot for the process lifetime).
        {
            let mut running = self.running.write().await;
            running.retain(|&id| id != task_id);
        }
        self.remove_from_queue(task_id).await;

        // Add log
        self.add_log(task_id, LogLevel::Info, "Task completed".to_string())
            .await?;

        // Consciousness OBSERVER (store is the one implementation; gated on
        // the store's own config.enabled inside these calls). One read of the
        // task record; no lock is held across the store calls.
        let (task_summary, user_id, loop_task, workspace_id, project_id) = {
            let tasks = self.tasks.read().await;
            match tasks.get(&task_id) {
                Some(t) => (
                    consciousness_hooks::summarize_inputs(t.inputs.as_ref()),
                    Some(t.user_id),
                    consciousness_hooks::is_loop_task_value(t.inputs.as_ref()),
                    t.workspace_id,
                    t.project_id,
                ),
                None => ("Task".to_string(), None, false, None, None),
            }
        };
        consciousness_hooks::post_task_experience(
            &self.config.consciousness_path,
            task_id,
            &task_summary,
            true,
            user_id,
            Some(total_tokens),
            None,
            !loop_task,
        );
        consciousness_hooks::add_to_perception_window(
            &self.config.consciousness_path,
            task_id,
            "task",
            "completed",
        );

        // Save to disk
        let _ = self.save_to_disk().await;

        // TASK-LIFECYCLE RIPPLE (R5): completion moves the live graph and
        // wakes the assistant, as the R5 note always promised (only create
        // and fail did before). "updated" matches fail_task's event name (the
        // UI's graph-event union is created/updated/deleted). Loop-generated
        // tasks are skipped: their project-scoped ripple would queue a
        // GraphRipple re-expansion candidate for the very AMT amt_loop is
        // working on (a feedback loop), and the actors already ignore them.
        // TaskFailedActor only fires on status "failed", so a "completed"
        // ripple cannot feed it.
        if !loop_task {
            emit_task_ripple("updated", task_id, workspace_id, project_id);
        }

        tracing::info!("Completed task {}", task_id);

        Ok(())
    }

    // ------------------------------------------------------------------------------------------------------------------------------
    // COMPLETION GATE (operator 2026-11-11): a task completes only when ALL its steps and child work are complete; completed is FINAL.
    // ------------------------------------------------------------------------------------------------------------------------------

    /// The child work still blocking `walk`'s completion (all descendants through `parent_task_id`; empty = free to complete):
    /// - a work-order node (`wo_*`: AMT branch, step) that is not completed — except a node with no items and no children (nothing to settle),
    ///   and except `wo_capability` subtrees: a capability gap is cross-walk work whose `verify` item needs THIS walk to run, so gating on it would
    ///   deadlock (and one unfillable gap would hold every walk that reported it open forever); they stay visible in the work-order tree;
    /// - a child task of any other kind (an AMT re-expansion registered under the walk) that is not completed;
    ///   `failed` / `interrupted` ones block unless a LATER sibling with the same prompt completed (the re-expansion actor retries by registering
    ///   a new task per attempt) — a failure is never passed over; `cancelled` is the operator's own decision and does not block.
    /// Each blocker is `{task_id, kind, status, title}`.
    fn blockers_in(tasks: &HashMap<TaskID, StoredTask>, walk: TaskID) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut stack: Vec<TaskID> = tasks.values().filter(|t| t.parent_task_id == Some(walk)).map(|t| t.task_id).collect();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(t) = tasks.get(&id) else { continue };
            if t.kind == "wo_capability" {
                continue;
            }
            let kids: Vec<TaskID> = tasks.values().filter(|c| c.parent_task_id == Some(id)).map(|c| c.task_id).collect();
            let blocks = if t.kind.starts_with("wo_") {
                t.status != "completed" && t.status != "cancelled" && (!t.checklist.is_empty() || !kids.is_empty())
            } else {
                match t.status.as_str() {
                    "completed" | "cancelled" => false,
                    "failed" | "interrupted" => {
                        let prompt = t.inputs.as_ref().and_then(|i| i.get("prompt"));
                        !(prompt.is_some()
                            && tasks.values().any(|o| {
                                o.parent_task_id == t.parent_task_id
                                    && o.task_id > t.task_id
                                    && o.status == "completed"
                                    && o.inputs.as_ref().and_then(|i| i.get("prompt")) == prompt
                            }))
                    }
                    _ => true,
                }
            };
            if blocks {
                out.push(serde_json::json!({
                    "task_id": t.task_id, "kind": t.kind, "status": t.status,
                    "title": t.inputs.as_ref().and_then(|i| i.get("title").or_else(|| i.get("prompt"))).cloned(),
                }));
            }
            stack.extend(kids);
        }
        out.sort_by_key(|b| b["task_id"].as_u64().unwrap_or(0));
        out
    }

    /// The child work blocking this task's completion right now (see `blockers_in`).
    pub async fn blocking_children(&self, walk: TaskID) -> Vec<serde_json::Value> {
        let tasks = self.tasks.read().await;
        Self::blockers_in(&tasks, walk)
    }

    /// COMPLETION GATE. Complete the task now when nothing blocks it (`Ok(true)`, exactly `complete_task`); otherwise PARK it as
    /// `awaiting_children` with its outputs and tokens kept (`Ok(false)`): not completed, not failed, no live walk needed — the completion of the
    /// last blocking child releases it (`release_awaiting`, called from `complete_task` and `complete_item`, and re-checked on every gap sync).
    /// The check and the park happen under one lock, so a child finishing in between can never be missed. A task that is already completed
    /// stays exactly as it is (final).
    pub async fn complete_or_park(&self, task_id: TaskID, outputs: Option<serde_json::Value>, total_tokens: u32) -> OzoneResult<bool> {
        let parked_on = {
            let mut tasks = self.tasks.write().await;
            let blockers = Self::blockers_in(&tasks, task_id);
            match tasks.get_mut(&task_id) {
                Some(t) if t.status == "completed" => return Ok(true),
                Some(t) if !blockers.is_empty() => {
                    t.status = "awaiting_children".to_string();
                    t.completed_at = None;
                    t.outputs = outputs.clone();
                    t.total_tokens = total_tokens;
                    Some(blockers)
                }
                _ => None,
            }
        };
        match parked_on {
            None => {
                self.complete_task(task_id, outputs, total_tokens).await?;
                Ok(true)
            }
            Some(blockers) => {
                {
                    let mut running = self.running.write().await;
                    running.retain(|&id| id != task_id);
                }
                self.remove_from_queue(task_id).await;
                let ids: Vec<String> = blockers.iter().map(|b| format!("{} ({}, {})", b["task_id"], b["kind"].as_str().unwrap_or(""), b["status"].as_str().unwrap_or(""))).collect();
                self.add_log(
                    task_id,
                    LogLevel::Info,
                    format!("All steps ran; completion waits for open child work: {}", ids.join("; ")),
                )
                .await?;
                let _ = self.save_to_disk().await;
                let scope = self.tasks.read().await.get(&task_id).map(|t| (t.workspace_id, t.project_id));
                if let Some((workspace_id, project_id)) = scope {
                    emit_task_ripple("updated", task_id, workspace_id, project_id);
                }
                Ok(false)
            }
        }
    }

    /// Release every task parked as `awaiting_children` that nothing blocks any more — from `start` up through its ancestors (the walk is an
    /// ancestor of the work-order node or child task that just completed). The release is the deferred `complete_task`: status completed (final),
    /// the outputs/tokens parked with it, then the same consciousness/ripple tail. Idempotent; the status flip happens under the write lock so
    /// two concurrent releases cannot both complete the same task.
    pub async fn release_awaiting(&self, start: TaskID) {
        let mut cur = Some(start);
        let mut seen = std::collections::HashSet::new();
        while let Some(id) = cur {
            if !seen.insert(id) {
                break;
            }
            let (parent, released) = {
                let mut tasks = self.tasks.write().await;
                let Some(parked) = tasks.get(&id) else { break };
                let parent = parked.parent_task_id;
                let free = parked.status == "awaiting_children" && Self::blockers_in(&tasks, id).is_empty();
                let mut tokens = None;
                if free {
                    if let Some(t) = tasks.get_mut(&id) {
                        t.status = "completed".to_string();
                        t.completed_at = Some(now());
                        t.progress = 1.0;
                        tokens = Some(t.total_tokens);
                    }
                }
                (parent, tokens)
            };
            if let Some(tokens) = released {
                let _ = self.after_completion(id, tokens).await;
                tracing::info!(task_id = id, "parked task released: its last child work completed — completed (final)");
            }
            cur = parent;
        }
    }

    /// Re-check every parked task (boot, gap sync) — a child can have completed while no release hook ran.
    pub async fn release_all_awaiting(&self) {
        let parked: Vec<TaskID> = self.tasks.read().await.values().filter(|t| t.status == "awaiting_children").map(|t| t.task_id).collect();
        for id in parked {
            self.release_awaiting(id).await;
        }
    }

    /// Drop a finished (completed / failed / cancelled) task's entry from the
    /// in-memory queue so the `max_queued` bound applies to work that is
    /// actually queued.
    async fn remove_from_queue(&self, task_id: TaskID) {
        let mut queue = self.queue.write().await;
        queue.retain(|t| t.task_id != task_id);
    }

    /// Fail a task
    pub async fn fail_task(&self, task_id: TaskID, error: String) -> OzoneResult<()> {
        // Update task
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.status = "failed".to_string();
                task.completed_at = Some(now());
                task.error = Some(error.clone());
            }
        }

        // Remove from running and from the queue (see complete_task).
        {
            let mut running = self.running.write().await;
            running.retain(|&id| id != task_id);
        }
        self.remove_from_queue(task_id).await;

        // TASK-LIFECYCLE RIPPLE (R5): failure moves the live graph.
        if let Some(t) = self.tasks.read().await.get(&task_id) {
            emit_task_ripple("updated", task_id, t.workspace_id, t.project_id);
        }

        // Add log
        self.add_log(task_id, LogLevel::Error, format!("Task failed: {}", error))
            .await?;

        // Consciousness OBSERVER — now receives the failure CAUSE (it used to
        // be dropped). Loop-generated tasks record the experience but do not
        // move the emotional state.
        let (task_summary, user_id, loop_task) = {
            let tasks = self.tasks.read().await;
            match tasks.get(&task_id) {
                Some(t) => (
                    consciousness_hooks::summarize_inputs(t.inputs.as_ref()),
                    Some(t.user_id),
                    consciousness_hooks::is_loop_task_value(t.inputs.as_ref()),
                ),
                None => ("Task".to_string(), None, false),
            }
        };
        consciousness_hooks::post_task_experience(
            &self.config.consciousness_path,
            task_id,
            &task_summary,
            false,
            user_id,
            None,
            Some(error.as_str()),
            !loop_task,
        );
        consciousness_hooks::add_to_perception_window(
            &self.config.consciousness_path,
            task_id,
            "task",
            "failed",
        );

        // Save to disk
        let _ = self.save_to_disk().await;

        tracing::error!("Task {} failed: {}", task_id, error);

        Ok(())
    }

    // ========================================================================
    // TASK OPERATIONS
    // ========================================================================

    /// Get task by ID
    pub async fn get_task(&self, task_id: TaskID) -> Option<TaskData> {
        let tasks = self.tasks.read().await;
        tasks.get(&task_id).map(|t| self.to_task_data(task_id, t))
    }

    /// Update task progress
    pub async fn update_progress(
        &self,
        task_id: TaskID,
        current_step: u32,
        total_steps: u32,
    ) -> OzoneResult<()> {
        let progress = if total_steps > 0 {
            current_step as f32 / total_steps as f32
        } else {
            0.0
        };

        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.progress = progress;
        }

        // Also update execution state
        let mut states = self.execution_states.write().await;
        if let Some(state) = states.get_mut(&task_id) {
            state.current_step = current_step;
            state.total_steps = total_steps;
            state.last_updated = now();
        }

        Ok(())
    }

    /// Update a step with full extended metadata (the orchestrator's step
    /// model: stages, graph provenance, versioned measured notes). Finds the
    /// step or creates it on first report.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_step(
        &self,
        task_id: TaskID,
        step_index: u32,
        status: &str,
        tokens_used: Option<u32>,
        output_preview: Option<String>,
        execution_id: Option<String>,
        pipeline_name: &str,
        version_note: Option<String>,
        graph_ids_updated: Vec<String>,
        graph_ids_read: Vec<String>,
        stage_completed: Option<String>,
        stages_pending: Vec<String>,
        methodology_ids_applied: Vec<u64>,
        model_used: Option<String>,
    ) -> OzoneResult<()> {
        // Recorded for provenance; per-step persistence lands with the
        // execution-store expansion.
        let _ = execution_id;

        {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            if let Some(step) = task.steps.iter_mut().find(|s| s.step_index == step_index) {
                step.status = status.to_string();
                if let Some(tok) = tokens_used {
                    step.tokens_used = tok;
                }
                if let Some(preview) = output_preview {
                    step.output_summary = Some(preview);
                }
                if !pipeline_name.is_empty() {
                    step.action = pipeline_name.to_string();
                }
                if let Some(stage) = stage_completed {
                    if !step.stages_completed.contains(&stage) {
                        step.stages_completed.push(stage);
                    }
                    step.current_stage = None;
                }
                step.stages_pending = stages_pending;
                if !graph_ids_updated.is_empty() {
                    step.graph_ids_updated = graph_ids_updated;
                }
                if !graph_ids_read.is_empty() {
                    step.graph_ids_read = graph_ids_read;
                }
                step.methodology_ids_applied = methodology_ids_applied;
                if model_used.is_some() {
                    step.model_used = model_used.clone();
                }
                if let Some(note) = version_note {
                    step.version += 1;
                    step.version_notes.push(StepVersionNote {
                        version: step.version,
                        note,
                        timestamp: now(),
                        change_type: "Updated".to_string(),
                    });
                }
                if status == "completed" || status == "failed" {
                    step.completed_at = Some(now());
                }
            } else {
                task.steps.push(StoredTaskStep {
                    step_index,
                    action: if pipeline_name.is_empty() {
                        "step".to_string()
                    } else {
                        pipeline_name.to_string()
                    },
                    context_assembled: None,
                    context_sources: Vec::new(),
                    pipeline_id: 0,
                    status: status.to_string(),
                    started_at: Some(now()),
                    completed_at: if status == "completed" || status == "failed" {
                        Some(now())
                    } else {
                        None
                    },
                    tokens_used: tokens_used.unwrap_or(0),
                    output_summary: output_preview,
                    error: None,
                    model_used,
                    stages_completed: stage_completed.into_iter().collect(),
                    stages_pending,
                    current_stage: None,
                    graph_ids_read,
                    graph_ids_updated,
                    version: 1,
                    version_notes: version_note
                        .map(|n| {
                            vec![StepVersionNote {
                                version: 1,
                                note: n,
                                timestamp: now(),
                                change_type: "Created".to_string(),
                            }]
                        })
                        .unwrap_or_default(),
                    methodology_ids_applied,
                });
            }

            task.total_tokens = task.steps.iter().map(|s| s.tokens_used).sum();
        }
        }

        // Checkpoint to disk after every step — confirmed live this session
        // this was the actual gap behind "if it stops at a step it can
        // always cleanly resume": this function previously only mutated the
        // in-memory task record, so a crash mid-orchestration lost every
        // completed step's progress even though update_step had already
        // been called for each one — the on-disk record still showed the
        // task exactly as it was at enqueue time. save_to_disk snapshots
        // ALL tasks, not just this one, so this is a real (not free) write
        // per step; acceptable since step counts per task are small and
        // durability here is the explicit point.
        let _ = self.save_to_disk().await;

        // Consciousness OBSERVER: report terminal step transitions to the
        // perception window (queued/running updates are skipped — a write per
        // intermediate update would be noise). No-op when the store is disabled.
        if status == "completed" || status == "failed" {
            consciousness_hooks::add_to_perception_window(
                &self.config.consciousness_path,
                task_id,
                &format!("step {}", step_index),
                status,
            );
        }

        Ok(())
    }

    /// Persist the full "thinking cycle" onto a task record. Called once
    /// after orchestration completes (src/orchestrator/mod.rs) — task
    /// creation happens mid-run, before the thinking log is complete, so
    /// this can't be set at enqueue_task time. Silently no-ops if the task
    /// id doesn't exist (e.g. orchestration failed before task creation).
    pub async fn set_thinking_log(
        &self,
        task_id: TaskID,
        thinking_log: Vec<serde_json::Value>,
    ) -> OzoneResult<()> {
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.thinking_log = thinking_log;
            }
        }
        let _ = self.save_to_disk().await;
        Ok(())
    }

    /// Persist the CONTEXT OBJECT for one step: the assembled context that
    /// step will actually receive, plus provenance of how it was gathered
    /// (e.g. ["keyword-scan"] today, ["traversal:Contextual"] once the
    /// TraversalEngine feeds assembly). Called from Stage 7 right after the
    /// task exists — Stage 6 assembles before a durable task id exists, so
    /// the context rides in `OrchestrationState.step_contexts` until here.
    /// Find-or-create like update_step; a step that already has context gets
    /// OVERWRITTEN (latest assembly wins — callers re-mirroring after a
    /// re-assembly expect that).
    pub async fn update_step_context(
        &self,
        task_id: TaskID,
        step_index: u32,
        context_assembled: &str,
        context_sources: Vec<String>,
    ) -> OzoneResult<()> {
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                if let Some(step) = task.steps.iter_mut().find(|s| s.step_index == step_index) {
                    step.context_assembled = Some(context_assembled.to_string());
                    step.context_sources = context_sources;
                } else {
                    task.steps.push(StoredTaskStep {
                        step_index,
                        action: "context-assembly".to_string(),
                        pipeline_id: 0,
                        status: "pending".to_string(),
                        started_at: Some(now()),
                        completed_at: None,
                        tokens_used: 0,
                        output_summary: None,
                        error: None,
                        stages_completed: Vec::new(),
                        stages_pending: Vec::new(),
                        current_stage: None,
                        graph_ids_read: Vec::new(),
                        graph_ids_updated: Vec::new(),
                        version: 1,
                        version_notes: Vec::new(),
                        methodology_ids_applied: Vec::new(),
                        model_used: None,
                        context_assembled: Some(context_assembled.to_string()),
                        context_sources,
                    });
                }
            }
        }
        let _ = self.save_to_disk().await;
        Ok(())
    }

    /// Park an ORCHESTRATION task in the durable "awaiting" state (the walk
    /// returned an awaiting item — a gate could not be judged yet). Unlike
    /// `update_coordination_status` this is NOT restricted to source-tagged
    /// coordination tasks: the orchestrator wrapper owns this transition.
    /// `await_id` is recorded in the task's `inputs.await_id` so the order
    /// view and the resume route can join the task to its awaiting item.
    pub async fn mark_awaiting(&self, task_id: &TaskID, await_id: &str) -> bool {
        let ok = {
            let mut tasks = self.tasks.write().await;
            match tasks.get_mut(task_id) {
                Some(task) => {
                    task.status = "awaiting".to_string();
                    task.completed_at = None;
                    let mut inputs = task
                        .inputs
                        .take()
                        .unwrap_or_else(|| serde_json::json!({}));
                    if let Some(obj) = inputs.as_object_mut() {
                        obj.insert("await_id".to_string(), serde_json::json!(await_id));
                    }
                    task.inputs = Some(inputs);
                    true
                }
                None => false,
            }
        };
        if ok {
            let _ = self.save_to_disk().await;
            if let Some(t) = self.tasks.read().await.get(task_id) {
                emit_task_ripple("updated", *task_id, t.workspace_id, t.project_id);
            }
        }
        ok
    }

    /// FAILURE DOCTRINE (2026-11-02): the walk never continues past a failed step. The
    /// task is PAUSED at that step with the error recorded, and can be resumed from
    /// exactly there at any time (Tasks tab, project tabs, meta chat). The failed step's
    /// row is marked `failed`; every completed step keeps its recorded output.
    pub async fn pause_for_error(&self, task_id: TaskID, step_index: u32, error: &str) -> bool {
        let ok = {
            let mut tasks = self.tasks.write().await;
            match tasks.get_mut(&task_id) {
                Some(task) => {
                    task.status = "paused".to_string();
                    task.completed_at = None;
                    task.error = Some(format!("paused at step {step_index}: {error}"));
                    if let Some(st) = task.steps.iter_mut().find(|s| s.step_index == step_index) {
                        st.status = "failed".to_string();
                    }
                    true
                }
                None => false,
            }
        };
        if ok {
            {
                let mut running = self.running.write().await;
                running.retain(|&id| id != task_id);
            }
            let _ = self.save_to_disk().await;
            if let Some(t) = self.tasks.read().await.get(&task_id) {
                emit_task_ripple("updated", task_id, t.workspace_id, t.project_id);
            }
        }
        ok
    }

    /// The step a failure-paused task stopped at (parsed from its recorded error), or
    /// `None` when the task is not paused for a step failure.
    pub async fn paused_step_of(&self, task_id: TaskID) -> Option<u32> {
        let tasks = self.tasks.read().await;
        let t = tasks.get(&task_id)?;
        if t.status != "paused" {
            return None;
        }
        let e = t.error.as_deref()?;
        let rest = e.strip_prefix("paused at step ")?;
        rest.split(':').next()?.trim().parse().ok()
    }

    /// paused (step failure) -> running. Clears the recorded error and resets the failed
    /// step to `pending`. False when the task is missing or not failure-paused.
    pub async fn resume_paused(&self, task_id: TaskID) -> bool {
        let ok = {
            let mut tasks = self.tasks.write().await;
            match tasks.get_mut(&task_id) {
                Some(task)
                    if task.status == "paused"
                        && task.error.as_deref().map(|e| e.starts_with("paused at step ")).unwrap_or(false) =>
                {
                    task.status = "running".to_string();
                    task.error = None;
                    for st in task.steps.iter_mut() {
                        if st.status == "failed" {
                            st.status = "pending".to_string();
                        }
                    }
                    true
                }
                _ => false,
            }
        };
        if ok {
            {
                let mut running = self.running.write().await;
                if !running.contains(&task_id) {
                    running.push(task_id);
                }
            }
            let _ = self.save_to_disk().await;
            if let Some(t) = self.tasks.read().await.get(&task_id) {
                emit_task_ripple("updated", task_id, t.workspace_id, t.project_id);
            }
        }
        ok
    }

    /// Resume a parked task: awaiting -> running (the meta drain or the UI
    /// asked for resume after the item was judged). Returns false when the
    /// task does not exist or is not awaiting. Never touches other states.
    pub async fn resume_awaiting(&self, task_id: &TaskID) -> bool {
        let ok = {
            let mut tasks = self.tasks.write().await;
            match tasks.get_mut(task_id) {
                Some(task) if task.status == "awaiting" => {
                    task.status = "running".to_string();
                    true
                }
                _ => false,
            }
        };
        if ok {
            let _ = self.save_to_disk().await;
            if let Some(t) = self.tasks.read().await.get(task_id) {
                emit_task_ripple("updated", *task_id, t.workspace_id, t.project_id);
            }
        }
        ok
    }

    /// Coordination-task lifecycle: the routed agent flips its own task to
    /// completed/failed/queued. Only /task/create tasks (source-tagged
    /// inputs) are eligible — host-executed orchestration tasks keep their
    /// orchestrator-driven lifecycle. Returns Ok(false) when the task
    /// doesn't exist or isn't coordination-routed.
    pub async fn update_coordination_status(
        &self,
        task_id: &TaskID,
        status: &str,
        error: Option<String>,
    ) -> OzoneResult<bool> {
        match status {
            // "paused" + "interrupted" added 2026-09-27 (universal-order
            // Phase 1b, guide §9): Paused exists in TaskStatus but the
            // coordination gate rejected it, so "what's live vs paused"
            // was unstateable. Interrupted is the boot-reconciliation
            // state — settable here so restoration flows can mark it too.
            // "awaiting" added 2026-11-01 (awaiting-item doctrine): a walk is
            // parked until meta/consciousness (or, for jurisdiction
            // confirmations only, a human) produces a judgment. Durable:
            // boot reconciliation never maps it to "interrupted".
            "completed" | "failed" | "queued" | "running" | "paused" | "interrupted"
            | "awaiting" => {}
            other => {
                return Err(OzoneError::TaskError(format!(
                    "invalid coordination status: {}",
                    other
                )))
            }
        }
        let ok = {
            let mut tasks = self.tasks.write().await;
            match tasks.get_mut(task_id) {
                Some(task) => {
                    let is_coordination = task
                        .inputs
                        .as_ref()
                        .and_then(|i| i.get("source"))
                        .and_then(|v| v.as_str())
                        .is_some();
                    if !is_coordination {
                        return Ok(false);
                    }
                    let now = now();
                    task.status = status.to_string();
                    if status == "completed" || status == "failed" {
                        task.completed_at = Some(now);
                    }
                    if status == "queued" {
                        task.completed_at = None;
                    }
                    if let Some(e) = error {
                        task.error = Some(e);
                    }
                    true
                }
                None => false,
            }
        };
        if ok {
            // An agent-finished coordination task also leaves the queue (it
            // would otherwise hold a slot forever — the processor never runs).
            if status == "completed" || status == "failed" {
                self.remove_from_queue(*task_id).await;
            }
            consciousness_hooks::add_to_perception_window(
                &self.config.consciousness_path,
                *task_id,
                "task",
                status,
            );
            let _ = self.save_to_disk().await;
        }
        if ok {
            if let Some(t) = self.tasks.read().await.get(task_id) {
                emit_task_ripple("updated", *task_id, t.workspace_id, t.project_id);
            }
        }
        Ok(ok)
    }

    /// Persist the real AMT structure onto a task record — same
    /// set-once-after-completion convention as `set_thinking_log`, for the
    /// same reason (task creation happens mid-run, before the AMT's final
    /// validated state is known).
    pub async fn set_amt_summary(
        &self,
        task_id: TaskID,
        amt_summary: serde_json::Value,
    ) -> OzoneResult<()> {
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.amt_summary = Some(amt_summary);
            }
        }
        let _ = self.save_to_disk().await;
        Ok(())
    }

    /// Add log entry to task
    pub async fn add_log(
        &self,
        task_id: TaskID,
        level: LogLevel,
        message: String,
    ) -> OzoneResult<()> {
        let mut logs = self.logs.write().await;
        let task_logs = logs.entry(task_id).or_insert_with(Vec::new);

        task_logs.push(LogEntry {
            timestamp: now(),
            level,
            message,
            metadata: std::collections::HashMap::new(),
        });

        // Keep last 1000 logs per task
        while task_logs.len() > 1000 {
            task_logs.remove(0);
        }

        Ok(())
    }

    /// List tasks with optional filtering
    pub async fn list_tasks(
        &self,
        status_filter: Option<&str>,
        user_filter: Option<UserID>,
        limit: usize,
        offset: usize,
    ) -> Vec<TaskData> {
        let tasks = self.tasks.read().await;

        let mut results: Vec<TaskData> = tasks
            .iter()
            .filter(|(_, t)| {
                if let Some(status) = status_filter {
                    if t.status != status {
                        return false;
                    }
                }
                if let Some(user_id) = user_filter {
                    if t.user_id != user_id {
                        return false;
                    }
                }
                true
            })
            .map(|(id, t)| self.to_task_data(*id, t))
            .collect();

        // Sort by created_at descending
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply pagination
        results.into_iter().skip(offset).take(limit).collect()
    }

    /// Get task logs
    pub async fn get_logs(&self, task_id: TaskID, limit: Option<usize>) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.get(&task_id)
            .map(|l| {
                let limit = limit.unwrap_or(100);
                l.iter().rev().take(limit).cloned().collect()
            })
            .unwrap_or_default()
    }

    /// Get task timeline
    pub async fn get_timeline(&self, task_id: TaskID) -> Vec<TimelineEvent> {
        let tasks = self.tasks.read().await;
        let logs = self.logs.read().await;

        let mut timeline = Vec::new();

        if let Some(task) = tasks.get(&task_id) {
            timeline.push(TimelineEvent {
                timestamp: task.created_at,
                event_type: "created".to_string(),
                details: task.blueprint_id.map(|id| format!("Blueprint: {}", id)),
            });

            if let Some(started) = task.started_at {
                timeline.push(TimelineEvent {
                    timestamp: started,
                    event_type: "started".to_string(),
                    details: None,
                });
            }

            // Add step events
            for step in &task.steps {
                if let Some(started) = step.started_at {
                    timeline.push(TimelineEvent {
                        timestamp: started,
                        event_type: format!("step_{}_started", step.step_index),
                        details: Some(step.action.clone()),
                    });
                }
                if let Some(completed) = step.completed_at {
                    timeline.push(TimelineEvent {
                        timestamp: completed,
                        event_type: format!("step_{}_{}", step.step_index, step.status),
                        details: step.output_summary.clone(),
                    });
                }
            }

            // Add log events
            if let Some(task_logs) = logs.get(&task_id) {
                for log in task_logs {
                    timeline.push(TimelineEvent {
                        timestamp: log.timestamp,
                        event_type: format!("log_{:?}", log.level).to_lowercase(),
                        details: Some(log.message.clone()),
                    });
                }
            }

            if let Some(completed) = task.completed_at {
                timeline.push(TimelineEvent {
                    timestamp: completed,
                    event_type: task.status.clone(),
                    details: task.error.clone(),
                });
            }
        }

        // Sort by timestamp
        timeline.sort_by_key(|e| e.timestamp);

        timeline
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: TaskID) -> OzoneResult<()> {
        // Remove from queue if queued
        {
            let mut queue = self.queue.write().await;
            queue.retain(|t| t.task_id != task_id);
        }

        // Update task
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                // OPERATOR DECISION on a failed child (2026-11-11): a `failed` / `interrupted` CHILD task that keeps its walk parked can be
                // cancelled by the operator — that is the operator's own decision and it no longer blocks the walk (`blockers_in`). A failed
                // task with no parent stays finished, exactly as before.
                let failed_child = (task.status == "failed" || task.status == "interrupted") && task.parent_task_id.is_some();
                if task.status == "completed" || (task.status == "failed" && !failed_child) {
                    return Err(OzoneError::TaskError(format!(
                        "Task {} is already finished",
                        task_id
                    )));
                }
                task.status = "cancelled".to_string();
                task.completed_at = Some(now());
            }
        }

        // Remove from running
        {
            let mut running = self.running.write().await;
            running.retain(|&id| id != task_id);
        }

        // Add log
        self.add_log(task_id, LogLevel::Info, "Task cancelled".to_string())
            .await?;

        // Consciousness OBSERVER: cancellation is perceived, not an outcome
        // (no experience / emotion), mirroring the completed/failed events.
        consciousness_hooks::add_to_perception_window(
            &self.config.consciousness_path,
            task_id,
            "task",
            "cancelled",
        );

        // Save to disk
        let _ = self.save_to_disk().await;

        // TASK-LIFECYCLE RIPPLE (R5): cancellation moves the live graph (the
        // promised-but-missing event, like completion). Skipped for
        // loop-generated tasks (same feedback-loop reason as complete_task).
        {
            let tasks = self.tasks.read().await;
            if let Some(t) = tasks.get(&task_id) {
                if !consciousness_hooks::is_loop_task_value(t.inputs.as_ref()) {
                    emit_task_ripple("updated", task_id, t.workspace_id, t.project_id);
                }
            }
        }

        tracing::info!("Cancelled task {}", task_id);

        // the walk above may have been parked waiting on exactly this child: a cancelled child no longer blocks it
        let parent = self.tasks.read().await.get(&task_id).and_then(|t| t.parent_task_id);
        if let Some(parent) = parent {
            self.release_awaiting(parent).await;
        }

        Ok(())
    }

    /// Cooperative cancellation check — the orchestrator's step-execution
    /// loop polls this between steps (not mid-step; a step already in
    /// flight runs to completion) and stops issuing further steps once it
    /// sees "cancelled" here, set by `cancel_task`.
    pub async fn is_cancelled(&self, task_id: TaskID) -> bool {
        self.tasks
            .read()
            .await
            .get(&task_id)
            .map(|t| t.status == "cancelled")
            .unwrap_or(false)
    }

    /// Retry a failed task
    pub async fn retry_task(&self, task_id: TaskID) -> OzoneResult<TaskID> {
        let original = {
            let tasks = self.tasks.read().await;
            tasks.get(&task_id).cloned()
        };

        if let Some(task) = original {
            if task.status != "failed" && task.status != "cancelled" && task.status != "interrupted" {
                return Err(OzoneError::TaskError(format!(
                    "Task {} is not failed, interrupted or cancelled",
                    task_id
                )));
            }
            let (parent, kind) = (task.parent_task_id, task.kind.clone());

            let inputs: HashMap<String, serde_json::Value> = task
                .inputs
                .and_then(|i| serde_json::from_value(i).ok())
                .unwrap_or_default();

            let new_id = self
                .enqueue_task(
                    task.blueprint_id,
                    inputs,
                    task.user_id,
                    task.device_id,
                    task.workspace_id,
                    task.project_id,
                    TaskPriority::Normal,
                )
                .await?;
            // OPERATOR RETRY of a failed CHILD (2026-11-11): the retry is a NEW task instance (the failed one stays as it was) registered under
            // the SAME parent, so the walk's completion gate waits on it, and the failed original stops blocking once it completes
            // (`blockers_in`: a later sibling with the same prompt that completed supersedes it).
            if parent.is_some() {
                let mut tasks = self.tasks.write().await;
                if let Some(t) = tasks.get_mut(&new_id) {
                    t.parent_task_id = parent;
                    t.kind = kind;
                }
                drop(tasks);
                let _ = self.save_to_disk().await;
            }
            Ok(new_id)
        } else {
            Err(OzoneError::NotFound(format!("Task {} not found", task_id)))
        }
    }

    /// Compare multiple tasks
    pub async fn compare_tasks(&self, task_ids: &[TaskID]) -> Vec<TaskComparison> {
        let tasks = self.tasks.read().await;

        task_ids
            .iter()
            .filter_map(|&id| {
                tasks.get(&id).map(|t| {
                    let duration = match (t.started_at, t.completed_at) {
                        (Some(s), Some(c)) => Some(c - s),
                        _ => None,
                    };

                    TaskComparison {
                        task_id: id,
                        blueprint_id: t.blueprint_id,
                        status: t.status.clone(),
                        duration_secs: duration,
                        total_tokens: t.total_tokens,
                        created_at: t.created_at,
                    }
                })
            })
            .collect()
    }

    /// Clear completed tasks older than specified time
    pub async fn clear_completed(&self, older_than_secs: Option<u64>) -> usize {
        let cutoff = older_than_secs.map(|s| now().saturating_sub(s)).unwrap_or(0);

        let mut tasks = self.tasks.write().await;
        let mut logs = self.logs.write().await;

        let to_remove: Vec<TaskID> = tasks
            .iter()
            .filter(|(_, t)| {
                (t.status == "completed" || t.status == "failed" || t.status == "cancelled")
                    && t.completed_at.map(|c| c < cutoff).unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect();

        let count = to_remove.len();

        for id in &to_remove {
            tasks.remove(id);
            logs.remove(id);
        }

        drop(tasks);
        drop(logs);

        // Persist the removal. This used to call `save_to_disk()` WITHOUT
        // `.await` — the future was dropped unpolled, so cleared tasks came
        // back on the next boot.
        let _ = self.save_to_disk().await;

        count
    }

    /// Get queue status
    pub async fn get_queue_status(&self) -> (usize, usize, usize) {
        let queued = self.queue.read().await.len();
        let running = self.running.read().await.len();
        let max_concurrent = self.config.max_concurrent;
        (queued, running, max_concurrent)
    }

    // ========================================================================
    // REFINEMENT DAEMON (META LOOP)
    // ========================================================================

    /// Start the refinement daemon.
    ///
    /// INTENTIONALLY NOT STARTED anywhere (verified: no caller; lib.rs spawns
    /// `meta_loop`, which superseded it). Kept (nothing dropped) as the
    /// original design reference for methodology/category/modality refinement.
    pub async fn start_refinement_daemon(&self, zsei: Arc<dyn ZSEIAccess>) {
        if !self.refinement_config.enabled {
            return;
        }

        let mut running = self.refinement_running.write().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let refinement_running = Arc::clone(&self.refinement_running);
        let last_refinement = Arc::clone(&self.last_refinement);
        let config = self.refinement_config.clone();

        tokio::spawn(async move {
            loop {
                // Check if still running
                if !*refinement_running.read().await {
                    break;
                }

                // Check if it's time to run
                let last_run = *last_refinement.read().await;
                let now_ts = now();

                if now_ts - last_run >= config.interval_secs {
                    tracing::info!("Starting refinement daemon run");

                    // Run refinement tasks
                    let _ = Self::run_methodology_refinement(&zsei, &config).await;
                    let _ = Self::run_category_refinement(&zsei, &config).await;
                    let _ = Self::run_modality_refinement(&zsei, &config).await;
                    let _ = Self::run_deduplication(&zsei).await;

                    // Update last run timestamp
                    *last_refinement.write().await = now_ts;

                    tracing::info!("Refinement daemon run completed");
                }

                // Sleep for a while before checking again
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
    }

    /// Stop the refinement daemon
    pub async fn stop_refinement_daemon(&self) {
        *self.refinement_running.write().await = false;
    }

    /// The configured refinement settings (interval/enabled) — exposed so
    /// the real methodology meta-loop (src/orchestrator/meta_loop.rs, which
    /// needs LLM-calling capability TaskManager doesn't have) can respect
    /// the same enabled/interval_secs this daemon uses, rather than
    /// inventing a second config surface.
    pub fn refinement_config(&self) -> &RefinementConfig {
        &self.refinement_config
    }

    /// Methodology refinement - split complex methodologies
    async fn run_methodology_refinement(
        zsei: &Arc<dyn ZSEIAccess>,
        config: &RefinementConfig,
    ) -> OzoneResult<()> {
        // Get all methodologies
        let methodologies = zsei
            .search_by_keywords(&["*".to_string()], Some("Methodology"))
            .await
            .unwrap_or_default();

        for method_id in methodologies {
            if let Ok(Some(container)) = zsei.get_container(method_id).await {
                // Check principle count
                let principles: Vec<String> = container
                    .get("local_state")
                    .and_then(|ls| ls.get("storage"))
                    .and_then(|s| s.get("principles"))
                    .and_then(|p| serde_json::from_value(p.clone()).ok())
                    .unwrap_or_default();

                if principles.len() > config.max_principles_per_methodology {
                    tracing::info!(
                        "Methodology {} has {} principles, consider splitting",
                        method_id,
                        principles.len()
                    );

                    // TODO: Implement automatic splitting via LLM
                    // For now, just log the recommendation
                }
            }
        }

        Ok(())
    }

    /// Category refinement - split large categories
    async fn run_category_refinement(
        zsei: &Arc<dyn ZSEIAccess>,
        config: &RefinementConfig,
    ) -> OzoneResult<()> {
        // Get all categories
        let categories = zsei.get_categories("*").await.unwrap_or_default();

        for cat_id in categories {
            // Count methodologies in this category
            let methodologies = zsei
                .search_by_keywords(&[cat_id.to_string()], Some("Methodology"))
                .await
                .unwrap_or_default();

            if methodologies.len() > config.max_methodologies_per_category {
                tracing::info!(
                    "Category {} has {} methodologies, consider splitting",
                    cat_id,
                    methodologies.len()
                );

                // TODO: Implement automatic splitting via LLM
            }
        }

        Ok(())
    }

    /// Modality refinement - detect new modalities from usage patterns
    async fn run_modality_refinement(
        _zsei: &Arc<dyn ZSEIAccess>,
        _config: &RefinementConfig,
    ) -> OzoneResult<()> {
        // Analyze recent tasks for new data patterns
        // This would look for file types or data structures not covered by existing modalities

        // TODO: Implement modality detection
        // For now, just log that we're checking
        tracing::debug!("Checking for new modality patterns");

        Ok(())
    }

    /// Deduplication - find and merge duplicate content
    async fn run_deduplication(_zsei: &Arc<dyn ZSEIAccess>) -> OzoneResult<()> {
        // Find containers with very similar content
        // This would use semantic similarity to detect duplicates

        // TODO: Implement deduplication
        tracing::debug!("Running deduplication check");

        Ok(())
    }

    // ========================================================================
    // WORK-ORDER TREE (operator 2026-11-10) — the universal checklist on the EXISTING task records
    // ========================================================================
    // RULES (one rule, derived — never a separately-set flag):
    //   1. A work-order task is COMPLETE exactly when every one of its checklist items is done AND every child task is complete (and it has
    //      at least one item or child). Checklist completion = step completion = task completion; `settle` derives it and propagates upward.
    //   2. COMPLETED IS FINAL. `complete_item` never writes `done: false`; a completed task rejects `graft_item`; nothing reopens anything. The
    //      item's `completed_at` / `by` / `evidence` and the task's `completed_at` ARE the log. A RERUN is a NEW task (`create_work_task` with
    //      `rerun_of`), and an edit of the plan is a GRAFT — new items on an open task, or a new child under the original (`parent_task_id`) —
    //      never a rewrite of completed ones.
    //   3. The tree grows with the AMT: `create_work_task` is idempotent on (parent, kind, key), so a re-materialization after the AMT expanded
    //      adds only the NEW nodes, under their parents; an expanded branch gates its parent's completion (rule 1 counts every child).
    //   4. Work-order tasks have kind `wo_*`, are never queued, never gated by consciousness and do not appear in `/order/global` unless asked
    //      for by kind. Items are ticked ONLY by the real event that completes them.

    /// Create a work-order task (idempotent on `(parent, kind, key)` unless `rerun_of` is given: a rerun is always a NEW task). `items` are
    /// `(key, text)` checklist items, all open. Scope (user / device / workspace / project) comes from `scope_from` (or the parent).
    pub async fn create_work_task(
        &self,
        parent: Option<TaskID>,
        kind: &str,
        key: &str,
        title: &str,
        items: &[(String, String)],
        rerun_of: Option<TaskID>,
        scope_from: Option<TaskID>,
    ) -> Option<TaskID> {
        let mut tasks = self.tasks.write().await;
        if rerun_of.is_none() {
            if let Some((id, _)) = tasks.iter().find(|(_, t)| t.parent_task_id == parent && t.kind == kind && wo_key(t) == Some(key)) {
                return Some(*id);
            }
        }
        let scope = scope_from.or(parent).and_then(|id| tasks.get(&id)).map(|t| (t.user_id, t.device_id, t.workspace_id, t.project_id));
        let (user_id, device_id, workspace_id, project_id) = scope.unwrap_or((0, 0, None, None));
        let mut next = self.next_id.write().await;
        let task_id = *next;
        *next += 1;
        drop(next);
        let mut inputs = serde_json::json!({"kind": kind, "work_key": key, "title": title, "prompt": title, "source": "work-order"});
        if let Some(r) = rerun_of {
            inputs["rerun_of"] = serde_json::json!(r);
        }
        tasks.insert(
            task_id,
            StoredTask {
                task_id,
                blueprint_id: None,
                status: "open".to_string(),
                progress: 0.0,
                created_at: now(),
                started_at: None,
                completed_at: None,
                user_id,
                device_id,
                workspace_id,
                project_id,
                parent_task_id: parent,
                inputs: Some(inputs),
                outputs: None,
                steps: Vec::new(),
                total_tokens: 0,
                error: None,
                gate_result: None,
                thinking_log: Vec::new(),
                amt_summary: None,
                kind: kind.to_string(),
                due_at: None,
                remind_at: None,
                recurrence: None,
                meeting_url: None,
                external_ref: None,
                checklist: items.iter().map(|(k, t)| serde_json::json!({"key": k, "text": t, "done": false})).collect(),
                note_body: None,
            },
        );
        drop(tasks);
        self.logs.write().await.insert(task_id, Vec::new());
        let _ = self.save_to_disk().await;
        Some(task_id)
    }

    /// The newest work-order task of this `(kind, key)` under ANY parent (a global thing such as a capability gap lives under the walk that
    /// first reported it; later walks find it here).
    pub async fn find_work_task_any_parent(&self, kind: &str, key: &str) -> Option<TaskID> {
        self.tasks.read().await.iter().filter(|(_, t)| t.kind == kind && wo_key(t) == Some(key)).map(|(id, _)| *id).max()
    }

    /// The walk task that produced this persisted AMT container (task inputs carry `amt_container_id`) — the parent of work grafted onto that
    /// AMT after the walk (re-expansion).
    pub async fn find_walk_for_amt_container(&self, container_id: u64) -> Option<TaskID> {
        self.tasks
            .read()
            .await
            .iter()
            .filter(|(_, t)| t.inputs.as_ref().and_then(|i| i.get("amt_container_id")).and_then(|c| c.as_u64()) == Some(container_id))
            .map(|(id, _)| *id)
            .min()
    }

    /// Mark ONE checklist item done by the real event that completes it. Idempotent: an item that is already done is left exactly as it was
    /// (its original evidence / time / author stay the record) and `false` is returned. There is deliberately no way to set `done` back to
    /// false. After a tick the completion is re-derived for the task and its ancestors (`settle`).
    pub async fn complete_item(&self, task_id: TaskID, item_key: &str, evidence: &str, by: &str) -> bool {
        let mut ticked = false;
        {
            let mut tasks = self.tasks.write().await;
            if let Some(t) = tasks.get_mut(&task_id) {
                if let Some(item) = t.checklist.iter_mut().find(|i| i.get("key").and_then(|k| k.as_str()) == Some(item_key)) {
                    if item.get("done").and_then(|d| d.as_bool()) != Some(true) {
                        item["done"] = serde_json::json!(true);
                        item["completed_at"] = serde_json::json!(now());
                        item["by"] = serde_json::json!(by);
                        item["evidence"] = serde_json::json!(evidence);
                        ticked = true;
                    }
                }
            }
        }
        if ticked {
            self.settle(task_id).await;
            self.release_awaiting(task_id).await; // the walk above may have been parked waiting for exactly this item
            let _ = self.save_to_disk().await;
        }
        ticked
    }

    /// GRAFT a new checklist item onto an OPEN task (idempotent on `key`). A completed task is FINAL: it rejects the graft (`Err`), and the
    /// caller grafts a new child task under it instead (`create_work_task` with this task as `parent`).
    pub async fn graft_item(&self, task_id: TaskID, key: &str, text: &str) -> Result<bool, String> {
        let mut tasks = self.tasks.write().await;
        let Some(t) = tasks.get_mut(&task_id) else { return Err("no such task".into()) };
        if t.status == "completed" {
            return Err("completed is final — graft a child task (or start a rerun) instead".into());
        }
        if t.checklist.iter().any(|i| i.get("key").and_then(|k| k.as_str()) == Some(key)) {
            return Ok(false);
        }
        t.checklist.push(serde_json::json!({"key": key, "text": text, "done": false}));
        drop(tasks);
        let _ = self.save_to_disk().await;
        Ok(true)
    }

    /// Re-derive completion for a work-order task and its ancestors. A `wo_*` task becomes "completed" (once, final) when all its items are
    /// done and all its children are complete. A non-work-order ancestor (the walk's own task) keeps the status its own lifecycle sets.
    async fn settle(&self, start: TaskID) {
        let mut tasks = self.tasks.write().await;
        let mut cur = Some(start);
        while let Some(id) = cur {
            let Some(t) = tasks.get(&id) else { break };
            let parent = t.parent_task_id;
            if !t.kind.starts_with("wo_") {
                break;
            }
            let (already, items_done, has_items) = (t.status == "completed", t.checklist.iter().all(|i| i.get("done").and_then(|d| d.as_bool()) == Some(true)), !t.checklist.is_empty());
            let children: Vec<&StoredTask> = tasks.values().filter(|c| c.parent_task_id == Some(id)).collect();
            let children_done = children.iter().all(|c| c.status == "completed");
            let complete = items_done && children_done && (has_items || !children.is_empty());
            if already || !complete {
                break; // final stays final; an open task stays open and so does everything above it
            }
            if let Some(t) = tasks.get_mut(&id) {
                t.status = "completed".to_string();
                t.completed_at = Some(now());
                t.progress = 1.0;
            }
            cur = parent;
        }
    }

    /// The work-order tree rooted at `root` (derived at read time from the task records — nothing is copied): each node carries its checklist
    /// (with the completion record of every done item), its derived completion, `rerun_of`, and its children, oldest first.
    pub async fn work_order(&self, root: TaskID) -> Option<serde_json::Value> {
        let tasks = self.tasks.read().await;
        // NO depth cap (operator 2026-11-11: no fixed limits): a task is visited ONCE (a visited-set cycle check) — a corrupt parent link that loops
        // back to an ancestor cuts that edge, a legitimately deep tree is always walked to its leaves.
        fn node(tasks: &HashMap<TaskID, StoredTask>, id: TaskID, visited: &mut std::collections::HashSet<TaskID>) -> Option<serde_json::Value> {
            if !visited.insert(id) {
                return None;
            }
            let t = tasks.get(&id)?;
            let mut kids: Vec<TaskID> = tasks.values().filter(|c| c.parent_task_id == Some(id)).map(|c| c.task_id).collect();
            kids.sort();
            let children: Vec<serde_json::Value> = kids.iter().filter_map(|k| node(tasks, *k, visited)).collect();
            let done_items = t.checklist.iter().filter(|i| i.get("done").and_then(|d| d.as_bool()) == Some(true)).count();
            let children_complete = children.iter().all(|c| c["complete"].as_bool() == Some(true));
            let derived = done_items == t.checklist.len() && children_complete && (!t.checklist.is_empty() || !children.is_empty());
            Some(serde_json::json!({
                "task_id": t.task_id, "kind": t.kind, "key": wo_key(t), "title": t.inputs.as_ref().and_then(|i| i.get("title")).cloned(),
                "status": t.status, "complete": derived, "created_at": t.created_at, "completed_at": t.completed_at,
                "rerun_of": t.inputs.as_ref().and_then(|i| i.get("rerun_of")).cloned(),
                "checklist": t.checklist, "items_done": done_items, "items_total": t.checklist.len(),
                "children": children,
            }))
        }
        let mut visited: std::collections::HashSet<TaskID> = std::collections::HashSet::new();
        node(&tasks, root, &mut visited)
    }

    // ========================================================================
    // HELPERS
    // ========================================================================

    fn to_task_data(&self, task_id: TaskID, stored: &StoredTask) -> TaskData {
        TaskData {
            task_id,
            blueprint_id: stored.blueprint_id,
            status: stored.status.clone(),
            progress: stored.progress,
            created_at: stored.created_at,
            started_at: stored.started_at,
            completed_at: stored.completed_at,
            user_id: stored.user_id,
            device_id: stored.device_id,
            workspace_id: stored.workspace_id,
            project_id: stored.project_id,
            parent_task_id: stored.parent_task_id,
            child_count: 0, // Would need to scan for children
            error: stored.error.clone(),
            inputs: stored.inputs.clone(),
            outputs: stored.outputs.clone(),
            steps: stored
                .steps
                .iter()
                .map(|s| TaskStepData {
                    step_index: s.step_index,
                    action: s.action.clone(),
                    pipeline_id: s.pipeline_id,
                    status: s.status.clone(),
                    started_at: s.started_at,
                    completed_at: s.completed_at,
                    tokens_used: s.tokens_used,
                    output_summary: s.output_summary.clone(),
                    error: s.error.clone(),
                    stages_completed: s.stages_completed.clone(),
                    stages_pending: s.stages_pending.clone(),
                    current_stage: s.current_stage.clone(),
                    graph_ids_read: s.graph_ids_read.clone(),
                    graph_ids_updated: s.graph_ids_updated.clone(),
                    version: s.version,
                    version_notes: s.version_notes.clone(),
                    methodology_ids_applied: s.methodology_ids_applied.clone(),
                    model_used: s.model_used.clone(),
                    context_assembled: s.context_assembled.clone(),
                    context_sources: s.context_sources.clone(),
                })
                .collect(),
            total_tokens: stored.total_tokens,
            gate_result: stored.gate_result.clone(),
            thinking_log: stored.thinking_log.clone(),
            amt_summary: stored.amt_summary.clone(),
            kind: stored.kind.clone(),
            due_at: stored.due_at,
            remind_at: stored.remind_at,
            recurrence: stored.recurrence.clone(),
            meeting_url: stored.meeting_url.clone(),
            external_ref: stored.external_ref.clone(),
            checklist: stored.checklist.clone(),
            note_body: stored.note_body.clone(),
        }
    }
}

// ============================================================================
// ZSEI ACCESS TRAIT
// ============================================================================

/// Trait for ZSEI access (for refinement daemon)
#[async_trait::async_trait]
pub trait ZSEIAccess: Send + Sync {
    async fn get_container(&self, container_id: u64) -> Result<Option<serde_json::Value>, String>;
    async fn search_by_keywords(
        &self,
        keywords: &[String],
        container_type: Option<&str>,
    ) -> Result<Vec<u64>, String>;
    async fn get_categories(&self, modality: &str) -> Result<Vec<u64>, String>;
    async fn create_container(
        &self,
        parent_id: u64,
        container: serde_json::Value,
    ) -> Result<u64, String>;
    async fn update_container(
        &self,
        container_id: u64,
        updates: serde_json::Value,
    ) -> Result<(), String>;
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// The work-order key of a task (inputs.work_key) — the idempotency key within its parent and kind.
fn wo_key(t: &StoredTask) -> Option<&str> {
    t.inputs.as_ref().and_then(|i| i.get("work_key")).and_then(|k| k.as_str())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_manager_creation() {
        let config = TaskQueueConfig::default();
        let refinement_config = RefinementConfig::default();
        let manager = TaskManager::new(config, refinement_config).unwrap();

        let (queued, running, max) = manager.get_queue_status().await;
        assert_eq!(queued, 0);
        assert_eq!(running, 0);
        assert_eq!(max, 5);
    }

    #[tokio::test]
    async fn test_enqueue_and_complete() {
        let mut config = TaskQueueConfig::default();
        config.consciousness_enabled = false;
        config.storage_path = "/tmp/test_tasks".to_string();

        let refinement_config = RefinementConfig {
            enabled: false,
            ..Default::default()
        };

        let manager = TaskManager::new(config, refinement_config).unwrap();

        let mut inputs = HashMap::new();
        inputs.insert("prompt".to_string(), serde_json::json!("Test task"));

        let task_id = manager
            .enqueue_task(None, inputs, 1, 1, None, None, TaskPriority::Normal)
            .await
            .unwrap();

        assert!(task_id > 0);

        let task = manager.get_task(task_id).await.unwrap();
        assert_eq!(task.status, "queued");

        manager
            .complete_task(task_id, Some(serde_json::json!({"result": "done"})), 100)
            .await
            .unwrap();

        let task = manager.get_task(task_id).await.unwrap();
        assert_eq!(task.status, "completed");
        assert_eq!(task.total_tokens, 100);
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    // T-A4: amt_summary genuinely survives a restart on the task record.
    // `TaskManager::new` always calls `load_from_disk_sync()` (see above),
    // so constructing a second manager against the same storage_path is a
    // real simulation of a process restart — not an in-memory-only check —
    // and `set_amt_summary` calling `save_to_disk()` is exactly what needs
    // proving actually round-trips through the real persistence backend.
    // Own temp dir per test run (not the shared /tmp/test_tasks the
    // existing tests above use) so this doesn't race other tests touching
    // the same path.
    #[tokio::test]
    async fn amt_summary_survives_restart() {
        let storage_path = std::env::temp_dir()
            .join(format!(
                "amt_summary_restart_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .to_string_lossy()
            .into_owned();

        let refinement_config = RefinementConfig { enabled: false, ..Default::default() };

        let task_id;
        let summary = serde_json::json!({
            "total_nodes": 7,
            "verified_nodes": 3,
            "unverified_nodes": 4,
            "max_depth": 2
        });
        {
            let mut config = TaskQueueConfig::default();
            config.consciousness_enabled = false;
            config.storage_path = storage_path.clone();
            let manager = TaskManager::new(config, refinement_config.clone()).unwrap();

            let mut inputs = HashMap::new();
            inputs.insert("prompt".to_string(), serde_json::json!("amt persistence test"));
            task_id = manager
                .enqueue_task(None, inputs, 1, 1, None, None, TaskPriority::Normal)
                .await
                .unwrap();

            manager.set_amt_summary(task_id, summary.clone()).await.unwrap();

            // Sanity check within the same process before simulating restart.
            let task = manager.get_task(task_id).await.unwrap();
            assert_eq!(task.amt_summary, Some(summary.clone()));
        }

        // Fresh TaskManager, same storage_path — the real restart simulation.
        let mut config2 = TaskQueueConfig::default();
        config2.consciousness_enabled = false;
        config2.storage_path = storage_path;
        let reloaded_manager = TaskManager::new(config2, refinement_config).unwrap();

        let reloaded_task = reloaded_manager
            .get_task(task_id)
            .await
            .expect("task must survive restart");
        assert_eq!(
            reloaded_task.amt_summary,
            Some(summary),
            "amt_summary must round-trip through real disk persistence across a restart"
        );
    }

    // T-U3: /task/update's real guard (update_coordination_status) only
    // touches source-tagged (coordination) tasks — a real host-executed
    // task (an actual orchestration run, no "source" input) must be
    // refused, not silently mutated by an agent's handoff-tracking call.
    #[tokio::test]
    async fn coordination_update_refuses_a_host_executed_task() {
        let storage_path = std::env::temp_dir()
            .join(format!(
                "task_lifecycle_guard_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .to_string_lossy()
            .into_owned();
        let refinement_config = RefinementConfig { enabled: false, ..Default::default() };
        let mut config = TaskQueueConfig::default();
        config.consciousness_enabled = false;
        config.storage_path = storage_path;
        let manager = TaskManager::new(config, refinement_config).unwrap();

        // A real host-executed task: a genuine orchestration request, no
        // "source" tag — never created via the coordination API.
        let mut host_inputs = HashMap::new();
        host_inputs.insert("prompt".to_string(), serde_json::json!("what is 2+2?"));
        let host_task_id = manager
            .enqueue_task(None, host_inputs, 1, 1, None, None, TaskPriority::Normal)
            .await
            .unwrap();

        let refused = manager
            .update_coordination_status(&host_task_id, "completed", None)
            .await
            .unwrap();
        assert!(!refused, "a host-executed task (no source tag) must be refused, not updated");
        let task = manager.get_task(host_task_id).await.unwrap();
        assert_ne!(task.status, "completed", "the refused update must not have mutated the task");

        // A real coordination task: source-tagged, created via the
        // coordination API — this one IS allowed to be updated.
        let mut coord_inputs = HashMap::new();
        coord_inputs.insert("source".to_string(), serde_json::json!("zcode"));
        coord_inputs.insert("description".to_string(), serde_json::json!("[host-ops] restart requested"));
        let coord_task_id = manager
            .enqueue_task(None, coord_inputs, 1, 1, None, None, TaskPriority::Normal)
            .await
            .unwrap();

        let allowed = manager
            .update_coordination_status(&coord_task_id, "completed", None)
            .await
            .unwrap();
        assert!(allowed, "a real source-tagged coordination task must be updatable");
        let task = manager.get_task(coord_task_id).await.unwrap();
        assert_eq!(task.status, "completed");
    }
}
