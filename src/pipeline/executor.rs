//! Pipeline executor - invokes actual pipeline implementations
//!
//! The executor calls the actual pipeline code which lives in pipelines/ directory.
//! This maintains separation between core (here) and pipeline logic (pipelines/).

use crate::config::PipelineConfig;
use crate::types::{
    PipelineID, TaskID, OzoneError, OzoneResult,
    PipelineInput, PipelineOutput, PipelineBlueprint,
    BuiltinPipeline, Value,
};
use std::collections::HashMap;
use std::process::Command;
use std::path::PathBuf;

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
        task_id: TaskID,
    ) -> OzoneResult<PipelineOutput> {
        // Check concurrency limit
        let current = self.running_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if current >= self.max_concurrent {
            self.running_count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            return Err(OzoneError::PipelineError(
                "Maximum concurrent pipelines reached".into()
            ));
        }
        
        let result = self.execute_inner(blueprint, input, task_id).await;
        
        self.running_count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        
        result
    }
    
    /// Internal execution
    async fn execute_inner(
        &self,
        blueprint: &PipelineBlueprint,
        input: PipelineInput,
        task_id: TaskID,
    ) -> OzoneResult<PipelineOutput> {
        let pipeline_id = blueprint.pipeline_id;
        
        tracing::info!("Executing pipeline {} (task {})", blueprint.name, task_id);
        
        // Determine execution method based on pipeline type
        if self.is_builtin(pipeline_id) {
            self.execute_builtin(pipeline_id, input, task_id).await
        } else {
            self.execute_custom(blueprint, input, task_id).await
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
        task_id: TaskID,
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
                tracing::error!("Pipeline {} not found at {:?}", pipeline_name, pipeline_dir);
                return Ok(PipelineOutput {
                    data: {
                        let mut map = HashMap::new();
                        map.insert(
                            "error".into(),
                            Value::String(format!("Pipeline {} not found", pipeline_name))
                        );
                        map
                    },
                    task_id,
                    success: false,
                    error: Some(format!("Pipeline {} implementation not found", pipeline_name)),
                });
            }
        };
        
        // Execute the pipeline
        self.invoke_pipeline(&pipeline_path, input, task_id).await
    }
    
    /// Get builtin pipeline category and name from ID
    fn get_builtin_info(&self, pipeline_id: PipelineID) -> (&'static str, String) {
        let (category, name) = match pipeline_id {
            1 => ("core", "auth"),
            2 => ("core", "theme_loader"),
            3 => ("core", "zsei_query"),
            4 => ("core", "zsei_write"),
            5 => ("core", "task_manager"),
            6 => ("core", "workspace_tab"),
            7 => ("core", "library_tab"),
            8 => ("core", "settings_tab"),
            9 => ("core", "prompt"),
            10 => ("core", "voice"),
            11 => ("core", "methodology_fetch"),
            12 => ("core", "methodology_create"),
            13 => ("core", "blueprint_search"),
            14 => ("core", "blueprint_create"),
            15 => ("core", "pipeline_creation"),
            16 => ("core", "zero_shot_simulation"),
            17 => ("core", "traversal_ml"),
            18 => ("core", "code_analysis"),
            19 => ("core", "package_context"),
            20 => ("core", "text_analysis"),
            21 => ("core", "context_aggregation"),
            22 => ("core", "graph_visualization"),
            23 => ("core", "task_recommendation"),
            24 => ("core", "reordering"),
            25 => ("core", "browser_navigation"),
            26 => ("core", "integrity_check"),
            27 => ("core", "consensus"),
            28 => ("core", "external_reference"),
            29 => ("core", "package_relationship"),
            30 => ("core", "file_link"),
            31 => ("core", "url_link"),
            32 => ("core", "package_link"),
            33 => ("core", "sync"),
            34 => ("core", "device_register"),
            35 => ("core", "home_return"),
            36 => ("core", "task_viewer"),
            37 => ("core", "log_viewer"),
            38 => ("core", "device_status"),
            // Consciousness pipelines
            39 => ("consciousness", "decision_gate"),
            40 => ("consciousness", "emotional_state"),
            41 => ("consciousness", "experience_memory"),
            42 => ("consciousness", "reflection"),
            43 => ("consciousness", "self_model"),
            44 => ("consciousness", "consciousness_query"),
            45 => ("consciousness", "experience_playback"),
            46 => ("consciousness", "emotional_response"),
            47 => ("consciousness", "consciousness_sync"),
            48 => ("consciousness", "collective_consciousness"),
            49 => ("consciousness", "consciousness_integrity"),
            50 => ("consciousness", "consciousness_config"),
            51 => ("consciousness", "experience_search"),
            52 => ("consciousness", "emotional_calibration"),
            53 => ("consciousness", "self_awareness"),
            54 => ("consciousness", "consciousness_dashboard"),
            _ => ("core", "unknown"),
        };
        (category, name.to_string())
    }
    
    /// Execute a custom pipeline
    async fn execute_custom(
        &self,
        blueprint: &PipelineBlueprint,
        input: PipelineInput,
        task_id: TaskID,
    ) -> OzoneResult<PipelineOutput> {
        // Find custom pipeline
        let pipeline_path = self.custom_path.join(&blueprint.name);
        
        if !pipeline_path.exists() {
            return Err(OzoneError::NotFound(
                format!("Custom pipeline {} not found", blueprint.name)
            ));
        }
        
        self.invoke_pipeline(&pipeline_path, input, task_id).await
    }
    
    /// Invoke a pipeline executable/script
    async fn invoke_pipeline(
        &self,
        pipeline_path: &PathBuf,
        input: PipelineInput,
        task_id: TaskID,
    ) -> OzoneResult<PipelineOutput> {
        // Serialize input to JSON
        let input_json = serde_json::to_string(&input)
            .map_err(|e| OzoneError::SerializationError(e.to_string()))?;
        
        // Determine how to execute based on file extension
        let extension = pipeline_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        let output = match extension {
            "rs" | "" => {
                // Rust executable
                Command::new(pipeline_path)
                    .arg("--input")
                    .arg(&input_json)
                    .arg("--task-id")
                    .arg(task_id.to_string())
                    .output()
                    .map_err(|e| OzoneError::PipelineError(e.to_string()))?
            }
            "py" => {
                // Python script
                Command::new("python3")
                    .arg(pipeline_path)
                    .arg("--input")
                    .arg(&input_json)
                    .arg("--task-id")
                    .arg(task_id.to_string())
                    .output()
                    .map_err(|e| OzoneError::PipelineError(e.to_string()))?
            }
            "js" | "ts" => {
                // JavaScript/TypeScript
                Command::new("node")
                    .arg(pipeline_path)
                    .arg("--input")
                    .arg(&input_json)
                    .arg("--task-id")
                    .arg(task_id.to_string())
                    .output()
                    .map_err(|e| OzoneError::PipelineError(e.to_string()))?
            }
            _ => {
                return Err(OzoneError::PipelineError(
                    format!("Unsupported pipeline type: {}", extension)
                ));
            }
        };
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(PipelineOutput {
                data: HashMap::new(),
                task_id,
                success: false,
                error: Some(stderr.into()),
            });
        }
        
        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let output_data: HashMap<String, Value> = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| {
                let mut map = HashMap::new();
                map.insert("raw_output".into(), Value::String(stdout.into()));
                map
            });
        
        Ok(PipelineOutput {
            data: output_data,
            task_id,
            success: true,
            error: None,
        })
    }
    
}
