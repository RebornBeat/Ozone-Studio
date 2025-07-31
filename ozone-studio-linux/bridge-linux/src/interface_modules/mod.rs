// =============================================================================
// bridge-linux/src/interface_modules/mod.rs
// Modular Interface System - The Plugin Architecture for Unlimited UI/UX Types
// =============================================================================

/*
ARCHITECTURAL PHILOSOPHY:

This module implements BRIDGE's revolutionary modular interface architecture. Think of it like
a sophisticated plugin system where each "interface module" is a complete, self-contained UI/UX
implementation for a specific way humans interact with the AGI.

The genius of this design is that each interface module can have completely different:
- User interface designs (chat UI vs voice UI vs spatial gesture UI vs neural interface)
- Interaction paradigms (text conversation vs speech vs hand gestures vs thought)
- Technical implementations (web-based vs native vs AR/VR vs brain-computer interface)
- User experience flows (conversational vs command-based vs spatial vs direct neural)

Yet they all plug into the same underlying BRIDGE coordination system and can seamlessly
work together through the modality coordination layer.

This is production-ready, extensible architecture that can accommodate interface types
we haven't even imagined yet, while maintaining consistency and reliability.
*/

use std::collections::HashMap;
use std::time::{Duration, SystemTime, Instant};
use std::sync::Arc;
use std::fmt;
use std::any::Any;

// Async runtime and concurrency
use tokio::sync::{RwLock, Mutex, mpsc, oneshot, broadcast};
use tokio::time::{sleep, timeout, interval};
use tokio::task::JoinHandle;

// Serialization and data handling
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Import shared protocol types for ecosystem coordination
use shared_protocols::{
    EcosystemIdentity,
    ComponentType,
    HumanGuidance,
    AuthorityLevel,
    TaskOrchestrationRequest,
    ProtocolError,
};

// Import security for user authentication and privacy
use shared_security::{
    SecurityError,
    AuthenticationCredentials,
    SecurityConfig,
    UserCertificate,
    SessionManager,
};

// Import modality coordination types from our sibling module
use crate::modality_coordination::{
    InputCoordinator,
    OutputCoordinator,
    ContextBridge,
    PreferenceManager,
    InputEvent,
    OutputEvent,
    ContextTransition,
};

// Import shared UI components
use crate::shared_ui_components::{
    NotificationSystem,
    StatusIndicators,
    AccessibilityFramework,
    ThemeManager,
    UIComponent,
};

// =============================================================================
// INTERFACE MODULE IMPLEMENTATIONS
// =============================================================================

// Each interface module is a complete, self-contained UI/UX implementation
pub mod text_scribe_module;     // Chat UI for BRIDGE-SCRIBE coordination
pub mod voice_module;           // Voice interface with audio controls
pub mod visual_gesture_module;  // Visual/gesture interface with spatial interaction
pub mod eeg_module;             // Brain-computer interface with neural processing
pub mod custom_module_framework; // Framework for creating new interface types

// =============================================================================
// CORE INTERFACE MODULE TRAIT
// =============================================================================

/*
Every interface module must implement this trait. This is the contract that ensures
all interface modules can work together while having completely different implementations.

Think of this like defining the "shape" that every plugin must have, while letting
each plugin implement that shape in completely different ways.
*/
#[async_trait::async_trait]
pub trait InterfaceModule: Send + Sync {
    // Module identification and metadata
    fn get_module_type(&self) -> InterfaceType;
    fn get_module_name(&self) -> String;
    fn get_supported_input_types(&self) -> Vec<InputType>;
    fn get_supported_output_types(&self) -> Vec<OutputType>;
    fn get_capabilities(&self) -> Vec<InterfaceCapability>;
    
    // Lifecycle management - every module needs to start up, run, and shut down cleanly
    async fn initialize(&mut self, config: ModuleConfiguration) -> Result<(), InterfaceModuleError>;
    async fn start(&mut self) -> Result<(), InterfaceModuleError>;
    async fn shutdown(&mut self) -> Result<(), InterfaceModuleError>;
    
    // Core interface operations - how the module processes human input and generates output
    async fn handle_input(&mut self, input: InputEvent) -> Result<ProcessedInput, InterfaceModuleError>;
    async fn generate_output(&mut self, response: EcosystemResponse) -> Result<OutputEvent, InterfaceModuleError>;
    
    // State management - modules need to preserve context across interactions
    async fn save_state(&self) -> Result<ModuleState, InterfaceModuleError>;
    async fn restore_state(&mut self, state: ModuleState) -> Result<(), InterfaceModuleError>;
    
    // Health and monitoring - production systems need observability
    async fn health_check(&self) -> Result<ModuleHealth, InterfaceModuleError>;
    async fn get_metrics(&self) -> Result<ModuleMetrics, InterfaceModuleError>;
    
    // Configuration and preferences - modules should be customizable
    async fn update_configuration(&mut self, config: ModuleConfiguration) -> Result<(), InterfaceModuleError>;
    async fn get_user_preferences(&self) -> Result<UserPreferences, InterfaceModuleError>;
    async fn set_user_preferences(&mut self, preferences: UserPreferences) -> Result<(), InterfaceModuleError>;
    
    // Security and authentication - every module needs to handle security properly
    async fn authenticate_user(&mut self, credentials: AuthenticationCredentials) -> Result<bool, InterfaceModuleError>;
    async fn validate_session(&self, session_token: &str) -> Result<bool, InterfaceModuleError>;
    
    // Advanced capabilities that some modules might implement
    async fn supports_real_time(&self) -> bool { false }
    async fn supports_offline_mode(&self) -> bool { false }
    async fn supports_multi_user(&self) -> bool { false }
    async fn supports_accessibility_features(&self) -> Vec<AccessibilityFeature> { Vec::new() }
}

// =============================================================================
// INTERFACE MANAGER - THE ORCHESTRATOR OF ALL INTERFACE MODULES
// =============================================================================

/*
The InterfaceManager is like a conductor orchestrating different musicians (interface modules).
Each musician plays a different instrument (UI/UX type), but the conductor ensures they all
work together harmoniously to create a beautiful symphony (seamless user experience).

This is the core system that:
1. Manages the lifecycle of all interface modules
2. Routes input/output between modules and the ecosystem
3. Coordinates between different modalities
4. Handles security and authentication across all interfaces
5. Provides monitoring and metrics for all interfaces
*/
pub struct InterfaceManager {
    // Active interface modules - this is our plugin registry
    active_modules: Arc<RwLock<HashMap<InterfaceType, Box<dyn InterfaceModule>>>>,
    
