//! K-ALGORITHM: relationship-path relevance — how a freshly persisted graph
//! links into the living graph (task 57 arc).
//!
//! Doctrine: candidates come from the RELATIONSHIP GRAPH, not just keyword
//! scans. A container reachable through real relationship edges from the
//! project (parent/child + explicit Context.relationships hops, walked by
//! ZSEI's structural traversal) is graph-adjacent and qualifies on any term
//! overlap; a keyword-only seed with no graph path needs stronger overlap.
//! There are deliberately NO candidate caps — the relationship path IS the
//! bound: the neighborhood walked at graph_max_depth is a small, relevant
//! set, and the write phase only touches candidates that clear their floor.
//!
//! Pure policy lives here (std-only, embedded by host + pipelines via
//! `#[path]` like every shared contract); the walk itself is the store's
//! structural traversal; the wire export to pipelines is
//! OZONE_RELEVANCE_POLICY (JSON, same field names as this struct).

/// Selector for how graph-adjacent a candidate must be to link.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelevancePolicy {
    /// Max relationship hops walked from the project container. 0 disables
    /// the walk entirely (keywords-only legacy behavior).
    pub graph_max_depth: u32,
    /// Shared keyword/topic terms required for candidates INSIDE the walked
    /// relationship neighborhood.
    pub neighborhood_shared_floor: usize,
    /// Shared terms required for keyword-seed candidates OUTSIDE the
    /// neighborhood (no graph path to the project).
    pub seed_shared_floor: usize,
}

impl RelevancePolicy {
    /// Default: walk 2 hops (project -> children -> their relation targets),
    /// any overlap qualifies inside the neighborhood, two shared terms for
    /// graph-distant seeds.
    pub const fn graph_first() -> Self {
        Self {
            graph_max_depth: 2,
            neighborhood_shared_floor: 1,
            seed_shared_floor: 2,
        }
    }

    /// Legacy behavior before the living-graph arc: no relationship walk,
    /// uniform two-term floor everywhere.
    pub const fn keywords_only() -> Self {
        Self {
            graph_max_depth: 0,
            neighborhood_shared_floor: 2,
            seed_shared_floor: 2,
        }
    }
}

impl Default for RelevancePolicy {
    fn default() -> Self {
        Self::graph_first()
    }
}
