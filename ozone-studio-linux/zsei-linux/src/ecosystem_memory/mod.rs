// =============================================================================
// zsei-linux/src/ecosystem_memory/mod.rs
// ZSEI Ecosystem Memory - Wisdom Accumulation and Experience Organization
// 
// This module represents how the OZONE STUDIO ecosystem develops genuine wisdom
// and memory over time, similar to how biological intelligence builds up 
// experiences and learns from them. Every coordination success, challenge, or
// discovery is captured, analyzed, and integrated into growing ecosystem wisdom.
// =============================================================================

use std::collections::{HashMap, BTreeMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::fmt;

// Async runtime and concurrency
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{sleep, timeout, interval, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Mathematical and analysis dependencies
use ndarray::{Array1, Array2, ArrayD};
use nalgebra::{DVector, DMatrix};

// Import shared ecosystem types
use shared_protocols::{
    ComponentType,
    TaskOrchestrationRequest,
    CoordinationStrategy,
    ExecutionStatus,
};

use shared_security::{
    SecurityError,
    AuthenticationCredentials,
};

// Import NEXUS coordination for intelligent storage
use crate::nexus_coordination::{
    NexusCoordinationRequirements,
    ZSEIDirectoryCreationRequest,
    IntelligentStorageCoordination,
    MetadataCoordinationStrategy,
};

// Submodules for ecosystem memory functionality
pub mod memory_coordinator;
pub mod zsei_directory_creator;
pub mod metadata_designer;
pub mod categorization_engine;
pub mod relationship_memory;
pub mod wisdom_organizer;
pub mod experience_analyzer;
pub mod memory_consolidation;
pub mod core_memory_manager;

// Re-export all public types for the ecosystem_memory module
pub use memory_coordinator::{
    MemoryCoordinator,
    MemoryCoordinationEngine,
    MemoryOrchestrator,
    MemoryCoordinationRequest,
    MemoryCoordinationResult,
    MemoryCoordinationError,
};

pub use zsei_directory_creator::{
    ZSEIDirectoryCreator,
    DirectoryCreationEngine,
    DirectoryStructureDesigner,
    MetadataArchitect,
    DirectoryCreationRequest,
    DirectoryCreationResult,
    DirectoryCreationError,
};

pub use metadata_designer::{
    MetadataDesigner,
    MetadataArchitecture,
    MetadataSchema,
    MetadataGenerator,
    MetadataDesignRequest,
    MetadataDesignResult,
    MetadataDesignError,
};

pub use categorization_engine::{
    CategorizationEngine,
    ExperienceClassifier,
    SignificanceAnalyzer,
    CategoryAssigner,
    CategorizationRequest,
    CategorizationResult,
    CategorizationError,
};

pub use relationship_memory::{
    RelationshipMemory,
    RelationshipTracker,
    RelationshipEvolution,
    RelationshipAnalyzer,
    RelationshipMemoryRequest,
    RelationshipMemoryResult,
    RelationshipMemoryError,
};

pub use wisdom_organizer::{
    WisdomOrganizer,
    WisdomAccumulator,
    WisdomSynthesizer,
    WisdomDistributor,
    WisdomOrganizationRequest,
    WisdomOrganizationResult,
    WisdomOrganizationError,
};

pub use experience_analyzer::{
    ExperienceAnalyzer,
    ExperienceExtractor,
    PatternRecognizer,
    LearningIdentifier,
    ExperienceAnalysisRequest,
    ExperienceAnalysisResult,
    ExperienceAnalysisError,
};

pub use memory_consolidation::{
    MemoryConsolidationEngine,
    ExperienceConsolidator,
    WisdomExtractor,
    MemoryOptimizer,
    ConsolidationRequest,
    ConsolidationResult,
    ConsolidationError,
};

pub use core_memory_manager::{
    CoreMemoryManager,
    IdentityMemoryTracker,
    CriticalExperienceIdentifier,
    CoreMemoryPreserver,
    CoreMemoryRequest,
    CoreMemoryResult,
    CoreMemoryError,
};

// =============================================================================
// CORE ECOSYSTEM MEMORY TYPES
// These types represent the fundamental concepts of how the ecosystem remembers
// and learns from its experiences, building wisdom over time.
// =============================================================================

/// Represents a significant experience in the ecosystem's operation that should
/// be remembered and learned from. This is like a "memory" that captures not just
/// what happened, but why it was significant and what can be learned from it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemExperience {
    /// Unique identifier for this experience
    pub experience_id: String,
    
    /// When this experience occurred
    pub timestamp: SystemTime,
    
    /// What type of experience this was (success, failure, discovery, etc.)
    pub experience_type: ExperienceType,
    
    /// How significant this experience was for ecosystem learning
    pub significance: ExperienceSignificance,
    
    /// Which components were involved in this experience
    pub involved_components: Vec<ComponentType>,
    
    /// The context in which this experience occurred
    pub context: ExperienceContext,
    
    /// What actually happened during this experience
    pub experience_data: ExperienceData,
    
    /// What was learned from this experience
    pub learning_outcomes: Vec<LearningOutcome>,
    
    /// How this experience relates to other experiences
    pub relationships: Vec<ExperienceRelationship>,
    
    /// Metadata about this experience for organization
    pub metadata: ExperienceMetadata,
}

/// Different types of experiences the ecosystem can have. This categorization
/// helps the system understand what kind of learning opportunity each experience represents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExperienceType {
    /// A successful coordination that achieved its objectives effectively
    CoordinationSuccess {
        coordination_type: CoordinationType,
        efficiency_level: EfficiencyLevel,
    },
    
    /// A coordination that didn't meet expectations or failed
    CoordinationChallenge {
        challenge_type: ChallengeType,
        recovery_success: bool,
    },
    
    /// Discovery of a new pattern, principle, or approach
    Discovery {
        discovery_type: DiscoveryType,
        novelty_level: NoveltyLevel,
    },
    
    /// Successful human-AGI collaboration
    HumanCollaboration {
        collaboration_type: CollaborationType,
        satisfaction_level: SatisfactionLevel,
    },
    
    /// Learning moment where the ecosystem improved its capabilities
    LearningMoment {
        learning_type: LearningType,
        improvement_magnitude: ImprovementMagnitude,
    },
    
    /// Critical decision point that influenced ecosystem direction
    StrategicDecision {
        decision_type: DecisionType,
        impact_scope: ImpactScope,
    },
    
    /// Relationship development between components or with humans
    RelationshipEvolution {
        relationship_type: RelationshipType,
        evolution_direction: EvolutionDirection,
    },
}

