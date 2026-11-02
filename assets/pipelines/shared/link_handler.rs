//! Shared LinkToModality writer — idempotent, bidirectional, honest.
//!
//! Included by a modality pipeline with
//!   #[path = "../../shared/link_handler.rs"]
//!   mod link_handler;
//! (the pipeline crates are deliberately outside the cargo workspace, so a
//! `#[path]` include is the established sharing mechanism — see
//! semantic_relations.rs / capture.rs).
//!
//! Contract (identical for every modality that adopts it):
//!   * the action payload is `{source_graph_id, target_graph_id,
//!     target_modality, relationship}` — the orchestrator sends exactly this
//!     from `src/orchestrator/graphs.rs`; `graph_id` is the ZSEI container id
//!     of the persisted ModalityGraph (math / text convention);
//!   * the write is container-level: a `RelatedTo` entry in
//!     `local_state.context.relationships` of BOTH containers, checked for an
//!     existing `(target_id, relation_type, discovered_via)` first so a
//!     repeated call (the PASS-3 until-stable loop calls every pass) never
//!     duplicates an edge;
//!   * failure is reported, never swallowed or fabricated: a missing
//!     container or an unreachable host yields `Err`, and the caller returns
//!     `success:false` with that text. No counts are invented — the result
//!     reports exactly which direction was newly added and which already
//!     existed.
//!
//! No new dependency: the caller passes its own `zsei_query` (every modality
//! crate defines one with the same signature), so this file needs only
//! `serde_json` and `std`.

use serde_json::{json, Value};
use std::future::Future;

/// `discovered_via` recorded on the edge. Same value text/math/3D use for an
/// explicit LinkToModality (an explicit call, not an inference).
pub const LINK_DISCOVERED_VIA: &str = "Manual";
/// Relation type written. Matches the 11 existing handlers.
pub const LINK_RELATION_TYPE: &str = "RelatedTo";

/// Outcome of a bidirectional link — what actually happened, nothing more.
#[derive(Debug, Clone, Copy)]
pub struct LinkOutcome {
    /// source -> target edge was newly written (false = already present).
    pub forward_added: bool,
    /// target -> source edge was newly written (false = already present).
    pub reverse_added: bool,
}

/// Add one `RelatedTo` edge `container_id -> target_id` unless an identical
/// `(target_id, relation_type, discovered_via)` edge is already there.
/// Returns `Ok(true)` when written, `Ok(false)` when it already existed.
pub async fn add_relationship<F, Fut>(
    zsei_query: &F,
    container_id: u64,
    target_id: u64,
) -> Result<bool, String>
where
    F: Fn(Value) -> Fut,
    Fut: Future<Output = Result<Value, String>>,
{
    let existing = zsei_query(json!({
        "GetContainer": { "container_id": container_id }
    }))
    .await?;
    let container_json = existing.get("Container").cloned().unwrap_or(existing);
    let context = container_json
        .pointer("/local_state/context")
        .cloned()
        .ok_or_else(|| format!("Container {} has no local_state.context", container_id))?;

    let mut relationships: Vec<Value> = context
        .get("relationships")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let already_linked = relationships.iter().any(|r| {
        r.get("target_id").and_then(|v| v.as_u64()) == Some(target_id)
            && r.get("relation_type").and_then(|v| v.as_str()) == Some(LINK_RELATION_TYPE)
            && r.get("discovered_via").and_then(|v| v.as_str()) == Some(LINK_DISCOVERED_VIA)
    });
    if already_linked {
        return Ok(false);
    }

    relationships.push(json!({
        "target_id": target_id,
        "relation_type": LINK_RELATION_TYPE,
        "confidence": 1.0,
        "discovered_via": LINK_DISCOVERED_VIA
    }));
    let mut context_updated = context.clone();
    context_updated["relationships"] = Value::Array(relationships);

    zsei_query(json!({
        "UpdateContainer": {
            "container_id": container_id,
            "updates": { "metadata": null, "context": context_updated, "storage": null, "hints": null }
        }
    }))
    .await?;
    Ok(true)
}

/// Write the edge in both directions. Stops at the first failure and says
/// which direction failed (the other may already have been written; the next
/// call is idempotent so a retry completes the pair).
pub async fn link_bidirectional<F, Fut>(
    zsei_query: &F,
    source_graph_id: u64,
    target_graph_id: u64,
) -> Result<LinkOutcome, String>
where
    F: Fn(Value) -> Fut,
    Fut: Future<Output = Result<Value, String>>,
{
    if source_graph_id == target_graph_id {
        return Err(format!(
            "link_to_modality: source and target are the same container ({})",
            source_graph_id
        ));
    }
    let forward_added = add_relationship(zsei_query, source_graph_id, target_graph_id)
        .await
        .map_err(|e| format!("link_to_modality: failed to write source->target relationship: {}", e))?;
    let reverse_added = add_relationship(zsei_query, target_graph_id, source_graph_id)
        .await
        .map_err(|e| format!("link_to_modality: failed to write target->source relationship: {}", e))?;
    Ok(LinkOutcome { forward_added, reverse_added })
}

/// Build the `link_result` JSON the orchestrator looks for
/// (`result.get("link_result").is_some()` in graphs.rs). Only real values:
/// the ids and relationship text from the call, which directions were newly
/// written, and a real wall-clock timestamp. No synthetic link id.
pub fn link_result_json(
    source_graph_id: u64,
    target_graph_id: u64,
    target_modality: &str,
    relationship: &Value,
    outcome: LinkOutcome,
) -> Value {
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    json!({
        "source_graph_id": source_graph_id,
        "target_graph_id": target_graph_id,
        "target_modality": target_modality,
        "relationship": relationship,
        "forward_added": outcome.forward_added,
        "reverse_added": outcome.reverse_added,
        "created_at_epoch_secs": created_at
    })
}
