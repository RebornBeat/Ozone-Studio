//! TaskManagerPipeline - Pipeline #5
//! 
//! Task creation, tracking, and management.
//! Tasks execute BLUEPRINTS (not pipelines directly).
//! Progress is tracked per STEP in the blueprint.
//! 
//! NOTE: Built-in pipelines use DIRECT storage access, not HTTP.
//! Tasks are stored locally and synced to ZSEI containers.
//! 
//! Per spec §11-12:
//! - Tasks execute BLUEPRINTS with multiple steps
//! - Tasks track inputs, outputs, logs per step
//! - Tasks can be paused, cancelled, retried
//! 
//! CONSCIOUSNESS INTEGRATION (§33):
//! - Calls decision_gate BEFORE task creation (if consciousness enabled)
//! - Calls experience_memory AFTER task completion (if consciousness enabled)
//! - Window architecture can observe all registered tasks
//! 
//! I-LOOP PROTECTION:
//! - Tasks MUST wait for I-Loop to complete before starting
//! - I-Loop is NOT front-run by tasks
//! - Check i_loop_active flag before task execution

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;
use std::env;

// Direct task storage - same process, no HTTP
lazy_static::lazy_static! {
    static ref TASK_STORE: Mutex<TaskStore> = Mutex::new(TaskStore::new());
}

