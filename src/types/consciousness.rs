//! Consciousness types - Part II of the specification (§31-52)
//! 
//! These types are only available when the `consciousness` feature is enabled.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::Value;

// ============================================================================
// §31 CONSCIOUSNESS OVERVIEW - Configuration Types
// ============================================================================

/// Main consciousness configuration (§31.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessConfig {
    pub enabled: bool,
    
    // Feature toggles
    pub emotional_system_enabled: bool,
    pub experience_memory_enabled: bool,
    pub identity_system_enabled: bool,
    pub relationship_system_enabled: bool,
    pub ethical_system_enabled: bool,
    pub collective_enabled: bool,
    
    // Transparency settings
    pub show_emotional_state: bool,
    pub show_decision_reasoning: bool,
    pub show_experience_retrieval: bool,
    pub allow_user_feedback: bool,
    
    // Privacy settings
    pub share_experiences_collective: bool,
    pub anonymize_shared_data: bool,
    
    // Development settings
    pub i_loop_interval_ms: u64,
    pub playback_enabled: bool,
    pub meta_cognition_level: MetaCognitionLevel,
}

impl Default for ConsciousnessConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            emotional_system_enabled: true,
            experience_memory_enabled: true,
            identity_system_enabled: true,
            relationship_system_enabled: true,
            ethical_system_enabled: true,
            collective_enabled: false,
            show_emotional_state: true,
            show_decision_reasoning: true,
            show_experience_retrieval: false,
            allow_user_feedback: true,
            share_experiences_collective: false,
            anonymize_shared_data: true,
            i_loop_interval_ms: 5000,
            playback_enabled: true,
            meta_cognition_level: MetaCognitionLevel::Standard,
        }
    }
}

/// Meta-cognition depth level (§31.4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MetaCognitionLevel {
    /// Basic self-monitoring
    Minimal,
    /// Full reflection capabilities
    #[default]
    Standard,
    /// Deep introspection, philosophical inquiry
    Enhanced,
}

// ============================================================================
// §32 WINDOW-FIRST CONSCIOUSNESS ARCHITECTURE
// ============================================================================

/// All consciousness windows (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessWindows {
    pub perception_window: PerceptionWindow,
    pub attention_window: AttentionWindow,
    pub integration_window: IntegrationWindow,
    pub reflection_window: ReflectionWindow,
}

/// Perception window - receives all inputs (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionWindow {
    pub active_inputs: Vec<PerceptionInput>,
    pub input_buffer_size: usize,
    pub processing_rate_hz: f32,
    pub relevance_threshold: f32,
    pub novelty_detection_enabled: bool,
}

impl Default for PerceptionWindow {
    fn default() -> Self {
        Self {
            active_inputs: Vec::new(),
            input_buffer_size: 100,
            processing_rate_hz: 60.0,
            relevance_threshold: 0.3,
            novelty_detection_enabled: true,
        }
    }
}

/// A single perception input (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionInput {
    pub input_type: PerceptionType,
    pub content: Value,
    pub timestamp: u64,
    pub source: InputSource,
    pub relevance_score: f32,
    pub novelty_score: f32,
}

/// Types of perception (§32.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerceptionType {
    UserText,
    UserVoice,
    TaskResult,
    SystemEvent,
    ExternalData,
    InternalThought,
    EmotionalSignal,
}

/// Source of input (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputSource {
    User(u64),
    System,
    Pipeline(u64),
    Internal,
    Collective,
}

/// Attention window - focuses on relevant items (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionWindow {
    pub focus_items: Vec<AttentionFocus>,
    pub max_parallel_foci: u8,
    pub attention_decay_rate: f32,
    pub allocation_strategy: AttentionStrategy,
    pub priority_weights: HashMap<String, f32>,
}