    // Configuration for each module type
    module_configurations: Arc<RwLock<HashMap<InterfaceType, ModuleConfiguration>>>,
    
    // Coordination systems that work across all modules
    modality_coordinator: Arc<Mutex<ModalityCoordinator>>,
    context_bridge: Arc<Mutex<ContextBridge>>,
    preference_manager: Arc<Mutex<PreferenceManager>>,
    
    // Security and session management across all interfaces
    session_manager: Arc<Mutex<SessionManager>>,
    authentication_manager: Arc<Mutex<AuthenticationManager>>,
    
    // Shared UI components available to all modules
    notification_system: Arc<Mutex<NotificationSystem>>,
    status_indicators: Arc<Mutex<StatusIndicators>>,
    accessibility_framework: Arc<Mutex<AccessibilityFramework>>,
    theme_manager: Arc<Mutex<ThemeManager>>,
    
    // Communication channels for coordinating with OZONE STUDIO
    ecosystem_coordinator: Arc<Mutex<EcosystemCoordinator>>,
    
    // Monitoring and metrics collection
    metrics_collector: Arc<Mutex<InterfaceMetricsCollector>>,
    health_monitor: Arc<Mutex<InterfaceHealthMonitor>>,
    
    // Module discovery and loading system for dynamic plugins
    module_loader: Arc<Mutex<ModuleLoader>>,
    plugin_registry: Arc<RwLock<PluginRegistry>>,
    
    // Event distribution system
    event_bus: Arc<Mutex<InterfaceEventBus>>,
    
    // Configuration
    config: InterfaceManagerConfig,
}

impl InterfaceManager {
    /// Create a new InterfaceManager with all supporting systems
    pub async fn new(config: InterfaceManagerConfig) -> Result<Self, InterfaceModuleError> {
        // Initialize all the supporting systems that interface modules will use
        let modality_coordinator = Arc::new(Mutex::new(
            ModalityCoordinator::new(config.modality_coordination.clone())
        ));
        
        let context_bridge = Arc::new(Mutex::new(
            ContextBridge::new(config.context_management.clone())
        ));
        
        let preference_manager = Arc::new(Mutex::new(
            PreferenceManager::new(config.preference_management.clone())
        ));
        
        let session_manager = Arc::new(Mutex::new(
            SessionManager::new(config.security.clone())?
        ));
        
        let authentication_manager = Arc::new(Mutex::new(
            AuthenticationManager::new(config.security.clone())?
        ));
        
        // Initialize shared UI components that all modules can use
        let notification_system = Arc::new(Mutex::new(
            NotificationSystem::new(config.ui_components.notification_config.clone())
        ));
        
        let status_indicators = Arc::new(Mutex::new(
            StatusIndicators::new(config.ui_components.status_config.clone())
        ));
        
        let accessibility_framework = Arc::new(Mutex::new(
            AccessibilityFramework::new(config.ui_components.accessibility_config.clone())
        ));
        
        let theme_manager = Arc::new(Mutex::new(
            ThemeManager::new(config.ui_components.theme_config.clone())
        ));
        
        // Initialize ecosystem coordination
        let ecosystem_coordinator = Arc::new(Mutex::new(
            EcosystemCoordinator::new(config.ecosystem_coordination.clone())
        ));
        
        // Initialize monitoring and metrics
        let metrics_collector = Arc::new(Mutex::new(
            InterfaceMetricsCollector::new(config.metrics_collection.clone())
        ));
        
        let health_monitor = Arc::new(Mutex::new(
            InterfaceHealthMonitor::new(config.health_monitoring.clone())
        ));
        
        // Initialize module loading and discovery
        let module_loader = Arc::new(Mutex::new(
            ModuleLoader::new(config.module_loading.clone())
        ));
        
        let plugin_registry = Arc::new(RwLock::new(
            PluginRegistry::new(config.plugin_discovery.clone())
        ));
        
        // Initialize event distribution
        let event_bus = Arc::new(Mutex::new(
            InterfaceEventBus::new(config.event_distribution.clone())
        ));
        
        Ok(Self {
            active_modules: Arc::new(RwLock::new(HashMap::new())),
            module_configurations: Arc::new(RwLock::new(HashMap::new())),
            modality_coordinator,
            context_bridge,
            preference_manager,
            session_manager,
            authentication_manager,
            notification_system,
            status_indicators,
            accessibility_framework,
            theme_manager,
            ecosystem_coordinator,
            metrics_collector,
            health_monitor,
            module_loader,
            plugin_registry,
            event_bus,
            config,
        })
    }
    
    /// Register and initialize a new interface module
    /// This is how we "plug in" new interface types
    pub async fn register_module(
        &mut self, 
        module_type: InterfaceType,
        mut module: Box<dyn InterfaceModule>,
        config: ModuleConfiguration
    ) -> Result<(), InterfaceModuleError> {
        // Initialize the module with its configuration
        module.initialize(config.clone()).await?;
        
        // Start the module
        module.start().await?;
        
        // Register the module in our active modules registry
        {
            let mut active_modules = self.active_modules.write().await;
            active_modules.insert(module_type.clone(), module);
        }
        
        // Store the configuration
        {
            let mut configurations = self.module_configurations.write().await;
            configurations.insert(module_type.clone(), config);
        }
        
        // Register the module with our coordination systems
        {
            let mut modality_coordinator = self.modality_coordinator.lock().await;
            modality_coordinator.register_module(module_type.clone()).await?;
        }
        
        // Set up monitoring for this module
        {
            let mut health_monitor = self.health_monitor.lock().await;
            health_monitor.start_monitoring_module(module_type.clone()).await?;
        }
        
        // Notify the ecosystem about the new interface capability
        {
            let mut ecosystem_coordinator = self.ecosystem_coordinator.lock().await;
            ecosystem_coordinator.notify_module_registration(module_type.clone()).await?;
        }
        
        Ok(())
    }
    
