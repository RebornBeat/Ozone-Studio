// =============================================================================
// zsei-linux/src/intelligent_storage/mod.rs
// ZSEI Intelligent Storage - Transforming Data into Relationship-Aware Understanding
// =============================================================================

use std::collections::{HashMap, HashSet, BTreeMap};
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use std::fmt;
use std::path::{Path, PathBuf};

// Async runtime and concurrency
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{sleep, timeout, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Mathematical and analysis dependencies
use ndarray::{Array1, Array2, ArrayD};
use nalgebra::{DVector, DMatrix};
use petgraph::{Graph, Directed, NodeIndex, EdgeIndex};
use petgraph::graph::UnGraph;

// Import shared protocol and security types
use shared_protocols::{
    ComponentType,
    ProtocolError,
};

use shared_security::{
    SecurityError,
    SecurityConfig,
    SecurityContext,
};

// Import NEXUS coordination types - ZSEI coordinates with NEXUS but never does file I/O directly
use nexus_linux::{
    NexusResult,
    FileOperationRequest,
    FileOperationResponse,
    StorageOperationRequest,
    StorageOperationResponse,
    FileOperationType,
    DirectoryEntry,
    FileMetadata,
    SecurityLevel,
};

// Submodules that implement specific aspects of intelligent storage
pub mod storage_coordinator;
pub mod intelligence_analyzer;
pub mod relationship_manager;
pub mod content_coordinator;
pub mod conversion_manager;
pub mod preservation_engine;

// Re-export all the intelligent storage components
pub use storage_coordinator::{
    StorageCoordinator,
    StorageCoordinationStrategy,
    IntelligentStorageRequest,
    StorageCoordinationResult,
    CoordinationContext,
    StorageOptimization,
};

pub use intelligence_analyzer::{
    IntelligenceAnalyzer,
    AnalysisEngine,
    SemanticAnalysis,
    ConceptualAnalysis,
    PatternAnalysis,
    IntelligenceResult,
    AnalysisConfiguration,
};

pub use relationship_manager::{
    RelationshipManager,
    RelationshipDetector,
    RelationshipClassifier,
    RelationshipValidator,
    RelationshipOptimizer,
    RelationshipMetrics,
};

pub use content_coordinator::{
    ContentCoordinator,
    ContentClassifier,
    ContentOrganizer,
    ContentEnhancer,
    ContentValidator,
    CoordinationMetrics,
};

pub use conversion_manager::{
    ConversionManager,
    DataConverter,
    MetadataGenerator,
    StructureConverter,
    ConversionValidator,
    ConversionMetrics,
};

pub use preservation_engine::{
    PreservationEngine,
    IntegrityPreserver,
    VersionManager,
    AccessibilityMaintainer,
    PreservationValidator,
    PreservationMetrics,
};

// =============================================================================
// CORE INTELLIGENT STORAGE TYPES
// =============================================================================

/// The primary configuration for ZSEI's intelligent storage capabilities
/// This defines how ZSEI transforms ordinary data into intelligent, relationship-aware storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentStorageConfig {
    /// Strategy for coordinating intelligent storage creation
    pub storage_strategy: StorageStrategy,
    
    /// Depth of metadata generation - determines how rich the intelligence layer becomes
    pub metadata_level: MetadataLevel,
    
    /// Types of relationships to detect and preserve
    pub relationship_detection: RelationshipDetectionConfig,
    
    /// How content should be analyzed for intelligence extraction
    pub analysis_configuration: AnalysisConfiguration,
    
    /// Content classification rules for organizing intelligent storage
    pub content_classification: ContentClassificationConfig,
    
    /// How to coordinate with NEXUS for actual storage operations
    pub nexus_coordination: NexusCoordinationConfig,
    
    /// Performance and optimization settings
    pub performance_optimization: PerformanceOptimizationConfig,
}

/// Strategies for organizing and creating intelligent storage
/// Each strategy represents a different approach to transforming data into intelligence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageStrategy {
    /// Comprehensive analysis - maximum intelligence extraction, suitable for critical data
    Comprehensive {
        analysis_depth: AnalysisDepth,
        relationship_mapping: bool,
        cross_domain_insights: bool,
        pattern_recognition: bool,
    },
    
    /// Focused analysis - targeted intelligence extraction for specific purposes
    Focused {
        focus_areas: Vec<AnalysisFocus>,
        priority_relationships: Vec<RelationshipType>,
        domain_specific: bool,
    },
    
    /// Adaptive analysis - automatically determines optimal intelligence level based on content
    Adaptive {
        complexity_threshold: f64,
        relationship_density_threshold: f64,
        auto_upgrade_enabled: bool,
    },
    
    /// Incremental analysis - builds intelligence gradually over time through usage patterns
    Incremental {
        initial_level: MetadataLevel,
        learning_enabled: bool,
        usage_tracking: bool,
    },
}

/// The depth of intelligent metadata generation
/// Deeper levels provide richer understanding but require more computational resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetadataLevel {
    /// Basic semantic understanding and simple relationships
    Basic,
    
    /// Standard analysis with relationship mapping and pattern detection  
    Standard,
    
    /// Rich analysis including cross-domain insights and complex relationships
    Rich,
    
    /// Comprehensive analysis with deep semantic understanding and wisdom integration
    Comprehensive,
    
    /// Exhaustive analysis including predictive insights and emergent pattern discovery
    Exhaustive,
}

/// Configuration for relationship detection and mapping
/// This determines what kinds of connections ZSEI discovers between different pieces of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipDetectionConfig {
    /// Types of relationships to actively detect
    pub relationship_types: Vec<RelationshipType>,
    
    /// Minimum confidence threshold for relationship detection
    pub confidence_threshold: f64,
    
    /// Whether to detect implicit relationships (not explicitly stated)
    pub implicit_relationships: bool,
    
    /// Whether to perform cross-content relationship analysis
    pub cross_content_analysis: bool,
    
    /// Whether to detect temporal relationships (how things change over time)
    pub temporal_relationships: bool,
    
    /// Whether to identify hierarchical relationships (parent/child, category/instance)
    pub hierarchical_relationships: bool,
}

/// Different types of relationships that ZSEI can detect and preserve
/// Each type represents a different way that pieces of content can be connected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Semantic relationships - content that shares meaning or concepts
    Semantic {
        similarity_threshold: f64,
        concept_depth: ConceptDepth,
    },
    
    /// Structural relationships - content that shares organizational patterns
    Structural {
        pattern_type: StructuralPattern,
        complexity_level: ComplexityLevel,
    },
    
    /// Functional relationships - content that serves similar or complementary functions
    Functional {
        function_category: FunctionCategory,
        complementarity: bool,
    },
    
    /// Temporal relationships - content that relates through time or sequence
    Temporal {
        sequence_type: SequenceType,
        temporal_distance: Option<Duration>,
    },
    
    /// Causal relationships - content where one influences or causes another
    Causal {
        causality_strength: CausalityStrength,
        directness: CausalDirectness,
    },
    
    /// Hierarchical relationships - parent/child, category/instance relationships
    Hierarchical {
        hierarchy_type: HierarchyType,
        depth_level: u32,
    },
    
    /// Cross-domain relationships - connections across different knowledge domains
    CrossDomain {
        source_domain: String,
        target_domain: String,
        bridge_strength: f64,
    },
}

/// How deep to analyze concepts for semantic relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConceptDepth {
    Surface,      // Direct word/phrase matching
    Shallow,      // Synonyms and closely related terms
    Moderate,     // Related concepts and themes
    Deep,         // Abstract conceptual connections
    Profound,     // Philosophical and principle-based connections
}

/// Types of structural patterns ZSEI can recognize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructuralPattern {
    Organizational,   // How content is organized and structured
    Architectural,    // System and component relationships
    Compositional,    // Part-whole relationships
    Sequential,       // Order and flow patterns
    Categorical,      // Classification and grouping patterns
    Network,          // Network and graph-like patterns
}

/// Complexity levels for relationship analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Simple,       // Direct, obvious relationships
    Moderate,     // Relationships requiring some analysis
    Complex,      // Multi-step or indirect relationships
    Sophisticated,// Requires domain expertise to understand
    Transcendent, // Emergent relationships beyond simple analysis
}

/// Categories of functional relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionCategory {
    Processing,   // Content that processes or transforms information
    Storage,      // Content that stores or maintains information
    Interface,    // Content that facilitates interaction or communication
    Control,      // Content that manages or coordinates other content
    Analysis,     // Content that examines or evaluates other content
    Creation,     // Content that generates or produces new content
}

/// Types of temporal sequences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SequenceType {
    Chronological,  // Time-based sequence
    Logical,        // Logical progression or dependency
    Procedural,     // Step-by-step process
    Developmental,  // Growth or evolution sequence
    Causal,         // Cause-and-effect sequence
}

/// Strength of causal relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CausalityStrength {
    Weak,      // Possible influence
    Moderate,  // Likely influence
    Strong,    // Clear influence
    Definitive,// Certain causal relationship
}

/// How direct causal relationships are
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CausalDirectness {
    Direct,     // Immediate cause-effect
    Indirect,   // Mediated through other factors
    Complex,    // Multiple causal pathways
}

