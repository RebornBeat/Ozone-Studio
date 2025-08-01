use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use std::fmt;

// Async runtime and concurrency - keep minimal for primitive operations
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{sleep, timeout, interval, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Basic parsing dependencies - only for primitive operations
use regex::Regex;

// Import shared protocol and security types
use shared_protocols::{
    EcosystemIdentity,
    ComponentType,
    AIAppCoordinationRequest,
    AIAppCoordinationResponse,
    TaskOrchestrationRequest,
    ExecutionStatus,
    ProtocolError,
    CoordinationStrategy,
    DomainRequirement,
};

use shared_security::{
    SecurityError,
    SecureComponent,
    AuthenticationCredentials,
    SecurityConfig,
    SecureChannel,
};

use methodology_runtime::{
    MethodologyRuntime,
    Methodology,
    ExecutionResult,
    RuntimeConfiguration,
    MethodologyRuntimeError,
    InstructionExecutor,
    Instruction,
    ExecutionContext,
};

// PRIMITIVE OPERATIONS ONLY - Core minimal capabilities
pub mod primitive_operations;
pub mod static_core;

// System integration modules
pub mod interfaces;
pub mod api;
pub mod utils;
pub mod security;

// Re-export ONLY primitive operations - these are the basic reflexes FORGE needs
pub use primitive_operations::{
    // Basic code parsing capabilities - recognizes structure but doesn't deeply analyze
    BasicCodeParser,
    SyntaxValidator,
    FileStructureReader,
    CodeStructureDetector,
    LanguageDetector,
    
    // MINIMAL text output - only for basic coordination and status
    SimpleStatusReporter,      // "Analysis complete", "Error in file X"
    BasicCodeFormatter,        // Basic indentation, simple syntax highlighting
    CoordinationMessenger,     // Simple messages to OZONE STUDIO during coordination
    
    // Coordination primitives for ecosystem integration
    CoordinationHandler,
    StatusReporter,
    ErrorHandler,
    MethodologyReceiver,
    InstructionProcessor,
    
    // Basic types needed for primitive operations
    CodeFile,
    BasicParseResult,
    SimpleCodeStructure,
    CoordinationRequest,
    CoordinationResponse,
    PrimitiveCapabilityStatus,
    LanguageInfo,
    ParsingMetrics,
};

// The static core that manages methodology loading and primitive coordination
pub use static_core::{
    ForgeStaticCore,
    PrimitiveCapabilities,
    MethodologyLoader,
    InstructionInterpreter,
    EcosystemCoordinator,
    CapabilityManager,
};

// Interface exports for ecosystem coordination
pub use interfaces::{
    OzoneInterface,
    ZSEIInterface,
    SparkInterface,
    NexusInterface,
    ScribeInterface,
    InterfaceCoordination,
    CoordinationMetrics,
};

// Comprehensive programming language support for primitive parsing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProgrammingLanguage {
    // Systems programming languages
    Rust,
    CPlusPlus,
    C,
    Go,
    
    // Application development languages
    Python,
    JavaScript,
    TypeScript,
    Java,
    CSharp,
    
    // Mobile development languages
    Swift,
    Kotlin,
    Dart,
    
    // Functional programming languages
    Haskell,
    Clojure,
    Elixir,
    Erlang,
    Scala,
    
    // Scripting and dynamic languages
    Ruby,
    PHP,
    Lua,
    PowerShell,
    Shell,
    
    // Data science and mathematical languages
    R,
    Julia,
    Matlab,
    
    // Markup and data languages
    HTML,
    CSS,
    XML,
    JSON,
    YAML,
    Markdown,
    SQL,
    
    // Infrastructure and configuration
    Dockerfile,
    
    // Extensible for unknown languages
    Other(String),
}