    /// Process input from any interface module and coordinate with the ecosystem
    pub async fn process_input(
        &mut self, 
        module_type: InterfaceType,
        input: InputEvent
    ) -> Result<OutputEvent, InterfaceModuleError> {
        // Get the appropriate interface module
        let processed_input = {
            let mut active_modules = self.active_modules.write().await;
            let module = active_modules.get_mut(&module_type)
                .ok_or(InterfaceModuleError::ModuleNotFound { module_type: module_type.clone() })?;
            
            // Let the module process the raw input into structured data
            module.handle_input(input).await?
        };
        
        // Coordinate with other modalities if needed
        let coordinated_input = {
            let mut modality_coordinator = self.modality_coordinator.lock().await;
            modality_coordinator.coordinate_input(processed_input).await?
        };
        
        // Send the coordinated input to OZONE STUDIO for processing
        let ecosystem_response = {
            let mut ecosystem_coordinator = self.ecosystem_coordinator.lock().await;
            ecosystem_coordinator.process_human_input(coordinated_input).await?
        };
        
        // Determine which module(s) should generate the output
        let output_module_type = {
            let mut modality_coordinator = self.modality_coordinator.lock().await;
            modality_coordinator.determine_output_modality(&ecosystem_response).await?
        };
        
        // Generate the appropriate output
        let output = {
            let mut active_modules = self.active_modules.write().await;
            let module = active_modules.get_mut(&output_module_type)
                .ok_or(InterfaceModuleError::ModuleNotFound { module_type: output_module_type })?;
            
            module.generate_output(ecosystem_response).await?
        };
        
        // Update metrics and monitoring
        {
            let mut metrics_collector = self.metrics_collector.lock().await;
            metrics_collector.record_interaction(module_type, output.clone()).await?;
        }
        
        Ok(output)
    }
    
    /// Get comprehensive status of all interface modules
    pub async fn get_interface_status(&self) -> Result<InterfaceManagerStatus, InterfaceModuleError> {
        let mut module_statuses = HashMap::new();
        
        // Collect health status from each active module
        {
            let active_modules = self.active_modules.read().await;
            for (module_type, module) in active_modules.iter() {
                let health = module.health_check().await?;
                let metrics = module.metrics_check().await?;
                
                module_statuses.insert(module_type.clone(), ModuleStatus {
                    module_type: module_type.clone(),
                    health,
                    metrics,
                    last_activity: SystemTime::now(), // This would be tracked properly
                });
            }
        }
        
        // Get overall coordination status
        let coordination_status = {
            let modality_coordinator = self.modality_coordinator.lock().await;
            modality_coordinator.get_coordination_status().await?
        };
        
        // Get ecosystem coordination status
        let ecosystem_status = {
            let ecosystem_coordinator = self.ecosystem_coordinator.lock().await;
            ecosystem_coordinator.get_ecosystem_connection_status().await?
        };
        
        Ok(InterfaceManagerStatus {
            overall_health: OverallHealth::Healthy, // Would be calculated from module statuses
            active_modules: module_statuses,
            coordination_status,
            ecosystem_status,
            uptime: SystemTime::now(), // Would track actual uptime
            total_interactions: 0, // Would track actual interactions
        })
    }
    
    /// Shutdown all interface modules gracefully
    pub async fn shutdown(&mut self) -> Result<(), InterfaceModuleError> {
        // Shutdown all active modules
        {
            let mut active_modules = self.active_modules.write().await;
            for (module_type, module) in active_modules.iter_mut() {
                if let Err(e) = module.shutdown().await {
                    eprintln!("Error shutting down module {:?}: {}", module_type, e);
                }
            }
            active_modules.clear();
        }
        
        // Shutdown coordination systems
        {
            let mut modality_coordinator = self.modality_coordinator.lock().await;
            modality_coordinator.shutdown().await?;
        }
        
        {
            let mut ecosystem_coordinator = self.ecosystem_coordinator.lock().await;
            ecosystem_coordinator.shutdown().await?;
        }
        
        Ok(())
    }
}

// =============================================================================
// RE-EXPORT INTERFACE MODULE IMPLEMENTATIONS
// =============================================================================

// Import and re-export each interface module implementation
pub use text_scribe_module::{
    TextScribeModule,
    TextScribeUI,
    ConversationInterface,
    DocumentHandler,
    WebInterface,
    MobileInterface,
    CLIInterface,
    TextScribeConfiguration,
    ChatUIState,
    ConversationContext,
};

pub use voice_module::{
    VoiceModule,
    VoiceUI,
    SpeechInterface,
    AudioControls,
    VoiceVisualization,
    VoiceConfiguration,
    AudioProcessingState,
    SpeechRecognitionContext,
};

pub use visual_gesture_module::{
    VisualGestureModule,
    GestureUI,
    SpatialInterface,
    CameraInterface,
    ARInterface,
    VisualGestureConfiguration,
    SpatialInteractionState,
    GestureRecognitionContext,
};

pub use eeg_module::{
    EEGModule,
    EEGUI,
    NeuralInterface,
    ThoughtVisualization,
    PrivacyControls,
    EEGConfiguration,
    NeuralProcessingState,
    BrainSignalContext,
};

pub use custom_module_framework::{
    CustomModuleFramework,
    CustomInterfaceAPI,
    PluginManager,
    InterfaceTemplate,
    ModuleBuilder,
    PluginLoader,
    CustomModuleConfiguration,
};

// =============================================================================
// CORE TYPE DEFINITIONS
// =============================================================================

