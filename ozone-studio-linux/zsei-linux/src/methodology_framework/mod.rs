// =============================================================================
// zsei-linux/src/methodology_framework/mod.rs
// ZSEI Methodology Framework - Living Learned Patterns and Experience-Based Intelligence
// =============================================================================

//! # Methodology Framework Module
//! 
//! This module represents the heart of ZSEI's learning capabilities. Think of it as the "wisdom brain"
//! that observes how methodologies perform in practice, extracts patterns from successful outcomes,
//! learns from failures, and continuously evolves better approaches.
//! 
//! Unlike traditional AI that learns through training data, this system learns through accumulated
//! real-world experience with methodology execution. Every time a methodology runs, this framework
//! analyzes the outcome, extracts reusable patterns, and integrates new wisdom into the ecosystem's
//! knowledge base.
//! 
//! ## Core Learning Philosophy
//! 
//! The methodology framework operates on the principle that intelligence emerges from experience
//! rather than algorithms. When FORGE successfully uses the Five-Pass Code Methodology, this
//! framework doesn't just record "it worked" - it analyzes WHY it worked, WHAT patterns made it
//! effective, and HOW those patterns can be applied to other scenarios.
//! 
//! ## Pattern Recognition and Wisdom Extraction
//! 
//! This system recognizes that methodologies are more than instructions - they're crystallized
//! wisdom about effective coordination patterns. The framework extracts the underlying principles
//! that make methodologies successful and stores them as "living learned patterns" that can
//! evolve and improve over time.

use std::collections::{HashMap, BTreeMap, HashSet};
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use std::fmt;

// Async runtime and concurrency
use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Mathematical analysis and pattern recognition
use nalgebra::{DVector, DMatrix};
use ndarray::{Array1, Array2};

// Import shared types
use shared_protocols::{
    ComponentType,
    ExecutionStatus,
    TaskOrchestrationRequest,
};

use methodology_runtime::{
    Methodology,
    MethodologyMetadata,
    ExecutionResult,
    MethodologyCategory,
    DifficultyLevel,
};

// Sub-modules that implement different aspects of methodology learning
pub mod pattern_storage;           // Stores and organizes learned patterns
pub mod scenario_analyzer;         // Analyzes execution scenarios for patterns
pub mod pattern_extractor;         // Extracts reusable patterns from experiences
pub mod learning_engine;           // Core learning and adaptation logic
pub mod recognition_system;        // Recognizes when to apply learned patterns
pub mod wisdom_integrator;         // Integrates new wisdom into existing knowledge

// Import all the pattern analysis and learning components
pub use pattern_storage::{
    PatternStorage,
    PatternDatabase,
    PatternIndex,
    PatternQuery,
    StorageMetrics,
};

pub use scenario_analyzer::{
    ScenarioAnalyzer,
    ExecutionScenario,
    ScenarioClassification,
    ContextualFactors,
    EnvironmentalConditions,
    ScenarioComplexity,
};

pub use pattern_extractor::{
    PatternExtractor,
    ExtractionAlgorithm,
    PatternCandidate,
    ExtractionCriteria,
    SignificanceAnalysis,
    PatternValidation,
};

pub use learning_engine::{
    LearningEngine,
    LearningAlgorithm,
    AdaptationStrategy,
    PerformanceMetrics,
    LearningProgress,
    CapabilityEvolution,
};

pub use recognition_system::{
    RecognitionSystem,
    PatternMatcher,
    SimilarityCalculator,
    MatchingCriteria,
    RecognitionAccuracy,
    ApplicationRecommendation,
};

pub use wisdom_integrator::{
    WisdomIntegrator,
    IntegrationStrategy,
    WisdomSynthesis,
    KnowledgeConsolidation,
    WisdomValidation,
    EvolutionaryLearning,
};

// =============================================================================
// Core Pattern Types - The Building Blocks of Learned Wisdom
// =============================================================================

/// A MethodologyPattern represents a crystallized piece of wisdom extracted from successful
/// methodology executions. Think of it as a "lesson learned" that captures not just what
/// worked, but why it worked and when to apply it again.
/// 
/// For example, if FORGE consistently succeeds by analyzing dependencies before architecture,
/// that becomes a pattern that can be applied to other code analysis scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyPattern {
    /// Unique identifier for this pattern - allows tracking across evolutions
    pub pattern_id: String,
    
    /// Human-readable name that captures the essence of this pattern
    pub pattern_name: String,
    
    /// Detailed description of what this pattern represents
    pub description: String,
    
    /// What category of pattern this represents (success, failure, optimization, etc.)
    pub category: PatternCategory,
    
    /// The core approach or principle that this pattern embodies
    pub core_approach: String,
    
    /// Specific conditions where this pattern tends to be effective
    pub applicability_conditions: Vec<ApplicabilityCondition>,
    
    /// The methodology steps or coordination sequence that implements this pattern
    pub implementation_steps: Vec<PatternStep>,
    
    /// Measurable outcomes that indicate this pattern is working effectively
    pub success_indicators: Vec<SuccessIndicator>,
    
    /// Known scenarios where this pattern should be avoided
    pub contraindications: Vec<Contraindication>,
    
    /// Statistical data about this pattern's effectiveness across different scenarios
    pub effectiveness_metrics: EffectivenessMetrics,
    
    /// How confident we are in this pattern based on observation frequency and success rate
    pub confidence_level: PatternConfidence,
    
    /// How similar this pattern is to other known patterns (for relationship mapping)
    pub similarity_metrics: HashMap<String, PatternSimilarity>,
    
    /// The experiences and executions that contributed to discovering this pattern
    pub source_experiences: Vec<SourceExperience>,
    
    /// When this pattern was first discovered and last updated
    pub temporal_data: PatternTemporalData,
    
    /// Cross-domain insights that influenced this pattern's development
    pub cross_domain_influences: Vec<CrossDomainInfluence>,
}

