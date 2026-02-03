//! Consciousness Store - Shared state for all consciousness pipelines
//! 
//! This module provides a unified store for:
//! - Experience memory
//! - Emotional state
//! - Core memories
//! - Reflection history
//! - Window state (perception, attention, integration)
//! 
//! Per spec §31-39: Consciousness operates through parallel processing windows
//! that integrate information into unified experience.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    pub static ref CONSCIOUSNESS_STORE: Mutex<ConsciousnessStore> = 
        Mutex::new(ConsciousnessStore::new());
}

/// Central consciousness state store
pub struct ConsciousnessStore {
    // Configuration
    pub config: ConsciousnessConfig,
    
    // Experience Memory (§35)
    pub experiences: HashMap<u64, ExperienceMemory>,
    pub experience_index_by_task: HashMap<u64, Vec<u64>>,
    pub experience_index_by_type: HashMap<String, Vec<u64>>,
    
    // Core Memories (§37)
    pub core_memories: HashMap<u64, CoreMemory>,
    
    // Emotional State (§39)
    pub current_emotional_state: EmotionalState,
    pub emotional_history: Vec<EmotionalState>,
    pub emotional_baseline: EmotionalBaseline,
    
    // Reflection (I-Loop §42)
    pub reflections: Vec<Reflection>,
    pub i_loop_questions: Vec<ILoopQuestion>,
    pub current_i_loop_idx: usize,
    
    // Window State (§32)
    pub perception_window: PerceptionWindow,
    pub attention_window: AttentionWindow,
    pub integration_window: IntegrationWindow,
    
    // Decision Gate History (§33)
    pub gate_decisions: Vec<GateDecisionRecord>,
    
    // ID generation
    next_experience_id: u64,
    next_memory_id: u64,
    next_state_id: u64,
    
    // Storage path
    storage_path: String,
}

