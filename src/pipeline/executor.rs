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
}

impl PipelineExecutor {
    /// Create new executor
    pub fn new(config: &PipelineConfig) -> OzoneResult<Self> {
        Ok(Self {
            builtin_path: PathBuf::from(&config.builtin_path),
            custom_path: PathBuf::from(&config.custom_path),
            max_concurrent: config.max_concurrent_pipelines,
            running_count: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// Execute a pipeline
    pub async fn execute(
        &self,
        blueprint: &PipelineBlueprint,
        input: PipelineInput,
        task_id: Option<TaskID>, // ← optional
    ) -> OzoneResult<PipelineOutput> {
        let execution_id = ExecutionID::new();
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

        let result = self
            .execute_inner(blueprint, input, execution_id, task_id)
            .await;

        self.running_count
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);

        // Wrap result with execution_id and task_id
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
        // Find the pipeline executable/module
        let (category, pipeline_name) = self.get_builtin_info(pipeline_id);

        // Build path: pipelines/{category}/{name}/main.rs (or compiled binary)
        let pipeline_dir = self.builtin_path.join(category).join(&pipeline_name);

        // Look for compiled binary first, then source
        let executable = if cfg!(windows) {
            pipeline_dir.join(format!("{}.exe", pipeline_name))
        } else {
            pipeline_dir.join(&pipeline_name)
        };

        let pipeline_path = if executable.exists() {
            executable
        } else {
            // Fall back to source file (will be interpreted/compiled)
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

    /// Get builtin pipeline category and name from ID
    /// Uses the central registry - NO HARDCODING
    fn get_builtin_info(&self, pipeline_id: PipelineID) -> (&'static str, String) {
        use crate::pipeline::registry::{
            get_pipeline_category, get_pipeline_folder, get_pipeline_info,
        };

        if let Some(info) = get_pipeline_info(pipeline_id) {
            (info.category, info.folder_name.clone())
        } else {
            // Fallback for unknown pipelines
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
        // Find custom pipeline
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

        tracing::debug!(
            execution_id = %execution_id,
            path = ?pipeline_path,
            extension = extension,
            "Preparing to invoke pipeline binary/script"
        );

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

        // Only pass task-id if we actually have one
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
}