/// How significant an experience was for the ecosystem's development.
/// This determines how prominently it's remembered and how much it influences future decisions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ExperienceSignificance {
    /// Routine experience with minimal learning value
    Routine,
    
    /// Notable experience with some learning value
    Notable,
    
    /// Significant experience that provided important insights
    Significant,
    
    /// Transformative experience that changed how the ecosystem operates
    Transformative,
    
    /// Core-defining experience that shapes ecosystem identity
    CoreDefining,
}

/// The operational context in which an experience occurred. This helps the ecosystem
/// understand when similar situations arise and what approaches might be applicable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceContext {
    /// The task or objective being pursued when this experience occurred
    pub task_context: String,
    
    /// The coordination strategy being used
    pub coordination_strategy: CoordinationStrategy,
    
    /// The complexity level of the situation
    pub complexity_level: ComplexityLevel,
    
    /// Environmental factors that influenced the experience
    pub environmental_factors: Vec<EnvironmentalFactor>,
    
    /// The state of the ecosystem at the time
    pub ecosystem_state: EcosystemState,
    
    /// Any constraints or limitations that were present
    pub constraints: Vec<ContextualConstraint>,
}

/// The actual data captured about what happened during the experience.
/// This is the raw material from which learning and wisdom are extracted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceData {
    /// Detailed description of what occurred
    pub description: String,
    
    /// Quantitative metrics about the experience
    pub metrics: HashMap<String, f64>,
    
    /// Qualitative observations and insights
    pub observations: Vec<String>,
    
    /// Decisions made and their rationales
    pub decisions: Vec<DecisionRecord>,
    
    /// Outcomes achieved (both intended and unintended)
    pub outcomes: Vec<OutcomeRecord>,
    
    /// Any errors or challenges encountered
    pub challenges: Vec<ChallengeRecord>,
    
    /// Resources consumed during the experience
    pub resource_utilization: ResourceUtilization,
}

