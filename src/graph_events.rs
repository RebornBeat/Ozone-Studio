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

#[cfg(test)]
mod tests {
    use super::*;

    // T-G3: visible_to scope rules.
    #[test]
    fn global_subscriber_sees_everything() {
        let evt = GraphEvent {
            event: "created",
            container_id: 1,
            parent_id: 8,
            container_type: "CoordinationEvent".into(),
            source: "test".into(),
            scope_keywords: vec!["ws:3".into()],
            timestamp: 0,
        };
        assert!(evt.visible_to(&["scope:global".into()]));
        assert!(evt.visible_to(&["ws:3".into()])); // exact scope match
        assert!(!evt.visible_to(&["ws:9".into()])); // different workspace: hidden
    }

    #[test]
    fn global_event_reaches_all_scopes() {
        let evt = GraphEvent {
            event: "created",
            container_id: 2,
            parent_id: 8,
            container_type: "CoordinationEvent".into(),
            source: "test".into(),
            scope_keywords: vec!["scope:global".into()],
            timestamp: 0,
        };
        assert!(evt.visible_to(&["scope:global".into()]));
        assert!(evt.visible_to(&["ws:3".into()]));
        assert!(evt.visible_to(&["ws:9".into(), "proj:7".into()]));
    }

    #[test]
    fn own_scope_events_match() {
        let evt = GraphEvent {
            event: "linked",
            container_id: 3,
            parent_id: 8,
            container_type: "FileRef".into(),
            source: "test".into(),
            scope_keywords: vec!["ws:3".into(), "proj:7".into()],
            timestamp: 0,
        };
        assert!(evt.visible_to(&["ws:3".into()]));
        assert!(evt.visible_to(&["proj:7".into()]));
        assert!(!evt.visible_to(&["ws:4".into()]));
    }

    // T-G1: publish → subscribe round-trip; reads emit nothing is enforced
    // at the ZSEI choke point (ripple_info returns None for queries).
    #[tokio::test]
    async fn publish_subscribe_roundtrip() {
        let mut rx = GraphEventHub::global().subscribe();
        emit(
            "created",
            42,
            8,
            "CoordinationEvent".to_string(),
            "test",
            vec!["scope:global".to_string()],
        );
        let evt = rx.recv().await.expect("event");
        assert_eq!(evt.container_id, 42);
        assert_eq!(evt.parent_id, 8);
        assert_eq!(evt.event, "created");
        assert!(evt.visible_to(&["ws:1".into()]));
    }

    #[tokio::test]
    async fn empty_hub_subscribe_does_not_panic() {
        // Subscribing before any publish then dropping is a valid lifecycle.
        let mut rx = GraphEventHub::global().subscribe();
        drop(rx);
    }

    fn test_zsei_config(name: &str) -> crate::config::ZSEIConfig {
        static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ozone_ge_{}_{}_{}",
            std::process::id(),
            name,
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        crate::config::ZSEIConfig {
            global_path: dir.join("global.mmap").to_string_lossy().into(),
            local_path: dir.join("local").to_string_lossy().into(),
            cache_path: dir.join("cache").to_string_lossy().into(),
            ml_path: dir.join("ml").to_string_lossy().into(),
            max_containers_in_memory: 1000,
            mmap_enabled: true,
            embedding_dimension: 384,
            pipeline_index_path: dir.join("pi.json").to_string_lossy().into(),
            methodology_index_path: dir.join("mi.json").to_string_lossy().into(),
            blueprint_index_path: dir.join("bi.json").to_string_lossy().into(),
        }
    }

    fn tiny_container(keywords: Vec<String>) -> crate::types::container::Container {
        use crate::types::container::*;
        Container {
            global_state: GlobalState {
                container_id: 0,
                parent_id: 0,
                child_ids: vec![],
                child_count: 0,
                version: 1,
            },
            local_state: LocalState {
                metadata: Metadata {
                    container_type: ContainerType::CoordinationEvent,
                    modality: Modality::Unknown,
                    created_at: 0,
                    updated_at: 0,
                    provenance: "test".into(),
                    permissions: 0,
                    owner_id: 0,
                    name: None,
                    materialized_path: None,
                },
                context: Context {
                    categories: vec![],
                    methodologies: vec![],
                    keywords,
                    topics: vec![],
                    relationships: vec![],
                    learned_associations: vec![],
                    embedding: None,
                },
                storage: StoragePointers {
                    db_shard_id: None,
                    vector_index_ref: None,
                    object_store_path: None,
                    compression_type: CompressionType::None,
                },
                hints: TraversalHints::default(),
                integrity: IntegrityData::default(),
                file_context: None,
                code_context: None,
                text_context: None,
                external_ref: None,
            },
        }
    }

    // T-G1 + T-G2, end to end through the real ZSEI choke point (not just
    // publish()/emit() called directly, which only proves the hub itself
    // works — this proves ZSEI::query genuinely wires into it). The global
    // hub is one process-wide singleton shared by every test in this
    // binary, so a distinctive marker keyword (not "did I get AN event")
    // is what actually identifies which event is mine amid real concurrent
    // noise from other tests' ZSEI writes.
    #[tokio::test]
    async fn real_create_container_emits_a_correctly_provenanced_event() {
        let marker = format!("t-g1-marker-{}", std::process::id());
        let zsei = crate::zsei::ZSEI::new(&test_zsei_config("g1")).unwrap();
        let mut rx = GraphEventHub::global().subscribe();

        let container = tiny_container(vec![marker.clone(), "ws:5".into()]);
        let result = zsei
            .query(crate::types::zsei::ZSEIQuery::CreateContainer { parent_id: 0, container })
            .await
            .unwrap();
        let new_id = match result {
            crate::types::zsei::ZSEIQueryResult::ContainerID(id) => id,
            other => panic!("expected ContainerID, got {:?}", other),
        };

        let evt = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let e = rx.recv().await.expect("hub closed unexpectedly");
                if e.scope_keywords.iter().any(|k| k == &marker) {
                    return e;
                }
            }
        })
        .await
        .expect("a real CreateContainer must emit a matching event within 2s");

        // T-G1: a real write emits.
        assert_eq!(evt.event, "created");
        assert_eq!(evt.container_id, new_id);
        assert_eq!(evt.parent_id, 0);
        // T-G2: type + the container's OWN keywords, captured pre-write —
        // not stale, not empty, not a generic default.
        assert_eq!(evt.container_type, "CoordinationEvent");
        assert!(evt.scope_keywords.contains(&"ws:5".to_string()));

        // T-G1 other half: a pure read of the same container emits nothing
        // new bearing this marker (ripple_info's match has no arm for
        // GetContainer — falls through to `_ => return None` — so this is a
        // real structural guarantee, not a timing coincidence; the bounded
        // wait just gives a concurrent false-positive every fair chance to
        // show up before concluding none did).
        zsei.get_container(new_id).await.unwrap();
        let saw_second_event = tokio::time::timeout(std::time::Duration::from_millis(300), async {
            loop {
                let e = rx.recv().await.expect("hub closed unexpectedly");
                if e.scope_keywords.iter().any(|k| k == &marker) {
                    return true;
                }
            }
        })
        .await
        .unwrap_or(false);
        assert!(!saw_second_event, "a pure read must never emit a graph event");
    }
}
