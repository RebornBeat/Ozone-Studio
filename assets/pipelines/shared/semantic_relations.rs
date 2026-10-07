//! Zero-shot relation validation shared by modality pipelines.
//!
//! Three checks, each recorded on the accepted relation so the verification
//! level is always visible:
//! - `validate`: text sources. Endpoints are extracted entity labels, the
//!   relation is a ZSEI name, and the evidence appears verbatim in the source.
//!   Verification "verbatim".
//! - `validate_mapped`: as `validate`, but a domain vocabulary label is accepted
//!   when `map_domain_relation` maps it to a ZSEI name. The original label is kept
//!   in `domain_relation`. Verification "verbatim".
//! - `validate_structural`: sources whose entities are numeric graph node IDs.
//!   Endpoints must be IDs from the node list the model saw, and the reason must
//!   be present, but there is no verbatim text to check. Verification
//!   "structural": the relation is checked, not evidence-verified.
//!
//! Every rejection carries the raw proposal and a stated reason. Nothing is dropped
//! silently. Confidence is a convention for storage: 1.0 for verbatim evidence,
//! 0.5 for structural-only checks. The model supplies no probability.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const ZSEI_RELATIONS: &[&str] = &[
    "Contains",
    "PartOf",
    "RelatedTo",
    "References",
    "Above",
    "Below",
    "NearTo",
    "InFrontOf",
    "Overlaps",
    "Precedes",
    "Follows",
    "Contradicts",
    "Supersedes",
    "SimilarTo",
];

pub const VERIFIED_VERBATIM: &str = "verbatim";
pub const VERIFIED_STRUCTURAL: &str = "structural";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcceptedRelation {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub evidence: String,
    #[serde(default)]
    pub verification: String,
    #[serde(default)]
    pub domain_relation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectedRelation {
    pub raw: String,
    pub reason: String,
}

#[derive(Debug, Default)]
pub struct Validated {
    pub accepted: Vec<AcceptedRelation>,
    pub rejected: Vec<RejectedRelation>,
}

/// Domain vocabulary label → nearest ZSEI relation. Labels not listed here are
/// rejected with their original text, so a new domain label is visible, not lost.
pub fn map_domain_relation(label: &str) -> Option<&'static str> {
    Some(match label {
        "TopologicallyConnected" | "TransmitsTo" | "RoutedThrough" | "FirewalledBy"
        | "DependsOnService" | "BGPPeersWith" | "ForwardedBy" | "TunneledThrough"
        | "Affects" | "CausedBy" | "Enables" | "Implies" | "Prevents" | "Performs"
        | "FunctionalRole" | "InterferesWith" | "DetectedAtRange"
        | "ReflectsWithAcousticSignature" | "MovingWithVelocity" | "BathymetryOf"
        | "ShadowedBy" | "AssociatedBiology" | "EmitsHeat" | "IndicatesMaterialState"
        | "AffectsBiologicalProcess" | "CausesExpansion" | "GradientBetween"
        | "AnomalyRelativeTo" => "RelatedTo",
        "ResolvedBy" | "AuthenticatedBy" | "SpeciesSignature" | "InstanceOf"
        | "DerivedFrom" => "References",
        "InSubnet" => "PartOf",
        "TemporalPrecedes" => "Precedes",
        "LayeredAbove" => "Above",
        "ThermallyCoupledTo" | "ThermalBridgeTo" => "NearTo",
        "IsothermEncloses" => "Contains",
        _ => return None,
    })
}

fn resolve_zsei(label: &str) -> Option<&'static str> {
    ZSEI_RELATIONS.iter().find(|r| **r == label).copied()
}

fn resolve_mapped(label: &str) -> Option<&'static str> {
    resolve_zsei(label).or_else(|| map_domain_relation(label))
}