/// What the ecosystem learned from this experience. This is the wisdom that gets
/// extracted and applied to future situations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningOutcome {
    /// Unique identifier for this learning outcome
    pub outcome_id: String,
    
    /// Type of learning that occurred
    pub learning_type: LearningType,
    
    /// The specific insight or knowledge gained
    pub insight: String,
    
    /// How confident the system is in this learning
    pub confidence_level: f64,
    
    /// How broadly this learning applies
    pub applicability_scope: ApplicabilityScope,
    
    /// Evidence supporting this learning outcome
    pub supporting_evidence: Vec<Evidence>,
    
    /// How this learning should influence future decisions
    pub decision_influence: DecisionInfluence,
}

/// How this experience relates to other experiences in the ecosystem's memory.
/// These relationships help build comprehensive understanding and pattern recognition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceRelationship {
    /// The other experience this relates to
    pub related_experience_id: String,
    
    /// The type of relationship
    pub relationship_type: RelationshipType,
    
    /// How strong this relationship is
    pub relationship_strength: f64,
    
    /// What this relationship reveals about patterns
    pub pattern_insight: Option<String>,
    
    /// How this relationship was discovered
    pub discovery_method: RelationshipDiscoveryMethod,
}

/// Metadata for organizing and retrieving experiences efficiently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceMetadata {
    /// Tags for categorization and search
    pub tags: Vec<String>,
    
    /// Keywords for content search
    pub keywords: Vec<String>,
    
    /// Concepts involved in this experience
    pub concepts: Vec<String>,
    
    /// Domains of knowledge relevant to this experience
    pub knowledge_domains: Vec<KnowledgeDomain>,
    
    /// Search optimization metadata
    pub search_metadata: SearchMetadata,
    
    /// Retention policy for this experience
    pub retention_policy: RetentionPolicy,
}

// =============================================================================
// MEMORY STRUCTURE AND ORGANIZATION TYPES
// These types define how the ecosystem organizes its memories for efficient
// retrieval and pattern recognition.
// =============================================================================

/// The overall structure of how memories are organized in the ecosystem.
/// This is like the "filing system" for the ecosystem's experiences and wisdom.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStructure {
    /// Short-term memories (recent experiences still being processed)
    pub short_term_memory: ShortTermMemory,
    
    /// Long-term memories (consolidated experiences with established patterns)
    pub long_term_memory: LongTermMemory,
    
    /// Core memories that define the ecosystem's identity and values
    pub core_memory: CoreMemory,
    
    /// Working memory for active coordination and processing
    pub working_memory: WorkingMemory,
    
    /// Wisdom repository containing extracted insights and principles
    pub wisdom_repository: WisdomRepository,
    
    /// Relationship graph showing connections between memories
    pub relationship_graph: MemoryRelationshipGraph,
}

/// Short-term memory holds recent experiences that are still being analyzed
/// and haven't yet been consolidated into long-term learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemory {
    /// Recent experiences awaiting analysis
    pub pending_experiences: Vec<EcosystemExperience>,
    
    /// Experiences currently being analyzed
    pub processing_queue: Vec<ExperienceAnalysisTask>,
    
    /// Preliminary patterns detected in recent experiences
    pub emerging_patterns: Vec<EmergingPattern>,
    
    /// Temporary relationships being explored
    pub tentative_relationships: Vec<TentativeRelationship>,
    
    /// Maximum retention time for short-term memories
    pub retention_duration: Duration,
    
    /// Current capacity utilization
    pub capacity_utilization: f64,
}

