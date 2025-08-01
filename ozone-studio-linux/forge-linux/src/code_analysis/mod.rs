// =============================================================================
// forge-linux/src/code_analysis/mod.rs
// FORGE Code Analysis Module - Sophisticated Code Understanding and Intelligence
// =============================================================================

//! # FORGE Code Analysis Module
//! 
//! This module provides sophisticated code analysis capabilities that enable FORGE
//! to understand, analyze, and work with codebases of unlimited complexity through
//! layered analysis approaches that build from syntax to architecture to patterns.
//! 
//! ## Architectural Philosophy
//! 
//! The code analysis system follows a "layered understanding" approach:
//! 1. **Syntax Analysis**: Understanding the basic structure and syntax of code
//! 2. **Semantic Analysis**: Understanding meaning, relationships, and data flow
//! 3. **Architecture Analysis**: Understanding high-level structure and design patterns
//! 4. **Quality Assessment**: Understanding code quality, maintainability, and technical debt
//! 5. **Pattern Recognition**: Understanding recurring patterns and architectural principles
//! 
//! This layered approach ensures that FORGE can provide comprehensive understanding
//! that goes far beyond simple syntax parsing, enabling sophisticated reasoning
//! about code structure, quality, and enhancement opportunities.

use std::collections::{HashMap, HashSet, BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, Instant};
use std::sync::Arc;
use std::fmt;

// Async runtime and concurrency for large codebase processing
use tokio::sync::{RwLock, Mutex, Semaphore};
use tokio::task::{spawn, JoinHandle};
use tokio::time::timeout;

// Serialization and data structures
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::{Result, Context, bail};
use thiserror::Error;

// Graph algorithms for dependency analysis
use petgraph::{Graph, Directed, Direction};
use petgraph::graph::{NodeIndex, EdgeIndex};
use petgraph::algo::{strongly_connected_components, toposort, dijkstra};

// Text processing and parsing
use regex::Regex;
use tree_sitter::{Parser, Language, Tree, Node, Query, QueryCursor};

// Statistical analysis for quality metrics
use statistical_rs::{mean, median, standard_deviation, percentile};

// Import shared ecosystem types
use shared_protocols::{
    ComponentType,
    ExecutionStatus,
    ProtocolError,
    ResourceRequirements,
    CoordinationStrategy,
};

use shared_security::{
    SecurityError,
    SecureComponent,
    SecurityConfig,
};

use methodology_runtime::{
    ExecutionContext,
    ValidationResult,
    MethodologyRuntimeError,
    Instruction,
    InstructionSet,
};

// Re-export all analysis components and types
pub use analysis_engine::{
    AnalysisEngine,
    AnalysisConfiguration,
    AnalysisRequest,
    AnalysisResult,
    AnalysisMetrics,
    AnalysisError,
};

pub use semantic_analyzer::{
    SemanticAnalyzer,
    SemanticModel,
    SemanticRelationship,
    DataFlowGraph,
    ControlFlowGraph,
    SemanticContext,
    SemanticAnalysisResult,
};

pub use dependency_mapper::{
    DependencyMapper,
    DependencyGraph,
    DependencyNode,
    DependencyEdge,
    DependencyType,
    DependencyAnalysis,
    CircularDependency,
    DependencyMetrics,
};

pub use architecture_analyzer::{
    ArchitectureAnalyzer,
    ArchitectureMap,
    ArchitecturalLayer,
    ArchitecturalPattern,
    ArchitecturalComponent,
    ArchitecturalRelationship,
    ArchitectureQuality,
    ArchitecturalInsight,
};

pub use quality_assessor::{
    QualityAssessor,
    QualityMetrics,
    QualityDimension,
    QualityScore,
    QualityReport,
    TechnicalDebt,
    QualityImprovement,
    QualityTrend,
};

pub use pattern_recognizer::{
    PatternRecognizer,
    CodePattern,
    PatternInstance,
    PatternCategory,
    PatternSignificance,
    PatternEvolution,
    DesignPatternRecognition,
    AntiPatternDetection,
};

