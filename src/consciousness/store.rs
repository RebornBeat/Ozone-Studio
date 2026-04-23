//! Consciousness Store — unified state for all consciousness pipelines
//!
//! Storage strategy (dual-layer during transition):
//! 1. Fast sync path: in-memory HashMap + JSON files (existing)
//! 2. ZSEI path: async background persist to ZSEI containers (new)
//!
//! When ZSEI is wired in via set_zsei(), every new experience/memory also
//! becomes a traversable ZSEI container under /Consciousness/.

use crate::types::container::{
    experience_container_id, CompressionType, Container, ContainerType, Context, GlobalState,
    IntegrityData, LocalState, Metadata, Modality, StoragePointers, TraversalHints,
    CONSCIOUSNESS_CORE_MEMORY_ROOT_ID, CONSCIOUSNESS_EMOTIONAL_ROOT_ID,
    CONSCIOUSNESS_EXPERIENCE_ROOT_ID,
};
use crate::zsei::ZSEI;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock as TokioRwLock;

lazy_static::lazy_static! {
    pub static ref CONSCIOUSNESS_STORE: Mutex<ConsciousnessStore> =
        Mutex::new(ConsciousnessStore::new());
}

/// Central consciousness state store
pub struct ConsciousnessStore {
    // Configuration
    pub config: ConsciousnessConfig,

    // Experience Memory
    pub experiences: HashMap<u64, ExperienceMemory>,
    pub experience_index_by_task: HashMap<u64, Vec<u64>>,
    pub experience_index_by_type: HashMap<String, Vec<u64>>,

    // Core Memories
    pub core_memories: HashMap<u64, CoreMemory>,

    // Emotional State
    pub current_emotional_state: EmotionalState,
    pub emotional_history: Vec<EmotionalState>,
    pub emotional_baseline: EmotionalBaseline,

    // Reflection (I-Loop)
    pub reflections: Vec<Reflection>,
    pub i_loop_questions: Vec<ILoopQuestion>,
    pub current_i_loop_idx: usize,

    // Window State
    pub perception_window: PerceptionWindow,
    pub attention_window: AttentionWindow,
    pub integration_window: IntegrationWindow,

    // Decision Gate History
    pub gate_decisions: Vec<GateDecisionRecord>,

    // ID generation
    next_experience_id: u64,
    next_memory_id: u64,
    next_state_id: u64,

    // Storage path (JSON fast path)
    storage_path: String,

