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

pub mod store;

pub use store::*;

// Type alias for backwards compatibility
pub type ConsciousnessSystem = store::ConsciousnessStore;

// Re-export key functions for easy access
pub use store::{
    is_consciousness_enabled,
    pre_task_gate,
    post_task_experience,
    get_relevant_experiences,
    get_current_emotional_state,
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
