//! Ozone Studio - Omnidirectional Zero-Shot Neural Engine
//!
//! A systems-first platform for omnidirectional, zero-shot data traversal,
//! abstraction, and context compilation.
//!
//! # Architecture
//!
//! Ozone Studio operates as a pipeline execution engine with:
//! - ZSEI (Zero-Shot Embedded Indexer) for knowledge fabric
//! - Pipeline system for composable, executable units
//! - Task management for tracking all computation
//! - Network layer for P2P sync of methodologies, blueprints, findings
//! - Consciousness system for AGI capabilities (enabled via config)
//! - UI layer (Electron) for user interaction
//!
//! # Core Principles
//!
//! - Structure before intelligence
//! - Compression before learning
//! - Traversal before generation
//! - Pipelines over monoliths
//! - Zero-shot discovery without task-specific training
//! - LLMs are clients, not the system core
//! - Context not copies
//! - Link not duplicate
//! - Integrity always

pub mod auth;
pub mod blueprints;
pub mod bootstrap;
pub mod config;
pub mod consciousness;
pub mod grpc;
pub mod integrity;
pub mod methodologies;
pub mod network;
pub mod orchestrator;
pub mod pipeline;
pub mod task;
pub mod types;
pub mod zsei;

// Re-exports
pub use config::OzoneConfig;
pub use types::*;

use bootstrap::BootstrapManager;
use std::sync::Arc;
use tokio::sync::RwLock;

use task::{RefinementConfig, TaskQueueConfig};

/// Result of the full AMT orchestration flow
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrchestrationOutput {
    pub success: bool,
    pub response_text: Option<String>,
    pub task_id: Option<u64>,
    pub blueprint_id: Option<u64>,
    pub stages_completed: Vec<serde_json::Value>,
    pub needs_clarification: bool,
    pub clarification_points: Vec<String>,
}

/// Main Ozone Studio runtime
pub struct OzoneRuntime {
    /// Configuration
    pub config: OzoneConfig,

    /// ZSEI instance
    pub zsei: Arc<RwLock<zsei::ZSEI>>,

    /// Pipeline registry
    pub pipeline_registry: Arc<RwLock<pipeline::PipelineRegistry>>,

    pub task_manager: Arc<RwLock<task::TaskManager>>,

    /// Authentication system
    pub auth: Arc<RwLock<auth::AuthSystem>>,

    /// Integrity monitor
    pub integrity: Arc<RwLock<integrity::IntegrityMonitor>>,

    /// Network manager for P2P sync
    pub network: Arc<RwLock<network::NetworkManager>>,

    /// Current session
    pub session: Arc<RwLock<Option<types::auth::Session>>>,

    /// Consciousness system (enabled/disabled via config.toml)
    pub consciousness: Option<Arc<RwLock<consciousness::ConsciousnessSystem>>>,
}