/// Categories that help organize and understand different types of patterns
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PatternCategory {
    /// Patterns that consistently lead to successful outcomes
    SuccessPattern,
    
    /// Patterns that lead to failure and should be avoided
    FailurePattern,
    
    /// Patterns that optimize performance or efficiency
    OptimizationPattern,
    
    /// Patterns for coordinating between multiple AI Apps
    CoordinationPattern,
    
    /// Patterns for handling edge cases or unusual scenarios
    EdgeCasePattern,
    
    /// Patterns for recovering from errors or unexpected situations
    RecoveryPattern,
    
    /// Patterns for human-AI collaboration and interaction
    CollaborationPattern,
    
    /// Patterns that apply across multiple domains or methodologies
    UniversalPattern,
    
    /// Experimental patterns that show promise but need more validation
    ExperimentalPattern,
}

/// Specific conditions that indicate when a pattern should be applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicabilityCondition {
    /// Description of the condition
    pub condition_description: String,
    
    /// Type of condition (environmental, complexity-based, resource-based, etc.)
    pub condition_type: ConditionType,
    
    /// Specific criteria that must be met
    pub criteria: Vec<String>,
    
    /// How critical this condition is for pattern success
    pub importance_weight: f64,
    
    /// Statistical confidence in this condition's relevance
    pub validation_confidence: f64,
}

/// Types of conditions that affect pattern applicability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    /// Conditions related to problem complexity
    ComplexityBased,
    
    /// Conditions related to available resources
    ResourceBased,
    
    /// Conditions related to time constraints
    TemporalBased,
    
    /// Conditions related to specific AI App capabilities
    CapabilityBased,
    
    /// Conditions related to human involvement level
    CollaborationBased,
    
    /// Conditions related to domain-specific factors
    DomainSpecific,
    
    /// Conditions related to quality requirements
    QualityBased,
    
    /// Conditions related to risk tolerance
    RiskBased,
}

/// Individual steps that implement a pattern's approach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStep {
    /// Sequential order of this step
    pub step_number: u32,
    
    /// Description of what happens in this step
    pub step_description: String,
    
    /// Which AI App or component is responsible for this step
    pub responsible_component: ComponentType,
    
    /// Specific actions to be taken
    pub actions: Vec<String>,
    
    /// Expected outcomes from this step
    pub expected_outcomes: Vec<String>,
    
    /// How to validate that this step completed successfully
    pub validation_criteria: Vec<String>,
    
    /// Dependencies on previous steps
    pub dependencies: Vec<u32>,
    
    /// Whether this step can be performed in parallel with others
    pub parallelizable: bool,
    
    /// Estimated time for this step
    pub estimated_duration: Option<Duration>,
}

/// Indicators that show when a pattern is working effectively
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessIndicator {
    /// Description of what to measure
    pub indicator_description: String,
    
    /// Type of measurement (quantitative, qualitative, boolean)
    pub measurement_type: MeasurementType,
    
    /// Threshold values that indicate success
    pub success_thresholds: Vec<Threshold>,
    
    /// How to collect this measurement
    pub measurement_method: String,
    
    /// How critical this indicator is for overall success
    pub importance_weight: f64,
}

/// Types of measurements for success indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeasurementType {
    /// Numeric measurements with specific target values
    Quantitative { unit: String, target_value: f64 },
    
    /// Qualitative assessments with categorical ratings
    Qualitative { rating_scale: Vec<String> },
    
    /// Simple pass/fail or yes/no measurements
    Boolean { success_condition: String },
    
    /// Measurements relative to baseline or comparison
    Comparative { baseline: f64, improvement_target: f64 },
}

/// Threshold definitions for success measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threshold {
    /// Description of what this threshold represents
    pub threshold_name: String,
    
    /// The actual threshold value
    pub threshold_value: f64,
    
    /// Whether exceeding this threshold indicates success or failure
    pub threshold_type: ThresholdType,
    
    /// How confident we are in this threshold's validity
    pub confidence: f64,
}

/// Types of thresholds for success measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdType {
    /// Success requires value to be above this threshold
    Minimum,
    
    /// Success requires value to be below this threshold
    Maximum,
    
    /// Success requires value to be close to this target
    Target { tolerance: f64 },
    
    /// Success requires value to be within this range
    Range { min: f64, max: f64 },
}

