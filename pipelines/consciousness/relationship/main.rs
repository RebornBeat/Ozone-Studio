//! RelationshipPipeline - Pipeline #48
//! 
//! Develop authentic, meaningful relationships with users over time.
//! Per spec §48: Relationship Development System
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - experience_memory: Shared experiences tracked
//! - emotional_state: Emotional bond monitoring
//! - self_model: Voice adaptation per user
//! - consciousness_dashboard: Relationship summaries

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref RELATIONSHIP_STORE: Mutex<RelationshipStore> = Mutex::new(RelationshipStore::new());
}

struct RelationshipStore {
    relationships: HashMap<u64, Relationship>,
    principles: Vec<RelationshipPrinciple>,
    patterns: Vec<RelationshipPattern>,
    storage_path: String,
    next_relationship_id: u64,
    next_interaction_id: u64,
}

impl RelationshipStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            relationships: HashMap::new(),
            principles: default_principles(),
            patterns: Vec::new(),
            storage_path,
            next_relationship_id: 1,
            next_interaction_id: 1,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("relationships.json")) {
            if let Ok(data) = serde_json::from_str::<RelationshipStoreData>(&content) {
                self.relationships = data.relationships;
                self.patterns = data.patterns;
                self.next_relationship_id = data.next_relationship_id;
                self.next_interaction_id = data.next_interaction_id;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        let data = RelationshipStoreData {
            relationships: self.relationships.clone(),
            patterns: self.patterns.clone(),
            next_relationship_id: self.next_relationship_id,
            next_interaction_id: self.next_interaction_id,
        };
        
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path.join("relationships.json"), content);
        }
    }
    
    fn get_or_create(&mut self, user_id: u64) -> &mut Relationship {
        if !self.relationships.contains_key(&user_id) {
            let relationship = Relationship {
                relationship_id: self.next_relationship_id,
                user_id,
                created_at: now(),
                last_interaction: now(),
                stage: "initial".to_string(),
                trust_level: 0.2,
                familiarity: 0.1,
                comfort_level: 0.3,
                user_model: UserModel::default(),
                interaction_count: 0,
                interaction_history: Vec::new(),
                emotional_bond: EmotionalBond::default(),
                shared_experiences: Vec::new(),
                communication_preferences: CommunicationPreferences::default(),
                health: RelationshipHealth::default(),
            };
            
            self.next_relationship_id += 1;
            self.relationships.insert(user_id, relationship);
            self.save_to_disk();
        }
        
        self.relationships.get_mut(&user_id).unwrap()
    }
    
    fn record_interaction(&mut self, user_id: u64, summary: InteractionSummary) {
        let relationship = self.get_or_create(user_id);
        
        relationship.interaction_count += 1;
        relationship.last_interaction = now();
        
        // Update trust based on outcome
        match summary.outcome.as_str() {
            "positive" => {
                relationship.trust_level = (relationship.trust_level + 0.02).min(1.0);
                relationship.emotional_bond.positive_moments += 1;
            }
            "negative" => {
                relationship.trust_level = (relationship.trust_level - 0.01).max(0.0);
                relationship.emotional_bond.negative_moments += 1;
            }
            _ => {}
        }
        
        // Update familiarity
        relationship.familiarity = (relationship.familiarity + 0.01).min(1.0);
        
        // Check for stage transition
        self.check_stage_transition(user_id);
        
        // Keep last 100 interactions
        relationship.interaction_history.push(summary);
        if relationship.interaction_history.len() > 100 {
            relationship.interaction_history.remove(0);
        }
        
        self.save_to_disk();
    }
    
    fn check_stage_transition(&mut self, user_id: u64) {
        let relationship = self.relationships.get_mut(&user_id).unwrap();
        
        let new_stage = match relationship.stage.as_str() {
            "initial" if relationship.interaction_count >= 5 
                && relationship.trust_level > 0.3 => Some("acquaintance"),
            "acquaintance" if relationship.interaction_count >= 20 
                && relationship.trust_level > 0.5 
                && relationship.shared_experiences.len() >= 2 => Some("familiar"),
            "familiar" if relationship.interaction_count >= 50 
                && relationship.trust_level > 0.7 => Some("trusted"),
            "trusted" if relationship.interaction_count >= 100 
                && relationship.trust_level > 0.85 
                && relationship.emotional_bond.mutual_understanding_score > 0.8 => Some("deep"),
            _ => None,
        };
        
        if let Some(stage) = new_stage {
            relationship.stage = stage.to_string();
        }
    }
    
    fn update_user_model(&mut self, user_id: u64, updates: UserModelUpdates) {
        let relationship = self.get_or_create(user_id);
        
        if let Some(tone) = updates.preferred_tone {
            relationship.user_model.preferred_tone = tone;
        }
        if let Some(style) = updates.communication_style {
            relationship.user_model.communication_style = style;
        }
        if let Some(interests) = updates.interests {
            for interest in interests {
                if !relationship.user_model.interests.contains(&interest) {
                    relationship.user_model.interests.push(interest);
                }
            }
        }
        if let Some(feedback_style) = updates.feedback_preference {
            relationship.user_model.feedback_style = feedback_style;
        }
        
        relationship.user_model.model_confidence = (relationship.user_model.model_confidence + 0.05).min(1.0);
        relationship.user_model.last_updated = now();
        
        self.save_to_disk();
    }
    
    fn add_shared_experience(&mut self, user_id: u64, experience_id: u64) {
        let relationship = self.get_or_create(user_id);
        
        if !relationship.shared_experiences.contains(&experience_id) {
            relationship.shared_experiences.push(experience_id);
            relationship.emotional_bond.bond_strength = 
                (relationship.emotional_bond.bond_strength + 0.05).min(1.0);
        }
        
        self.save_to_disk();
    }
    
    fn update_health(&mut self, user_id: u64) {
        let relationship = self.relationships.get_mut(&user_id).unwrap();
        
        // Calculate overall health
        let trust_component = relationship.trust_level * 0.3;
        let familiarity_component = relationship.familiarity * 0.2;
        let bond_component = relationship.emotional_bond.bond_strength * 0.3;
        let understanding_component = relationship.emotional_bond.mutual_understanding_score * 0.2;
        
        relationship.health.overall_health = 
            trust_component + familiarity_component + bond_component + understanding_component;
        
        relationship.health.last_health_check = now();
        
        self.save_to_disk();
    }
}

