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

/// What a rule needs in order to be carried out (docs/METHODOLOGY_BLUEPRINT_TOOLS_REVIEW_2026-11-04.md §3.1,
/// docs/MCP_TOOLS_MODALITY_ARCHITECTURE_REVIEW_2026-11-04.md). Methodologies list PROVIDERS per rule — a provider is an MCP SERVER
/// (the unit that is installed, trusted, started and shared; id = the `tools/<dir>` name, e.g. `threed-mcp`). Blueprint creation turns
/// each into a provider STEP; which of the provider's tools runs is decided at call time. Run-time search stays a fallback/learning path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolRequirement {
    /// "provider" (an MCP server id — the primary kind) | "mcp"/"tool" (a registered tool NAME — resolved to the provider that backs it)
    /// | "capability" (free text matched against provider capability strings) | "pipeline" (a pipeline id/name).
    pub kind: String,
    /// The provider id / tool name / capability string / pipeline.
    pub name: String,
    /// Why this rule needs it (shown in the marketplace relation view).
    #[serde(default)]
    pub purpose: Option<String>,
    /// True when the rule can still be carried out without it.
    #[serde(default)]
    pub optional: bool,
}

/// Decision rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRule {
    #[serde(default)]
    pub rule_id: u64,
    pub condition: String,
    pub action: String,
    pub priority: u8,
    pub exceptions: Vec<String>,
    /// Tools/MCPs/capabilities this rule needs (empty = none declared; legacy methodologies have none).
    #[serde(default)]
    pub requires: Vec<ToolRequirement>,
}

#[cfg(test)]
mod requires_tests {
    use super::*;
    #[test]
    fn legacy_rules_without_requires_still_load_and_bound_rules_round_trip() {
        let legacy: DecisionRule = serde_json::from_str(r#"{"condition":"c","action":"a","priority":2,"exceptions":[]}"#).unwrap();
        assert!(legacy.requires.is_empty());
        let bound: DecisionRule = serde_json::from_str(
            r#"{"rule_id":7,"condition":"c","action":"a","priority":1,"exceptions":[],"requires":[{"kind":"provider","name":"threed-mcp","purpose":"build the 3D clock"},{"kind":"capability","name":"cad_solid","optional":true}]}"#,
        )
        .unwrap();
        assert_eq!(bound.requires.len(), 2);
        assert_eq!(bound.requires[0].name, "threed-mcp");
        assert!(bound.requires[1].optional);
        let back = serde_json::to_string(&bound).unwrap();
        assert!(back.contains("\"requires\""));
    }
}