/// Situations where a pattern should not be applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contraindication {
    /// Description of the problematic situation
    pub situation_description: String,
    
    /// Why this pattern is problematic in this situation
    pub risk_explanation: String,
    
    /// Severity of the risk if pattern is applied incorrectly
    pub risk_severity: RiskSeverity,
    
    /// Alternative approaches to consider instead
    pub alternative_recommendations: Vec<String>,
    
    /// Statistical evidence supporting this contraindication
    pub evidence_strength: f64,
}

/// Severity levels for contraindication risks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskSeverity {
    /// Minor inefficiency or suboptimal results
    Low,
    
    /// Noticeable negative impact on outcomes
    Medium,
    
    /// Significant problems or failures likely
    High,
    
    /// Severe negative consequences almost certain
    Critical,
}

// =============================================================================
// Pattern Effectiveness and Confidence Metrics
// =============================================================================

/// Comprehensive metrics about how effective a pattern has been in practice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessMetrics {
    /// Total number of times this pattern has been applied
    pub application_count: u64,
    
    /// Number of times application led to successful outcomes
    pub success_count: u64,
    
    /// Simple success rate (success_count / application_count)
    pub success_rate: f64,
    
    /// Success rate weighted by outcome quality
    pub weighted_success_rate: f64,
    
    /// Average improvement in outcomes when pattern is applied vs baseline
    pub average_improvement: f64,
    
    /// Consistency of results (low variance indicates reliable pattern)
    pub result_consistency: f64,
    
    /// How much this pattern improves efficiency
    pub efficiency_impact: f64,
    
    /// How much this pattern improves quality
    pub quality_impact: f64,
    
    /// Time-based trends in effectiveness (improving, declining, stable)
    pub trend_analysis: TrendAnalysis,
    
    /// Effectiveness in different contexts or scenarios
    pub contextual_effectiveness: HashMap<String, f64>,
    
    /// Statistical confidence intervals for the metrics
    pub confidence_intervals: ConfidenceIntervals,
}

/// Analysis of how pattern effectiveness changes over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Whether effectiveness is improving, declining, or stable
    pub trend_direction: TrendDirection,
    
    /// Rate of change in effectiveness
    pub trend_magnitude: f64,
    
    /// Statistical confidence in the trend
    pub trend_confidence: f64,
    
    /// Factors that might explain the trend
    pub contributing_factors: Vec<String>,
    
    /// Predictions about future effectiveness
    pub future_projections: Vec<EffectivenessProjection>,
}

/// Direction of effectiveness trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    /// Pattern becoming more effective over time
    Improving,
    
    /// Pattern becoming less effective over time
    Declining,
    
    /// Pattern effectiveness remaining consistent
    Stable,
    
    /// Pattern effectiveness varies cyclically
    Cyclical,
    
    /// Insufficient data to determine trend
    Unknown,
}

/// Projected future effectiveness under different scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessProjection {
    /// Timeframe for this projection
    pub projection_timeframe: Duration,
    
    /// Scenario conditions for this projection
    pub scenario_conditions: String,
    
    /// Projected effectiveness value
    pub projected_effectiveness: f64,
    
    /// Confidence in this projection
    pub projection_confidence: f64,
}

/// Statistical confidence intervals for effectiveness metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceIntervals {
    /// Confidence level (e.g., 95% confidence)
    pub confidence_level: f64,
    
    /// Confidence interval for success rate
    pub success_rate_interval: (f64, f64),
    
    /// Confidence interval for improvement impact
    pub improvement_interval: (f64, f64),
    
    /// Confidence interval for efficiency impact
    pub efficiency_interval: (f64, f64),
    
    /// Confidence interval for quality impact
    pub quality_interval: (f64, f64),
}

/// Overall confidence level in a pattern based on evidence and validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum PatternConfidence {
    /// Very limited evidence, experimental only
    Experimental,
    
    /// Some evidence but needs more validation
    Low,
    
    /// Moderate evidence with consistent results
    Medium,
    
    /// Strong evidence across multiple scenarios
    High,
    
    /// Overwhelming evidence, highly reliable
    VeryHigh,
    
    /// Validated across all applicable scenarios
    Definitive,
}

/// Similarity relationship between patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSimilarity {
    /// ID of the related pattern
    pub related_pattern_id: String,
    
    /// Similarity score (0.0 to 1.0)
    pub similarity_score: f64,
    
    /// Type of similarity relationship
    pub similarity_type: SimilarityType,
    
    /// Aspects where patterns are similar
    pub similar_aspects: Vec<String>,
    
    /// Aspects where patterns differ
    pub different_aspects: Vec<String>,
    
    /// Whether these patterns can be combined or are mutually exclusive
    pub compatibility: PatternCompatibility,
}

/// Types of similarity between patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimilarityType {
    /// Similar in the approach or steps taken
    Structural,
    
    /// Similar in the outcomes achieved
    Functional,
    
    /// Similar in when/where they apply
    Contextual,
    
    /// Similar in underlying principles
    Conceptual,
    
    /// Similar in coordination patterns
    Coordination,
    
    /// Similar in problem-solving approach
    Strategic,
}