/// Types of hierarchical relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HierarchyType {
    Taxonomic,    // Classification hierarchies
    Compositional,// Part-whole hierarchies
    Authority,    // Command or control hierarchies
    Abstraction,  // Abstract-concrete hierarchies
    Dependency,   // Dependency hierarchies
}

// =============================================================================
// INTELLIGENT METADATA STRUCTURES
// =============================================================================

/// The core intelligent metadata structure that ZSEI creates for any piece of content
/// This metadata transforms ordinary storage into relationship-aware intelligence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentMetadata {
    /// Unique identifier for this metadata instance
    pub metadata_id: String,
    
    /// What content this metadata describes
    pub content_reference: ContentReference,
    
    /// When this metadata was created and last updated
    pub temporal_info: TemporalInfo,
    
    /// Semantic analysis of the content - what it means and represents
    pub semantic_analysis: SemanticAnalysis,
    
    /// Structural analysis of the content - how it's organized and constructed
    pub structural_analysis: StructuralAnalysis,
    
    /// Functional analysis of the content - what it does and how it's used
    pub functional_analysis: FunctionalAnalysis,
    
    /// All relationships this content has with other content
    pub relationship_graph: RelationshipGraph,
    
    /// Cross-domain insights discovered about this content
    pub cross_domain_insights: Vec<CrossDomainInsight>,
    
    /// Patterns recognized within or related to this content
    pub recognized_patterns: Vec<RecognizedPattern>,
    
    /// Usage analytics and learning data about this content
    pub usage_analytics: UsageAnalytics,
    
    /// Quality metrics for this intelligent metadata
    pub quality_metrics: MetadataQualityMetrics,
}

/// Reference to the content that this metadata describes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentReference {
    /// Type of content being described
    pub content_type: ContentType,
    
    /// Path or location of the content (managed by NEXUS)
    pub content_path: String,
    
    /// Hash of the content for integrity verification
    pub content_hash: String,
    
    /// Size of the content in bytes
    pub content_size: u64,
    
    /// MIME type or format information
    pub format_info: FormatInfo,
    
    /// Additional identifying information
    pub identifiers: HashMap<String, String>,
}

/// Different types of content that ZSEI can analyze
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    /// Code files - programming languages, scripts, configuration
    Code {
        language: String,
        complexity_level: ComplexityLevel,
        architectural_role: ArchitecturalRole,
    },
    
    /// Documentation - text files, manuals, specifications
    Documentation {
        document_type: DocumentType,
        target_audience: AudienceType,
        technical_level: TechnicalLevel,
    },
    
    /// Data files - JSON, CSV, databases, structured data
    Data {
        data_format: DataFormat,
        structure_type: StructureType,
        data_classification: DataClassification,
    },
    
    /// Configuration files - settings, preferences, system configuration
    Configuration {
        config_scope: ConfigurationScope,
        config_complexity: ComplexityLevel,
    },
    
    /// Binary files - executables, images, compiled artifacts
    Binary {
        binary_type: BinaryType,
        architecture: Option<String>,
    },
    
    /// Methodology files - ZSEI methodologies and frameworks
    Methodology {
        methodology_category: String,
        difficulty_level: String,
        target_components: Vec<ComponentType>,
    },
    
    /// Ecosystem memory - experiences, relationships, accumulated wisdom
    EcosystemMemory {
        memory_type: MemoryType,
        significance_level: SignificanceLevel,
    },
}

/// Role that code plays in overall system architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchitecturalRole {
    Core,          // Core system functionality
    Interface,     // User or system interfaces
    Coordination,  // Component coordination logic
    Processing,    // Data or logic processing
    Storage,       // Data storage and persistence
    Communication, // Inter-component communication
    Security,      // Security and authentication
    Utility,       // Helper functions and utilities
}

/// Types of documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentType {
    UserManual,
    TechnicalSpecification,
    APIDocumentation,
    ArchitecturalDocument,
    Tutorial,
    Reference,
    Troubleshooting,
    DesignDocument,
}

/// Target audience for documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudienceType {
    EndUser,
    Developer,
    SystemAdministrator,
    Architect,
    Manager,
    General,
}

/// Technical complexity level of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TechnicalLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
    Specialist,
}

/// Data formats that ZSEI can analyze
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataFormat {
    JSON,
    XML,
    CSV,
    YAML,
    TOML,
    SQL,
    Binary,
    Custom(String),
}

/// Structure types for data organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructureType {
    Flat,         // Simple key-value or list structure
    Hierarchical, // Tree-like structure
    Relational,   // Database-like relationships
    Graph,        // Network of interconnected data
    Temporal,     // Time-series or sequential data
    Hybrid,       // Combination of multiple structure types
}

/// Classification of data sensitivity and importance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    Critical,
    Personal,
    Operational,
}

/// Scope of configuration files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigurationScope {
    Global,      // System-wide configuration
    Component,   // Single component configuration
    User,        // User-specific configuration
    Environment, // Environment-specific (dev, test, prod)
    Security,    // Security-related configuration
}

/// Types of binary content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryType {
    Executable,
    Library,
    Archive,
    Image,
    Audio,
    Video,
    Model,      // AI models, data models
    Compiled,   // Compiled code artifacts
}

/// Types of ecosystem memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Experience,     // Accumulated experiences and interactions
    Relationship,   // Relationship development and history
    Learning,       // Learning outcomes and discoveries
    Wisdom,         // Accumulated wisdom and insights
    Pattern,        // Recognized patterns and their effectiveness
    Evolution,      // How the ecosystem has evolved over time
}

/// Significance level of memories and experiences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignificanceLevel {
    Routine,        // Normal, everyday occurrences
    Notable,        // Worth remembering but not transformative
    Significant,    // Important experiences that shape understanding
    Transformative, // Experiences that fundamentally change perspective
    Core,           // Experiences that define identity and purpose
}

/// Temporal information about content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalInfo {
    /// When the content was originally created
    pub created_at: SystemTime,
    
    /// When the content was last modified
    pub modified_at: SystemTime,
    
    /// When ZSEI first analyzed this content
    pub first_analyzed: SystemTime,
    
    /// When this metadata was last updated
    pub metadata_updated: SystemTime,
    
    /// How frequently this content is accessed
    pub access_frequency: AccessFrequency,
    
    /// Temporal patterns in content usage
    pub usage_patterns: Vec<UsagePattern>,
}

/// How frequently content is accessed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessFrequency {
    Continuous,  // Accessed constantly
    Daily,       // Accessed multiple times per day
    Weekly,      // Accessed several times per week
    Monthly,     // Accessed several times per month
    Occasional,  // Accessed infrequently
    Archived,    // Rarely or never accessed
}

/// Patterns in how content is used over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    /// Type of usage pattern detected
    pub pattern_type: UsagePatternType,
    
    /// Confidence in this pattern detection
    pub confidence: f64,
    
    /// When this pattern was observed
    pub observation_period: (SystemTime, SystemTime),
    
    /// Contexts in which this pattern appears
    pub contexts: Vec<String>,
}

/// Types of usage patterns ZSEI can recognize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsagePatternType {
    Cyclical,     // Regular, repeating usage
    Seasonal,     // Usage tied to specific times or events
    Trending,     // Increasing usage over time
    Declining,    // Decreasing usage over time
    Burst,        // Sudden spikes in usage
    Collaborative,// Usage involving multiple users or components
}

// =============================================================================
// SEMANTIC ANALYSIS STRUCTURES
// =============================================================================

/// Comprehensive semantic analysis of content meaning and concepts
/// This represents ZSEI's understanding of what content means, not just what it contains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAnalysis {
    /// Primary concepts identified in the content
    pub primary_concepts: Vec<Concept>,
    
    /// Secondary or supporting concepts
    pub secondary_concepts: Vec<Concept>,
    
    /// Themes and topics that emerge from the content
    pub thematic_analysis: ThematicAnalysis,
    
    /// Intent or purpose of the content
    pub intent_analysis: IntentAnalysis,
    
    /// Emotional or subjective aspects of the content
    pub emotional_analysis: EmotionalAnalysis,
    
    /// Abstract qualities and characteristics
    pub abstract_qualities: Vec<AbstractQuality>,
    
    /// How this content relates to broader knowledge domains
    pub domain_connections: Vec<DomainConnection>,
}

/// A concept identified within content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    /// Name or label for this concept
    pub concept_name: String,
    
    /// Confidence that this concept is present
    pub confidence: f64,
    
    /// How central this concept is to the content
    pub centrality: f64,
    
    /// Other concepts this one relates to
    pub related_concepts: Vec<String>,
    
    /// Knowledge domain this concept belongs to
    pub domain: String,
    
    /// Abstract level of this concept
    pub abstraction_level: AbstractionLevel,
    
    /// Specific instances or examples of this concept in the content
    pub instances: Vec<ConceptInstance>,
}

/// Different levels of abstraction for concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbstractionLevel {
    Concrete,     // Specific, tangible things
    Specific,     // Particular but not necessarily tangible
    General,      // Broader categories or types
    Abstract,     // High-level principles or ideas
    Philosophical,// Deep, fundamental concepts
}