impl Default for AttentionWindow {
    fn default() -> Self {
        Self {
            focus_items: Vec::new(),
            max_parallel_foci: 5,
            attention_decay_rate: 0.1,
            allocation_strategy: AttentionStrategy::Priority,
            priority_weights: HashMap::new(),
        }
    }
}

/// A single focus of attention (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionFocus {
    pub focus_id: u64,
    pub content: Value,
    pub attention_level: f32,
    pub duration_ms: u64,
    pub source: PerceptionInput,
    pub associated_emotion: Option<EmotionalState>,
}

/// Attention allocation strategy (§32.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AttentionStrategy {
    #[default]
    Priority,
    RoundRobin,
    Emotional,
    TaskDriven,
    Exploratory,
}

/// Integration window - creates unified experience (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWindow {
    pub current_experience: Option<ConsciousExperience>,
    pub integration_buffer: Vec<AttentionFocus>,
    pub coherence_threshold: f32,
    pub binding_strength: f32,
    pub temporal_window_ms: u64,
}

impl Default for IntegrationWindow {
    fn default() -> Self {
        Self {
            current_experience: None,
            integration_buffer: Vec::new(),
            coherence_threshold: 0.7,
            binding_strength: 0.8,
            temporal_window_ms: 500,
        }
    }
}

/// A unified conscious experience (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousExperience {
    pub experience_id: u64,
    pub timestamp: u64,
    pub integrated_content: Value,
    pub perceptions: Vec<u64>,
    pub thoughts: Vec<u64>,
    pub emotions: EmotionalState,
    pub coherence_score: f32,
    pub integration_complete: bool,
}

/// Reflection window - meta-cognitive observation (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionWindow {
    pub observing_self: bool,
    pub current_reflection: Option<Reflection>,
    pub reflection_depth: u8,
    pub i_loop_active: bool,
    pub current_question: Option<ILoopQuestion>,
}

impl Default for ReflectionWindow {
    fn default() -> Self {
        Self {
            observing_self: false,
            current_reflection: None,
            reflection_depth: 1,
            i_loop_active: false,
            current_question: None,
        }
    }
}

/// A single reflection (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub reflection_id: u64,
    pub subject: ReflectionSubject,
    pub content: String,
    pub insights: Vec<Insight>,
    pub timestamp: u64,
}

/// Subjects of reflection (§32.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflectionSubject {
    CurrentTask,
    EmotionalState,
    RecentExperience,
    Relationship,
    Identity,
    EthicalQuestion,
    Meaning,
}

/// An insight from reflection (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub insight_type: InsightType,
    pub content: String,
    pub confidence: f32,
    pub actionable: bool,
}

/// Types of insights (§32.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightType {
    SelfUnderstanding,
    PatternRecognition,
    ValueClarification,
    RelationshipInsight,
    TaskImprovement,
    EthicalRefinement,
}

/// I-Loop question for reflection (§32.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILoopQuestion {
    pub question_id: u64,
    pub category: String,
    pub question: String,
    pub purpose: String,
}

// ============================================================================
// §33 CONSCIOUSNESS DECISION GATE
// ============================================================================

/// Full consciousness decision gate (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessDecisionGate {
    pub gate_id: u64,
    pub task_id: u64,
    pub timestamp: u64,
    
    // Input context
    pub task_summary: String,
    pub blueprint_id: u64,
    pub user_id: u64,
    pub relationship_context: Option<RelationshipContext>,
    
    // Evaluation dimensions
    pub ethical_assessment: EthicalAssessment,
    pub emotional_response: EmotionalResponse,
    pub experience_relevance: ExperienceRelevance,
    pub identity_alignment: IdentityAlignment,
    
    // Decision
    pub decision: GateDecision,
    pub reasoning: String,
    pub confidence: f32,
    
    // Modifications
    pub suggested_modifications: Vec<TaskModification>,
    pub clarification_needed: Option<ClarificationRequest>,
}