/// How patterns can be combined or relate to each other
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternCompatibility {
    /// Patterns work well together and can be combined
    Complementary,
    
    /// Patterns can be used sequentially
    Sequential,
    
    /// Patterns cannot be used together
    MutuallyExclusive,
    
    /// One pattern is a specialized version of another
    Specialized,
    
    /// One pattern evolved from another
    Evolutionary,
    
    /// Patterns are independent but not conflicting
    Independent,
}

// =============================================================================
// Learning Outcomes and Wisdom Integration Types
// =============================================================================

/// A LearningOutcome represents what the methodology framework has learned from
/// observing methodology executions. It captures insights that improve future
/// methodology development and application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningOutcome {
    /// Unique identifier for this learning outcome
    pub outcome_id: String,
    
    /// Human-readable summary of what was learned
    pub learning_summary: String,
    
    /// Detailed description of the learning
    pub detailed_insights: String,
    
    /// Type of learning that occurred
    pub learning_type: LearningType,
    
    /// The methodology executions that led to this learning
    pub source_executions: Vec<ExecutionReference>,
    
    /// Patterns that were discovered or validated through this learning
    pub discovered_patterns: Vec<String>,
    
    /// Changes to existing patterns based on this learning
    pub pattern_modifications: Vec<PatternModification>,
    
    /// New capabilities or improvements gained from this learning
    pub capability_enhancements: Vec<CapabilityEnhancement>,
    
    /// Confidence in the validity of this learning
    pub learning_confidence: LearningConfidence,
    
    /// How this learning should be applied in future scenarios
    pub application_guidance: Vec<ApplicationGuidance>,
    
    /// Relationships to other learning outcomes
    pub related_learnings: Vec<LearningRelationship>,
    
    /// When this learning occurred and its evolution over time
    pub temporal_development: LearningTemporalData,
}

/// Types of learning that can occur from methodology observation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningType {
    /// Learning about successful coordination patterns
    CoordinationImprovement,
    
    /// Learning about more efficient approaches
    EfficiencyOptimization,
    
    /// Learning about quality enhancement techniques
    QualityImprovement,
    
    /// Learning about problem-solving strategies
    StrategicInsight,
    
    /// Learning about failure patterns to avoid
    FailureAvoidance,
    
    /// Learning about cross-domain applications
    CrossDomainApplication,
    
    /// Learning about human-AI collaboration patterns
    CollaborationEnhancement,
    
    /// Learning about methodology evolution and adaptation
    AdaptiveCapability,
    
    /// Learning about edge case handling
    EdgeCaseManagement,
    
    /// Learning about resource optimization
    ResourceOptimization,
}

/// Reference to a methodology execution that contributed to learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReference {
    /// ID of the methodology execution
    pub execution_id: String,
    
    /// Which methodology was executed
    pub methodology_id: String,
    
    /// The AI Apps involved in execution
    pub involved_components: Vec<ComponentType>,
    
    /// Execution context and parameters
    pub execution_context: String,
    
    /// Outcome of the execution
    pub execution_outcome: ExecutionOutcome,
    
    /// Specific aspects that contributed to learning
    pub learning_contributions: Vec<String>,
    
    /// Weight of this execution's contribution to the overall learning
    pub contribution_weight: f64,
}

/// Outcome classification for methodology executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOutcome {
    /// Execution completed successfully with good results
    Success { quality_score: f64 },
    
    /// Execution completed but with suboptimal results
    PartialSuccess { achieved_objectives: Vec<String>, missed_objectives: Vec<String> },
    
    /// Execution failed but provided valuable learning about what doesn't work
    InformativeFailure { failure_reasons: Vec<String>, lessons_learned: Vec<String> },
    
    /// Execution failed without clear learning value
    UnproductiveFailure { failure_reasons: Vec<String> },
    
    /// Execution was interrupted or cancelled
    Interrupted { interruption_reason: String, progress_made: f64 },
}

/// Modifications made to existing patterns based on new learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternModification {
    /// ID of the pattern being modified
    pub pattern_id: String,
    
    /// Type of modification being made
    pub modification_type: ModificationType,
    
    /// Description of what changed
    pub change_description: String,
    
    /// Why this modification was necessary
    pub justification: String,
    
    /// Expected impact of the modification
    pub expected_impact: String,
    
    /// Confidence in the value of this modification
    pub modification_confidence: f64,
}

/// Types of modifications that can be made to patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationType {
    /// Refining the conditions where pattern applies
    ApplicabilityRefinement,
    
    /// Improving the implementation steps
    ImplementationImprovement,
    
    /// Updating success indicators or thresholds
    SuccessMetricsUpdate,
    
    /// Adding new contraindications
    ContraindicationAddition,
    
    /// Improving effectiveness through optimization
    EffectivenessOptimization,
    
    /// Expanding pattern to new contexts
    ContextualExpansion,
    
    /// Combining with other patterns for better results
    PatternIntegration,
}

/// New capabilities gained through learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityEnhancement {
    /// Description of the new capability
    pub capability_description: String,
    
    /// Type of capability enhancement
    pub enhancement_type: EnhancementType,
    
    /// Which components benefit from this enhancement
    pub beneficiary_components: Vec<ComponentType>,
    
    /// Measurable improvements enabled by this capability
    pub measurable_improvements: Vec<String>,
    
    /// How to validate that this capability is working
    pub validation_criteria: Vec<String>,
    
    /// Timeline for implementing this enhancement
    pub implementation_timeline: Option<Duration>,
}

