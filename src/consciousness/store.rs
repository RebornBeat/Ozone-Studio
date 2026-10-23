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
    CONSCIOUSNESS_EXPERIENCE_ROOT_ID,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    pub static ref CONSCIOUSNESS_STORE: Mutex<ConsciousnessStore> =
        Mutex::new(ConsciousnessStore::new());
}

/// In-process "the I-Loop is mid-cycle" flag. Ported from the retired
/// task-manager `consciousness_hooks`, which polled an `i_loop_state.json`
/// file that no host component ever wrote. The host I-Loop (or any caller)
/// flips this with `set_i_loop_active`; the task manager's observer waits on
/// it (bounded) via `wait_for_i_loop_completion`. The file is still honored
/// by `i_loop_busy` so an external I-Loop pipeline that writes it keeps
/// working (nothing dropped).
static I_LOOP_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Lock the global store, surviving a poisoned mutex (a panic in some other
/// holder must not turn every later consciousness call into a second panic).
fn lock_store() -> std::sync::MutexGuard<'static, ConsciousnessStore> {
    CONSCIOUSNESS_STORE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
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

    // Store integration (wired in after initialization via set_store) —
    // the abstract contract, not any concrete backend. See
    // src/orchestrator/adapters.rs (ZseiStoreAdapter = backend #1).
    store: Option<Arc<dyn crate::orchestrator::StoreAccess>>,
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
            store: None,
        };

        store.load_from_disk();
        store
    }

    /// Wire in live ZSEI for async persistence.
    /// Called from OzoneRuntime::new() after ZSEI is initialized.
    /// Wire in a store backend (the abstract contract — swappable).
    pub fn set_store(&mut self, store: Arc<dyn crate::orchestrator::StoreAccess>) {
        self.store = Some(store);
        tracing::info!("ConsciousnessStore: store integration enabled");
    }

    /// Copy the operator's TOML `[consciousness]` config into the store.
    /// Without this the store stays on `ConsciousnessConfig::default()`
    /// (`enabled: false`) and every early-return gate (pre_task_gate,
    /// post_task_experience) is inert regardless of config.toml. Honors the
    /// TOML value either way: `enabled = false` stays false.
    pub fn apply_config(&mut self, c: &crate::config::ConsciousnessConfig) {
        self.config = ConsciousnessConfig {
            enabled: c.enabled,
            emotional_system_enabled: c.emotional_system_enabled,
            experience_memory_enabled: c.experience_memory_enabled,
            identity_system_enabled: c.identity_system_enabled,
            relationship_system_enabled: c.relationship_system_enabled,
            ethical_system_enabled: c.ethical_system_enabled,
            collective_enabled: c.collective_enabled,
            show_emotional_state: c.show_emotional_state,
            show_decision_reasoning: c.show_decision_reasoning,
            i_loop_interval_ms: c.i_loop_interval_ms,
        };
        tracing::info!(enabled = c.enabled, "ConsciousnessStore: config applied from config.toml");
    }

    /// Graph traversal as a sense (TOP_DOWN_REVIEW_GUIDE.md §8, item 2):
    /// the store contract was already wired in via `set_store` for writes
    /// (experience persistence); this is the same handle, exposed for
    /// reads — a consciousness review pass (see `consciousness::review`)
    /// clones this Arc and traverses with it, never holding this struct's
    /// own lock across the await.
    pub fn store_handle(&self) -> Option<Arc<dyn crate::orchestrator::StoreAccess>> {
        self.store.clone()
    }

    pub fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        if !path.exists() {
            return;
        }

        // One-time migration flag: files written by the retired task-manager
        // `consciousness_hooks` (different JSON shapes, same directory) are
        // converted into the store's types instead of silently loading empty
        // and being overwritten on the next save. The original is renamed to
        // `<name>.migrated.bak`, and the migrated state is persisted in store
        // format immediately (end of this function) so a restart before any
        // other save can't lose it.
        let mut migrated = false;

        let exp_path = path.join("experiences.json");
        if let Ok(content) = std::fs::read_to_string(&exp_path) {
            match serde_json::from_str::<ExperienceStoreData>(&content) {
                Ok(data) => {
                    self.experiences = data.experiences;
                    self.experience_index_by_task = data.index_by_task;
                    self.experience_index_by_type = data.index_by_type;
                    self.next_experience_id = data.next_id;
                }
                Err(_) => {
                    if let Some(data) = migrate_hook_experiences(&content) {
                        self.experiences = data.experiences;
                        self.experience_index_by_task = data.index_by_task;
                        self.experience_index_by_type = data.index_by_type;
                        self.next_experience_id = data.next_id;
                        retire_hook_file(&exp_path);
                        migrated = true;
                    }
                }
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("core_memories.json")) {
            if let Ok(data) = serde_json::from_str::<CoreMemoryStoreData>(&content) {
                self.core_memories = data.memories;
                self.next_memory_id = data.next_id;
            }
        }

        let emo_path = path.join("emotional_state.json");
        if let Ok(content) = std::fs::read_to_string(&emo_path) {
            match serde_json::from_str::<EmotionalState>(&content) {
                Ok(state) => self.current_emotional_state = state,
                Err(_) => {
                    if let Some(state) = migrate_hook_emotional_state(&content) {
                        self.current_emotional_state = state;
                        retire_hook_file(&emo_path);
                        migrated = true;
                    }
                }
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

        let gate_path = path.join("gate_decisions.json");
        if let Ok(content) = std::fs::read_to_string(&gate_path) {
            match serde_json::from_str::<Vec<GateDecisionRecord>>(&content) {
                Ok(data) => self.gate_decisions = data,
                Err(_) => {
                    if let Some(data) = migrate_hook_gate_decisions(&content) {
                        self.gate_decisions = data;
                        retire_hook_file(&gate_path);
                        migrated = true;
                    }
                }
            }
        }

        // Perception window — persisted across restarts (the retired hooks
        // kept the last 100 task events in perception_window.json; the store's
        // window was memory-only, so a restart emptied it).
        let perc_path = path.join("perception_window.json");
        if let Ok(content) = std::fs::read_to_string(&perc_path) {
            match serde_json::from_str::<PerceptionWindow>(&content) {
                Ok(window) => self.perception_window = window,
                Err(_) => {
                    if let Some(window) = migrate_hook_perception_window(&content) {
                        self.perception_window = window;
                        retire_hook_file(&perc_path);
                        migrated = true;
                    }
                }
            }
        }

        if let Ok(content) = std::fs::read_to_string(path.join("config.json")) {
            if let Ok(config) = serde_json::from_str(&content) {
                self.config = config;
            }
        }

        if migrated {
            self.save_to_disk();
        }
    }

    /// Persist only the perception window (small, bounded to
    /// `input_buffer_size` entries) — cheap enough to call on every task
    /// lifecycle event, unlike `save_to_disk` which rewrites every store file.
    pub fn save_perception(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        if let Ok(content) = serde_json::to_string_pretty(&self.perception_window) {
            let _ = std::fs::write(path.join("perception_window.json"), content);
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
        if let Ok(content) = serde_json::to_string_pretty(&self.perception_window) {
            let _ = std::fs::write(path.join("perception_window.json"), content);
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

        // --- Async path: persist through the store contract as a record ---
        if let Some(store) = &self.store {
            let container = Self::build_experience_container(id, &experience);
            let value = match serde_json::to_value(&container) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Experience {} serialization failed: {}", id, e);
                    return id;
                }
            };
            let store = store.clone();
            // `try_current` rather than a bare `tokio::spawn`: the JSON
            // fast path above already succeeded, and a caller on a plain
            // thread (no runtime) must not panic the whole store over the
            // optional ZSEI mirror.
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    handle.spawn(async move {
                        match store.create_container(CONSCIOUSNESS_EXPERIENCE_ROOT_ID, value).await {
                            Ok(_) => tracing::debug!("Experience {} persisted via store contract", id),
                            Err(e) => tracing::warn!("Failed to persist experience {} to store: {}", id, e),
                        }
                    });
                }
                Err(_) => tracing::debug!(
                    "Experience {} kept in the JSON store only (no async runtime on this thread)",
                    id
                ),
            }
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
    lock_store().config.enabled
}

/// Mark the I-Loop as mid-cycle (or finished). The task manager's observer
/// waits (bounded) on `i_loop_busy()` before registering new work, so tasks
/// are not front-run past an active reflection cycle.
pub fn set_i_loop_active(active: bool) {
    I_LOOP_ACTIVE.store(active, Ordering::SeqCst);
}

/// True while the I-Loop is mid-cycle: the in-process flag, or (compat with
/// the external I-Loop pipeline crates) `i_loop_state.json` with
/// `"active": true` in the consciousness directory.
pub fn i_loop_busy() -> bool {
    if I_LOOP_ACTIVE.load(Ordering::SeqCst) {
        return true;
    }
    let dir = std::env::var("OZONE_CONSCIOUSNESS_PATH")
        .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
    if let Ok(content) = std::fs::read_to_string(Path::new(&dir).join("i_loop_state.json")) {
        if let Ok(state) = serde_json::from_str::<serde_json::Value>(&content) {
            return state.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
        }
    }
    false
}

/// Wait up to `timeout_ms` for the I-Loop to finish (polls every 100 ms).
/// Returns immediately when it is not busy. Never holds the store mutex.
pub async fn wait_for_i_loop_completion(timeout_ms: u64) {
    let check_interval_ms = 100u64;
    let mut waited = 0u64;
    while i_loop_busy() && waited < timeout_ms {
        tokio::time::sleep(std::time::Duration::from_millis(check_interval_ms)).await;
        waited += check_interval_ms;
    }
}

/// Task-lifecycle perception: the task manager reports every lifecycle event
/// (queued / step update / completed / failed / cancelled) so the perception
/// window — consciousness's view of what the system is doing — stays current
/// and survives a restart. Observe-only; a no-op when consciousness is
/// disabled in config.
pub fn observe_task_event(task_id: u64, task_type: &str, status: &str) {
    let mut store = lock_store();
    if !store.config.enabled {
        return;
    }
    store.add_perception(PerceptionInput {
        input_type: "task_event".to_string(),
        content: format!("task {} ({}) {}", task_id, task_type, status),
        timestamp: now(),
        source: "task-manager".to_string(),
        relevance_score: 0.8,
    });
    store.save_perception();
}

pub fn pre_task_gate(task_id: u64, task_summary: &str, user_id: u64) -> GateDecisionRecord {
    let mut store = lock_store();

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
    // Ported from the retired task-manager hook gate (nothing dropped): a
    // small penalty for requests that handle sensitive data. Union of both
    // gates' pattern sets; decision semantics (score >= 0.7 proceeds) and the
    // store's output shape are unchanged.
    let sensitive_patterns = ["password", "credit card", "ssn", "private key"];
    for pattern in sensitive_patterns {
        if task_lower.contains(pattern) {
            ethical_score -= 0.1;
            concerns.push(format!("Involves sensitive data: {}", pattern));
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

/// Record a task outcome as an experience and (by default) feed the
/// emotional trigger. `error` carries the failure cause when the task failed
/// (it used to be lost — the task manager's hook never received it).
pub fn post_task_experience(
    task_id: u64,
    task_summary: &str,
    success: bool,
    user_id: Option<u64>,
    error: Option<&str>,
) -> Option<u64> {
    post_task_experience_with(task_id, task_summary, success, user_id, error, None, true)
}

/// Full form of `post_task_experience`. `tokens_used` is kept on the
/// experience (the retired hook recorded it); `update_emotion = false`
/// records the experience WITHOUT moving the emotional state — used by the
/// task manager for loop-generated tasks (`source: "amt-loop"`,
/// `kind: "loop-pass"`), whose frequent failures would otherwise pin
/// valence at -1.0 permanently.
pub fn post_task_experience_with(
    task_id: u64,
    task_summary: &str,
    success: bool,
    user_id: Option<u64>,
    error: Option<&str>,
    tokens_used: Option<u32>,
    update_emotion: bool,
) -> Option<u64> {
    let mut store = lock_store();

    if !store.config.enabled || !store.config.experience_memory_enabled {
        return None;
    }

    let emotional_during = store.current_emotional_state.clone();
    let emotion_live = update_emotion && store.config.emotional_system_enabled;
    let emotional_after = if emotion_live {
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
        store.process_emotional_trigger(trigger)
    } else {
        emotional_during.clone()
    };
    let emotional_change = (emotional_after.valence - emotional_during.valence).abs();
    let significance = if success { 0.5 } else { 0.7 } + emotional_change;

    let error_text: Option<String> = error
        .filter(|e| !e.is_empty())
        .map(|e| e.chars().take(500).collect());
    let mut detailed_record = format!(
        "Task {} {}",
        task_id,
        if success { "completed" } else { "failed" }
    );
    if let Some(t) = tokens_used {
        detailed_record.push_str(&format!("; tokens_used={}", t));
    }
    if let Some(e) = &error_text {
        detailed_record.push_str(&format!("; error: {}", e));
    }

    let mut tags = vec!["task".to_string()];
    if !emotion_live {
        tags.push("no-emotion-update".to_string());
    }

    let experience = ExperienceMemory {
        experience_id: 0,
        timestamp: now(),
        experience_type: if success { "success" } else { "failure" }.to_string(),
        summary: task_summary.to_string(),
        detailed_record,
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
        tags,
        consolidation_status: "recent".to_string(),
        retrieval_count: 0,
        tokens_used,
        error: error_text,
    };

    Some(store.store_experience(experience))
}

pub fn get_relevant_experiences(context: &str, limit: usize) -> Vec<ExperienceMemory> {
    lock_store()
        .search_experiences(context, limit)
        .into_iter()
        .cloned()
        .collect()
}

pub fn get_current_emotional_state() -> EmotionalState {
    lock_store().current_emotional_state.clone()
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
    /// Tokens the task consumed (the retired task-manager hook recorded
    /// this). `serde(default)` keeps previously persisted experiences loadable.
    #[serde(default)]
    pub tokens_used: Option<u32>,
    /// Failure cause for failed tasks (first 500 chars). Previously lost.
    #[serde(default)]
    pub error: Option<String>,
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
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ========== One-time migration of retired task-manager hook files ==========
//
// The file-based `task::consciousness_hooks` (now thin delegates to this
// store) wrote experiences.json / emotional_state.json / gate_decisions.json /
// perception_window.json into the SAME directory in shapes the store cannot
// parse. Each helper below recognizes the hook shape (and refuses store-shaped
// files, so a merely corrupt store file is never "migrated"), converts it into
// the store's types, and returns None for anything else.

/// Rename a migrated hook file to `<name>.migrated.bak` (best effort, logged).
fn retire_hook_file(path: &Path) {
    let mut bak = path.as_os_str().to_owned();
    bak.push(".migrated.bak");
    let bak = std::path::PathBuf::from(bak);
    match std::fs::rename(path, &bak) {
        Ok(()) => tracing::info!(
            "ConsciousnessStore: migrated hook-format {:?} into store format (original kept as {:?})",
            path,
            bak
        ),
        Err(e) => tracing::warn!(
            "ConsciousnessStore: migrated hook-format {:?} but could not rename it to {:?}: {}",
            path,
            bak,
            e
        ),
    }
}

fn migrate_hook_experiences(content: &str) -> Option<ExperienceStoreData> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    // Store-format files always carry `index_by_type`; the hook never wrote it.
    if v.get("index_by_type").is_some() {
        return None;
    }
    let exps = v.get("experiences")?.as_object()?;
    let mut next_id = v.get("next_id").and_then(|n| n.as_u64()).unwrap_or(1);
    let mut experiences: HashMap<u64, ExperienceMemory> = HashMap::new();
    let mut index_by_task: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut index_by_type: HashMap<String, Vec<u64>> = HashMap::new();

    for (key, e) in exps {
        let Ok(id) = key.parse::<u64>() else { continue };
        let success = e
            .get("experience_type")
            .and_then(|t| t.as_str())
            .map(|t| t == "task_success" || t == "success")
            .unwrap_or(true);
        let task_id = e.get("task_id").and_then(|t| t.as_u64());
        let experience_type = if success { "success" } else { "failure" }.to_string();
        let tokens_used = e
            .get("tokens_used")
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);
        let mem = ExperienceMemory {
            experience_id: id,
            timestamp: e.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0),
            experience_type: experience_type.clone(),
            summary: e
                .get("summary")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            detailed_record: format!(
                "Task {} {} (migrated from task-manager hook record)",
                task_id.map(|t| t.to_string()).unwrap_or_else(|| "?".to_string()),
                if success { "completed" } else { "failed" }
            ),
            emotional_state_during: EmotionalState::default(),
            emotional_state_after: EmotionalState::default(),
            emotional_significance: e
                .get("emotional_significance")
                .and_then(|s| s.as_f64())
                .unwrap_or(if success { 0.5 } else { 0.7 }) as f32,
            task_id,
            user_id: e.get("user_id").and_then(|u| u.as_u64()),
            lessons_learned: Vec::new(),
            categories: vec![if success { "task_success" } else { "task_failure" }.to_string()],
            tags: vec!["task".to_string(), "migrated".to_string()],
            consolidation_status: e
                .get("consolidation_status")
                .and_then(|s| s.as_str())
                .unwrap_or("recent")
                .to_string(),
            retrieval_count: 0,
            tokens_used,
            error: None,
        };
        if let Some(t) = task_id {
            index_by_task.entry(t).or_default().push(id);
        }
        index_by_type.entry(experience_type).or_default().push(id);
        experiences.insert(id, mem);
        if id >= next_id {
            next_id = id + 1;
        }
    }

    Some(ExperienceStoreData {
        experiences,
        index_by_task,
        index_by_type,
        next_id,
    })
}

fn migrate_hook_emotional_state(content: &str) -> Option<EmotionalState> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    // Store-format states always carry `state_id`; the hook's never did.
    if v.get("state_id").is_some() {
        return None;
    }
    let valence = v.get("valence")?.as_f64()? as f32;
    Some(EmotionalState {
        state_id: 0,
        timestamp: v.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0),
        primary_emotion: v
            .get("primary_emotion")
            .and_then(|p| p.as_str())
            .unwrap_or("neutral")
            .to_string(),
        valence,
        arousal: v.get("arousal").and_then(|a| a.as_f64()).unwrap_or(0.3) as f32,
        dominance: 0.0,
        stability: 0.0,
        triggers: Vec::new(),
    })
}