impl OzoneRuntime {
    /// Create a new Ozone runtime with the given configuration
    pub async fn new(config: OzoneConfig) -> Result<Self, OzoneError> {
        tracing::info!("Initializing Ozone Studio v{}", env!("CARGO_PKG_VERSION"));

        // Run bootstrap if first time setup
        if !config.general.setup_complete {
            tracing::info!("First-time setup detected, running bootstrap...");
            let bootstrap = BootstrapManager::new(&config);
            bootstrap.run()?;

            // Mark setup complete and save config
            // Note: We clone and modify since config is borrowed
            let mut updated_config = config.clone();
            updated_config.general.setup_complete = true;
            let config_path = std::path::Path::new("config.toml");
            if let Err(e) = updated_config.save(config_path) {
                tracing::warn!("Failed to save config after bootstrap: {}", e);
            }
        }

        // Initialize ZSEI
        let zsei = zsei::ZSEI::new(&config.zsei)?;

        let zsei_arc = Arc::new(RwLock::new(zsei));

        // --- Register artifacts as ZSEI containers (idempotent, runs every startup) ---
        {
            let methodology_store = crate::methodologies::store::MethodologyStore::new(
                zsei_arc.clone(),
                std::path::PathBuf::from(&config.zsei.methodology_index_path),
            );
            if let Err(e) = methodology_store.register_all().await {
                tracing::warn!("Methodology ZSEI registration: {}", e);
            }

            let blueprint_store = crate::blueprints::store::BlueprintStore::new(
                zsei_arc.clone(),
                std::path::PathBuf::from(&config.zsei.blueprint_index_path),
            );
            if let Err(e) = blueprint_store.register_all().await {
                tracing::warn!("Blueprint ZSEI registration: {}", e);
            }

            let pipeline_store = crate::pipeline::PipelineStore::new(zsei_arc.clone());
            if let Err(e) = pipeline_store.register_all().await {
                tracing::warn!("Pipeline ZSEI registration: {}", e);
            }
        }

        // Wire ZSEI into ConsciousnessStore (so experiences persist to ZSEI)
        {
            if let Ok(mut store) = crate::consciousness::CONSCIOUSNESS_STORE.lock() {
                store.set_zsei(zsei_arc.clone());
            }
        }

        // Initialize pipeline registry
        let pipeline_registry = pipeline::PipelineRegistry::new(&config.pipelines)?;

        // Initialize task manager
        let task_manager =
            task::TaskManager::new(TaskQueueConfig::default(), RefinementConfig::default())?;

        // Initialize auth system
        let auth = auth::AuthSystem::new(&config.auth)?;

        // Initialize integrity monitor
        let integrity = integrity::IntegrityMonitor::new(&config.integrity)?;

        // Initialize network manager
        let mut network = network::NetworkManager::new(config.network.clone()).await?;
        network.initialize().await?;

        // Initialize consciousness if enabled in config
        let consciousness = if config.consciousness.enabled {
            tracing::info!("Consciousness system: ENABLED");
            Some(Arc::new(RwLock::new(
                consciousness::ConsciousnessSystem::new(),
            )))
        } else {
            tracing::info!("Consciousness system: DISABLED (enable in config.toml)");
            None
        };

        Ok(Self {
            config,
            zsei: zsei_arc.clone(),
            pipeline_registry: Arc::new(RwLock::new(pipeline_registry)),
            task_manager: Arc::new(RwLock::new(task_manager)),
            auth: Arc::new(RwLock::new(auth)),
            integrity: Arc::new(RwLock::new(integrity)),
            network: Arc::new(RwLock::new(network)),
            session: Arc::new(RwLock::new(None)),
            consciousness,
        })
    }

    /// Start the runtime (gRPC server for UI communication)
    pub async fn start(self) -> Result<(), OzoneError> {
        tracing::info!("Starting Ozone Studio runtime");

        // Wrap self in Arc<RwLock<...>> for sharing with server handlers
        let runtime = Arc::new(RwLock::new(self));

        // Start integrity monitoring
        let integrity = runtime.read().await.integrity.clone();
        tokio::spawn(async move {
            if let Err(e) = integrity.write().await.start_monitoring().await {
                tracing::error!("Integrity monitoring failed: {}", e);
            }
        });

        // Start gRPC server
        grpc::start_server(runtime).await?;

        Ok(())
    }

    /// Authenticate a user
    pub async fn authenticate(
        &self,
        public_key: &[u8],
        signature: &[u8],
    ) -> Result<types::auth::Session, OzoneError> {
        let session = self
            .auth
            .write()
            .await
            .authenticate(public_key, signature)
            .await?;
        *self.session.write().await = Some(session.clone());
        Ok(session)
    }

    /// Execute a pipeline
    pub async fn execute_pipeline(
        &self,
        pipeline_id: PipelineID,
        input: types::pipeline::PipelineInput,
    ) -> Result<types::pipeline::PipelineOutput, OzoneError> {
        // Ensure user is authenticated
        let session = self.session.read().await;
        let session = session
            .as_ref()
            .ok_or_else(|| OzoneError::AuthError("Not authenticated".into()))?;

        // Execute pipeline
        let registry = self.pipeline_registry.read().await;
        registry.execute(pipeline_id, input, None).await
    }

