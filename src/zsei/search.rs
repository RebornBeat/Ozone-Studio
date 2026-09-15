//! K-ALGORITHM: search strategies — selectable, registered, swappable.
//!
//! The keyword search over containers is NOT one hardcoded scan: strategies
//! are registered here by name, one is the default, and callers (or config,
//! later) select. Default: `scan` — full scan, substring match with a
//! minimum keyword length (fixes a confirmed false-positive storm short
//! filler words caused under the old behavior). `scan-legacy` — the
//! pre-fix behavior, preserved and selectable rather than deleted. Also
//! ships `exact` — whole-term equality. Traversal-based and
//! embedding-based strategies register the same way.
//!
//! Policy: algorithms are never removed here, only superseded — an
//! improved implementation gets its own registration (or becomes the new
//! default under the existing name) while whatever it replaces stays
//! registered under its own name so it's still selectable.

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

/// Common short English words that are ~never meaningful search signal but
/// are frequent enough — and long enough to survive a bare length check —
/// to cause real false-positive substring matches. Confirmed live: the
/// query keyword "the" (an ordinary 3-letter stopword, present in almost
/// any sentence) is itself a substring of "pho-to-SYNTHE-sis", so a
/// completely unrelated "say hello" request matched a stored photosynthesis
/// container purely because both happened to contain "the". A bare length
/// threshold doesn't catch this — the word is a legitimate length, just
/// semantically empty. Not exhaustive; covers the highest-frequency
/// offenders rather than being a full NLP stopword list.
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "her", "was",
    "one", "our", "out", "day", "get", "has", "him", "his", "how", "man", "new",
    "now", "see", "two", "way", "who", "did", "its", "let", "put", "say", "she",
    "too", "use", "with", "have", "this", "that", "from", "they", "will", "would",
    "there", "their", "what", "about", "which", "when", "your", "then", "than",
    "into", "over", "such", "only", "also", "been", "were", "being", "more",
];

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
        // Keywords shorter than 3 chars, or common stopwords, carry ~no
        // discriminating signal and are the direct cause of a real,
        // confirmed-live false-positive storm: bidirectional substring
        // matching means a short or common query word is a substring of
        // (or contains) unrelated stored keywords — a 1-char word like "a"
        // matches "modality"/"math"/"data"/etc, and a stopword like "the"
        // matches "photosynthesis" (contains "synTHEsis"). Both are
        // confirmed-live failure modes (a SAST-audit request flooded with
        // ~25 irrelevant structural-root entries; a "say hello" request
        // pulling in an unrelated stored photosynthesis container).
        // Filtering both sides — query AND stored keywords — fixes this for
        // every keyword-based container search system-wide, not just the
        // call sites that surfaced it.
        let is_signal = |k: &str| k.len() >= 3 && !STOPWORDS.contains(&k);
        let lowered: Vec<String> = keywords
            .iter()
            .map(|k| k.to_lowercase())
            .filter(|k| is_signal(k))
            .collect();
        if lowered.is_empty() {
            return Ok(Vec::new());
        }
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
                    .filter(|s| is_signal(s))
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

/// PRESERVED, NOT DELETED — the original `scan` behavior before the
/// false-positive fix (see `KeywordScan`'s doc comment): bidirectional
/// substring match with no minimum keyword length. Confirmed live to cause
/// near-universal false matches on any query containing a short filler word
/// (e.g. "a" matches "modality", "math", "data", ...) — no longer the
/// default, but kept registered and selectable under its own name per
/// policy: algorithms are converted into selectable options here, never
/// dropped outright, even when superseded.
pub struct LooseScan;

impl SearchStrategy for LooseScan {
    fn id(&self) -> &'static str {
        "scan-legacy"
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
    /// Ships with `scan` (default), `scan-legacy` (preserved pre-fix
    /// behavior), and `exact` registered.
    pub fn new() -> Self {
        let mut strategies = HashMap::new();
        strategies.insert("scan".to_string(), Arc::new(KeywordScan) as Arc<dyn SearchStrategy>);
        strategies.insert("scan-legacy".to_string(), Arc::new(LooseScan) as Arc<dyn SearchStrategy>);
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