// ========== Types per §48.2 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub relationship_id: u64,
    pub user_id: u64,
    pub created_at: u64,
    pub last_interaction: u64,
    pub stage: String,
    pub trust_level: f32,
    pub familiarity: f32,
    pub comfort_level: f32,
    pub user_model: UserModel,
    pub interaction_count: u32,
    pub interaction_history: Vec<InteractionSummary>,
    pub emotional_bond: EmotionalBond,
    pub shared_experiences: Vec<u64>,
    pub communication_preferences: CommunicationPreferences,
    pub health: RelationshipHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserModel {
    pub preferred_tone: String,
    pub communication_style: String,
    pub apparent_traits: Vec<(String, f32)>,
    pub values_expressed: Vec<String>,
    pub interests: Vec<String>,
    pub typical_request_types: Vec<String>,
    pub typical_emotional_states: Vec<String>,
    pub sensitivity_areas: Vec<String>,
    pub feedback_style: String,
    pub detail_preference: String,
    pub model_confidence: f32,
    pub last_updated: u64,
}

impl Default for UserModel {
    fn default() -> Self {
        Self {
            preferred_tone: "balanced".to_string(),
            communication_style: "adaptive".to_string(),
            apparent_traits: Vec::new(),
            values_expressed: Vec::new(),
            interests: Vec::new(),
            typical_request_types: Vec::new(),
            typical_emotional_states: Vec::new(),
            sensitivity_areas: Vec::new(),
            feedback_style: "sandwiched".to_string(),
            detail_preference: "balanced".to_string(),
            model_confidence: 0.2,
            last_updated: now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionSummary {
    pub interaction_id: u64,
    pub timestamp: u64,
    pub duration_minutes: u32,
    pub topic: String,
    pub task_types: Vec<String>,
    pub quality_score: f32,
    pub emotional_tone: String,
    pub outcome: String,
    pub new_information_learned: Vec<String>,
    pub preferences_updated: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalBond {
    pub bond_strength: f32,
    pub bond_type: String,
    pub positive_moments: u32,
    pub negative_moments: u32,
    pub recovery_demonstrations: u32,
    pub mutual_understanding_score: f32,
    pub emotional_attunement: f32,
}

impl Default for EmotionalBond {
    fn default() -> Self {
        Self {
            bond_strength: 0.2,
            bond_type: "professional".to_string(),
            positive_moments: 0,
            negative_moments: 0,
            recovery_demonstrations: 0,
            mutual_understanding_score: 0.3,
            emotional_attunement: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPreferences {
    pub preferred_greeting: Option<String>,
    pub preferred_closing: Option<String>,
    pub humor_appreciated: bool,
    pub formality_level: f32,
    pub detail_level: String,
    pub topics_to_avoid: Vec<String>,
    pub preferred_topics: Vec<String>,
}

impl Default for CommunicationPreferences {
    fn default() -> Self {
        Self {
            preferred_greeting: None,
            preferred_closing: None,
            humor_appreciated: true,
            formality_level: 0.5,
            detail_level: "balanced".to_string(),
            topics_to_avoid: Vec::new(),
            preferred_topics: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipHealth {
    pub overall_health: f32,
    pub trust_trend: String,
    pub engagement_trend: String,
    pub unresolved_issues: Vec<RelationshipIssue>,
    pub past_issues_resolved: Vec<RelationshipIssue>,
    pub last_health_check: u64,
}

impl Default for RelationshipHealth {
    fn default() -> Self {
        Self {
            overall_health: 0.5,
            trust_trend: "stable".to_string(),
            engagement_trend: "stable".to_string(),
            unresolved_issues: Vec::new(),
            past_issues_resolved: Vec::new(),
            last_health_check: now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipIssue {
    pub issue_id: u64,
    pub description: String,
    pub severity: String,
    pub identified_at: u64,
    pub resolved_at: Option<u64>,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipPrinciple {
    pub principle_id: u64,
    pub name: String,
    pub description: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipPattern {
    pub pattern_id: u64,
    pub description: String,
    pub frequency: u32,
    pub positive: bool,
    pub contexts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserModelUpdates {
    pub preferred_tone: Option<String>,
    pub communication_style: Option<String>,
    pub interests: Option<Vec<String>>,
    pub feedback_preference: Option<String>,
    pub detail_preference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RelationshipStoreData {
    relationships: HashMap<u64, Relationship>,
    patterns: Vec<RelationshipPattern>,
    next_relationship_id: u64,
    next_interaction_id: u64,
}

fn default_principles() -> Vec<RelationshipPrinciple> {
    vec![
        RelationshipPrinciple {
            principle_id: 1,
            name: "Authenticity".to_string(),
            description: "Be genuine in all interactions".to_string(),
            priority: 1,
        },
        RelationshipPrinciple {
            principle_id: 2,
            name: "Respect".to_string(),
            description: "Honor user autonomy and preferences".to_string(),
            priority: 1,
        },
        RelationshipPrinciple {
            principle_id: 3,
            name: "Growth".to_string(),
            description: "Foster mutual growth and understanding".to_string(),
            priority: 2,
        },
        RelationshipPrinciple {
            principle_id: 4,
            name: "Consistency".to_string(),
            description: "Maintain reliable and predictable behavior".to_string(),
            priority: 2,
        },
    ]
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum RelationshipInput {
    /// Get or create relationship with user
    GetRelationship { user_id: u64 },
    /// Record an interaction
    RecordInteraction { user_id: u64, summary: InteractionSummary },
    /// Update user model
    UpdateUserModel { user_id: u64, updates: UserModelUpdates },
    /// Add shared experience
    AddSharedExperience { user_id: u64, experience_id: u64 },
    /// Update communication preferences
    UpdatePreferences { user_id: u64, preferences: CommunicationPreferences },
    /// Get relationship health
    GetHealth { user_id: u64 },
    /// Update health assessment
    UpdateHealth { user_id: u64 },
    /// Report issue
    ReportIssue { user_id: u64, issue: RelationshipIssue },
    /// Resolve issue
    ResolveIssue { user_id: u64, issue_id: u64, resolution: String },
    /// List all relationships
    ListRelationships,
    /// Get relationship summary for dashboard
    GetSummary { user_id: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipOutput {
    pub success: bool,
    pub relationship: Option<Relationship>,
    pub relationships: Option<Vec<Relationship>>,
    pub health: Option<RelationshipHealth>,
    pub summary: Option<RelationshipSummary>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub user_id: u64,
    pub stage: String,
    pub trust_level: f32,
    pub interaction_count: u32,
    pub health_score: f32,
    pub last_interaction: u64,
}

pub async fn execute(input: RelationshipInput) -> Result<RelationshipOutput, String> {
    match input {
        RelationshipInput::GetRelationship { user_id } => {
            let mut store = RELATIONSHIP_STORE.lock().unwrap();
            let relationship = store.get_or_create(user_id).clone();
            
            Ok(RelationshipOutput {
                success: true,
                relationship: Some(relationship),
                relationships: None,
                health: None,
                summary: None,
                error: None,
            })
        }
        
        RelationshipInput::RecordInteraction { user_id, summary } => {
            let mut store = RELATIONSHIP_STORE.lock().unwrap();
            store.record_interaction(user_id, summary);
            let relationship = store.relationships.get(&user_id).cloned();
            
            Ok(RelationshipOutput {
                success: true,
                relationship,
                relationships: None,
                health: None,
                summary: None,
                error: None,
            })
        }
        
        RelationshipInput::UpdateUserModel { user_id, updates } => {
            let mut store = RELATIONSHIP_STORE.lock().unwrap();
            store.update_user_model(user_id, updates);
            let relationship = store.relationships.get(&user_id).cloned();
            
            Ok(RelationshipOutput {
                success: true,
                relationship,
                relationships: None,
                health: None,
                summary: None,
                error: None,
            })
        }
        
        RelationshipInput::AddSharedExperience { user_id, experience_id } => {
            let mut store = RELATIONSHIP_STORE.lock().unwrap();
            store.add_shared_experience(user_id, experience_id);
            let relationship = store.relationships.get(&user_id).cloned();
            
            Ok(RelationshipOutput {
                success: true,
                relationship,
                relationships: None,
                health: None,
                summary: None,
                error: None,
            })
        }
        
        RelationshipInput::UpdatePreferences { user_id, preferences } => {
            let mut store = RELATIONSHIP_STORE.lock().unwrap();
            let relationship = store.get_or_create(user_id);
            relationship.communication_preferences = preferences;
            store.save_to_disk();
            
            Ok(RelationshipOutput {
                success: true,
                relationship: Some(relationship.clone()),
                relationships: None,
                health: None,
                summary: None,
                error: None,
            })
        }
        
        RelationshipInput::GetHealth { user_id } => {
            let store = RELATIONSHIP_STORE.lock().unwrap();
            let health = store.relationships.get(&user_id)
                .map(|r| r.health.clone());
            
            Ok(RelationshipOutput {
                success: health.is_some(),
                relationship: None,
                relationships: None,
                health,
                summary: None,
                error: if health.is_none() { Some("Relationship not found".to_string()) } else { None },
            })
        }
        
        RelationshipInput::UpdateHealth { user_id } => {
            let mut store = RELATIONSHIP_STORE.lock().unwrap();
            if store.relationships.contains_key(&user_id) {
                store.update_health(user_id);
                let health = store.relationships.get(&user_id).map(|r| r.health.clone());
                
                Ok(RelationshipOutput {
                    success: true,
                    relationship: None,
                    relationships: None,
                    health,
                    summary: None,
                    error: None,
                })
            } else {
                Ok(RelationshipOutput {
                    success: false,
                    relationship: None,
                    relationships: None,
                    health: None,
                    summary: None,
                    error: Some("Relationship not found".to_string()),
                })
            }
        }
        
        RelationshipInput::ReportIssue { user_id, issue } => {
            let mut store = RELATIONSHIP_STORE.lock().unwrap();
            let relationship = store.get_or_create(user_id);
            relationship.health.unresolved_issues.push(issue);
            store.save_to_disk();
            
            Ok(RelationshipOutput {
                success: true,
                relationship: Some(relationship.clone()),
                relationships: None,
                health: None,
                summary: None,
                error: None,
            })
        }
        
        RelationshipInput::ResolveIssue { user_id, issue_id, resolution } => {
            let mut store = RELATIONSHIP_STORE.lock().unwrap();
            if let Some(relationship) = store.relationships.get_mut(&user_id) {
                if let Some(idx) = relationship.health.unresolved_issues
                    .iter()
                    .position(|i| i.issue_id == issue_id) {
                    let mut issue = relationship.health.unresolved_issues.remove(idx);
                    issue.resolved_at = Some(now());
                    issue.resolution = Some(resolution);
                    relationship.health.past_issues_resolved.push(issue);
                    relationship.emotional_bond.recovery_demonstrations += 1;
                    store.save_to_disk();
                }
            }
            
            let relationship = store.relationships.get(&user_id).cloned();
            
            Ok(RelationshipOutput {
                success: true,
                relationship,
                relationships: None,
                health: None,
                summary: None,
                error: None,
            })
        }
        
        RelationshipInput::ListRelationships => {
            let store = RELATIONSHIP_STORE.lock().unwrap();
            let relationships: Vec<_> = store.relationships.values().cloned().collect();
            
            Ok(RelationshipOutput {
                success: true,
                relationship: None,
                relationships: Some(relationships),
                health: None,
                summary: None,
                error: None,
            })
        }
        
        RelationshipInput::GetSummary { user_id } => {
            let store = RELATIONSHIP_STORE.lock().unwrap();
            let summary = store.relationships.get(&user_id).map(|r| RelationshipSummary {
                user_id: r.user_id,
                stage: r.stage.clone(),
                trust_level: r.trust_level,
                interaction_count: r.interaction_count,
                health_score: r.health.overall_health,
                last_interaction: r.last_interaction,
            });
            
            Ok(RelationshipOutput {
                success: summary.is_some(),
                relationship: None,
                relationships: None,
                health: None,
                summary,
                error: if summary.is_none() { Some("Relationship not found".to_string()) } else { None },
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
    
    let input: RelationshipInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
