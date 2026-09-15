//! Real, search-backed population of national/local jurisdiction content.
//!
//! This is deliberately separate from `jurisdiction.rs` (the pure
//! enforcement mechanism) because it does something that module's own rule
//! explicitly forbids for itself: originate new rule content. The
//! difference is the source: this file never asks an LLM "what is this
//! country's law" from training knowledge — it makes a real web search call
//! (pipeline 56) and uses only what a real HTTP response actually returned,
//! with the retrieved URL and snippet preserved verbatim in the rule
//! (`JurisdictionRule::source`/`retrieved_snippet`). If search finds
//! nothing, or web search itself is unconfigured, the honest result is "no
//! content for this region yet" — never a fabricated fallback. This mirrors
//! the established honesty contract already used for methodology gaps and
//! web search unavailability elsewhere in this codebase.
//!
//! Scope is deliberately narrow: a small, fixed list of guardrail-relevant
//! topics (matching the spirit of the existing Global/UN starting set — not
//! "all law"), triggered lazily per-region the first time a real,
//! hardware-or-explicitly-configured region is actually seen with no
//! national/local content loaded yet — never a bulk sweep across every
//! country, which would not be a responsible use of a search API and would
//! produce a mountain of unreviewed content no human has looked at.
//!
//! Every rule this produces gets `RuleAction::Log` regardless of topic —
//! never `Block`/`RequireConfirmation`/`Warn`. Assigning real enforcement
//! severity to an automatically retrieved search snippet (which might be a
//! summary/marketing page, not the actual statute) would be exactly the
//! false-confidence risk this whole design has rejected from the start.
//! `Log` makes the content visible/auditable without ever interrupting a
//! real request — a human reviewing `jurisdiction_gate_result` or the
//! stored content file can deliberately upgrade a specific rule's action
//! once they've verified it against the real source.

use super::jurisdiction::{JurisdictionContentFile, JurisdictionRule, JurisdictionScope, RuleAction};
use super::{PipelineExecutor, StoreAccess};
use std::collections::HashSet;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

/// (trigger keyword — matched against a request's prompt by the gate,
/// search topic phrase — what we actually search for, human label — for
/// logging only). Intentionally small; matches the same class of topic the
/// hand-curated Global set already covers. Extend this list deliberately,
/// not by having an LLM invent more topics.
const JURISDICTION_SEARCH_TOPICS: &[(&str, &str, &str)] = &[
    ("child", "minors online safety protection law", "minors_protection"),
    ("privacy", "data privacy personal information protection law", "data_privacy"),
    ("consumer", "consumer protection law", "consumer_protection"),
];

/// Process-lifetime memory of which regions have already been attempted, so
/// a still-empty region doesn't re-trigger a real search call on every
/// single request. Not persisted across restarts — worst case is one retry
/// per boot, which is fine; the alternative (persisting "already tried and
/// found nothing" forever) risks never retrying a region whose real content
/// simply wasn't findable yet.
fn attempted_regions() -> &'static Mutex<HashSet<String>> {
    static ATTEMPTED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    ATTEMPTED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Real, checkable signal — was the retrieved URL on a domain that looks
/// official (government/international-body) — not a fabricated confidence
/// score. Deliberately simple substring checks; a wrong classification here
/// only affects the recorded `official_source` flag, never whether the
/// content is trusted for enforcement (nothing this function produces ever
/// gets an action stronger than `Log`).
fn looks_official(url: &str) -> bool {
    let u = url.to_lowercase();
    u.contains(".gov")
        || u.contains(".gouv")
        || u.contains(".europa.eu")
        || u.contains("legislation.gov")
        || u.contains(".un.org")
        || u.contains("official-gazette")
}

