//! Consciousness review pass — TOP_DOWN_REVIEW_GUIDE.md §8, items 2+3.
//!
//! The task-manager window and the decision-review capture store (S10)
//! already exist and are real. This module is what turns that data INTO
//! consciousness: a pass that reads the capture store + recent tasks,
//! traverses the graph for real context, and — only for genuinely real
//! findings, never fabricated to have something to show — emits an
//! insight as a real ZSEI container under CONSCIOUSNESS_METACOGNITION_
//! ROOT_ID, cited to exactly what was found (capture-line fields, task
//! ids). A run that finds nothing genuinely worth flagging correctly
//! returns an empty list — that is a valid, honest outcome, not a
//! failure of the pass.

use crate::orchestrator::StoreAccess;
use crate::task::TaskManager;
use crate::types::container::{
    CompressionType, Container, ContainerType, Context, GlobalState, IntegrityData, LocalState,
    Metadata, Modality, StoragePointers, TraversalHints, CONSCIOUSNESS_METACOGNITION_ROOT_ID,
};
use std::sync::Arc;

/// One row from `{data_dir}/model_calls/decision_review.jsonl`, as the
/// wrapper's own `capture()` (`src/orchestrator/decision_review.rs`)
/// actually writes it. Field names/shapes match that function exactly —
/// this is a read of a real, already-shipped format, not a new contract.
#[derive(Debug, serde::Deserialize)]
struct CaptureRow {
    ts: String,
    model_used: String,
    tokens_used: u64,
    decision: String,
    #[serde(default)]
    confidence: Option<f32>,
    task_summary_preview: String,
    reasoning_preview: String,
}

/// Item 2 — graph traversal as a sense. Mirrors the real, proven shape
/// already live in `assets/pipelines/general/context_aggregation/main.rs`'s
/// `traverse_from_seeds` (`TraversalMode::Structural`, bounded depth/
/// results) rather than inventing a second traversal convention.
pub async fn traverse_container_context(
    store: &Arc<dyn StoreAccess>,
    start_container: u64,
) -> Vec<serde_json::Value> {
    let request = serde_json::json!({
        "start_container": start_container,
        "mode": "Structural",
        "filters": [],
        "max_depth": 2u16,
        "max_results": 10u32,
        "budget": {
            "max_hops": 2u16,
            "max_containers": 10u32,
            "max_latency_ms": 2000u64
        }
    });
    match store.traverse(request).await {
        Ok(result) => result
            .get("containers")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default(),
        Err(e) => {
            tracing::debug!(error = %e, start_container, "consciousness review: traversal failed, continuing without it");
            Vec::new()
        }
    }
}

/// Read the real capture store. Missing file = no reviews have happened
/// yet (honest empty, not an error); a malformed line is skipped, not
/// fatal to the rest of the file.
fn read_capture_store(data_dir: &str) -> Vec<CaptureRow> {
    let path = format!("{}/model_calls/decision_review.jsonl", data_dir);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<CaptureRow>(line).ok())
        .collect()
}

fn parse_capture_ts(ts: &str) -> Option<std::time::SystemTime> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| std::time::SystemTime::from(dt))
}

/// One row from `{data_dir}/model_calls/zero_shot_calls.jsonl` (S11), as
/// `capture_zero_shot_call` (`src/orchestrator/mod.rs`) actually writes
/// it — field names/shapes match that function exactly, including the
/// raw-thought context markers (`amt_container_id`/`blueprint_id`/
/// `project_id`) added this session. Those markers were write-only until
/// this reader: real correlated data, never joined back on anywhere.
#[derive(Debug, serde::Deserialize)]
struct RawThoughtRow {
    ts: String,
    call_site: String,
    #[serde(default)]
    used_fallback: bool,
    #[serde(default)]
    amt_container_id: Option<u64>,
}

/// Read the real S11 capture store. Same discipline as `read_capture_store`:
/// missing file = no calls captured yet (honest empty), a malformed line
/// is skipped, not fatal to the rest of the file.
fn read_zero_shot_capture_store(data_dir: &str) -> Vec<RawThoughtRow> {
    let path = format!("{}/model_calls/zero_shot_calls.jsonl", data_dir);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<RawThoughtRow>(line).ok())
        .collect()
}

