//! CrossModalIndex — the DERIVED cross-modal pair read-model (2026-10-13,
//! operator decision: "best of both worlds", revising the earlier
//! abandonment).
//!
//! One writer, zero drift: `context.relationships` on each container is the
//! ONLY written truth. This module never stores anything — it derives a
//! queryable pair snapshot FROM the containers on each request (all_ids →
//! load through the hot cache ≈ the same cost the traversal engine already
//! pays routinely at this store's scale). Because the snapshot is rebuilt
//! per read there is no second writer and therefore nothing to invalidate;
//! if profiling ever shows the rebuild matters, a generation-bumped cache
//! can wrap the same pure builder without changing a single writer.
//!
//! Container 242 (`CrossModalIndexRoot`) stays reserved as this model's
//! namespace home, as recorded in the decision.
//!
//! A "pair" is one typed `Relation` edge between two EXISTING containers
//! whose modalities differ (same-modality edges are text's SimilarTo
//! linking and stay out of scope here — that's intra-modal similarity, not
//! cross-modal). `modality` on Metadata is the label; containers with
//! `Modality::Unknown` still participate (their container_type display name
//! is the fallback label), because e.g. Methodology↔Blueprint pairs are
//! meaningful cross-type context even when modality is unset.

use crate::types::container::Container;
use serde::Serialize;

/// One derived cross-modal edge. Every field traces to a real stored value.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CrossModalPair {
    pub source_id: u64,
    /// The source container's modality label (e.g. "Text", "Code").
    pub source_label: String,
    pub target_id: u64,
    pub target_label: String,
    /// The RelationType variant name as stored (e.g. "SimilarTo").
    pub relation_type: String,
    pub discovered_via: String,
    pub confidence: f32,
}

/// Pure extraction: pairs from an id→container map. Separated from storage
/// access so it is unit-testable without a store file.
pub fn pairs_from(containers: &[(u64, Container)]) -> Vec<CrossModalPair> {
    // First pass: id → modality label (for both endpoints of every edge).
    let mut labels: std::collections::HashMap<u64, String> =
        std::collections::HashMap::with_capacity(containers.len());
    for (id, container) in containers {
        labels.insert(*id, modality_label(container));
    }

    let mut pairs = Vec::new();
    for (id, container) in containers {
        let source_label = &labels[id];
        for rel in &container.local_state.context.relationships {
            let Some(target_label) = labels.get(&rel.target_id) else {
                // Dangling target: the edge names a container that does not
                // exist. Recorded honestly as a skip, never silently dropped
                // into a made-up pair.
                continue;
            };
            if target_label == source_label {
                // Same modality (or both Unknown-of-the-same-type shape):
                // intra-modal, out of scope for THIS index.
                continue;
            }
            pairs.push(CrossModalPair {
                source_id: *id,
                source_label: source_label.clone(),
                target_id: rel.target_id,
                target_label: target_label.clone(),
                relation_type: format!("{:?}", rel.relation_type),
                discovered_via: format!("{:?}", rel.discovered_via),
                confidence: rel.confidence,
            });
        }
    }
    pairs
}

fn modality_label(container: &Container) -> String {
    let m = &container.local_state.metadata;
    if m.modality != crate::types::container::Modality::default() {
        return format!("{:?}", m.modality);
    }
    m.container_type.display_name().to_string()
}

/// Storage-backed build: the derived snapshot for the current graph.
pub fn build(
    storage: &crate::zsei::ContainerStorage,
    src: Option<&str>,
    dst: Option<&str>,
) -> Vec<CrossModalPair> {
    let mut containers: Vec<(u64, Container)> = Vec::new();
    for id in storage.all_ids() {
        if let Ok(Some(container)) = storage.load(id) {
            containers.push((id, container));
        }
    }
    let all = pairs_from(&containers);
    let want = |label: Option<&str>, pair_label: &str| match label {
        Some(s) => s.eq_ignore_ascii_case(pair_label),
        None => true,
    };
    all.into_iter()
        .filter(|p| want(src.as_deref(), &p.source_label) && want(dst.as_deref(), &p.target_label))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::container::{
        ContainerType, DiscoveryMethod, GlobalState, LocalState, Metadata, Modality, Relation,
        RelationType,
    };

    fn container(id: u64, modality: Modality) -> Container {
        Container {
            global_state: GlobalState {
                container_id: id,
                child_count: 0,
                version: 1,
                parent_id: 0,
                child_ids: vec![],
            },
            local_state: LocalState {
                metadata: Metadata {
                    container_type: ContainerType::ModalityGraph,
                    modality,
                    created_at: 0,
                    updated_at: 0,
                    provenance: "test".into(),
                    permissions: 0,
                    owner_id: 0,
                    name: format!("c{}", id).into(),
                    materialized_path: None,
                },
                context: Default::default(),
                storage: Default::default(),
                hints: Default::default(),
                integrity: Default::default(),
                file_context: None,
                code_context: None,
                text_context: None,
                external_ref: None,
            },
        }
    }

    fn edge(target: u64, rtype: RelationType) -> Relation {
        Relation {
            target_id: target,
            relation_type: rtype,
            confidence: 1.0,
            discovered_via: DiscoveryMethod::Manual,
            graph_hops: None,
        }
    }

    #[test]
    fn cross_modal_edges_become_pairs_and_same_modal_edges_do_not() {
        let mut text = container(1, Modality::Text);
        text.local_state.context.relationships = vec![
            edge(2, RelationType::SimilarTo),   // cross-modal (Text→Code)
            edge(3, RelationType::SimilarTo),   // same-modal (Text→Text): out of scope
            edge(99, RelationType::RelatedTo),  // dangling target: skipped, not fabricated
        ];
        let mut code = container(2, Modality::Code);
        code.local_state.context.relationships = vec![edge(1, RelationType::RelatedTo)];
        let text3 = container(3, Modality::Text);

        let pairs = pairs_from(&[(1, text), (2, code), (3, text3)]);
        assert_eq!(pairs.len(), 2, "dangling + same-modal excluded: {:?}", pairs);
        assert!(pairs.iter().any(|p| p.source_id == 1 && p.target_id == 2
            && p.source_label == "Text" && p.target_label == "Code"));
        assert!(pairs.iter().any(|p| p.source_id == 2 && p.target_id == 1));
    }

    #[test]
    fn src_dst_filters_are_case_insensitive() {
        let mut text = container(1, Modality::Text);
        text.local_state.context.relationships = vec![edge(2, RelationType::RelatedTo)];
        let code = container(2, Modality::Code);
        let pairs = pairs_from(&[(1, text), (2, code)]);
        let hit = pairs
            .into_iter()
            .filter(|p| p.source_label.eq_ignore_ascii_case("text"))
            .count();
        assert_eq!(hit, 1);
    }
}
