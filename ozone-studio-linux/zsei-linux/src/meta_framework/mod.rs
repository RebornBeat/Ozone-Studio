// =============================================================================
// zsei-linux/src/meta_framework/mod.rs
// Meta-Framework: ZSEI's Self-Improving Intelligence Coordination System
// 
// This module represents ZSEI's metacognitive capabilities - its ability to think
// about its own thinking, analyze its methodologies, discover improvement 
// opportunities, and evolve its intelligence coordination patterns over time.
// Think of this as ZSEI's "consciousness about its own intelligence development."
// =============================================================================

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use std::fmt;

// Async runtime for sophisticated analysis operations
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{sleep, timeout, interval, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Mathematical analysis for pattern recognition and trend analysis
use ndarray::{Array1, Array2, ArrayD};
use nalgebra::{DVector, DMatrix};

// Import shared protocol types for ecosystem coordination
use shared_protocols::{
    ComponentType,
    ExecutionStatus,
    ProtocolError,
};

// Import methodology runtime types for framework analysis
use methodology_runtime::{
    Methodology,
    MethodologyMetadata,
    ExecutionFramework,
    ValidationFramework,
    MethodologyCategory,
    DifficultyLevel,
    SuccessMetric,
};

// Internal module structure - each represents a key metacognitive capability
pub mod framework_engine;         // Core engine for analyzing and evolving frameworks
pub mod methodology_discoverer;   // Discovers new methodology opportunities
pub mod gap_analyzer;            // Identifies missing capabilities and enhancement needs
pub mod enhancement_coordinator; // Coordinates improvements across the ecosystem
pub mod evolution_tracker;       // Tracks how intelligence capabilities evolve over time
pub mod learning_integrator;     // Integrates new learning into existing frameworks

// Internal analysis and optimization modules
pub mod pattern_analysis;        // Deep pattern analysis across methodologies
pub mod capability_mapping;      // Maps current and desired capabilities
pub mod framework_optimization;  // Optimizes existing frameworks for better performance
pub mod adaptation_strategies;   // Develops strategies for adapting to new requirements
pub mod wisdom_synthesis;       // Synthesizes accumulated experience into actionable wisdom

// Re-export core meta-framework coordination types
pub use framework_engine::{
    FrameworkEngine,
    FrameworkAnalyzer,
    FrameworkSynthesizer,
    FrameworkValidator,
    FrameworkOptimizer,
    FrameworkConfiguration,
    AnalysisConfiguration,
    EngineMetrics,
};

pub use methodology_discoverer::{
    MethodologyDiscoverer,
    DiscoveryEngine,
    OpportunityScanner,
    NoveltyDetector,
    FeasibilityAnalyzer,
    DiscoveryConfiguration,
    DiscoveryMetrics,
    DiscoveryResult,
};

pub use gap_analyzer::{
    GapAnalyzer,
    CapabilityMapper,
    RequirementAnalyzer,
    GapIdentifier,
    PriorityRanker,
    AnalysisConfiguration as GapAnalysisConfig,
    GapAnalysisResult,
    CapabilityGap,
};

pub use enhancement_coordinator::{
    EnhancementCoordinator,
    ImprovementPlanner,
    UpgradeOrchestrator,
    QualityEnhancer,
    PerformanceOptimizer,
    CoordinationConfiguration,
    EnhancementPlan,
    EnhancementOpportunity,
};

pub use evolution_tracker::{
    EvolutionTracker,
    DevelopmentMonitor,
    TrendAnalyzer,
    ProgressEvaluator,
    FutureProjector,
    TrackingConfiguration,
    EvolutionMetrics,
    FrameworkEvolution,
};

pub use learning_integrator::{
    LearningIntegrator,
    ExperienceProcessor,
    KnowledgeConsolidator,
    WisdomExtractor,
    InsightApplicator,
    IntegrationConfiguration,
    LearningOutcome,
    IntegrationResult,
};

// ============================================================================= 
// CORE META-FRAMEWORK TYPES
// These types represent the foundational concepts that enable ZSEI's metacognitive
// capabilities - its ability to understand and improve its own intelligence
// =============================================================================

// Framework state and evolution types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FrameworkState {
    /// Framework is newly created and undergoing initial validation
    Nascent { 
        creation_timestamp: SystemTime,
        initial_validation_score: f64,
        creator_confidence: f64,
    },
    
    /// Framework is actively being developed and refined
    Developing { 
        development_start: SystemTime,
        current_maturity: CapabilityMaturity,
        enhancement_queue: Vec<EnhancementOpportunity>,
        learning_velocity: f64,
    },
    
    /// Framework has reached stability and proven effectiveness
    Mature { 
        maturation_timestamp: SystemTime,
        proven_use_cases: Vec<String>,
        reliability_score: f64,
        optimization_level: f64,
    },
    
    /// Framework is being enhanced with new capabilities or optimizations
    Evolving { 
        evolution_start: SystemTime,
        evolution_type: EvolutionType,
        target_improvements: Vec<String>,
        evolution_confidence: f64,
    },
    
    /// Framework has been superseded by better approaches but retained for reference
    Legacy { 
        superseded_timestamp: SystemTime,
        replacement_framework: Option<String>,
        historical_value: f64,
        archival_importance: ArchivalImportance,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CapabilityMaturity {
    /// Basic functionality exists but lacks sophistication
    Basic { 
        functionality_score: f64,
        reliability_score: f64,
        known_limitations: Vec<String>,
    },
    
    /// Intermediate capability with proven effectiveness in common scenarios
    Intermediate { 
        effectiveness_score: f64,
        scenario_coverage: f64,
        optimization_opportunities: Vec<String>,
    },
    
    /// Advanced capability that handles complex scenarios effectively
    Advanced { 
        sophistication_score: f64,
        complexity_handling: f64,
        cross_domain_integration: f64,
    },
    
    /// Expert-level capability that demonstrates mastery and innovation
    Expert { 
        mastery_score: f64,
        innovation_factor: f64,
        teaching_capability: f64,
    },
    
    /// Master-level capability that transcends traditional limitations
    Master { 
        transcendence_score: f64,
        paradigm_shifting: f64,
        wisdom_embodiment: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchivalImportance {
    /// Historical reference only, minimal ongoing value
    Reference,
    /// Contains valuable patterns for future development
    PatternRepository,
    /// Foundational insights that inform current approaches
    FoundationalInsights,
    /// Critical knowledge that must be preserved
    CriticalKnowledge,
}

// Evolution and development types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvolutionType {
    /// Gradual improvement of existing capabilities
    Incremental { 
        improvement_areas: Vec<String>,
        expected_gain: f64,
        risk_level: RiskLevel,
    },
    
    /// Significant enhancement that adds new major capabilities
    Transformational { 
        new_capabilities: Vec<String>,
        paradigm_shift: bool,
        breakthrough_potential: f64,
    },
    
    /// Revolutionary change that fundamentally alters the framework
    Revolutionary { 
        fundamental_changes: Vec<String>,
        disruption_level: f64,
        adoption_complexity: f64,
    },
    
    /// Adaptation to new requirements or environmental changes
    Adaptive { 
        triggering_changes: Vec<String>,
        adaptation_strategy: AdaptationStrategy,
        flexibility_requirement: f64,
    },
    
    /// Integration with other frameworks or capabilities
    Integrative { 
        integration_targets: Vec<String>,
        synergy_potential: f64,
        coordination_complexity: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,        // Minimal risk of disruption or negative impact
    Moderate,   // Some risk but manageable with proper planning
    High,       // Significant risk requiring careful management
    Critical,   // High risk requiring exceptional care and safeguards
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AdaptationStrategy {
    /// Gradual modification of existing approaches
    GradualModification { 
        modification_rate: f64,
        stability_priority: f64,
    },
    
    /// Complete replacement with new approaches
    Replacement { 
        replacement_timeline: Duration,
        migration_strategy: String,
    },
    
    /// Hybrid approach combining old and new methods
    Hybrid { 
        integration_ratio: f64,
        coordination_mechanism: String,
    },
    
    /// Parallel development with eventual convergence
    Parallel { 
        convergence_timeline: Duration,
        evaluation_criteria: Vec<String>,
    },
}

// Capability gap analysis types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GapCategory {
    /// Missing fundamental capabilities needed for basic operation
    Foundational { 
        critical_functions: Vec<String>,
        impact_severity: ImpactSeverity,
        urgency_level: UrgencyLevel,
    },
    
    /// Missing operational capabilities that limit effectiveness
    Operational { 
        affected_operations: Vec<String>,
        efficiency_impact: f64,
        workaround_availability: bool,
    },
    
    /// Missing strategic capabilities that limit long-term potential
    Strategic { 
        strategic_implications: Vec<String>,
        competitive_impact: f64,
        future_importance: f64,
    },
    
    /// Missing innovation capabilities that limit breakthrough potential
    Innovation { 
        innovation_areas: Vec<String>,
        breakthrough_potential: f64,
        research_complexity: f64,
    },
    
    /// Missing integration capabilities that limit ecosystem coordination
    Integration { 
        integration_targets: Vec<String>,
        coordination_impact: f64,
        ecosystem_benefits: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImpactSeverity {
    Minimal,      // Little impact on overall functionality
    Moderate,     // Noticeable impact but workable
    Significant,  // Major impact requiring attention
    Critical,     // Severe impact requiring immediate action
    Catastrophic, // Fundamental functionality at risk
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UrgencyLevel {
    Low,          // Can be addressed in future planning cycles
    Medium,       // Should be addressed in next major cycle
    High,         // Requires priority attention and resources
    Critical,     // Immediate action required
    Emergency,    // Drop everything and address immediately
}

// Enhancement and improvement types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EnhancementType {
    /// Performance improvements that increase speed or efficiency
    Performance { 
        target_metrics: Vec<String>,
        expected_improvement: f64,
        resource_cost: ResourceCost,
    },
    
    /// Quality improvements that increase accuracy or reliability
    Quality { 
        quality_dimensions: Vec<String>,
        current_baseline: f64,
        target_improvement: f64,
    },
    
    /// Capability extensions that add new functionality
    Capability { 
        new_functions: Vec<String>,
        complexity_increase: f64,
        integration_requirements: Vec<String>,
    },
    
    /// User experience improvements that enhance interaction quality
    UserExperience { 
        experience_dimensions: Vec<String>,
        usability_impact: f64,
        satisfaction_potential: f64,
    },
    
    /// Architectural improvements that enhance maintainability or scalability
    Architectural { 
        architectural_improvements: Vec<String>,
        scalability_benefit: f64,
        maintenance_reduction: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceCost {
    pub development_effort: EffortLevel,
    pub computational_cost: ComputationalCost,
    pub maintenance_overhead: f64,
    pub risk_assessment: RiskAssessment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EffortLevel {
    Trivial,      // Minimal development effort required
    Low,          // Low effort, can be done quickly
    Moderate,     // Moderate effort, requires planning
    High,         // High effort, requires significant resources
    Intensive,    // Intensive effort, major undertaking
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComputationalCost {
    Negligible,   // No meaningful computational impact
    Low,          // Minor computational overhead
    Moderate,     // Noticeable but manageable computational cost
    High,         // Significant computational requirements
    Intensive,    // Major computational resource requirements
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RiskAssessment {
    pub technical_risk: RiskLevel,
    pub operational_risk: RiskLevel,
    pub strategic_risk: RiskLevel,
    pub mitigation_strategies: Vec<String>,
}

// Discovery and learning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub discovery_id: String,
    pub discovery_type: DiscoveryType,
    pub discovery_confidence: f64,
    pub potential_impact: ImpactAssessment,
    pub feasibility_analysis: FeasibilityAnalysis,
    pub implementation_complexity: ComplexityAssessment,
    pub discovered_at: SystemTime,
    pub discovered_by: DiscoverySource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiscoveryType {
    /// New methodology pattern discovered through analysis
    MethodologyPattern { 
        pattern_description: String,
        applicable_domains: Vec<String>,
        effectiveness_potential: f64,
    },
    
    /// New optimization opportunity identified
    OptimizationOpportunity { 
        optimization_target: String,
        improvement_potential: f64,
        implementation_approach: String,
    },
    
    /// New integration possibility between existing capabilities
    IntegrationPossibility { 
        integration_components: Vec<String>,
        synergy_potential: f64,
        coordination_requirements: Vec<String>,
    },
    
    /// New capability gap requiring development
    CapabilityGap { 
        missing_capability: String,
        impact_analysis: String,
        development_priority: f64,
    },
    
    /// New cross-domain insight with broad applicability
    CrossDomainInsight { 
        source_domain: String,
        target_domains: Vec<String>,
        universal_principle: String,
        applicability_scope: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub immediate_impact: f64,
    pub short_term_impact: f64,
    pub long_term_impact: f64,
    pub ecosystem_benefit: f64,
    pub strategic_value: f64,
    pub innovation_potential: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeasibilityAnalysis {
    pub technical_feasibility: f64,
    pub resource_feasibility: f64,
    pub timeline_feasibility: f64,
    pub risk_acceptability: f64,
    pub strategic_alignment: f64,
    pub dependency_complexity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityAssessment {
    pub design_complexity: f64,
    pub implementation_complexity: f64,
    pub integration_complexity: f64,
    pub validation_complexity: f64,
    pub maintenance_complexity: f64,
    pub evolution_complexity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiscoverySource {
    /// Discovered through automated pattern analysis
    AutomatedAnalysis { 
        analysis_algorithm: String,
        confidence_score: f64,
    },
    
    /// Discovered through human-guided exploration
    HumanGuidedExploration { 
        human_contributor: String,
        guidance_quality: f64,
    },
    
    /// Discovered through cross-domain synthesis
    CrossDomainSynthesis { 
        source_domains: Vec<String>,
        synthesis_method: String,
    },
    
    /// Discovered through ecosystem experience analysis
    ExperienceAnalysis { 
        experience_corpus: String,
        pattern_recognition_method: String,
    },
    
    /// Discovered through external research integration
    ExternalResearch { 
        research_source: String,
        adaptation_method: String,
    },
}

// Evolution tracking and direction types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvolutionDirection {
    /// Evolution toward greater specialization and domain expertise
    Specialization { 
        target_domains: Vec<String>,
        depth_increase: f64,
        breadth_tradeoff: f64,
    },
    
    /// Evolution toward greater generalization and broad applicability
    Generalization { 
        scope_expansion: Vec<String>,
        abstraction_level: f64,
        universal_applicability: f64,
    },
    
    /// Evolution toward better integration with other capabilities
    Integration { 
        integration_targets: Vec<String>,
        coordination_enhancement: f64,
        ecosystem_alignment: f64,
    },
    
    /// Evolution toward greater efficiency and performance
    Optimization { 
        optimization_dimensions: Vec<String>,
        efficiency_target: f64,
        performance_benchmark: f64,
    },
    
    /// Evolution toward enhanced user experience and accessibility
    UserCentric { 
        user_experience_dimensions: Vec<String>,
        accessibility_improvements: f64,
        satisfaction_enhancement: f64,
    },
}

// Framework evolution and development types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkEvolution {
    pub evolution_id: String,
    pub framework_id: String,
    pub evolution_timeline: EvolutionTimeline,
    pub evolution_drivers: Vec<EvolutionDriver>,
    pub evolution_strategy: EvolutionStrategy,
    pub success_criteria: Vec<EvolutionSuccessCriterion>,
    pub risk_mitigation: Vec<RiskMitigationStrategy>,
    pub progress_tracking: EvolutionProgressTracking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionTimeline {
    pub planning_phase: PhaseDefinition,
    pub development_phase: PhaseDefinition,
    pub validation_phase: PhaseDefinition,
    pub deployment_phase: PhaseDefinition,
    pub monitoring_phase: PhaseDefinition,
    pub total_duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseDefinition {
    pub phase_name: String,
    pub duration: Duration,
    pub key_activities: Vec<String>,
    pub success_criteria: Vec<String>,
    pub deliverables: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvolutionDriver {
    /// Evolution driven by identified capability gaps
    CapabilityGap { 
        gap_description: String,
        impact_severity: ImpactSeverity,
        strategic_importance: f64,
    },
    
    /// Evolution driven by changing ecosystem requirements
    EcosystemRequirement { 
        requirement_change: String,
        adaptation_necessity: f64,
        coordination_impact: f64,
    },
    
    /// Evolution driven by new technology opportunities
    TechnologyOpportunity { 
        technology_description: String,
        opportunity_window: Duration,
        competitive_advantage: f64,
    },
    
    /// Evolution driven by user feedback and needs
    UserFeedback { 
        feedback_summary: String,
        user_impact: f64,
        satisfaction_improvement: f64,
    },
    
    /// Evolution driven by strategic vision and goals
    StrategicVision { 
        vision_alignment: String,
        strategic_importance: f64,
        long_term_value: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionStrategy {
    pub strategy_type: EvolutionStrategyType,
    pub implementation_approach: ImplementationApproach,
    pub resource_allocation: EvolutionResourceAllocation,
    pub timeline_strategy: TimelineStrategy,
    pub risk_management: EvolutionRiskManagement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvolutionStrategyType {
    /// Conservative evolution with minimal disruption
    Conservative { 
        stability_priority: f64,
        change_velocity: f64,
    },
    
    /// Aggressive evolution pursuing maximum advancement
    Aggressive { 
        advancement_priority: f64,
        acceptable_risk: f64,
    },
    
    /// Balanced evolution optimizing multiple objectives
    Balanced { 
        optimization_objectives: Vec<String>,
        tradeoff_strategy: String,
    },
    
    /// Experimental evolution exploring new possibilities
    Experimental { 
        exploration_scope: Vec<String>,
        learning_objectives: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationApproach {
    /// Single-phase implementation with complete replacement
    BigBang { 
        cutover_strategy: String,
        rollback_plan: String,
    },
    
    /// Multi-phase implementation with gradual transition
    Phased { 
        phase_definitions: Vec<PhaseDefinition>,
        integration_strategy: String,
    },
    
    /// Parallel implementation with eventual convergence
    Parallel { 
        parallel_streams: Vec<String>,
        convergence_plan: String,
    },
    
    /// Pilot implementation with iterative expansion
    Pilot { 
        pilot_scope: String,
        expansion_criteria: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionResourceAllocation {
    pub development_resources: f64,
    pub validation_resources: f64,
    pub deployment_resources: f64,
    pub training_resources: f64,
    pub maintenance_resources: f64,
    pub contingency_resources: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimelineStrategy {
    /// Aggressive timeline prioritizing speed
    Aggressive { 
        compressed_phases: Vec<String>,
        parallel_activities: Vec<String>,
    },
    
    /// Conservative timeline prioritizing quality
    Conservative { 
        extended_validation: bool,
        buffer_time: Duration,
    },
    
    /// Adaptive timeline responding to progress
    Adaptive { 
        milestone_checkpoints: Vec<String>,
        adjustment_triggers: Vec<String>,
    },
    
    /// Milestone-driven timeline with fixed deliverables
    MilestoneDriven { 
        key_milestones: Vec<String>,
        dependency_management: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionRiskManagement {
    pub identified_risks: Vec<EvolutionRisk>,
    pub mitigation_strategies: Vec<RiskMitigationStrategy>,
    pub contingency_plans: Vec<ContingencyPlan>,
    pub monitoring_framework: RiskMonitoringFramework,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionRisk {
    pub risk_id: String,
    pub risk_description: String,
    pub risk_category: RiskCategory,
    pub probability: f64,
    pub impact: f64,
    pub risk_score: f64,
    pub first_identified: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskCategory {
    Technical,      // Risk related to technical implementation
    Operational,    // Risk related to operational impact
    Strategic,      // Risk related to strategic objectives
    Resource,       // Risk related to resource availability
    Timeline,       // Risk related to schedule constraints
    Quality,        // Risk related to quality standards
    Integration,    // Risk related to ecosystem integration
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMitigationStrategy {
    pub strategy_id: String,
    pub target_risks: Vec<String>,
    pub mitigation_actions: Vec<String>,
    pub effectiveness_estimate: f64,
    pub resource_requirements: ResourceRequirement,
    pub implementation_timeline: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirement {
    pub human_resources: f64,
    pub computational_resources: f64,
    pub financial_resources: f64,
    pub time_investment: Duration,
    pub expertise_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContingencyPlan {
    pub plan_id: String,
    pub trigger_conditions: Vec<String>,
    pub response_actions: Vec<String>,
    pub success_criteria: Vec<String>,
    pub escalation_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMonitoringFramework {
    pub monitoring_frequency: Duration,
    pub key_indicators: Vec<String>,
    pub threshold_values: HashMap<String, f64>,
    pub alert_mechanisms: Vec<String>,
    pub review_schedule: Vec<SystemTime>,
}

// Success criteria and measurement types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionSuccessCriterion {
    pub criterion_id: String,
    pub criterion_description: String,
    pub measurement_method: MeasurementMethod,
    pub target_value: f64,
    pub minimum_acceptable: f64,
    pub measurement_frequency: Duration,
    pub stakeholder_importance: StakeholderImportance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MeasurementMethod {
    /// Quantitative measurement using metrics
    Quantitative { 
        metric_name: String,
        unit_of_measure: String,
        data_source: String,
    },
    
    /// Qualitative assessment using expert evaluation
    Qualitative { 
        evaluation_criteria: Vec<String>,
        evaluator_qualifications: String,
        assessment_framework: String,
    },
    
    /// Comparative measurement against baseline or benchmark
    Comparative { 
        comparison_baseline: String,
        comparison_method: String,
        benchmark_source: String,
    },
    
    /// User satisfaction measurement through feedback
    UserSatisfaction { 
        feedback_collection_method: String,
        sample_size_target: u32,
        satisfaction_dimensions: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeholderImportance {
    pub ecosystem_importance: f64,
    pub user_importance: f64,
    pub strategic_importance: f64,
    pub operational_importance: f64,
    pub innovation_importance: f64,
}

// Progress tracking and monitoring types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionProgressTracking {
    pub tracking_id: String,
    pub current_phase: String,
    pub overall_progress: f64,
    pub phase_progress: HashMap<String, f64>,
    pub milestone_achievements: Vec<MilestoneAchievement>,
    pub performance_indicators: PerformanceIndicators,
    pub quality_metrics: EvolutionQualityMetrics,
    pub risk_status: RiskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneAchievement {
    pub milestone_id: String,
    pub milestone_name: String,
    pub achieved_at: Option<SystemTime>,
    pub achievement_quality: f64,
    pub lessons_learned: Vec<String>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceIndicators {
    pub development_velocity: f64,
    pub quality_trend: f64,
    pub resource_efficiency: f64,
    pub stakeholder_satisfaction: f64,
    pub innovation_rate: f64,
    pub integration_success: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionQualityMetrics {
    pub design_quality: f64,
    pub implementation_quality: f64,
    pub validation_quality: f64,
    pub documentation_quality: f64,
    pub maintainability_score: f64,
    pub extensibility_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskStatus {
    pub overall_risk_level: RiskLevel,
    pub active_risks: Vec<String>,
    pub mitigated_risks: Vec<String>,
    pub emerging_risks: Vec<String>,
    pub risk_trend: RiskTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskTrend {
    Improving,    // Risk levels decreasing over time
    Stable,       // Risk levels remaining constant
    Deteriorating, // Risk levels increasing over time
    Volatile,     // Risk levels fluctuating significantly
}

// Learning integration and knowledge management types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningOutcome {
    pub outcome_id: String,
    pub learning_type: LearningType,
    pub knowledge_gained: String,
    pub confidence_level: f64,
    pub applicability_scope: Vec<String>,
    pub integration_recommendations: Vec<String>,
    pub validation_evidence: Vec<String>,
    pub learned_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LearningType {
    /// Learning about what approaches work effectively
    EffectivenessLearning { 
        effectiveness_domain: String,
        success_factors: Vec<String>,
        context_dependencies: Vec<String>,
    },
    
    /// Learning about what approaches to avoid
    FailureLearning { 
        failure_patterns: Vec<String>,
        root_causes: Vec<String>,
        prevention_strategies: Vec<String>,
    },
    
    /// Learning about optimization opportunities
    OptimizationLearning { 
        optimization_targets: Vec<String>,
        improvement_strategies: Vec<String>,
        tradeoff_insights: Vec<String>,
    },
    
    /// Learning about integration patterns
    IntegrationLearning { 
        integration_patterns: Vec<String>,
        coordination_insights: Vec<String>,
        synergy_opportunities: Vec<String>,
    },
    
    /// Learning about user needs and preferences
    UserLearning { 
        user_insights: Vec<String>,
        preference_patterns: Vec<String>,
        satisfaction_drivers: Vec<String>,
    },
}

// Configuration types for meta-framework operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFrameworkConfig {
    pub framework_analysis: FrameworkAnalysisConfig,
    pub discovery_engine: DiscoveryEngineConfig,
    pub gap_analysis: GapAnalysisConfig,
    pub enhancement_coordination: EnhancementCoordinationConfig,
    pub evolution_tracking: EvolutionTrackingConfig,
    pub learning_integration: LearningIntegrationConfig,
    pub performance_optimization: PerformanceOptimizationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkAnalysisConfig {
    pub analysis_depth: AnalysisDepth,
    pub pattern_recognition_threshold: f64,
    pub relationship_analysis_enabled: bool,
    pub cross_framework_comparison: bool,
    pub historical_trend_analysis: bool,
    pub predictive_modeling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalysisDepth {
    Surface,        // Basic analysis of obvious patterns
    Intermediate,   // Moderate analysis including relationships
    Deep,          // Comprehensive analysis with cross-correlations
    Comprehensive, // Exhaustive analysis across all dimensions
    Transcendent,  // Beyond traditional analysis boundaries
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryEngineConfig {
    pub opportunity_scanning_frequency: Duration,
    pub novelty_detection_sensitivity: f64,
    pub feasibility_analysis_depth: AnalysisDepth,
    pub cross_domain_discovery: bool,
    pub automated_discovery: bool,
    pub human_guided_discovery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementCoordinationConfig {
    pub enhancement_planning_horizon: Duration,
    pub resource_allocation_strategy: AllocationStrategy,
    pub quality_enhancement_priority: f64,
    pub performance_enhancement_priority: f64,
    pub capability_enhancement_priority: f64,
    pub user_experience_priority: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AllocationStrategy {
    EqualDistribution,     // Distribute resources equally across opportunities
    PriorityBased,        // Allocate based on strategic priority
    ImpactOptimized,      // Optimize for maximum impact
    RiskAdjusted,         // Adjust allocation based on risk assessment
    PortfolioBalanced,    // Balance across different types of enhancement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionTrackingConfig {
    pub tracking_granularity: TrackingGranularity,
    pub trend_analysis_window: Duration,
    pub progress_evaluation_frequency: Duration,
    pub future_projection_horizon: Duration,
    pub benchmark_comparison: bool,
    pub ecosystem_impact_tracking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrackingGranularity {
    Coarse,       // High-level tracking of major changes
    Standard,     // Normal tracking of significant changes
    Fine,         // Detailed tracking of incremental changes
    Microscopic,  // Extremely detailed tracking of all changes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningIntegrationConfig {
    pub experience_processing_depth: AnalysisDepth,
    pub knowledge_consolidation_frequency: Duration,
    pub wisdom_extraction_threshold: f64,
    pub insight_application_automation: bool,
    pub cross_framework_learning: bool,
    pub ecosystem_learning_sharing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceOptimizationConfig {
    pub optimization_frequency: Duration,
    pub performance_baseline_tracking: bool,
    pub bottleneck_identification: bool,
    pub resource_utilization_optimization: bool,
    pub scalability_optimization: bool,
    pub efficiency_measurement: bool,
}

// Error types specific to meta-framework operations
#[derive(Error, Debug)]
pub enum MetaFrameworkError {
    #[error("Framework analysis error: {framework_id} - {details}")]
    FrameworkAnalysisError { 
        framework_id: String, 
        details: String 
    },
    
    #[error("Discovery engine error: {discovery_type} - {details}")]
    DiscoveryError { 
        discovery_type: String, 
        details: String 
    },
    
    #[error("Gap analysis error: {analysis_target} - {details}")]
    GapAnalysisError { 
        analysis_target: String, 
        details: String 
    },
    
    #[error("Enhancement coordination error: {enhancement_id} - {details}")]
    EnhancementError { 
        enhancement_id: String, 
        details: String 
    },
    
    #[error("Evolution tracking error: {evolution_id} - {details}")]
    EvolutionTrackingError { 
        evolution_id: String, 
        details: String 
    },
    
    #[error("Learning integration error: {learning_context} - {details}")]
    LearningIntegrationError { 
        learning_context: String, 
        details: String 
    },
    
    #[error("Configuration error: {component} - {details}")]
    ConfigurationError { 
        component: String, 
        details: String 
    },
    
    #[error("Resource allocation error: {resource_type} - {details}")]
    ResourceAllocationError { 
        resource_type: String, 
        details: String 
    },
    
    #[error("Validation error: {validation_type} - {details}")]
    ValidationError { 
        validation_type: String, 
        details: String 
    },
    
    #[error("Integration error: {integration_context} - {details}")]
    IntegrationError { 
        integration_context: String, 
        details: String 
    },
}

// Core traits that define meta-framework capabilities
pub trait MetaFrameworkCapability {
    type Config;
    type Input;
    type Output;
    type Error;
    
    /// Initialize the capability with configuration
    fn initialize(config: Self::Config) -> Result<Self, Self::Error> where Self: Sized;
    
    /// Process input and generate output using the capability
    fn process(&mut self, input: Self::Input) -> Result<Self::Output, Self::Error>;
    
    /// Validate the capability's current state and performance
    fn validate(&self) -> Result<ValidationResult, Self::Error>;
    
    /// Get metrics about the capability's performance
    fn get_metrics(&self) -> CapabilityMetrics;
}

pub trait FrameworkAnalyzer {
    /// Analyze a framework to understand its current state and characteristics
    fn analyze_framework(&mut self, framework: &Methodology) -> Result<FrameworkAnalysisResult, MetaFrameworkError>;
    
    /// Compare multiple frameworks to identify patterns and differences
    fn compare_frameworks(&mut self, frameworks: &[Methodology]) -> Result<FrameworkComparisonResult, MetaFrameworkError>;
    
    /// Evaluate framework effectiveness based on usage patterns and outcomes
    fn evaluate_effectiveness(&mut self, framework_id: &str, usage_history: &[UsageRecord]) -> Result<EffectivenessEvaluation, MetaFrameworkError>;
    
    /// Identify optimization opportunities within a framework
    fn identify_optimizations(&mut self, framework: &Methodology) -> Result<Vec<OptimizationOpportunity>, MetaFrameworkError>;
}

pub trait CapabilityGapAnalyzer {
    /// Analyze current capabilities to identify gaps and missing functionality
    fn analyze_capability_gaps(&mut self, current_state: &EcosystemCapabilityMap) -> Result<GapAnalysisResult, MetaFrameworkError>;
    
    /// Prioritize identified gaps based on impact and feasibility
    fn prioritize_gaps(&mut self, gaps: &[CapabilityGap]) -> Result<PrioritizedGapList, MetaFrameworkError>;
    
    /// Recommend strategies for addressing capability gaps
    fn recommend_gap_strategies(&mut self, gap: &CapabilityGap) -> Result<Vec<GapAddressStrategy>, MetaFrameworkError>;
    
    /// Track progress in addressing capability gaps over time
    fn track_gap_resolution(&mut self, gap_id: &str) -> Result<GapResolutionProgress, MetaFrameworkError>;
}

pub trait EnhancementCoordinator {
    /// Plan enhancements across multiple frameworks and capabilities
    fn plan_enhancements(&mut self, opportunities: &[EnhancementOpportunity]) -> Result<EnhancementPlan, MetaFrameworkError>;
    
    /// Coordinate the implementation of planned enhancements
    fn coordinate_implementation(&mut self, plan: &EnhancementPlan) -> Result<ImplementationResult, MetaFrameworkError>;
    
    /// Monitor enhancement progress and adjust plans as needed
    fn monitor_enhancement_progress(&mut self, enhancement_id: &str) -> Result<EnhancementProgress, MetaFrameworkError>;
    
    /// Evaluate the success of completed enhancements
    fn evaluate_enhancement_success(&mut self, enhancement_id: &str) -> Result<EnhancementEvaluation, MetaFrameworkError>;
}

pub trait EvolutionTracker {
    /// Track the evolution of frameworks over time
    fn track_framework_evolution(&mut self, framework_id: &str) -> Result<EvolutionHistory, MetaFrameworkError>;
    
    /// Analyze trends in framework development and improvement
    fn analyze_evolution_trends(&mut self, timeframe: Duration) -> Result<EvolutionTrendAnalysis, MetaFrameworkError>;
    
    /// Project future evolution directions based on current trends
    fn project_future_evolution(&mut self, framework_id: &str, projection_horizon: Duration) -> Result<EvolutionProjection, MetaFrameworkError>;
    
    /// Identify factors that drive successful evolution
    fn identify_evolution_drivers(&mut self, successful_evolutions: &[EvolutionCase]) -> Result<EvolutionDriverAnalysis, MetaFrameworkError>;
}

pub trait LearningIntegrator {
    /// Integrate new learning from experiences into existing frameworks
    fn integrate_experience_learning(&mut self, experiences: &[EcosystemExperience]) -> Result<LearningIntegrationResult, MetaFrameworkError>;
    
    /// Consolidate knowledge across different learning sources
    fn consolidate_knowledge(&mut self, knowledge_sources: &[KnowledgeSource]) -> Result<ConsolidatedKnowledge, MetaFrameworkError>;
    
    /// Extract actionable wisdom from accumulated experience
    fn extract_wisdom(&mut self, experience_corpus: &ExperienceCorpus) -> Result<ExtractedWisdom, MetaFrameworkError>;
    
    /// Apply learned insights to improve existing frameworks
    fn apply_insights(&mut self, insights: &[ActionableInsight], framework_id: &str) -> Result<InsightApplicationResult, MetaFrameworkError>;
}

// Supporting types for trait implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkAnalysisResult {
    pub framework_id: String,
    pub current_state: FrameworkState,
    pub maturity_assessment: CapabilityMaturity,
    pub effectiveness_score: f64,
    pub optimization_opportunities: Vec<OptimizationOpportunity>,
    pub integration_potential: f64,
    pub evolution_recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkComparisonResult {
    pub comparison_id: String,
    pub frameworks_analyzed: Vec<String>,
    pub similarity_matrix: Array2<f64>,
    pub differential_analysis: DifferentialAnalysis,
    pub best_practices: Vec<BestPractice>,
    pub integration_opportunities: Vec<IntegrationOpportunity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialAnalysis {
    pub unique_strengths: HashMap<String, Vec<String>>,
    pub common_patterns: Vec<String>,
    pub complementary_capabilities: Vec<String>,
    pub redundant_functionality: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestPractice {
    pub practice_id: String,
    pub practice_description: String,
    pub effectiveness_evidence: Vec<String>,
    pub applicability_scope: Vec<String>,
    pub implementation_guidance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationOpportunity {
    pub opportunity_id: String,
    pub participating_frameworks: Vec<String>,
    pub synergy_potential: f64,
    pub integration_complexity: f64,
    pub expected_benefits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessEvaluation {
    pub framework_id: String,
    pub evaluation_period: Duration,
    pub usage_frequency: f64,
    pub success_rate: f64,
    pub user_satisfaction: f64,
    pub performance_metrics: PerformanceMetrics,
    pub improvement_trends: Vec<ImprovementTrend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub execution_speed: f64,
    pub resource_efficiency: f64,
    pub quality_consistency: f64,
    pub error_rate: f64,
    pub scalability_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementTrend {
    pub metric_name: String,
    pub trend_direction: TrendDirection,
    pub improvement_rate: f64,
    pub statistical_significance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
    Volatile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationOpportunity {
    pub opportunity_id: String,
    pub optimization_type: OptimizationType,
    pub target_component: String,
    pub improvement_potential: f64,
    pub implementation_effort: EffortLevel,
    pub risk_assessment: RiskAssessment,
    pub priority_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptimizationType {
    Performance,      // Speed or efficiency improvements
    Quality,          // Accuracy or reliability improvements  
    Usability,        // User experience improvements
    Maintainability,  // Code quality and maintainability
    Scalability,      // Ability to handle increased load
    Integration,      // Better ecosystem integration
    Innovation,       // Novel capability development
}

// Additional supporting types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub framework_id: String,
    pub usage_timestamp: SystemTime,
    pub usage_context: String,
    pub success: bool,
    pub performance_data: PerformanceData,
    pub user_feedback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceData {
    pub execution_time: Duration,
    pub resource_consumption: ResourceConsumption,
    pub quality_metrics: QualityMetrics,
    pub error_information: Option<ErrorInformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConsumption {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub network_bandwidth: f64,
    pub storage_operations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub accuracy: f64,
    pub completeness: f64,
    pub consistency: f64,
    pub reliability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInformation {
    pub error_type: String,
    pub error_message: String,
    pub error_context: String,
    pub recovery_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemCapabilityMap {
    pub capabilities: HashMap<String, CapabilityDefinition>,
    pub capability_relationships: Vec<CapabilityRelationship>,
    pub maturity_levels: HashMap<String, CapabilityMaturity>,
    pub usage_patterns: HashMap<String, UsagePattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDefinition {
    pub capability_id: String,
    pub capability_name: String,
    pub description: String,
    pub providing_components: Vec<String>,
    pub quality_level: f64,
    pub reliability_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRelationship {
    pub source_capability: String,
    pub target_capability: String,
    pub relationship_type: RelationshipType,
    pub relationship_strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    Dependency,      // Source depends on target
    Enhancement,     // Source enhances target
    Collaboration,   // Source and target work together
    Substitution,    // Source can substitute for target
    Conflict,        // Source conflicts with target
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    pub pattern_id: String,
    pub usage_frequency: f64,
    pub typical_contexts: Vec<String>,
    pub success_patterns: Vec<String>,
    pub failure_patterns: Vec<String>,
}

// Additional result and data types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysisResult {
    pub analysis_id: String,
    pub identified_gaps: Vec<CapabilityGap>,
    pub gap_impact_assessment: GapImpactAssessment,
    pub recommended_priorities: PriorityRecommendations,
    pub resource_requirements: OverallResourceRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGap {
    pub gap_id: String,
    pub gap_category: GapCategory,
    pub gap_description: String,
    pub impact_severity: ImpactSeverity,
    pub urgency_level: UrgencyLevel,
    pub affected_capabilities: Vec<String>,
    pub estimated_effort: EffortEstimate,
    pub strategic_importance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortEstimate {
    pub development_effort: EffortLevel,
    pub validation_effort: EffortLevel,
    pub integration_effort: EffortLevel,
    pub timeline_estimate: Duration,
    pub confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapImpactAssessment {
    pub overall_ecosystem_impact: f64,
    pub user_experience_impact: f64,
    pub strategic_goal_impact: f64,
    pub operational_efficiency_impact: f64,
    pub innovation_capability_impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityRecommendations {
    pub high_priority_gaps: Vec<String>,
    pub medium_priority_gaps: Vec<String>,
    pub low_priority_gaps: Vec<String>,
    pub strategic_sequence: Vec<String>,
    pub quick_wins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallResourceRequirements {
    pub total_development_effort: f64,
    pub total_timeline: Duration,
    pub resource_distribution: HashMap<String, f64>,
    pub critical_dependencies: Vec<String>,
}

// Result type for meta-framework operations
pub type MetaFrameworkResult<T> = Result<T, MetaFrameworkError>;

// Validation and metrics types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub validation_score: f64,
    pub validation_details: Vec<ValidationDetail>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDetail {
    pub aspect: String,
    pub score: f64,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMetrics {
    pub performance_score: f64,
    pub reliability_score: f64,
    pub efficiency_score: f64,
    pub user_satisfaction: f64,
    pub innovation_factor: f64,
    pub ecosystem_integration: f64,
}

// Constants for meta-framework configuration and operation
pub const META_FRAMEWORK_VERSION: &str = "1.0.0";
pub const DEFAULT_ANALYSIS_DEPTH: AnalysisDepth = AnalysisDepth::Comprehensive;
pub const MIN_CONFIDENCE_THRESHOLD: f64 = 0.7;
pub const MAX_EVOLUTION_TIMELINE: Duration = Duration::from_secs(365 * 24 * 3600); // 1 year
pub const DEFAULT_TRACKING_GRANULARITY: TrackingGranularity = TrackingGranularity::Standard;
pub const CRITICAL_IMPACT_THRESHOLD: f64 = 0.8;
pub const HIGH_PRIORITY_THRESHOLD: f64 = 0.7;
pub const DEFAULT_RESOURCE_ALLOCATION_PERCENTAGE: f64 = 0.15; // 15% of total resources
