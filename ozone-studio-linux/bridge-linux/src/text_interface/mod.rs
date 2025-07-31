// =============================================================================
// bridge-linux/src/text_interface/mod.rs
// Text Interface Module - Natural Language Processing and Conversation Management
// Core component for human-friendly text interaction with the OZONE STUDIO ecosystem
// =============================================================================

// The text_interface module serves as the foundational layer for all text-based human interaction
// with the OZONE STUDIO ecosystem. Unlike other AI Apps that communicate through structured protocols,
// BRIDGE must translate natural human language into ecosystem coordination requests and format
// complex ecosystem responses into human-readable text. This module bridges the gap between
// human communication patterns and the sophisticated coordination capabilities of the AGI.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use std::fmt;

// Async runtime for handling concurrent conversations and long-running text processing
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{sleep, timeout, interval, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Natural language processing dependencies for understanding human communication
use regex::Regex;
use chrono::{DateTime, Utc, Local, NaiveDateTime};

// Import shared ecosystem types for coordination with other AI Apps
use shared_protocols::{
    EcosystemIdentity,
    ComponentType,
    TaskOrchestrationRequest,
    ConsciousnessRequest,
    AIAppCoordinationRequest,
    AIAppCoordinationResponse,
    ExecutionStatus,
    ComplexityLevel,
    CoordinationStrategy,
    StrategicAlignment,
    HumanGuidance,
    HumanGuidanceType,
    AuthorityLevel,
    ProtocolError,
};

// Import security types for protecting user conversations and personal data
use shared_security::{
    SecurityError,
    SecureComponent,
    AuthenticationCredentials,
    SecurityConfig,
    UserCertificate,
    SessionManager,
    SecureSession,
};

// Text processing submodules - each handles a specific aspect of human text interaction
pub mod text_processor;           // Core text analysis and processing engine
pub mod natural_language_handler; // Human language understanding and intent recognition
pub mod command_interpreter;      // Command parsing for power users and automation
pub mod response_formatter;       // Human-friendly response formatting and presentation
pub mod conversation_manager;     // Multi-turn conversation state and context management
pub mod personalization_engine;   // User preference learning and adaptation
pub mod context_analyzer;         // Semantic context analysis for better understanding
pub mod intent_classifier;        // Classification of user intents and goals
pub mod response_optimizer;       // Response optimization for clarity and engagement

// Re-export core text interface types for use throughout BRIDGE
pub use text_processor::{
    TextProcessor,
    TextProcessingEngine,
    ProcessingContext,
    TextAnalysis,
    ProcessingMetrics,
    ProcessingConfiguration,
    ProcessingError,
};

pub use natural_language_handler::{
    NaturalLanguageHandler,
    LanguageProcessor,
    IntentRecognition,
    EntityExtraction,
    SentimentAnalysis,
    LanguageModel,
    ProcessingPipeline,
    LanguageProcessingError,
};

pub use command_interpreter::{
    CommandInterpreter,
    CommandParser,
    CommandValidator,
    CommandExecutor,
    CommandRegistry,
    CommandDefinition,
    CommandResult,
    CommandError,
};

pub use response_formatter::{
    ResponseFormatter,
    FormattingEngine,
    PresentationStyle,
    FormattingContext,
    ResponseTemplate,
    FormattingRule,
    FormattingError,
};

pub use conversation_manager::{
    ConversationManager,
    ConversationState,
    ConversationHistory,
    ConversationContext,
    ContextManager,
    SessionManager as ConversationSessionManager,
    ConversationError,
};

pub use personalization_engine::{
    PersonalizationEngine,
    UserProfile,
    PreferenceManager,
    AdaptationEngine,
    LearningSystem,
    PersonalizationContext,
    PersonalizationError,
};

pub use context_analyzer::{
    ContextAnalyzer,
    SemanticContext,
    ContextualInformation,
    ContextExtraction,
    ContextRelationship,
    ContextError,
};

pub use intent_classifier::{
    IntentClassifier,
    IntentCategory,
    IntentConfidence,
    IntentParameters,
    IntentModel,
    ClassificationError,
};

pub use response_optimizer::{
    ResponseOptimizer,
    OptimizationStrategy,
    OptimizationMetrics,
    OptimizationContext,
    OptimizationError,
};

// Core text interface request and response types
// These types define the API between BRIDGE's text interface and the rest of the ecosystem

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextProcessingRequest {
    // Unique identifier for tracking this processing request through the ecosystem
    pub request_id: String,
    
    // The actual human input text that needs to be processed and understood
    pub user_input: String,
    
    // Rich conversation context that helps understand the user's intent
    pub conversation_context: ConversationContext,
    
    // Processing options that control how the text should be analyzed and handled
    pub processing_options: ProcessingOptions,
    
    // Security context for ensuring proper authorization and audit trails
    pub security_context: Option<TextSecurityContext>,
    
    // Timestamp for tracking processing latency and conversation timing
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextProcessingResponse {
    // Matches the request_id to correlate responses with requests
    pub request_id: String,
    
    // The interpreted intent and structure extracted from the user's text
    pub interpreted_intent: InterpretedIntent,
    
    // Coordination requirements for fulfilling the user's request through the ecosystem
    pub ecosystem_coordination: EcosystemCoordination,
    
    // How the response should be prepared and formatted for the user
    pub response_preparation: ResponsePreparation,
    
    // Any errors or warnings that occurred during text processing
    pub processing_status: ProcessingStatus,
    
    // Metadata about the processing operation for monitoring and optimization
    pub processing_metadata: ProcessingMetadata,
}

// Conversation context captures the rich state needed for multi-turn conversations
// This is essential for maintaining natural human-like interactions across multiple exchanges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    // Unique identifier for this conversation session
    pub conversation_id: String,
    
    // Authenticated user identity for personalization and security
    pub user_id: String,
    
    // Complete history of the conversation for context-aware processing
    pub context_history: Vec<ConversationTurn>,
    
    // User-specific preferences that influence text processing and formatting
    pub user_preferences: UserPreferences,
    
    // Current conversation state (active, paused, completed, etc.)
    pub conversation_state: ConversationState,
    
    // Active topics and entities being discussed for context continuity
    pub active_context: HashMap<String, ContextualInformation>,
    
    // Ongoing ecosystem operations that might influence the conversation
    pub active_operations: Vec<ActiveOperation>,
}

// Each turn in a conversation includes both what was said and how it was processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub turn_id: String,
    pub speaker: Speaker,
    pub content: String,
    pub timestamp: SystemTime,
    pub intent: Option<InterpretedIntent>,
    pub processing_metadata: Option<ProcessingMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Speaker {
    Human(String),           // Human user with their user ID
    OZONEAGI,               // The OZONE STUDIO AGI consciousness
    System,                 // System messages and notifications
}

// User preferences control how text is processed and responses are formatted
// This enables personalized interaction patterns that adapt to individual users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    // Language and localization preferences
    pub language: String,
    pub locale: String,
    pub timezone: String,
    
    // Communication style preferences
    pub communication_style: CommunicationStyle,
    pub verbosity_level: VerbosityLevel,
    pub technical_level: TechnicalLevel,
    
    // Interface preferences
    pub response_format: ResponseFormat,
    pub notification_preferences: NotificationPreferences,
    
    // Privacy and security preferences
    pub data_retention: DataRetentionPreference,
    pub sharing_permissions: SharingPermissions,
    
    // Learning and adaptation preferences
    pub personalization_enabled: bool,
    pub learning_from_interactions: bool,
    
    // Accessibility preferences
    pub accessibility_features: Vec<AccessibilityFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationStyle {
    Formal,
    Professional,
    Casual,
    Friendly,
    Concise,
    Detailed,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerbosityLevel {
    Minimal,      // Brief, direct responses
    Concise,      // Clear and to-the-point
    Standard,     // Balanced explanation level
    Detailed,     // Comprehensive explanations
    Verbose,      // Extensive detail and context
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TechnicalLevel {
    Basic,        // Minimal technical jargon
    Standard,     // Common technical terms explained
    Advanced,     // Technical terms assumed to be known
    Expert,       // Full technical depth
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseFormat {
    PlainText,
    Markdown,
    StructuredText,
    Interactive,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub task_completion: bool,
    pub system_status: bool,
    pub error_alerts: bool,
    pub progress_updates: bool,
    pub learning_insights: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataRetentionPreference {
    Session,      // Keep data only for current session
    ShortTerm,    // Keep data for a few days
    Standard,     // Keep data for standard retention period
    LongTerm,     // Keep data for extended period
    Permanent,    // Keep data indefinitely
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingPermissions {
    pub ecosystem_learning: bool,
    pub anonymized_analytics: bool,
    pub improvement_feedback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessibilityFeature {
    ScreenReader,
    HighContrast,
    LargeText,
    SimplifiedLanguage,
    VoiceOutput,
    KeyboardNavigation,
}

// Processing options control how text is analyzed and what kind of processing is performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    // Enable natural language understanding for intent extraction
    pub natural_language_understanding: bool,
    
    // Enable command interpretation for structured commands
    pub command_interpretation: bool,
    
    // Enable context awareness for multi-turn conversations
    pub context_awareness: bool,
    
    // Enable response optimization for clarity and engagement
    pub response_optimization: bool,
    
    // Enable personalization based on user preferences and history
    pub personalization: bool,
    
    // Enable ecosystem coordination planning
    pub ecosystem_coordination: bool,
    
    // Processing priority level
    pub priority: ProcessingPriority,
    
    // Maximum processing time before timeout
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingPriority {
    Low,
    Normal,
    High,
    Urgent,
    Interactive,  // For real-time user interaction
}

// Security context for text processing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSecurityContext {
    pub user_identity: String,
    pub session_token: String,
    pub permissions: Vec<String>,
    pub audit_required: bool,
    pub data_classification: DataClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    Personal,
    Sensitive,
}

// Interpreted intent represents the system's understanding of what the user wants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpretedIntent {
    // Primary intent category (question, command, request, conversation, etc.)
    pub intent_type: IntentType,
    
    // Confidence score for the intent classification (0.0 to 1.0)
    pub confidence_score: f64,
    
    // Extracted parameters and entities from the user's input
    pub parameters: HashMap<String, IntentParameter>,
    
    // References to previous context that influenced this interpretation
    pub context_references: Vec<String>,
    
    // Sentiment analysis of the user's input
    pub sentiment: SentimentAnalysis,
    
    // Urgency or priority indicated by the user's language
    pub urgency: UrgencyLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentType {
    Question,              // User is asking for information
    Command,               // User wants to execute a specific action
    Request,               // User is requesting assistance with a task
    Conversation,          // User is engaging in conversational interaction
    TaskInterruption,      // User wants to interrupt or modify an ongoing task
    MethodologyCreation,   // User wants help creating a new methodology
    DocumentProcessing,    // User wants to process or analyze documents
    SystemInquiry,         // User is asking about system status or capabilities
    Feedback,              // User is providing feedback or corrections
    Clarification,         // User is seeking clarification on a previous response
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentParameter {
    pub parameter_type: ParameterType,
    pub value: String,
    pub confidence: f64,
    pub source_text: String,  // The part of the input text where this was extracted
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    Entity,       // Named entity (person, place, thing)
    Action,       // Action or verb
    Object,       // Object of action
    Modifier,     // Adjective or adverb
    Constraint,   // Limitation or requirement
    Context,      // Contextual information
    Preference,   // User preference
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,
    Normal,
    High,
    Critical,
    Emergency,
}

// Ecosystem coordination defines how the user's request should be handled by other AI Apps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemCoordination {
    // Whether this request requires orchestration by OZONE STUDIO
    pub requires_orchestration: bool,
    
    // Specific AI Apps that should be involved in handling this request
    pub target_ai_apps: Vec<ComponentType>,
    
    // Type of coordination required
    pub coordination_type: CoordinationType,
    
    // Estimated duration for completing the request
    pub estimated_duration: Option<Duration>,
    
    // Complexity level of the coordination required
    pub complexity_level: ComplexityLevel,
    
    // Strategic alignment for the request
    pub strategic_alignment: StrategicAlignment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationType {
    Direct,           // Direct communication with a single AI App
    Coordinated,      // Multi-AI App coordination through OZONE STUDIO
    Interactive,      // Ongoing interactive session
    Background,       // Background processing
    Streaming,        // Real-time streaming response
}

// Response preparation defines how the eventual response should be formatted for the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePreparation {
    // Type of response that will be generated
    pub response_type: ResponseType,
    
    // Formatting requirements for the response
    pub formatting_requirements: FormattingRequirements,
    
    // User experience optimization settings
    pub user_experience_optimization: UXOptimization,
    
    // Personalization settings to apply
    pub personalization_settings: PersonalizationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseType {
    Direct,           // Direct answer or response
    Coordinated,      // Response assembled from multiple AI Apps
    Interactive,      // Interactive response requiring user engagement
    Progressive,      // Progressive response that updates over time
    Streaming,        // Real-time streaming response
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingRequirements {
    pub format: ResponseFormat,
    pub structure: ResponseStructure,
    pub visual_elements: Vec<VisualElement>,
    pub accessibility_features: Vec<AccessibilityFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStructure {
    Linear,           // Sequential, linear response
    Hierarchical,     // Organized in sections and subsections
    Interactive,      // Interactive elements and controls
    Tabular,          // Table or list format
    Narrative,        // Story or explanation format
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisualElement {
    CodeBlock,
    Table,
    List,
    Diagram,
    Chart,
    Image,
    Link,
    Button,
    ProgressBar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UXOptimization {
    pub response_timing: ResponseTiming,
    pub clarity_optimization: bool,
    pub engagement_optimization: bool,
    pub cognitive_load_reduction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseTiming {
    Immediate,        // Respond immediately
    Batched,          // Batch multiple updates
    Progressive,      // Progressive disclosure
    OnDemand,         // Respond when ready
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizationSettings {
    pub apply_user_preferences: bool,
    pub adapt_communication_style: bool,
    pub include_context_references: bool,
    pub learning_integration: bool,
}

// Processing status indicates the outcome of text processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStatus {
    pub success: bool,
    pub confidence: f64,
    pub warnings: Vec<ProcessingWarning>,
    pub errors: Vec<ProcessingError>,
    pub suggestions: Vec<ProcessingSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingWarning {
    pub warning_type: WarningType,
    pub message: String,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningType {
    AmbiguousIntent,
    LowConfidence,
    MissingContext,
    SecurityConcern,
    PrivacyIssue,
    PerformanceImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingSuggestion {
    pub suggestion_type: SuggestionType,
    pub message: String,
    pub benefit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    Clarification,
    Alternative,
    Enhancement,
    Optimization,
    Learning,
}

// Processing metadata provides insight into how the text was processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetadata {
    pub processing_time: Duration,
    pub models_used: Vec<String>,
    pub confidence_scores: HashMap<String, f64>,
    pub resource_usage: ResourceUsage,
    pub quality_metrics: QualityMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time: Duration,
    pub memory_used: u64,
    pub api_calls_made: u32,
    pub tokens_processed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub accuracy_estimate: f64,
    pub completeness_score: f64,
    pub relevance_score: f64,
    pub user_satisfaction_prediction: f64,
}

// Active operations track ecosystem activities relevant to the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveOperation {
    pub operation_id: String,
    pub operation_type: String,
    pub status: ExecutionStatus,
    pub progress: f64,
    pub estimated_completion: Option<SystemTime>,
    pub description: String,
}

// Configuration for the text interface module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInterfaceConfig {
    // Core processing configuration
    pub text_processing: TextProcessingConfig,
    pub natural_language_handling: NaturalLanguageConfig,
    pub command_interpretation: CommandInterpretationConfig,
    pub response_formatting: ResponseFormattingConfig,
    pub conversation_management: ConversationManagementConfig,
    
    // Performance and resource configuration
    pub performance: PerformanceConfig,
    pub resource_limits: ResourceLimitsConfig,
    
    // Security and privacy configuration
    pub security: TextSecurityConfig,
    pub privacy: PrivacyConfig,
    
    // Integration configuration
    pub ecosystem_integration: EcosystemIntegrationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextProcessingConfig {
    pub max_input_length: usize,
    pub processing_timeout: Duration,
    pub confidence_threshold: f64,
    pub enable_caching: bool,
    pub parallel_processing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalLanguageConfig {
    pub language_models: Vec<String>,
    pub supported_languages: Vec<String>,
    pub intent_classification_threshold: f64,
    pub entity_extraction_enabled: bool,
    pub sentiment_analysis_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInterpretationConfig {
    pub command_prefix: String,
    pub strict_parsing: bool,
    pub auto_correction: bool,
    pub command_history: bool,
    pub custom_commands_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormattingConfig {
    pub default_format: ResponseFormat,
    pub max_response_length: usize,
    pub enable_rich_formatting: bool,
    pub accessibility_compliance: bool,
    pub localization_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationManagementConfig {
    pub max_conversation_length: usize,
    pub context_window_size: usize,
    pub session_timeout: Duration,
    pub context_preservation: bool,
    pub conversation_summarization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub max_concurrent_requests: usize,
    pub request_queue_size: usize,
    pub processing_workers: usize,
    pub cache_size: usize,
    pub optimization_level: OptimizationLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    Minimal,
    Balanced,
    Performance,
    Quality,
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsConfig {
    pub max_memory_usage: u64,
    pub max_cpu_usage: f64,
    pub max_processing_time: Duration,
    pub max_api_calls_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSecurityConfig {
    pub input_sanitization: bool,
    pub output_filtering: bool,
    pub audit_logging: bool,
    pub encryption_required: bool,
    pub access_control_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub data_anonymization: bool,
    pub retention_policy: DataRetentionPreference,
    pub sharing_restrictions: Vec<String>,
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyLevel {
    Public,
    Standard,
    Enhanced,
    Maximum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemIntegrationConfig {
    pub ozone_studio_endpoint: String,
    pub coordination_timeout: Duration,
    pub retry_attempts: u32,
    pub streaming_enabled: bool,
    pub background_processing: bool,
}

// Error types specific to text interface processing
#[derive(Error, Debug)]
pub enum TextInterfaceError {
    #[error("Processing error: {operation} - {details}")]
    ProcessingError { operation: String, details: String },
    
    #[error("Natural language understanding error: {details}")]
    NLUError { details: String },
    
    #[error("Command interpretation error: {command} - {details}")]
    CommandError { command: String, details: String },
    
    #[error("Response formatting error: {format} - {details}")]
    FormattingError { format: String, details: String },
    
    #[error("Conversation management error: {conversation_id} - {details}")]
    ConversationError { conversation_id: String, details: String },
    
    #[error("Personalization error: {user_id} - {details}")]
    PersonalizationError { user_id: String, details: String },
    
    #[error("Security violation: {details}")]
    SecurityViolation { details: String },
    
    #[error("Resource limit exceeded: {resource} - {limit}")]
    ResourceLimitExceeded { resource: String, limit: String },
    
    #[error("Integration error: {component} - {details}")]
    IntegrationError { component: String, details: String },
    
    #[error("Configuration error: {parameter} - {details}")]
    ConfigurationError { parameter: String, details: String },
}

// Core traits for text interface components
pub trait TextInterfaceComponent {
    type Config;
    type Input;
    type Output;
    type Error;
    
    fn initialize(config: Self::Config) -> Result<Self, Self::Error> where Self: Sized;
    fn process(&mut self, input: Self::Input) -> Result<Self::Output, Self::Error>;
    fn get_metrics(&self) -> ProcessingMetrics;
    fn shutdown(&mut self) -> Result<(), Self::Error>;
}

pub trait ConversationContextProvider {
    fn get_conversation_context(&self, conversation_id: &str) -> Result<ConversationContext, TextInterfaceError>;
    fn update_conversation_context(&mut self, context: ConversationContext) -> Result<(), TextInterfaceError>;
    fn create_new_conversation(&mut self, user_id: &str) -> Result<String, TextInterfaceError>;
    fn end_conversation(&mut self, conversation_id: &str) -> Result<(), TextInterfaceError>;
}

pub trait UserPreferenceProvider {
    fn get_user_preferences(&self, user_id: &str) -> Result<UserPreferences, TextInterfaceError>;
    fn update_user_preferences(&mut self, user_id: &str, preferences: UserPreferences) -> Result<(), TextInterfaceError>;
    fn learn_from_interaction(&mut self, user_id: &str, interaction: &TextProcessingRequest, feedback: Option<UserFeedback>) -> Result<(), TextInterfaceError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    pub feedback_type: FeedbackType,
    pub rating: f64,
    pub comment: Option<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    Positive,
    Negative,
    Neutral,
    Correction,
    Enhancement,
}

// Result type for text interface operations
pub type TextInterfaceResult<T> = Result<T, TextInterfaceError>;

// Constants for text interface configuration
pub const DEFAULT_MAX_INPUT_LENGTH: usize = 10000;
pub const DEFAULT_PROCESSING_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_CONFIDENCE_THRESHOLD: f64 = 0.7;
pub const DEFAULT_MAX_CONVERSATION_LENGTH: usize = 1000;
pub const DEFAULT_CONTEXT_WINDOW_SIZE: usize = 20;
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour

// Utility functions for text interface operations
pub fn create_default_processing_options() -> ProcessingOptions {
    ProcessingOptions {
        natural_language_understanding: true,
        command_interpretation: true,
        context_awareness: true,
        response_optimization: true,
        personalization: true,
        ecosystem_coordination: true,
        priority: ProcessingPriority::Normal,
        timeout: DEFAULT_PROCESSING_TIMEOUT,
    }
}

pub fn create_default_user_preferences() -> UserPreferences {
    UserPreferences {
        language: "en".to_string(),
        locale: "en_US".to_string(),
        timezone: "UTC".to_string(),
        communication_style: CommunicationStyle::Professional,
        verbosity_level: VerbosityLevel::Standard,
        technical_level: TechnicalLevel::Standard,
        response_format: ResponseFormat::Markdown,
        notification_preferences: NotificationPreferences {
            task_completion: true,
            system_status: true,
            error_alerts: true,
            progress_updates: true,
            learning_insights: false,
        },
        data_retention: DataRetentionPreference::Standard,
        sharing_permissions: SharingPermissions {
            ecosystem_learning: true,
            anonymized_analytics: true,
            improvement_feedback: true,
        },
        personalization_enabled: true,
        learning_from_interactions: true,
        accessibility_features: vec![],
    }
}

// Helper macros for text interface operations
#[macro_export]
macro_rules! validate_input_length {
    ($input:expr, $max_length:expr) => {
        if $input.len() > $max_length {
            return Err(TextInterfaceError::ProcessingError {
                operation: "input_validation".to_string(),
                details: format!("Input length {} exceeds maximum {}", $input.len(), $max_length),
            });
        }
    };
}

#[macro_export]
macro_rules! check_processing_timeout {
    ($start_time:expr, $timeout:expr) => {
        if $start_time.elapsed() > $timeout {
            return Err(TextInterfaceError::ProcessingError {
                operation: "timeout_check".to_string(),
                details: "Processing timeout exceeded".to_string(),
            });
        }
    };
}

// Version and compatibility information
pub const TEXT_INTERFACE_VERSION: &str = "1.0.0";
pub const SUPPORTED_PROTOCOL_VERSION: &str = "1.0.0";
pub const MIN_ECOSYSTEM_VERSION: &str = "1.0.0";