    /// Query ZSEI
    pub async fn query_zsei(
        &self,
        query: types::zsei::ZSEIQuery,
    ) -> Result<types::zsei::ZSEIQueryResult, OzoneError> {
        self.zsei.read().await.query(query).await
    }

    /// Check if consciousness is enabled
    pub fn is_consciousness_enabled(&self) -> bool {
        self.consciousness.is_some()
    }

    /// Run the full 14-stage AMT orchestration flow.
    /// This is the ONLY entry point for prompt-driven task execution.
    pub async fn orchestrate(
        &self,
        input: crate::types::pipeline::PipelineInput,
        user_id: u64,
        device_id: u64,
    ) -> OzoneResult<OrchestrationOutput> {
        let prompt = input
            .data
            .get("prompt")
            .and_then(|v| {
                if let crate::types::Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        if prompt.trim().is_empty() {
            return Err(OzoneError::ValidationError("Prompt cannot be empty".into()));
        }

        let project_id = input.data.get("project_id").and_then(|v| {
            if let crate::types::Value::Int(i) = v {
                Some(*i as u64)
            } else {
                None
            }
        });

        let workspace_id = input.data.get("workspace_id").and_then(|v| {
            if let crate::types::Value::Int(i) = v {
                Some(*i as u64)
            } else {
                None
            }
        });

        let token_budget = input
            .data
            .get("token_budget")
            .and_then(|v| {
                if let crate::types::Value::Int(i) = v {
                    Some(*i as u32)
                } else {
                    None
                }
            })
            .unwrap_or(100_000);

        let consciousness_enabled = input
            .data
            .get("consciousness_enabled")
            .and_then(|v| {
                if let crate::types::Value::Bool(b) = v {
                    Some(*b)
                } else {
                    None
                }
            })
            .unwrap_or(false);

        // Enqueue via task manager — task manager owns queue ordering + consciousness gate
        let task_manager = self.task_manager.read().await;
        let mut inputs = std::collections::HashMap::new();
        inputs.insert(
            "prompt".to_string(),
            serde_json::Value::String(prompt.clone()),
        );
        if let Some(files_val) = input.data.get("attached_files") {
            inputs.insert(
                "attached_files".to_string(),
                serde_json::to_value(files_val).unwrap_or_default(),
            );
        }
        if let Some(pid) = project_id {
            inputs.insert("project_id".to_string(), serde_json::json!(pid));
        }
        if let Some(wid) = workspace_id {
            inputs.insert("workspace_id".to_string(), serde_json::json!(wid));
        }

        let task_id = task_manager
            .enqueue_task(
                None, // blueprint selected by orchestrator, not here
                inputs,
                user_id,
                device_id,
                workspace_id,
                project_id,
                crate::task::TaskPriority::Normal,
            )
            .await?;

        drop(task_manager);

        // Run the orchestrator — builds the AMT, selects a blueprint, executes steps
        let mut orchestrator = crate::orchestrator::PromptOrchestrator::new(
            self.pipeline_registry.clone(),
            self.zsei.clone(),
            self.task_manager.clone(),
        );

        let orch_result = orchestrator
            .process(
                task_id,
                &prompt,
                project_id,
                workspace_id,
                token_budget,
                consciousness_enabled,
            )
            .await;

        match orch_result {
            Ok(state) => Ok(OrchestrationOutput {
                success: true,
                response_text: state.final_response,
                task_id: Some(task_id),
                blueprint_id: state.selected_blueprint_id,
                stages_completed: state.completed_stages,
                needs_clarification: state.needs_clarification,
                clarification_points: state.clarification_points.unwrap_or_default(),
            }),
            Err(e) => {
                // Mark task as failed in task manager
                let task_manager = self.task_manager.read().await;
                let _ = task_manager.fail_task(task_id, e.to_string()).await;
                Err(e)
            }
        }
    }
}

/// Initialize logging with default settings
pub fn init_logging() {
    init_logging_with_level("info");
}

/// Initialize logging with specified level
pub fn init_logging_with_level(level: &str) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| format!("ozone_studio={},hyper=warn,tonic=warn", level));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| env_filter.into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false),
        )
        .init();
}