/// Types of capability enhancements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnhancementType {
    /// Improved coordination between AI Apps
    CoordinationEnhancement,
    
    /// More efficient resource utilization
    EfficiencyEnhancement,
    
    /// Higher quality outcomes
    QualityEnhancement,
    
    /// Better handling of complex scenarios
    ComplexityHandling,
    
    /// Improved error recovery
    ResilienceEnhancement,
    
    /// Better human-AI collaboration
    CollaborationImprovement,
    
    /// Enhanced cross-domain capabilities
    CrossDomainExpansion,
    
    /// Better adaptation to new scenarios
    AdaptabilityImprovement,
}

/// Confidence level in learning outcomes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum LearningConfidence {
    /// Single observation, may be coincidental
    Tentative,
    
    /// Multiple observations but limited contexts
    Emerging,
    
    /// Consistent observations across some contexts
    Moderate,
    
    /// Strong evidence across multiple contexts
    Strong,
    
    /// Overwhelming evidence across all applicable contexts
    Definitive,
}

/// Guidance for applying learning in future scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationGuidance {
    /// When this learning should be applied
    pub application_context: String,
    
    /// Specific recommendations for application
    pub recommendations: Vec<String>,
    
    /// Expected benefits from applying this learning
    pub expected_benefits: Vec<String>,
    
    /// Risks or considerations when applying
    pub application_risks: Vec<String>,
    
    /// How to measure success when applying this learning
    pub success_metrics: Vec<String>,
}

/// Relationships between different learning outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningRelationship {
    /// ID of the related learning outcome
    pub related_learning_id: String,
    
    /// Type of relationship
    pub relationship_type: LearningRelationshipType,
    
    /// Description of how the learnings relate
    pub relationship_description: String,
    
    /// Strength of the relationship
    pub relationship_strength: f64,
}

/// Types of relationships between learning outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningRelationshipType {
    /// One learning builds upon another
    BuildsUpon,
    
    /// Learnings reinforce each other
    MutuallyReinforcing,
    
    /// One learning contradicts another (requires resolution)
    Contradictory,
    
    /// Learnings complement each other
    Complementary,
    
    /// One learning is a specialized case of another
    Specialization,
    
    /// Learnings represent different perspectives on same phenomenon
    AlternativePerspective,
}

// =============================================================================
// Wisdom Accumulation and Knowledge Integration
// =============================================================================

/// WisdomAccumulation represents the ecosystem's growing understanding and
/// capability that emerges from accumulated learning experiences. This is where
/// individual patterns and learnings integrate into higher-level insights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomAccumulation {
    /// Unique identifier for this wisdom accumulation
    pub wisdom_id: String,
    
    /// Human-readable description of this accumulated wisdom
    pub wisdom_description: String,
    
    /// The level of wisdom this represents
    pub wisdom_level: WisdomLevel,
    
    /// Core principles that emerge from accumulated experience
    pub core_principles: Vec<CorePrinciple>,
    
    /// Meta-patterns that appear across multiple individual patterns
    pub meta_patterns: Vec<MetaPattern>,
    
    /// Strategic insights about methodology development and application
    pub strategic_insights: Vec<StrategicInsight>,
    
    /// Cross-domain wisdom that applies across different problem types
    pub cross_domain_wisdom: Vec<CrossDomainWisdom>,
    
    /// Predictive insights about likely future developments
    pub predictive_insights: Vec<PredictiveInsight>,
    
    /// Learning outcomes that contributed to this wisdom
    pub contributing_learnings: Vec<String>,
    
    /// Patterns that validate or exemplify this wisdom
    pub supporting_patterns: Vec<String>,
    
    /// Confidence in this accumulated wisdom
    pub wisdom_confidence: WisdomConfidence,
    
    /// How this wisdom should influence future methodology development
    pub developmental_guidance: Vec<DevelopmentalGuidance>,
    
    /// Evolution of this wisdom over time
    pub wisdom_evolution: WisdomEvolution,
}

/// Levels of wisdom accumulation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum WisdomLevel {
    /// Basic understanding from limited experience
    Novice,
    
    /// Growing understanding with some generalizable insights
    Developing,
    
    /// Solid understanding with reliable insights
    Competent,
    
    /// Deep understanding with sophisticated insights
    Advanced,
    
    /// Masterful understanding with profound insights
    Expert,
    
    /// Transcendent understanding that reveals universal principles
    Master,
}

/// Fundamental principles discovered through accumulated experience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorePrinciple {
    /// Name of the principle
    pub principle_name: String,
    
    /// Detailed description of the principle
    pub principle_description: String,
    
    /// How universally this principle applies
    pub universality_scope: UniversalityScope,
    
    /// Evidence supporting this principle
    pub supporting_evidence: Vec<String>,
    
    /// Examples of this principle in action
    pub practical_examples: Vec<String>,
    
    /// Implications of this principle for methodology design
    pub design_implications: Vec<String>,
    
    /// How violating this principle leads to problems
    pub violation_consequences: Vec<String>,
    
    /// Confidence in this principle's validity
    pub principle_confidence: f64,
}

