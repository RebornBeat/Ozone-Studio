//! # OZONE STUDIO
//!
//! OZONE STUDIO (Omnidirectional Zero-shot Neural Engine) is a revolutionary AGI
//! infrastructure that utilizes Zero-Shot Bolted Embedding technology to create
//! a self-extending cognitive framework across computing environments.
//!
//! ## Core Capabilities
//!
//! * **Zero-Shot Bolted Embedding**: Dynamic relationship-aware embeddings
//! * **Multi-modal Analysis**: Unified understanding across code, images, audio, and video
//! * **Self-extending Knowledge**: Autonomous knowledge exploration and extension
//! * **System Integration**: Seamless integration with operating systems
//!
//! ## Getting Started
//!
//! ```rust
//! use ozone_studio::{OzoneCore, OzoneConfig};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a default configuration
//!     let config = OzoneConfig::default();
//!
//!     // Initialize OZONE core
//!     let mut ozone = OzoneCore::new(config);
//!     ozone.init()?;
//!
//!     // Process some text
//!     let input = ozone_studio::Input {
//!         content: "What is Zero-Shot Bolted Embedding?".as_bytes().to_vec(),
//!         modality: ozone_studio::Modality::Text,
//!         metadata: None,
//!         context: None,
//!     };
//!
//!     // Get response
//!     let output = ozone.process_input(input).await?;
//!     println!("Response: {}", String::from_utf8_lossy(&output.content));
//!
//!     Ok(())
//! }
//! ```

// Standard library imports
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

// External crate imports
use tokio::sync::{RwLock, mpsc};

// Public modules
pub mod analyzers;
pub mod api;
pub mod cli;
pub mod config;
pub mod embedding;
pub mod errors;
pub mod expansion;
pub mod integration;
pub mod llm;
pub mod marketplace;
pub mod overlay;
pub mod reasoning;
pub mod security;
pub mod storage;
pub mod utils;

// Re-exports for convenience
pub use crate::analyzers::common::AnalysisResult;
pub use crate::embedding::zsbe::ZeroBoltedEmbedding;
pub use crate::errors::OzoneError;
pub use crate::reasoning::common::ReasoningResult;

/// The central OZONE system coordinator
pub struct OzoneCore {
    /// Configuration settings
    config: OzoneConfig,

    /// Zero-Shot Bolted Embedding manager
    zsbe_manager: embedding::ZSBEManager,

    /// Omnidirectional knowledge expander
    omni_expander: expansion::OmniExpander,

    /// Storage system
    storage: Arc<RwLock<storage::OmniStore>>,

    /// Reasoning engine
    reasoning: reasoning::Engine,

    /// System integration layer
    system_integration: integration::SystemIntegration,

    /// UI overlay system
    overlay: Option<overlay::OzoneOverlay>,

    /// Event handling system
    event_handler: utils::EventHandler,

    /// Input channel
    input_tx: mpsc::Sender<Input>,

    /// Input receiver
    input_rx: mpsc::Receiver<Input>,

    /// Output channel
    output_tx: mpsc::Sender<Output>,

    /// Output receiver
    output_rx: mpsc::Receiver<Output>,
}

impl OzoneCore {
    /// Create a new OZONE core system
    pub fn new(config: OzoneConfig) -> Self {
        let (input_tx, input_rx) = mpsc::channel(100);
        let (output_tx, output_rx) = mpsc::channel(100);

        let storage = Arc::new(RwLock::new(storage::OmniStore::new(
            config.storage_config.clone(),
        )));
        let zsbe_manager =
            embedding::ZSBEManager::new(config.embedding_config.clone(), Arc::clone(&storage));
        let omni_expander = expansion::OmniExpander::new().with_storage(Arc::clone(&storage));

        Self {
            config: config.clone(),
            zsbe_manager,
            omni_expander,
            storage,
            reasoning: reasoning::Engine::new(config.reasoning_config.clone()),
            system_integration: integration::SystemIntegration::new(),
            overlay: if config.enable_overlay {
                Some(overlay::OzoneOverlay::new())
            } else {
                None
            },
            event_handler: utils::EventHandler::new(),
            input_tx,
            input_rx,
            output_tx,
            output_rx,
        }
    }

    /// Initialize the system with all components
    pub fn init(&mut self) -> Result<(), OzoneError> {
        // Initialize storage system
        self.storage.blocking_write().init()?;

        // Initialize ZSBE manager
        self.zsbe_manager.init()?;

        // Initialize reasoning engine
        self.reasoning.init()?;

        // Initialize system integration
        self.system_integration.init()?;

        // Initialize overlay if enabled
        if let Some(overlay) = &mut self.overlay {
            overlay.init()?;
        }

        // Register event handlers
        self.register_event_handlers()?;

        // Start the main processing loop
        self.start_processing_loop();

        Ok(())
    }