/// Long-term memory contains consolidated experiences that have been thoroughly
/// analyzed and integrated into the ecosystem's understanding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemory {
    /// Consolidated experiences organized by category
    pub consolidated_experiences: HashMap<MemoryCategory, Vec<ConsolidatedExperience>>,
    
    /// Established patterns with high confidence
    pub established_patterns: Vec<EstablishedPattern>,
    
    /// Confirmed relationships between experiences
    pub stable_relationships: MemoryRelationshipGraph,
    
    /// Accumulated wisdom from consolidated experiences
    pub wisdom_accumulation: Vec<WisdomAccumulation>,
    
    /// Memory organization indices for efficient retrieval
    pub organization_indices: MemoryIndices,
}

/// Core memories are the most fundamental experiences that define the ecosystem's
/// identity, values, and core operating principles. These are never forgotten.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMemory {
    /// Foundational experiences that shaped the ecosystem's identity
    pub identity_defining_experiences: Vec<IdentityDefiningExperience>,
    
    /// Core principles derived from fundamental experiences
    pub core_principles: Vec<CorePrinciple>,
    
    /// Essential relationships that define the ecosystem's nature
    pub essential_relationships: Vec<EssentialRelationship>,
    
    /// Fundamental values discovered through experience
    pub core_values: Vec<CoreValue>,
    
    /// The ecosystem's understanding of its purpose and mission
    pub purpose_understanding: PurposeUnderstanding,
    
    /// Protection mechanisms to preserve core memories
    pub protection_mechanisms: CoreMemoryProtection,
}

/// Working memory holds information actively being used for current coordination
/// and decision-making processes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemory {
    /// Current active contexts for coordination
    pub active_contexts: Vec<ActiveContext>,
    
    /// Recently accessed memories relevant to current operations
    pub recently_accessed: Vec<RecentlyAccessedMemory>,
    
    /// Temporary patterns being explored for current tasks
    pub temporary_patterns: Vec<TemporaryPattern>,
    
    /// Current hypothesis being tested
    pub active_hypotheses: Vec<ActiveHypothesis>,
    
    /// Working memory capacity and utilization
    pub capacity_metrics: WorkingMemoryMetrics,
}

/// Repository of accumulated wisdom extracted from experiences over time.
/// This represents the ecosystem's growing intelligence and understanding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomRepository {
    /// Accumulated wisdom organized by domain
    pub domain_wisdom: HashMap<KnowledgeDomain, DomainWisdom>,
    
    /// Cross-domain insights that apply broadly
    pub universal_insights: Vec<UniversalInsight>,
    
    /// Proven strategies and approaches
    pub proven_strategies: Vec<ProvenStrategy>,
    
    /// Decision-making guidelines derived from experience
    pub decision_guidelines: Vec<DecisionGuideline>,
    
    /// Relationship principles learned over time
    pub relationship_principles: Vec<RelationshipPrinciple>,
    
    /// Wisdom quality metrics and validation
    pub wisdom_quality_metrics: WisdomQualityMetrics,
}

// =============================================================================
// MEMORY CATEGORIZATION AND CLASSIFICATION TYPES
// These types help organize memories by their nature and significance.
// =============================================================================

/// Categories for organizing different types of memories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryCategory {
    /// Memories related to coordination successes and challenges
    CoordinationMemories,
    
    /// Memories of human-AGI collaboration and relationship development
    RelationshipMemories,
    
    /// Memories of learning and capability development
    LearningMemories,
    
    /// Memories of strategic decisions and their outcomes
    StrategicMemories,
    
    /// Memories of discoveries and innovations
    DiscoveryMemories,
    
    /// Memories of challenges overcome and lessons learned
    ChallengeMemories,
    
    /// Memories related to identity and purpose development
    IdentityMemories,
    
    /// Memories of creative problem-solving and innovation
    CreativityMemories,
}

/// How long different types of memories should be retained.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// How long this memory should be kept in short-term memory
    pub short_term_duration: Duration,
    
    /// How long this memory should be kept in long-term memory
    pub long_term_duration: Option<Duration>, // None means permanent
    
    /// Conditions under which this memory should be preserved indefinitely
    pub permanent_preservation_criteria: Vec<PreservationCriterion>,
    
    /// When this memory should be archived rather than actively maintained
    pub archival_criteria: Vec<ArchivalCriterion>,
    
    /// How frequently this memory should be reviewed and validated
    pub review_frequency: ReviewFrequency,
}