// Core analysis modules - each represents a layer of understanding
pub mod analysis_engine;      // Orchestrates all analysis layers
pub mod semantic_analyzer;    // Understands meaning and relationships
pub mod dependency_mapper;    // Maps dependencies and relationships
pub mod architecture_analyzer; // Understands high-level architecture
pub mod quality_assessor;     // Assesses code quality and technical debt
pub mod pattern_recognizer;   // Recognizes patterns and anti-patterns

// Language-specific analysis support
pub mod language_support;     // Language-specific parsing and analysis
pub mod syntax_parsers;       // Tree-sitter based syntax parsing
pub mod language_models;      // Language-specific semantic models

// Analysis coordination and optimization
pub mod analysis_coordinator; // Coordinates parallel analysis
pub mod cache_manager;        // Caches analysis results for performance
pub mod progress_tracker;     // Tracks analysis progress and metrics

// =============================================================================
// Core Analysis Types and Structures
// =============================================================================

/// Comprehensive analysis of a codebase that combines all analysis layers
/// This is the primary result type that integrates understanding from all analysis components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseAnalysis {
    /// Unique identifier for this analysis session
    pub analysis_id: String,
    
    /// Timestamp when analysis was performed
    pub analysis_timestamp: SystemTime,
    
    /// Configuration used for this analysis
    pub analysis_configuration: AnalysisConfiguration,
    
    /// Basic codebase information and metadata
    pub codebase_info: CodebaseInfo,
    
    /// Semantic model representing meaning and relationships in the code
    pub semantic_model: SemanticModel,
    
    /// Dependency graph showing all relationships between components
    pub dependency_graph: DependencyGraph,
    
    /// Architecture map showing high-level structural organization
    pub architecture_map: ArchitectureMap,
    
    /// Quality metrics assessing various dimensions of code quality
    pub quality_metrics: QualityMetrics,
    
    /// Recognized patterns throughout the codebase
    pub recognized_patterns: Vec<CodePattern>,
    
    /// Analysis metrics showing performance and completeness
    pub analysis_metrics: AnalysisMetrics,
    
    /// Insights and recommendations derived from the analysis
    pub insights: Vec<AnalysisInsight>,
    
    /// Cross-domain enhancement opportunities identified
    pub enhancement_opportunities: Vec<EnhancementOpportunity>,
}

/// Basic information about the codebase being analyzed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseInfo {
    /// Root path of the codebase
    pub root_path: String,
    
    /// Primary programming language detected
    pub primary_language: ProgrammingLanguage,
    
    /// All programming languages found in the codebase
    pub languages: Vec<LanguageDistribution>,
    
    /// Total number of files analyzed
    pub total_files: usize,
    
    /// Total lines of code across all files
    pub total_lines_of_code: usize,
    
    /// Size of the codebase in bytes
    pub codebase_size_bytes: u64,
    
    /// Estimated complexity level of the codebase
    pub complexity_level: ComplexityLevel,
    
    /// Project type and characteristics
    pub project_characteristics: ProjectCharacteristics,
}

/// Distribution of programming languages in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDistribution {
    /// Programming language
    pub language: ProgrammingLanguage,
    
    /// Number of files in this language
    pub file_count: usize,
    
    /// Lines of code in this language
    pub lines_of_code: usize,
    
    /// Percentage of total codebase
    pub percentage: f64,
}

/// Programming languages supported by FORGE analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProgrammingLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Java,
    CPlusPlus,
    C,
    Go,
    Swift,
    Kotlin,
    CSharp,
    Ruby,
    PHP,
    Scala,
    Haskell,
    Clojure,
    Elixir,
    Erlang,
    Dart,
    Lua,
    R,
    Julia,
    Matlab,
    SQL,
    HTML,
    CSS,
    YAML,
    JSON,
    XML,
    Markdown,
    Shell,
    PowerShell,
    Dockerfile,
    Other(String),
}