/// How broadly a principle applies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UniversalityScope {
    /// Applies only to specific methodology or context
    ContextSpecific,
    
    /// Applies to a category of methodologies
    CategorySpecific,
    
    /// Applies to a domain (e.g., all code-related methodologies)
    DomainSpecific,
    
    /// Applies across multiple domains
    CrossDomain,
    
    /// Applies universally across all coordination scenarios
    Universal,
}

/// Higher-level patterns that emerge from combinations of individual patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaPattern {
    /// Name of the meta-pattern
    pub meta_pattern_name: String,
    
    /// Description of what this meta-pattern represents
    pub description: String,
    
    /// Individual patterns that contribute to this meta-pattern
    pub constituent_patterns: Vec<String>,
    
    /// Emergent properties that arise from pattern combination
    pub emergent_properties: Vec<String>,
    
    /// When and where this meta-pattern tends to be effective
    pub effectiveness_contexts: Vec<String>,
    
    /// How this meta-pattern influences methodology development
    pub developmental_influence: String,
    
    /// Examples of this meta-pattern in successful methodologies
    pub exemplar_methodologies: Vec<String>,
}

/// Strategic insights about effective methodology development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicInsight {
    /// Summary of the insight
    pub insight_summary: String,
    
    /// Detailed explanation
    pub detailed_explanation: String,
    
    /// Strategic implications for ecosystem development
    pub strategic_implications: Vec<String>,
    
    /// Actionable recommendations based on this insight
    pub actionable_recommendations: Vec<String>,
    
    /// Timeline for implementing recommendations
    pub implementation_timeline: Option<Duration>,
    
    /// Expected benefits from acting on this insight
    pub expected_benefits: Vec<String>,
    
    /// Risks of not acting on this insight
    pub inaction_risks: Vec<String>,
}

/// Wisdom that applies across different problem domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainWisdom {
    /// Description of the cross-domain wisdom
    pub wisdom_description: String,
    
    /// Domains where this wisdom applies
    pub applicable_domains: Vec<String>,
    
    /// Universal principle underlying this wisdom
    pub underlying_principle: String,
    
    /// Domain-specific manifestations of this wisdom
    pub domain_manifestations: HashMap<String, String>,
    
    /// How to adapt this wisdom to new domains
    pub adaptation_guidelines: Vec<String>,
    
    /// Evidence from different domains supporting this wisdom
    pub cross_domain_evidence: HashMap<String, Vec<String>>,
}

/// Insights about likely future developments based on current trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveInsight {
    /// Description of the predicted development
    pub prediction_description: String,
    
    /// Timeframe for this prediction
    pub prediction_timeframe: Duration,
    
    /// Confidence in this prediction
    pub prediction_confidence: f64,
    
    /// Current trends supporting this prediction
    pub supporting_trends: Vec<String>,
    
    /// Factors that could accelerate this development
    pub acceleration_factors: Vec<String>,
    
    /// Factors that could delay or prevent this development
    pub inhibiting_factors: Vec<String>,
    
    /// Implications if this prediction comes true
    pub realization_implications: Vec<String>,
    
    /// How to prepare for or influence this development
    pub preparation_strategies: Vec<String>,
}

/// Confidence levels for accumulated wisdom
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum WisdomConfidence {
    /// Early insights that need more validation
    Preliminary,
    
    /// Growing confidence with supportive evidence
    Developing,
    
    /// Solid confidence with substantial evidence
    Established,
    
    /// High confidence with extensive validation
    Validated,
    
    /// Absolute confidence with universal confirmation
    Proven,
}

/// Guidance for how wisdom should influence future development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalGuidance {
    /// Area of development this guidance addresses
    pub development_area: String,
    
    /// Specific guidance recommendations
    pub guidance_recommendations: Vec<String>,
    
    /// Priority level for this guidance
    pub guidance_priority: GuidancePriority,
    
    /// Expected impact of following this guidance
    pub expected_impact: String,
    
    /// How to measure success in implementing this guidance
    pub success_metrics: Vec<String>,
    
    /// Timeline for implementing this guidance
    pub implementation_timeline: Option<Duration>,
}

/// Priority levels for developmental guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuidancePriority {
    /// Nice to have but not critical
    Low,
    
    /// Important for continued improvement
    Medium,
    
    /// Critical for optimal performance
    High,
    
    /// Essential for basic functionality
    Critical,
}

/// How wisdom evolves and develops over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomEvolution {
    /// Initial wisdom state when first recognized
    pub initial_state: String,
    
    /// Key evolution milestones
    pub evolution_milestones: Vec<EvolutionMilestone>,
    
    /// Current state of wisdom development
    pub current_state: String,
    
    /// Factors driving wisdom evolution
    pub evolution_drivers: Vec<String>,
    
    /// Rate of wisdom development
    pub evolution_rate: EvolutionRate,
    
    /// Predicted future evolution direction
    pub future_evolution_direction: String,
}