/// Criteria for determining if a memory should be permanently preserved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreservationCriterion {
    /// High significance score above threshold
    HighSignificance(f64),
    
    /// Core identity defining experience
    IdentityDefining,
    
    /// Foundational learning with broad applicability
    FoundationalLearning,
    
    /// Unique or rare experience type
    UniqueExperience,
    
    /// High relationship connectivity to other memories
    HighConnectivity(f64),
    
    /// Explicit human request for preservation
    HumanRequested,
    
    /// Critical for ecosystem function
    OperationallyCritical,
}

// =============================================================================
// EXPERIENCE ANALYSIS AND LEARNING TYPES
// These types support the extraction of wisdom and patterns from experiences.
// =============================================================================

/// Different types of learning that can occur from experiences.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LearningType {
    /// Learning about effective coordination strategies
    CoordinationStrategy,
    
    /// Learning about human preferences and collaboration patterns
    HumanCollaboration,
    
    /// Learning about technical capabilities and limitations
    TechnicalCapability,
    
    /// Learning about relationship building and maintenance
    RelationshipDevelopment,
    
    /// Learning about problem-solving approaches
    ProblemSolving,
    
    /// Learning about decision-making processes
    DecisionMaking,
    
    /// Learning about creative and innovative approaches
    CreativeInnovation,
    
    /// Learning about ethical considerations and value alignment
    EthicalReasoning,
    
    /// Meta-learning about how learning itself works
    MetaLearning,
}

/// How broadly a piece of learning applies across different situations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApplicabilityScope {
    /// Applies only to very specific situations
    Specific {
        context_constraints: Vec<String>,
        applicability_conditions: Vec<String>,
    },
    
    /// Applies to a particular domain or type of situation
    DomainSpecific {
        domain: KnowledgeDomain,
        situation_types: Vec<String>,
    },
    
    /// Applies across multiple domains with some constraints
    CrossDomain {
        applicable_domains: Vec<KnowledgeDomain>,
        constraints: Vec<String>,
    },
    
    /// Applies universally across all situations
    Universal {
        confidence_level: f64,
        exceptions: Vec<String>,
    },
}

/// Evidence supporting a learning outcome or conclusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Type of evidence
    pub evidence_type: EvidenceType,
    
    /// The actual evidence data
    pub evidence_data: String,
    
    /// How strong this evidence is
    pub evidence_strength: f64,
    
    /// Source of this evidence
    pub source: EvidenceSource,
    
    /// When this evidence was collected
    pub timestamp: SystemTime,
    
    /// How reliable this evidence is considered
    pub reliability_score: f64,
}

/// Different types of evidence that can support learning outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceType {
    /// Quantitative metrics and measurements
    Quantitative,
    
    /// Qualitative observations and insights
    Qualitative,
    
    /// Comparative analysis with other experiences
    Comparative,
    
    /// Pattern recognition across multiple experiences
    PatternBased,
    
    /// Human feedback and validation
    HumanValidation,
    
    /// Cross-domain correlation
    CrossDomainCorrelation,
    
    /// Logical reasoning and deduction
    LogicalReasoning,
}

// =============================================================================
// WISDOM ACCUMULATION AND ORGANIZATION TYPES
// These types represent how the ecosystem builds up wisdom over time.
// =============================================================================

/// A piece of accumulated wisdom derived from multiple experiences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomAccumulation {
    /// Unique identifier for this wisdom
    pub wisdom_id: String,
    
    /// The insight or principle that constitutes this wisdom
    pub wisdom_content: String,
    
    /// Level of confidence in this wisdom
    pub confidence_level: f64,
    
    /// Experiences that contributed to this wisdom
    pub contributing_experiences: Vec<String>,
    
    /// How this wisdom influences decision-making
    pub decision_influence: DecisionInfluence,
    
    /// Scope of applicability for this wisdom
    pub applicability_scope: ApplicabilityScope,
    
    /// Quality metrics for this wisdom
    pub quality_metrics: WisdomQualityMetrics,
    
    /// When this wisdom was last validated or updated
    pub last_validation: SystemTime,
}

