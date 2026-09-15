//! Pipeline system - The execution engine
//!
//! Based on Section 10 of the specification.
//!
//! # Key Principle
//!
//! Pipelines are SEPARATE from core. The core only:
//! - Registers pipeline metadata
//! - Executes pipelines by invoking them
//! - Tracks execution via TaskManager
//!
//! Pipeline LOGIC lives in the pipelines/ directory, not here.

mod executor;
pub(crate) mod registry;
mod store;

pub mod remote;
pub use executor::*;
pub use registry::*;
pub use remote::{RemotePipeline, RemotePipelineInfo, RemotePipelines};
pub use store::*;

use crate::config::PipelineConfig;
use crate::types::pipeline::{
    BuiltinPipeline, PipelineBlueprint, PipelineInput, PipelineOutput, Schema,
};
use crate::types::{OzoneError, OzoneResult, PipelineID, TaskID};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Pipeline registry - manages available pipelines
pub struct PipelineRegistry {
    /// Configuration
    config: PipelineConfig,

    /// Registered pipeline blueprints (metadata only)
    blueprints: Arc<RwLock<HashMap<PipelineID, PipelineBlueprint>>>,

    /// Pipeline executor
    executor: PipelineExecutor,

    /// Builtin pipeline path
    builtin_path: PathBuf,

    /// Custom pipeline path
    custom_path: PathBuf,
}

impl PipelineRegistry {
    /// Create new pipeline registry
    pub fn new(config: &PipelineConfig) -> OzoneResult<Self> {
        let builtin_path = PathBuf::from(&config.builtin_path);
        let custom_path = PathBuf::from(&config.custom_path);

        // Ensure directories exist
        std::fs::create_dir_all(&builtin_path).map_err(|e| {
            OzoneError::PipelineError(format!("Failed to create builtin dir: {}", e))
        })?;
        std::fs::create_dir_all(&custom_path).map_err(|e| {
            OzoneError::PipelineError(format!("Failed to create custom dir: {}", e))
        })?;

        let executor = PipelineExecutor::new(config)?;

        // load runtime pipeline registry from index.json
        let index_path = std::path::PathBuf::from(&config.index_path);
        // Self-heal: installs that completed bootstrap before this file ever
        // existed (bootstrap only runs once, gated on setup_complete) would
        // otherwise be stuck forever with an empty runtime pipeline table —
        // regenerate it in place rather than requiring a fresh bootstrap.
        if !index_path.exists() {
            if let Some(parent) = index_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let index = crate::bootstrap::BootstrapManager::get_default_pipeline_index();
            match serde_json::to_string_pretty(&index) {
                Ok(content) => match std::fs::write(&index_path, content) {
                    Ok(()) => tracing::info!(
                        "Generated missing pipeline index at {} (self-heal)",
                        index_path.display()
                    ),
                    Err(e) => tracing::warn!("Failed to write pipeline index: {}", e),
                },
                Err(e) => tracing::warn!("Failed to serialize default pipeline index: {}", e),
            }
        }
        if index_path.exists() {
            if let Err(e) = registry::load_pipeline_registry_from_index(&index_path) {
                tracing::warn!(
                    "Failed to load pipeline registry from index (using compile-time fallback): {}",
                    e
                );
            } else {
                tracing::info!("Pipeline registry loaded from index.json");
            }
        }

        // Build blueprints HashMap during initialization (before wrapping in Arc)
        let mut blueprints_map = HashMap::new();

        // Load builtin pipelines into the map
        Self::load_builtin_pipelines_into(&mut blueprints_map)?;

        // The compile-time PIPELINE_INFO table above only covers ids 1-55
        // (general + consciousness). Modality pipelines (100+) and anything
        // else index.json knows about were NEVER added to this gate map —
        // meaning every dispatch to them (RegistryExecutorAdapter::execute /
        // PipelineRegistry::execute, both check this map before remote-or-
        // builtin dispatch is even attempted) has always failed with
        // "Pipeline {id} not found", regardless of whether a builtin binary
        // for that pipeline actually exists and would otherwise run fine.
        // Confirmed live: pipeline 100 (text modality), called as the very
        // first real pipeline in every orchestration's Stage 2, failing
        // instantly on every single /orchestrate request.
        let mut seeded_from_index = 0usize;
        for id in registry::get_runtime_pipeline_ids() {
            if blueprints_map.contains_key(&id) {
                continue;
            }
            let (name, description) = registry::get_pipeline_info(id)
                .map(|info| (info.name.clone(), info.description.clone()))
                .unwrap_or_else(|| (format!("Pipeline {}", id), String::new()));
            blueprints_map.insert(
                id,
                PipelineBlueprint {
                    pipeline_id: id,
                    name,
                    version: crate::types::SemVer::default(),
                    author: Vec::new(),
                    description,
                    specification: crate::types::pipeline::BlueprintSpec {
                        input_schema: Schema::default(),
                        output_schema: Schema::default(),
                        dependencies: Vec::new(),
                        sub_pipelines: Vec::new(),
                        execution_flow: crate::types::pipeline::ExecutionFlow::Sequential(Vec::new()),
                    },
                    implementations: Vec::new(),
                    content_hash: [0u8; 32],
                    peers: Vec::new(),
                    consensus_status: crate::types::pipeline::ConsensusStatus::Accepted,
                    verified_by: 0,
                },
            );
            seeded_from_index += 1;
        }
        if seeded_from_index > 0 {
            tracing::info!(
                "Seeded {} additional pipelines from index.json into the execution gate (modality pipelines etc. outside the compile-time 1-55 range)",
                seeded_from_index
            );
        }

        tracing::info!("Loaded {} builtin pipelines", blueprints_map.len());

        Ok(Self {
            config: config.clone(),
            blueprints: Arc::new(RwLock::new(blueprints_map)),
            executor,
            builtin_path,
            custom_path,
        })
    }

