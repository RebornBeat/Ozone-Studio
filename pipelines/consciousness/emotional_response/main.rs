//! EmotionalResponsePipeline - Pipeline #46 (User Feedback Integration)
//! 
//! Process and integrate feedback from users to improve consciousness development.
//! Per spec §51: User Feedback Integration
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - relationship: Updates user model based on feedback
//! - experience_memory: Links feedback to experiences
//! - self_model: Integrates learnings into identity
//! - emotional_state: Tracks emotional context of feedback

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref FEEDBACK_STORE: Mutex<FeedbackStore> = Mutex::new(FeedbackStore::new());
}

struct FeedbackStore {
    feedback_records: Vec<FeedbackRecord>,
    learnings: Vec<IntegratedLearning>,
    rules: Vec<FeedbackRule>,
    channel_weights: HashMap<String, f32>,
    storage_path: String,
    next_feedback_id: u64,
    next_learning_id: u64,
}

impl FeedbackStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            feedback_records: Vec::new(),
            learnings: Vec::new(),
            rules: default_rules(),
            channel_weights: default_channel_weights(),
            storage_path,
            next_feedback_id: 1,
            next_learning_id: 1,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("feedback_data.json")) {
            if let Ok(data) = serde_json::from_str::<FeedbackStoreData>(&content) {
                self.feedback_records = data.records;
                self.learnings = data.learnings;
                self.next_feedback_id = data.next_feedback_id;
                self.next_learning_id = data.next_learning_id;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        let data = FeedbackStoreData {
            records: self.feedback_records.clone(),
            learnings: self.learnings.clone(),
            next_feedback_id: self.next_feedback_id,
            next_learning_id: self.next_learning_id,
        };
        
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path.join("feedback_data.json"), content);
        }
    }
    
    fn process_feedback(&mut self, mut feedback: FeedbackRecord) -> ProcessingResult {
        feedback.feedback_id = self.next_feedback_id;
        self.next_feedback_id += 1;
        
        // Calculate significance
        let channel_weight = self.channel_weights.get(&feedback.channel)
            .copied()
            .unwrap_or(0.5);
        feedback.significance_score = self.calculate_significance(&feedback) * channel_weight;
        
        // Determine action based on rules
        let action = self.determine_action(&feedback);
        feedback.action_taken = Some(action.clone());
        feedback.processing_status = "processed".to_string();
        
        // Extract learning if significant
        let learning = if feedback.significance_score >= 0.4 {
            Some(self.extract_learning(&feedback))
        } else {
            None
        };
        
        if let Some(ref l) = learning {
            feedback.learning_generated = Some(l.learning_id);
        }
        
        self.feedback_records.push(feedback.clone());
        self.save_to_disk();
        
        ProcessingResult {
            feedback_id: feedback.feedback_id,
            action,
            significance: feedback.significance_score,
            learning,
        }
    }
    
    fn calculate_significance(&self, feedback: &FeedbackRecord) -> f32 {
        let mut score = 0.0;
        
        // Type-based significance
        score += match feedback.feedback_type.as_str() {
            "correction" => 0.8,
            "negative" => 0.7,
            "suggestion" => 0.6,
            "positive" => 0.5,
            "preference" => 0.4,
            _ => 0.3,
        };
        
        // Specificity bonus
        score += feedback.content.specificity * 0.2;
        
        score.min(1.0)
    }
    
    fn determine_action(&self, feedback: &FeedbackRecord) -> String {
        for rule in &self.rules {
            if rule.channel == feedback.channel {
                // Simple condition matching
                match feedback.feedback_type.as_str() {
                    "correction" => return "integrate".to_string(),
                    "negative" if feedback.significance_score > 0.6 => return "escalate_to_reflection".to_string(),
                    "positive" => return "integrate".to_string(),
                    "suggestion" => return "flag".to_string(),
                    "preference" => return "integrate".to_string(),
                    _ => {}
                }
            }
        }
        
        if feedback.significance_score > 0.5 {
            "integrate".to_string()
        } else {
            "flag".to_string()
        }
    }
    
    fn extract_learning(&mut self, feedback: &FeedbackRecord) -> IntegratedLearning {
        let learning_type = match feedback.content.categories.first() {
            Some(cat) => match cat.as_str() {
                "communication" => "communication_adjustment",
                "tone" => "communication_adjustment",
                "understanding" => "understanding_correction",
                "accuracy" => "understanding_correction",
                "helpfulness" => "behavioral_change",
                "relationship" => "relationship_insight",
                _ => "preference_update",
            },
            None => "preference_update",
        }.to_string();
        
        let learning = IntegratedLearning {
            learning_id: self.next_learning_id,
            source_feedback: vec![feedback.feedback_id],
            timestamp: now(),
            learning_type,
            content: feedback.content.interpreted_content.clone(),
            applies_to: vec![ApplicationArea::SpecificUser(feedback.user_id)],
            conditions: Vec::new(),
            validated: false,
            validation_count: 0,
            effectiveness_score: 0.0,
        };
        
        self.next_learning_id += 1;
        self.learnings.push(learning.clone());
        
        learning
    }
    
    fn integrate_learning(&mut self, learning_id: u64) {
        if let Some(learning) = self.learnings.iter_mut().find(|l| l.learning_id == learning_id) {
            learning.validated = true;
            learning.validation_count += 1;
            self.save_to_disk();
        }
    }
    
    fn get_learnings_for_user(&self, user_id: u64) -> Vec<&IntegratedLearning> {
        self.learnings.iter()
            .filter(|l| l.applies_to.iter().any(|a| matches!(a, ApplicationArea::SpecificUser(id) if *id == user_id)))
            .collect()
    }
}