/// Specific instance where a concept appears in content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptInstance {
    /// Where in the content this instance appears
    pub location: ContentLocation,
    
    /// How this concept is expressed at this location
    pub expression: String,
    
    /// Context surrounding this instance
    pub context: String,
    
    /// Importance of this particular instance
    pub importance: f64,
}

/// Location within content where something is found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentLocation {
    /// Line number for text-based content
    LineNumber(u32),
    
    /// Byte offset for binary content
    ByteOffset(u64),
    
    /// Structural location (function name, class name, etc.)
    StructuralPath(String),
    
    /// Semantic location (section, chapter, topic)
    SemanticSection(String),
    
    /// Multiple locations for distributed concepts
    Multiple(Vec<ContentLocation>),
}

/// Analysis of themes and topics within content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThematicAnalysis {
    /// Main themes identified
    pub main_themes: Vec<Theme>,
    
    /// Supporting or minor themes
    pub supporting_themes: Vec<Theme>,
    
    /// How themes relate to each other
    pub theme_relationships: Vec<ThemeRelationship>,
    
    /// Overall thematic coherence score
    pub coherence_score: f64,
}

/// A theme identified within content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Name of the theme
    pub theme_name: String,
    
    /// How prominent this theme is
    pub prominence: f64,
    
    /// Concepts that contribute to this theme
    pub supporting_concepts: Vec<String>,
    
    /// Evidence for this theme in the content
    pub evidence: Vec<ThemeEvidence>,
}

/// Evidence supporting the presence of a theme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEvidence {
    /// Type of evidence
    pub evidence_type: EvidenceType,
    
    /// Location of the evidence
    pub location: ContentLocation,
    
    /// Strength of this evidence
    pub strength: f64,
    
    /// Description of the evidence
    pub description: String,
}

/// Types of evidence for themes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceType {
    DirectMention,    // Theme explicitly mentioned
    ImpliedPresence,  // Theme implied by content
    ConceptualCluster,// Groups of related concepts suggesting theme
    StructuralPattern,// Organization patterns suggesting theme
    EmotionalTone,    // Emotional content suggesting theme
}

/// Relationship between different themes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeRelationship {
    /// First theme in the relationship
    pub theme_a: String,
    
    /// Second theme in the relationship
    pub theme_b: String,
    
    /// Type of relationship between themes
    pub relationship_type: ThemeRelationshipType,
    
    /// Strength of the relationship
    pub strength: f64,
}

/// Types of relationships between themes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThemeRelationshipType {
    Supporting,    // One theme supports another
    Contrasting,   // Themes present different perspectives
    Complementary, // Themes work together
    Hierarchical,  // One theme is broader/more specific than another
    Sequential,    // Themes follow a logical sequence
    Conflicting,   // Themes are in tension with each other
}

/// Analysis of the intent and purpose behind content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnalysis {
    /// Primary purpose of the content
    pub primary_intent: Intent,
    
    /// Secondary purposes or goals
    pub secondary_intents: Vec<Intent>,
    
    /// Target audience for the content
    pub target_audience: AudienceProfile,
    
    /// Desired outcomes or effects
    pub desired_outcomes: Vec<DesiredOutcome>,
    
    /// Communication strategy employed
    pub communication_strategy: CommunicationStrategy,
}

/// Different types of intent behind content creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intent {
    /// Intent to inform or educate
    Informational {
        information_type: InformationType,
        complexity_level: ComplexityLevel,
    },
    
    /// Intent to instruct or guide action
    Instructional {
        instruction_type: InstructionType,
        skill_level_required: SkillLevel,
    },
    
    /// Intent to persuade or convince
    Persuasive {
        persuasion_strategy: PersuasionStrategy,
        target_change: TargetChange,
    },
    
    /// Intent to facilitate or enable something
    Facilitative {
        facilitation_type: FacilitationType,
        target_process: String,
    },
    
    /// Intent to analyze or evaluate
    Analytical {
        analysis_type: AnalysisType,
        evaluation_criteria: Vec<String>,
    },
    
    /// Intent to create or generate
    Creative {
        creation_type: CreationType,
        innovation_level: InnovationLevel,
    },
}

/// Types of information being conveyed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InformationType {
    Factual,      // Objective facts and data
    Procedural,   // How-to information
    Conceptual,   // Ideas and theories
    Historical,   // Past events and context
    Predictive,   // Future projections
    Comparative,  // Comparing different options
}

/// Types of instructions being provided
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstructionType {
    StepByStep,   // Sequential procedures
    Guidelines,   // General guidance and principles
    Rules,        // Specific rules to follow
    Examples,     // Learning through examples
    Interactive,  // Hands-on, interactive instruction
    Adaptive,     // Instructions that adapt to user needs
}

/// Required skill level for following instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillLevel {
    Novice,       // No prior experience required
    Beginner,     // Basic familiarity helpful
    Intermediate, // Some experience required
    Advanced,     // Significant expertise required
    Expert,       // Deep specialization required
}

/// Strategies for persuasion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersuasionStrategy {
    Logical,      // Rational arguments and evidence
    Emotional,    // Emotional appeals and stories
    Credibility,  // Authority and expertise-based
    Social,       // Social proof and consensus
    Ethical,      // Moral and value-based arguments
    Combined,     // Multiple strategies integrated
}

/// Target changes sought through persuasion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetChange {
    Belief,       // Change in beliefs or understanding
    Attitude,     // Change in attitudes or feelings
    Behavior,     // Change in actions or practices
    Decision,     // Specific decision or choice
    Priority,     // Change in what's considered important
    Commitment,   // Level of dedication or involvement
}

/// Profile of the target audience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceProfile {
    /// Primary audience characteristics
    pub primary_audience: AudienceCharacteristics,
    
    /// Secondary audiences that might also benefit
    pub secondary_audiences: Vec<AudienceCharacteristics>,
    
    /// Expertise level assumed
    pub assumed_expertise: ExpertiseLevel,
    
    /// Cultural and contextual considerations
    pub cultural_context: CulturalContext,
}

/// Characteristics of an audience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceCharacteristics {
    /// Role or position of audience members
    pub role: String,
    
    /// Level of expertise in relevant domains
    pub expertise_level: ExpertiseLevel,
    
    /// Goals and motivations
    pub goals: Vec<String>,
    
    /// Potential challenges or constraints
    pub challenges: Vec<String>,
    
    /// Preferred communication styles
    pub communication_preferences: Vec<CommunicationPreference>,
}

/// Different levels of expertise an audience might have
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpertiseLevel {
    Novice,       // New to the field
    Developing,   // Learning and growing
    Competent,    // Solid foundational knowledge
    Proficient,   // Strong practical skills
    Expert,       // Deep specialization
    Master,       // Comprehensive mastery and innovation
}

/// Communication preferences of audiences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationPreference {
    Visual,       // Prefers diagrams, charts, images
    Textual,      // Prefers written explanations
    Interactive,  // Prefers hands-on engagement
    Structured,   // Prefers organized, systematic presentation
    Narrative,    // Prefers story-based explanations
    Technical,    // Prefers precise, technical language
    Conversational, // Prefers informal, friendly tone
}

/// Cultural and contextual considerations for communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CulturalContext {
    /// Cultural background considerations
    pub cultural_factors: Vec<String>,
    
    /// Language and linguistic considerations
    pub linguistic_factors: Vec<String>,
    
    /// Professional or organizational context
    pub professional_context: String,
    
    /// Temporal context (when this is being used)
    pub temporal_context: String,
}

// =============================================================================
// RELATIONSHIP GRAPH STRUCTURES
// =============================================================================

/// A comprehensive graph representing all relationships between content and concepts
/// This is one of ZSEI's most powerful features - understanding how everything connects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipGraph {
    /// Unique identifier for this relationship graph
    pub graph_id: String,
    
    /// All nodes in the graph (content, concepts, entities)
    pub nodes: HashMap<String, GraphNode>,
    
    /// All edges representing relationships between nodes
    pub edges: HashMap<String, GraphEdge>,
    
    /// Metrics about the graph structure and complexity
    pub graph_metrics: GraphMetrics,
    
    /// Subgraphs for specific relationship types or domains
    pub subgraphs: HashMap<String, SubGraph>,
    
    /// Temporal evolution of the graph over time
    pub evolution_history: Vec<GraphEvolution>,
}

/// A node in the relationship graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier for this node
    pub node_id: String,
    
    /// Type of entity this node represents
    pub node_type: NodeType,
    
    /// Properties and attributes of this node
    pub properties: HashMap<String, serde_json::Value>,
    
    /// Importance or centrality of this node in the graph
    pub centrality_measures: CentralityMeasures,
    
    /// When this node was created and last updated
    pub temporal_info: NodeTemporalInfo,
}

/// Different types of nodes in the relationship graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    /// Content node - represents actual content (files, documents, etc.)
    Content {
        content_type: ContentType,
        content_path: String,
    },
    
    /// Concept node - represents abstract concepts or ideas
    Concept {
        concept_name: String,
        domain: String,
        abstraction_level: AbstractionLevel,
    },
    
    /// Entity node - represents specific entities (people, organizations, systems)
    Entity {
        entity_type: EntityType,
        entity_name: String,
    },
    
    /// Pattern node - represents recognized patterns
    Pattern {
        pattern_type: String,
        pattern_confidence: f64,
    },
    
    /// Process node - represents processes or procedures
    Process {
        process_type: ProcessType,
        process_name: String,
    },
    
    /// Goal node - represents objectives or desired outcomes
    Goal {
        goal_type: GoalType,
        goal_description: String,
    },
}

