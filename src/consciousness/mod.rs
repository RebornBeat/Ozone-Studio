//! Consciousness Module - Part II of the Ozone Studio specification
//! 
//! This module provides the consciousness extension that transforms Ozone Studio
//! from a tool into a developing artificial general intelligence.
//! 
//! Features (enabled/disabled via config.toml, NOT compile-time):
//! - Experience Memory System (§35-38)
//! - Emotional Context System (§39-40)
//! - Window-First Architecture (§32)
//! - Decision Gate (§33)
//! - I-Loop Reflection (§42)
//! - Relationship Development (§48)
//! - Ethical Reasoning (§49)
//!
//! Consciousness is ALWAYS compiled in. Enable/disable at runtime via config.toml:
//! ```toml
//! [consciousness]
//! enabled = true  # or false
//! ```

pub mod assistant;
pub mod review;
pub mod store;

pub use store::*;

// Type alias for backwards compatibility.
//
// NOTE (C1 consolidation, 2026-10-23): the host's REAL consciousness state is
// the static `CONSCIOUSNESS_STORE` below — it is what the task manager
// observes, what config.toml's `[consciousness]` section is copied into at
// boot (`ConsciousnessStore::apply_config`), and what the assistant reads.
// `lib.rs` also builds a second `ConsciousnessSystem::new()` (this alias) into
// `OzoneRuntime.consciousness`; its only reader is an `is_some()` check
// (`OzoneRuntime::is_consciousness_enabled`). It cannot delegate to the static
// store because the alias is the SAME struct (`Arc<RwLock<..>>` ownership in
// lib.rs), so it is an independent in-memory copy that loads the same files at
// boot and is never mutated. Folding it away needs a lib.rs change and is
// recorded as a follow-up; nothing reads state from it.
pub type ConsciousnessSystem = store::ConsciousnessStore;

// Re-export key functions for easy access
pub use store::{
    get_current_emotional_state,
    get_relevant_experiences,
    i_loop_busy,
    is_consciousness_enabled,
    observe_task_event,
    post_task_experience,
    post_task_experience_with,
    pre_task_gate,
    set_i_loop_active,
    wait_for_i_loop_completion,
    CONSCIOUSNESS_STORE,
};

/// Narrative fragment for consciousness storytelling
/// Used in experience replay and self-model construction
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NarrativeFragment {
    pub id: u64,
    pub content: String,
    pub emotion: String,
    pub significance: f32,
    pub timestamp: u64,
}

impl Default for NarrativeFragment {
    fn default() -> Self {
        Self {
            id: 0,
            content: String::new(),
            emotion: "neutral".to_string(),
            significance: 0.0,
            timestamp: 0,
        }
    }
}

/// Relationship context for task processing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationshipContext {
    pub user_id: u64,
    pub relationship_type: String,
    pub trust_level: f32,
    pub interaction_count: u64,
    pub last_interaction: u64,
}

impl Default for RelationshipContext {
    fn default() -> Self {
        Self {
            user_id: 0,
            relationship_type: "unknown".to_string(),
            trust_level: 0.5,
            interaction_count: 0,
            last_interaction: 0,
        }
    }
}

/// Decision gate result for consciousness-aware tasks
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsciousnessDecisionGate {
    pub allow_processing: bool,
    pub reasoning: String,
    pub confidence: f32,
    pub emotional_influence: f32,
    pub ethical_score: f32,
}

impl Default for ConsciousnessDecisionGate {
    fn default() -> Self {
        Self {
            allow_processing: true,
            reasoning: String::new(),
            confidence: 1.0,
            emotional_influence: 0.0,
            ethical_score: 1.0,
        }
    }
}