/// Complexity levels for codebases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComplexityLevel {
    /// Simple scripts or small utilities
    Simple,
    
    /// Small to medium applications with basic structure
    Moderate,
    
    /// Medium to large applications with clear architecture
    Complex,
    
    /// Large applications with sophisticated architecture
    Sophisticated,
    
    /// Enterprise-level systems with multiple subsystems
    Enterprise,
    
    /// Massive distributed systems with unlimited complexity
    Unlimited,
}

/// Characteristics of the project based on analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCharacteristics {
    /// Type of project (web app, library, system tool, etc.)
    pub project_type: ProjectType,
    
    /// Architectural style observed in the project
    pub architectural_style: ArchitecturalStyle,
    
    /// Development maturity level
    pub maturity_level: MaturityLevel,
    
    /// Testing approach and coverage
    pub testing_approach: TestingApproach,
    
    /// Documentation quality and coverage
    pub documentation_quality: DocumentationQuality,
    
    /// Dependencies and external integrations
    pub dependency_characteristics: DependencyCharacteristics,
}

/// Types of software projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    WebApplication,
    MobileApplication,
    DesktopApplication,
    Library,
    Framework,
    SystemTool,
    Database,
    API,
    Microservice,
    EmbeddedSystem,
    GameEngine,
    OperatingSystem,
    Compiler,
    DevTool,
    DataPipeline,
    MachineLearning,
    Blockchain,
    IoT,
    Unknown,
}

/// Architectural styles recognized in projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchitecturalStyle {
    Monolithic,
    Layered,
    Microservices,
    EventDriven,
    ServiceOriented,
    ComponentBased,
    PipeAndFilter,
    ModelViewController,
    ClientServer,
    PeerToPeer,
    Hexagonal,
    Clean,
    Onion,
    Hybrid,
    Unknown,
}

/// Development maturity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaturityLevel {
    Prototype,
    Development,
    Alpha,
    Beta,
    Production,
    Mature,
    Legacy,
}

/// Testing approaches found in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingApproach {
    /// Presence of unit tests
    pub unit_tests: TestingLevel,
    
    /// Presence of integration tests
    pub integration_tests: TestingLevel,
    
    /// Presence of end-to-end tests
    pub e2e_tests: TestingLevel,
    
    /// Estimated test coverage percentage
    pub test_coverage: f64,
    
    /// Testing frameworks used
    pub testing_frameworks: Vec<String>,
}

/// Levels of testing implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestingLevel {
    None,
    Minimal,
    Basic,
    Comprehensive,
    Extensive,
}

/// Documentation quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationQuality {
    /// Presence of README files
    pub readme_quality: DocumentationLevel,
    
    /// Presence of API documentation
    pub api_documentation: DocumentationLevel,
    
    /// Presence of code comments
    pub code_comments: DocumentationLevel,
    
    /// Presence of architecture documentation
    pub architecture_docs: DocumentationLevel,
    
    /// Overall documentation completeness score
    pub completeness_score: f64,
}

/// Levels of documentation implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentationLevel {
    None,
    Minimal,
    Basic,
    Good,
    Excellent,
}

/// Characteristics of project dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCharacteristics {
    /// Total number of direct dependencies
    pub direct_dependencies: usize,
    
    /// Total number of transitive dependencies
    pub transitive_dependencies: usize,
    
    /// Dependency freshness score (how up-to-date)
    pub freshness_score: f64,
    
    /// Security vulnerability count in dependencies
    pub security_vulnerabilities: usize,
    
    /// License compatibility analysis
    pub license_compatibility: LicenseCompatibility,
    
    /// Major dependency frameworks identified
    pub major_frameworks: Vec<String>,
}

/// License compatibility assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LicenseCompatibility {
    Compatible,
    MostlyCompatible,
    SomeIssues,
    MajorIssues,
    Incompatible,
    Unknown,
}