/*
These types define the vocabulary that all interface modules use to communicate.
Think of these as the "common language" that lets completely different interface
types work together seamlessly.
*/

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterfaceType {
    // Current interface modules
    TextScribe,          // Chat-based text interface with SCRIBE coordination
    Voice,               // Voice input/output with speech recognition and synthesis
    VisualGesture,       // Visual input with gesture recognition and spatial output
    EEG,                 // Brain-computer interface with neural signal processing
    
    // Future interface types (framework can handle these when they're created)
    Haptic,              // Touch and force feedback interfaces
    AR,                  // Augmented reality spatial interfaces
    VR,                  // Virtual reality immersive interfaces  
    IoT,                 // Internet of Things device interfaces
    API,                 // Programmatic API interfaces
    Biometric,           // Biometric input interfaces (heart rate, etc.)
    
    // Custom interfaces that users can create
    Custom(String),      // User-defined interface types
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputType {
    // Text input modalities
    Keyboard,
    Touchscreen,
    Handwriting,
    
    // Audio input modalities
    Microphone,
    SpeechRecognition,
    AudioCommands,
    
    // Visual input modalities
    Camera,
    GestureRecognition,
    EyeTracking,
    FacialExpression,
    
    // Neural input modalities
    EEGSignals,
    BrainWaves,
    ThoughtPatterns,
    
    // Physical input modalities
    TouchSensors,
    MotionSensors,
    BiometricSensors,
    
    // Custom input types
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputType {
    // Visual output modalities
    Screen,
    Display,
    LEDs,
    Projection,
    AR,
    VR,
    
    // Audio output modalities
    Speakers,
    Headphones,
    SpeechSynthesis,
    AudioFeedback,
    
    // Haptic output modalities
    Vibration,
    Forcefeedback,
    Tactile,
    
    // Neural output modalities (future)
    NeuralStimulation,
    DirectBrainFeedback,
    
    // Custom output types
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterfaceCapability {
    // Basic interaction capabilities
    TextProcessing,
    VoiceRecognition,
    GestureRecognition,
    VisualProcessing,
    
    // Advanced capabilities
    RealTimeProcessing,
    OfflineMode,
    MultiUser,
    Accessibility,
    
    // Security capabilities
    BiometricAuth,
    EncryptedCommunication,
    SessionManagement,
    
    // Coordination capabilities
    ModalitySwitching,
    ContextPreservation,
    StateSync,
    
    // Custom capabilities
    Custom(String),
}

// =============================================================================
// CONFIGURATION TYPES
// =============================================================================

/*
Configuration is crucial for a modular system. Each interface module needs to be
highly configurable to adapt to different user needs, device capabilities, and
deployment scenarios.
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfiguration {
    pub module_type: InterfaceType,
    pub enabled: bool,
    pub initialization_params: HashMap<String, serde_json::Value>,
    pub ui_configuration: UIConfiguration,
    pub security_configuration: SecurityConfiguration,
    pub performance_configuration: PerformanceConfiguration,
    pub accessibility_configuration: AccessibilityConfiguration,
    pub custom_settings: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfiguration {
    pub theme: String,
    pub layout: String,
    pub responsive_design: bool,
    pub animations_enabled: bool,
    pub custom_css: Option<String>,
    pub viewport_settings: ViewportSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportSettings {
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub scaling_factor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfiguration {
    pub authentication_required: bool,
    pub session_timeout: Duration,
    pub encryption_enabled: bool,
    pub audit_logging: bool,
    pub privacy_mode: PrivacyMode,
    pub data_retention: DataRetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyMode {
    Standard,
    Enhanced,
    Maximum,
    Custom(HashMap<String, serde_json::Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRetentionPolicy {
    pub conversation_history_days: u32,
    pub session_data_days: u32,
    pub metrics_data_days: u32,
    pub auto_delete_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfiguration {
    pub response_timeout: Duration,
    pub max_concurrent_users: u32,
    pub resource_limits: ResourceLimits,
    pub caching_enabled: bool,
    pub optimization_level: OptimizationLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_percent: f32,
    pub max_storage_mb: u64,
    pub max_network_mbps: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    Battery,      // Optimize for power efficiency
    Performance,  // Optimize for speed and responsiveness  
    Balanced,     // Balance between efficiency and performance
    Custom(HashMap<String, serde_json::Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityConfiguration {
    pub screen_reader_support: bool,
    pub high_contrast_mode: bool,
    pub large_text_support: bool,
    pub voice_navigation: bool,
    pub keyboard_only_navigation: bool,
    pub color_blind_support: bool,
    pub custom_accessibility_features: Vec<String>,
}

// =============================================================================
// EVENT AND DATA TYPES
// =============================================================================

/*
These types define how information flows through the interface module system.
They need to be rich enough to capture all the nuances of different interaction
modalities while being structured enough for reliable processing.
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedInput {
    pub input_id: String,
    pub module_type: InterfaceType,
    pub input_type: InputType,
    pub content: InputContent,
    pub context: InputContext,
    pub metadata: InputMetadata,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputContent {
    // Text-based input
    Text {
        content: String,
        language: Option<String>,
        sentiment: Option<f32>,
        intent: Option<String>,
    },
    
    // Voice input
    Audio {
        transcript: String,
        audio_data: Option<Vec<u8>>,
        confidence: f32,
        speaker_id: Option<String>,
    },
    
    // Visual input
    Visual {
        description: String,
        image_data: Option<Vec<u8>>,
        recognized_objects: Vec<String>,
        gestures: Vec<String>,
    },
    
    // Neural input
    Neural {
        intent: String,
        confidence: f32,
        signal_data: Option<Vec<f32>>,
        processing_notes: Vec<String>,
    },
    
    // Document input
    Document {
        content: String,
        format: String,
        metadata: HashMap<String, serde_json::Value>,
        extracted_text: String,
    },
    
    // Command input
    Command {
        command: String,
        parameters: HashMap<String, serde_json::Value>,
        target: Option<String>,
    },
    
    // Custom input types
    Custom(HashMap<String, serde_json::Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputContext {
    pub user_id: String,
    pub session_id: String,
    pub conversation_id: Option<String>,
    pub device_id: String,
    pub location_context: Option<LocationContext>,
    pub temporal_context: TemporalContext,
    pub interaction_history: Vec<String>,
    pub current_task: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationContext {
    pub physical_location: Option<String>,
    pub digital_location: String, // Which interface, which screen, etc.
    pub spatial_context: Option<SpatialContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialContext {
    pub coordinates: (f32, f32, f32), // x, y, z coordinates
    pub orientation: (f32, f32, f32), // rotation angles
    pub scale: f32,
    pub reference_frame: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContext {
    pub time_of_day: String,
    pub day_of_week: String,
    pub timezone: String,
    pub urgency_level: UrgencyLevel,
    pub expected_response_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,       // Can wait for optimal processing
    Normal,    // Standard response time expected
    High,      // User is waiting actively  
    Critical,  // Immediate response required
    Emergency, // Override normal processing for urgent response
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMetadata {
    pub processing_time: Duration,
    pub confidence_scores: HashMap<String, f32>,
    pub quality_metrics: HashMap<String, f32>,
    pub preprocessing_steps: Vec<String>,
    pub validation_results: ValidationResults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    pub is_valid: bool,
    pub validation_errors: Vec<String>,
    pub security_check_passed: bool,
    pub content_safety_passed: bool,
}

// Ecosystem response from OZONE STUDIO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemResponse {
    pub response_id: String,
    pub response_type: ResponseType,
    pub content: ResponseContent,
    pub context: ResponseContext,
    pub metadata: ResponseMetadata,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseType {
    DirectAnswer,        // Simple answer to a question
    TaskResult,          // Result of a completed task
    TaskProgress,        // Progress update on ongoing task
    Clarification,       // Request for more information
    Error,               // Error message
    SystemStatus,        // System status information
    Notification,        // Proactive notification
    Custom(String),      // Custom response types
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseContent {
    // Text response
    Text {
        content: String,
        formatting: Option<String>,
        language: Option<String>,
        tone: Option<String>,
    },
    
    // Rich content response
    RichContent {
        text: String,
        media: Vec<MediaItem>,
        interactive_elements: Vec<InteractiveElement>,
        layout_hints: LayoutHints,
    },
    
    // Data response
    Data {
        structured_data: serde_json::Value,
        visualization_type: Option<String>,
        schema: Option<String>,
    },
    
    // Status response
    Status {
        status: String,
        details: HashMap<String, serde_json::Value>,
        actions: Vec<String>,
    },
    
    // Custom response content
    Custom(HashMap<String, serde_json::Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub media_type: String,
    pub url: Option<String>,
    pub data: Option<Vec<u8>>,
    pub alt_text: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveElement {
    pub element_type: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub event_handlers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutHints {
    pub preferred_layout: String,
    pub responsive_breakpoints: Vec<u32>,
    pub grid_preferences: Option<GridPreferences>,
    pub spacing_preferences: SpacingPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPreferences {
    pub columns: u32,
    pub rows: Option<u32>,
    pub gap: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpacingPreferences {
    pub margin: u32,
    pub padding: u32,
    pub line_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContext {
    pub conversation_id: String,
    pub response_to: Option<String>, // ID of the input this responds to
    pub continuation_context: Option<String>,
    pub suggested_follow_ups: Vec<String>,
    pub related_information: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub processing_time: Duration,
    pub confidence: f32,
    pub ai_apps_involved: Vec<ComponentType>,
    pub methodology_used: Option<String>,
    pub quality_score: f32,
    pub personalization_applied: bool,
}

// =============================================================================
// MONITORING AND HEALTH TYPES
// =============================================================================

/*
Production systems need comprehensive monitoring and health checking.
These types provide observability into how well each interface module
is performing and help identify issues before they impact users.
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleHealth {
    pub module_type: InterfaceType,
    pub overall_status: HealthStatus,
    pub component_health: HashMap<String, ComponentHealthStatus>,
    pub last_health_check: SystemTime,
    pub uptime: Duration,
    pub error_rate: f32,
    pub response_time_avg: Duration,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,       // All systems operating normally
    Degraded,      // Some issues but still functional
    Critical,      // Major issues affecting functionality
    Offline,       // Module is not responding
    Maintenance,   // Module is in maintenance mode
    Starting,      // Module is starting up
    Stopping,      // Module is shutting down
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealthStatus {
    pub component_name: String,
    pub status: HealthStatus,
    pub last_check: SystemTime,
    pub error_count: u32,
    pub warning_count: u32,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: u64,
    pub storage_usage_mb: u64,
    pub network_usage_mbps: f32,
    pub active_connections: u32,
    pub thread_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetrics {
    pub module_type: InterfaceType,
    pub interactions_count: u64,
    pub successful_interactions: u64,
    pub failed_interactions: u64,
    pub average_response_time: Duration,
    pub peak_response_time: Duration,
    pub user_satisfaction_score: Option<f32>,
    pub feature_usage: HashMap<String, u64>,
    pub error_breakdown: HashMap<String, u32>,
    pub performance_trends: PerformanceTrends,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrends {
    pub response_time_trend: Vec<(SystemTime, Duration)>,
    pub error_rate_trend: Vec<(SystemTime, f32)>,
    pub usage_trend: Vec<(SystemTime, u64)>,
    pub resource_usage_trend: Vec<(SystemTime, ResourceUsage)>,
}

// =============================================================================
// STATE MANAGEMENT TYPES
// =============================================================================

/*
Interface modules need to maintain state across interactions and sessions.
This is especially important for conversational interfaces where context
matters enormously for providing good user experiences.
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleState {
    pub module_type: InterfaceType,
    pub state_version: u32,
    pub last_updated: SystemTime,
    pub user_sessions: HashMap<String, UserSessionState>,
    pub global_state: HashMap<String, serde_json::Value>,
    pub cached_data: HashMap<String, CachedData>,
    pub configuration_overrides: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSessionState {
    pub session_id: String,
    pub user_id: String,
    pub started_at: SystemTime,
    pub last_activity: SystemTime,
    pub conversation_context: ConversationContext,
    pub user_preferences: UserPreferences,
    pub temporary_data: HashMap<String, serde_json::Value>,
    pub interaction_history: Vec<InteractionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub conversation_id: String,
    pub current_topic: Option<String>,
    pub context_history: Vec<ContextFrame>,
    pub user_intent: Option<String>,
    pub conversation_phase: ConversationPhase,
    pub emotional_context: EmotionalContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFrame {
    pub frame_id: String,
    pub timestamp: SystemTime,
    pub content_summary: String,
    pub key_entities: Vec<String>,
    pub relationships: Vec<String>,
    pub importance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationPhase {
    Opening,        // Starting a new conversation
    Information,    // Gathering or providing information
    TaskExecution,  // Working on a specific task
    Clarification,  // Resolving ambiguities
    Conclusion,     // Wrapping up the conversation
    Maintenance,    // Maintaining ongoing relationship
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalContext {
    pub user_mood: Option<String>,
    pub conversation_tone: String,
    pub stress_indicators: Vec<String>,
    pub satisfaction_level: Option<f32>,
    pub engagement_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub interface_preferences: InterfacePreferences,
    pub communication_preferences: CommunicationPreferences,
    pub accessibility_preferences: AccessibilityPreferences,
    pub privacy_preferences: PrivacyPreferences,
    pub personalization_preferences: PersonalizationPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfacePreferences {
    pub preferred_modalities: Vec<InterfaceType>,
    pub ui_theme: String,
    pub animation_preferences: AnimationPreferences,
    pub layout_preferences: LayoutPreferences,
    pub interaction_speed: InteractionSpeed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnimationPreferences {
    None,           // No animations
    Minimal,        // Only essential animations
    Standard,       // Standard animation set
    Rich,           // Full animations
    Custom(HashMap<String, bool>), // Custom animation settings
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPreferences {
    pub density: LayoutDensity,
    pub information_hierarchy: InformationHierarchy,
    pub color_scheme: ColorScheme,
    pub typography_preferences: TypographyPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutDensity {
    Compact,        // Dense information layout
    Standard,       // Balanced information density
    Spacious,       // Generous spacing
    Custom(f32),    // Custom density factor
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InformationHierarchy {
    Flat,           // All information at same level
    Structured,     // Clear hierarchical organization
    Progressive,    // Progressive disclosure
    Custom(String), // Custom hierarchy rules
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorScheme {
    Light,
    Dark,
    HighContrast,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypographyPreferences {
    pub font_size: FontSize,
    pub font_family: String,
    pub line_spacing: f32,
    pub reading_mode: ReadingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
    Custom(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadingMode {
    Standard,
    DyslexiaFriendly,
    LowVision,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionSpeed {
    Slow,           // Longer timeouts, more confirmation
    Standard,       // Standard interaction timing
    Fast,           // Quick interactions, minimal delays
    Adaptive,       // Adapt to user's interaction patterns
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPreferences {
    pub response_length: ResponseLength,
    pub technical_level: TechnicalLevel,
    pub explanation_style: ExplanationStyle,
    pub language_preferences: LanguagePreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseLength {
    Concise,        // Brief, to-the-point responses
    Standard,       // Balanced response length
    Detailed,       // Comprehensive responses
    Adaptive,       // Adapt to context and user behavior
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TechnicalLevel {
    Beginner,       // Non-technical explanations
    Intermediate,   // Some technical detail
    Advanced,       // Full technical detail
    Expert,         // Assume high technical knowledge
    Adaptive,       // Adapt to user's demonstrated knowledge
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExplanationStyle {
    Practical,      // Focus on practical applications
    Theoretical,    // Include theoretical background
    StepByStep,     // Break down into steps
    Conceptual,     // Focus on concepts and principles
    Examples,       // Heavy use of examples
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguagePreferences {
    pub primary_language: String,
    pub secondary_languages: Vec<String>,
    pub dialect_preferences: Option<String>,
    pub cultural_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityPreferences {
    pub visual_accessibility: VisualAccessibilityPreferences,
    pub auditory_accessibility: AuditoryAccessibilityPreferences,
    pub motor_accessibility: MotorAccessibilityPreferences,
    pub cognitive_accessibility: CognitiveAccessibilityPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualAccessibilityPreferences {
    pub screen_reader_compatible: bool,
    pub high_contrast: bool,
    pub large_text: bool,
    pub color_blind_friendly: bool,
    pub reduce_motion: bool,
    pub focus_indicators: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditoryAccessibilityPreferences {
    pub captions_enabled: bool,
    pub audio_descriptions: bool,
    pub visual_indicators_for_audio: bool,
    pub adjustable_audio_speed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorAccessibilityPreferences {
    pub keyboard_only_navigation: bool,
    pub extended_timeouts: bool,
    pub large_click_targets: bool,
    pub voice_control: bool,
    pub gesture_alternatives: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveAccessibilityPreferences {
    pub simple_language: bool,
    pub clear_navigation: bool,
    pub consistent_interface: bool,
    pub error_prevention: bool,
    pub progress_indicators: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyPreferences {
    pub data_collection_level: DataCollectionLevel,
    pub analytics_sharing: bool,
    pub personalization_data_usage: bool,
    pub conversation_history_retention: ConversationRetention,
    pub third_party_sharing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataCollectionLevel {
    Minimal,        // Only essential data
    Standard,       // Standard data collection
    Enhanced,       // Additional data for better experience
    Full,           // All available data
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationRetention {
    Session,        // Only for current session
    Short,          // Few days
    Standard,       // Few weeks
    Long,           // Few months
    Permanent,      // Keep indefinitely
    Custom(Duration), // Custom retention period
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizationPreferences {
    pub adaptive_interface: bool,
    pub learning_from_interactions: bool,
    pub predictive_suggestions: bool,
    pub contextual_customization: bool,
    pub cross_device_personalization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedData {
    pub data_type: String,
    pub data: serde_json::Value,
    pub cached_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub access_count: u32,
    pub last_accessed: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub interaction_id: String,
    pub timestamp: SystemTime,
    pub input_summary: String,
    pub output_summary: String,
    pub success: bool,
    pub response_time: Duration,
    pub user_satisfaction: Option<f32>,
}

// =============================================================================
// SUPPORTING SYSTEM TYPES
// =============================================================================

/*
These types define the supporting systems that help coordinate between
interface modules and provide shared services.
*/

// Configuration for the entire interface management system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceManagerConfig {
    pub modality_coordination: ModalityCoordinationConfig,
    pub context_management: ContextManagementConfig,
    pub preference_management: PreferenceManagementConfig,
    pub ui_components: UIComponentsConfig,
    pub ecosystem_coordination: EcosystemCoordinationConfig,
    pub metrics_collection: MetricsCollectionConfig,
    pub health_monitoring: HealthMonitoringConfig,
    pub module_loading: ModuleLoadingConfig,
    pub plugin_discovery: PluginDiscoveryConfig,
    pub event_distribution: EventDistributionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityCoordinationConfig {
    pub enabled: bool,
    pub automatic_switching: bool,
    pub context_preservation: bool,
    pub preference_learning: bool,
    pub fallback_modality: InterfaceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManagementConfig {
    pub context_preservation_duration: Duration,
    pub max_context_frames: usize,
    pub context_compression: bool,
    pub cross_session_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceManagementConfig {
    pub automatic_learning: bool,
    pub explicit_preference_collection: bool,
    pub preference_synchronization: bool,
    pub preference_backup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIComponentsConfig {
    pub notification_config: NotificationConfig,
    pub status_config: StatusConfig,
    pub accessibility_config: AccessibilityConfig,
    pub theme_config: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub persistence: Duration,
    pub priority_filtering: bool,
    pub user_control: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusConfig {
    pub real_time_updates: bool,
    pub detailed_status: bool,
    pub historical_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    pub default_features: Vec<String>,
    pub automatic_detection: bool,
    pub user_customization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub default_theme: String,
    pub theme_switching: bool,
    pub custom_themes: bool,
    pub automatic_theme_selection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemCoordinationConfig {
    pub ozone_studio_endpoint: String,
    pub coordination_timeout: Duration,
    pub retry_attempts: u32,
    pub health_check_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsCollectionConfig {
    pub enabled: bool,
    pub collection_interval: Duration,
    pub detailed_metrics: bool,
    pub performance_tracking: bool,
    pub user_analytics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitoringConfig {
    pub enabled: bool,
    pub check_interval: Duration,
    pub alert_thresholds: HashMap<String, f32>,
    pub automatic_recovery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLoadingConfig {
    pub dynamic_loading: bool,
    pub module_directories: Vec<String>,
    pub security_validation: bool,
    pub dependency_resolution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDiscoveryConfig {
    pub enabled: bool,
    pub discovery_directories: Vec<String>,
    pub automatic_loading: bool,
    pub version_compatibility_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDistributionConfig {
    pub enabled: bool,
    pub event_buffering: bool,
    pub reliable_delivery: bool,
    pub event_filtering: bool,
}

// =============================================================================
// STATUS AND MONITORING TYPES
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceManagerStatus {
    pub overall_health: OverallHealth,
    pub active_modules: HashMap<InterfaceType, ModuleStatus>,
    pub coordination_status: CoordinationStatus,
    pub ecosystem_status: EcosystemConnectionStatus,
    pub uptime: SystemTime,
    pub total_interactions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverallHealth {
    Healthy,
    Degraded,
    Critical,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatus {
    pub module_type: InterfaceType,
    pub health: ModuleHealth,
    pub metrics: ModuleMetrics,
    pub last_activity: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationStatus {
    pub modality_coordination_active: bool,
    pub context_bridge_healthy: bool,
    pub preference_learning_active: bool,
    pub active_coordinations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConnectionStatus {
    pub connected_to_ozone_studio: bool,
    pub last_communication: SystemTime,
    pub communication_quality: f32,
    pub pending_requests: u32,
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/*
Comprehensive error handling is crucial for a production system with
multiple interface modules that could fail in different ways.
*/

#[derive(Error, Debug)]
pub enum InterfaceModuleError {
    #[error("Module not found: {module_type:?}")]
    ModuleNotFound { module_type: InterfaceType },
    
    #[error("Module initialization failed: {module_type:?} - {details}")]
    ModuleInitializationFailed { module_type: InterfaceType, details: String },
    
    #[error("Module operation failed: {module_type:?} - {operation} - {details}")]
    ModuleOperationFailed { module_type: InterfaceType, operation: String, details: String },
    
    #[error("Input processing error: {input_type:?} - {details}")]
    InputProcessingError { input_type: InputType, details: String },
    
    #[error("Output generation error: {output_type:?} - {details}")]
    OutputGenerationError { output_type: OutputType, details: String },
    
    #[error("Coordination error: {details}")]
    CoordinationError { details: String },
    
    #[error("Configuration error: {module_type:?} - {details}")]
    ConfigurationError { module_type: InterfaceType, details: String },
    
    #[error("Security error: {operation} - {details}")]
    SecurityError { operation: String, details: String },
    
    #[error("Authentication error: {details}")]
    AuthenticationError { details: String },
    
    #[error("Session management error: {details}")]
    SessionError { details: String },
    
    #[error("Resource error: {resource} - {details}")]
    ResourceError { resource: String, details: String },
    
    #[error("Network error: {endpoint} - {details}")]
    NetworkError { endpoint: String, details: String },
    
    #[error("Timeout error: {operation} timed out after {duration:?}")]
    TimeoutError { operation: String, duration: Duration },
    
    #[error("Validation error: {validation_type} - {details}")]
    ValidationError { validation_type: String, details: String },
    
    #[error("Plugin error: {plugin_name} - {details}")]
    PluginError { plugin_name: String, details: String },
    
    #[error("State management error: {details}")]
    StateError { details: String },
    
    #[error("Serialization error: {details}")]
    SerializationError { details: String },
    
    #[error("Protocol error: {protocol} - {details}")]
    ProtocolError { protocol: String, details: String },
}

// =============================================================================
// RESULT TYPE AND UTILITY MACROS
// =============================================================================

pub type InterfaceResult<T> = Result<T, InterfaceModuleError>;

// Utility macro for creating interface module errors
#[macro_export]
macro_rules! interface_error {
    ($error_type:ident { $($field:ident: $value:expr),* }) => {
        InterfaceModuleError::$error_type { $($field: $value.into()),* }
    };
}

// =============================================================================
// PLACEHOLDER SUPPORTING SYSTEM STRUCTS
// =============================================================================

/*
These are placeholder structs for the supporting systems. In a real implementation,
these would be fully implemented with their own modules and comprehensive functionality.
The key point is that they demonstrate the architecture - each interface module
gets access to these shared services.
*/

// Modality coordination system
pub struct ModalityCoordinator {
    config: ModalityCoordinationConfig,
    registered_modules: HashMap<InterfaceType, Vec<InputType>>,
    coordination_rules: Vec<CoordinationRule>,
    active_coordinations: HashMap<String, ActiveCoordination>,
}

impl ModalityCoordinator {
    pub fn new(config: ModalityCoordinationConfig) -> Self {
        Self {
            config,
            registered_modules: HashMap::new(),
            coordination_rules: Vec::new(),
            active_coordinations: HashMap::new(),
        }
    }
    
    pub async fn register_module(&mut self, module_type: InterfaceType) -> InterfaceResult<()> {
        // Implementation would register the module and its capabilities
        Ok(())
    }
    
    pub async fn coordinate_input(&mut self, input: ProcessedInput) -> InterfaceResult<ProcessedInput> {
        // Implementation would coordinate the input with other modalities
        Ok(input)
    }
    
    pub async fn determine_output_modality(&mut self, response: &EcosystemResponse) -> InterfaceResult<InterfaceType> {
        // Implementation would determine the best output modality
        Ok(InterfaceType::TextScribe)
    }
    
    pub async fn get_coordination_status(&self) -> InterfaceResult<CoordinationStatus> {
        // Implementation would return current coordination status
        Ok(CoordinationStatus {
            modality_coordination_active: true,
            context_bridge_healthy: true,
            preference_learning_active: true,
            active_coordinations: 0,
        })
    }
    
    pub async fn shutdown(&mut self) -> InterfaceResult<()> {
        // Implementation would clean up coordination resources
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CoordinationRule {
    pub rule_id: String,
    pub conditions: Vec<String>,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ActiveCoordination {
    pub coordination_id: String,
    pub involved_modules: Vec<InterfaceType>,
    pub started_at: SystemTime,
}

// Authentication manager for user authentication across all interface modules
pub struct AuthenticationManager {
    config: SecurityConfig,
    active_sessions: HashMap<String, UserSession>,
    certificate_store: HashMap<String, UserCertificate>,
}

impl AuthenticationManager {
    pub fn new(config: SecurityConfig) -> Result<Self, InterfaceModuleError> {
        Ok(Self {
            config,
            active_sessions: HashMap::new(),
            certificate_store: HashMap::new(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub session_id: String,
    pub user_id: String,
    pub created_at: SystemTime,
    pub last_activity: SystemTime,
}

// Ecosystem coordinator for communicating with OZONE STUDIO
pub struct EcosystemCoordinator {
    config: EcosystemCoordinationConfig,
    connection_status: EcosystemConnectionStatus,
}

impl EcosystemCoordinator {
    pub fn new(config: EcosystemCoordinationConfig) -> Self {
        Self {
            config,
            connection_status: EcosystemConnectionStatus {
                connected_to_ozone_studio: false,
                last_communication: SystemTime::now(),
                communication_quality: 0.0,
                pending_requests: 0,
            },
        }
    }
    
    pub async fn notify_module_registration(&mut self, module_type: InterfaceType) -> InterfaceResult<()> {
        // Implementation would notify OZONE STUDIO about new interface capability
        Ok(())
    }
    
    pub async fn process_human_input(&mut self, input: ProcessedInput) -> InterfaceResult<EcosystemResponse> {
        // Implementation would send input to OZONE STUDIO and get response
        Ok(EcosystemResponse {
            response_id: Uuid::new_v4().to_string(),
            response_type: ResponseType::DirectAnswer,
            content: ResponseContent::Text {
                content: "This is a placeholder response".to_string(),
                formatting: None,
                language: Some("en".to_string()),
                tone: Some("helpful".to_string()),
            },
            context: ResponseContext {
                conversation_id: "placeholder".to_string(),
                response_to: None,
                continuation_context: None,
                suggested_follow_ups: Vec::new(),
                related_information: Vec::new(),
            },
            metadata: ResponseMetadata {
                processing_time: Duration::from_millis(100),
                confidence: 0.95,
                ai_apps_involved: vec![ComponentType::TextFrameworkSpecialist],
                methodology_used: None,
                quality_score: 0.9,
                personalization_applied: false,
            },
            timestamp: SystemTime::now(),
        })
    }
    
    pub async fn get_ecosystem_connection_status(&self) -> InterfaceResult<EcosystemConnectionStatus> {
        Ok(self.connection_status.clone())
    }
    
    pub async fn shutdown(&mut self) -> InterfaceResult<()> {
        // Implementation would clean up ecosystem connections
        Ok(())
    }
}

// Metrics collector for gathering performance and usage data
pub struct InterfaceMetricsCollector {
    config: MetricsCollectionConfig,
    collected_metrics: HashMap<InterfaceType, Vec<MetricDataPoint>>,
}

impl InterfaceMetricsCollector {
    pub fn new(config: MetricsCollectionConfig) -> Self {
        Self {
            config,
            collected_metrics: HashMap::new(),
        }
    }
    
    pub async fn record_interaction(&mut self, module_type: InterfaceType, output: OutputEvent) -> InterfaceResult<()> {
        // Implementation would record the interaction for metrics
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MetricDataPoint {
    pub timestamp: SystemTime,
    pub metric_type: String,
    pub value: f64,
    pub metadata: HashMap<String, String>,
}

// Health monitor for tracking the health of all interface modules
pub struct InterfaceHealthMonitor {
    config: HealthMonitoringConfig,
    health_data: HashMap<InterfaceType, Vec<HealthDataPoint>>,
}

impl InterfaceHealthMonitor {
    pub fn new(config: HealthMonitoringConfig) -> Self {
        Self {
            config,
            health_data: HashMap::new(),
        }
    }
    
    pub async fn start_monitoring_module(&mut self, module_type: InterfaceType) -> InterfaceResult<()> {
        // Implementation would start health monitoring for the module
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct HealthDataPoint {
    pub timestamp: SystemTime,
    pub status: HealthStatus,
    pub metrics: HashMap<String, f64>,
}

// Module loader for dynamically loading interface modules
pub struct ModuleLoader {
    config: ModuleLoadingConfig,
    loaded_modules: HashMap<String, ModuleInfo>,
}

impl ModuleLoader {
    pub fn new(config: ModuleLoadingConfig) -> Self {
        Self {
            config,
            loaded_modules: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub module_name: String,
    pub module_version: String,
    pub module_path: String,
    pub loaded_at: SystemTime,
}

// Plugin registry for discovering and managing plugins
pub struct PluginRegistry {
    config: PluginDiscoveryConfig,
    discovered_plugins: HashMap<String, PluginInfo>,
}

impl PluginRegistry {
    pub fn new(config: PluginDiscoveryConfig) -> Self {
        Self {
            config,
            discovered_plugins: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub plugin_name: String,
    pub plugin_version: String,
    pub plugin_type: InterfaceType,
    pub capabilities: Vec<InterfaceCapability>,
}

// Event bus for distributing events across interface modules
pub struct InterfaceEventBus {
    config: EventDistributionConfig,
    event_channels: HashMap<String, mpsc::UnboundedSender<InterfaceEvent>>,
}

impl InterfaceEventBus {
    pub fn new(config: EventDistributionConfig) -> Self {
        Self {
            config,
            event_channels: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceEvent {
    pub event_id: String,
    pub event_type: String,
    pub source_module: InterfaceType,
    pub target_modules: Vec<InterfaceType>,
    pub payload: serde_json::Value,
    pub timestamp: SystemTime,
}

// =============================================================================
// CONSTANTS AND VERSION INFO
// =============================================================================

pub const INTERFACE_MODULES_VERSION: &str = "1.0.0";
pub const MAX_CONCURRENT_MODULES: usize = 16;
pub const DEFAULT_MODULE_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_COORDINATION_TIMEOUT: Duration = Duration::from_secs(10);
pub const MAX_CACHED_STATES: usize = 1000;
pub const MAX_INTERACTION_HISTORY: usize = 10000;
