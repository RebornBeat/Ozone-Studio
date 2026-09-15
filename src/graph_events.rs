//! Graph event ripple — the living-graph nervous system.
//!
//! Every successful graph WRITE (container created/updated/deleted, file/
//! URL/package linked) flows through `ZSEI::query` — and this hub turns
//! each one into a scoped event any part of the system can subscribe to:
//! the WebSocket push channel (UI + agents get real push, closing the
//! poll-only gap), the monitor feed, and per-interest hooks (a modality or
//! project hook fires when graph data it cares about changes — the ripple
//! effect: update a file link in one project and everything scoped to it
//! can react).
//!
//! Delivery model: broadcast to ALL subscribers; scope filtering is
//! subscriber-side (events carry their scope keywords — `scope:global`,
//! `ws:<id>`, `proj:<id>` — from the container's own context, matching the
//! scoping spine in src/context_mirror.rs). Slow subscribers lag via the
//! broadcast channel's ring; they never block graph writes.

use std::sync::OnceLock;
use tokio::sync::broadcast;

/// One graph mutation, captured at the ZSEI query choke point.
#[derive(Debug, Clone)]
pub struct GraphEvent {
    /// created | updated | deleted | linked
    pub event: &'static str,
    pub container_id: u64,
    pub parent_id: u64,
    /// ContainerType display name (e.g. "CoordinationEvent", "FileRef").
    pub container_type: String,
    /// Provenance: who/what caused the write (agent name, "orchestrator",
    /// "context-mirror", …) — empty when unknown at the choke point.
    pub source: String,
    /// Scope keywords copied from the container's context
    /// (["ws:3"] / ["scope:global"] / ["ws:3","proj:7"]) — subscribers
    /// scope-filter on these.
    pub scope_keywords: Vec<String>,
    pub timestamp: u64,
}

impl GraphEvent {
    /// True if this event is visible to the given scope — a subscriber at
    /// global scope sees everything; a ws/proj subscriber sees globals and
    /// its own scope's events.
    pub fn visible_to(&self, scope_keywords: &[String]) -> bool {
        if scope_keywords.iter().any(|k| k == "scope:global") {
            return true;
        }
        if self.scope_keywords.iter().any(|k| k == "scope:global") {
            return true;
        }
        scope_keywords.iter().any(|k| self.scope_keywords.contains(k))
    }
}

static HUB: OnceLock<GraphEventHub> = OnceLock::new();

#[derive(Clone)]
pub struct GraphEventHub {
    tx: broadcast::Sender<std::sync::Arc<GraphEvent>>,
}

impl GraphEventHub {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    /// The process-wide hub (created on first use).
    pub fn global() -> &'static GraphEventHub {
        HUB.get_or_init(GraphEventHub::new)
    }

    /// Publish a graph event. Never blocks the graph write: if no
    /// subscriber is listening this is a no-op; if the ring is full the
    /// oldest events lag out (subscribers handle `RecvError::Lagged`).
    pub fn publish(&self, event: GraphEvent) {
        let _ = self.tx.send(std::sync::Arc::new(event));
    }

    /// Subscribe to the ripple. Returns a receiver of shared events —
    /// cheap to hold in websockets, hooks, agents.
    pub fn subscribe(&self) -> broadcast::Receiver<std::sync::Arc<GraphEvent>> {
        self.tx.subscribe()
    }
}

/// Convenience for the ZSEI choke point: build + publish in one call.
pub fn emit(
    event: &'static str,
    container_id: u64,
    parent_id: u64,
    container_type: String,
    source: &str,
    scope_keywords: Vec<String>,
) {
    GraphEventHub::global().publish(GraphEvent {
        event,
        container_id,
        parent_id,
        container_type,
        source: source.to_string(),
        scope_keywords,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    });
}