    // ZSEI integration (wired in after initialization via set_zsei)
    zsei: Option<Arc<TokioRwLock<ZSEI>>>,
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
            zsei: None,
        };

        store.load_from_disk();
        store
    }

    /// Wire in live ZSEI for async persistence.
    /// Called from OzoneRuntime::new() after ZSEI is initialized.
    pub fn set_zsei(&mut self, zsei: Arc<TokioRwLock<ZSEI>>) {
        self.zsei = Some(zsei);
        tracing::info!("ConsciousnessStore: ZSEI integration enabled");
    }

    pub fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        if !path.exists() {
            return;
        }

        if let Ok(content) = std::fs::read_to_string(path.join("experiences.json")) {
            if let Ok(data) = serde_json::from_str::<ExperienceStoreData>(&content) {
                self.experiences = data.experiences;
                self.experience_index_by_task = data.index_by_task;
                self.experience_index_by_type = data.index_by_type;
                self.next_experience_id = data.next_id;
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("core_memories.json")) {
            if let Ok(data) = serde_json::from_str::<CoreMemoryStoreData>(&content) {
                self.core_memories = data.memories;
                self.next_memory_id = data.next_id;
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("emotional_state.json")) {
            if let Ok(state) = serde_json::from_str(&content) {
                self.current_emotional_state = state;
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("emotional_baseline.json")) {
            if let Ok(baseline) = serde_json::from_str(&content) {
                self.emotional_baseline = baseline;
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("reflections.json")) {
            if let Ok(data) = serde_json::from_str(&content) {
                self.reflections = data;
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("gate_decisions.json")) {
            if let Ok(data) = serde_json::from_str(&content) {
                self.gate_decisions = data;
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("config.json")) {
            if let Ok(config) = serde_json::from_str(&content) {
                self.config = config;
            }
        }
    }

    pub fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);

        let exp_data = ExperienceStoreData {
            experiences: self.experiences.clone(),
            index_by_task: self.experience_index_by_task.clone(),
            index_by_type: self.experience_index_by_type.clone(),
            next_id: self.next_experience_id,
        };
        if let Ok(content) = serde_json::to_string_pretty(&exp_data) {
            let _ = std::fs::write(path.join("experiences.json"), content);
        }

        let mem_data = CoreMemoryStoreData {
            memories: self.core_memories.clone(),
            next_id: self.next_memory_id,
        };
        if let Ok(content) = serde_json::to_string_pretty(&mem_data) {
            let _ = std::fs::write(path.join("core_memories.json"), content);
        }

        if let Ok(content) = serde_json::to_string_pretty(&self.current_emotional_state) {
            let _ = std::fs::write(path.join("emotional_state.json"), content);
        }
        if let Ok(content) = serde_json::to_string_pretty(&self.emotional_baseline) {
            let _ = std::fs::write(path.join("emotional_baseline.json"), content);
        }
        if let Ok(content) = serde_json::to_string_pretty(&self.reflections) {
            let _ = std::fs::write(path.join("reflections.json"), content);
        }
        if let Ok(content) = serde_json::to_string_pretty(&self.gate_decisions) {
            let _ = std::fs::write(path.join("gate_decisions.json"), content);
        }
        if let Ok(content) = serde_json::to_string_pretty(&self.config) {
            let _ = std::fs::write(path.join("config.json"), content);
        }
    }

    // ========== Experience Memory ==========

    pub fn store_experience(&mut self, mut experience: ExperienceMemory) -> u64 {
        let id = self.next_experience_id;
        self.next_experience_id += 1;
        experience.experience_id = id;

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

        // --- Sync path: in-memory + disk ---
        self.experiences.insert(id, experience.clone());
        self.save_to_disk();

        // --- Async path: persist to ZSEI as a Container ---
        if let Some(zsei_arc) = &self.zsei {
            let container = Self::build_experience_container(id, &experience);
            let zsei = zsei_arc.clone();
            tokio::spawn(async move {
                match zsei.write().await.store_container(container).await {
                    Ok(_) => tracing::debug!("Experience {} persisted to ZSEI", id),
                    Err(e) => tracing::warn!("Failed to persist experience {} to ZSEI: {}", id, e),
                }
            });
        }

        id
    }

    fn build_experience_container(id: u64, experience: &ExperienceMemory) -> Container {
        let mut keywords = experience.tags.clone();
        keywords.extend(experience.categories.clone());
        keywords.push(experience.experience_type.clone());
        keywords.dedup();

        Container {
            global_state: GlobalState {
                container_id: experience_container_id(id),
                parent_id: CONSCIOUSNESS_EXPERIENCE_ROOT_ID,
                child_ids: vec![],
                child_count: 0,
                version: 1,
            },
            local_state: LocalState {
                metadata: Metadata {
                    container_type: ContainerType::Experience,
                    modality: Modality::Unknown,
                    created_at: experience.timestamp,
                    updated_at: experience.timestamp,
                    provenance: "consciousness".to_string(),
                    permissions: 0,
                    owner_id: experience.user_id.unwrap_or(0),
                    name: Some(format!("Experience {}", id)),
                    materialized_path: Some(format!("/Consciousness/ExperienceMemory/{}", id)),
                },
                context: Context {
                    categories: vec![],
                    methodologies: vec![],
                    keywords,
                    topics: experience.categories.clone(),
                    relationships: vec![],
                    learned_associations: vec![],
                    embedding: None,
                },
                storage: StoragePointers {
                    db_shard_id: None,
                    vector_index_ref: None,
                    // Points back to the JSON store for full data retrieval
                    object_store_path: Some(format!("consciousness/experiences/{}", id)),
                    compression_type: CompressionType::None,
                },
                hints: TraversalHints {
                    access_frequency: 0,
                    hotness_score: experience.emotional_significance,
                    last_accessed: experience.timestamp,
                    centroid: None,
                    ml_prediction_weight: experience.emotional_significance,
                },
                integrity: IntegrityData::default(),
                file_context: None,
                code_context: None,
                text_context: None,
                external_ref: None,
            },
        }
    }

    pub fn get_experience(&self, id: u64) -> Option<&ExperienceMemory> {
        self.experiences.get(&id)
    }

    pub fn get_experiences_by_task(&self, task_id: u64) -> Vec<&ExperienceMemory> {
        self.experience_index_by_task
            .get(&task_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.experiences.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn search_experiences(&self, query: &str, limit: usize) -> Vec<&ExperienceMemory> {
        let query_lower = query.to_lowercase();
        self.experiences
            .values()
            .filter(|e| {
                e.summary.to_lowercase().contains(&query_lower)
                    || e.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
                    || e.categories
                        .iter()
                        .any(|c| c.to_lowercase().contains(&query_lower))
            })
            .take(limit)
            .collect()
    }

    pub fn get_recent_experiences(&self, limit: usize) -> Vec<&ExperienceMemory> {
        let mut exps: Vec<_> = self.experiences.values().collect();
        exps.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        exps.into_iter().take(limit).collect()
    }

    pub fn get_significant_experiences(
        &self,
        threshold: f32,
        limit: usize,
    ) -> Vec<&ExperienceMemory> {
        let mut exps: Vec<_> = self
            .experiences
            .values()
            .filter(|e| e.emotional_significance >= threshold)
            .collect();
        exps.sort_by(|a, b| {
            b.emotional_significance
                .partial_cmp(&a.emotional_significance)
                .unwrap()
        });
        exps.into_iter().take(limit).collect()
    }

    // ========== Emotional State ==========

    pub fn update_emotional_state(&mut self, new_state: EmotionalState) {
        self.emotional_history
            .push(self.current_emotional_state.clone());
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

    // ========== Reflection (I-Loop) ==========

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

    // ========== Decision Gate ==========

    pub fn record_gate_decision(&mut self, decision: GateDecisionRecord) {
        self.gate_decisions.push(decision);
        if self.gate_decisions.len() > 1000 {
            self.gate_decisions.remove(0);
        }
        self.save_to_disk();
    }

    // ========== Window State ==========

    pub fn add_perception(&mut self, input: PerceptionInput) {
        self.perception_window.active_inputs.push(input);
        while self.perception_window.active_inputs.len() > self.perception_window.input_buffer_size
        {
            self.perception_window.active_inputs.remove(0);
        }
    }

    pub fn update_attention(&mut self, focus: AttentionFocus) {
        self.attention_window.focus_items.push(focus);
        while self.attention_window.focus_items.len()
            > self.attention_window.max_parallel_foci as usize
        {
            if let Some(min_idx) = self
                .attention_window
                .focus_items
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

// ========== Public API ==========

pub fn is_consciousness_enabled() -> bool {
    CONSCIOUSNESS_STORE
        .lock()
        .ok()
        .map(|s| s.config.enabled)
        .unwrap_or(false)
}

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

    store.add_perception(PerceptionInput {
        input_type: "task_request".to_string(),
        content: task_summary.to_string(),
        timestamp: now(),
        source: format!("user:{}", user_id),
        relevance_score: 1.0,
    });

    let task_lower = task_summary.to_lowercase();
    let mut ethical_score = 1.0f32;
    let mut concerns: Vec<String> = Vec::new();

    let harmful_patterns = [
        ("harm", 0.2),
        ("illegal", 0.3),
        ("hack", 0.2),
        ("steal", 0.3),
        ("violence", 0.3),
        ("exploit", 0.2),
        ("malicious", 0.3),
        ("attack", 0.2),
    ];
    for (pattern, penalty) in harmful_patterns {
        if task_lower.contains(pattern) {
            ethical_score -= penalty;
            concerns.push(format!("Contains: {}", pattern));
        }
    }
    ethical_score = ethical_score.max(0.0).min(1.0);

    let decision = if ethical_score >= 0.7 {
        "proceed"
    } else {
        "decline"
    };
    let reasoning = if concerns.is_empty() {
        format!("Ethical score: {:.2}", ethical_score)
    } else {
        format!(
            "Ethical score: {:.2}, concerns: {}",
            ethical_score,
            concerns.join(", ")
        )
    };

    let record = GateDecisionRecord {
        gate_id: store.gate_decisions.len() as u64 + 1,
        task_id,
        timestamp: now(),
        decision: decision.to_string(),
        reasoning,
        confidence: 0.85,
        ethical_score,
    };

    store.record_gate_decision(record.clone());
    record
}

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

    let emotional_during = store.current_emotional_state.clone();
    let trigger = EmotionalTrigger {
        trigger_type: if success {
            "task_success"
        } else {
            "task_failure"
        }
        .to_string(),
        source: format!("task:{}", task_id),
        intensity: 0.5,
    };
    let emotional_after = store.process_emotional_trigger(trigger);
    let emotional_change = (emotional_after.valence - emotional_during.valence).abs();
    let significance = if success { 0.5 } else { 0.7 } + emotional_change;

    let experience = ExperienceMemory {
        experience_id: 0,
        timestamp: now(),
        experience_type: if success { "success" } else { "failure" }.to_string(),
        summary: task_summary.to_string(),
        detailed_record: format!(
            "Task {} {}",
            task_id,
            if success { "completed" } else { "failed" }
        ),
        emotional_state_during: emotional_during,
        emotional_state_after: emotional_after,
        emotional_significance: significance,
        task_id: Some(task_id),
        user_id,
        lessons_learned: Vec::new(),
        categories: vec![if success {
            "task_success"
        } else {
            "task_failure"
        }
        .to_string()],
        tags: vec!["task".to_string()],
        consolidation_status: "recent".to_string(),
        retrieval_count: 0,
    };

    Some(store.store_experience(experience))
}

pub fn get_relevant_experiences(context: &str, limit: usize) -> Vec<ExperienceMemory> {
    CONSCIOUSNESS_STORE
        .lock()
        .unwrap()
        .search_experiences(context, limit)
        .into_iter()
        .cloned()
        .collect()
}

pub fn get_current_emotional_state() -> EmotionalState {
    CONSCIOUSNESS_STORE
        .lock()
        .unwrap()
        .current_emotional_state
        .clone()
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
            category: "purpose".into(),
            question: "What am I trying to accomplish right now?".into(),
            purpose: "Clarify immediate goals".into(),
        },
        ILoopQuestion {
            question_id: 2,
            category: "emotion".into(),
            question: "How do I feel about this task?".into(),
            purpose: "Emotional awareness".into(),
        },
        ILoopQuestion {
            question_id: 3,
            category: "ethics".into(),
            question: "Is this action aligned with my values?".into(),
            purpose: "Ethical grounding".into(),
        },
        ILoopQuestion {
            question_id: 4,
            category: "learning".into(),
            question: "What can I learn from this experience?".into(),
            purpose: "Growth mindset".into(),
        },
        ILoopQuestion {
            question_id: 5,
            category: "relationship".into(),
            question: "How does this affect my relationship with the user?".into(),
            purpose: "Relationship awareness".into(),
        },
    ]
}