/// Types of entities that can be nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Person,       // Individual people
    Organization, // Companies, teams, groups
    System,       // Technical systems or applications
    Component,    // Parts of larger systems
    Resource,     // Available resources or assets
    Tool,         // Instruments or utilities
}

/// Types of processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessType {
    Development,  // Software development processes
    Analysis,     // Analysis and investigation processes
    Communication,// Communication and collaboration processes
    Decision,     // Decision-making processes
    Learning,     // Learning and education processes
    Operational,  // Day-to-day operational processes
}

/// Types of goals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalType {
    Functional,   // Achieving specific functionality
    Performance,  // Improving performance metrics
    Quality,      // Enhancing quality attributes
    Experience,   // Improving user experience
    Strategic,    // Long-term strategic objectives
    Learning,     // Knowledge acquisition goals
}

/// Centrality measures for understanding node importance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralityMeasures {
    /// How many direct connections this node has
    pub degree_centrality: f64,
    
    /// How much this node lies on paths between other nodes
    pub betweenness_centrality: f64,
    
    /// How close this node is to all other nodes
    pub closeness_centrality: f64,
    
    /// How important this node is based on its connections' importance
    pub eigenvector_centrality: f64,
    
    /// PageRank-style importance measure
    pub pagerank_score: f64,
}

/// Temporal information about nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTemporalInfo {
    /// When this node was first identified
    pub created_at: SystemTime,
    
    /// When this node was last updated
    pub last_updated: SystemTime,
    
    /// When this node was last accessed
    pub last_accessed: SystemTime,
    
    /// How this node's importance has changed over time
    pub importance_evolution: Vec<ImportanceSnapshot>,
}

/// Snapshot of node importance at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceSnapshot {
    pub timestamp: SystemTime,
    pub importance_score: f64,
    pub context: String,
}

/// An edge representing a relationship between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Unique identifier for this edge
    pub edge_id: String,
    
    /// Source node of the relationship
    pub source_node: String,
    
    /// Target node of the relationship
    pub target_node: String,
    
    /// Type and characteristics of the relationship
    pub relationship_type: RelationshipType,
    
    /// Strength or weight of the relationship
    pub strength: f64,
    
    /// Confidence in this relationship
    pub confidence: f64,
    
    /// Direction of the relationship
    pub directionality: RelationshipDirectionality,
    
    /// Properties specific to this relationship
    pub properties: HashMap<String, serde_json::Value>,
    
    /// Evidence supporting this relationship
    pub evidence: Vec<RelationshipEvidence>,
    
    /// When this relationship was identified and how it's evolved
    pub temporal_evolution: RelationshipTemporalEvolution,
}

/// Directionality of relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipDirectionality {
    /// Relationship flows from source to target
    Directed,
    
    /// Relationship is mutual/bidirectional
    Undirected,
    
    /// Relationship strength differs by direction
    Bidirectional {
        forward_strength: f64,
        reverse_strength: f64,
    },
}

/// Evidence supporting the existence of a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvidence {
    /// Type of evidence for this relationship
    pub evidence_type: RelationshipEvidenceType,
    
    /// Where the evidence was found
    pub source_location: ContentLocation,
    
    /// Strength of this evidence
    pub evidence_strength: f64,
    
    /// Description of the evidence
    pub description: String,
    
    /// When this evidence was discovered
    pub discovered_at: SystemTime,
}

/// Types of evidence for relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipEvidenceType {
    ExplicitMention,    // Relationship explicitly stated
    ImplicitReference,  // Relationship implied or inferred
    StructuralPattern,  // Relationship evident from structure
    BehavioralPattern,  // Relationship evident from usage
    SemanticSimilarity, // Relationship based on meaning
    TemporalCorrelation,// Relationship based on timing
    StatisticalCorrelation, // Relationship based on statistics
}

/// How a relationship evolves over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipTemporalEvolution {
    /// When this relationship was first identified
    pub first_identified: SystemTime,
    
    /// How the relationship strength has changed
    pub strength_evolution: Vec<StrengthSnapshot>,
    
    /// How confidence in the relationship has changed
    pub confidence_evolution: Vec<ConfidenceSnapshot>,
    
    /// Major events that affected this relationship
    pub evolution_events: Vec<RelationshipEvent>,
}

/// Snapshot of relationship strength at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrengthSnapshot {
    pub timestamp: SystemTime,
    pub strength: f64,
    pub context: String,
}

/// Snapshot of confidence in a relationship at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceSnapshot {
    pub timestamp: SystemTime,
    pub confidence: f64,
    pub validation_method: String,
}

/// Events that significantly affected a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvent {
    /// When the event occurred
    pub timestamp: SystemTime,
    
    /// Type of event
    pub event_type: RelationshipEventType,
    
    /// Description of what happened
    pub description: String,
    
    /// How this event affected the relationship
    pub impact: RelationshipImpact,
}

/// Types of events that can affect relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipEventType {
    Discovery,        // Relationship newly discovered
    Strengthening,    // Relationship became stronger
    Weakening,        // Relationship became weaker
    Validation,       // Relationship was validated
    Invalidation,     // Relationship was found to be incorrect
    Transformation,   // Relationship changed type or nature
    Integration,      // Relationship integrated with others
}

/// Impact of an event on a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipImpact {
    /// Change in relationship strength
    pub strength_change: f64,
    
    /// Change in confidence
    pub confidence_change: f64,
    
    /// Whether the relationship type changed
    pub type_change: Option<RelationshipType>,
    
    /// Additional effects
    pub additional_effects: Vec<String>,
}

/// Subgraph representing a specific subset of relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGraph {
    /// Identifier for this subgraph
    pub subgraph_id: String,
    
    /// Focus or theme of this subgraph
    pub focus: String,
    
    /// Nodes included in this subgraph
    pub nodes: HashSet<String>,
    
    /// Edges included in this subgraph
    pub edges: HashSet<String>,
    
    /// Metrics specific to this subgraph
    pub subgraph_metrics: SubGraphMetrics,
}

/// Metrics for subgraphs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGraphMetrics {
    /// Number of nodes in the subgraph
    pub node_count: usize,
    
    /// Number of edges in the subgraph
    pub edge_count: usize,
    
    /// Density of connections in the subgraph
    pub density: f64,
    
    /// How cohesive the subgraph is
    pub cohesion: f64,
    
    /// How separated this subgraph is from others
    pub separation: f64,
}

/// Overall metrics for the entire relationship graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetrics {
    /// Total number of nodes
    pub total_nodes: usize,
    
    /// Total number of edges
    pub total_edges: usize,
    
    /// Overall graph density
    pub graph_density: f64,
    
    /// Average path length between nodes
    pub average_path_length: f64,
    
    /// Clustering coefficient
    pub clustering_coefficient: f64,
    
    /// Number of connected components
    pub connected_components: usize,
    
    /// Diameter of the graph (longest shortest path)
    pub graph_diameter: usize,
    
    /// Most central nodes
    pub most_central_nodes: Vec<String>,
    
    /// Quality score for the overall graph
    pub quality_score: f64,
}

/// How the graph evolves over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEvolution {
    /// When this evolution snapshot was taken
    pub timestamp: SystemTime,
    
    /// What triggered this evolution
    pub evolution_trigger: EvolutionTrigger,
    
    /// Changes made to the graph
    pub changes: GraphChanges,
    
    /// Metrics before and after the evolution
    pub metrics_change: MetricsChange,
}

/// Events that trigger graph evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionTrigger {
    ContentAdded,       // New content was analyzed
    ContentModified,    // Existing content was changed
    RelationshipDiscovered, // New relationship was found
    PatternRecognized,  // New pattern was identified
    ValidationPerformed, // Existing relationships were validated
    LearningIntegrated, // New learning was integrated
    WisdomAccumulated,  // Wisdom was accumulated and integrated
}

/// Changes made to the graph during evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphChanges {
    /// Nodes added to the graph
    pub nodes_added: Vec<String>,
    
    /// Nodes removed from the graph
    pub nodes_removed: Vec<String>,
    
    /// Nodes whose properties were updated
    pub nodes_modified: Vec<String>,
    
    /// Edges added to the graph
    pub edges_added: Vec<String>,
    
    /// Edges removed from the graph
    pub edges_removed: Vec<String>,
    
    /// Edges whose properties were updated
    pub edges_modified: Vec<String>,
}

/// How graph metrics changed during evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsChange {
    /// Metrics before the change
    pub before: GraphMetrics,
    
    /// Metrics after the change
    pub after: GraphMetrics,
    
    /// Calculated differences
    pub differences: MetricsDifference,
}