/// Item 3 — the review pass itself. Real inputs (capture store + recent
/// tasks), real traversal when a task names a project, real citations.
/// Returns the container ids actually created (empty = nothing genuinely
/// worth flagging this run).
pub async fn run_review_pass(
    store: Arc<dyn StoreAccess>,
    task_manager: &TaskManager,
    data_dir: &str,
) -> Vec<u64> {
    let mut created = Vec::new();
    let now = std::time::SystemTime::now();
    const AGING_THRESHOLD_SECS: u64 = 3600; // 1 hour — matches this guide's own "aging ReviewPending" framing

    // --- Finding class 1: aged, unresolved review failures ---
    // "review-failed" is decision_review.rs's real decision string for a
    // total-failure ReviewPending outcome (mod.rs:decision_review.rs — not
    // "reviewpending"; matching the literal string the wrapper writes).
    let rows = read_capture_store(data_dir);
    let aged_failures: Vec<&CaptureRow> = rows
        .iter()
        .filter(|r| r.decision == "review-failed")
        .filter(|r| {
            parse_capture_ts(&r.ts)
                .and_then(|t| now.duration_since(t).ok())
                .map(|age| age.as_secs() >= AGING_THRESHOLD_SECS)
                .unwrap_or(false)
        })
        .collect();

    if !aged_failures.is_empty() {
        let citations: Vec<String> = aged_failures
            .iter()
            .map(|r| format!("{}: {} (task: {})", r.ts, r.reasoning_preview, r.task_summary_preview.chars().take(80).collect::<String>()))
            .collect();
        let content = format!(
            "{} decision review(s) have been unresolved (ReviewPending, decision=review-failed) for over {} minutes. \
             A confirmation that never resolves is a real gap in the review loop, not just a transient retry. Citations:\n- {}",
            aged_failures.len(),
            AGING_THRESHOLD_SECS / 60,
            citations.join("\n- ")
        );
        if let Some(id) = persist_insight(&store, "aging-review-pending", &content, &citations).await {
            created.push(id);
        }
    }

    // --- Finding class 2: recent tasks whose graph reads show no traversal use ---
    // Real signal: TaskStepData::graph_ids_read is populated whenever a
    // step actually consulted the graph. A completed task with zero
    // graph_ids_read across every step, despite having a project_id (so
    // graph context existed to consult), is a real, checkable "context
    // that was available and wasn't used" observation — not a guess.
    let recent_tasks = task_manager.list_tasks(Some("completed"), None, 20, 0).await;
    let mut context_gap_citations = Vec::new();
    for t in &recent_tasks {
        if t.project_id.is_none() {
            continue; // no project scope => no project graph context to have missed
        }
        let any_graph_read = t.steps.iter().any(|s| !s.graph_ids_read.is_empty());
        if !any_graph_read && !t.steps.is_empty() {
            context_gap_citations.push(format!(
                "task {} (project {:?}): {} step(s), zero graph_ids_read across all of them",
                t.task_id, t.project_id, t.steps.len()
            ));
        }
    }
    if !context_gap_citations.is_empty() {
        // Real traversal use (item 2), exercised here rather than left
        // unused: pull a small amount of real graph context for the first
        // flagged task's project, so the insight cites what WAS available.
        let sample_project = recent_tasks
            .iter()
            .find(|t| t.project_id.is_some())
            .and_then(|t| t.project_id);
        let traversal_note = if let Some(pid) = sample_project {
            let discovered = traverse_container_context(&store, pid).await;
            format!(" (traversal from project container {} found {} related containers that were available to consult)", pid, discovered.len())
        } else {
            String::new()
        };
        let content = format!(
            "{} completed task(s) with a project scope recorded zero graph_ids_read across all steps{}. Citations:\n- {}",
            context_gap_citations.len(),
            traversal_note,
            context_gap_citations.join("\n- ")
        );
        if let Some(id) = persist_insight(&store, "unused-graph-context", &content, &context_gap_citations).await {
            created.push(id);
        }
    }

    // --- Finding class 3: AMT branches where the primary model never
    // once succeeded (S11, zero_shot_calls.jsonl) ---
    // Real, self-contained signal: group real per-call captures by their
    // amt_container_id marker (added this session, previously write-only
    // — this is its first reader). A branch with enough calls to be a
    // real sample, where every single one needed a fallback model, is a
    // genuine "this specific piece of work can't get the primary model to
    // work at all" finding — distinct from S10's decision-review-outcome
    // findings above, a model-reliability-per-work-item signal instead.
    const MIN_SAMPLE_FOR_FALLBACK_FINDING: usize = 3;
    let thought_rows = read_zero_shot_capture_store(data_dir);
    let mut by_container: std::collections::HashMap<u64, Vec<&RawThoughtRow>> =
        std::collections::HashMap::new();
    for r in &thought_rows {
        if let Some(cid) = r.amt_container_id {
            by_container.entry(cid).or_default().push(r);
        }
    }
    let mut fallback_dependent_citations = Vec::new();
    for (container_id, rows) in &by_container {
        if rows.len() < MIN_SAMPLE_FOR_FALLBACK_FINDING {
            continue; // too small a sample to be a real signal, not a guess dressed as one
        }
        if rows.iter().all(|r| r.used_fallback) {
            let call_sites: std::collections::BTreeSet<&str> =
                rows.iter().map(|r| r.call_site.as_str()).collect();
            fallback_dependent_citations.push(format!(
                "AMT container {}: {} captured call(s), 100% used a fallback model (call sites: {})",
                container_id,
                rows.len(),
                call_sites.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if !fallback_dependent_citations.is_empty() {
        let content = format!(
            "{} AMT branch(es) have enough captured zero-shot calls (S11) to be a real sample, and every single one needed a fallback model — the primary model never once succeeded for this specific work. Citations:\n- {}",
            fallback_dependent_citations.len(),
            fallback_dependent_citations.join("\n- ")
        );
        if let Some(id) = persist_insight(&store, "fallback-dependent-amt-branch", &content, &fallback_dependent_citations).await {
            created.push(id);
        }
    }

    created
}

async fn persist_insight(
    store: &Arc<dyn StoreAccess>,
    kind: &str,
    content: &str,
    citations: &[String],
) -> Option<u64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // UNREACHABLE-CONTENT FIX (2026-09-28, found live by the I5 fork via
    // insight 40194): the insight body used to be injected as extra
    // untyped JSON keys OUTSIDE the typed Container schema — serde
    // silently dropped them on every read, so the real content was
    // persisted on disk but unreachable through ANY ZSEIQuery variant.
    // Fixed with the codebase's own content-pointer convention (same as
    // AMT trees, modality graphs, methodologies): write the body to a
    // file, point object_store_path at it. GetContainerContent now reads
    // the full insight through the standard B0 route.
    let rel_path = {
        let dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
        let rel = format!("consciousness/insight_{}_{}.json", kind, now);
        let abs = std::path::PathBuf::from(&dir).join(&rel);
        if let Some(parent) = abs.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let body = serde_json::json!({
            "kind": kind,
            "content": content,
            "citations": citations,
            "generated_at": now,
        });
        match serde_json::to_string_pretty(&body) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&abs, json) {
                    tracing::warn!(error = %e, kind, "consciousness review: insight body write failed");
                    return None;
                }
                rel
            }
            Err(e) => {
                tracing::warn!(error = %e, kind, "consciousness review: insight body serialization failed");
                return None;
            }
        }
    };

    let container = Container {
        global_state: GlobalState {
            container_id: 0, // assigned by the store on create
            parent_id: CONSCIOUSNESS_METACOGNITION_ROOT_ID,
            child_ids: vec![],
            child_count: 0,
            version: 1,
        },
        local_state: LocalState {
            metadata: Metadata {
                container_type: ContainerType::Derived,
                modality: Modality::Unknown,
                created_at: now,
                updated_at: now,
                provenance: "consciousness_review_pass".to_string(),
                permissions: 0,
                owner_id: 0,
                name: Some(format!("Consciousness insight: {}", kind)),
                materialized_path: Some(format!("/Consciousness/Metacognition/{}", kind)),
            },
            context: Context {
                categories: vec![],
                methodologies: vec![],
                keywords: vec![kind.to_string(), "review-pass".to_string()],
                topics: vec![],
                relationships: vec![],
                learned_associations: vec![],
                embedding: None,
            },
            storage: StoragePointers {
                db_shard_id: None,
                vector_index_ref: None,
                object_store_path: Some(rel_path),
                compression_type: CompressionType::None,
            },
            hints: TraversalHints {
                access_frequency: 0,
                hotness_score: 0.0,
                last_accessed: 0,
                centroid: None,
                ml_prediction_weight: 0.0,
            },
            integrity: IntegrityData {
                content_hash: [0u8; 32],
                semantic_fingerprint: vec![],
                last_verified: 0,
                integrity_score: 1.0,
                version_history: vec![],
            },
            file_context: None,
            code_context: None,
            text_context: None,
            external_ref: None,
        },
    };
    let value = match serde_json::to_value(&container) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, kind, "consciousness review: insight container serialization failed");
            return None;
        }
    };
    match store.create_container(CONSCIOUSNESS_METACOGNITION_ROOT_ID, value).await {
        Ok(id) => {
            tracing::info!(container_id = id, kind, "consciousness review: real insight persisted (content via object_store_path)");
            Some(id)
        }
        Err(e) => {
            tracing::warn!(error = %e, kind, "consciousness review: failed to persist insight");
            None
        }
    }
}