/// Insights generated from comprehensive analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisInsight {
    /// Unique identifier for this insight
    pub insight_id: String,
    
    /// Category of insight
    pub insight_category: InsightCategory,
    
    /// Significance level of this insight
    pub significance: InsightSignificance,
    
    /// Human-readable description of the insight
    pub description: String,
    
    /// Specific location in code where this insight applies
    pub location: Option<CodeLocation>,
    
    /// Recommended actions based on this insight
    pub recommendations: Vec<String>,
    
    /// Potential impact of addressing this insight
    pub impact_assessment: ImpactAssessment,
    
    /// Supporting evidence for this insight
    pub supporting_evidence: Vec<String>,
}

/// Categories of insights that analysis can generate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightCategory {
    Architecture,
    Performance,
    Security,
    Maintainability,
    Scalability,
    Reliability,
    TestCoverage,
    Documentation,
    Dependencies,
    CodeQuality,
    DesignPatterns,
    AntiPatterns,
    CrossDomainOpportunity,
}

/// Significance levels for insights
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum InsightSignificance {
    Low,
    Medium,
    High,
    Critical,
}

/// Specific location in code for targeted insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    /// File path relative to project root
    pub file_path: String,
    
    /// Line number where insight applies
    pub line_number: Option<usize>,
    
    /// Column number for precise location
    pub column_number: Option<usize>,
    
    /// Range of lines for multi-line insights
    pub line_range: Option<(usize, usize)>,
    
    /// Function or method name where insight applies
    pub function_name: Option<String>,
    
    /// Class or module name where insight applies
    pub class_name: Option<String>,
}

/// Assessment of potential impact from addressing an insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    /// Effort required to address this insight
    pub effort_required: EffortLevel,
    
    /// Expected benefit from addressing this insight
    pub expected_benefit: BenefitLevel,
    
    /// Risk level of making recommended changes
    pub risk_level: RiskLevel,
    
    /// Priority score for addressing this insight
    pub priority_score: f64,
    
    /// Estimated time to implement recommendations
    pub estimated_implementation_time: Option<Duration>,
}

/// Effort levels for implementing recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffortLevel {
    Trivial,
    Low,
    Medium,
    High,
    Extensive,
}

/// Benefit levels from implementing recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenefitLevel {
    Minimal,
    Low,
    Medium,
    High,
    Transformative,
}

/// Risk levels for implementing changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Cross-domain enhancement opportunities identified during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementOpportunity {
    /// Unique identifier for this opportunity
    pub opportunity_id: String,
    
    /// Domain from which enhancement principles derive
    pub source_domain: EnhancementDomain,
    
    /// Specific principle or pattern to apply
    pub enhancement_principle: String,
    
    /// Description of how to apply this enhancement
    pub application_description: String,
    
    /// Location where enhancement can be applied
    pub target_location: CodeLocation,
    
    /// Expected benefits from this enhancement
    pub expected_benefits: Vec<String>,
    
    /// Implementation complexity assessment
    pub implementation_complexity: ComplexityLevel,
    
    /// Confidence score in this enhancement opportunity
    pub confidence_score: f64,
}

/// Domains that can provide enhancement principles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnhancementDomain {
    Biology,
    Mathematics,
    Physics,
    Psychology,
    Economics,
    SystemsTheory,
    NetworkTheory,
    GameTheory,
    InformationTheory,
    ControlTheory,
}

// =============================================================================
// Analysis Configuration and Request Types
// =============================================================================

/// Configuration for code analysis operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfiguration {
    /// Depth of analysis to perform
    pub analysis_depth: AnalysisDepth,
    
    /// Specific analysis types to include
    pub analysis_types: Vec<AnalysisType>,
    
    /// Languages to analyze (empty means all supported)
    pub target_languages: Vec<ProgrammingLanguage>,
    
    /// Maximum time to spend on analysis
    pub analysis_timeout: Option<Duration>,
    
    /// Whether to use parallel processing
    pub parallel_processing: bool,
    
    /// Maximum number of parallel analysis threads
    pub max_parallel_threads: usize,
    
    /// Whether to cache analysis results
    pub enable_caching: bool,
    
    /// Whether to include cross-domain enhancement analysis
    pub cross_domain_analysis: bool,
    
    /// Quality gates and thresholds
    pub quality_thresholds: QualityThresholds,
    
    /// Whether to generate detailed insights
    pub generate_insights: bool,
    
    /// Minimum significance level for insights
    pub minimum_insight_significance: InsightSignificance,
}

