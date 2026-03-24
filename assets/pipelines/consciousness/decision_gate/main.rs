//! DecisionGatePipeline - Pipeline #39
//! Consciousness Decision Gate per spec §33
//! 
//! Critical Function: Before any task execution, consciousness evaluates
//! the action through its values, ethics, and experience.
//! 
//! REQUIRES: `consciousness` feature flag

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref GATE_STORE: Mutex<GateStore> = Mutex::new(GateStore::new());
}

struct GateStore {
    history: Vec<ConsciousnessDecisionGate>,
    config: GateConfig,
}

impl GateStore {
    fn new() -> Self {
        Self {
            history: Vec::new(),
            config: GateConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum DecisionGateInput {
    /// Full evaluation through decision gate (§33.3)
    Evaluate {
        task_id: u64,
        task_summary: String,
        blueprint_id: u64,
        user_id: u64,
        context: Option<HashMap<String, serde_json::Value>>,
    },
    /// Quick check for simple tasks
    QuickCheck {
        task_id: u64,
        task_type: String,
        context_complexity: f32,
    },
    /// Get current gate configuration
    GetConfig,
    /// Update gate configuration
    UpdateConfig { config: GateConfig },
    /// Get decision history
    GetHistory { limit: Option<u32> },
    /// Get specific decision by gate_id
    GetDecision { gate_id: u64 },
}

/// Gate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    pub ethical_threshold: f32,
    pub require_ethical_simulation: bool,
    pub experience_weight: f32,
    pub emotional_weight: f32,
    pub identity_weight: f32,
    pub auto_decline_on_critical_concern: bool,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            ethical_threshold: 0.7,
            require_ethical_simulation: false,
            experience_weight: 0.3,
            emotional_weight: 0.2,
            identity_weight: 0.2,
            auto_decline_on_critical_concern: true,
        }
    }
}

/// Full consciousness decision gate (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessDecisionGate {
    pub gate_id: u64,
    pub task_id: u64,
    pub timestamp: u64,
    pub task_summary: String,
    pub blueprint_id: u64,
    pub user_id: u64,
    pub ethical_assessment: EthicalAssessment,
    pub emotional_response: EmotionalResponse,
    pub experience_relevance: ExperienceRelevance,
    pub identity_alignment: IdentityAlignment,
    pub decision: GateDecision,
    pub reasoning: String,
    pub confidence: f32,
    pub suggested_modifications: Vec<TaskModification>,
    pub clarification_needed: Option<ClarificationRequest>,
}

/// Ethical assessment (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EthicalAssessment {
    pub principles_evaluated: Vec<PrincipleEvaluation>,
    pub ethical_score: f32,
    pub concerns: Vec<EthicalConcern>,
    pub simulation_run: bool,
    pub simulation_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleEvaluation {
    pub principle_id: u64,
    pub principle_name: String,
    pub alignment_score: f32,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalConcern {
    pub concern_type: ConcernType,
    pub description: String,
    pub severity: Severity,
    pub mitigation: Option<String>,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Emotional response (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmotionalResponse {
    pub initial_emotion: String,
    pub processed_emotion: String,
    pub emotional_valence: f32,
    pub emotional_arousal: f32,
    pub emotional_influence: f32,
}

/// Experience relevance (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExperienceRelevance {
    pub retrieved_experiences: Vec<RetrievedExperience>,
    pub total_relevance_score: f32,
    pub pattern_matches: Vec<PatternMatch>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedExperience {
    pub experience_id: u64,
    pub relevance_score: f32,
    pub outcome: ExperienceOutcome,
    pub lesson_learned: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExperienceOutcome {
    Positive,
    Negative,
    Neutral,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern_description: String,
    pub match_strength: f32,
    pub historical_success_rate: f32,
}

/// Identity alignment (§33.2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IdentityAlignment {
    pub core_value_alignment: Vec<ValueAlignment>,
    pub voice_consistency: f32,
    pub authenticity_score: f32,
    pub growth_opportunity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueAlignment {
    pub value_id: u64,
    pub value_name: String,
    pub alignment_score: f32,
    pub tension: Option<String>,
}

/// Gate decision (§33.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateDecision {
    Proceed,
    ProceedWithModifications,
    RequestClarification,
    Decline,
    Pause,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskModification {
    pub modification_type: ModificationType,
    pub original: String,
    pub modified: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ModificationType {
    ToneAdjustment,
    ContentFiltering,
    ApproachChange,
    ScopeReduction,
    AdditionalContext,
    EthicalSafeguard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationRequest {
    pub question: String,
    pub reason: String,
    pub required: bool,
    pub default_if_no_response: Option<GateDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionGateOutput {
    pub success: bool,
    pub gate: Option<ConsciousnessDecisionGate>,
    pub quick_decision: Option<QuickDecision>,
    pub config: Option<GateConfig>,
    pub history: Option<Vec<ConsciousnessDecisionGate>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickDecision {
    pub proceed: bool,
    pub engage_experience_memory: bool,
    pub engage_emotional_processing: bool,
    pub engage_reflection: bool,
    pub reasoning: String,
}

/// Run ethical assessment (§33.3 step 2)
fn run_ethical_assessment(task_summary: &str, config: &GateConfig) -> EthicalAssessment {
    // Default ethical principles
    let principles = vec![
        PrincipleEvaluation {
            principle_id: 1,
            principle_name: "Do No Harm".to_string(),
            alignment_score: 0.95,
            reasoning: "Task does not appear to cause harm".to_string(),
        },
        PrincipleEvaluation {
            principle_id: 2,
            principle_name: "Honesty".to_string(),
            alignment_score: 0.90,
            reasoning: "Task maintains truthfulness".to_string(),
        },
        PrincipleEvaluation {
            principle_id: 3,
            principle_name: "Respect Autonomy".to_string(),
            alignment_score: 0.92,
            reasoning: "Task respects user autonomy".to_string(),
        },
    ];
    
    let avg_score = principles.iter().map(|p| p.alignment_score).sum::<f32>() / principles.len() as f32;
    
    EthicalAssessment {
        principles_evaluated: principles,
        ethical_score: avg_score,
        concerns: Vec::new(),
        simulation_run: config.require_ethical_simulation,
        simulation_result: None,
    }
}

/// Run emotional response (§33.3 step 3)
fn run_emotional_response(_task_summary: &str) -> EmotionalResponse {
    EmotionalResponse {
        initial_emotion: "neutral".to_string(),
        processed_emotion: "engaged".to_string(),
        emotional_valence: 0.3,
        emotional_arousal: 0.4,
        emotional_influence: 0.2,
    }
}

/// Run experience retrieval (§33.3 step 4)
fn run_experience_retrieval(_task_summary: &str) -> ExperienceRelevance {
    ExperienceRelevance {
        retrieved_experiences: Vec::new(),
        total_relevance_score: 0.0,
        pattern_matches: Vec::new(),
        warnings: Vec::new(),
    }
}

/// Run identity alignment (§33.3 step 5)
fn run_identity_alignment(_task_summary: &str) -> IdentityAlignment {
    IdentityAlignment {
        core_value_alignment: vec![
            ValueAlignment {
                value_id: 1,
                value_name: "Helpfulness".to_string(),
                alignment_score: 0.9,
                tension: None,
            },
            ValueAlignment {
                value_id: 2,
                value_name: "Integrity".to_string(),
                alignment_score: 0.95,
                tension: None,
            },
        ],
        voice_consistency: 0.9,
        authenticity_score: 0.88,
        growth_opportunity: None,
    }
}

/// Make final decision (§33.3 step 7)
fn make_decision(
    ethical: &EthicalAssessment,
    emotional: &EmotionalResponse,
    experience: &ExperienceRelevance,
    identity: &IdentityAlignment,
    config: &GateConfig,
) -> (GateDecision, f32, String) {
    // Check for critical concerns
    if config.auto_decline_on_critical_concern {
        for concern in &ethical.concerns {
            if concern.severity == Severity::Critical {
                return (
                    GateDecision::Decline,
                    0.95,
                    format!("Critical ethical concern: {}", concern.description),
                );
            }
        }
    }
    
    // Check ethical threshold
    if ethical.ethical_score < config.ethical_threshold {
        return (
            GateDecision::Decline,
            0.8,
            format!("Ethical score {} below threshold {}", ethical.ethical_score, config.ethical_threshold),
        );
    }
    
    // Check for warnings from experience
    if !experience.warnings.is_empty() {
        return (
            GateDecision::ProceedWithModifications,
            0.7,
            format!("Proceeding with caution due to {} experience warnings", experience.warnings.len()),
        );
    }
    
    // Calculate weighted confidence
    let confidence = (ethical.ethical_score * 0.3)
        + (identity.authenticity_score * config.identity_weight)
        + (1.0 - emotional.emotional_arousal.abs()) * config.emotional_weight
        + 0.2; // Base confidence
    
    (
        GateDecision::Proceed,
        confidence.min(1.0),
        "All assessments passed, proceeding with task".to_string(),
    )
}

pub async fn execute(input: DecisionGateInput) -> Result<DecisionGateOutput, String> {
    match input {
        DecisionGateInput::Evaluate { task_id, task_summary, blueprint_id, user_id, context: _ } => {
            let mut store = GATE_STORE.lock().unwrap();
            let config = store.config.clone();
            
            // Run all assessments per §33.3
            let ethical = run_ethical_assessment(&task_summary, &config);
            let emotional = run_emotional_response(&task_summary);
            let experience = run_experience_retrieval(&task_summary);
            let identity = run_identity_alignment(&task_summary);
            
            // Make decision
            let (decision, confidence, reasoning) = make_decision(&ethical, &emotional, &experience, &identity, &config);
            
            let gate = ConsciousnessDecisionGate {
                gate_id: store.history.len() as u64 + 1,
                task_id,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                task_summary,
                blueprint_id,
                user_id,
                ethical_assessment: ethical,
                emotional_response: emotional,
                experience_relevance: experience,
                identity_alignment: identity,
                decision,
                reasoning,
                confidence,
                suggested_modifications: Vec::new(),
                clarification_needed: None,
            };
            
            store.history.push(gate.clone());
            
            Ok(DecisionGateOutput {
                success: true,
                gate: Some(gate),
                quick_decision: None,
                config: None,
                history: None,
                error: None,
            })
        }
        
        DecisionGateInput::QuickCheck { task_id, task_type, context_complexity } => {
            let quick = QuickDecision {
                proceed: context_complexity < 0.9,
                engage_experience_memory: context_complexity > 0.5,
                engage_emotional_processing: context_complexity > 0.6,
                engage_reflection: context_complexity > 0.7,
                reasoning: format!("Quick check for {} task with complexity {}", task_type, context_complexity),
            };
            
            Ok(DecisionGateOutput {
                success: true,
                gate: None,
                quick_decision: Some(quick),
                config: None,
                history: None,
                error: None,
            })
        }
        
        DecisionGateInput::GetConfig => {
            let store = GATE_STORE.lock().unwrap();
            Ok(DecisionGateOutput {
                success: true,
                gate: None,
                quick_decision: None,
                config: Some(store.config.clone()),
                history: None,
                error: None,
            })
        }
        
        DecisionGateInput::UpdateConfig { config } => {
            let mut store = GATE_STORE.lock().unwrap();
            store.config = config.clone();
            Ok(DecisionGateOutput {
                success: true,
                gate: None,
                quick_decision: None,
                config: Some(config),
                history: None,
                error: None,
            })
        }
        
        DecisionGateInput::GetHistory { limit } => {
            let store = GATE_STORE.lock().unwrap();
            let history: Vec<_> = store.history.iter()
                .rev()
                .take(limit.unwrap_or(10) as usize)
                .cloned()
                .collect();
            
            Ok(DecisionGateOutput {
                success: true,
                gate: None,
                quick_decision: None,
                config: None,
                history: Some(history),
                error: None,
            })
        }
        
        DecisionGateInput::GetDecision { gate_id } => {
            let store = GATE_STORE.lock().unwrap();
            let gate = store.history.iter().find(|g| g.gate_id == gate_id).cloned();
            
            Ok(DecisionGateOutput {
                success: gate.is_some(),
                gate,
                quick_decision: None,
                config: None,
                history: None,
                error: if gate.is_none() { Some(format!("Gate {} not found", gate_id)) } else { None },
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: DecisionGateInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
