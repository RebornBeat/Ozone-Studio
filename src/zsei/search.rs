//! K-ALGORITHM: search strategies — selectable, registered, swappable.
//!
//! The keyword search over containers is NOT one hardcoded scan: strategies
//! are registered here by name, one is the default, and callers (or config,
//! later) select. Default: `scan` — full scan with substring match over
//! context keywords + topics. Also ships `exact` — whole-term equality.
//! Traversal-based and embedding-based strategies register the same way.

use crate::types::container::ContainerType;
use crate::types::ContainerID;
use crate::types::OzoneResult;
use super::storage::ContainerStorage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// One search strategy: storage + keywords (+ optional container-type filter)
/// → matching container ids.
pub trait SearchStrategy: Send + Sync {
    /// Registry name ("scan", "exact", ...).
    fn id(&self) -> &'static str;

    fn search(
        &self,
        storage: &ContainerStorage,
        keywords: &[String],
        type_filter: Option<ContainerType>,
    ) -> OzoneResult<Vec<ContainerID>>;
}

/// DEFAULT — full scan, substring match against context keywords + topics.
pub struct KeywordScan;

impl SearchStrategy for KeywordScan {
    fn id(&self) -> &'static str {
        "scan"
    }

    fn search(
        &self,
        storage: &ContainerStorage,
        keywords: &[String],
        type_filter: Option<ContainerType>,
    ) -> OzoneResult<Vec<ContainerID>> {
        let lowered: Vec<String> = keywords.iter().map(|k| k.to_lowercase()).collect();
        let mut results = Vec::new();
        for id in storage.all_ids() {
            if let Some(container) = storage.load(id)? {
                if let Some(ct) = type_filter {
                    if container.local_state.metadata.container_type != ct {
                        continue;
                    }
                }
                let haystack: Vec<String> = container
                    .local_state
                    .context
                    .keywords
                    .iter()
                    .chain(container.local_state.context.topics.iter())
                    .map(|s| s.to_lowercase())
                    .collect();
                if lowered.iter().any(|k| {
                    haystack
                        .iter()
                        .any(|h| h.contains(k.as_str()) || k.contains(h.as_str()))
                }) {
                    results.push(id);
                }
            }
        }
        Ok(results)
    }
}

/// Whole-term equality match (no substring) — precision-oriented.
pub struct ExactMatch;

impl SearchStrategy for ExactMatch {
    fn id(&self) -> &'static str {
        "exact"
    }

    fn search(
        &self,
        storage: &ContainerStorage,
        keywords: &[String],
        type_filter: Option<ContainerType>,
    ) -> OzoneResult<Vec<ContainerID>> {
        let lowered: Vec<String> = keywords.iter().map(|k| k.to_lowercase()).collect();
        let mut results = Vec::new();
        for id in storage.all_ids() {
            if let Some(container) = storage.load(id)? {
                if let Some(ct) = type_filter {
                    if container.local_state.metadata.container_type != ct {
                        continue;
                    }
                }
                let haystack: Vec<String> = container
                    .local_state
                    .context
                    .keywords
                    .iter()
                    .chain(container.local_state.context.topics.iter())
                    .map(|s| s.to_lowercase())
                    .collect();
                if lowered.iter().any(|k| haystack.iter().any(|h| *h == *k)) {
                    results.push(id);
                }
            }
        }
        Ok(results)
    }
}

/// Registry of search strategies with a selectable default.
/// Unknown strategy names fall back to the default rather than erroring —
/// the caller decides whether an empty result is meaningful.
pub struct SearchRegistry {
    strategies: RwLock<HashMap<String, Arc<dyn SearchStrategy>>>,
    default: RwLock<String>,
}

impl SearchRegistry {
    /// Ships with `scan` (default) and `exact` registered.
    pub fn new() -> Self {
        let mut strategies = HashMap::new();
        strategies.insert("scan".to_string(), Arc::new(KeywordScan) as Arc<dyn SearchStrategy>);
        strategies.insert("exact".to_string(), Arc::new(ExactMatch) as Arc<dyn SearchStrategy>);
        Self {
            strategies: RwLock::new(strategies),
            default: RwLock::new("scan".to_string()),
        }
    }

    /// Register (or replace) a strategy by id.
    pub async fn register(&self, strategy: Arc<dyn SearchStrategy>) {
        self.strategies
            .write()
            .await
            .insert(strategy.id().to_string(), strategy);
    }

    /// Select the default strategy by id (no-op if unknown).
    pub async fn set_default(&self, id: &str) -> bool {
        let mut default = self.default.write().await;
        if self.strategies.read().await.contains_key(id) {
            *default = id.to_string();
            true
        } else {
            false
        }
    }

    /// Run a search: named strategy if present, else the default.
    pub async fn run(
        &self,
        strategy: Option<&str>,
        storage: &ContainerStorage,
        keywords: &[String],
        type_filter: Option<ContainerType>,
    ) -> OzoneResult<Vec<ContainerID>> {
        let strategies = self.strategies.read().await;
        let default_name = self.default.read().await.clone();
        let chosen = strategy
            .and_then(|name| strategies.get(name))
            .or_else(|| strategies.get(&default_name))
            .or_else(|| strategies.get("scan"))
            .cloned()
            .expect("search registry must always contain the scan strategy");
        chosen.search(storage, keywords, type_filter)
    }
}

impl Default for SearchRegistry {
    fn default() -> Self {
        Self::new()
    }
}