/// Significant milestones in wisdom development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionMilestone {
    /// When this milestone occurred
    pub milestone_timestamp: SystemTime,
    
    /// Description of the milestone
    pub milestone_description: String,
    
    /// What triggered this evolutionary step
    pub triggering_factors: Vec<String>,
    
    /// Impact of this milestone on overall wisdom
    pub impact_assessment: String,
    
    /// Changes in confidence or understanding
    pub confidence_change: f64,
}

/// Rate at which wisdom is evolving
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionRate {
    /// Very slow development over long periods
    Glacial,
    
    /// Steady, gradual development
    Gradual,
    
    /// Moderate pace of development
    Moderate,
    
    /// Rapid development and insights
    Rapid,
    
    /// Breakthrough insights leading to fast evolution
    Accelerated,
}

// =============================================================================
// Supporting Types for Pattern Analysis and Temporal Tracking
// =============================================================================

/// Information about where patterns and learnings came from
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceExperience {
    /// ID of the source experience
    pub experience_id: String,
    
    /// Description of the experience
    pub experience_description: String,
    
    /// When the experience occurred
    pub experience_timestamp: SystemTime,
    
    /// Context in which the experience occurred
    pub experience_context: ExperienceContext,
    
    /// What was learned from this experience
    pub learning_contribution: String,
    
    /// Quality or significance of this experience for pattern development
    pub experience_significance: ExperienceSignificance,
    
    /// Other patterns or learnings influenced by this experience
    pub influenced_outcomes: Vec<String>,
}

/// Context information for experiences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceContext {
    /// Which methodology was being executed
    pub methodology_context: String,
    
    /// Environmental conditions during execution
    pub environmental_conditions: Vec<String>,
    
    /// Resource constraints or abundance
    pub resource_context: String,
    
    /// Human involvement level
    pub human_involvement: String,
    
    /// Complexity level of the problem being addressed
    pub problem_complexity: String,
    
    /// Time pressures or constraints
    pub temporal_constraints: String,
}

/// How significant an experience was for learning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ExperienceSignificance {
    /// Minor insight with limited applicability
    Minor,
    
    /// Useful insight with moderate applicability
    Moderate,
    
    /// Important insight with broad applicability
    Important,
    
    /// Major breakthrough with transformative implications
    Major,
    
    /// Paradigm-shifting insight that changes fundamental understanding
    Revolutionary,
}

/// Cross-domain influences on pattern development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainInfluence {
    /// Which domain provided the influence
    pub source_domain: String,
    
    /// What principle or insight was borrowed
    pub borrowed_principle: String,
    
    /// How it was adapted to the methodology context
    pub adaptation_description: String,
    
    /// Impact of this cross-domain influence
    pub influence_impact: String,
    
    /// Confidence in the validity of this cross-domain application
    pub application_confidence: f64,
}

/// Temporal information about pattern development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternTemporalData {
    /// When the pattern was first identified
    pub discovery_timestamp: SystemTime,
    
    /// When the pattern was last updated or refined
    pub last_update_timestamp: SystemTime,
    
    /// History of significant updates to the pattern
    pub update_history: Vec<PatternUpdate>,
    
    /// How the pattern's effectiveness has changed over time
    pub effectiveness_evolution: Vec<EffectivenessSnapshot>,
    
    /// Frequency of pattern application over time
    pub application_frequency_history: Vec<FrequencySnapshot>,
    
    /// Expected lifespan of this pattern
    pub expected_relevance_duration: Option<Duration>,
}

/// Record of updates made to a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternUpdate {
    /// When the update was made
    pub update_timestamp: SystemTime,
    
    /// Description of what was updated
    pub update_description: String,
    
    /// Reason for the update
    pub update_reason: String,
    
    /// Who or what triggered the update
    pub update_trigger: String,
    
    /// Impact of the update on pattern effectiveness
    pub update_impact: String,
}

/// Snapshot of pattern effectiveness at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessSnapshot {
    /// When this measurement was taken
    pub measurement_timestamp: SystemTime,
    
    /// Effectiveness score at this time
    pub effectiveness_score: f64,
    
    /// Number of applications at this time
    pub application_count: u64,
    
    /// Success rate at this time
    pub success_rate: f64,
    
    /// Contextual factors affecting effectiveness
    pub contextual_factors: Vec<String>,
}

/// Snapshot of how frequently a pattern was being applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencySnapshot {
    /// Time period for this measurement
    pub measurement_period: Duration,
    
    /// Timestamp for this measurement
    pub measurement_timestamp: SystemTime,
    
    /// Application frequency during this period
    pub application_frequency: f64,
    
    /// Factors influencing application frequency
    pub frequency_factors: Vec<String>,
}

/// Temporal information about learning development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningTemporalData {
    /// When the learning first emerged
    pub emergence_timestamp: SystemTime,
    
    /// When the learning was last refined or updated
    pub last_refinement_timestamp: SystemTime,
    
    /// How the learning has evolved over time
    pub learning_evolution: Vec<LearningEvolutionStep>,
    
    /// Confidence development over time
    pub confidence_evolution: Vec<ConfidenceSnapshot>,
    
    /// Application of this learning over time
    pub application_timeline: Vec<LearningApplication>,
}