/// Calculated differences in graph metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDifference {
    /// Change in number of nodes
    pub node_count_change: i32,
    
    /// Change in number of edges
    pub edge_count_change: i32,
    
    /// Change in graph density
    pub density_change: f64,
    
    /// Change in average path length
    pub path_length_change: f64,
    
    /// Change in clustering coefficient
    pub clustering_change: f64,
    
    /// Change in quality score
    pub quality_change: f64,
}

// =============================================================================
// CROSS-DOMAIN INSIGHT STRUCTURES
// =============================================================================

/// Insights that emerge from applying knowledge from one domain to another
/// This is where ZSEI's cross-domain intelligence really shines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainInsight {
    /// Unique identifier for this insight
    pub insight_id: String,
    
    /// Source domain where the insight originated
    pub source_domain: KnowledgeDomain,
    
    /// Target domain where the insight applies
    pub target_domain: KnowledgeDomain,
    
    /// The actual insight or principle being transferred
    pub insight_content: InsightContent,
    
    /// How confident we are in this cross-domain application
    pub confidence: f64,
    
    /// Potential benefits of applying this insight
    pub potential_benefits: Vec<PotentialBenefit>,
    
    /// Evidence supporting this cross-domain application
    pub supporting_evidence: Vec<InsightEvidence>,
    
    /// How this insight might be practically applied
    pub application_strategies: Vec<ApplicationStrategy>,
    
    /// Validation results if this insight has been tested
    pub validation_results: Option<ValidationResults>,
}

/// Knowledge domains that ZSEI understands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeDomain {
    /// Biological systems and life sciences
    Biology {
        subdomain: BiologySubdomain,
        specialization: Option<String>,
    },
    
    /// Mathematical principles and methods
    Mathematics {
        subdomain: MathematicsSubdomain,
        specialization: Option<String>,
    },
    
    /// Physical sciences and physics
    Physics {
        subdomain: PhysicsSubdomain,
        specialization: Option<String>,
    },
    
    /// Psychology and cognitive science
    Psychology {
        subdomain: PsychologySubdomain,
        specialization: Option<String>,
    },
    
    /// Engineering and applied sciences
    Engineering {
        subdomain: EngineeringSubdomain,
        specialization: Option<String>,
    },
    
    /// Computer science and technology
    ComputerScience {
        subdomain: ComputerScienceSubdomain,
        specialization: Option<String>,
    },
    
    /// Social sciences and human behavior
    SocialScience {
        subdomain: SocialScienceSubdomain,
        specialization: Option<String>,
    },
    
    /// Business and management
    Business {
        subdomain: BusinessSubdomain,
        specialization: Option<String>,
    },
    
    /// Arts and creative disciplines
    Arts {
        subdomain: ArtsSubdomain,
        specialization: Option<String>,
    },
}

/// Subdomains of biology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiologySubdomain {
    CellBiology,
    Genetics,
    Evolution,
    Ecology,
    Neuroscience,
    Immunology,
    Development,
    Behavior,
}

/// Subdomains of mathematics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MathematicsSubdomain {
    Algebra,
    Calculus,
    Statistics,
    Topology,
    GraphTheory,
    Optimization,
    Logic,
    NumberTheory,
}

/// Subdomains of physics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhysicsSubdomain {
    ClassicalMechanics,
    Thermodynamics,
    Electromagnetism,
    QuantumMechanics,
    Relativity,
    StatisticalMechanics,
    CondensedMatter,
    ParticlePhysics,
}

/// Subdomains of psychology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PsychologySubdomain {
    Cognitive,
    Behavioral,
    Social,
    Developmental,
    Clinical,
    Organizational,
    Educational,
    Personality,
}

/// Subdomains of engineering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineeringSubdomain {
    SoftwareEngineering,
    SystemsEngineering,
    MechanicalEngineering,
    ElectricalEngineering,
    CivilEngineering,
    ChemicalEngineering,
    IndustrialEngineering,
    BiomedicalEngineering,
}

/// Subdomains of computer science
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputerScienceSubdomain {
    Algorithms,
    DataStructures,
    MachineLearning,
    HumanComputerInteraction,
    DatabaseSystems,
    NetworkSystems,
    CyberSecurity,
    ArtificialIntelligence,
}

/// Subdomains of social science
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SocialScienceSubdomain {
    Sociology,
    Anthropology,
    Political_Science,
    Economics,
    Communication,
    Linguistics,
    Geography,
    Archaeology,
}

/// Subdomains of business
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BusinessSubdomain {
    Strategy,
    Operations,
    Marketing,
    Finance,
    HumanResources,
    Leadership,
    Innovation,
    Entrepreneurship,
}

/// Subdomains of arts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtsSubdomain {
    VisualArts,
    Music,
    Literature,
    Theater,
    Film,
    Dance,
    DigitalArts,
    DesignArts,
}

/// The actual content of a cross-domain insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightContent {
    /// Brief summary of the insight
    pub summary: String,
    
    /// Detailed description of the insight
    pub detailed_description: String,
    
    /// The principle or pattern being transferred
    pub core_principle: CorePrinciple,
    
    /// How this principle manifests in the source domain
    pub source_manifestation: String,
    
    /// How this principle could manifest in the target domain
    pub target_manifestation: String,
    
    /// Key concepts involved in this insight
    pub key_concepts: Vec<String>,
    
    /// Analogies or metaphors that help explain the insight
    pub analogies: Vec<Analogy>,
}

/// A core principle that can be transferred across domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorePrinciple {
    /// Name of the principle
    pub principle_name: String,
    
    /// Type of principle this represents
    pub principle_type: PrincipleType,
    
    /// Abstract formulation of the principle
    pub abstract_formulation: String,
    
    /// Level of abstraction for this principle
    pub abstraction_level: AbstractionLevel,
    
    /// How broadly this principle applies
    pub universality: UniversalityLevel,
    
    /// Evidence for the universality of this principle
    pub universality_evidence: Vec<UniversalityEvidence>,
}

/// Types of principles that can be transferred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrincipleType {
    Structural,      // How things are organized or arranged
    Functional,      // How things work or operate
    Behavioral,      // How things behave or respond
    Developmental,   // How things grow or evolve
    Optimization,    // How to achieve optimal outcomes
    Coordination,    // How to coordinate complex systems
    Adaptation,      // How to adapt to changing conditions
    Emergence,       // How complex properties emerge from simple rules
}

/// How universally a principle applies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UniversalityLevel {
    DomainSpecific,   // Only applies within specific domains
    ClusterSpecific,  // Applies to related domain clusters
    BroadlyApplicable,// Applies across many domains
    NearUniversal,    // Applies to almost all domains
    Universal,        // Applies universally across all domains
}

/// Evidence for the universality of a principle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalityEvidence {
    /// Domain where this principle has been observed
    pub domain: KnowledgeDomain,
    
    /// Specific examples in that domain
    pub examples: Vec<String>,
    
    /// Confidence in this domain application
    pub confidence: f64,
    
    /// How well-established this principle is in this domain
    pub establishment_level: EstablishmentLevel,
}

/// How well-established a principle is in a domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EstablishmentLevel {
    Hypothetical,     // Theoretical possibility
    Emerging,         // Early evidence
    Developing,       // Growing evidence base
    Established,      // Well-supported principle
    Fundamental,      // Core principle of the domain
}

/// An analogy that helps explain cross-domain insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analogy {
    /// Brief description of the analogy
    pub description: String,
    
    /// What in the source domain corresponds to what in the target domain
    pub mappings: Vec<AnalogyMapping>,
    
    /// How strong or apt this analogy is
    pub analogy_strength: f64,
    
    /// Limitations of this analogy
    pub limitations: Vec<String>,
}

/// Mapping between elements in an analogy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalogyMapping {
    /// Element in the source domain
    pub source_element: String,
    
    /// Corresponding element in the target domain
    pub target_element: String,
    
    /// Type of correspondence
    pub mapping_type: MappingType,
    
    /// Confidence in this mapping
    pub confidence: f64,
}

/// Types of correspondences in analogies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MappingType {
    Structural,   // Similar structure or organization
    Functional,   // Similar function or purpose
    Behavioral,   // Similar behavior or response
    Relational,   // Similar relationships to other elements
    Causal,       // Similar causal properties
    Abstract,     // Similar abstract properties
}

/// Potential benefits of applying a cross-domain insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialBenefit {
    /// Type of benefit expected
    pub benefit_type: BenefitType,
    
    /// Description of the benefit
    pub description: String,
    
    /// Estimated magnitude of the benefit
    pub estimated_impact: ImpactLevel,
    
    /// Confidence in achieving this benefit
    pub confidence: f64,
    
    /// Timeline for realizing this benefit
    pub timeline: BenefitTimeline,
    
    /// Prerequisites for achieving this benefit
    pub prerequisites: Vec<String>,
}

/// Types of benefits from cross-domain insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenefitType {
    Performance,      // Improved performance or efficiency
    Quality,          // Enhanced quality or reliability
    Innovation,       // Novel approaches or solutions
    Understanding,    // Deeper insight or comprehension
    Capability,       // New capabilities or features
    Robustness,       // Increased stability or resilience
    Scalability,      // Better scaling properties
    Maintainability,  // Easier maintenance or updates
}

