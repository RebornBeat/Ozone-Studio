//! Methodology types - Section 13 of the specification

use serde::{Deserialize, Serialize};
use super::{ContainerID, PublicKey, SemVer, pipeline::ConsensusStatus};

/// Methodology definition (§13.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Methodology {
    pub methodology_id: ContainerID,
    pub name: String,
    pub description: String,
    
    // Category binding
    pub category_ids: Vec<ContainerID>,
    
    // Discoverability
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    
    // Content
    pub principles: Vec<Principle>,
    pub heuristics: Vec<Heuristic>,
    pub decision_rules: Vec<DecisionRule>,
    
    // Usage tracking
    pub usage_count: u64,
    pub success_rate: f32,
    
    // Metadata
    pub created_at: u64,
    pub created_by: PublicKey,
    pub version: SemVer,
    
    // Distribution
    pub distributed: bool,
    pub consensus_status: ConsensusStatus,
}

/// Principle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principle {
    pub principle_id: u64,
    pub statement: String,
    pub rationale: String,
    pub applicability: Vec<String>,
}

/// Heuristic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heuristic {
    pub heuristic_id: u64,
    pub name: String,
    pub description: String,
    pub when_to_apply: String,
    pub examples: Vec<String>,
}

/// Decision rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRule {
    pub rule_id: u64,
    pub condition: String,
    pub action: String,
    pub priority: u8,
    pub exceptions: Vec<String>,
}