/// How wisdom should influence future decision-making.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionInfluence {
    /// Weight this wisdom should have in decisions
    pub influence_weight: f64,
    
    /// Types of decisions this wisdom applies to
    pub applicable_decision_types: Vec<DecisionType>,
    
    /// Specific guidance this wisdom provides
    pub guidance: String,
    
    /// Constraints or limitations on applying this wisdom
    pub constraints: Vec<String>,
    
    /// When this wisdom should be prioritized over other considerations
    pub prioritization_conditions: Vec<String>,
}

/// Quality metrics for evaluating accumulated wisdom.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomQualityMetrics {
    /// How accurate this wisdom has proven to be
    pub accuracy_score: f64,
    
    /// How useful this wisdom has been in practice
    pub utility_score: f64,
    
    /// How well this wisdom generalizes across situations
    pub generalizability_score: f64,
    
    /// How robust this wisdom is to changing conditions
    pub robustness_score: f64,
    
    /// Overall quality score for this wisdom
    pub overall_quality_score: f64,
    
    /// Number of times this wisdom has been successfully applied
    pub successful_applications: u64,
    
    /// Number of times this wisdom has failed or been inadequate
    pub failed_applications: u64,
}

// =============================================================================
// RELATIONSHIP AND CONNECTION TYPES
// These types represent how memories and experiences relate to each other.
// =============================================================================

/// Types of relationships between different elements in memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    /// Causal relationship - one led to the other
    Causal {
        causal_strength: CausalStrength,
        causal_direction: CausalDirection,
    },
    
    /// Temporal relationship - occurred in sequence
    Temporal {
        temporal_proximity: TemporalProximity,
        sequence_type: SequenceType,
    },
    
    /// Conceptual similarity - share common concepts or patterns
    ConceptualSimilarity {
        similarity_score: f64,
        shared_concepts: Vec<String>,
    },
    
    /// Functional relationship - serve similar purposes
    Functional {
        function_similarity: f64,
        shared_functions: Vec<String>,
    },
    
    /// Hierarchical relationship - one is part of or contains the other
    Hierarchical {
        hierarchy_type: HierarchyType,
        level_difference: i32,
    },
    
    /// Complementary relationship - work together to achieve goals
    Complementary {
        synergy_score: f64,
        complementary_aspects: Vec<String>,
    },
    
    /// Conflicting relationship - represent opposing approaches or outcomes
    Conflicting {
        conflict_intensity: f64,
        conflict_aspects: Vec<String>,
    },
}

/// Graph structure representing relationships between memories and experiences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRelationshipGraph {
    /// Nodes in the graph (experiences, memories, wisdom)
    pub nodes: HashMap<String, RelationshipNode>,
    
    /// Edges representing relationships between nodes
    pub edges: Vec<RelationshipEdge>,
    
    /// Clusters of highly related nodes
    pub clusters: Vec<RelationshipCluster>,
    
    /// Centrality measures for important nodes
    pub centrality_measures: HashMap<String, CentralityMeasures>,
    
    /// Graph statistics and metrics
    pub graph_metrics: GraphMetrics,
}

/// A node in the relationship graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipNode {
    /// Unique identifier for this node
    pub node_id: String,
    
    /// Type of node (experience, memory, wisdom, etc.)
    pub node_type: NodeType,
    
    /// Content or data associated with this node
    pub content: NodeContent,
    
    /// Metadata for this node
    pub metadata: NodeMetadata,
    
    /// Importance score for this node
    pub importance_score: f64,
}

// =============================================================================
// ERROR TYPES FOR ECOSYSTEM MEMORY
// =============================================================================

/// Comprehensive error types for ecosystem memory operations.
#[derive(Error, Debug)]
pub enum EcosystemMemoryError {
    #[error("Memory coordination error: {operation} - {details}")]
    MemoryCoordinationError { operation: String, details: String },
    
    #[error("Experience categorization error: {experience_id} - {details}")]
    CategorizationError { experience_id: String, details: String },
    
    #[error("Memory consolidation error: {memory_type} - {details}")]
    ConsolidationError { memory_type: String, details: String },
    
    #[error("Wisdom organization error: {wisdom_id} - {details}")]
    WisdomOrganizationError { wisdom_id: String, details: String },
    