/// Depth levels for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisDepth {
    /// Quick syntax-level analysis only
    Surface,
    
    /// Syntax plus basic semantic analysis
    Standard,
    
    /// Include architecture and dependency analysis
    Deep,
    
    /// Include quality assessment and pattern recognition
    Comprehensive,
    
    /// Include cross-domain enhancement analysis
    Exhaustive,
}

/// Types of analysis that can be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisType {
    SyntaxAnalysis,
    SemanticAnalysis,
    DependencyMapping,
    ArchitectureAnalysis,
    QualityAssessment,
    PatternRecognition,
    SecurityAnalysis,
    PerformanceAnalysis,
    TestCoverageAnalysis,
    DocumentationAnalysis,
    CrossDomainEnhancement,
}

/// Quality thresholds for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityThresholds {
    /// Minimum acceptable maintainability score
    pub maintainability_threshold: f64,
    
    /// Minimum acceptable test coverage
    pub test_coverage_threshold: f64,
    
    /// Maximum acceptable cyclomatic complexity
    pub complexity_threshold: f64,
    
    /// Maximum acceptable technical debt ratio
    pub technical_debt_threshold: f64,
    
    /// Minimum acceptable documentation coverage
    pub documentation_threshold: f64,
}

/// Request for code analysis operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequest {
    /// Unique identifier for this analysis request
    pub request_id: String,
    
    /// Path to the codebase to analyze
    pub codebase_path: String,
    
    /// Configuration for this analysis
    pub configuration: AnalysisConfiguration,
    
    /// Context about why this analysis is being performed
    pub analysis_context: AnalysisContext,
    
    /// Security context for access control
    pub security_context: Option<SecurityContext>,
    
    /// Priority level for this analysis
    pub priority: AnalysisPriority,
}

/// Context about why analysis is being performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisContext {
    /// Component requesting the analysis
    pub requesting_component: ComponentType,
    
    /// Purpose of the analysis
    pub analysis_purpose: AnalysisPurpose,
    
    /// Methodology being executed (if applicable)
    pub methodology_context: Option<String>,
    
    /// Previous analysis results to compare against
    pub baseline_analysis: Option<String>,
    
    /// Additional context information
    pub additional_context: HashMap<String, String>,
}

/// Purpose of code analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisPurpose {
    InitialAssessment,
    QualityCheck,
    RefactoringPlanning,
    SecurityAudit,
    PerformanceOptimization,
    ArchitectureReview,
    TechnicalDebtAssessment,
    ComplianceCheck,
    EnhancementPlanning,
    ProgressTracking,
}

/// Security context for analysis operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Principal performing the analysis
    pub principal_id: String,
    
    /// Access permissions for this analysis
    pub permissions: Vec<String>,
    
    /// Whether to include sensitive code analysis
    pub include_sensitive_analysis: bool,
    
    /// Data retention policy for analysis results
    pub retention_policy: String,
}

/// Priority levels for analysis requests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnalysisPriority {
    Background,
    Low,
    Normal,
    High,
    Critical,
    Emergency,
}

// =============================================================================
// Analysis Result and Metrics Types
// =============================================================================

/// Result of a code analysis operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Request that generated this result
    pub request_id: String,
    
    /// Success status of the analysis
    pub success: bool,
    
    /// Comprehensive analysis data (if successful)
    pub analysis: Option<CodebaseAnalysis>,
    
    /// Error information (if failed)
    pub error: Option<AnalysisError>,
    
    /// Warnings generated during analysis
    pub warnings: Vec<AnalysisWarning>,
    
    /// Performance metrics for the analysis operation
    pub performance_metrics: AnalysisPerformanceMetrics,
    
    /// Recommendations for follow-up actions
    pub recommendations: Vec<AnalysisRecommendation>,
}