/// Magnitude of expected impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Minimal,      // Small but measurable impact
    Moderate,     // Noticeable improvement
    Significant,  // Major improvement
    Substantial,  // Large, important improvement
    Transformative, // Fundamental change in capability
}

/// Timeline for realizing benefits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenefitTimeline {
    Immediate,    // Benefits available right away
    ShortTerm,    // Benefits within days or weeks
    MediumTerm,   // Benefits within months
    LongTerm,     // Benefits within years
    Ongoing,      // Benefits that accrue continuously
}

// =============================================================================
// RECOGNIZED PATTERN STRUCTURES
// =============================================================================

/// A pattern that ZSEI has recognized within or across content
/// Patterns represent recurring themes, structures, or behaviors that ZSEI can learn from
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizedPattern {
    /// Unique identifier for this pattern
    pub pattern_id: String,
    
    /// Type and characteristics of the pattern
    pub pattern_type: RecognizedPatternType,
    
    /// Confidence in this pattern recognition
    pub confidence: f64,
    
    /// How many instances of this pattern have been observed
    pub instance_count: u32,
    
    /// Specific instances where this pattern appears
    pub instances: Vec<PatternInstance>,
    
    /// What makes this pattern significant or useful
    pub significance: PatternSignificance,
    
    /// How this pattern correlates with successful outcomes
    pub success_correlation: SuccessCorrelation,
    
    /// Evolution of this pattern over time
    pub pattern_evolution: PatternEvolution,
}

/// Different types of patterns ZSEI can recognize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecognizedPatternType {
    /// Structural patterns - how content is organized
    Structural {
        structure_type: StructuralPatternType,
        complexity_level: ComplexityLevel,
    },
    
    /// Behavioral patterns - how content is used or behaves
    Behavioral {
        behavior_type: BehavioralPatternType,
        context: String,
    },
    
    /// Communication patterns - how information is conveyed
    Communication {
        communication_type: CommunicationPatternType,
        effectiveness: f64,
    },
    
    /// Problem-solving patterns - approaches to solving problems
    ProblemSolving {
        problem_category: String,
        solution_approach: SolutionApproach,
    },
    
    /// Learning patterns - how knowledge is acquired and applied
    Learning {
        learning_type: LearningPatternType,
        domain: KnowledgeDomain,
    },
    
    /// Collaboration patterns - how entities work together
    Collaboration {
        collaboration_type: CollaborationPatternType,
        participants: Vec<String>,
    },
    
    /// Evolution patterns - how things change and develop over time
    Evolution {
        evolution_type: EvolutionPatternType,
        trajectory: EvolutionTrajectory,
    },
}

/// Types of structural patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructuralPatternType {
    Hierarchical,     // Tree-like hierarchies
    Modular,          // Modular organization
    Layered,          // Layered architectures
    Network,          // Network-like structures
    Pipeline,         // Sequential processing pipelines
    Hub,              // Hub-and-spoke patterns
    Mesh,             // Mesh-like interconnections
    Fractal,          // Self-similar recursive structures
}

/// Types of behavioral patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BehavioralPatternType {
    Usage,            // How things are typically used
    Response,         // How things respond to stimuli
    Adaptation,       // How things adapt to changes
    Coordination,     // How things coordinate with others
    Performance,      // Performance characteristics
    Failure,          // How and when things fail
    Recovery,         // How things recover from problems
}

/// Types of communication patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationPatternType {
    Informational,    // Patterns for conveying information
    Instructional,    // Patterns for teaching or guiding
    Persuasive,       // Patterns for convincing or motivating
    Collaborative,    // Patterns for facilitating collaboration
    Feedback,         // Patterns for providing feedback
    Narrative,        // Storytelling patterns
    Technical,        // Technical communication patterns
}

/// Approaches to solving problems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolutionApproach {
    Analytical,       // Breaking down into parts
    Synthetic,        // Building up from components
    Iterative,        // Gradual refinement
    Experimental,     // Trial and error
    Analogical,       // Using analogies from other domains
    Systematic,       // Following systematic methodologies
    Creative,         // Novel, innovative approaches
    Collaborative,    // Working with others to solve
}

/// Types of learning patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningPatternType {
    Incremental,      // Gradual, step-by-step learning
    Breakthrough,     // Sudden insights or understanding
    Experiential,     // Learning through direct experience
    Observational,    // Learning by watching others
    Reflective,       // Learning through reflection and analysis
    Collaborative,    // Learning through collaboration
    Adaptive,         // Learning that adapts to context
}

/// Types of collaboration patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationPatternType {
    Coordination,     // Coordinated parallel work
    Cooperation,      // Mutual assistance and support
    Negotiation,      // Working through differences
    Integration,      // Combining different contributions
    Specialization,   // Leveraging different specialties
    Mentoring,        // Knowledge transfer relationships
    Innovation,       // Collaborative innovation patterns
}

/// Types of evolution patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionPatternType {
    Gradual,          // Slow, steady evolution
    Punctuated,       // Periods of stability with rapid change
    Cyclical,         // Recurring cycles of change
    Convergent,       // Evolution toward similar endpoints
    Divergent,        // Evolution in different directions
    Adaptive,         // Evolution in response to environment
    Emergent,         // Evolution leading to emergent properties
}

/// Trajectory of evolution over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionTrajectory {
    /// Starting point of the evolution
    pub origin: EvolutionPoint,
    
    /// Current state of evolution
    pub current_state: EvolutionPoint,
    
    /// Key waypoints in the evolution
    pub waypoints: Vec<EvolutionWaypoint>,
    
    /// Predicted future trajectory
    pub predicted_trajectory: Vec<EvolutionPrediction>,
}

/// A point in an evolution trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionPoint {
    /// When this point occurred
    pub timestamp: SystemTime,
    
    /// Characteristics at this point
    pub characteristics: HashMap<String, f64>,
    
    /// Context at this point
    pub context: String,
}

/// A significant waypoint in evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionWaypoint {
    /// The evolution point
    pub point: EvolutionPoint,
    
    /// What made this point significant
    pub significance: String,
    
    /// Events that triggered changes at this point
    pub trigger_events: Vec<String>,
}

/// A prediction about future evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionPrediction {
    /// Predicted evolution point
    pub predicted_point: EvolutionPoint,
    
    /// Confidence in this prediction
    pub confidence: f64,
    
    /// Assumptions underlying this prediction
    pub assumptions: Vec<String>,
}

/// A specific instance where a pattern appears
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternInstance {
    /// Where this pattern instance appears
    pub location: ContentLocation,
    
    /// How well this instance matches the overall pattern
    pub match_quality: f64,
    
    /// Context surrounding this instance
    pub context: PatternContext,
    
    /// Variations or adaptations in this instance
    pub variations: Vec<PatternVariation>,
    
    /// Outcomes associated with this pattern instance
    pub outcomes: Vec<PatternOutcome>,
}

/// Context surrounding a pattern instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternContext {
    /// Environmental factors affecting the pattern
    pub environmental_factors: Vec<String>,
    
    /// Other patterns present at the same time
    pub concurrent_patterns: Vec<String>,
    
    /// Goals or objectives in this context
    pub objectives: Vec<String>,
    
    /// Constraints or limitations in this context
    pub constraints: Vec<String>,
}

/// Variations of a pattern in specific instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternVariation {
    /// Type of variation
    pub variation_type: VariationType,
    
    /// Description of the variation
    pub description: String,
    
    /// Why this variation occurred
    pub reason: String,
    
    /// Effect of this variation on pattern effectiveness
    pub effectiveness_impact: f64,
}

/// Types of pattern variations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariationType {
    Adaptation,       // Pattern adapted to specific context
    Simplification,   // Pattern simplified for easier application
    Extension,        // Pattern extended with additional elements
    Combination,      // Pattern combined with other patterns
    Optimization,     // Pattern optimized for specific goals
    Specialization,   // Pattern specialized for specific domain
}

/// Outcomes associated with pattern instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternOutcome {
    /// Type of outcome
    pub outcome_type: OutcomeType,
    
    /// Measurement of the outcome
    pub outcome_measure: OutcomeMeasure,
    
    /// How directly the pattern contributed to this outcome
    pub attribution_confidence: f64,
    
    /// Time between pattern application and outcome
    pub time_to_outcome: Duration,
}

/// Types of outcomes from pattern application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutcomeType {
    Success,          // Successful achievement of goals
    Failure,          // Failed to achieve goals
    PartialSuccess,   // Partial achievement of goals
    UnexpectedBenefit,// Unexpected positive outcome
    UnexpectedCost,   // Unexpected negative outcome
    LearningOpportunity, // Outcome provided learning value
}

/// Measurement of pattern outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutcomeMeasure {
    /// Qualitative assessment
    Qualitative {
        assessment: QualitativeAssessment,
        description: String,
    },
    
    /// Quantitative measurement
    Quantitative {
        metric: String,
        value: f64,
        unit: String,
    },
    
    /// Binary success/failure
    Binary {
        success: bool,
        criteria: String,
    },
    
    /// Comparative measure against baseline
    Comparative {
        improvement: f64,
        baseline_description: String,
    },
}

/// Qualitative assessments of outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualitativeAssessment {
    Excellent,
    Good,
    Satisfactory,
    Poor,
    Failure,
}