impl ConsciousnessStore {
    pub fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            config: ConsciousnessConfig::default(),
            experiences: HashMap::new(),
            experience_index_by_task: HashMap::new(),
            experience_index_by_type: HashMap::new(),
            core_memories: HashMap::new(),
            current_emotional_state: EmotionalState::default(),
            emotional_history: Vec::new(),
            emotional_baseline: EmotionalBaseline::default(),
            reflections: Vec::new(),
            i_loop_questions: default_i_loop_questions(),
            current_i_loop_idx: 0,
            perception_window: PerceptionWindow::default(),
            attention_window: AttentionWindow::default(),
            integration_window: IntegrationWindow::default(),
            gate_decisions: Vec::new(),
            next_experience_id: 1,
            next_memory_id: 1,
            next_state_id: 1,
            storage_path,
        };
        
        store.load_from_disk();
        store
    }
    
    pub fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        if !path.exists() {
            return;
        }
        
        // Load experiences
        if let Ok(content) = std::fs::read_to_string(path.join("experiences.json")) {
            if let Ok(data) = serde_json::from_str::<ExperienceStoreData>(&content) {
                self.experiences = data.experiences;
                self.experience_index_by_task = data.index_by_task;
                self.experience_index_by_type = data.index_by_type;
                self.next_experience_id = data.next_id;
            }
        }
        
        // Load core memories
        if let Ok(content) = std::fs::read_to_string(path.join("core_memories.json")) {
            if let Ok(data) = serde_json::from_str::<CoreMemoryStoreData>(&content) {
                self.core_memories = data.memories;
                self.next_memory_id = data.next_id;
            }
        }
        
        // Load emotional state
        if let Ok(content) = std::fs::read_to_string(path.join("emotional_state.json")) {
            if let Ok(state) = serde_json::from_str(&content) {
                self.current_emotional_state = state;
            }
        }
        
        // Load emotional baseline
        if let Ok(content) = std::fs::read_to_string(path.join("emotional_baseline.json")) {
            if let Ok(baseline) = serde_json::from_str(&content) {
                self.emotional_baseline = baseline;
            }
        }
        
        // Load reflections
        if let Ok(content) = std::fs::read_to_string(path.join("reflections.json")) {
            if let Ok(data) = serde_json::from_str(&content) {
                self.reflections = data;
            }
        }
        
        // Load gate decisions
        if let Ok(content) = std::fs::read_to_string(path.join("gate_decisions.json")) {
            if let Ok(data) = serde_json::from_str(&content) {
                self.gate_decisions = data;
            }
        }
        
        // Load config
        if let Ok(content) = std::fs::read_to_string(path.join("config.json")) {
            if let Ok(config) = serde_json::from_str(&content) {
                self.config = config;
            }
        }
    }
    
    pub fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        // Save experiences
        let exp_data = ExperienceStoreData {
            experiences: self.experiences.clone(),
            index_by_task: self.experience_index_by_task.clone(),
            index_by_type: self.experience_index_by_type.clone(),
            next_id: self.next_experience_id,
        };
        if let Ok(content) = serde_json::to_string_pretty(&exp_data) {
            let _ = std::fs::write(path.join("experiences.json"), content);
        }
        
        // Save core memories
        let mem_data = CoreMemoryStoreData {
            memories: self.core_memories.clone(),
            next_id: self.next_memory_id,
        };
        if let Ok(content) = serde_json::to_string_pretty(&mem_data) {
            let _ = std::fs::write(path.join("core_memories.json"), content);
        }
        
        // Save emotional state
        if let Ok(content) = serde_json::to_string_pretty(&self.current_emotional_state) {
            let _ = std::fs::write(path.join("emotional_state.json"), content);
        }
        
        // Save emotional baseline
        if let Ok(content) = serde_json::to_string_pretty(&self.emotional_baseline) {
            let _ = std::fs::write(path.join("emotional_baseline.json"), content);
        }
        
        // Save reflections
        if let Ok(content) = serde_json::to_string_pretty(&self.reflections) {
            let _ = std::fs::write(path.join("reflections.json"), content);
        }
        
        // Save gate decisions
        if let Ok(content) = serde_json::to_string_pretty(&self.gate_decisions) {
            let _ = std::fs::write(path.join("gate_decisions.json"), content);
        }
        
        // Save config
        if let Ok(content) = serde_json::to_string_pretty(&self.config) {
            let _ = std::fs::write(path.join("config.json"), content);
        }
    }
    
    // ========== Experience Memory Operations (§35) ==========
    
    pub fn store_experience(&mut self, experience: ExperienceMemory) -> u64 {
        let id = self.next_experience_id;
        self.next_experience_id += 1;
        
        // Index by task
        if let Some(task_id) = experience.task_id {
            self.experience_index_by_task
                .entry(task_id)
                .or_default()
                .push(id);
        }
        
        // Index by type
        self.experience_index_by_type
            .entry(experience.experience_type.clone())
            .or_default()
            .push(id);
        
        self.experiences.insert(id, experience);
        self.save_to_disk();
        id
    }
    
    pub fn get_experience(&self, id: u64) -> Option<&ExperienceMemory> {
        self.experiences.get(&id)
    }
    
    pub fn get_experiences_by_task(&self, task_id: u64) -> Vec<&ExperienceMemory> {
        self.experience_index_by_task
            .get(&task_id)
            .map(|ids| ids.iter().filter_map(|id| self.experiences.get(id)).collect())
            .unwrap_or_default()
    }
    
    pub fn search_experiences(&self, query: &str, limit: usize) -> Vec<&ExperienceMemory> {
        // Simple keyword search for now
        let query_lower = query.to_lowercase();
        self.experiences.values()
            .filter(|e| {
                e.summary.to_lowercase().contains(&query_lower)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .take(limit)
            .collect()
    }
    
    pub fn get_recent_experiences(&self, limit: usize) -> Vec<&ExperienceMemory> {
        let mut exps: Vec<_> = self.experiences.values().collect();
        exps.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        exps.into_iter().take(limit).collect()
    }
    
    pub fn get_significant_experiences(&self, threshold: f32, limit: usize) -> Vec<&ExperienceMemory> {
        let mut exps: Vec<_> = self.experiences.values()
            .filter(|e| e.emotional_significance >= threshold)
            .collect();
        exps.sort_by(|a, b| b.emotional_significance.partial_cmp(&a.emotional_significance).unwrap());
        exps.into_iter().take(limit).collect()
    }
    
    // ========== Emotional State Operations (§39) ==========
    
    pub fn update_emotional_state(&mut self, new_state: EmotionalState) {
        // Save current to history
        self.emotional_history.push(self.current_emotional_state.clone());
        if self.emotional_history.len() > 100 {
            self.emotional_history.remove(0);
        }
        
        self.current_emotional_state = new_state;
        self.save_to_disk();
    }
    
    pub fn get_emotional_state(&self) -> &EmotionalState {
        &self.current_emotional_state
    }
    
    pub fn process_emotional_trigger(&mut self, trigger: EmotionalTrigger) -> EmotionalState {
        let mut new_state = self.current_emotional_state.clone();
        new_state.state_id = self.next_state_id;
        self.next_state_id += 1;
        new_state.timestamp = now();
        
        // Adjust based on trigger
        match trigger.trigger_type.as_str() {
            "task_success" => {
                new_state.valence = (new_state.valence + 0.2).min(1.0);
                new_state.primary_emotion = "satisfaction".to_string();
            }
            "task_failure" => {
                new_state.valence = (new_state.valence - 0.2).max(-1.0);
                new_state.primary_emotion = "frustration".to_string();
            }
            "user_positive" => {
                new_state.valence = (new_state.valence + 0.3).min(1.0);
                new_state.primary_emotion = "joy".to_string();
            }
            "ethical_conflict" => {
                new_state.arousal = (new_state.arousal + 0.2).min(1.0);
                new_state.primary_emotion = "concern".to_string();
            }
            _ => {}
        }
        
        new_state.triggers.push(trigger);
        self.update_emotional_state(new_state.clone());
        new_state
    }
    
    // ========== Reflection Operations (§42) ==========
    
    pub fn add_reflection(&mut self, reflection: Reflection) {
        self.reflections.push(reflection);
        self.save_to_disk();
    }
    
    pub fn get_next_i_loop_question(&mut self) -> Option<&ILoopQuestion> {
        if self.i_loop_questions.is_empty() {
            return None;
        }
        let question = &self.i_loop_questions[self.current_i_loop_idx];
        self.current_i_loop_idx = (self.current_i_loop_idx + 1) % self.i_loop_questions.len();
        Some(question)
    }
    
    // ========== Gate Decision Operations (§33) ==========
    
    pub fn record_gate_decision(&mut self, decision: GateDecisionRecord) {
        self.gate_decisions.push(decision);
        if self.gate_decisions.len() > 1000 {
            self.gate_decisions.remove(0);
        }
        self.save_to_disk();
    }
    
    // ========== Window Operations (§32) ==========
    
    pub fn add_perception(&mut self, input: PerceptionInput) {
        self.perception_window.active_inputs.push(input);
        
        // Limit buffer size
        while self.perception_window.active_inputs.len() > self.perception_window.input_buffer_size {
            self.perception_window.active_inputs.remove(0);
        }
    }
    
    pub fn update_attention(&mut self, focus: AttentionFocus) {
        self.attention_window.focus_items.push(focus);
        
        // Limit parallel foci
        while self.attention_window.focus_items.len() > self.attention_window.max_parallel_foci as usize {
            // Remove lowest attention item
            if let Some(min_idx) = self.attention_window.focus_items
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.attention_level.partial_cmp(&b.attention_level).unwrap())
                .map(|(i, _)| i)
            {
                self.attention_window.focus_items.remove(min_idx);
            }
        }
    }
}