    /// Process input from any source
    pub async fn process_input(&self, input: Input) -> Result<Output, OzoneError> {
        // Send input to the processing loop
        self.input_tx
            .send(input)
            .await
            .map_err(|e| OzoneError::InputError(format!("Failed to send input: {}", e)))?;

        // Wait for output
        self.output_rx
            .clone()
            .recv()
            .await
            .ok_or_else(|| OzoneError::OutputError("No output received".to_string()))
    }

    /// Start the main processing loop
    fn start_processing_loop(&self) {
        let input_rx = self.input_rx.clone();
        let output_tx = self.output_tx.clone();
        let zsbe_manager = self.zsbe_manager.clone();
        let reasoning = self.reasoning.clone();
        let omni_expander = self.omni_expander.clone();

        tokio::spawn(async move {
            let mut rx = input_rx;

            while let Some(input) = rx.recv().await {
                // Process the input
                match Self::process_input_internal(&zsbe_manager, &reasoning, &omni_expander, input)
                    .await
                {
                    Ok(output) => {
                        // Send output
                        if let Err(e) = output_tx.send(output).await {
                            eprintln!("Failed to send output: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error processing input: {}", e);

                        // Send error output
                        let error_output = Output {
                            content: format!("Error: {}", e).into_bytes(),
                            modality: Modality::Text,
                            knowledge_paths: Vec::new(),
                            reasoning_trace: ReasoningTrace {
                                steps: Vec::new(),
                                confidence: 0.0,
                            },
                        };

                        if let Err(e) = output_tx.send(error_output).await {
                            eprintln!("Failed to send error output: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// Internal input processing implementation
    async fn process_input_internal(
        zsbe_manager: &embedding::ZSBEManager,
        reasoning: &reasoning::Engine,
        omni_expander: &expansion::OmniExpander,
        input: Input,
    ) -> Result<Output, OzoneError> {
        // 1. Analyze the input
        let analysis_result = match input.modality {
            Modality::Text => analyzers::text::analyze(&input).await?,
            Modality::Code => analyzers::code::analyze(&input).await?,
            Modality::Image => analyzers::image::analyze(&input).await?,
            Modality::Audio => analyzers::audio::analyze(&input).await?,
            Modality::Video => analyzers::video::analyze(&input).await?,
            Modality::System => analyzers::system::analyze(&input).await?,
        };

        // 2. Generate Zero-Shot Bolted Embeddings
        let embedding = zsbe_manager.generate_embedding(&analysis_result).await?;

        // 3. Map relationships
        let relationships = zsbe_manager.map_relationships(&embedding).await?;

        // 4. Integrate into knowledge structure
        // (This happens inside the storage system)

        // 5. Navigate knowledge paths
        let context = input.context.unwrap_or_default();
        let paths = reasoning.navigate_paths(&context, &embedding).await?;

        // 6. Perform reasoning
        let reasoning_result = reasoning.process(&analysis_result, &paths).await?;

        // 7. Generate response
        let output = reasoning.generate_response(&reasoning_result).await?;

        // 8. Trigger knowledge expansion in the background
        tokio::spawn(async move {
            if let Err(e) = omni_expander
                .expand_from_interaction(&embedding, &reasoning_result, &output)
                .await
            {
                eprintln!("Error in knowledge expansion: {}", e);
            }
        });

        Ok(output)
    }

    /// Register all event handlers
    fn register_event_handlers(&mut self) -> Result<(), OzoneError> {
        // Register system event handlers
        self.event_handler.register(
            utils::EventType::SystemEvent,
            Box::new(|event| {
                // Handle system events
                Ok(())
            }),
        )?;

        // Register user interaction event handlers
        self.event_handler.register(
            utils::EventType::UserInteraction,
            Box::new(|event| {
                // Handle user interactions
                Ok(())
            }),
        )?;

        // Register knowledge update event handlers
        self.event_handler.register(
            utils::EventType::KnowledgeUpdate,
            Box::new(|event| {
                // Handle knowledge updates
                Ok(())
            }),
        )?;

        Ok(())
    }

    /// Get system information
    pub fn get_system_info(&self) -> Result<SystemInfo, OzoneError> {
        self.system_integration.get_system_info()
    }

    /// Execute a system command
    pub fn execute_command(
        &self,
        command: integration::SystemCommand,
    ) -> Result<integration::SystemCommandResult, OzoneError> {
        self.system_integration.execute_command(command)
    }

    /// Enable or disable overlay
    pub fn set_overlay_enabled(&mut self, enabled: bool) -> Result<(), OzoneError> {
        if enabled && self.overlay.is_none() {
            self.overlay = Some(overlay::OzoneOverlay::new());
            self.overlay.as_mut().unwrap().init()?;
        } else if !enabled {
            self.overlay = None;
        }

        self.config.enable_overlay = enabled;
        Ok(())
    }
}

/// Configuration for the OZONE system
#[derive(Clone)]
pub struct OzoneConfig {
    /// Storage configuration
    pub storage_config: storage::StorageConfig,

    /// Embedding configuration
    pub embedding_config: embedding::EmbeddingConfig,

    /// Reasoning configuration
    pub reasoning_config: reasoning::ReasoningConfig,

    /// System integration configuration
    pub system_config: integration::SystemConfig,

    /// Whether to enable the UI overlay
    pub enable_overlay: bool,
}

impl Default for OzoneConfig {
    fn default() -> Self {
        Self {
            storage_config: storage::StorageConfig::default(),
            embedding_config: embedding::EmbeddingConfig::default(),
            reasoning_config: reasoning::ReasoningConfig::default(),
            system_config: integration::SystemConfig::default(),
            enable_overlay: false,
        }
    }
}

/// Input to the OZONE system
pub struct Input {
    /// The content to process
    pub content: Vec<u8>,

    /// The modality of the content
    pub modality: Modality,

    /// Optional metadata
    pub metadata: Option<serde_json::Value>,

    /// Optional context
    pub context: Option<Context>,
}

/// Output from the OZONE system
pub struct Output {
    /// The response content
    pub content: Vec<u8>,

    /// The modality of the response
    pub modality: Modality,

    /// Knowledge paths used
    pub knowledge_paths: Vec<KnowledgePath>,

    /// Reasoning trace
    pub reasoning_trace: ReasoningTrace,
}

/// Modality types
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Modality {
    /// Text modality
    Text,

    /// Code modality
    Code,

    /// Image modality
    Image,

    /// Audio modality
    Audio,

    /// Video modality
    Video,

    /// System modality
    System,
}

/// Knowledge path
pub struct KnowledgePath {
    /// Path nodes
    pub nodes: Vec<String>,

    /// Path relevance
    pub relevance: f32,

    /// Path confidence
    pub confidence: f32,
}

/// Reasoning trace
pub struct ReasoningTrace {
    /// Reasoning steps
    pub steps: Vec<ReasoningStep>,

    /// Overall confidence
    pub confidence: f32,
}

/// Reasoning step
pub struct ReasoningStep {
    /// Operation description
    pub operation: String,

    /// Input to the step
    pub input: String,

    /// Output from the step
    pub output: String,

    /// Step confidence
    pub confidence: f32,
}

/// Context
#[derive(Default)]
pub struct Context {
    /// Recent interactions
    pub recent_interactions: Vec<Interaction>,

    /// Active applications
    pub active_applications: Vec<ApplicationInfo>,

    /// User preferences
    pub user_preferences: UserPreferences,

    /// System state
    pub system_state: SystemState,
}

/// Interaction
pub struct Interaction {
    /// Timestamp
    pub timestamp: SystemTime,

    /// Content
    pub content: String,

    /// Modality
    pub modality: Modality,
}

/// Application info
pub struct ApplicationInfo {
    /// Application name
    pub name: String,

    /// Window ID
    pub window_id: u64,

    /// Is foreground
    pub foreground: bool,
}

/// User preferences
#[derive(Default)]
pub struct UserPreferences {
    /// Response style
    pub response_style: String,

    /// Verbosity level
    pub verbosity_level: u8,

    /// Favorite domains
    pub favorite_domains: Vec<String>,
}

/// System state
#[derive(Default)]
pub struct SystemState {
    /// CPU usage
    pub cpu_usage: f32,

    /// Memory usage
    pub memory_usage: f32,

    /// Battery level
    pub battery_level: Option<f32>,
}

/// System information
pub struct SystemInfo {
    /// Operating system
    pub os: String,

    /// Kernel version
    pub kernel: String,

    /// Hostname
    pub hostname: String,

    /// Username
    pub username: String,

    /// CPU information
    pub cpu_info: String,

    /// Total memory in bytes
    pub memory_total: u64,

    /// Free memory in bytes
    pub memory_free: u64,

    /// Total disk space in bytes
    pub disk_total: u64,

    /// Free disk space in bytes
    pub disk_free: u64,
}