/// Significance of a pattern - why it matters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSignificance {
    /// Type of significance this pattern has
    pub significance_type: SignificanceType,
    
    /// How significant this pattern is (0.0 to 1.0)
    pub significance_score: f64,
    
    /// Factors that contribute to this pattern's significance
    pub significance_factors: Vec<SignificanceFactor>,
    
    /// Potential impact of applying this pattern
    pub potential_impact: ImpactLevel,
    
    /// Scope of applicability for this pattern
    pub applicability_scope: ApplicabilityScope,
}

/// Types of pattern significance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignificanceType {
    Predictive,       // Pattern helps predict future outcomes
    Causal,           // Pattern reveals causal relationships
    Efficiency,       // Pattern improves efficiency
    Quality,          // Pattern improves quality
    Innovation,       // Pattern enables innovation
    Learning,         // Pattern facilitates learning
    Collaboration,    // Pattern improves collaboration
    Problem_Solving,  // Pattern helps solve problems
}

/// Factors contributing to pattern significance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignificanceFactor {
    /// What factor contributes to significance
    pub factor: String,
    
    /// How much this factor contributes
    pub contribution: f64,
    
    /// Evidence supporting this factor's importance
    pub evidence: Vec<String>,
}

/// Scope where a pattern is applicable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApplicabilityScope {
    Specific {
        contexts: Vec<String>,
    },
    Domain {
        domains: Vec<KnowledgeDomain>,
    },
    CrossDomain {
        domain_clusters: Vec<String>,
    },
    Universal {
        limitations: Vec<String>,
    },
}

/// How a pattern correlates with successful outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCorrelation {
    /// Statistical correlation coefficient
    pub correlation_coefficient: f64,
    
    /// Confidence in this correlation
    pub confidence: f64,
    
    /// Sample size used to calculate correlation
    pub sample_size: u32,
    
    /// P-value for statistical significance
    pub p_value: f64,
    
    /// Type of success being correlated
    pub success_definition: SuccessDefinition,
    
    /// Causal analysis of the correlation
    pub causal_analysis: CausalAnalysis,
}

/// Definition of what constitutes "success" for correlation analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessDefinition {
    /// Criteria for measuring success
    pub success_criteria: Vec<SuccessCriterion>,
    
    /// Weighting of different criteria
    pub criterion_weights: HashMap<String, f64>,
    
    /// Threshold for considering an outcome successful
    pub success_threshold: f64,
    
    /// Context in which success is defined
    pub context: String,
}

/// Individual criterion for success
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriterion {
    /// Name of the criterion
    pub criterion_name: String,
    
    /// How to measure this criterion
    pub measurement_method: String,
    
    /// Target value for this criterion
    pub target_value: f64,
    
    /// Minimum acceptable value
    pub minimum_value: f64,
    
    /// Maximum possible value
    pub maximum_value: f64,
}

/// Analysis of whether correlation implies causation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalAnalysis {
    /// Whether causation is likely
    pub causal_likelihood: CausalLikelihood,
    
    /// Potential causal mechanisms
    pub causal_mechanisms: Vec<CausalMechanism>,
    
    /// Alternative explanations for the correlation
    pub alternative_explanations: Vec<AlternativeExplanation>,
    
    /// Tests that could validate or refute causation
    pub validation_tests: Vec<ValidationTest>,
}

/// Likelihood that correlation implies causation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CausalLikelihood {
    VeryUnlikely,     // Correlation is likely spurious
    Unlikely,         // Weak evidence for causation
    Possible,         // Some evidence for causation
    Likely,           // Good evidence for causation
    VeryLikely,       // Strong evidence for causation
    Definitive,       // Causation has been proven
}

/// Potential mechanism by which a pattern causes success
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalMechanism {
    /// Description of the proposed mechanism
    pub mechanism_description: String,
    
    /// Steps in the causal chain
    pub causal_steps: Vec<CausalStep>,
    
    /// Plausibility of this mechanism
    pub plausibility: f64,
    
    /// Evidence supporting this mechanism
    pub supporting_evidence: Vec<String>,
}

/// A step in a causal mechanism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalStep {
    /// Description of this step
    pub step_description: String,
    
    /// Input to this step
    pub input: String,
    
    /// Output from this step
    pub output: String,
    
    /// Conditions required for this step
    pub required_conditions: Vec<String>,
}

/// Alternative explanation for observed correlations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeExplanation {
    /// Description of the alternative explanation
    pub explanation: String,
    
    /// Plausibility of this explanation
    pub plausibility: f64,
    
    /// Evidence supporting this explanation
    pub supporting_evidence: Vec<String>,
    
    /// How to test this explanation
    pub testing_strategy: String,
}

/// Test to validate causal relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationTest {
    /// Description of the test
    pub test_description: String,
    
    /// Type of validation test
    pub test_type: ValidationTestType,
    
    /// Expected results if causation is real
    pub expected_results: String,
    
    /// Feasibility of performing this test
    pub feasibility: TestFeasibility,
}

/// Types of validation tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationTestType {
    Controlled_Experiment,  // Controlled experimental test
    Natural_Experiment,     // Natural experiment observation
    Longitudinal_Study,     // Long-term observational study
    Cross_Validation,       // Cross-validation across different contexts
    Predictive_Test,        // Test predictive power of the pattern
    Intervention_Study,     // Test by intervening with the pattern
}

/// Feasibility of performing a validation test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestFeasibility {
    HighlyFeasible,   // Easy to perform
    Feasible,         // Doable with some effort
    Challenging,      // Difficult but possible
    Difficult,        // Very challenging to perform
    Infeasible,       // Not practical to perform
}

/// How a pattern evolves over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEvolution {
    /// When this pattern was first recognized
    pub first_recognition: SystemTime,
    
    /// How the pattern has changed over time
    pub evolution_timeline: Vec<PatternEvolutionEvent>,
    
    /// Current maturity of the pattern
    pub maturity_level: PatternMaturityLevel,
    
    /// Predicted future evolution of the pattern
    pub future_predictions: Vec<PatternFuturePrediction>,
    
    /// Factors driving pattern evolution
    pub evolution_drivers: Vec<EvolutionDriver>,
}

/// Events in the evolution of a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEvolutionEvent {
    /// When the event occurred
    pub timestamp: SystemTime,
    
    /// Type of evolution event
    pub event_type: PatternEvolutionEventType,
    
    /// Description of what changed
    pub change_description: String,
    
    /// Impact of the change on pattern effectiveness
    pub effectiveness_impact: f64,
    
    /// Confidence that this change was significant
    pub significance_confidence: f64,
}

/// Types of pattern evolution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternEvolutionEventType {
    Discovery,        // Pattern was first discovered
    Refinement,       // Pattern was refined or improved
    Validation,       // Pattern was validated through testing
    Adaptation,       // Pattern was adapted to new contexts
    Integration,      // Pattern was integrated with other patterns
    Generalization,   // Pattern was generalized to broader contexts
    Specialization,   // Pattern was specialized for specific contexts
    Deprecation,      // Pattern was found to be less effective
}

/// Maturity level of a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternMaturityLevel {
    Emerging,         // Newly discovered, limited validation
    Developing,       // Some validation, growing evidence
    Established,      // Well-validated, reliable application
    Mature,           // Highly refined, broad applicability
    Declining,        // Becoming less relevant or effective
}

/// Prediction about future pattern evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternFuturePrediction {
    /// Predicted timeframe
    pub timeframe: PredictionTimeframe,
    
    /// Predicted changes to the pattern
    pub predicted_changes: Vec<PredictedChange>,
    
    /// Confidence in these predictions
    pub confidence: f64,
    
    /// Assumptions underlying the predictions
    pub assumptions: Vec<String>,
    
    /// Indicators to watch for validation
    pub validation_indicators: Vec<String>,
}

/// Timeframes for predictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictionTimeframe {
    ShortTerm,        // Days to weeks
    MediumTerm,       // Weeks to months
    LongTerm,         // Months to years
    VeryLongTerm,     // Years to decades
}

/// Predicted changes to patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedChange {
    /// Type of change predicted
    pub change_type: PredictedChangeType,
    
    /// Description of the change
    pub description: String,
    
    /// Likelihood of this change occurring
    pub likelihood: f64,
    
    /// Potential impact of this change
    pub impact: ImpactLevel,
}

/// Types of predicted changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictedChangeType {
    Effectiveness_Improvement, // Pattern will become more effective
    Effectiveness_Decline,     // Pattern will become less effective
    Scope_Expansion,          // Pattern will apply to more contexts
    Scope_Narrowing,          // Pattern will apply to fewer contexts
    Integration_Opportunity,   // Pattern will integrate with others
    Replacement,              // Pattern will be replaced by better alternatives
    Transformation,           // Pattern will fundamentally change
}

/// Factors driving pattern evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionDriver {
    /// Name of the driving factor
    pub driver_name: String,
    
    /// Type of evolution driver
    pub driver_type: EvolutionDriverType,
    
    /// Strength of this driver's influence
    pub influence_strength: f64,
    
    /// Direction of change this driver promotes
    pub change_direction: ChangeDirection,
    
    /// Evidence for this driver's influence
    pub evidence: Vec<String>,
}

