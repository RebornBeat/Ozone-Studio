use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use std::fmt;

// Async runtime and concurrency
use tokio::sync::{RwLock, Mutex, mpsc, oneshot, broadcast};
use tokio::time::{sleep, timeout, interval, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Web framework and HTTP handling
use axum::{Router, extract::State, response::Json, http::StatusCode};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

// Import shared protocol and security types
use shared_protocols::{
    EcosystemIdentity,
    ComponentType,
    HumanGuidance,
    AuthorityLevel,
    ProtocolError,
};

use shared_security::{
    SecurityError,
    SecureComponent,
    AuthenticationCredentials,
    SecurityConfig,
    UserCertificate,
    SessionManager,
};

// Core interface modules
pub mod text_interface;
pub mod document_handling;
pub mod task_interruption;
pub mod monitoring;
pub mod methodology_creation;
pub mod interface_modules;
pub mod modality_coordination;
pub mod shared_ui_components;

// System integration modules
pub mod user_authentication;
pub mod interfaces;
pub mod api;
pub mod utils;
pub mod security;

// Re-export core interface types
pub use text_interface::{
    TextProcessor,
    NaturalLanguageHandler,
    CommandInterpreter,
    ResponseFormatter,
    ConversationManager,
    TextProcessingRequest,
    TextProcessingResponse,
    ConversationContext,
    UserPreferences,
};

pub use document_handling::{
    DocumentProcessor,
    UploadHandler,
    FormatDetector,
    ContentExtractor,
    ScribeCoordinator,
    DocumentMetadata,
    ProcessingOptions,
    ExtractionResult,
    DocumentAnalysis,
};

pub use task_interruption::{
    InterruptionHandler,
    OverrideCoordinator,
    AuthorityValidator,
    SafePauseManager,
    ResumptionCoordinator,
    InterruptionRequest,
    InterruptionResponse,
    ModificationInstructions,
    SafetyRequirements,
};

pub use monitoring::{
    EcosystemMonitor,
    TaskVisualizer,
    ProgressTracker,
    StatusReporter,
    HealthDashboard,
    MonitoringMetrics,
    SystemStatus,
    TaskProgress,
    HealthIndicators,
};

pub use methodology_creation::{
    CreationInterface,
    GuidanceCollector,
    RequirementAnalyzer,
    ZSEICoordinator,
    ValidationHandler,
    MethodologyCreationRequest,
    CreationGuidance,
    RequirementSpecification,
    ValidationResult,
};

pub use interface_modules::{
    InterfaceManager,
    TextScribeModule,
    VoiceModule,
    VisualGestureModule,
    EEGModule,
    CustomModuleFramework,
    InterfaceModule,
    InterfaceType,
    ModuleConfiguration,
};

pub use modality_coordination::{
    InputCoordinator,
    OutputCoordinator,
    ContextBridge,
    PreferenceManager,
    ModalityConfig,
    InputEvent,
    OutputEvent,
    ContextTransition,
};

pub use shared_ui_components::{
    NotificationSystem,
    StatusIndicators,
    AccessibilityFramework,
    ThemeManager,
    UIComponent,
    NotificationLevel,
    AccessibilityFeature,
};

// User authentication exports
pub use user_authentication::{
    CertificatePairing,
    DeviceRegistration,
    SessionManagement,
    MultiFactorAuth,
    UserAuthorization,
    UserRegistrationFlow,
    DevicePairingFlow,
    AuthenticationState,
};

// Interface exports
pub use interfaces::{
    OzoneInterface,
    ScribeInterface,
    NexusInterface,
    EcosystemInterface,
    InterfaceCoordination,
};

// Core BRIDGE configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub text_interface: TextInterfaceConfig,
    pub document_handling: DocumentHandlingConfig,
    pub task_interruption: TaskInterruptionConfig,
    pub monitoring: MonitoringConfig,
    pub methodology_creation: MethodologyCreationConfig,
    pub interface_modules: InterfaceModulesConfig,
    pub user_authentication: UserAuthenticationConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInterfaceConfig {
    pub text_processing: bool,
    pub natural_language_handling: bool,
    pub command_interpretation: bool,
    pub response_formatting: bool,
    pub conversation_management: bool,
    pub max_conversation_length: usize,
    pub context_preservation: bool,
    pub personalization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHandlingConfig {
    pub document_processing: bool,
    pub upload_handling: bool,
    pub format_detection: bool,
    pub content_extraction: bool,
    pub scribe_coordination: bool,
    pub max_file_size: u64,
    pub supported_formats: Vec<DocumentFormat>,
    pub auto_processing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentFormat {
    PDF,
    DOCX,
    TXT,
    MD,
    HTML,
    RTF,
    ODT,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInterruptionConfig {
    pub interruption_handling: bool,
    pub override_coordination: bool,
    pub authority_validation: bool,
    pub safe_pause_management: bool,
    pub resumption_coordination: bool,
    pub interruption_timeout: Duration,
    pub authority_levels: Vec<AuthorityLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub ecosystem_monitoring: bool,
    pub task_visualization: bool,
    pub progress_tracking: bool,
    pub status_reporting: bool,
    pub health_dashboard: bool,
    pub monitoring_interval: Duration,
    pub alert_thresholds: AlertThresholds,
    pub visualization_options: VisualizationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub error_rate_threshold: f64,
    pub response_time_threshold: Duration,
    pub resource_usage_threshold: f64,
    pub health_score_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationOptions {
    pub real_time_updates: bool,
    pub interactive_elements: bool,
    pub customizable_dashboards: bool,
    pub export_capabilities: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyCreationConfig {
    pub creation_interface: bool,
    pub guidance_collection: bool,
    pub requirement_analysis: bool,
    pub zsei_coordination: bool,
    pub validation_handling: bool,
    pub guided_creation: bool,
    pub template_library: bool,
    pub validation_strictness: ValidationStrictness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationStrictness {
    Permissive,
    Standard,
    Strict,
    Comprehensive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceModulesConfig {
    pub interface_management: bool,
    pub text_scribe_module: TextScribeModuleConfig,
    pub voice_module: VoiceModuleConfig,
    pub visual_gesture_module: VisualGestureModuleConfig,
    pub eeg_module: EEGModuleConfig,
    pub custom_module_framework: CustomModuleFrameworkConfig,
    pub modality_coordination: ModalityCoordinationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextScribeModuleConfig {
    pub enabled: bool,
    pub chat_interface: bool,
    pub document_interface: bool,
    pub web_interface: bool,
    pub mobile_interface: bool,
    pub cli_interface: bool,
    pub real_time_processing: bool,
    pub context_awareness: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceModuleConfig {
    pub enabled: bool,
    pub speech_recognition: bool,
    pub speech_synthesis: bool,
    pub voice_commands: bool,
    pub emotion_detection: bool,
    pub noise_cancellation: bool,
    pub voice_personalization: bool,
    pub audio_quality: AudioQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioQuality {
    Basic,
    Standard,
    High,
    StudioQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualGestureModuleConfig {
    pub enabled: bool,
    pub gesture_recognition: bool,
    pub eye_tracking: bool,
    pub facial_expression: bool,
    pub spatial_interaction: bool,
    pub ar_interface: bool,
    pub camera_quality: CameraQuality,
    pub gesture_library: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CameraQuality {
    Basic,
    Standard,
    High,
    Professional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EEGModuleConfig {
    pub enabled: bool,
    pub neural_interface: bool,
    pub thought_recognition: bool,
    pub neural_feedback: bool,
    pub privacy_controls: EEGPrivacyControls,
    pub signal_processing: bool,
    pub calibration_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EEGPrivacyControls {
    pub explicit_consent_required: bool,
    pub data_encryption: bool,
    pub local_processing_only: bool,
    pub automatic_deletion: bool,
    pub user_data_control: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomModuleFrameworkConfig {
    pub enabled: bool,
    pub plugin_support: bool,
    pub api_extensions: bool,
    pub custom_interfaces: bool,
    pub development_tools: bool,
    pub security_validation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityCoordinationConfig {
    pub input_coordination: bool,
    pub output_coordination: bool,
    pub context_bridging: bool,
    pub preference_management: bool,
    pub seamless_switching: bool,
    pub context_preservation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAuthenticationConfig {
    pub certificate_pairing: bool,
    pub device_registration: bool,
    pub session_management: bool,
    pub multi_factor_auth: bool,
    pub user_authorization: bool,
    pub session_timeout: Duration,
    pub device_limit: usize,
    pub security_level: UserSecurityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserSecurityLevel {
    Basic,
    Standard,
    High,
    Maximum,
}

// Core BRIDGE interaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanInteractionRequest {
    pub interaction_id: String,
    pub interaction_type: InteractionType,
    pub input_modality: InputModality,
    pub output_modality: Vec<OutputModality>,
    pub user_context: UserContext,
    pub processing_options: ProcessingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    Conversation,
    TaskRequest,
    DocumentProcessing,
    SystemQuery,
    Interruption,
    MethodologyCreation,
    Monitoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputModality {
    Text,
    Voice,
    Gesture,
    EEG,
    Document,
    Mixed(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputModality {
    Text,
    Voice,
    Visual,
    Haptic,
    Notification,
    Dashboard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: String,
    pub session_id: String,
    pub device_info: DeviceInfo,
    pub preferences: UserPreferences,
    pub conversation_history: ConversationHistory,
    pub relationship_context: RelationshipContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_type: DeviceType,
    pub capabilities: DeviceCapabilities,
    pub interface_preferences: InterfacePreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Desktop,
    Laptop,
    Mobile,
    Tablet,
    AR,
    VR,
    IoT,
    Embedded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub screen_size: Option<ScreenSize>,
    pub audio_capabilities: AudioCapabilities,
    pub camera_capabilities: Option<CameraCapabilities>,
    pub sensors: Vec<SensorType>,
    pub input_methods: Vec<InputMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
    pub pixel_density: f64,
    pub color_depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCapabilities {
    pub input: bool,
    pub output: bool,
    pub noise_cancellation: bool,
    pub spatial_audio: bool,
    pub quality: AudioQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraCapabilities {
    pub resolution: String,
    pub frame_rate: u32,
    pub depth_sensing: bool,
    pub face_detection: bool,
    pub gesture_recognition: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensorType {
    Accelerometer,
    Gyroscope,
    Magnetometer,
    GPS,
    HeartRate,
    Temperature,
    Pressure,
    EEG,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputMethod {
    Keyboard,
    Mouse,
    Touch,
    Voice,
    Camera,
    Gesture,
    EEG,
    Stylus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfacePreferences {
    pub preferred_modalities: Vec<InputModality>,
    pub accessibility_needs: Vec<AccessibilityNeed>,
    pub theme_preference: ThemePreference,
    pub interaction_style: InteractionStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessibilityNeed {
    ScreenReader,
    LargeText,
    HighContrast,
    VoiceControl,
    MotorAssistance,
    CognitiveSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThemePreference {
    Light,
    Dark,
    HighContrast,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionStyle {
    Conversational,
    Direct,
    Exploratory,
    Efficient,
    Detailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    pub recent_interactions: Vec<InteractionRecord>,
    pub context_summary: String,
    pub relationship_milestones: Vec<RelationshipMilestone>,
    pub preference_learnings: Vec<PreferenceLearning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub interaction_id: String,
    pub timestamp: SystemTime,
    pub interaction_type: InteractionType,
    pub user_input: String,
    pub system_response: String,
    pub satisfaction_rating: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMilestone {
    pub milestone_id: String,
    pub milestone_type: MilestoneType,
    pub description: String,
    pub achieved_at: SystemTime,
    pub significance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MilestoneType {
    FirstInteraction,
    TrustBuilding,
    CollaborativeSuccess,
    ProblemSolving,
    PersonalSharing,
    LongTermCommitment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceLearning {
    pub preference_type: String,
    pub learned_preference: String,
    pub confidence: f64,
    pub learned_at: SystemTime,
    pub validation_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipContext {
    pub relationship_id: String,
    pub relationship_stage: RelationshipStage,
    pub trust_level: f64,
    pub collaboration_history: Vec<CollaborationRecord>,
    pub communication_patterns: CommunicationPatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipStage {
    Initial,
    Developing,
    Established,
    Collaborative,
    Partnership,
    DeepConnection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationRecord {
    pub collaboration_id: String,
    pub collaboration_type: String,
    pub outcome: CollaborationOutcome,
    pub duration: Duration,
    pub satisfaction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationOutcome {
    Successful,
    PartialSuccess,
    Learning,
    Unsuccessful,
    Transformative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPatterns {
    pub preferred_style: InteractionStyle,
    pub response_timing_preference: ResponseTiming,
    pub detail_level_preference: DetailLevel,
    pub feedback_preferences: FeedbackPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseTiming {
    Immediate,
    Considered,
    Detailed,
    Comprehensive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetailLevel {
    Summary,
    Standard,
    Detailed,
    Comprehensive,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackPreferences {
    pub progress_updates: bool,
    pub reasoning_explanations: bool,
    pub alternative_suggestions: bool,
    pub learning_insights: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanInteractionResponse {
    pub interaction_id: String,
    pub response_content: ResponseContent,
    pub interaction_insights: InteractionInsights,
    pub relationship_development: RelationshipDevelopment,
    pub system_status: SystemStatusInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContent {
    pub primary_response: String,
    pub additional_information: Vec<String>,
    pub suggested_actions: Vec<String>,
    pub related_resources: Vec<String>,
    pub follow_up_opportunities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionInsights {
    pub user_intent_analysis: String,
    pub emotional_context: String,
    pub satisfaction_prediction: f64,
    pub learning_opportunities: Vec<String>,
    pub improvement_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipDevelopment {
    pub trust_changes: f64,
    pub understanding_improvements: Vec<String>,
    pub collaboration_opportunities: Vec<String>,
    pub relationship_milestones: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusInfo {
    pub ecosystem_health: f64,
    pub active_tasks: Vec<String>,
    pub performance_metrics: HashMap<String, f64>,
    pub availability_status: AvailabilityStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AvailabilityStatus {
    FullyAvailable,
    LimitedCapacity,
    Degraded,
    Maintenance,
    Emergency,
}

// Error types for BRIDGE
#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Interface error: {modality} - {details}")]
    InterfaceError { modality: String, details: String },
    
    #[error("Authentication error: {user_id} - {details}")]
    AuthenticationError { user_id: String, details: String },
    
    #[error("Document processing error: {document} - {details}")]
    DocumentProcessingError { document: String, details: String },
    
    #[error("Task interruption error: {task_id} - {details}")]
    TaskInterruptionError { task_id: String, details: String },
    
    #[error("Monitoring error: {component} - {details}")]
    MonitoringError { component: String, details: String },
    
    #[error("Methodology creation error: {phase} - {details}")]
    MethodologyCreationError { phase: String, details: String },
    
    #[error("Modality coordination error: {modalities} - {details}")]
    ModalityCoordinationError { modalities: String, details: String },
    
    #[error("User session error: {session_id} - {details}")]
    UserSessionError { session_id: String, details: String },
    
    #[error("Configuration error: {component} - {details}")]
    ConfigurationError { component: String, details: String },
}

// Core BRIDGE traits
pub trait HumanInterface {
    type Config;
    type Error;
    
    fn initialize_interface(config: Self::Config) -> Result<Self, Self::Error> where Self: Sized;
    fn process_human_interaction(&mut self, request: HumanInteractionRequest) -> Result<HumanInteractionResponse, Self::Error>;
    fn handle_task_interruption(&mut self, interruption: InterruptionRequest) -> Result<InterruptionResponse, Self::Error>;
    fn manage_user_session(&mut self, session_operation: SessionOperation) -> Result<SessionResult, Self::Error>;
    fn coordinate_with_ecosystem(&mut self, coordination_request: EcosystemCoordinationRequest) -> Result<EcosystemCoordinationResponse, Self::Error>;
}

pub trait MultiModalInterface {
    fn register_interface_module(&mut self, module: Box<dyn InterfaceModule>) -> Result<String, BridgeError>;
    fn coordinate_input(&mut self, input: InputEvent) -> Result<ProcessedInput, BridgeError>;
    fn coordinate_output(&mut self, response: EcosystemResponse, preferences: OutputPreferences) -> Result<OutputEvent, BridgeError>;
    fn bridge_context(&mut self, transition: ContextTransition) -> Result<ContextBridgeResult, BridgeError>;
    fn manage_preferences(&mut self, user_id: &str, preference_update: PreferenceUpdate) -> Result<(), BridgeError>;
}

pub trait UserRelationshipInterface {
    fn recognize_user(&mut self, recognition_data: UserRecognitionData) -> Result<UserIdentity, BridgeError>;
    fn develop_relationship(&mut self, interaction: InteractionRecord) -> Result<RelationshipDevelopment, BridgeError>;
    fn preserve_context(&mut self, context_preservation: ContextPreservationRequest) -> Result<ContextPreservationResult, BridgeError>;
    fn enhance_collaboration(&mut self, collaboration_request: CollaborationEnhancementRequest) -> Result<CollaborationEnhancement, BridgeError>;
}

// Additional operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionOperation {
    Create,
    Validate,
    Refresh,
    Terminate,
    Update,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResult {
    pub session_id: String,
    pub operation_success: bool,
    pub session_state: SessionState,
    pub expiration: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionState {
    Active,
    Expired,
    Suspended,
    Terminated,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemCoordinationRequest {
    pub coordination_type: EcosystemCoordinationType,
    pub target_components: Vec<ComponentType>,
    pub coordination_context: String,
    pub human_context: UserContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EcosystemCoordinationType {
    TaskRequest,
    StatusQuery,
    Interruption,
    Monitoring,
    Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemCoordinationResponse {
    pub coordination_id: String,
    pub coordination_result: String,
    pub affected_components: Vec<ComponentType>,
    pub follow_up_actions: Vec<String>,
}

// Additional interface types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedInput {
    pub input_id: String,
    pub interpreted_intent: String,
    pub confidence: f64,
    pub context_enrichment: Vec<String>,
    pub routing_recommendation: ComponentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemResponse {
    pub response_id: String,
    pub source_component: ComponentType,
    pub response_content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPreferences {
    pub preferred_modalities: Vec<OutputModality>,
    pub detail_level: DetailLevel,
    pub timing_preference: ResponseTiming,
    pub accessibility_requirements: Vec<AccessibilityNeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBridgeResult {
    pub bridge_id: String,
    pub context_preserved: bool,
    pub transition_success: bool,
    pub continuity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceUpdate {
    pub preference_category: String,
    pub new_preference: String,
    pub confidence: f64,
    pub source: PreferenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreferenceSource {
    ExplicitUserInput,
    BehaviorAnalysis,
    InteractionPattern,
    FeedbackAnalysis,
}

// Result type for BRIDGE operations
pub type BridgeResult<T> = Result<T, BridgeError>;

// Constants for BRIDGE configuration
pub const BRIDGE_VERSION: &str = "1.0.0";
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour
pub const MAX_CONVERSATION_HISTORY: usize = 1000;
pub const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_DOCUMENT_SIZE: u64 = 100 * 1024 * 1024; // 100MB