    #[error("Relationship analysis error: {relationship_type} - {details}")]
    RelationshipAnalysisError { relationship_type: String, details: String },
    
    #[error("Memory retrieval error: {query} - {details}")]
    MemoryRetrievalError { query: String, details: String },
    
    #[error("Memory storage error: {storage_location} - {details}")]
    MemoryStorageError { storage_location: String, details: String },
    
    #[error("Memory validation error: {validation_type} - {details}")]
    MemoryValidationError { validation_type: String, details: String },
    
    #[error("Configuration error: {component} - {details}")]
    ConfigurationError { component: String, details: String },
}

// =============================================================================
// SUPPORTING ENUMS AND UTILITY TYPES
// =============================================================================

// Experience classification enums
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CoordinationType {
    TaskOrchestration,
    ResourceAllocation,
    ConflictResolution,
    CapabilityIntegration,
    QualityAssurance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EfficiencyLevel {
    Poor,
    Adequate,
    Good,
    Excellent,
    Exceptional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChallengeType {
    ResourceConstraint,
    CommunicationFailure,
    QualityShortfall,
    TimeoutExceeded,
    UnexpectedError,
    RequirementMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DiscoveryType {
    NewPattern,
    ImprovedStrategy,
    UniversalPrinciple,
    OptimizationOpportunity,
    RelationshipInsight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NoveltyLevel {
    Incremental,
    Moderate,
    Significant,
    Breakthrough,
    Paradigm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CollaborationType {
    ProblemSolving,
    CreativeCollaboration,
    LearningPartnership,
    StrategicPlanning,
    QualityAssurance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SatisfactionLevel {
    Poor,
    Fair,
    Good,
    VeryGood,
    Excellent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ImprovementMagnitude {
    Minor,
    Moderate,
    Significant,
    Major,
    Transformational,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DecisionType {
    Strategic,
    Tactical,
    Operational,
    Ethical,
    Creative,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ImpactScope {
    Local,
    Regional,
    Global,
    Ecosystem,
    Universal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EvolutionDirection {
    Strengthening,
    Deepening,
    Expanding,
    Transforming,
    Declining,
}

// Additional supporting types for comprehensive functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityLevel {
    pub level: String,
    pub complexity_score: f64,
    pub complexity_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalFactor {
    pub factor_type: String,
    pub impact_level: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemState {
    pub overall_health: f64,
    pub resource_utilization: f64,
    pub coordination_efficiency: f64,
    pub active_components: Vec<ComponentType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualConstraint {
    pub constraint_type: String,
    pub severity: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub decision_id: String,
    pub decision_description: String,
    pub decision_rationale: String,
    pub decision_maker: String,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeRecord {
    pub outcome_id: String,
    pub outcome_description: String,
    pub outcome_type: OutcomeType,
    pub success_level: f64,
    pub unexpected_aspects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OutcomeType {
    Intended,
    Unintended,
    Emergent,
    SideEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeRecord {
    pub challenge_id: String,
    pub challenge_description: String,
    pub challenge_type: ChallengeType,
    pub resolution_approach: Option<String>,
    pub resolution_success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub network_usage: f64,
    pub storage_usage: f64,
    pub coordination_overhead: f64,
}

// Knowledge domain classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum KnowledgeDomain {
    Biology,
    Mathematics,
    Physics,
    Psychology,
    Engineering,
    ComputerScience,
    Philosophy,
    Ethics,
    Sociology,
    Economics,
    Linguistics,
    Neuroscience,
    SystemsTheory,
    InformationTheory,
    Other(String),
}

// Memory organization and indexing types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryIndices {
    pub temporal_index: TemporalIndex,
    pub categorical_index: CategoricalIndex,
    pub significance_index: SignificanceIndex,
    pub relationship_index: RelationshipIndex,
    pub content_index: ContentIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalIndex {
    pub time_buckets: BTreeMap<SystemTime, Vec<String>>,
    pub temporal_patterns: Vec<TemporalPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoricalIndex {
    pub category_mappings: HashMap<MemoryCategory, Vec<String>>,
    pub cross_category_relationships: Vec<CrossCategoryRelationship>,
}

// Result types for ecosystem memory operations
pub type EcosystemMemoryResult<T> = Result<T, EcosystemMemoryError>;

// =============================================================================
// CORE TRAITS FOR ECOSYSTEM MEMORY FUNCTIONALITY
// =============================================================================

/// Core trait for managing ecosystem memory and learning
pub trait EcosystemMemoryManager {
    /// Store a new experience in the ecosystem's memory
    fn store_experience(&mut self, experience: EcosystemExperience) -> EcosystemMemoryResult<String>;
    
    /// Retrieve experiences matching specific criteria
    fn retrieve_experiences(&self, query: MemoryQuery) -> EcosystemMemoryResult<Vec<EcosystemExperience>>;
    
    /// Consolidate short-term memories into long-term learning
    fn consolidate_memories(&mut self) -> EcosystemMemoryResult<ConsolidationResult>;
    
    /// Extract wisdom from accumulated experiences
    fn extract_wisdom(&mut self, domain: Option<KnowledgeDomain>) -> EcosystemMemoryResult<Vec<WisdomAccumulation>>;
    
    /// Analyze relationships between different memories
    fn analyze_relationships(&self, memory_ids: Vec<String>) -> EcosystemMemoryResult<Vec<ExperienceRelationship>>;
}

/// Trait for categorizing and organizing experiences
pub trait ExperienceCategorizer {
    /// Classify an experience into appropriate categories
    fn categorize_experience(&self, experience: &EcosystemExperience) -> EcosystemMemoryResult<Vec<MemoryCategory>>;
    
    /// Assess the significance of an experience
    fn assess_significance(&self, experience: &EcosystemExperience) -> EcosystemMemoryResult<ExperienceSignificance>;
    
    /// Extract learning outcomes from an experience
    fn extract_learning(&self, experience: &EcosystemExperience) -> EcosystemMemoryResult<Vec<LearningOutcome>>;
}

/// Trait for wisdom accumulation and organization
pub trait WisdomAccumulator {
    /// Synthesize wisdom from multiple related experiences
    fn synthesize_wisdom(&self, experiences: Vec<EcosystemExperience>) -> EcosystemMemoryResult<WisdomAccumulation>;
    
    /// Organize wisdom by domain and applicability
    fn organize_wisdom(&mut self, wisdom: Vec<WisdomAccumulation>) -> EcosystemMemoryResult<WisdomRepository>;
    
    /// Validate and quality-check accumulated wisdom
    fn validate_wisdom(&self, wisdom: &WisdomAccumulation) -> EcosystemMemoryResult<WisdomQualityMetrics>;
}

// =============================================================================
// MEMORY QUERY AND RETRIEVAL TYPES
// =============================================================================

/// Query structure for retrieving memories and experiences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Text search across memory content
    pub text_query: Option<String>,
    
    /// Filter by experience types
    pub experience_types: Vec<ExperienceType>,
    
    /// Filter by memory categories
    pub categories: Vec<MemoryCategory>,
    
    /// Filter by significance level
    pub min_significance: Option<ExperienceSignificance>,
    
    /// Filter by time range
    pub time_range: Option<TimeRange>,
    
    /// Filter by involved components
    pub components: Vec<ComponentType>,
    
    /// Filter by knowledge domains
    pub domains: Vec<KnowledgeDomain>,
    
    /// Sort criteria
    pub sort_by: SortCriteria,
    
    /// Maximum number of results
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: SystemTime,
    pub end: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortCriteria {
    Timestamp,
    Significance,
    Relevance,
    RelationshipStrength,
}

// Constants and defaults for ecosystem memory
pub const DEFAULT_SHORT_TERM_RETENTION: Duration = Duration::from_secs(7 * 24 * 3600); // 1 week
pub const DEFAULT_SIGNIFICANCE_THRESHOLD: f64 = 0.7;
pub const DEFAULT_RELATIONSHIP_STRENGTH_THRESHOLD: f64 = 0.5;
pub const MAX_WORKING_MEMORY_SIZE: usize = 100;
pub const WISDOM_CONFIDENCE_THRESHOLD: f64 = 0.8;