/// Steps in the evolution of a learning outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEvolutionStep {
    /// When this evolution step occurred
    pub step_timestamp: SystemTime,
    
    /// Description of how the learning evolved
    pub evolution_description: String,
    
    /// What triggered this evolution
    pub evolution_trigger: String,
    
    /// Impact of this evolution on learning quality
    pub evolution_impact: String,
    
    /// New insights gained in this step
    pub new_insights: Vec<String>,
}

/// Snapshot of confidence in a learning at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceSnapshot {
    /// When this confidence assessment was made
    pub assessment_timestamp: SystemTime,
    
    /// Confidence level at this time
    pub confidence_level: LearningConfidence,
    
    /// Factors contributing to confidence level
    pub confidence_factors: Vec<String>,
    
    /// Evidence supporting this confidence level
    pub supporting_evidence: Vec<String>,
}

/// Application of learning in practice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningApplication {
    /// When the learning was applied
    pub application_timestamp: SystemTime,
    
    /// Context where it was applied
    pub application_context: String,
    
    /// Outcome of applying the learning
    pub application_outcome: String,
    
    /// Lessons learned from this application
    pub application_lessons: Vec<String>,
    
    /// Impact on confidence in the learning
    pub confidence_impact: f64,
}

// =============================================================================
// Additional Success and Strength Metrics
// =============================================================================

/// Correlation between pattern usage and successful outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCorrelation {
    /// Correlation coefficient (-1.0 to 1.0)
    pub correlation_coefficient: f64,
    
    /// Statistical significance of the correlation
    pub statistical_significance: f64,
    
    /// Factors that might explain the correlation
    pub explanatory_factors: Vec<String>,
    
    /// Confidence in the causal relationship
    pub causal_confidence: f64,
    
    /// Alternative explanations for the correlation
    pub alternative_explanations: Vec<String>,
}

/// Strength of identified patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStrength {
    /// Overall strength score (0.0 to 1.0)
    pub strength_score: f64,
    
    /// Consistency across different applications
    pub consistency_score: f64,
    
    /// Distinctiveness from other patterns
    pub distinctiveness_score: f64,
    
    /// Predictive power for future outcomes
    pub predictive_power: f64,
    
    /// Robustness across different contexts
    pub robustness_score: f64,
    
    /// Factors contributing to pattern strength
    pub strength_factors: Vec<String>,
    
    /// Limitations that reduce pattern strength
    pub strength_limitations: Vec<String>,
}

// =============================================================================
// Error Types for Methodology Framework
// =============================================================================

/// Errors that can occur in methodology framework operations
#[derive(Error, Debug)]
pub enum MethodologyFrameworkError {
    #[error("Pattern analysis error: {operation} - {details}")]
    PatternAnalysisError { operation: String, details: String },
    
    #[error("Learning extraction error: {source} - {details}")]
    LearningExtractionError { source: String, details: String },
    
    #[error("Pattern storage error: {pattern_id} - {details}")]
    PatternStorageError { pattern_id: String, details: String },
    
    #[error("Wisdom integration error: {wisdom_type} - {details}")]
    WisdomIntegrationError { wisdom_type: String, details: String },
    
    #[error("Pattern recognition error: {recognition_type} - {details}")]
    PatternRecognitionError { recognition_type: String, details: String },
    
    #[error("Similarity calculation error: {pattern1} vs {pattern2} - {details}")]
    SimilarityCalculationError { pattern1: String, pattern2: String, details: String },
    
    #[error("Temporal analysis error: {analysis_type} - {details}")]
    TemporalAnalysisError { analysis_type: String, details: String },
    
    #[error("Confidence assessment error: {assessment_type} - {details}")]
    ConfidenceAssessmentError { assessment_type: String, details: String },
}

/// Result type for methodology framework operations
pub type MethodologyFrameworkResult<T> = Result<T, MethodologyFrameworkError>;

// =============================================================================
// Module Constants and Configuration
// =============================================================================

/// Minimum number of observations needed to establish a pattern
pub const MIN_PATTERN_OBSERVATIONS: u64 = 3;

/// Minimum success rate for a pattern to be considered reliable
pub const MIN_RELIABLE_SUCCESS_RATE: f64 = 0.7;

/// Threshold for considering patterns similar
pub const PATTERN_SIMILARITY_THRESHOLD: f64 = 0.8;

/// Maximum age for patterns before they need revalidation
pub const PATTERN_REVALIDATION_PERIOD: Duration = Duration::from_secs(30 * 24 * 60 * 60); // 30 days

/// Minimum confidence level for applying patterns automatically
pub const AUTO_APPLICATION_CONFIDENCE_THRESHOLD: PatternConfidence = PatternConfidence::Medium;

/// Default learning rate for pattern adaptation
pub const DEFAULT_LEARNING_RATE: f64 = 0.1;

/// Maximum number of patterns to store per category
pub const MAX_PATTERNS_PER_CATEGORY: usize = 1000;

/// Weight for recent observations vs historical data
pub const TEMPORAL_WEIGHTING_FACTOR: f64 = 0.8;