impl ProgrammingLanguage {
    /// Detect programming language from file extension - primitive operation
    pub fn from_extension(extension: &str) -> Self {
        match extension.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "py" => Self::Python,
            "js" => Self::JavaScript,
            "ts" => Self::TypeScript,
            "java" => Self::Java,
            "cpp" | "cc" | "cxx" => Self::CPlusPlus,
            "c" => Self::C,
            "go" => Self::Go,
            "swift" => Self::Swift,
            "kt" | "kts" => Self::Kotlin,
            "cs" => Self::CSharp,
            "rb" => Self::Ruby,
            "php" => Self::PHP,
            "scala" | "sc" => Self::Scala,
            "hs" => Self::Haskell,
            "clj" | "cljs" => Self::Clojure,
            "ex" | "exs" => Self::Elixir,
            "erl" => Self::Erlang,
            "dart" => Self::Dart,
            "lua" => Self::Lua,
            "r" => Self::R,
            "jl" => Self::Julia,
            "m" => Self::Matlab,
            "sql" => Self::SQL,
            "html" | "htm" => Self::HTML,
            "css" => Self::CSS,
            "yaml" | "yml" => Self::YAML,
            "json" => Self::JSON,
            "xml" => Self::XML,
            "md" => Self::Markdown,
            "sh" | "bash" => Self::Shell,
            "ps1" => Self::PowerShell,
            _ if extension == "dockerfile" => Self::Dockerfile,
            _ => Self::Other(extension.to_string()),
        }
    }
    
    /// Get basic file extensions for this language - primitive operation
    pub fn typical_extensions(&self) -> Vec<&'static str> {
        match self {
            Self::Rust => vec!["rs"],
            Self::Python => vec!["py"],
            Self::JavaScript => vec!["js"],
            Self::TypeScript => vec!["ts"],
            Self::Java => vec!["java"],
            Self::CPlusPlus => vec!["cpp", "cc", "cxx"],
            Self::C => vec!["c"],
            Self::Go => vec!["go"],
            Self::Swift => vec!["swift"],
            Self::Kotlin => vec!["kt", "kts"],
            Self::CSharp => vec!["cs"],
            Self::Ruby => vec!["rb"],
            Self::PHP => vec!["php"],
            Self::Scala => vec!["scala", "sc"],
            Self::Haskell => vec!["hs"],
            Self::Clojure => vec!["clj", "cljs"],
            Self::Elixir => vec!["ex", "exs"],
            Self::Erlang => vec!["erl"],
            Self::Dart => vec!["dart"],
            Self::Lua => vec!["lua"],
            Self::R => vec!["r"],
            Self::Julia => vec!["jl"],
            Self::Matlab => vec!["m"],
            Self::SQL => vec!["sql"],
            Self::HTML => vec!["html", "htm"],
            Self::CSS => vec!["css"],
            Self::YAML => vec!["yaml", "yml"],
            Self::JSON => vec!["json"],
            Self::XML => vec!["xml"],
            Self::Markdown => vec!["md"],
            Self::Shell => vec!["sh", "bash"],
            Self::PowerShell => vec!["ps1"],
            Self::Dockerfile => vec!["dockerfile"],
            Self::Other(_) => vec![],
        }
    }
}