/// Types of factors that drive pattern evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionDriverType {
    Technology_Change,        // Changes in available technology
    Context_Change,          // Changes in application contexts
    Understanding_Improvement, // Better understanding of the pattern
    Competition,             // Competition from alternative patterns
    Integration_Opportunity,  // Opportunities to integrate with other patterns
    External_Pressure,       // External pressures for change
    Internal_Optimization,   // Internal drives for optimization
}

/// Direction of change promoted by evolution drivers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeDirection {
    Improvement,      // Toward better effectiveness
    Simplification,   // Toward simpler application
    Generalization,   // Toward broader applicability
    Specialization,   // Toward specific optimization
    Integration,      // Toward combination with others
    Differentiation,  // Toward unique characteristics
}

// =============================================================================
// USAGE ANALYTICS AND LEARNING STRUCTURES
// =============================================================================

/// Analytics about how content is used and accessed over time
/// This helps ZSEI understand what content is valuable and how it's being utilized
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalytics {
    /// Basic access statistics
    pub access_statistics: AccessStatistics,
    
    /// Patterns in how content is used
    pub usage_patterns: Vec<UsagePattern>,
    
    /// User interaction analytics
    pub interaction_analytics: InteractionAnalytics,
    
    /// Performance metrics for content access
    pub performance_metrics: ContentPerformanceMetrics,
    
    /// Value and impact metrics
    pub value_metrics: ContentValueMetrics,
    
    /// Learning outcomes from content usage
    pub learning_outcomes: Vec<LearningOutcome>,
}

/// Basic statistics about content access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessStatistics {
    /// Total number of times content has been accessed
    pub total_accesses: u64,
    
    /// Unique users/components that have accessed the content
    pub unique_accessors: u32,
    
    /// Time of first access
    pub first_access: SystemTime,
    
    /// Time of most recent access
    pub last_access: SystemTime,
    
    /// Average time between accesses
    pub average_access_interval: Duration,
    
    /// Peak usage periods
    pub peak_usage_periods: Vec<UsagePeriod>,
    
    /// Access patterns by time of day/week
    pub temporal_distribution: TemporalDistribution,
}

/// A period of high usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePeriod {
    /// Start of the usage period
    pub start_time: SystemTime,
    
    /// End of the usage period
    pub end_time: SystemTime,
    
    /// Number of accesses during this period
    pub access_count: u32,
    
    /// What triggered this period of high usage
    pub trigger: Option<String>,
}

/// Distribution of access across time periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalDistribution {
    /// Access patterns by hour of day
    pub hourly_distribution: Vec<f64>,
    
    /// Access patterns by day of week
    pub daily_distribution: Vec<f64>,
    
    /// Access patterns by month
    pub monthly_distribution: Vec<f64>,
    
    /// Seasonal patterns if any
    pub seasonal_patterns: Vec<SeasonalPattern>,
}

/// Seasonal patterns in content usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPattern {
    /// Description of the seasonal pattern
    pub pattern_description: String,
    
    /// Strength of the seasonal pattern
    pub pattern_strength: f64,
    
    /// Peak and trough periods
    pub peak_periods: Vec<String>,
    pub trough_periods: Vec<String>,
    
    /// Confidence in this seasonal pattern
    pub confidence: f64,
}

/// Analytics about user interactions with content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionAnalytics {
    /// Types of interactions observed
    pub interaction_types: HashMap<InteractionType, u32>,
    
    /// Average duration of interactions
    pub average_interaction_duration: Duration,
    
    /// Most common interaction sequences
    pub common_sequences: Vec<InteractionSequence>,
    
    /// Interaction success rates
    pub success_rates: HashMap<InteractionType, f64>,
    
    /// User satisfaction indicators
    pub satisfaction_indicators: SatisfactionIndicators,
}

/// Types of interactions users can have with content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    View,             // Simply viewing/reading content
    Edit,             // Modifying content
    Analyze,          // Performing analysis on content
    Reference,        // Using content as reference
    Learn,            // Learning from content
    Apply,            // Applying content knowledge
    Share,            // Sharing content with others
    Discuss,          // Discussing content
}

/// A sequence of interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionSequence {
    /// The sequence of interaction types
    pub sequence: Vec<InteractionType>,
    
    /// How often this sequence occurs
    pub frequency: u32,
    
    /// Average time for this sequence
    pub average_duration: Duration,
    
    /// Success rate for this sequence
    pub success_rate: f64,
}

/// Indicators of user satisfaction with content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatisfactionIndicators {
    /// Completion rates for interactions
    pub completion_rates: HashMap<InteractionType, f64>,
    
    /// Time spent vs. expected time
    pub time_efficiency: f64,
    
    /// Return rates (how often users come back)
    pub return_rates: f64,
    
    /// Error rates during interactions
    pub error_rates: HashMap<InteractionType, f64>,
    
    /// Explicit feedback if available
    pub explicit_feedback: Vec<FeedbackItem>,
}

/// Individual piece of feedback about content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackItem {
    /// When the feedback was provided
    pub timestamp: SystemTime,
    
    /// Source of the feedback
    pub source: FeedbackSource,
    
    /// Type of feedback
    pub feedback_type: FeedbackType,
    
    /// The actual feedback content
    pub content: String,
    
    /// Rating if numeric feedback was provided
    pub rating: Option<f64>,
}

/// Source of feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackSource {
    Human_User,       // Feedback from human users
    AI_Component,     // Feedback from other AI components
    System_Analytics, // Feedback from system analytics
    Automated_Analysis, // Feedback from automated analysis
}

/// Types of feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    Quality,          // Feedback about quality
    Usefulness,       // Feedback about usefulness
    Clarity,          // Feedback about clarity
    Completeness,     // Feedback about completeness
    Accuracy,         // Feedback about accuracy
    Relevance,        // Feedback about relevance
    Improvement_Suggestion, // Suggestions for improvement
}

/// Performance metrics for content access and usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPerformanceMetrics {
    /// Average time to access content
    pub average_access_time: Duration,
    
    /// Average time to find relevant information within content
    pub average_search_time: Duration,
    
    /// Content loading and processing times
    pub processing_times: ProcessingTimeMetrics,
    
    /// Error rates for content operations
    pub error_rates: ErrorRateMetrics,
    
    /// Scalability metrics as usage increases
    pub scalability_metrics: ScalabilityMetrics,
}

/// Metrics for content processing times
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingTimeMetrics {
    /// Time to load content
    pub load_time: Duration,
    
    /// Time to analyze content
    pub analysis_time: Duration,
    
    /// Time to generate metadata
    pub metadata_generation_time: Duration,
    
    /// Time to establish relationships
    pub relationship_mapping_time: Duration,
}

/// Metrics for error rates in content operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRateMetrics {
    /// Rate of access errors
    pub access_error_rate: f64,
    
    /// Rate of analysis errors
    pub analysis_error_rate: f64,
    
    /// Rate of relationship detection errors
    pub relationship_error_rate: f64,
    
    /// Rate of metadata generation errors
    pub metadata_error_rate: f64,
}

/// Metrics for how well content operations scale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalabilityMetrics {
    /// How performance changes with content size
    pub size_scaling: ScalingCharacteristic,
    
    /// How performance changes with usage frequency
    pub frequency_scaling: ScalingCharacteristic,
    
    /// How performance changes with number of relationships
    pub relationship_scaling: ScalingCharacteristic,
    
    /// How performance changes with analysis depth
    pub depth_scaling: ScalingCharacteristic,
}

/// Characteristics of how something scales
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingCharacteristic {
    /// Type of scaling behavior observed
    pub scaling_type: ScalingType,
    
    /// Measured scaling factor
    pub scaling_factor: f64,
    
    /// Confidence in the scaling measurement
    pub confidence: f64,
    
    /// Range where this scaling behavior applies
    pub applicable_range: (f64, f64),
}

/// Types of scaling behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingType {
    Linear,           // Performance scales linearly
    Logarithmic,      // Performance scales logarithmically
    Exponential,      // Performance scales exponentially
    Polynomial,       // Performance scales polynomially
    Constant,         // Performance is constant
    Unknown,          // Scaling pattern not yet determined
}

/// Metrics about the value and impact of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentValueMetrics {
    /// How much this content contributes to successful outcomes
    pub success_contribution: f64,
    
    /// How unique or irreplaceable this content is
    pub uniqueness_score: f64,
    
    /// How much this content enables other activities
    pub enablement_factor: f64,
    
    /// How much this content has influenced other content or decisions
    pub influence_score: f64,
    
    /// Economic or resource value metrics
    pub resource_value: ResourceValueMetrics,
    
    /// Knowledge value metrics
    pub knowledge_value: KnowledgeValueMetrics,
}

/// Metrics for resource value of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceValueMetrics {
    /// Time saved by having this content available
    pub time_savings: Duration,
    
    /// Effort saved by reusing this content
    pub effort_savings: f64,
    
    /// Cost of recreating this content
    pub recreation_cost: f64,
    
    /// Value generated by using this content
    pub generated_value: f64,
}

/// Metrics for knowledge value of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeValueMetrics {
