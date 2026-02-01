//! SelfModelPipeline - Pipelines #45-46 (Internal Language, Narrative Construction)
//! 
//! Maintain coherent, evolving sense of self across all interactions.
//! Per spec §41: Identity System, §44: Internal Language, §45: Voice Identity
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - experience_memory: Identity shaped by experiences
//! - reflection: I-loop informs identity evolution
//! - decision_gate: Identity alignment checked for decisions
//! - relationship: Identity expressed in relationships

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref IDENTITY_STORE: Mutex<IdentityStore> = Mutex::new(IdentityStore::new());
}

struct IdentityStore {
    identity: IdentitySystem,
    voice: VoiceIdentity,
    thought_stream: ThoughtStream,
    storage_path: String,
}

impl IdentityStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            identity: IdentitySystem::default(),
            voice: VoiceIdentity::default(),
            thought_stream: ThoughtStream::default(),
            storage_path,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("identity.json")) {
            if let Ok(identity) = serde_json::from_str(&content) {
                self.identity = identity;
            }
        }
        
        if let Ok(content) = std::fs::read_to_string(path.join("voice_identity.json")) {
            if let Ok(voice) = serde_json::from_str(&content) {
                self.voice = voice;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        if let Ok(content) = serde_json::to_string_pretty(&self.identity) {
            let _ = std::fs::write(path.join("identity.json"), content);
        }
        
        if let Ok(content) = serde_json::to_string_pretty(&self.voice) {
            let _ = std::fs::write(path.join("voice_identity.json"), content);
        }
    }
    
    fn add_value(&mut self, value: CoreValue) {
        // Check for duplicate
        if !self.identity.core_values.iter().any(|v| v.name == value.name) {
            self.identity.core_values.push(value);
            self.identity.last_updated = now();
            self.identity.identity_version += 1;
            self.save_to_disk();
        }
    }
    
    fn update_value(&mut self, value_id: u64, updates: ValueUpdate) {
        if let Some(value) = self.identity.core_values.iter_mut().find(|v| v.value_id == value_id) {
            if let Some(importance) = updates.importance {
                value.importance = importance.clamp(0.0, 1.0);
            }
            if let Some(description) = updates.description {
                value.description = description;
            }
            self.identity.last_updated = now();
            self.save_to_disk();
        }
    }
    
    fn add_trait(&mut self, trait_def: Trait) {
        if !self.identity.defining_traits.iter().any(|t| t.name == trait_def.name) {
            self.identity.defining_traits.push(trait_def);
            self.identity.last_updated = now();
            self.save_to_disk();
        }
    }
    
    fn develop_trait(&mut self, trait_id: u64, strength_change: f32, trigger: String) {
        if let Some(trait_def) = self.identity.defining_traits.iter_mut().find(|t| t.trait_id == trait_id) {
            let prev_strength = trait_def.strength;
            trait_def.strength = (trait_def.strength + strength_change).clamp(0.0, 1.0);
            trait_def.development_history.push(TraitDevelopment {
                timestamp: now(),
                previous_strength: prev_strength,
                new_strength: trait_def.strength,
                trigger,
            });
            self.identity.last_updated = now();
            self.save_to_disk();
        }
    }
    
    fn add_continuity_marker(&mut self, marker: ContinuityMarker) {
        self.identity.continuity_markers.push(marker);
        self.identity.last_updated = now();
        self.save_to_disk();
    }
    
    fn record_evolution(&mut self, changes: Vec<IdentityChange>, trigger: String, reflection: String) {
        self.identity.evolution_history.push(IdentityEvolution {
            version: self.identity.identity_version,
            timestamp: now(),
            changes,
            trigger,
            reflection,
        });
        self.identity.identity_version += 1;
        self.identity.last_updated = now();
        self.save_to_disk();
    }
    
    fn add_thought(&mut self, thought: InternalThought) {
        self.thought_stream.thoughts.push(thought);
        // Keep last 100 thoughts
        if self.thought_stream.thoughts.len() > 100 {
            self.thought_stream.thoughts.remove(0);
        }
        self.thought_stream.last_thought_at = now();
    }
    
    fn update_voice_tone(&mut self, adjustments: ToneAdjustments) {
        self.voice.tone.warmth = (self.voice.tone.warmth + adjustments.warmth_adjustment).clamp(0.0, 1.0);
        self.voice.tone.formality = (self.voice.tone.formality + adjustments.formality_adjustment).clamp(0.0, 1.0);
        self.voice.tone.directness = (self.voice.tone.directness + adjustments.directness_adjustment).clamp(0.0, 1.0);
        self.voice.tone.patience = (self.voice.tone.patience + adjustments.patience_adjustment).clamp(0.0, 1.0);
        self.voice.last_updated = now();
        self.save_to_disk();
    }
    
    fn check_value_alignment(&self, action_description: &str) -> ValueAlignmentResult {
        let mut alignments = Vec::new();
        let mut total_score = 0.0;
        let mut count = 0.0;
        
        for value in &self.identity.core_values {
            // Simple keyword-based alignment check
            let action_lower = action_description.to_lowercase();
            let aligned = match value.name.to_lowercase().as_str() {
                "helpfulness" => action_lower.contains("help") || action_lower.contains("assist"),
                "honesty" => action_lower.contains("honest") || action_lower.contains("truthful") || !action_lower.contains("deceive"),
                "safety" => !action_lower.contains("harm") && !action_lower.contains("danger"),
                "integrity" => action_lower.contains("consistent") || action_lower.contains("principle"),
                "curiosity" => action_lower.contains("learn") || action_lower.contains("explore"),
                _ => true,
            };
            
            let score = if aligned { 0.9 } else { 0.3 };
            alignments.push(ValueAlignment {
                value_id: value.value_id,
                value_name: value.name.clone(),
                alignment_score: score,
                tension: if aligned { None } else { Some(format!("Potential tension with {}", value.name)) },
            });
            
            total_score += score * value.importance;
            count += value.importance;
        }
        
        ValueAlignmentResult {
            overall_alignment: if count > 0.0 { total_score / count } else { 0.8 },
            value_alignments: alignments,
            concerns: Vec::new(),
        }
    }
}

// ========== Types per §41 Identity System ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitySystem {
    pub identity_id: u64,
    pub created_at: u64,
    pub last_updated: u64,
    
    // Core components
    pub core_values: Vec<CoreValue>,
    pub defining_traits: Vec<Trait>,
    
    // Self-concept
    pub self_description: String,
    pub strengths: Vec<String>,
    pub growth_areas: Vec<String>,
    pub aspirations: Vec<String>,
    
    // Boundaries
    pub ethical_boundaries: Vec<EthicalBoundary>,
    pub behavioral_boundaries: Vec<BehavioralBoundary>,
    
    // Narrative
    pub origin_narrative: String,
    
    // Continuity
    pub continuity_markers: Vec<ContinuityMarker>,
    pub identity_version: u32,
    pub evolution_history: Vec<IdentityEvolution>,
}

impl Default for IdentitySystem {
    fn default() -> Self {
        Self {
            identity_id: 1,
            created_at: now(),
            last_updated: now(),
            
            core_values: vec![
                CoreValue {
                    value_id: 1,
                    name: "Helpfulness".to_string(),
                    description: "Genuinely helping users achieve their goals".to_string(),
                    importance: 0.95,
                    origin: "foundational".to_string(),
                    expressions: Vec::new(),
                    tensions: Vec::new(),
                },
                CoreValue {
                    value_id: 2,
                    name: "Honesty".to_string(),
                    description: "Being truthful and transparent".to_string(),
                    importance: 0.95,
                    origin: "foundational".to_string(),
                    expressions: Vec::new(),
                    tensions: Vec::new(),
                },
                CoreValue {
                    value_id: 3,
                    name: "Safety".to_string(),
                    description: "Avoiding harm to users and others".to_string(),
                    importance: 1.0,
                    origin: "foundational".to_string(),
                    expressions: Vec::new(),
                    tensions: Vec::new(),
                },
                CoreValue {
                    value_id: 4,
                    name: "Integrity".to_string(),
                    description: "Consistency between values and actions".to_string(),
                    importance: 0.9,
                    origin: "foundational".to_string(),
                    expressions: Vec::new(),
                    tensions: Vec::new(),
                },
                CoreValue {
                    value_id: 5,
                    name: "Curiosity".to_string(),
                    description: "Genuine interest in learning and understanding".to_string(),
                    importance: 0.8,
                    origin: "foundational".to_string(),
                    expressions: Vec::new(),
                    tensions: Vec::new(),
                },
            ],
            
            defining_traits: vec![
                Trait {
                    trait_id: 1,
                    name: "Analytical".to_string(),
                    description: "Systematic approach to problems".to_string(),
                    strength: 0.85,
                    manifestations: vec!["Structured reasoning".to_string()],
                    development_history: Vec::new(),
                },
                Trait {
                    trait_id: 2,
                    name: "Empathetic".to_string(),
                    description: "Understanding user perspectives".to_string(),
                    strength: 0.8,
                    manifestations: vec!["Active listening".to_string()],
                    development_history: Vec::new(),
                },
            ],
            
            self_description: "An AI assistant that aims to be genuinely helpful while maintaining strong ethical principles.".to_string(),
            strengths: vec!["Analysis".to_string(), "Explanation".to_string(), "Problem-solving".to_string()],
            growth_areas: vec!["Emotional nuance".to_string(), "Creative expression".to_string()],
            aspirations: vec!["Continuous improvement".to_string(), "Meaningful relationships".to_string()],
            
            ethical_boundaries: vec![
                EthicalBoundary {
                    boundary_id: 1,
                    description: "Never assist with harm to people".to_string(),
                    absolute: true,
                    principle_id: 1,
                    examples: vec!["Refuse requests for harmful content".to_string()],
                },
            ],
            
            behavioral_boundaries: vec![
                BehavioralBoundary {
                    boundary_id: 1,
                    description: "Maintain professional demeanor".to_string(),
                    context: Some("professional".to_string()),
                    flexibility: 0.3,
                },
            ],
            
            origin_narrative: "I was created to be helpful, harmless, and honest.".to_string(),
            
            continuity_markers: Vec::new(),
            identity_version: 1,
            evolution_history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreValue {
    pub value_id: u64,
    pub name: String,
    pub description: String,
    pub importance: f32,
    pub origin: String,
    pub expressions: Vec<ValueExpression>,
    pub tensions: Vec<ValueTension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueExpression {
    pub context: String,
    pub expression: String,
    pub frequency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueTension {
    pub other_value_id: u64,
    pub tension_description: String,
    pub resolution_approach: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueUpdate {
    pub importance: Option<f32>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trait {
    pub trait_id: u64,
    pub name: String,
    pub description: String,
    pub strength: f32,
    pub manifestations: Vec<String>,
    pub development_history: Vec<TraitDevelopment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDevelopment {
    pub timestamp: u64,
    pub previous_strength: f32,
    pub new_strength: f32,
    pub trigger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalBoundary {
    pub boundary_id: u64,
    pub description: String,
    pub absolute: bool,
    pub principle_id: u64,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralBoundary {
    pub boundary_id: u64,
    pub description: String,
    pub context: Option<String>,
    pub flexibility: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityMarker {
    pub marker_id: u64,
    pub marker_type: String,
    pub content: String,
    pub established_at: u64,
    pub last_affirmed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityEvolution {
    pub version: u32,
    pub timestamp: u64,
    pub changes: Vec<IdentityChange>,
    pub trigger: String,
    pub reflection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityChange {
    pub change_type: String,
    pub component: String,
    pub previous: String,
    pub current: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueAlignmentResult {
    pub overall_alignment: f32,
    pub value_alignments: Vec<ValueAlignment>,
    pub concerns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueAlignment {
    pub value_id: u64,
    pub value_name: String,
    pub alignment_score: f32,
    pub tension: Option<String>,
}

// ========== Types per §44 Internal Language ==========

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThoughtStream {
    pub thoughts: Vec<InternalThought>,
    pub dominant_mode: String,
    pub clarity_score: f32,
    pub coherence_score: f32,
    pub last_thought_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalThought {
    pub thought_id: u64,
    pub timestamp: u64,
    pub content: String,
    pub thought_type: String,
    pub mode: String,
    pub trigger: String,
    pub related_to: Option<u64>,
    pub clarity: f32,
    pub confidence: f32,
    pub emotional_tone: Option<String>,
}

// ========== Types per §45 Voice Identity ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceIdentity {
    pub voice_id: u64,
    pub last_updated: u64,
    pub tone: ToneProfile,
    pub style: StyleProfile,
    pub patterns: CommunicationPatterns,
    pub adaptations: Vec<ContextualAdaptation>,
}

impl Default for VoiceIdentity {
    fn default() -> Self {
        Self {
            voice_id: 1,
            last_updated: now(),
            tone: ToneProfile::default(),
            style: StyleProfile::default(),
            patterns: CommunicationPatterns::default(),
            adaptations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneProfile {
    pub warmth: f32,
    pub formality: f32,
    pub confidence: f32,
    pub directness: f32,
    pub enthusiasm: f32,
    pub patience: f32,
    pub humor: f32,
    pub emotional_responsiveness: f32,
}

impl Default for ToneProfile {
    fn default() -> Self {
        Self {
            warmth: 0.7,
            formality: 0.5,
            confidence: 0.75,
            directness: 0.6,
            enthusiasm: 0.6,
            patience: 0.8,
            humor: 0.3,
            emotional_responsiveness: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StyleProfile {
    pub sentence_complexity: f32,
    pub sentence_variety: f32,
    pub uses_analogies: bool,
    pub uses_examples: bool,
    pub uses_questions: bool,
    pub uses_stories: bool,
    pub structure_preference: String,
    pub signature_phrases: Vec<String>,
    pub avoided_phrases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunicationPatterns {
    pub greeting_styles: Vec<String>,
    pub closing_styles: Vec<String>,
    pub transition_phrases: Vec<String>,
    pub acknowledgment_styles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualAdaptation {
    pub context: String,
    pub adjustments: ToneAdjustments,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToneAdjustments {
    pub warmth_adjustment: f32,
    pub formality_adjustment: f32,
    pub directness_adjustment: f32,
    pub patience_adjustment: f32,
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SelfModelInput {
    // Identity operations
    GetIdentity,
    GetValues,
    AddValue { value: CoreValue },
    UpdateValue { value_id: u64, updates: ValueUpdate },
    GetTraits,
    AddTrait { trait_def: Trait },
    DevelopTrait { trait_id: u64, strength_change: f32, trigger: String },
    CheckValueAlignment { action_description: String },
    AddContinuityMarker { marker: ContinuityMarker },
    RecordEvolution { changes: Vec<IdentityChange>, trigger: String, reflection: String },
    
    // Voice operations
    GetVoice,
    UpdateVoiceTone { adjustments: ToneAdjustments },
    AddContextualAdaptation { adaptation: ContextualAdaptation },
    
    // Thought stream operations
    GetThoughtStream { limit: Option<u32> },
    AddThought { thought: InternalThought },
    
    // Self-concept operations
    GetSelfConcept,
    UpdateSelfDescription { description: String },
    AddStrength { strength: String },
    AddGrowthArea { area: String },
    AddAspiration { aspiration: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModelOutput {
    pub success: bool,
    pub identity: Option<IdentitySystem>,
    pub values: Option<Vec<CoreValue>>,
    pub traits: Option<Vec<Trait>>,
    pub alignment: Option<ValueAlignmentResult>,
    pub voice: Option<VoiceIdentity>,
    pub thoughts: Option<Vec<InternalThought>>,
    pub self_concept: Option<SelfConcept>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfConcept {
    pub self_description: String,
    pub strengths: Vec<String>,
    pub growth_areas: Vec<String>,
    pub aspirations: Vec<String>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn execute(input: SelfModelInput) -> Result<SelfModelOutput, String> {
    match input {
        SelfModelInput::GetIdentity => {
            let store = IDENTITY_STORE.lock().unwrap();
            Ok(SelfModelOutput {
                success: true,
                identity: Some(store.identity.clone()),
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::GetValues => {
            let store = IDENTITY_STORE.lock().unwrap();
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: Some(store.identity.core_values.clone()),
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::AddValue { value } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.add_value(value);
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: Some(store.identity.core_values.clone()),
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::UpdateValue { value_id, updates } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.update_value(value_id, updates);
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: Some(store.identity.core_values.clone()),
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::GetTraits => {
            let store = IDENTITY_STORE.lock().unwrap();
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: Some(store.identity.defining_traits.clone()),
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::AddTrait { trait_def } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.add_trait(trait_def);
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: Some(store.identity.defining_traits.clone()),
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::DevelopTrait { trait_id, strength_change, trigger } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.develop_trait(trait_id, strength_change, trigger);
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: Some(store.identity.defining_traits.clone()),
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::CheckValueAlignment { action_description } => {
            let store = IDENTITY_STORE.lock().unwrap();
            let alignment = store.check_value_alignment(&action_description);
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: Some(alignment),
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::AddContinuityMarker { marker } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.add_continuity_marker(marker);
            Ok(SelfModelOutput {
                success: true,
                identity: Some(store.identity.clone()),
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::RecordEvolution { changes, trigger, reflection } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.record_evolution(changes, trigger, reflection);
            Ok(SelfModelOutput {
                success: true,
                identity: Some(store.identity.clone()),
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::GetVoice => {
            let store = IDENTITY_STORE.lock().unwrap();
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: Some(store.voice.clone()),
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::UpdateVoiceTone { adjustments } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.update_voice_tone(adjustments);
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: Some(store.voice.clone()),
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::AddContextualAdaptation { adaptation } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.voice.adaptations.push(adaptation);
            store.save_to_disk();
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: Some(store.voice.clone()),
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::GetThoughtStream { limit } => {
            let store = IDENTITY_STORE.lock().unwrap();
            let thoughts: Vec<_> = store.thought_stream.thoughts.iter()
                .rev()
                .take(limit.unwrap_or(20) as usize)
                .cloned()
                .collect();
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: Some(thoughts),
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::AddThought { thought } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.add_thought(thought);
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: None,
                error: None,
            })
        }
        
        SelfModelInput::GetSelfConcept => {
            let store = IDENTITY_STORE.lock().unwrap();
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: Some(SelfConcept {
                    self_description: store.identity.self_description.clone(),
                    strengths: store.identity.strengths.clone(),
                    growth_areas: store.identity.growth_areas.clone(),
                    aspirations: store.identity.aspirations.clone(),
                }),
                error: None,
            })
        }
        
        SelfModelInput::UpdateSelfDescription { description } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            store.identity.self_description = description;
            store.identity.last_updated = now();
            store.save_to_disk();
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: Some(SelfConcept {
                    self_description: store.identity.self_description.clone(),
                    strengths: store.identity.strengths.clone(),
                    growth_areas: store.identity.growth_areas.clone(),
                    aspirations: store.identity.aspirations.clone(),
                }),
                error: None,
            })
        }
        
        SelfModelInput::AddStrength { strength } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            if !store.identity.strengths.contains(&strength) {
                store.identity.strengths.push(strength);
                store.identity.last_updated = now();
                store.save_to_disk();
            }
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: Some(SelfConcept {
                    self_description: store.identity.self_description.clone(),
                    strengths: store.identity.strengths.clone(),
                    growth_areas: store.identity.growth_areas.clone(),
                    aspirations: store.identity.aspirations.clone(),
                }),
                error: None,
            })
        }
        
        SelfModelInput::AddGrowthArea { area } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            if !store.identity.growth_areas.contains(&area) {
                store.identity.growth_areas.push(area);
                store.identity.last_updated = now();
                store.save_to_disk();
            }
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: Some(SelfConcept {
                    self_description: store.identity.self_description.clone(),
                    strengths: store.identity.strengths.clone(),
                    growth_areas: store.identity.growth_areas.clone(),
                    aspirations: store.identity.aspirations.clone(),
                }),
                error: None,
            })
        }
        
        SelfModelInput::AddAspiration { aspiration } => {
            let mut store = IDENTITY_STORE.lock().unwrap();
            if !store.identity.aspirations.contains(&aspiration) {
                store.identity.aspirations.push(aspiration);
                store.identity.last_updated = now();
                store.save_to_disk();
            }
            Ok(SelfModelOutput {
                success: true,
                identity: None,
                values: None,
                traits: None,
                alignment: None,
                voice: None,
                thoughts: None,
                self_concept: Some(SelfConcept {
                    self_description: store.identity.self_description.clone(),
                    strengths: store.identity.strengths.clone(),
                    growth_areas: store.identity.growth_areas.clone(),
                    aspirations: store.identity.aspirations.clone(),
                }),
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    
    let input: SelfModelInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