// Primitive operation errors - only for basic operations
#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("Basic parsing error: {0}")]
    ParsingError(String),
    
    #[error("Language detection error: {0}")]
    LanguageDetectionError(String),
    
    #[error("Coordination error: {0}")]
    CoordinationError(String),
    
    #[error("Methodology execution error: {0}")]
    MethodologyError(#[from] MethodologyRuntimeError),
    
    #[error("Security error: {0}")]
    SecurityError(#[from] SecurityError),
    
    #[error("Protocol error: {0}")]
    ProtocolError(#[from] ProtocolError),
    
    #[error("File operation error: {0}")]
    FileOperationError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Primitive capability error: {0}")]
    PrimitiveCapabilityError(String),
}

// Basic configuration for primitive operations only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeConfig {
    // Basic parsing configuration - what we can handle primitively
    pub basic_parsing: BasicParsingConfig,
    
    // Ecosystem integration configuration  
    pub ecosystem_integration: EcosystemIntegrationConfig,
    
    // Security configuration
    pub security: SecurityConfig,
    
    // Performance configuration for primitive operations
    pub performance: PrimitivePerformanceConfig,
    
    // Methodology runtime configuration
    pub methodology_runtime: RuntimeConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicParsingConfig {
    /// Languages supported for primitive parsing
    pub supported_languages: Vec<ProgrammingLanguage>,
    
    /// Maximum file size for primitive parsing (larger files need methodology coordination)
    pub max_file_size: u64,
    
    /// Timeout for primitive parsing operations
    pub timeout_duration: Duration,
    
    /// Whether to cache basic parsing results
    pub cache_parsing_results: bool,
    
    /// Maximum lines to parse for structure detection
    pub max_lines_for_structure: u32,
    
    /// Whether to validate syntax during basic parsing
    pub validate_syntax: bool,
}

impl Default for BasicParsingConfig {
    fn default() -> Self {
        Self {
            supported_languages: vec![
                ProgrammingLanguage::Rust,
                ProgrammingLanguage::Python,
                ProgrammingLanguage::JavaScript,
                ProgrammingLanguage::TypeScript,
                ProgrammingLanguage::Java,
                ProgrammingLanguage::Go,
                ProgrammingLanguage::CPlusPlus,
                ProgrammingLanguage::C,
            ],
            max_file_size: 1024 * 1024, // 1MB for primitive operations
            timeout_duration: Duration::from_secs(5),
            cache_parsing_results: true,
            max_lines_for_structure: 1000,
            validate_syntax: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemIntegrationConfig {
    /// OZONE STUDIO endpoint for orchestration coordination
    pub ozone_studio_endpoint: String,
    
    /// ZSEI endpoint for methodology coordination
    pub zsei_endpoint: String,
    
    /// SPARK endpoint for AI processing coordination
    pub spark_endpoint: String,
    
    /// NEXUS endpoint for file operations
    pub nexus_endpoint: String,
    
    /// SCRIBE endpoint for text processing coordination
    pub scribe_endpoint: String,
    
    /// Timeout for ecosystem coordination
    pub coordination_timeout: Duration,
    
    /// Health check interval
    pub health_check_interval: Duration,
    
    /// Whether to automatically register with OZONE STUDIO
    pub automatic_registration: bool,
    
    /// Component identity for ecosystem registration
    pub component_identity: EcosystemIdentity,
}

impl Default for EcosystemIntegrationConfig {
    fn default() -> Self {
        Self {
            ozone_studio_endpoint: "http://localhost:8080".to_string(),
            zsei_endpoint: "http://localhost:8081".to_string(),
            spark_endpoint: "http://localhost:8082".to_string(),
            nexus_endpoint: "http://localhost:8083".to_string(),
            scribe_endpoint: "http://localhost:8084".to_string(),
            coordination_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            automatic_registration: true,
            component_identity: EcosystemIdentity::new(
                "FORGE".to_string(),
                ComponentType::CodeFramework,
                "1.0.0".to_string(),
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitivePerformanceConfig {
    /// Maximum memory usage for primitive operations
    pub max_memory_usage: u64,
    
    /// Cache size for parsing results
    pub cache_size: u64,
    
    /// Worker threads for primitive operations
    pub worker_threads: usize,
    
    /// Request queue size
    pub request_queue_size: usize,
    
    /// Whether to enable performance monitoring
    pub enable_monitoring: bool,
}

impl Default for PrimitivePerformanceConfig {
    fn default() -> Self {
        Self {
            max_memory_usage: 512 * 1024 * 1024, // 512MB
            cache_size: 100 * 1024 * 1024, // 100MB
            worker_threads: 4,
            request_queue_size: 1000,
            enable_monitoring: true,
        }
    }
}

// Basic types for primitive operations only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicCodeAnalysisRequest {
    pub request_id: String,
    pub file_paths: Vec<String>,
    pub analysis_type: BasicAnalysisType,
    pub timeout: Duration,
    pub language_hint: Option<ProgrammingLanguage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BasicAnalysisType {
    /// Validate syntax only - primitive operation
    SyntaxValidation,
    
    /// Detect basic code structure - primitive operation
    StructureDetection,
    
    /// Basic parsing for file inventory - primitive operation
    BasicParsing,
    
    /// File inventory and language detection - primitive operation
    FileInventory,
    
    /// Language detection only - primitive operation
    LanguageDetection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicCodeAnalysisResult {
    pub request_id: String,
    pub results: Vec<BasicAnalysisResult>,
    pub processing_time: Duration,
    pub status: AnalysisStatus,
    pub primitive_operations_used: Vec<PrimitiveCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAnalysisResult {
    pub file_path: String,
    pub analysis_type: BasicAnalysisType,
    pub result: BasicResultData,
    pub errors: Vec<String>,
    pub language_detected: Option<ProgrammingLanguage>,
    pub processing_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BasicResultData {
    /// Simple syntax validation result
    SyntaxValid(bool),
    
    /// Basic code structure - not sophisticated analysis
    Structure(SimpleCodeStructure),
    
    /// Simple parse tree representation - basic only
    ParseTree(String),
    
    /// Basic file information
    FileInfo(BasicFileInfo),
    
    /// Language detection result
    Language(ProgrammingLanguage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleCodeStructure {
    /// Detected file type
    pub file_type: String,
    
    /// Detected programming language
    pub language: ProgrammingLanguage,
    
    /// Function names found (basic detection only)
    pub functions: Vec<String>,
    
    /// Class names found (basic detection only)
    pub classes: Vec<String>,
    
    /// Import statements found (basic detection only)
    pub imports: Vec<String>,
    
    /// Basic metrics
    pub line_count: u32,
    pub character_count: u64,
    pub estimated_complexity: BasicComplexity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BasicComplexity {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicFileInfo {
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub last_modified: SystemTime,
    pub file_type: String,
    pub language: Option<ProgrammingLanguage>,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisStatus {
    Success,
    PartialSuccess,
    Failed,
    Timeout,
    UnsupportedLanguage,
    FileTooLarge,
}

// Methodology execution request - how sophisticated capabilities are invoked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyExecutionRequest {
    pub methodology_id: String,
    pub execution_context: ExecutionContext,
    pub parameters: HashMap<String, serde_json::Value>,
    pub coordination_requirements: CoordinationRequirements,
    pub requested_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationRequirements {
    /// Whether ZSEI coordination is needed for intelligence
    pub requires_zsei_coordination: bool,
    
    /// Whether SPARK processing is needed for AI capabilities
    pub requires_spark_processing: bool,
    
    /// Whether NEXUS file operations are needed
    pub requires_nexus_file_operations: bool,
    
    /// Whether SCRIBE documentation coordination is needed
    pub requires_scribe_documentation: bool,
    
    /// Whether OZONE STUDIO orchestration is required
    pub requires_ozone_studio_orchestration: bool,
}

// Basic result types for primitive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicOperationResult {
    pub operation_id: String,
    pub operation_type: String,
    pub status: OperationStatus,
    pub result_data: Option<serde_json::Value>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub processing_time: Duration,
    pub primitive_capabilities_used: Vec<PrimitiveCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    Success,
    Failed,
    InProgress,
    Cancelled,
    RequiresMethodology, // When primitive operation is insufficient
}

// Simple coordination types for ecosystem integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationState {
    pub component_id: String,
    pub status: ComponentStatus,
    pub capabilities: Vec<PrimitiveCapability>,
    pub active_methodologies: Vec<String>,
    pub last_heartbeat: SystemTime,
    pub memory_usage: u64,
    pub cpu_usage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentStatus {
    Initializing,
    Ready,
    Processing,
    WaitingForCoordination,
    Error,
    Shutdown,
}

// Primitive capabilities that FORGE's static core provides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimitiveCapability {
    /// Basic code parsing - recognizes structure but doesn't deeply analyze
    BasicCodeParsing,
    
    /// Syntax validation - checks if code is syntactically correct
    SyntaxValidation,
    
    /// File reading through NEXUS coordination
    FileReading,
    
    /// Language detection from file extensions and basic content analysis
    LanguageDetection,
    
    /// Basic code structure detection (functions, classes, imports)
    StructureDetection,
    
    /// Simple status reporting - basic status messages only
    SimpleStatusReporting,
    
    /// Basic code formatting - indentation and simple syntax highlighting
    BasicCodeFormatting,
    
    /// Coordination messaging - simple messages to other ecosystem components
    CoordinationMessaging,
    
    /// Ecosystem coordination - ability to coordinate with other AI Apps
    EcosystemCoordination,
    
    /// Methodology execution - ability to load and execute methodologies
    MethodologyExecution,
}

// Language information for primitive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub language: ProgrammingLanguage,
    pub confidence: f32, // 0.0 to 1.0
    pub detection_method: DetectionMethod,
    pub supported_operations: Vec<PrimitiveCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    FileExtension,
    ContentAnalysis,
    Heuristic,
    UserProvided,
}

// Parsing metrics for performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsingMetrics {
    pub files_processed: u32,
    pub total_lines_processed: u32,
    pub total_processing_time: Duration,
    pub average_processing_time_per_file: Duration,
    pub cache_hit_rate: f32,
    pub error_rate: f32,
}

// Re-export essential types for ecosystem coordination
pub type ForgeResult<T> = Result<T, ForgeError>;

// Main FORGE static core initialization - entry point for the AI App
pub async fn initialize_forge_static_core(config: ForgeConfig) -> ForgeResult<ForgeStaticCore> {
    // Initialize the static core with primitive capabilities only
    ForgeStaticCore::initialize(config).await
}

// Helper function to create default configuration
pub fn create_default_config() -> ForgeConfig {
    ForgeConfig {
        basic_parsing: BasicParsingConfig::default(),
        ecosystem_integration: EcosystemIntegrationConfig::default(),
        security: SecurityConfig::default(),
        performance: PrimitivePerformanceConfig::default(),
        methodology_runtime: RuntimeConfiguration::default(),
    }
}

// Helper function for language detection - primitive operation
pub fn detect_language_from_path(file_path: &Path) -> Option<ProgrammingLanguage> {
    file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(ProgrammingLanguage::from_extension)
}

// Helper function to check if language is supported for primitive operations
pub fn is_language_supported_for_primitives(language: &ProgrammingLanguage) -> bool {
    match language {
        ProgrammingLanguage::Other(_) => false,
        _ => true, // All defined languages have basic primitive support
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(
            ProgrammingLanguage::from_extension("rs"),
            ProgrammingLanguage::Rust
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("py"),
            ProgrammingLanguage::Python
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("unknown"),
            ProgrammingLanguage::Other("unknown".to_string())
        );
    }

    #[test]
    fn test_language_extensions() {
        let rust_extensions = ProgrammingLanguage::Rust.typical_extensions();
        assert!(rust_extensions.contains(&"rs"));
        
        let cpp_extensions = ProgrammingLanguage::CPlusPlus.typical_extensions();
        assert!(cpp_extensions.contains(&"cpp"));
        assert!(cpp_extensions.contains(&"cc"));
    }

    #[test]
    fn test_primitive_capability_enumeration() {
        // Ensure we have appropriate primitive capabilities defined
        let capabilities = vec![
            PrimitiveCapability::BasicCodeParsing,
            PrimitiveCapability::SyntaxValidation,
            PrimitiveCapability::LanguageDetection,
            PrimitiveCapability::EcosystemCoordination,
        ];
        
        assert_eq!(capabilities.len(), 4);
    }
}
