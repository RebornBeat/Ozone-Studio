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
            zsei: Arc::new(RwLock::new(zsei)),
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