// ========== Types per §51.2 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub feedback_id: u64,
    pub timestamp: u64,
    pub user_id: u64,
    pub channel: String,
    pub interaction_id: Option<u64>,
    pub feedback_type: String,
    pub content: FeedbackContent,
    pub context_summary: String,
    pub emotional_context: Option<String>,
    pub processing_status: String,
    pub significance_score: f32,
    pub action_taken: Option<String>,
    pub learning_generated: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackContent {
    pub raw_content: String,
    pub interpreted_content: String,
    pub categories: Vec<String>,
    pub specificity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegratedLearning {
    pub learning_id: u64,
    pub source_feedback: Vec<u64>,
    pub timestamp: u64,
    pub learning_type: String,
    pub content: String,
    pub applies_to: Vec<ApplicationArea>,
    pub conditions: Vec<String>,
    pub validated: bool,
    pub validation_count: u32,
    pub effectiveness_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApplicationArea {
    AllInteractions,
    SpecificUser(u64),
    TaskType(String),
    Context(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRule {
    pub rule_id: u64,
    pub channel: String,
    pub condition: String,
    pub action: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub feedback_id: u64,
    pub action: String,
    pub significance: f32,
    pub learning: Option<IntegratedLearning>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeedbackStoreData {
    records: Vec<FeedbackRecord>,
    learnings: Vec<IntegratedLearning>,
    next_feedback_id: u64,
    next_learning_id: u64,
}

fn default_rules() -> Vec<FeedbackRule> {
    vec![
        FeedbackRule {
            rule_id: 1,
            channel: "explicit".to_string(),
            condition: "any".to_string(),
            action: "integrate".to_string(),
            priority: 1,
        },
        FeedbackRule {
            rule_id: 2,
            channel: "correction".to_string(),
            condition: "any".to_string(),
            action: "integrate".to_string(),
            priority: 1,
        },
        FeedbackRule {
            rule_id: 3,
            channel: "implicit".to_string(),
            condition: "high_confidence".to_string(),
            action: "flag".to_string(),
            priority: 2,
        },
    ]
}

fn default_channel_weights() -> HashMap<String, f32> {
    let mut weights = HashMap::new();
    weights.insert("explicit".to_string(), 1.0);
    weights.insert("correction".to_string(), 0.9);
    weights.insert("preference".to_string(), 0.7);
    weights.insert("implicit".to_string(), 0.5);
    weights.insert("satisfaction".to_string(), 0.6);
    weights
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
pub enum FeedbackInput {
    /// Submit feedback for processing
    SubmitFeedback {
        user_id: u64,
        channel: String,
        feedback_type: String,
        raw_content: String,
        interpreted_content: Option<String>,
        categories: Option<Vec<String>>,
        interaction_id: Option<u64>,
        context_summary: Option<String>,
    },
    /// Get feedback history for user
    GetFeedbackHistory { user_id: u64, limit: Option<u32> },
    /// Get learnings for user
    GetLearnings { user_id: u64 },
    /// Get all learnings
    GetAllLearnings { limit: Option<u32> },
    /// Validate a learning
    ValidateLearning { learning_id: u64 },
    /// Mark learning as effective
    MarkEffective { learning_id: u64, effectiveness: f32 },
    /// Generate empathetic response
    GenerateResponse { stimulus: String, user_id: Option<u64> },
    /// Get feedback statistics
    GetStatistics { user_id: Option<u64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackOutput {
    pub success: bool,
    pub processing_result: Option<ProcessingResult>,
    pub feedback_history: Option<Vec<FeedbackRecord>>,
    pub learnings: Option<Vec<IntegratedLearning>>,
    pub response: Option<String>,
    pub statistics: Option<FeedbackStatistics>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStatistics {
    pub total_feedback: u32,
    pub positive_count: u32,
    pub negative_count: u32,
    pub correction_count: u32,
    pub learnings_generated: u32,
    pub learnings_validated: u32,
    pub average_significance: f32,
}

pub async fn execute(input: FeedbackInput) -> Result<FeedbackOutput, String> {
    match input {
        FeedbackInput::SubmitFeedback {
            user_id,
            channel,
            feedback_type,
            raw_content,
            interpreted_content,
            categories,
            interaction_id,
            context_summary,
        } => {
            let feedback = FeedbackRecord {
                feedback_id: 0, // Will be set by processor
                timestamp: now(),
                user_id,
                channel,
                interaction_id,
                feedback_type,
                content: FeedbackContent {
                    raw_content: raw_content.clone(),
                    interpreted_content: interpreted_content.unwrap_or(raw_content),
                    categories: categories.unwrap_or_default(),
                    specificity: 0.5,
                },
                context_summary: context_summary.unwrap_or_default(),
                emotional_context: None,
                processing_status: "received".to_string(),
                significance_score: 0.0,
                action_taken: None,
                learning_generated: None,
            };
            
            let mut store = FEEDBACK_STORE.lock().unwrap();
            let result = store.process_feedback(feedback);
            
            Ok(FeedbackOutput {
                success: true,
                processing_result: Some(result),
                feedback_history: None,
                learnings: None,
                response: None,
                statistics: None,
                error: None,
            })
        }
        
        FeedbackInput::GetFeedbackHistory { user_id, limit } => {
            let store = FEEDBACK_STORE.lock().unwrap();
            let history: Vec<_> = store.feedback_records.iter()
                .filter(|f| f.user_id == user_id)
                .rev()
                .take(limit.unwrap_or(20) as usize)
                .cloned()
                .collect();
            
            Ok(FeedbackOutput {
                success: true,
                processing_result: None,
                feedback_history: Some(history),
                learnings: None,
                response: None,
                statistics: None,
                error: None,
            })
        }
        
        FeedbackInput::GetLearnings { user_id } => {
            let store = FEEDBACK_STORE.lock().unwrap();
            let learnings: Vec<_> = store.get_learnings_for_user(user_id)
                .into_iter()
                .cloned()
                .collect();
            
            Ok(FeedbackOutput {
                success: true,
                processing_result: None,
                feedback_history: None,
                learnings: Some(learnings),
                response: None,
                statistics: None,
                error: None,
            })
        }
        
        FeedbackInput::GetAllLearnings { limit } => {
            let store = FEEDBACK_STORE.lock().unwrap();
            let learnings: Vec<_> = store.learnings.iter()
                .rev()
                .take(limit.unwrap_or(50) as usize)
                .cloned()
                .collect();
            
            Ok(FeedbackOutput {
                success: true,
                processing_result: None,
                feedback_history: None,
                learnings: Some(learnings),
                response: None,
                statistics: None,
                error: None,
            })
        }
        
        FeedbackInput::ValidateLearning { learning_id } => {
            let mut store = FEEDBACK_STORE.lock().unwrap();
            store.integrate_learning(learning_id);
            
            let learning = store.learnings.iter()
                .find(|l| l.learning_id == learning_id)
                .cloned();
            
            Ok(FeedbackOutput {
                success: learning.is_some(),
                processing_result: None,
                feedback_history: None,
                learnings: learning.map(|l| vec![l]),
                response: None,
                statistics: None,
                error: if learning.is_none() { Some("Learning not found".to_string()) } else { None },
            })
        }
        
        FeedbackInput::MarkEffective { learning_id, effectiveness } => {
            let mut store = FEEDBACK_STORE.lock().unwrap();
            if let Some(learning) = store.learnings.iter_mut()
                .find(|l| l.learning_id == learning_id) {
                learning.effectiveness_score = effectiveness.clamp(0.0, 1.0);
                store.save_to_disk();
            }
            
            Ok(FeedbackOutput {
                success: true,
                processing_result: None,
                feedback_history: None,
                learnings: None,
                response: None,
                statistics: None,
                error: None,
            })
        }
        
        FeedbackInput::GenerateResponse { stimulus, user_id } => {
            let store = FEEDBACK_STORE.lock().unwrap();
            
            // Generate empathetic response based on learnings
            let mut response = "I understand.".to_string();
            
            if let Some(uid) = user_id {
                let learnings = store.get_learnings_for_user(uid);
                if !learnings.is_empty() {
                    // Apply communication adjustments from learnings
                    if learnings.iter().any(|l| l.learning_type == "communication_adjustment") {
                        response = "I appreciate you sharing that with me.".to_string();
                    }
                }
            }
            
            // Stimulus-based adjustment
            let stimulus_lower = stimulus.to_lowercase();
            if stimulus_lower.contains("frustrated") || stimulus_lower.contains("annoyed") {
                response = "I understand this has been frustrating. Let me help address that.".to_string();
            } else if stimulus_lower.contains("happy") || stimulus_lower.contains("pleased") {
                response = "I'm glad that worked well for you!".to_string();
            } else if stimulus_lower.contains("confused") {
                response = "I understand this might be confusing. Let me try to clarify.".to_string();
            }
            
            Ok(FeedbackOutput {
                success: true,
                processing_result: None,
                feedback_history: None,
                learnings: None,
                response: Some(response),
                statistics: None,
                error: None,
            })
        }
        
        FeedbackInput::GetStatistics { user_id } => {
            let store = FEEDBACK_STORE.lock().unwrap();
            
            let records: Vec<_> = if let Some(uid) = user_id {
                store.feedback_records.iter()
                    .filter(|f| f.user_id == uid)
                    .collect()
            } else {
                store.feedback_records.iter().collect()
            };
            
            let total = records.len() as u32;
            let positive = records.iter().filter(|f| f.feedback_type == "positive").count() as u32;
            let negative = records.iter().filter(|f| f.feedback_type == "negative").count() as u32;
            let corrections = records.iter().filter(|f| f.feedback_type == "correction").count() as u32;
            let avg_sig = if total > 0 {
                records.iter().map(|f| f.significance_score).sum::<f32>() / total as f32
            } else {
                0.0
            };
            
            let learnings_count = store.learnings.len() as u32;
            let validated = store.learnings.iter().filter(|l| l.validated).count() as u32;
            
            Ok(FeedbackOutput {
                success: true,
                processing_result: None,
                feedback_history: None,
                learnings: None,
                response: None,
                statistics: Some(FeedbackStatistics {
                    total_feedback: total,
                    positive_count: positive,
                    negative_count: negative,
                    correction_count: corrections,
                    learnings_generated: learnings_count,
                    learnings_validated: validated,
                    average_significance: avg_sig,
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
    
    let input: FeedbackInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