/// Attempt to populate real, search-backed national-scope jurisdiction
/// content for `region` (e.g. "US", "US-CA", "DO" — same free-form label as
/// `JurisdictionConfig.instance_region`). Fire-and-forget: intended to be
/// spawned as a background task (see `jurisdiction.rs`'s
/// `stage_jurisdiction_gate`) so a first-seen region never adds web-search
/// latency to the live request that triggered it. Every failure path here
/// is silent-but-logged (`tracing::warn!`) rather than propagated — this is
/// best-effort background population, not something a live request should
/// ever depend on succeeding.
pub async fn populate_jurisdiction_content_for_region(
    executor: Arc<dyn PipelineExecutor>,
    store: Arc<dyn StoreAccess>,
    region: String,
) {
    let region_key = region.to_lowercase();
    {
        let mut attempted = attempted_regions().lock().await;
        if attempted.contains(&region_key) {
            return;
        }
        attempted.insert(region_key.clone());
    }

    tracing::info!(region = %region, "Jurisdiction: no national/local content loaded yet — attempting real search-backed population");

    let mut rules: Vec<JurisdictionRule> = Vec::new();

    for (trigger, topic_query, label) in JURISDICTION_SEARCH_TOPICS {
        let query = format!("{} law {}", region, topic_query);
        let search_input = serde_json::json!({
            "action": {"type": "Search", "query": query, "max_results": 5}
        });

        let result = match executor.execute(56, search_input).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(region = %region, topic = %label, error = %e, "Jurisdiction content search call failed");
                continue;
            }
        };

        let success = result.get("success").and_then(|s| s.as_bool()).unwrap_or(false);
        if !success {
            let err = result.get("error").and_then(|e| e.as_str()).unwrap_or("unknown");
            tracing::info!(region = %region, topic = %label, error = %err, "Jurisdiction content search unavailable for this topic — leaving unpopulated (honest gap, not fabricated)");
            continue;
        }

        let results = result.get("results").and_then(|r| r.as_array()).cloned().unwrap_or_default();
        if results.is_empty() {
            continue;
        }

        // Prefer the first official-looking result; fall back to the
        // first result at all if none look official — still real, still
        // cited, just recorded with official_source: false so a reviewer
        // knows to weight it accordingly.
        let chosen = results
            .iter()
            .find(|r| r.get("url").and_then(|u| u.as_str()).map(looks_official).unwrap_or(false))
            .or_else(|| results.first());

        let Some(chosen) = chosen else { continue };
        let Some(url) = chosen.get("url").and_then(|u| u.as_str()) else { continue };
        let title = chosen.get("title").and_then(|t| t.as_str()).unwrap_or("");
        let snippet = chosen.get("snippet").and_then(|s| s.as_str()).unwrap_or("");

        rules.push(JurisdictionRule {
            scope: JurisdictionScope::National(region.clone()),
            condition: trigger.to_string(),
            action: RuleAction::Log,
            source: url.to_string(),
            retrieved_snippet: Some(format!("{} — {}", title, snippet).trim().to_string()),
            official_source: Some(looks_official(url)),
        });
    }

    if rules.is_empty() {
        tracing::info!(region = %region, "Jurisdiction: search-backed population found nothing usable for this region — remains unpopulated (national/local layer stays empty, Global scope still applies)");
        return;
    }

    let content = JurisdictionContentFile {
        disclaimer: format!(
            "This content was auto-retrieved via a live web search (not generated from training \
             knowledge) on region \"{}\" for a small fixed set of guardrail-relevant topics. Every \
             rule's action is Log ONLY — it is recorded/visible but never blocks, warns, or requires \
             confirmation for a real request. This is NOT verified legal compliance and NOT a \
             substitute for qualified legal counsel: `retrieved_snippet` is a search result excerpt, \
             not full statute text, and `official_source` reflects only a domain-name heuristic at \
             retrieval time, not manual verification. A human must review each rule's real source \
             before upgrading its action beyond Log.",
            region
        ),
        rules,
    };

    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let dir = format!("{}/jurisdiction", data_dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!(region = %region, error = %e, "Failed to create jurisdiction content directory");
        return;
    }
    let object_store_path = format!("jurisdiction/{}.json", region_key);
    let full_path = format!("{}/{}.json", dir, region_key);
    let json = match serde_json::to_string_pretty(&content) {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!(region = %region, error = %e, "Failed to serialize jurisdiction content");
            return;
        }
    };
    if let Err(e) = std::fs::write(&full_path, &json) {
        tracing::warn!(region = %region, path = %full_path, error = %e, "Failed to write jurisdiction content file");
        return;
    }

    // Real ZSEI container referencing the file, same shape as the manual
    // Global-scope setup command (docs/JURISDICTION_SETUP.md), parented
    // under the real JurisdictionRoot structural container rather than
    // left dangling — the same "hardcoded parent_id: 0" class of bug fixed
    // elsewhere this session for text/code/AMT graphs.
    let ts = now_secs();
    let content_hash: Vec<u8> = vec![0u8; 32];
    let container = serde_json::json!({
        "global_state": {"container_id": 0, "child_count": 0, "version": 1, "parent_id": 0, "child_ids": []},
        "local_state": {
            "metadata": {
                "container_type": "JurisdictionRuleSet",
                "modality": "Unknown",
                "created_at": ts, "updated_at": ts,
                "provenance": "jurisdiction_search", "permissions": 0, "owner_id": 0,
                "name": format!("{} Jurisdiction Rules (search-retrieved, Log-only)", region),
                "materialized_path": null
            },
            "context": {
                "categories": [], "methodologies": [],
                "keywords": [region_key.clone()],
                "topics": ["jurisdiction", "national-scope", "search-retrieved"],
                "relationships": [], "learned_associations": [], "embedding": null
            },
            "storage": {
                "db_shard_id": null, "vector_index_ref": null,
                "object_store_path": object_store_path,
                "compression_type": "None"
            },
            "hints": {"access_frequency": 0, "hotness_score": 0.0, "last_accessed": 0, "centroid": null, "ml_prediction_weight": 0.0},
            "integrity": {"content_hash": content_hash, "semantic_fingerprint": [], "last_verified": 0, "integrity_score": 1.0, "version_history": []},
            "file_context": null, "code_context": null, "text_context": null, "external_ref": null
        }
    });

    const JURISDICTION_ROOT_ID: u64 = 7; // crate::types::container::JURISDICTION_ROOT_ID
    match store.create_container(JURISDICTION_ROOT_ID, container).await {
        Ok(id) => {
            tracing::info!(region = %region, container_id = id, path = %full_path, "Jurisdiction: real search-backed national-scope content populated");
        }
        Err(e) => {
            tracing::warn!(region = %region, error = %e, "Failed to register jurisdiction content container (file was written, but won't be found by load_jurisdiction_rules until this is retried/fixed)");
        }
    }
}
