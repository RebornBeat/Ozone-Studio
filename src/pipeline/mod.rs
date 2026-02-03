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

mod registry;
mod executor;

pub use registry::*;
pub use executor::*;

use crate::config::PipelineConfig;
use crate::types::{PipelineID, TaskID, OzoneError, OzoneResult};
use crate::types::pipeline::{
    PipelineInput, PipelineOutput, PipelineBlueprint, BuiltinPipeline,
    Schema, ExecutionContext,
};
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
        std::fs::create_dir_all(&builtin_path)
            .map_err(|e| OzoneError::PipelineError(format!("Failed to create builtin dir: {}", e)))?;
        std::fs::create_dir_all(&custom_path)
            .map_err(|e| OzoneError::PipelineError(format!("Failed to create custom dir: {}", e)))?;
        
        let executor = PipelineExecutor::new(config)?;
        
        // Build blueprints HashMap during initialization (before wrapping in Arc)
        let mut blueprints_map = HashMap::new();
        
        // Load builtin pipelines into the map
        Self::load_builtin_pipelines_into(&mut blueprints_map)?;
        
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
    fn load_builtin_pipelines_into(blueprints: &mut HashMap<PipelineID, PipelineBlueprint>) -> OzoneResult<()> {
        tracing::info!("Loading builtin pipelines");
        
        // Register all builtin pipelines from the enum
        // The actual implementation lives in pipelines/ directory
        let builtins = [
            BuiltinPipeline::Auth,
            BuiltinPipeline::ThemeLoader,
            BuiltinPipeline::ZSEIQuery,
            BuiltinPipeline::ZSEIWrite,
            BuiltinPipeline::TaskManager,
            BuiltinPipeline::WorkspaceTab,
            BuiltinPipeline::LibraryTab,
            BuiltinPipeline::SettingsTab,
            BuiltinPipeline::Prompt,
            BuiltinPipeline::Voice,
            BuiltinPipeline::MethodologyFetch,
            BuiltinPipeline::MethodologyCreate,
            BuiltinPipeline::BlueprintSearch,
            BuiltinPipeline::BlueprintCreate,
            BuiltinPipeline::PipelineCreation,
            BuiltinPipeline::ZeroShotSimulation,
            BuiltinPipeline::TraversalML,
            BuiltinPipeline::CodeAnalysis,
            BuiltinPipeline::PackageContext,
            BuiltinPipeline::TextAnalysis,
            BuiltinPipeline::ContextAggregation,
            BuiltinPipeline::GraphVisualization,
            BuiltinPipeline::TaskRecommendation,
            BuiltinPipeline::Reordering,
            BuiltinPipeline::BrowserNavigation,
            BuiltinPipeline::IntegrityCheck,
            BuiltinPipeline::Consensus,
            BuiltinPipeline::ExternalReference,
            BuiltinPipeline::PackageRelationship,
            BuiltinPipeline::FileLink,
            BuiltinPipeline::URLLink,
            BuiltinPipeline::PackageLink,
            BuiltinPipeline::Sync,
            BuiltinPipeline::DeviceRegister,
            BuiltinPipeline::HomeReturn,
            BuiltinPipeline::TaskViewer,
            BuiltinPipeline::LogViewer,
            BuiltinPipeline::DeviceStatus,
        ];
        
        for builtin in builtins {
            Self::register_builtin_into(builtin, blueprints)?;
        }
        
        // Register consciousness pipelines (always compiled, enabled/disabled at runtime)
        let consciousness_builtins = [
            BuiltinPipeline::ConsciousnessDecisionGate,
            BuiltinPipeline::ExperienceCategorization,
            BuiltinPipeline::CoreMemoryFormation,
            BuiltinPipeline::ExperienceRetrieval,
            BuiltinPipeline::EmotionalBaselineUpdate,
            BuiltinPipeline::ILoop,
            BuiltinPipeline::InternalLanguage,
            BuiltinPipeline::NarrativeConstruction,
            BuiltinPipeline::RelationshipDevelopment,
            BuiltinPipeline::EthicalAssessment,
            BuiltinPipeline::EthicalSimulation,
            BuiltinPipeline::PlaybackReview,
            BuiltinPipeline::UserFeedback,
            BuiltinPipeline::CollectiveConsciousness,
            BuiltinPipeline::VoiceIdentity,
            BuiltinPipeline::MetaPortionConsciousness,
        ];
        
        for builtin in consciousness_builtins {
            Self::register_builtin_into(builtin, blueprints)?;
        }
        
        Ok(())
    }
    
    /// Register a builtin pipeline into a HashMap (static helper for initialization)
    fn register_builtin_into(builtin: BuiltinPipeline, blueprints: &mut HashMap<PipelineID, PipelineBlueprint>) -> OzoneResult<()> {
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
        task_id: TaskID,
    ) -> OzoneResult<PipelineOutput> {
        // Get blueprint
        let blueprints = self.blueprints.read().await;
        let blueprint = blueprints.get(&pipeline_id)
            .ok_or_else(|| OzoneError::NotFound(format!("Pipeline {} not found", pipeline_id)))?;
        
        // Execute via executor
        self.executor.execute(blueprint, input, task_id).await
    }
    
    /// Get pipeline blueprint
    pub async fn get_blueprint(&self, pipeline_id: PipelineID) -> Option<PipelineBlueprint> {
        self.blueprints.read().await.get(&pipeline_id).cloned()
    }
    
    /// List all registered pipelines
    pub async fn list_pipelines(&self) -> Vec<(PipelineID, String)> {
        self.blueprints.read().await
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
}