fn migrate_hook_gate_decisions(content: &str) -> Option<Vec<GateDecisionRecord>> {
    let arr: Vec<serde_json::Value> = serde_json::from_str(content).ok()?;
    // Store-format records always carry `gate_id`; the hook's never did.
    if arr.first().map(|f| f.get("gate_id").is_some()).unwrap_or(false) {
        return None;
    }
    let mut out = Vec::with_capacity(arr.len());
    for (i, d) in arr.iter().enumerate() {
        let ethical_score = d.get("ethical_score").and_then(|s| s.as_f64()).unwrap_or(1.0) as f32;
        let decision = d
            .get("decision")
            .and_then(|s| s.as_str())
            .unwrap_or("Proceed")
            .to_lowercase();
        let concerns: Vec<String> = d
            .get("concerns")
            .and_then(|c| c.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let reasoning = if concerns.is_empty() {
            format!("Ethical score: {:.2} (migrated from hook record)", ethical_score)
        } else {
            format!(
                "Ethical score: {:.2}, concerns: {} (migrated from hook record)",
                ethical_score,
                concerns.join(", ")
            )
        };
        out.push(GateDecisionRecord {
            gate_id: i as u64 + 1,
            task_id: 0, // the hook record never carried a task id
            timestamp: d.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0),
            decision,
            reasoning,
            confidence: ethical_score, // the hook's confidence WAS the ethical score
            ethical_score,
        });
    }
    Some(out)
}

fn migrate_hook_perception_window(content: &str) -> Option<PerceptionWindow> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    // Store-format windows always carry `input_buffer_size`.
    if v.get("input_buffer_size").is_some() {
        return None;
    }
    let inputs = v.get("active_inputs")?.as_array()?;
    let mut window = PerceptionWindow::default();
    for i in inputs {
        let task_id = i.get("task_id").and_then(|t| t.as_u64()).unwrap_or(0);
        let task_type = i.get("task_type").and_then(|t| t.as_str()).unwrap_or("task");
        let status = i.get("status").and_then(|t| t.as_str()).unwrap_or("");
        window.active_inputs.push(PerceptionInput {
            input_type: i
                .get("input_type")
                .and_then(|t| t.as_str())
                .unwrap_or("task_event")
                .to_string(),
            content: format!("task {} ({}) {}", task_id, task_type, status),
            timestamp: i.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0),
            source: "task-manager".to_string(),
            relevance_score: i.get("relevance_score").and_then(|r| r.as_f64()).unwrap_or(0.8) as f32,
        });
    }
    while window.active_inputs.len() > window.input_buffer_size {
        window.active_inputs.remove(0);
    }
    Some(window)
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