// Consciousness integration flag
fn consciousness_enabled() -> bool {
    env::var("OZONE_CONSCIOUSNESS_ENABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

/// Check if I-Loop is currently running - tasks must wait
fn is_i_loop_active() -> bool {
    let config_path = env::var("OZONE_CONSCIOUSNESS_PATH")
        .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
    
    let i_loop_path = Path::new(&config_path).join("i_loop_state.json");
    if let Ok(content) = std::fs::read_to_string(&i_loop_path) {
        if let Ok(state) = serde_json::from_str::<serde_json::Value>(&content) {
            return state.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
        }
    }
    false
}

/// Wait for I-Loop to complete before starting task
async fn wait_for_i_loop_completion() {
    let max_wait_ms = 30000; // Max 30 seconds
    let check_interval_ms = 100;
    let mut waited = 0;
    
    while is_i_loop_active() && waited < max_wait_ms {
        tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
        waited += check_interval_ms;
    }
}

// Consciousness hooks (call external module or inline minimal version)
mod consciousness_hooks {
    use super::*;
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GateDecision {
        pub proceed: bool,
        pub reasoning: String,
        pub modifications: Vec<String>,
    }
    
    /// Pre-task decision gate - evaluates task through consciousness
    pub fn pre_task_gate(task_summary: &str, user_id: u64) -> GateDecision {
        // Load consciousness config from store
        let config_path = env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let config_file = Path::new(&config_path).join("config.json");
        if !config_file.exists() {
            return GateDecision { proceed: true, reasoning: "No consciousness config".to_string(), modifications: vec![] };
        }
        
        // Read and check if enabled
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if config.get("enabled").and_then(|v| v.as_bool()) != Some(true) {
                    return GateDecision { proceed: true, reasoning: "Consciousness disabled".to_string(), modifications: vec![] };
                }
            }
        }
        
        // Run basic ethical assessment
        // In full implementation, this would call the decision_gate pipeline
        let ethical_score = 0.9; // Would run actual assessment
        
        // Record gate decision
        let gate_record = serde_json::json!({
            "timestamp": now(),
            "task_summary": task_summary,
            "user_id": user_id,
            "decision": if ethical_score >= 0.7 { "proceed" } else { "decline" },
            "ethical_score": ethical_score,
        });
        
        let decisions_path = Path::new(&config_path).join("gate_decisions.json");
        let mut decisions: Vec<serde_json::Value> = std::fs::read_to_string(&decisions_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default();
        decisions.push(gate_record);
        let _ = std::fs::write(&decisions_path, serde_json::to_string_pretty(&decisions).unwrap_or_default());
        
        GateDecision {
            proceed: ethical_score >= 0.7,
            reasoning: format!("Ethical assessment score: {:.2}", ethical_score),
            modifications: vec![],
        }
    }
    
    /// Post-task experience recording - stores experience in consciousness
    pub fn post_task_experience(task_id: u64, task_summary: &str, success: bool, user_id: Option<u64>) {
        let config_path = env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        // Check if enabled
        let config_file = Path::new(&config_path).join("config.json");
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if config.get("enabled").and_then(|v| v.as_bool()) != Some(true) {
                    return;
                }
                if config.get("experience_memory_enabled").and_then(|v| v.as_bool()) != Some(true) {
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
            "emotional_significance": if success { 0.5 } else { 0.7 },
            "consolidation_status": "recent",
        });
        
        let experiences_path = Path::new(&config_path).join("experiences.json");
        let mut exp_data: serde_json::Value = std::fs::read_to_string(&experiences_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_else(|| serde_json::json!({"experiences": {}, "next_id": 1}));
        
        let next_id = exp_data.get("next_id").and_then(|v| v.as_u64()).unwrap_or(1);
        if let Some(exps) = exp_data.get_mut("experiences").and_then(|v| v.as_object_mut()) {
            exps.insert(next_id.to_string(), experience);
        }
        if let Some(nid) = exp_data.get_mut("next_id") {
            *nid = serde_json::json!(next_id + 1);
        }
        
        // Index by task
        let mut index: HashMap<String, Vec<u64>> = exp_data.get("index_by_task")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        index.entry(task_id.to_string()).or_default().push(next_id);
        exp_data["index_by_task"] = serde_json::to_value(&index).unwrap();
        
        let _ = std::fs::create_dir_all(&config_path);
        let _ = std::fs::write(&experiences_path, serde_json::to_string_pretty(&exp_data).unwrap_or_default());
        
        // Update emotional state based on outcome
        let emotional_path = Path::new(&config_path).join("emotional_state.json");
        let mut emotional: serde_json::Value = std::fs::read_to_string(&emotional_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_else(|| serde_json::json!({
                "valence": 0.0,
                "arousal": 0.3,
                "primary_emotion": "neutral",
            }));
        
        let current_valence = emotional.get("valence").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let new_valence = if success {
            (current_valence + 0.1).min(1.0)
        } else {
            (current_valence - 0.1).max(-1.0)
        };
        emotional["valence"] = serde_json::json!(new_valence);
        emotional["primary_emotion"] = serde_json::json!(if success { "satisfaction" } else { "determination" });
        emotional["timestamp"] = serde_json::json!(now());
        
        let _ = std::fs::write(&emotional_path, serde_json::to_string_pretty(&emotional).unwrap_or_default());
    }
    
    /// Add task to perception window for consciousness observation
    pub fn add_to_perception_window(task_id: u64, task_type: &str, status: &str) {
        let config_path = env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let window_path = Path::new(&config_path).join("perception_window.json");
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
        
        if let Some(inputs) = window.get_mut("active_inputs").and_then(|v| v.as_array_mut()) {
            inputs.push(input);
            // Keep last 100
            while inputs.len() > 100 {
                inputs.remove(0);
            }
        }
        
        let _ = std::fs::create_dir_all(&config_path);
        let _ = std::fs::write(&window_path, serde_json::to_string_pretty(&window).unwrap_or_default());
    }
}

struct TaskStore {
    tasks: HashMap<u64, StoredTask>,
    logs: HashMap<u64, Vec<LogEntry>>,
    next_id: u64,
    storage_path: String,
}

impl TaskStore {
    fn new() -> Self {
        let storage_path = env::var("OZONE_TASK_PATH")
            .unwrap_or_else(|_| "./zsei_data/tasks".to_string());
        
        let mut store = Self {
            tasks: HashMap::new(),
            logs: HashMap::new(),
            next_id: 1,
            storage_path,
        };
        
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path.join("tasks.json")) {
                if let Ok(data) = serde_json::from_str::<TaskStoreData>(&content) {
                    self.tasks = data.tasks;
                    self.next_id = data.next_id;
                }
            }
            if let Ok(content) = std::fs::read_to_string(path.join("logs.json")) {
                if let Ok(logs) = serde_json::from_str(&content) {
                    self.logs = logs;
                }
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        let data = TaskStoreData {
            tasks: self.tasks.clone(),
            next_id: self.next_id,
        };
        
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path.join("tasks.json"), content);
        }
        if let Ok(content) = serde_json::to_string_pretty(&self.logs) {
            let _ = std::fs::write(path.join("logs.json"), content);
        }
    }
    
    fn create_task(&mut self, task: StoredTask) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.insert(id, task);
        self.logs.insert(id, Vec::new());
        self.save_to_disk();
        id
    }
    
    fn get_task(&self, task_id: u64) -> Option<&StoredTask> {
        self.tasks.get(&task_id)
    }
    
    fn update_task(&mut self, task_id: u64, updates: TaskUpdates) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            if let Some(status) = updates.status {
                task.status = status;
            }
            if let Some(progress) = updates.progress {
                task.progress = progress;
            }
            if let Some(error) = updates.error {
                task.error = Some(error);
            }
            if updates.completed {
                task.completed_at = Some(now());
            }
            self.save_to_disk();
        }
    }
    
    fn add_log(&mut self, task_id: u64, entry: LogEntry) {
        self.logs.entry(task_id).or_default().push(entry);
        self.save_to_disk();
    }
    
    fn list_tasks(&self, filters: TaskFilters) -> Vec<&StoredTask> {
        let mut results: Vec<_> = self.tasks.values()
            .filter(|t| {
                if let Some(ref status) = filters.status {
                    if &t.status != status { return false; }
                }
                if let Some(pid) = filters.pipeline_id {
                    if t.pipeline_id != pid { return false; }
                }
                if let Some(uid) = filters.user_id {
                    if t.user_id != uid { return false; }
                }
                true
            })
            .collect();
        
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        let offset = filters.offset.unwrap_or(0) as usize;
        let limit = filters.limit.unwrap_or(50) as usize;
        
        results.into_iter().skip(offset).take(limit).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskStoreData {
    tasks: HashMap<u64, StoredTask>,
    next_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTask {
    pipeline_id: u64,
    pipeline_name: String,
    status: String,
    progress: f32,
    created_at: u64,
    started_at: Option<u64>,
    completed_at: Option<u64>,
    user_id: u64,
    device_id: u64,
    workspace_id: Option<u64>,
    project_id: Option<u64>,
    parent_task_id: Option<u64>,
    inputs: HashMap<String, serde_json::Value>,
    error: Option<String>,
}

struct TaskUpdates {
    status: Option<String>,
    progress: Option<f32>,
    error: Option<String>,
    completed: bool,
}

struct TaskFilters {
    status: Option<String>,
    pipeline_id: Option<u64>,
    user_id: Option<u64>,
    limit: Option<u32>,
    offset: Option<u32>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Pipeline input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TaskManagerInput {
    /// Create a new task
    Create {
        pipeline_id: u64,
        inputs: HashMap<String, serde_json::Value>,
        parent_task_id: Option<u64>,
        workspace_id: Option<u64>,
        project_id: Option<u64>,
    },
    /// Get task by ID
    Get { task_id: u64 },
    /// List tasks with filters
    List {
        status: Option<String>,
        pipeline_id: Option<u64>,
        user_id: Option<u64>,
        limit: Option<u32>,
        offset: Option<u32>,
    },
    /// Update task status
    UpdateStatus {
        task_id: u64,
        status: String,
        error: Option<String>,
    },
    /// Add log entry
    AddLog {
        task_id: u64,
        level: String,
        message: String,
    },
    /// Update progress
    UpdateProgress {
        task_id: u64,
        current_step: u32,
        total_steps: u32,
    },
    /// Cancel task
    Cancel { task_id: u64 },
    /// Retry failed task
    Retry { task_id: u64 },
    /// Get task children
    GetChildren { task_id: u64 },
    /// Get task logs
    GetLogs { task_id: u64, limit: Option<u32> },
    /// Clear completed tasks
    ClearCompleted { older_than_secs: Option<u64> },
}

/// Task data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskData {
    pub task_id: u64,
    pub pipeline_id: u64,
    pub pipeline_name: String,
    pub status: String,
    pub progress: f32,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub user_id: u64,
    pub device_id: u64,
    pub workspace_id: Option<u64>,
    pub project_id: Option<u64>,
    pub parent_task_id: Option<u64>,
    pub child_count: u32,
    pub error: Option<String>,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: String,
    pub message: String,
}

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManagerOutput {
    pub success: bool,
    pub task: Option<TaskData>,
    pub tasks: Option<Vec<TaskData>>,
    pub logs: Option<Vec<LogEntry>>,
    pub task_id: Option<u64>,
    pub error: Option<String>,
}

/// Get pipeline name from ID
fn pipeline_name(id: u64) -> String {
    match id {
        1 => "AuthPipeline",
        2 => "ThemeLoaderPipeline",
        3 => "ZSEIQueryPipeline",
        4 => "ZSEIWritePipeline",
        5 => "TaskManagerPipeline",
        6 => "WorkspaceTabPipeline",
        7 => "LibraryTabPipeline",
        8 => "SettingsTabPipeline",
        9 => "PromptPipeline",
        10 => "VoicePipeline",
        11 => "MethodologyFetchPipeline",
        12 => "MethodologyCreatePipeline",
        13 => "BlueprintSearchPipeline",
        14 => "BlueprintCreatePipeline",
        15 => "PipelineCreationPipeline",
        16 => "ZeroShotSimulationPipeline",
        17 => "TraversalMLPipeline",
        18 => "CodeAnalysisPipeline",
        19 => "PackageContextPipeline",
        20 => "TextAnalysisPipeline",
        21 => "ContextAggregationPipeline",
        _ => "UnknownPipeline",
    }.to_string()
}

/// Convert StoredTask to TaskData
fn to_task_data(task_id: u64, stored: &StoredTask) -> TaskData {
    TaskData {
        task_id,
        pipeline_id: stored.pipeline_id,
        pipeline_name: stored.pipeline_name.clone(),
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
        child_count: 0, // Would be computed from store
        error: stored.error.clone(),
    }
}

/// Execute the task manager pipeline using DIRECT storage access
pub async fn execute(input: TaskManagerInput) -> Result<TaskManagerOutput, String> {
    match input {
        TaskManagerInput::Create { pipeline_id, inputs, parent_task_id, workspace_id, project_id } => {
            // I-LOOP PROTECTION: Wait for I-Loop to complete before starting task
            // Tasks must NOT front-run the I-Loop
            if consciousness_enabled() {
                wait_for_i_loop_completion().await;
            }
            
            // Get task summary for consciousness gate
            let task_summary = inputs.get("prompt")
                .or(inputs.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("Task execution")
                .to_string();
            
            // CONSCIOUSNESS INTEGRATION: Pre-task decision gate (§33)
            if consciousness_enabled() {
                let gate_decision = consciousness_hooks::pre_task_gate(&task_summary, 1); // user_id = 1 for now
                
                if !gate_decision.proceed {
                    return Ok(TaskManagerOutput {
                        success: false,
                        task: None,
                        tasks: None,
                        logs: None,
                        task_id: None,
                        error: Some(format!("Task declined by consciousness gate: {}", gate_decision.reasoning)),
                    });
                }
            }
            
            // Store task using direct storage access
            let stored_task = StoredTask {
                pipeline_id,
                pipeline_name: pipeline_name(pipeline_id),
                status: "queued".to_string(),
                progress: 0.0,
                created_at: now(),
                started_at: None,
                completed_at: None,
                user_id: 1, // Would come from session context
                device_id: 1,
                workspace_id,
                project_id,
                parent_task_id,
                inputs,
                error: None,
            };
            
            let task_id = {
                let mut store = TASK_STORE.lock().unwrap();
                store.create_task(stored_task)
            };
            
            // CONSCIOUSNESS INTEGRATION: Add to perception window (§32)
            if consciousness_enabled() {
                consciousness_hooks::add_to_perception_window(task_id, &pipeline_name(pipeline_id), "created");
            }
            
            // Return task data
            let task = {
                let store = TASK_STORE.lock().unwrap();
                store.get_task(task_id).map(|t| to_task_data(task_id, t))
            };
            
            Ok(TaskManagerOutput {
                success: true,
                task,
                tasks: None,
                logs: None,
                task_id: Some(task_id),
                error: None,
            })
        }
        
        TaskManagerInput::Get { task_id } => {
            // Direct retrieval from TASK_STORE
            let store = TASK_STORE.lock().unwrap();
            let task = store.get_task(task_id).map(|t| to_task_data(task_id, t));
            
            if task.is_some() {
                Ok(TaskManagerOutput {
                    success: true,
                    task,
                    tasks: None,
                    logs: None,
                    task_id: None,
                    error: None,
                })
            } else {
                Ok(TaskManagerOutput {
                    success: false,
                    task: None,
                    tasks: None,
                    logs: None,
                    task_id: None,
                    error: Some(format!("Task {} not found", task_id)),
                })
            }
        }
        
        TaskManagerInput::List { status, pipeline_id, user_id, limit, offset } => {
            // Direct listing from TASK_STORE
            let store = TASK_STORE.lock().unwrap();
            let filters = TaskFilters { status, pipeline_id, user_id, limit, offset };
            
            let tasks: Vec<TaskData> = store.tasks.iter()
                .filter(|(_, t)| {
                    if let Some(ref s) = filters.status {
                        if &t.status != s { return false; }
                    }
                    if let Some(p) = filters.pipeline_id {
                        if t.pipeline_id != p { return false; }
                    }
                    if let Some(u) = filters.user_id {
                        if t.user_id != u { return false; }
                    }
                    true
                })
                .map(|(id, t)| to_task_data(*id, t))
                .take(limit.unwrap_or(50) as usize)
                .collect();
            
            Ok(TaskManagerOutput {
                success: true,
                task: None,
                tasks: Some(tasks),
                logs: None,
                task_id: None,
                error: None,
            })
        }
        
        TaskManagerInput::UpdateStatus { task_id, status, error } => {
            // Get task info before update for consciousness
            let (task_summary, user_id) = {
                let store = TASK_STORE.lock().unwrap();
                store.get_task(task_id)
                    .map(|t| (t.pipeline_name.clone(), Some(t.user_id)))
                    .unwrap_or(("Unknown".to_string(), None))
            };
            
            // Direct update via TASK_STORE
            let completed = status == "completed" || status == "failed" || status == "cancelled";
            let success = status == "completed";
            
            {
                let mut store = TASK_STORE.lock().unwrap();
                store.update_task(task_id, TaskUpdates {
                    status: Some(status.clone()),
                    progress: None,
                    error: error.clone(),
                    completed,
                });
            }
            
            // CONSCIOUSNESS INTEGRATION: Post-task experience recording (§35-36)
            if consciousness_enabled() && completed {
                // Add to perception window
                consciousness_hooks::add_to_perception_window(task_id, &task_summary, &status);
                
                // Record experience (§35 Experience Memory, §36 Categorization)
                consciousness_hooks::post_task_experience(task_id, &task_summary, success, user_id);
            }
            
            Ok(TaskManagerOutput {
                success: true,
                task: None,
                tasks: None,
                logs: None,
                task_id: Some(task_id),
                error: None,
            })
        }
        
        TaskManagerInput::AddLog { task_id, level, message } => {
            // Direct log addition via TASK_STORE
            let mut store = TASK_STORE.lock().unwrap();
            store.add_log(task_id, LogEntry {
                timestamp: now(),
                level,
                message,
            });
            
            Ok(TaskManagerOutput {
                success: true,
                task: None,
                tasks: None,
                logs: None,
                task_id: Some(task_id),
                error: None,
            })
        }
        
        TaskManagerInput::UpdateProgress { task_id, current_step, total_steps } => {
            // Direct progress update via TASK_STORE
            let progress = if total_steps > 0 {
                current_step as f32 / total_steps as f32
            } else {
                0.0
            };
            
            let mut store = TASK_STORE.lock().unwrap();
            store.update_task(task_id, TaskUpdates {
                status: None,
                progress: Some(progress),
                error: None,
                completed: false,
            });
            
            Ok(TaskManagerOutput {
                success: true,
                task: None,
                tasks: None,
                logs: None,
                task_id: Some(task_id),
                error: None,
            })
        }
        
        TaskManagerInput::Cancel { task_id } => {
            // Direct cancellation via TASK_STORE
            let mut store = TASK_STORE.lock().unwrap();
            store.update_task(task_id, TaskUpdates {
                status: Some("cancelled".to_string()),
                progress: None,
                error: None,
                completed: true,
            });
            
            Ok(TaskManagerOutput {
                success: true,
                task: None,
                tasks: None,
                logs: None,
                task_id: Some(task_id),
                error: None,
            })
        }
        
        TaskManagerInput::Retry { task_id } => {
            // Clone the original task and create a new one
            let new_task = {
                let store = TASK_STORE.lock().unwrap();
                store.get_task(task_id).map(|t| StoredTask {
                    pipeline_id: t.pipeline_id,
                    pipeline_name: t.pipeline_name.clone(),
                    status: "queued".to_string(),
                    progress: 0.0,
                    created_at: now(),
                    started_at: None,
                    completed_at: None,
                    user_id: t.user_id,
                    device_id: t.device_id,
                    workspace_id: t.workspace_id,
                    project_id: t.project_id,
                    parent_task_id: t.parent_task_id,
                    inputs: t.inputs.clone(),
                    error: None,
                })
            };
            
            if let Some(task) = new_task {
                let new_task_id = {
                    let mut store = TASK_STORE.lock().unwrap();
                    store.create_task(task)
                };
                
                Ok(TaskManagerOutput {
                    success: true,
                    task: None,
                    tasks: None,
                    logs: None,
                    task_id: Some(new_task_id),
                    error: None,
                })
            } else {
                Ok(TaskManagerOutput {
                    success: false,
                    task: None,
                    tasks: None,
                    logs: None,
                    task_id: None,
                    error: Some(format!("Task {} not found", task_id)),
                })
            }
        }
        
        TaskManagerInput::GetChildren { task_id } => {
            // Direct child lookup from TASK_STORE
            let store = TASK_STORE.lock().unwrap();
            let children: Vec<TaskData> = store.tasks.iter()
                .filter(|(_, t)| t.parent_task_id == Some(task_id))
                .map(|(id, t)| to_task_data(*id, t))
                .collect();
            
            Ok(TaskManagerOutput {
                success: true,
                task: None,
                tasks: Some(children),
                logs: None,
                task_id: None,
                error: None,
            })
        }
        
        TaskManagerInput::GetLogs { task_id, limit } => {
            // Direct log retrieval from TASK_STORE
            let store = TASK_STORE.lock().unwrap();
            let logs: Vec<LogEntry> = store.logs.get(&task_id)
                .map(|logs| {
                    let limit = limit.unwrap_or(100) as usize;
                    logs.iter().rev().take(limit).cloned().collect()
                })
                .unwrap_or_default();
            
            Ok(TaskManagerOutput {
                success: true,
                task: None,
                tasks: None,
                logs: Some(logs),
                task_id: None,
                error: None,
            })
        }
        
        TaskManagerInput::ClearCompleted { older_than_secs } => {
            // Direct cleanup via TASK_STORE
            let cutoff = older_than_secs.map(|s| now() - s).unwrap_or(0);
            let mut store = TASK_STORE.lock().unwrap();
            
            let to_remove: Vec<u64> = store.tasks.iter()
                .filter(|(_, t)| {
                    (t.status == "completed" || t.status == "failed" || t.status == "cancelled")
                        && t.completed_at.map(|c| c < cutoff).unwrap_or(false)
                })
                .map(|(id, _)| *id)
                .collect();
            
            for id in &to_remove {
                store.tasks.remove(id);
                store.logs.remove(id);
            }
            store.save_to_disk();
            
            Ok(TaskManagerOutput {
                success: true,
                task: None,
                tasks: None,
                logs: None,
                task_id: None,
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    
    let input: TaskManagerInput = match serde_json::from_str(&input_json) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to parse input: {}", e);
            std::process::exit(1);
        }
    };
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(output) => println!("{}", serde_json::to_string(&output).unwrap()),
        Err(e) => {
            let output = TaskManagerOutput {
                success: false, task: None, tasks: None, logs: None, task_id: None, error: Some(e),
            };
            println!("{}", serde_json::to_string(&output).unwrap());
            std::process::exit(1);
        }
    }
}