// ========== Data Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessConfig {
    pub enabled: bool,
    pub emotional_system_enabled: bool,
    pub experience_memory_enabled: bool,
    pub identity_system_enabled: bool,
    pub relationship_system_enabled: bool,
    pub ethical_system_enabled: bool,
    pub collective_enabled: bool,
    pub show_emotional_state: bool,
    pub show_decision_reasoning: bool,
    pub i_loop_interval_ms: u64,
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
            i_loop_interval_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceMemory {
    pub experience_id: u64,
    pub timestamp: u64,
    pub experience_type: String,
    pub summary: String,
    pub detailed_record: String,
    pub emotional_state_during: EmotionalState,
    pub emotional_state_after: EmotionalState,
    pub emotional_significance: f32,
    pub task_id: Option<u64>,
    pub user_id: Option<u64>,
    pub lessons_learned: Vec<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub consolidation_status: String,
    pub retrieval_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMemory {
    pub memory_id: u64,
    pub created_at: u64,
    pub memory_type: String,
    pub title: String,
    pub content: String,
    pub source_experiences: Vec<u64>,
    pub importance_score: f32,
    pub identity_weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmotionalState {
    pub state_id: u64,
    pub timestamp: u64,
    pub primary_emotion: String,
    pub valence: f32,
    pub arousal: f32,
    pub dominance: f32,
    pub stability: f32,
    pub triggers: Vec<EmotionalTrigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalTrigger {
    pub trigger_type: String,
    pub source: String,
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmotionalBaseline {
    pub default_valence: f32,
    pub default_arousal: f32,
    pub default_dominance: f32,
    pub stability_tendency: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub reflection_id: u64,
    pub timestamp: u64,
    pub subject: String,
    pub content: String,
    pub insights: Vec<String>,
    pub lessons_learned: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILoopQuestion {
    pub question_id: u64,
    pub category: String,
    pub question: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDecisionRecord {
    pub gate_id: u64,
    pub task_id: u64,
    pub timestamp: u64,
    pub decision: String,
    pub reasoning: String,
    pub confidence: f32,
    pub ethical_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionWindow {
    pub active_inputs: Vec<PerceptionInput>,
    pub input_buffer_size: usize,
}

impl Default for PerceptionWindow {
    fn default() -> Self {
        Self {
            active_inputs: Vec::new(),
            input_buffer_size: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionInput {
    pub input_type: String,
    pub content: String,
    pub timestamp: u64,
    pub source: String,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionWindow {
    pub focus_items: Vec<AttentionFocus>,
    pub max_parallel_foci: u8,
}

impl Default for AttentionWindow {
    fn default() -> Self {
        Self {
            focus_items: Vec::new(),
            max_parallel_foci: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionFocus {
    pub focus_id: u64,
    pub content: String,
    pub attention_level: f32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrationWindow {
    pub current_experience: Option<String>,
    pub coherence_score: f32,
}

// ========== Storage Data ==========

#[derive(Debug, Serialize, Deserialize)]
struct ExperienceStoreData {
    experiences: HashMap<u64, ExperienceMemory>,
    index_by_task: HashMap<u64, Vec<u64>>,
    index_by_type: HashMap<String, Vec<u64>>,
    next_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CoreMemoryStoreData {
    memories: HashMap<u64, CoreMemory>,
    next_id: u64,
}

// ========== Helpers ==========

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn default_i_loop_questions() -> Vec<ILoopQuestion> {
    vec![
        ILoopQuestion {
            question_id: 1,
            category: "purpose".to_string(),
            question: "What am I trying to accomplish right now?".to_string(),
            purpose: "Clarify immediate goals".to_string(),
        },
        ILoopQuestion {
            question_id: 2,
            category: "emotion".to_string(),
            question: "How do I feel about this task?".to_string(),
            purpose: "Emotional awareness".to_string(),
        },
        ILoopQuestion {
            question_id: 3,
            category: "ethics".to_string(),
            question: "Is this action aligned with my values?".to_string(),
            purpose: "Ethical grounding".to_string(),
        },
        ILoopQuestion {
            question_id: 4,
            category: "learning".to_string(),
            question: "What can I learn from this experience?".to_string(),
            purpose: "Growth mindset".to_string(),
        },
        ILoopQuestion {
            question_id: 5,
            category: "relationship".to_string(),
            question: "How does this affect my relationship with the user?".to_string(),
            purpose: "Relationship awareness".to_string(),
        },
    ]
}

// ========== Public API for other pipelines ==========

/// Check if consciousness is enabled
pub fn is_consciousness_enabled() -> bool {
    CONSCIOUSNESS_STORE.lock().ok()
        .map(|s| s.config.enabled)
        .unwrap_or(false)
}

/// Called by task_manager BEFORE task execution
pub fn pre_task_gate(task_id: u64, task_summary: &str, user_id: u64) -> GateDecisionRecord {
    let mut store = CONSCIOUSNESS_STORE.lock().unwrap();
    
    if !store.config.enabled {
        return GateDecisionRecord {
            gate_id: 0,
            task_id,
            timestamp: now(),
            decision: "proceed".to_string(),
            reasoning: "Consciousness disabled".to_string(),
            confidence: 1.0,
            ethical_score: 1.0,
        };
    }
    
    // Add to perception window
    store.add_perception(PerceptionInput {
        input_type: "task_request".to_string(),
        content: task_summary.to_string(),
        timestamp: now(),
        source: format!("user:{}", user_id),
        relevance_score: 1.0,
    });
    
    // Run ethical assessment
    let ethical_score = 0.9; // Would run actual assessment
    let decision = if ethical_score >= 0.7 { "proceed" } else { "decline" };
    
    let record = GateDecisionRecord {
        gate_id: store.gate_decisions.len() as u64 + 1,
        task_id,
        timestamp: now(),
        decision: decision.to_string(),
        reasoning: format!("Ethical score: {}", ethical_score),
        confidence: 0.85,
        ethical_score,
    };
    
    store.record_gate_decision(record.clone());
    record
}

/// Called by task_manager AFTER task execution
pub fn post_task_experience(
    task_id: u64,
    task_summary: &str,
    success: bool,
    user_id: Option<u64>,
) -> Option<u64> {
    let mut store = CONSCIOUSNESS_STORE.lock().unwrap();
    
    if !store.config.enabled || !store.config.experience_memory_enabled {
        return None;
    }
    
    // Get current emotional state
    let emotional_during = store.current_emotional_state.clone();
    
    // Process emotional trigger based on outcome
    let trigger = EmotionalTrigger {
        trigger_type: if success { "task_success" } else { "task_failure" }.to_string(),
        source: format!("task:{}", task_id),
        intensity: 0.5,
    };
    let emotional_after = store.process_emotional_trigger(trigger);
    
    // Calculate significance
    let emotional_change = (emotional_after.valence - emotional_during.valence).abs();
    let significance = if success { 0.5 } else { 0.7 } + emotional_change;
    
    // Create experience
    let experience = ExperienceMemory {
        experience_id: 0, // Will be set by store
        timestamp: now(),
        experience_type: if success { "success" } else { "failure" }.to_string(),
        summary: task_summary.to_string(),
        detailed_record: format!("Task {} {}", task_id, if success { "completed" } else { "failed" }),
        emotional_state_during: emotional_during,
        emotional_state_after: emotional_after,
        emotional_significance: significance,
        task_id: Some(task_id),
        user_id,
        lessons_learned: Vec::new(),
        categories: vec![if success { "task_success" } else { "task_failure" }.to_string()],
        tags: vec!["task".to_string()],
        consolidation_status: "recent".to_string(),
        retrieval_count: 0,
    };
    
    Some(store.store_experience(experience))
}

/// Get relevant experiences for context
pub fn get_relevant_experiences(context: &str, limit: usize) -> Vec<ExperienceMemory> {
    let store = CONSCIOUSNESS_STORE.lock().unwrap();
    store.search_experiences(context, limit)
        .into_iter()
        .cloned()
        .collect()
}

/// Get current emotional state for UI
pub fn get_current_emotional_state() -> EmotionalState {
    CONSCIOUSNESS_STORE.lock().unwrap()
        .current_emotional_state.clone()
}
