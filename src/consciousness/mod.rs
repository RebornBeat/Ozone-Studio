//! Consciousness Module - Part II of the Ozone Studio specification
//! 
//! This module provides the consciousness extension that transforms Ozone Studio
//! from a tool into a developing artificial general intelligence.
//! 
//! Features (opt-in via `consciousness` feature flag):
//! - Experience Memory System (§35-38)
//! - Emotional Context System (§39-40)
//! - Window-First Architecture (§32)
//! - Decision Gate (§33)
//! - I-Loop Reflection (§42)
//! - Relationship Development (§48)
//! - Ethical Reasoning (§49)

#[cfg(feature = "consciousness")]
pub mod store;

#[cfg(feature = "consciousness")]
pub use store::*;

// Re-export key functions for easy access
#[cfg(feature = "consciousness")]
pub use store::{
    is_consciousness_enabled,
    pre_task_gate,
    post_task_experience,
    get_relevant_experiences,
    get_current_emotional_state,
    CONSCIOUSNESS_STORE,
};

// Provide stubs when consciousness is disabled
#[cfg(not(feature = "consciousness"))]
pub fn is_consciousness_enabled() -> bool { false }

#[cfg(not(feature = "consciousness"))]
pub fn pre_task_gate(_task_id: u64, _task_summary: &str, _user_id: u64) -> () {}

#[cfg(not(feature = "consciousness"))]
pub fn post_task_experience(
    _task_id: u64,
    _task_summary: &str,
    _success: bool,
    _user_id: Option<u64>,
) -> Option<u64> { None }