fn field_text(item: &Value, key: &str) -> String {
    match item.get(key) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Check {
    Verbatim,
    Structural,
}

fn check(
    proposal: &Value,
    entities: &[String],
    source_text: &str,
    resolve: fn(&str) -> Option<&'static str>,
    mode: Check,
    unmapped_reason: &str,
) -> Validated {
    let mut out = Validated::default();
    let Some(items) = proposal.get("relations").and_then(|r| r.as_array()) else {
        out.rejected.push(RejectedRelation {
            raw: proposal.to_string(),
            reason: "model output has no 'relations' array".into(),
        });
        return out;
    };
    for item in items {
        let raw = item.to_string();
        let reject = |reason: &str| RejectedRelation {
            raw: raw.clone(),
            reason: reason.to_string(),
        };
        let from = field_text(item, "from");
        let to = field_text(item, "to");
        let label = field_text(item, "relation");
        let evidence = field_text(item, "evidence");
        if from.is_empty() || to.is_empty() || label.is_empty() || evidence.is_empty() {
            out.rejected.push(reject("missing from, to, relation or evidence"));
            continue;
        }
        let Some(relation) = resolve(&label) else {
            out.rejected.push(reject(unmapped_reason));
            continue;
        };
        if from == to {
            out.rejected.push(reject("relation endpoints are the same entity"));
            continue;
        }
        if !entities.contains(&from) || !entities.contains(&to) {
            out.rejected.push(reject("endpoint is not an extracted entity"));
            continue;
        }
        if mode == Check::Verbatim && !source_text.contains(&evidence) {
            out.rejected.push(reject("evidence is not verbatim in the source"));
            continue;
        }
        out.accepted.push(AcceptedRelation {
            from,
            to,
            relation: relation.to_string(),
            evidence,
            verification: match mode {
                Check::Verbatim => VERIFIED_VERBATIM,
                Check::Structural => VERIFIED_STRUCTURAL,
            }
            .to_string(),
            domain_relation: (label != relation).then_some(label),
        });
    }
    out
}

pub fn validate(proposal: &Value, entities: &[String], source_text: &str) -> Validated {
    check(
        proposal,
        entities,
        source_text,
        resolve_zsei,
        Check::Verbatim,
        "relation is not a ZSEI relation name",
    )
}

pub fn validate_mapped(proposal: &Value, entities: &[String], source_text: &str) -> Validated {
    check(
        proposal,
        entities,
        source_text,
        resolve_mapped,
        Check::Verbatim,
        "relation has no ZSEI mapping (domain label kept in the rejected record)",
    )
}

pub fn validate_structural(proposal: &Value, node_ids: &[String]) -> Validated {
    check(
        proposal,
        node_ids,
        "",
        resolve_mapped,
        Check::Structural,
        "relation has no ZSEI mapping (domain label kept in the rejected record)",
    )
}

/// The ZSEI `Relation` JSON for `context.relationships`, marked as zero-shot discovered.
pub fn zsei_relation_json(target_id: u64, relation: &str) -> Value {
    zsei_relation_json_for(target_id, relation, VERIFIED_VERBATIM)
}

pub fn zsei_relation_json_for(target_id: u64, relation: &str, verification: &str) -> Value {
    let confidence = if verification == VERIFIED_STRUCTURAL { 0.5 } else { 1.0 };
    json!({
        "target_id": target_id,
        "relation_type": relation,
        "confidence": confidence,
        "discovered_via": "ZeroShot",
        "graph_hops": null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entities() -> Vec<String> {
        vec!["Ada".into(), "Babbage".into()]
    }

    #[test]
    fn accepts_verbatim_evidence_between_extracted_entities() {
        let src = "Ada worked with Babbage on the engine.";
        let p = json!({"relations": [{"from": "Ada", "to": "Babbage", "relation": "RelatedTo", "evidence": "Ada worked with Babbage"}]});
        let v = validate(&p, &entities(), src);
        assert_eq!(v.accepted.len(), 1);
        assert_eq!(v.accepted[0].verification, VERIFIED_VERBATIM);
        assert!(v.rejected.is_empty());
    }

    #[test]
    fn rejects_evidence_not_in_source() {
        let p = json!({"relations": [{"from": "Ada", "to": "Babbage", "relation": "RelatedTo", "evidence": "invented sentence"}]});
        let v = validate(&p, &entities(), "Ada worked with Babbage.");
        assert!(v.accepted.is_empty());
        assert_eq!(v.rejected[0].reason, "evidence is not verbatim in the source");
    }

    #[test]
    fn rejects_unknown_entity_unknown_relation_and_self_relation() {
        let src = "Ada and Babbage.";
        let p = json!({"relations": [
            {"from": "Ada", "to": "Turing", "relation": "RelatedTo", "evidence": "Ada"},
            {"from": "Ada", "to": "Babbage", "relation": "Loves", "evidence": "Ada"},
            {"from": "Ada", "to": "Ada", "relation": "SimilarTo", "evidence": "Ada"}
        ]});
        let v = validate(&p, &entities(), src);
        assert!(v.accepted.is_empty());
        assert_eq!(v.rejected.len(), 3);
    }

    #[test]
    fn missing_relations_array_is_reported() {
        let v = validate(&json!({"text": "no structure"}), &entities(), "x");
        assert_eq!(v.rejected.len(), 1);
    }

    #[test]
    fn relation_json_is_marked_zero_shot() {
        let r = zsei_relation_json(7, "Above");
        assert_eq!(r["discovered_via"], "ZeroShot");
        assert_eq!(r["relation_type"], "Above");
        assert_eq!(r["confidence"], 1.0);
    }

    #[test]
    fn mapped_domain_label_is_kept_and_translated() {
        let src = "Ada and Babbage.";
        let p = json!({"relations": [{"from": "Ada", "to": "Babbage", "relation": "CausedBy", "evidence": "Ada and Babbage"}]});
        let v = validate_mapped(&p, &entities(), src);
        assert_eq!(v.accepted[0].relation, "RelatedTo");
        assert_eq!(v.accepted[0].domain_relation.as_deref(), Some("CausedBy"));
    }

    #[test]
    fn unmapped_domain_label_is_rejected_with_reason() {
        let p = json!({"relations": [{"from": "Ada", "to": "Babbage", "relation": "Invented", "evidence": "Ada"}]});
        let v = validate_mapped(&p, &entities(), "Ada");
        assert!(v.accepted.is_empty());
        assert!(v.rejected[0].reason.starts_with("relation has no ZSEI mapping"));
    }

    #[test]
    fn structural_accepts_numeric_ids_without_verbatim_evidence() {
        let ids: Vec<String> = vec!["3".into(), "7".into()];
        let p = json!({"relations": [{"from": 3, "to": 7, "relation": "TransmitsTo", "evidence": "model reason text"}]});
        let v = validate_structural(&p, &ids);
        assert_eq!(v.accepted.len(), 1);
        assert_eq!(v.accepted[0].verification, VERIFIED_STRUCTURAL);
        assert_eq!(v.accepted[0].relation, "RelatedTo");
        assert_eq!(zsei_relation_json_for(1, "RelatedTo", VERIFIED_STRUCTURAL)["confidence"], 0.5);
    }

    #[test]
    fn structural_rejects_ids_not_in_node_list() {
        let ids: Vec<String> = vec!["3".into()];
        let p = json!({"relations": [{"from": 3, "to": 9, "relation": "RelatedTo", "evidence": "reason"}]});
        let v = validate_structural(&p, &ids);
        assert!(v.accepted.is_empty());
    }
}