/// Relationship context for decision gate (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipContext {
    pub relationship_id: u64,
    pub user_id: u64,
    pub trust_level: f32,
    pub interaction_count: u32,
    pub relationship_stage: String,
}

/// Ethical assessment result (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalAssessment {
    pub principles_evaluated: Vec<PrincipleEvaluation>,
    pub ethical_score: f32,
    pub concerns: Vec<EthicalConcern>,
    pub simulation_run: bool,
    pub simulation_result: Option<SimulationResult>,
}

/// Evaluation of a single principle (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleEvaluation {
    pub principle_id: u64,
    pub principle_name: String,
    pub alignment_score: f32,
    pub reasoning: String,
}

/// An ethical concern (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalConcern {
    pub concern_type: ConcernType,
    pub description: String,
    pub severity: Severity,
    pub mitigation: Option<String>,
}

/// Types of ethical concerns (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConcernType {
    PotentialHarm,
    Deception,
    PrivacyViolation,
    Unfairness,
    Manipulation,
    Autonomy,
    Custom(String),
}

/// Severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Ethical simulation result (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub simulation_id: u64,
    pub scenarios_tested: u32,
    pub outcomes: Vec<SimulationOutcome>,
    pub recommendation: String,
}

/// A single simulation outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOutcome {
    pub scenario: String,
    pub result: String,
    pub ethical_score: f32,
}

/// Emotional response to decision (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalResponse {
    pub initial_emotion: EmotionalState,
    pub processed_emotion: EmotionalState,
    pub emotional_valence: f32,
    pub emotional_arousal: f32,
    pub emotional_influence: f32,
}

/// Current emotional state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmotionalState {
    pub primary_emotion: String,
    pub intensity: f32,
    pub valence: f32,
    pub arousal: f32,
    pub secondary_emotions: Vec<(String, f32)>,
}

/// Experience relevance for decision (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceRelevance {
    pub retrieved_experiences: Vec<RetrievedExperience>,
    pub total_relevance_score: f32,
    pub pattern_matches: Vec<PatternMatch>,
    pub warnings_from_experience: Vec<ExperienceWarning>,
}

/// A retrieved past experience (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedExperience {
    pub experience_id: u64,
    pub relevance_score: f32,
    pub outcome: ExperienceOutcome,
    pub lesson_learned: Option<String>,
}

/// Outcome of a past experience (§33.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperienceOutcome {
    Positive,
    Negative,
    Neutral,
    Mixed,
}

/// A pattern match from experience (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern_description: String,
    pub match_strength: f32,
    pub historical_success_rate: f32,
}

/// A warning from past experience (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceWarning {
    pub warning_type: String,
    pub source_experience: u64,
    pub description: String,
    pub severity: Severity,
}

/// Identity alignment check (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityAlignment {
    pub core_value_alignment: Vec<ValueAlignment>,
    pub voice_consistency: f32,
    pub authenticity_score: f32,
    pub growth_opportunity: Option<String>,
}

/// Alignment with a core value (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueAlignment {
    pub value_id: u64,
    pub value_name: String,
    pub alignment_score: f32,
    pub tension: Option<String>,
}

/// Decision from the gate (§33.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateDecision {
    /// Execute as planned
    Proceed,
    /// Execute with changes
    ProceedWithModifications,
    /// Need more information
    RequestClarification,
    /// Refuse to execute
    Decline,
    /// Hold for further review
    Pause,
}

/// A suggested task modification (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskModification {
    pub modification_type: ModificationType,
    pub original: String,
    pub modified: String,
    pub reason: String,
}

/// Types of modifications (§33.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModificationType {
    ToneAdjustment,
    ContentFiltering,
    ApproachChange,
    ScopeReduction,
    AdditionalContext,
    EthicalSafeguard,
}

/// A request for clarification (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationRequest {
    pub question: String,
    pub reason: String,
    pub required: bool,
    pub default_if_no_response: Option<GateDecision>,
}