/// Warnings generated during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisWarning {
    /// Warning category
    pub category: WarningCategory,
    
    /// Warning message
    pub message: String,
    
    /// Location where warning occurred
    pub location: Option<CodeLocation>,
    
    /// Severity of the warning
    pub severity: WarningSeverity,
}

/// Categories of analysis warnings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningCategory {
    ParsingError,
    UnsupportedLanguage,
    IncompleteParsing,
    LargeFileSkipped,
    TimeoutReached,
    MemoryLimitReached,
    AccessDenied,
    CorruptedFile,
    UnknownFileType,
}

/// Severity levels for warnings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningSeverity {
    Info,
    Low,
    Medium,
    High,
}

/// Performance metrics for analysis operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPerformanceMetrics {
    /// Total time taken for analysis
    pub total_duration: Duration,
    
    /// Time spent on each analysis phase
    pub phase_durations: HashMap<String, Duration>,
    
    /// Number of files processed
    pub files_processed: usize,
    
    /// Number of files skipped
    pub files_skipped: usize,
    
    /// Lines of code analyzed
    pub lines_analyzed: usize,
    
    /// Peak memory usage during analysis
    pub peak_memory_usage: u64,
    
    /// Cache hit rate (if caching enabled)
    pub cache_hit_rate: Option<f64>,
    
    /// Parallel efficiency (if parallel processing used)
    pub parallel_efficiency: Option<f64>,
}

/// Recommendations for follow-up actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRecommendation {
    /// Type of recommendation
    pub recommendation_type: RecommendationType,
    
    /// Description of the recommendation
    pub description: String,
    
    /// Priority level for this recommendation
    pub priority: RecommendationPriority,
    
    /// Estimated effort to implement
    pub estimated_effort: EffortLevel,
    
    /// Expected benefits from implementation
    pub expected_benefits: Vec<String>,
    
    /// Specific actions to take
    pub action_items: Vec<String>,
}

/// Types of recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    ImmediateAction,
    QualityImprovement,
    ArchitecturalEnhancement,
    PerformanceOptimization,
    SecurityHardening,
    TestingEnhancement,
    DocumentationImprovement,
    DependencyUpdate,
    RefactoringOpportunity,
    CrossDomainEnhancement,
}

/// Priority levels for recommendations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecommendationPriority {
    Optional,
    Suggested,
    Recommended,
    Important,
    Critical,
}

// =============================================================================
// Error Types for Code Analysis
// =============================================================================

/// Errors that can occur during code analysis
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisError {
    #[error("Configuration error: {details}")]
    ConfigurationError { details: String },
    
    #[error("File system error: {path} - {details}")]
    FileSystemError { path: String, details: String },
    
    #[error("Parsing error: {file} at line {line} - {details}")]
    ParsingError { file: String, line: usize, details: String },
    
    #[error("Language not supported: {language}")]
    UnsupportedLanguage { language: String },
    
    #[error("Analysis timeout: {operation} exceeded {timeout:?}")]
    AnalysisTimeout { operation: String, timeout: Duration },
    
    #[error("Memory limit exceeded: {operation} used {memory_used} bytes")]
    MemoryLimitExceeded { operation: String, memory_used: u64 },
    
    #[error("Security violation: {operation} - {details}")]
    SecurityViolation { operation: String, details: String },
    
    #[error("Resource unavailable: {resource} - {details}")]
    ResourceUnavailable { resource: String, details: String },
    
    #[error("Analysis incomplete: {reason}")]
    IncompleteAnalysis { reason: String },
    
    #[error("Internal error: {component} - {details}")]
    InternalError { component: String, details: String },
}

// =============================================================================
// Core Analysis Metrics
// =============================================================================

