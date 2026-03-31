//! Pipeline executor - invokes actual pipeline implementations
//!
//! The executor calls the actual pipeline code which lives in pipelines/ directory.
//! This maintains separation between core (here) and pipeline logic (pipelines/).

use crate::config::PipelineConfig;
use crate::types::pipeline::ExecutionID;
use crate::types::{
    BuiltinPipeline, OzoneError, OzoneResult, PipelineBlueprint, PipelineID, PipelineInput,
    PipelineOutput, TaskID, Value,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ProgressStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PipelineProgress {
    pub execution_id: String,
    pub pipeline_id: PipelineID,
    pub pipeline_name: String, // human-readable for UI
    pub task_id: Option<TaskID>,
    pub step_index: Option<u32>, // which blueprint step triggered this
    pub status: ProgressStatus,
    pub progress_percent: u8,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub tokens_used: Option<u32>,
    pub error: Option<String>,
}

/// Pipeline executor
pub struct PipelineExecutor {
    /// Path to builtin pipelines
    builtin_path: PathBuf,

    /// Path to custom pipelines
    custom_path: PathBuf,

    /// Maximum concurrent pipelines
    max_concurrent: usize,

    /// Currently running pipeline count
    running_count: std::sync::atomic::AtomicUsize,

    progress_map: Arc<tokio::sync::RwLock<std::collections::HashMap<String, PipelineProgress>>>,
    cancel_set: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
}

impl PipelineExecutor {
    /// Create new executor
    pub fn new(config: &PipelineConfig) -> OzoneResult<Self> {
        Ok(Self {
            builtin_path: PathBuf::from(&config.builtin_path),
            custom_path: PathBuf::from(&config.custom_path),
            max_concurrent: config.max_concurrent_pipelines,
            running_count: std::sync::atomic::AtomicUsize::new(0),
            progress_map: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            cancel_set: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
        })
    }

    /// Execute a pipeline
    pub async fn execute(
        &self,
        blueprint: &PipelineBlueprint,
        input: PipelineInput,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let execution_id = ExecutionID::new();
        let execution_id_str = execution_id.as_str().to_string();

        tracing::info!(
            pipeline = %blueprint.name,
            execution_id = %execution_id,
            task_id = ?task_id,
            "Starting pipeline execution"
        );

        let current = self
            .running_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if current >= self.max_concurrent {
            self.running_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            tracing::warn!(
                execution_id = %execution_id,
                "Pipeline execution rejected: max concurrent ({}) reached",
                self.max_concurrent
            );
            return Err(OzoneError::PipelineError(format!(
                "Maximum concurrent pipelines ({}) reached",
                self.max_concurrent
            )));
        }

        // Register progress early
        let pipeline_name = crate::pipeline::registry::get_pipeline_info(blueprint.pipeline_id)
            .map(|info| info.name.clone())
            .unwrap_or_else(|| blueprint.name.clone());

        {
            let mut map = self.progress_map.write().await;
            map.insert(
                execution_id_str.clone(),
                PipelineProgress {
                    execution_id: execution_id_str.clone(),
                    pipeline_id: blueprint.pipeline_id,
                    pipeline_name,
                    task_id,
                    step_index: None, // Will be set by orchestrator via set_step_context()
                    status: ProgressStatus::Running,
                    progress_percent: 0,
                    started_at: now_secs(), // assume now_secs() helper exists
                    completed_at: None,
                    tokens_used: None,
                    error: None,
                },
            );
        }

        {
            let cancelled = self.cancel_set.read().await;
            if cancelled.contains(execution_id_str) {
                self.running_count
                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                let mut map = self.progress_map.write().await;
                if let Some(p) = map.get_mut(execution_id_str) {
                    p.status = ProgressStatus::Cancelled;
                    p.completed_at = Some(now_secs());
                }
                return Err(OzoneError::PipelineError(format!(
                    "Execution {} cancelled",
                    execution_id
                )));
            }
        }

        let result = self
            .execute_inner(blueprint, input, execution_id, task_id)
            .await;

        self.running_count
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);

        // Update progress on completion
        let final_status = if result.is_ok() {
            ProgressStatus::Completed
        } else {
            ProgressStatus::Failed
        };

        {
            let mut map = self.progress_map.write().await;
            if let Some(progress) = map.get_mut(&execution_id_str) {
                progress.status = final_status;
                progress.progress_percent = 100;
                progress.completed_at = Some(now_secs());
                if let Err(ref e) = result {
                    progress.error = Some(e.to_string());
                }
            }
        }

        // Wrap result
        match result {
            Ok(mut output) => {
                output.execution_id = execution_id;
                output.task_id = task_id;
                Ok(output)
            }
            Err(e) => Err(e),
        }
    }

    /// Internal execution logic
    async fn execute_inner(
        &self,
        blueprint: &PipelineBlueprint,
        input: PipelineInput,
        execution_id: ExecutionID,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let pipeline_id = blueprint.pipeline_id;

        tracing::debug!(
            execution_id = %execution_id,
            pipeline_id = pipeline_id,
            "Executing inner pipeline logic"
        );

        if self.is_builtin(pipeline_id) {
            self.execute_builtin(pipeline_id, input, execution_id, task_id)
                .await
        } else {
            self.execute_custom(blueprint, input, execution_id, task_id)
                .await
        }
    }

    /// Check if pipeline is builtin
    fn is_builtin(&self, pipeline_id: PipelineID) -> bool {
        pipeline_id <= 54 // Builtin IDs are 1-54
    }

    /// Execute a builtin pipeline
    async fn execute_builtin(
        &self,
        pipeline_id: PipelineID,
        input: PipelineInput,
        execution_id: ExecutionID,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let (category, pipeline_name) = self.get_builtin_info(pipeline_id);

        let pipeline_dir = self.builtin_path.join(category).join(&pipeline_name);

        let executable = if cfg!(windows) {
            pipeline_dir.join(format!("{}.exe", pipeline_name))
        } else {
            pipeline_dir.join(&pipeline_name)
        };

        let pipeline_path = if executable.exists() {
            executable
        } else {
            let main_rs = pipeline_dir.join("main.rs");
            if main_rs.exists() {
                main_rs
            } else {
                tracing::error!(
                    execution_id = %execution_id,
                    pipeline_name = %pipeline_name,
                    path = ?pipeline_dir,
                    "Builtin pipeline implementation not found"
                );
                return Ok(PipelineOutput {
                    execution_id,
                    task_id,
                    data: {
                        let mut map = HashMap::new();
                        map.insert(
                            "error".into(),
                            serde_json::Value::String(format!(
                                "Pipeline {} not found",
                                pipeline_name
                            )),
                        );
                        map
                    },
                    success: false,
                    error: Some(format!(
                        "Pipeline {} implementation not found",
                        pipeline_name
                    )),
                });
            }
        };

        tracing::info!(
            execution_id = %execution_id,
            pipeline_path = ?pipeline_path,
            "Invoking builtin pipeline"
        );

        self.invoke_pipeline(&pipeline_path, input, execution_id, task_id)
            .await
    }

    /// Get builtin pipeline category and name from ID (uses central registry)
    fn get_builtin_info(&self, pipeline_id: PipelineID) -> (&'static str, String) {
        use crate::pipeline::registry::get_pipeline_info;

        if let Some(info) = get_pipeline_info(pipeline_id) {
            (info.category, info.folder_name.clone())
        } else {
            tracing::warn!(
                "Pipeline {} not found in registry, using fallback path",
                pipeline_id
            );
            ("core", format!("pipeline_{}", pipeline_id))
        }
    }

    /// Execute a custom pipeline
    async fn execute_custom(
        &self,
        blueprint: &PipelineBlueprint,
        input: PipelineInput,
        execution_id: ExecutionID,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let pipeline_path = self.custom_path.join(&blueprint.name);

        if !pipeline_path.exists() {
            tracing::error!(
                execution_id = %execution_id,
                custom_path = ?self.custom_path,
                blueprint_name = %blueprint.name,
                "Custom pipeline not found"
            );
            return Err(OzoneError::NotFound(format!(
                "Custom pipeline {} not found (execution {})",
                blueprint.name, execution_id
            )));
        }

        tracing::info!(
            execution_id = %execution_id,
            pipeline_path = ?pipeline_path,
            "Invoking custom pipeline"
        );

        self.invoke_pipeline(&pipeline_path, input, execution_id, task_id)
            .await
    }

    /// Invoke a pipeline executable/script
    async fn invoke_pipeline(
        &self,
        pipeline_path: &PathBuf,
        input: PipelineInput,
        execution_id: ExecutionID,
        task_id: Option<TaskID>,
    ) -> OzoneResult<PipelineOutput> {
        let input_json = serde_json::to_string(&input)
            .map_err(|e| OzoneError::SerializationError(e.to_string()))?;

        let extension = pipeline_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mut cmd = match extension {
            "rs" | "" => Command::new(pipeline_path),
            "py" => {
                let mut c = Command::new("python3");
                c.arg(pipeline_path);
                c
            }
            "js" | "ts" => {
                let mut c = Command::new("node");
                c.arg(pipeline_path);
                c
            }
            _ => {
                return Err(OzoneError::PipelineError(format!(
                    "Unsupported pipeline type: {} (execution {})",
                    extension, execution_id
                )));
            }
        };

        cmd.arg("--input")
            .arg(&input_json)
            .arg("--execution-id")
            .arg(execution_id.as_str());

        if let Some(tid) = task_id {
            cmd.arg("--task-id").arg(tid.to_string());
        }

        let output = cmd.output().map_err(|e| {
            OzoneError::PipelineError(format!(
                "Failed to execute pipeline (execution {}): {}",
                execution_id, e
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!(
                execution_id = %execution_id,
                stderr = %stderr,
                "Pipeline execution failed (non-zero exit)"
            );
            return Ok(PipelineOutput {
                execution_id,
                task_id,
                data: HashMap::new(),
                success: false,
                error: Some(stderr.into()),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let output_data: HashMap<String, serde_json::Value> = serde_json::from_str(&stdout)
            .unwrap_or_else(|parse_err| {
                tracing::warn!(
                    execution_id = %execution_id,
                    parse_error = %parse_err,
                    "Pipeline output not valid JSON — using raw stdout"
                );
                let mut map = HashMap::new();
                map.insert(
                    "raw_output".into(),
                    serde_json::Value::String(stdout.into()),
                );
                map
            });

        tracing::info!(
            execution_id = %execution_id,
            success = true,
            "Pipeline execution completed successfully"
        );

        Ok(PipelineOutput {
            execution_id,
            task_id,
            data: output_data,
            success: true,
            error: None,
        })
    }

    /// Called by the orchestrator before/during step execution to link this execution to a specific task step
    pub async fn set_step_context(&self, execution_id: &str, task_id: TaskID, step_index: u32) {
        let mut map = self.progress_map.write().await;
        if let Some(p) = map.get_mut(execution_id) {
            p.task_id = Some(task_id);
            p.step_index = Some(step_index);
        }
    }

    /// Get progress for an execution
    pub async fn get_progress(&self, execution_id: &str) -> Option<PipelineProgress> {
        let map = self.progress_map.read().await;
        map.get(execution_id).cloned()
    }

    /// Request cancellation of an execution
    pub async fn cancel(&self, execution_id: &str) -> bool {
        let map = self.progress_map.read().await;
        if map
            .get(execution_id)
            .map(|p| matches!(p.status, ProgressStatus::Running | ProgressStatus::Queued))
            .unwrap_or(false)
        {
            drop(map);
            let mut cancel_set = self.cancel_set.write().await;
            cancel_set.insert(execution_id.to_string());
            true
        } else {
            false
        }
    }

    /// Clean up completed executions older than TTL
    pub async fn cleanup_old_progress(&self, ttl_secs: u64) {
        let now = now_secs();
        let mut map = self.progress_map.write().await;
        map.retain(|_, p| match p.status {
            ProgressStatus::Running | ProgressStatus::Queued => true,
            _ => p.completed_at.map(|t| now - t < ttl_secs).unwrap_or(false),
        });

        let mut cancel_set = self.cancel_set.write().await;
        cancel_set.retain(|id| map.contains_key(id));
    }

    /// Get shared progress map reference (for HTTP handler / UI access)
    pub fn progress_map(
        &self,
    ) -> Arc<tokio::sync::RwLock<std::collections::HashMap<String, PipelineProgress>>> {
        self.progress_map.clone()
    }
}

// Helper function - you should define this in a common utils module if not already present
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
