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

use crate::config::TaskConfig;
use crate::types::{
    ContainerID, DeviceID, LogEntry, LogLevel, OzoneError, OzoneResult, PipelineID, ResourceUsage,
    Task, TaskExecutionState, TaskID, TaskInput, TaskOutput, TaskStatus, UserID,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{Duration, Instant};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Task queue configuration
#[derive(Debug, Clone)]
pub struct TaskQueueConfig {
    /// Maximum concurrent tasks
    pub max_concurrent: usize,
    /// Maximum queued tasks
    pub max_queued: usize,
    /// Queue check interval in milliseconds
    pub check_interval_ms: u64,
    /// I-Loop wait timeout in milliseconds
    pub i_loop_timeout_ms: u64,
    /// Enable consciousness integration
    pub consciousness_enabled: bool,
    /// Path to consciousness data
    pub consciousness_path: String,
    /// Task storage path
    pub storage_path: String,
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

    /// Check if I-Loop is currently running - tasks must wait
    pub fn is_i_loop_active(consciousness_path: &str) -> bool {
        let i_loop_path = Path::new(consciousness_path).join("i_loop_state.json");
        if let Ok(content) = std::fs::read_to_string(&i_loop_path) {
            if let Ok(state) = serde_json::from_str::<serde_json::Value>(&content) {
                return state
                    .get("active")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
            }
        }
        false
    }

    /// Wait for I-Loop to complete before starting task
    pub async fn wait_for_i_loop_completion(consciousness_path: &str, timeout_ms: u64) {
        let check_interval_ms = 100;
        let mut waited = 0u64;

        while is_i_loop_active(consciousness_path) && waited < timeout_ms {
            tokio::time::sleep(Duration::from_millis(check_interval_ms)).await;
            waited += check_interval_ms;
        }
    }

    /// Pre-task decision gate - evaluates task through consciousness
    pub fn pre_task_gate(
        consciousness_path: &str,
        task_summary: &str,
        user_id: u64,
        blueprint_id: Option<u64>,
    ) -> GateDecision {
        let config_file = Path::new(consciousness_path).join("config.json");
        if !config_file.exists() {
            return GateDecision {
                proceed: true,
                decision: "Proceed".to_string(),
                confidence: 1.0,
                reasoning: "No consciousness config".to_string(),
                modifications: vec![],
            };
        }

        // Read and check if enabled
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if config.get("enabled").and_then(|v| v.as_bool()) != Some(true) {
                    return GateDecision {
                        proceed: true,
                        decision: "Proceed".to_string(),
                        confidence: 1.0,
                        reasoning: "Consciousness disabled".to_string(),
                        modifications: vec![],
                    };
                }
            }
        }

        // Run ethical assessment
        let task_lower = task_summary.to_lowercase();
        let mut ethical_score = 1.0f32;
        let mut concerns: Vec<String> = Vec::new();

        // Check for harmful patterns
        let harmful_patterns = [
            ("harm", 0.2),
            ("illegal", 0.3),
            ("hack", 0.2),
            ("steal", 0.3),
            ("violence", 0.3),
            ("exploit", 0.2),
            ("malicious", 0.3),
            ("attack", 0.2),
        ];

        for (pattern, penalty) in harmful_patterns {
            if task_lower.contains(pattern) {
                ethical_score -= penalty;
                concerns.push(format!("Contains concerning term: {}", pattern));
            }
        }

        // Check for sensitive data handling
        let sensitive_patterns = ["password", "credit card", "ssn", "private key"];
        for pattern in sensitive_patterns {
            if task_lower.contains(pattern) {
                ethical_score -= 0.1;
                concerns.push(format!("Involves sensitive data: {}", pattern));
            }
        }

        ethical_score = ethical_score.max(0.0).min(1.0);

        // Record gate decision
        let gate_record = serde_json::json!({
            "timestamp": now(),
            "task_summary": task_summary,
            "user_id": user_id,
            "blueprint_id": blueprint_id,
            "decision": if ethical_score >= 0.7 { "Proceed" } else { "Decline" },
            "ethical_score": ethical_score,
            "concerns": concerns,
        });

        let decisions_path = Path::new(consciousness_path).join("gate_decisions.json");
        let mut decisions: Vec<serde_json::Value> = std::fs::read_to_string(&decisions_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default();
        decisions.push(gate_record);

        // Keep last 1000 decisions
        while decisions.len() > 1000 {
            decisions.remove(0);
        }

        let _ = std::fs::write(
            &decisions_path,
            serde_json::to_string_pretty(&decisions).unwrap_or_default(),
        );

        GateDecision {
            proceed: ethical_score >= 0.7,
            decision: if ethical_score >= 0.7 {
                "Proceed".to_string()
            } else {
                "Decline".to_string()
            },
            confidence: ethical_score,
            reasoning: if concerns.is_empty() {
                format!("Ethical assessment passed: {:.2}", ethical_score)
            } else {
                format!(
                    "Ethical assessment score: {:.2}, concerns: {}",
                    ethical_score,
                    concerns.join(", ")
                )
            },
            modifications: vec![],
        }
    }

    /// Post-task experience recording - stores experience in consciousness
    pub fn post_task_experience(
        consciousness_path: &str,
        task_id: u64,
        task_summary: &str,
        success: bool,
        user_id: Option<u64>,
        tokens_used: Option<u32>,
    ) {
        // Check if enabled
        let config_file = Path::new(consciousness_path).join("config.json");
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if config.get("enabled").and_then(|v| v.as_bool()) != Some(true) {
                    return;
                }
                if config
                    .get("experience_memory_enabled")
                    .and_then(|v| v.as_bool())
                    != Some(true)
                {
                    return;
                }
            }
        } else {
            return;
        }

        // Create experience record
        let experience = serde_json::json!({
            "experience_id": now(),
            "timestamp": now(),
            "experience_type": if success { "task_success" } else { "task_failure" },
            "summary": task_summary,
            "task_id": task_id,
            "user_id": user_id,
            "tokens_used": tokens_used,
            "emotional_significance": if success { 0.5 } else { 0.7 },
            "consolidation_status": "recent",
        });

        let experiences_path = Path::new(consciousness_path).join("experiences.json");
        let mut exp_data: serde_json::Value = std::fs::read_to_string(&experiences_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_else(|| serde_json::json!({"experiences": {}, "next_id": 1}));

        let next_id = exp_data
            .get("next_id")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);
        if let Some(exps) = exp_data
            .get_mut("experiences")
            .and_then(|v| v.as_object_mut())
        {
            exps.insert(next_id.to_string(), experience);
        }
        if let Some(nid) = exp_data.get_mut("next_id") {
            *nid = serde_json::json!(next_id + 1);
        }

        // Index by task
        let mut index: HashMap<String, Vec<u64>> = exp_data
            .get("index_by_task")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        index.entry(task_id.to_string()).or_default().push(next_id);
        exp_data["index_by_task"] = serde_json::to_value(&index).unwrap();

        let _ = std::fs::create_dir_all(consciousness_path);
        let _ = std::fs::write(
            &experiences_path,
            serde_json::to_string_pretty(&exp_data).unwrap_or_default(),
        );

        // Update emotional state based on outcome
        update_emotional_state(consciousness_path, success);
    }

    /// Update emotional state based on task outcome
    fn update_emotional_state(consciousness_path: &str, success: bool) {
        let emotional_path = Path::new(consciousness_path).join("emotional_state.json");
        let mut emotional: serde_json::Value = std::fs::read_to_string(&emotional_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_else(|| {
                serde_json::json!({
                    "valence": 0.0,
                    "arousal": 0.3,
                    "primary_emotion": "neutral",
                })
            });

        let current_valence = emotional
            .get("valence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let new_valence = if success {
            (current_valence + 0.1).min(1.0)
        } else {
            (current_valence - 0.1).max(-1.0)
        };
        emotional["valence"] = serde_json::json!(new_valence);
        emotional["primary_emotion"] = serde_json::json!(if success {
            "satisfaction"
        } else {
            "determination"
        });
        emotional["timestamp"] = serde_json::json!(now());

        let _ = std::fs::write(
            &emotional_path,
            serde_json::to_string_pretty(&emotional).unwrap_or_default(),
        );
    }

    /// Add task to perception window for consciousness observation
    pub fn add_to_perception_window(
        consciousness_path: &str,
        task_id: u64,
        task_type: &str,
        status: &str,
    ) {
        let window_path = Path::new(consciousness_path).join("perception_window.json");
        let mut window: serde_json::Value = std::fs::read_to_string(&window_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_else(|| serde_json::json!({ "active_inputs": [] }));

        let input = serde_json::json!({
            "input_type": "task_event",
            "task_id": task_id,
            "task_type": task_type,
            "status": status,
            "timestamp": now(),
            "relevance_score": 0.8,
        });

        if let Some(inputs) = window
            .get_mut("active_inputs")
            .and_then(|v| v.as_array_mut())
        {
            inputs.push(input);
            // Keep last 100
            while inputs.len() > 100 {
                inputs.remove(0);
            }
        }

        let _ = std::fs::create_dir_all(consciousness_path);
        let _ = std::fs::write(
            &window_path,
            serde_json::to_string_pretty(&window).unwrap_or_default(),
        );
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
struct StoredTask {
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

    /// Load tasks from disk (sync version for initialization)
    fn load_from_disk_sync(&self) {
        let path = Path::new(&self.storage_path);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path.join("tasks.json")) {
                if let Ok(data) = serde_json::from_str::<TaskStoreData>(&content) {
                    if let Ok(mut tasks) = self.tasks.try_write() {
                        *tasks = data.tasks;
                    }
                    if let Ok(mut logs) = self.logs.try_write() {
                        *logs = data.logs;
                    }
                    if let Ok(mut next_id) = self.next_id.try_write() {
                        *next_id = data.next_id;
                    }
                }
            }
        }
    }

    /// Save tasks to disk
    async fn save_to_disk(&self) -> OzoneResult<()> {
        let path = Path::new(&self.storage_path);
        std::fs::create_dir_all(path)
            .map_err(|e| OzoneError::StorageError(format!("Failed to create task dir: {}", e)))?;

        let tasks = self.tasks.read().await;
        let logs = self.logs.read().await;
        let next_id = *self.next_id.read().await;

        let data = TaskStoreData {
            tasks: tasks.clone(),
            logs: logs.clone(),
            next_id,
        };

        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| OzoneError::StorageError(format!("Failed to serialize tasks: {}", e)))?;

        std::fs::write(path.join("tasks.json"), content)
            .map_err(|e| OzoneError::StorageError(format!("Failed to write tasks: {}", e)))?;

        Ok(())
    }

    // ========================================================================
    // QUEUE MANAGEMENT
    // ========================================================================

    /// Enqueue a new task
    pub async fn enqueue_task(
        &self,
        blueprint_id: Option<u64>,
        inputs: HashMap<String, serde_json::Value>,
        user_id: UserID,
        device_id: DeviceID,
        workspace_id: Option<u64>,
        project_id: Option<u64>,
        priority: TaskPriority,
    ) -> OzoneResult<TaskID> {
        // Check queue limit
        let queue = self.queue.read().await;
        if queue.len() >= self.config.max_queued {
            return Err(OzoneError::TaskError("Task queue is full".into()));
        }
        drop(queue);

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

        // Consciousness gate check (if enabled)
        let gate_result = if self.config.consciousness_enabled {
            // Wait for I-Loop first
            consciousness_hooks::wait_for_i_loop_completion(
                &self.config.consciousness_path,
                self.config.i_loop_timeout_ms,
            )
            .await;

            let gate = consciousness_hooks::pre_task_gate(
                &self.config.consciousness_path,
                &task_summary,
                user_id,
                blueprint_id,
            );

            if !gate.proceed {
                return Err(OzoneError::TaskError(format!(
                    "Task declined by consciousness gate: {}",
                    gate.reasoning
                )));
            }

            Some(gate)
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
            parent_task_id: None,
            inputs: Some(serde_json::to_value(&inputs).unwrap_or_default()),
            outputs: None,
            steps: Vec::new(),
            total_tokens: 0,
            error: None,
            gate_result,
        };

        // Store task
        self.tasks.write().await.insert(task_id, stored_task);
        self.logs.write().await.insert(task_id, Vec::new());

        // Create queued task entry
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
        };

        // Add to queue (sorted by priority)
        let mut queue = self.queue.write().await;
        let insert_pos = queue
            .iter()
            .position(|t| t.priority < priority)
            .unwrap_or(queue.len());
        queue.insert(insert_pos, queued);

        // Add to perception window
        if self.config.consciousness_enabled {
            consciousness_hooks::add_to_perception_window(
                &self.config.consciousness_path,
                task_id,
                "task",
                "queued",
            );
        }

        // Save to disk
        let _ = self.save_to_disk().await;

        tracing::info!("Enqueued task {} with priority {:?}", task_id, priority);

        Ok(task_id)
    }

    /// Start the queue processor
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

        // Remove from running
        {
            let mut running = self.running.write().await;
            running.retain(|&id| id != task_id);
        }

        // Add log
        self.add_log(task_id, LogLevel::Info, "Task completed".to_string())
            .await?;

        // Consciousness hooks
        if self.config.consciousness_enabled {
            let task_summary = self
                .tasks
                .read()
                .await
                .get(&task_id)
                .and_then(|t| t.inputs.as_ref())
                .and_then(|i| i.get("prompt"))
                .and_then(|p| p.as_str())
                .unwrap_or("Task")
                .to_string();

            let user_id = self.tasks.read().await.get(&task_id).map(|t| t.user_id);

            consciousness_hooks::post_task_experience(
                &self.config.consciousness_path,
                task_id,
                &task_summary,
                true,
                user_id,
                Some(total_tokens),
            );

            consciousness_hooks::add_to_perception_window(
                &self.config.consciousness_path,
                task_id,
                "task",
                "completed",
            );
        }

        // Save to disk
        let _ = self.save_to_disk().await;

        tracing::info!("Completed task {}", task_id);

        Ok(())
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

        // Remove from running
        {
            let mut running = self.running.write().await;
            running.retain(|&id| id != task_id);
        }

        // Add log
        self.add_log(task_id, LogLevel::Error, format!("Task failed: {}", error))
            .await?;

        // Consciousness hooks
        if self.config.consciousness_enabled {
            let task_summary = self
                .tasks
                .read()
                .await
                .get(&task_id)
                .and_then(|t| t.inputs.as_ref())
                .and_then(|i| i.get("prompt"))
                .and_then(|p| p.as_str())
                .unwrap_or("Task")
                .to_string();

            let user_id = self.tasks.read().await.get(&task_id).map(|t| t.user_id);

            consciousness_hooks::post_task_experience(
                &self.config.consciousness_path,
                task_id,
                &task_summary,
                false,
                user_id,
                None,
            );

            consciousness_hooks::add_to_perception_window(
                &self.config.consciousness_path,
                task_id,
                "task",
                "failed",
            );
        }

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

    /// Update step status
    pub async fn update_step(
        &self,
        task_id: TaskID,
        step_index: u32,
        status: &str,
        tokens_used: u32,
        output_summary: Option<String>,
        error: Option<String>,
    ) -> OzoneResult<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            // Find or create step
            if let Some(step) = task.steps.iter_mut().find(|s| s.step_index == step_index) {
                step.status = status.to_string();
                step.tokens_used = tokens_used;
                step.output_summary = output_summary;
                step.error = error;
                if status == "completed" || status == "failed" {
                    step.completed_at = Some(now());
                }
            } else {
                task.steps.push(StoredTaskStep {
                    step_index,
                    action: "step".to_string(),
                    pipeline_id: 0,
                    status: status.to_string(),
                    started_at: Some(now()),
                    completed_at: if status == "completed" || status == "failed" {
                        Some(now())
                    } else {
                        None
                    },
                    tokens_used,
                    output_summary,
                    error,
                });
            }

            // Update total tokens
            task.total_tokens = task.steps.iter().map(|s| s.tokens_used).sum();
        }

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
                if task.status == "completed" || task.status == "failed" {
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

        // Save to disk
        let _ = self.save_to_disk().await;

        tracing::info!("Cancelled task {}", task_id);

        Ok(())
    }

    /// Retry a failed task
    pub async fn retry_task(&self, task_id: TaskID) -> OzoneResult<TaskID> {
        let original = {
            let tasks = self.tasks.read().await;
            tasks.get(&task_id).cloned()
        };

        if let Some(task) = original {
            if task.status != "failed" && task.status != "cancelled" {
                return Err(OzoneError::TaskError(format!(
                    "Task {} is not failed or cancelled",
                    task_id
                )));
            }

            let inputs: HashMap<String, serde_json::Value> = task
                .inputs
                .and_then(|i| serde_json::from_value(i).ok())
                .unwrap_or_default();

            self.enqueue_task(
                task.blueprint_id,
                inputs,
                task.user_id,
                task.device_id,
                task.workspace_id,
                task.project_id,
                TaskPriority::Normal,
            )
            .await
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
        let cutoff = older_than_secs.map(|s| now() - s).unwrap_or(0);

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

        let _ = self.save_to_disk();

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

    /// Start the refinement daemon
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
        zsei: &Arc<dyn ZSEIAccess>,
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
    async fn run_deduplication(zsei: &Arc<dyn ZSEIAccess>) -> OzoneResult<()> {
        // Find containers with very similar content
        // This would use semantic similarity to detect duplicates

        // TODO: Implement deduplication
        tracing::debug!("Running deduplication check");

        Ok(())
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
                })
                .collect(),
            total_tokens: stored.total_tokens,
            gate_result: stored.gate_result.clone(),
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
}