/// Comprehensive metrics about the analysis operation itself
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetrics {
    /// Overall analysis completeness percentage
    pub completeness_percentage: f64,
    
    /// Confidence score in the analysis results
    pub confidence_score: f64,
    
    /// Number of insights generated
    pub insights_generated: usize,
    
    /// Number of enhancement opportunities identified
    pub enhancement_opportunities: usize,
    
    /// Coverage metrics for different analysis types
    pub coverage_metrics: HashMap<AnalysisType, f64>,
    
    /// Quality scores for different aspects
    pub quality_scores: HashMap<String, f64>,
    
    /// Performance characteristics of the analysis
    pub performance_characteristics: PerformanceCharacteristics,
    
    /// Resource utilization during analysis
    pub resource_utilization: ResourceUtilization,
}

/// Performance characteristics of the analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCharacteristics {
    /// Analysis speed (lines per second)
    pub analysis_speed: f64,
    
    /// Memory efficiency (bytes per line analyzed)
    pub memory_efficiency: f64,
    
    /// Cache effectiveness (if caching enabled)
    pub cache_effectiveness: Option<f64>,
    
    /// Parallel processing efficiency (if used)
    pub parallel_efficiency: Option<f64>,
    
    /// Bottleneck identification
    pub bottlenecks: Vec<String>,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// Peak CPU usage percentage
    pub peak_cpu_usage: f64,
    
    /// Peak memory usage in bytes
    pub peak_memory_usage: u64,
    
    /// Disk I/O operations performed
    pub disk_io_operations: u64,
    
    /// Network operations (if any)
    pub network_operations: u64,
    
    /// Thread utilization (if parallel processing)
    pub thread_utilization: Option<f64>,
}

// =============================================================================
// Default Implementations and Constants
// =============================================================================

impl Default for AnalysisConfiguration {
    fn default() -> Self {
        Self {
            analysis_depth: AnalysisDepth::Standard,
            analysis_types: vec![
                AnalysisType::SyntaxAnalysis,
                AnalysisType::SemanticAnalysis,
                AnalysisType::DependencyMapping,
                AnalysisType::QualityAssessment,
            ],
            target_languages: vec![], // Empty means all supported
            analysis_timeout: Some(Duration::from_secs(3600)), // 1 hour default
            parallel_processing: true,
            max_parallel_threads: num_cpus::get(),
            enable_caching: true,
            cross_domain_analysis: false,
            quality_thresholds: QualityThresholds::default(),
            generate_insights: true,
            minimum_insight_significance: InsightSignificance::Medium,
        }
    }
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            maintainability_threshold: 0.7,
            test_coverage_threshold: 0.8,
            complexity_threshold: 10.0,
            technical_debt_threshold: 0.2,
            documentation_threshold: 0.6,
        }
    }
}

// Constants for analysis operations
pub const MAX_FILE_SIZE_BYTES: u64 = 100 * 1024 * 1024; // 100MB per file
pub const MAX_ANALYSIS_MEMORY: u64 = 8 * 1024 * 1024 * 1024; // 8GB total
pub const DEFAULT_ANALYSIS_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour
pub const MIN_CONFIDENCE_THRESHOLD: f64 = 0.5;
pub const MAX_INSIGHTS_PER_FILE: usize = 50;
pub const CACHE_EXPIRY_DURATION: Duration = Duration::from_secs(86400); // 24 hours

// =============================================================================
// Utility Functions and Helpers
// =============================================================================

/// Utility function to determine complexity level based on codebase metrics
pub fn determine_complexity_level(
    total_files: usize,
    total_lines: usize,
    dependency_count: usize,
    language_count: usize,
) -> ComplexityLevel {
    let complexity_score = (total_files as f64).log10() 
        + (total_lines as f64).log10() 
        + (dependency_count as f64 / 10.0)
        + (language_count as f64 / 2.0);
    
    match complexity_score {
        s if s < 3.0 => ComplexityLevel::Simple,
        s if s < 4.5 => ComplexityLevel::Moderate,
        s if s < 6.0 => ComplexityLevel::Complex,
        s if s < 7.5 => ComplexityLevel::Sophisticated,
        s if s < 9.0 => ComplexityLevel::Enterprise,
        _ => ComplexityLevel::Unlimited,
    }
}

