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
//! 2026-11-08 — `token`: TOKEN-AWARE and RANKED (docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md). Whole-term / whole-token matching (no substring flood:
//! "file" no longer matches "profile"), no stopword list, markers meet only markers, and the result is ORDERED best-first by the deterministic v0
//! key (scope, match class, matched terms, validation, real hit history, graph distance, recency) fused with every active model backend. It is
//! selectable per query (`strategy: "token"`, `StoreAccess::search_ranked`) and as the global default via `OZONE_SEARCH_DEFAULT=token`; `scan`
//! (substring + stoplist), `scan-legacy` and `exact` stay registered as selectable baselines for the evaluation harness. Every search is also
//! logged as a HIT event (`keyword_program::record_search`) — the training data.
//!
//! Policy: algorithms are never removed here, only superseded — an
//! improved implementation gets its own registration (or becomes the new
//! default under the existing name) while whatever it replaces stays
//! registered under its own name so it's still selectable.

use crate::types::container::ContainerType;
use crate::types::ContainerID;
use crate::types::OzoneResult;
use super::storage::ContainerStorage;
use crate::shared_contracts::keyword_model as km;
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

/// TOKEN-AWARE, RANKED (see the module doc). Matches on a container's NAME, keywords and topics by whole term / whole token only, then
/// orders the matches with `keyword_program::order_for_search` (v0 tuple + active backends, reciprocal-rank-fused).
pub struct TokenAware;

impl SearchStrategy for TokenAware {
    fn id(&self) -> &'static str {
        "token"
    }

    fn search(
        &self,
        storage: &ContainerStorage,
        keywords: &[String],
        type_filter: Option<ContainerType>,
    ) -> OzoneResult<Vec<ContainerID>> {
        let query: Vec<String> = keywords.iter().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let mut cands: Vec<km::Candidate> = Vec::new();
        for id in storage.all_ids() {
            let Some(container) = storage.load(id)? else { continue };
            if let Some(ct) = type_filter {
                if container.local_state.metadata.container_type != ct {
                    continue;
                }
            }
            let ctx = &container.local_state.context;
            let name = container.local_state.metadata.name.clone().unwrap_or_default();
            let mut terms: Vec<String> = ctx.keywords.clone();
            terms.extend(ctx.topics.iter().cloned());
            let Some(info) = km::match_info(&query, &name, &terms, false) else { continue };
            let matched_keywords: Vec<String> =
                ctx.keywords.iter().filter(|k| km::match_info(&query, "", &[(*k).clone()], false).is_some()).cloned().collect();
            let facts = crate::keyword_program::facts_from_context(
                info.best,
                info.matched_terms,
                &ctx.keyword_meta,
                &matched_keywords,
                container.local_state.hints.last_accessed,
                container.local_state.hints.hotness_score,
            );
            let mut cand_terms = km::meaning_terms(&terms);
            if cand_terms.is_empty() && !name.is_empty() {
                cand_terms.push(name);
            }
            cands.push(km::Candidate { id, terms: cand_terms, facts });
        }
        Ok(crate::keyword_program::order_for_search(&query, &cands))
    }
}

/// LIST ALL (2026-11-08): every container id (optionally of one type), keywords ignored — the enumeration door the keyword settlement pass
/// needs (`StoreAccess::list_ids`). Not a search; registered so it rides the existing query surface without a new `ZSEIQuery` variant.
pub struct ListAll;

impl SearchStrategy for ListAll {
    fn id(&self) -> &'static str {
        "all"
    }

    fn search(
        &self,
        storage: &ContainerStorage,
        _keywords: &[String],
        type_filter: Option<ContainerType>,
    ) -> OzoneResult<Vec<ContainerID>> {
        let mut out = Vec::new();
        for id in storage.all_ids() {
            match type_filter {
                None => out.push(id),
                Some(ct) => {
                    if let Some(c) = storage.load(id)? {
                        if c.local_state.metadata.container_type == ct {
                            out.push(id);
                        }
                    }
                }
            }
        }
        Ok(out)
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
        strategies.insert("token".to_string(), Arc::new(TokenAware) as Arc<dyn SearchStrategy>);
        strategies.insert("all".to_string(), Arc::new(ListAll) as Arc<dyn SearchStrategy>);
        // The global default stays `scan` until the evaluation harness says otherwise; `OZONE_SEARCH_DEFAULT=token` flips it (any registered id).
        let default = std::env::var("OZONE_SEARCH_DEFAULT")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| strategies.contains_key(v.as_str()))
            .unwrap_or_else(|| "scan".to_string());
        Self {
            strategies: RwLock::new(strategies),
            default: RwLock::new(default),
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
        let ids = chosen.search(storage, keywords, type_filter)?;
        crate::keyword_program::record_search(keywords, chosen.id(), &ids);
        Ok(ids)
    }
}

impl Default for SearchRegistry {
    fn default() -> Self {
        Self::new()
    }
}
