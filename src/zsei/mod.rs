//! ZSEI - Zero-Shot Embedded Indexer
//! 
//! The knowledge fabric that everything runs through.
//! Based on Section 6 of the specification.
//!
//! # Architecture
//!
//! ZSEI stores containers with:
//! - GlobalState: mmap-friendly ID lists (parent/child relationships)
//! - LocalState: metadata, context, storage pointers, traversal hints
//!
//! # Key Principles
//!
//! - Structure before intelligence
//! - Compression before learning
//! - Traversal before generation
//! - Context not copies (link files, store semantic meaning)
//! - Zero-shot discovery (no task-specific training)

mod storage;
mod traversal;
mod query;
pub mod search;

pub use storage::*;
pub use traversal::*;
pub use query::*;

use crate::config::ZSEIConfig;
use crate::types::{ContainerID, OzoneResult};
use crate::types::container::Container;
use crate::types::zsei::{ZSEIQuery, ZSEIQueryResult, TraversalRequest, TraversalResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main ZSEI instance
pub struct ZSEI {
    /// Configuration
    config: ZSEIConfig,

    /// Container storage (mmap-backed) - wrapped in RwLock for interior mutability
    storage: Arc<RwLock<ContainerStorage>>,

    /// In-memory cache for hot containers
    cache: Arc<RwLock<HashMap<ContainerID, Container>>>,

    /// Traversal engine
    traversal: TraversalEngine,

    /// Query processor
    query_processor: Arc<RwLock<QueryProcessor>>,

    /// Integrity monitor (T-I4 call site) — when wired, every container
    /// mutation first snapshots the PRE-WRITE content into the monitor's
    /// blake3-verified rollback layer. None in tests that don't need it.
    integrity: Option<Arc<tokio::sync::RwLock<crate::integrity::IntegrityMonitor>>>,
}

impl ZSEI {
    /// Create a new ZSEI instance
    pub fn new(config: &ZSEIConfig) -> OzoneResult<Self> {
        tracing::info!("Initializing ZSEI");
        
        // Initialize storage
        let storage = ContainerStorage::new(config)?;
        
        // Initialize traversal engine
        let traversal = TraversalEngine::new(config)?;
        
        // Initialize query processor
        let query_processor = QueryProcessor::new();
        
        Ok(Self {
            config: config.clone(),
            storage: Arc::new(RwLock::new(storage)),
            cache: Arc::new(RwLock::new(HashMap::new())),
            traversal,
            query_processor: Arc::new(RwLock::new(query_processor)),
            integrity: None,
        })
    }

    /// Wire the integrity monitor (T-I4) — called once at boot after both
    /// ZSEI and the monitor exist. Idempotent.
    pub fn set_integrity(
        &mut self,
        integrity: Arc<tokio::sync::RwLock<crate::integrity::IntegrityMonitor>>,
    ) {
        self.integrity = Some(integrity);
    }

    /// Snapshot a container's CURRENT content into the integrity monitor's
    /// rollback layer before a mutation overwrites it. Best-effort: a failed
    /// snapshot must never block the update itself.
    async fn pre_write_snapshot(&self, container_id: ContainerID) {
        let Some(integrity) = self.integrity.as_ref() else {
            return;
        };
        let old = match self.storage.read().await.load(container_id) {
            Ok(Some(c)) => c,
            _ => return,
        };
        match serde_json::to_vec(&old) {
            Ok(bytes) => {
                if let Err(e) = integrity
                    .read()
                    .await
                    .create_snapshot(container_id, &bytes)
                    .await
                {
                    tracing::warn!(
                        container_id,
                        error = %e,
                        "pre-write integrity snapshot failed (non-fatal)"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(container_id, error = %e, "pre-write snapshot serialize failed (non-fatal)");
            }
        }
    }
    
    /// Query ZSEI — THE graph write choke point. Every successful mutation
    /// (create/update/delete/link) publishes a scoped GraphEvent to the
    /// living-graph ripple (src/graph_events.rs): websockets, monitor, and
    /// per-interest hooks all see graph changes as they happen.
    pub async fn query(&self, query: ZSEIQuery) -> OzoneResult<ZSEIQueryResult> {
        // Capture write provenance BEFORE the query consumes its payload.
        let ripple = Self::ripple_info(&query);
        // UPDATE-ID FIX prerequisite: update/delete results don't carry a
        // ContainerID, so the emit below fell back to id 0 and downstream
        // subscribers fetched the WRONG payload (found live 2026-10-02:
        // actors logged container_id=0 → matched=false forever). The query
        // knows its target — grab it before `query` is consumed.
        let query_target_id = match &query {
            ZSEIQuery::UpdateContainer { container_id, .. } => Some(*container_id),
            ZSEIQuery::DeleteContainer { container_id } => Some(*container_id),
            _ => None,
        };
        // T-I4: feed the container about to be mutated into the integrity
        // monitor's blake3-verified rollback layer, so every update/delete
        // has a real pre-write snapshot the periodic check can verify.
        match &query {
            ZSEIQuery::UpdateContainer { container_id, .. }
            | ZSEIQuery::DeleteContainer { container_id } => {
                self.pre_write_snapshot(*container_id).await;
            }
            _ => {}
        }
        // B18 fix: Delete's real type/keywords must be captured BEFORE the
        // delete runs — the container won't exist to look up afterward.
        // ripple_info() can only hand back ContainerType::default()/empty
        // keywords for Delete (it has no container to read, only an id),
        // which made every delete ripple invisible to any scope-filtered
        // (non-global) subscriber in graph_events::visible_to.
        let pre_delete_real = if let ZSEIQuery::DeleteContainer { container_id } = &query {
            self.get_container(*container_id).await.ok().flatten()
        } else {
            None
        };
        let result = {
            let mut qp = self.query_processor.write().await;
            let mut storage = self.storage.write().await;
            qp.process(&mut storage, &self.traversal, query).await?
        };
        if let Some((event, parent_id, mut container_type, mut scope_keywords)) = ripple {
            let container_id = match &result {
                ZSEIQueryResult::ContainerID(id) => Some(*id),
                _ => None,
            };
            // CACHE COHERENCY: process() writes through storage directly and
            // bypasses the hot cache — invalidate the affected ids so the
            // next read sees the new state (found live: claim dedupe read a
            // stale root child list through the cache).
            if let Some(id) = container_id {
                self.cache.write().await.remove(&id);
            }
            if parent_id != 0 {
                self.cache.write().await.remove(&parent_id);
            }
            // B18 fix: same bug as Delete, but the opposite timing — Update
            // must be looked up AFTER process() so it reflects the real
            // post-update state, and it can only be a fresh storage read
            // (not the pre-write snapshot) because the update already
            // changed it. The cache invalidation above just ran for this
            // same id, so this get_container() reads real storage, not a
            // stale cache entry.
            if event == "deleted" {
                if let Some(c) = &pre_delete_real {
                    container_type = c.local_state.metadata.container_type.display_name().to_string();
                    scope_keywords = c.local_state.context.keywords.clone();
                }
            } else if event == "updated" {
                if let Ok(Some(c)) = self.get_container(parent_id).await {
                    container_type = c.local_state.metadata.container_type.display_name().to_string();
                    scope_keywords = c.local_state.context.keywords.clone();
                }
            }
            // UPDATE-ID FIX: recover the target when the result is silent
            // (see the query_target_id capture above for the full story).
            let container_id = container_id.or(query_target_id);
            if let Some(id) = container_id {
                crate::graph_events::emit(event, id, parent_id, container_type, "zsei", scope_keywords);
            } else if event != "created" {
                // Non-allocating mutations (update/delete/link) carry their
                // target id in the ripple info; success = it rippled.
                crate::graph_events::emit(event, 0, parent_id, container_type, "zsei", scope_keywords);
            }
        }
        Ok(result)
    }

    /// Extract (event, parent, type, scope_keywords) from a write query —
    /// called pre-execution because CreateContainer moves its container.
    /// For UpdateContainer/DeleteContainer the type/keywords here are only
    /// a fallback (used if the real container lookup in query() fails) —
    /// query() overwrites them with the real container's values, since a
    /// bare id has no type/keywords to give without reading storage.
    ///
    /// LinkFile/LinkURL/LinkPackage's arms below are currently DEAD CODE:
    /// query.rs's QueryProcessor::process() has no match arm for any of
    /// the three, so they always fall into its catch-all
    /// `Err("Unsupported query type")`. query()'s `?` on process() then
    /// returns before this function's result is ever used for them. Real
    /// file/url/package linking happens through the separate
    /// file_link/url_link/package_link pipelines calling
    /// CreateContainer/UpdateContainer directly, not through these
    /// ZSEIQuery variants. Left as-is (harmless, unreachable) rather than
    /// implemented, since making them live is a query.rs design decision
    /// (what should LinkFile actually do?) outside this fix's scope.
    fn ripple_info(query: &ZSEIQuery) -> Option<(&'static str, u64, String, Vec<String>)> {
        use crate::types::container::ContainerType;
        Some(match query {
            ZSEIQuery::CreateContainer { parent_id, container } => (
                "created",
                *parent_id,
                container.local_state.metadata.container_type.display_name().to_string(),
                container.local_state.context.keywords.clone(),
            ),
            ZSEIQuery::UpdateContainer { container_id, .. } => {
                ("updated", *container_id, ContainerType::default().display_name().to_string(), Vec::new())
            }
            ZSEIQuery::DeleteContainer { container_id } => (
                "deleted", *container_id, ContainerType::default().display_name().to_string(), Vec::new(),
            ),
            // Dead code — see doc comment above.
            ZSEIQuery::LinkFile { project_id, .. } => (
                "linked", *project_id, "FileRef".to_string(), Vec::new(),
            ),
            ZSEIQuery::LinkURL { project_id, .. } => (
                "linked", *project_id, "UrlRef".to_string(), Vec::new(),
            ),
            ZSEIQuery::LinkPackage { project_id, .. } => (
                "linked", *project_id, "PackageRef".to_string(), Vec::new(),
            ),
            _ => return None,
        })
    }
    
    /// Get a container by ID
    pub async fn get_container(&self, id: ContainerID) -> OzoneResult<Option<Container>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(container) = cache.get(&id) {
                return Ok(Some(container.clone()));
            }
        }
        
        // Load from storage
        let storage = self.storage.read().await;
        let container = storage.load(id)?;
        
        // Update cache if found
        if let Some(ref c) = container {
            let mut cache = self.cache.write().await;
            if cache.len() < self.config.max_containers_in_memory {
                cache.insert(id, c.clone());
            }
        }
        
        Ok(container)
    }
    
    /// Store a container
    pub async fn store_container(&self, container: Container) -> OzoneResult<ContainerID> {
        let id = container.global_state.container_id;
        
        // Store to disk
        {
            let mut storage = self.storage.write().await;
            storage.store(&container)?;
        }
        
        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(id, container);
        }
        
        Ok(id)
    }
    
    /// Traverse from a starting container
    pub async fn traverse(&self, request: TraversalRequest) -> OzoneResult<TraversalResult> {
        let storage = self.storage.read().await;
        self.traversal.traverse(&storage, request).await
    }
    
    /// Get root container ID
    pub fn root_id(&self) -> ContainerID {
        0 // Root is always ID 0
    }
}