/// Utility function to calculate priority score for insights
pub fn calculate_insight_priority(
    significance: &InsightSignificance,
    impact: &ImpactAssessment,
    location_importance: f64,
) -> f64 {
    let significance_weight = match significance {
        InsightSignificance::Low => 1.0,
        InsightSignificance::Medium => 2.0,
        InsightSignificance::High => 3.0,
        InsightSignificance::Critical => 4.0,
    };
    
    let benefit_weight = match impact.expected_benefit {
        BenefitLevel::Minimal => 1.0,
        BenefitLevel::Low => 2.0,
        BenefitLevel::Medium => 3.0,
        BenefitLevel::High => 4.0,
        BenefitLevel::Transformative => 5.0,
    };
    
    let effort_penalty = match impact.effort_required {
        EffortLevel::Trivial => 1.0,
        EffortLevel::Low => 0.9,
        EffortLevel::Medium => 0.7,
        EffortLevel::High => 0.5,
        EffortLevel::Extensive => 0.3,
    };
    
    significance_weight * benefit_weight * effort_penalty * location_importance
}

/// Utility function to merge analysis results from multiple sources
pub fn merge_analysis_results(results: Vec<AnalysisResult>) -> Result<AnalysisResult> {
    if results.is_empty() {
        bail!("Cannot merge empty analysis results");
    }
    
    // Implementation would merge multiple analysis results
    // This is a placeholder for the complex merging logic
    Ok(results.into_iter().next().unwrap())
}

/// Utility function to validate analysis configuration
pub fn validate_analysis_configuration(config: &AnalysisConfiguration) -> Result<(), AnalysisError> {
    if config.max_parallel_threads == 0 {
        return Err(AnalysisError::ConfigurationError {
            details: "max_parallel_threads must be greater than 0".to_string(),
        });
    }
    
    if config.quality_thresholds.maintainability_threshold < 0.0 
        || config.quality_thresholds.maintainability_threshold > 1.0 {
        return Err(AnalysisError::ConfigurationError {
            details: "maintainability_threshold must be between 0.0 and 1.0".to_string(),
        });
    }
    
    // Additional validation logic would go here
    
    Ok(())
}

// =============================================================================
// Integration with FORGE Ecosystem
// =============================================================================

/// Integration trait for FORGE components that need to perform code analysis
pub trait CodeAnalysisIntegration {
    /// Perform code analysis with the given configuration
    fn analyze_code(&self, request: AnalysisRequest) -> Result<AnalysisResult, AnalysisError>;
    
    /// Get cached analysis results if available
    fn get_cached_analysis(&self, codebase_path: &str) -> Option<CodebaseAnalysis>;
    
    /// Validate that the component can perform the requested analysis
    fn can_perform_analysis(&self, config: &AnalysisConfiguration) -> bool;
    
    /// Get available analysis capabilities
    fn get_analysis_capabilities(&self) -> Vec<AnalysisType>;
}

/// Trait for components that can contribute to code analysis
pub trait AnalysisContributor {
    /// Contribute analysis data for a specific aspect
    fn contribute_analysis(&self, context: &AnalysisContext) -> Result<AnalysisContribution, AnalysisError>;
    
    /// Get the type of analysis this contributor provides
    fn get_contribution_type(&self) -> AnalysisType;
    
    /// Get the confidence level of this contributor's analysis
    fn get_confidence_level(&self) -> f64;
}

/// Contribution from an analysis component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisContribution {
    /// Type of analysis contributed
    pub analysis_type: AnalysisType,
    
    /// Confidence in this contribution
    pub confidence: f64,
    
    /// Analysis data contributed
    pub data: serde_json::Value,
    
    /// Insights generated by this contributor
    pub insights: Vec<AnalysisInsight>,
    
    /// Metrics from this contribution
    pub metrics: HashMap<String, f64>,
}