    /// Load builtin pipeline metadata into a HashMap (called during initialization)
    /// Uses PIPELINE_INFO from registry as the single source of truth
    fn load_builtin_pipelines_into(
        blueprints: &mut HashMap<PipelineID, PipelineBlueprint>,
    ) -> OzoneResult<()> {
        tracing::info!("Loading builtin pipelines from registry");

        // Use PIPELINE_INFO from registry - THE SOURCE OF TRUTH
        // This avoids hardcoding pipeline lists multiple times
        for (id, info) in registry::PIPELINE_INFO.iter() {
            let blueprint = PipelineBlueprint {
                pipeline_id: *id,
                name: info.name.clone(),
                version: crate::types::SemVer::default(),
                author: Vec::new(), // System
                description: info.description.clone(),
                specification: crate::types::pipeline::BlueprintSpec {
                    input_schema: Schema::default(),
                    output_schema: Schema::default(),
                    dependencies: Vec::new(),
                    sub_pipelines: Vec::new(),
                    execution_flow: crate::types::pipeline::ExecutionFlow::Sequential(Vec::new()),
                },
                implementations: Vec::new(),
                content_hash: [0u8; 32],
                peers: Vec::new(),
                consensus_status: crate::types::pipeline::ConsensusStatus::Accepted,
                verified_by: 0,
            };
            blueprints.insert(*id, blueprint);
        }

        Ok(())
    }

    /// DEPRECATED: Use PIPELINE_INFO from registry instead
    /// Kept for backward compatibility
    #[deprecated(note = "Use registry::PIPELINE_INFO instead")]
    fn register_builtin_into(
        builtin: BuiltinPipeline,
        blueprints: &mut HashMap<PipelineID, PipelineBlueprint>,
    ) -> OzoneResult<()> {
        let blueprint = PipelineBlueprint {
            pipeline_id: builtin.id(),
            name: builtin.name().into(),
            version: crate::types::SemVer::default(),
            author: Vec::new(), // System
            description: format!("Built-in {} pipeline", builtin.name()),
            specification: crate::types::pipeline::BlueprintSpec {
                input_schema: Schema::default(),
                output_schema: Schema::default(),
                dependencies: Vec::new(),
                sub_pipelines: Vec::new(),
                execution_flow: crate::types::pipeline::ExecutionFlow::Sequential(Vec::new()),
            },
            implementations: Vec::new(),
            content_hash: [0u8; 32],
            peers: Vec::new(),
            consensus_status: crate::types::pipeline::ConsensusStatus::Accepted,
            verified_by: 0,
        };

        // Direct insert during initialization (before wrapping in Arc<RwLock>)
        blueprints.insert(builtin.id(), blueprint);

        Ok(())
    }

    /// Execute a pipeline
    pub async fn execute(
        &self,
        pipeline_id: PipelineID,
        input: PipelineInput,
        task_id: Option<TaskID>, //
    ) -> OzoneResult<PipelineOutput> {
        let blueprints = self.blueprints.read().await;
        let blueprint = blueprints
            .get(&pipeline_id)
            .ok_or_else(|| OzoneError::NotFound(format!("Pipeline {} not found", pipeline_id)))?;

        self.executor.execute(blueprint, input, task_id).await
    }

    /// Registration table for self-connecting pipelines (delegate).
    pub fn remote_pipelines(&self) -> Arc<crate::pipeline::remote::RemotePipelines> {
        self.executor.remote_pipelines()
    }

    /// Monitor activity hub (delegate).
    pub fn activity_hub(&self) -> Arc<crate::monitor::ActivityHub> {
        self.executor.activity_hub()
    }

    /// Read-only access to the executor's progress map (gRPC/dashboard use).
    pub fn progress_map(&self) -> Arc<tokio::sync::RwLock<HashMap<String, PipelineProgress>>> {
        self.executor.progress_map()
    }

    /// Cancel a running execution by id (gRPC/dashboard use).
    pub async fn cancel_execution(&self, execution_id: &str) -> bool {
        self.executor.cancel(execution_id).await
    }

    /// Get pipeline blueprint
    pub async fn get_blueprint(&self, pipeline_id: PipelineID) -> Option<PipelineBlueprint> {
        self.blueprints.read().await.get(&pipeline_id).cloned()
    }

    /// List all registered pipelines
    pub async fn list_pipelines(&self) -> Vec<(PipelineID, String)> {
        self.blueprints
            .read()
            .await
            .iter()
            .map(|(id, bp)| (*id, bp.name.clone()))
            .collect()
    }

    /// Register a custom pipeline
    pub async fn register_custom(&self, blueprint: PipelineBlueprint) -> OzoneResult<PipelineID> {
        let id = blueprint.pipeline_id;
        self.blueprints.write().await.insert(id, blueprint);
        Ok(id)
    }

    /// Remove a custom pipeline from the execution gate — mirrors
    /// register_custom, called when a remote agent deregisters so its id
    /// doesn't linger as a phantom "known" pipeline that then fails inside
    /// RemotePipelines::execute instead of at this gate.
    pub async fn unregister_custom(&self, pipeline_id: PipelineID) -> bool {
        self.blueprints.write().await.remove(&pipeline_id).is_some()
    }
}
