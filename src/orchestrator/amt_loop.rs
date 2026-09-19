//! Real AMT re-expansion loop: periodically reviews AMT re-expansion
//! candidates recorded live during orchestration (see amt.rs's
//! PromptOrchestrator::record_amt_reexpansion_candidate, called from
//! build_amt whenever a freshly-persisted, project-anchored AMT still has a
//! node with no source provenance behind it — AMTNode.verified is never a
//! fabricated score, see its own doc comment) and, for a candidate not
//! already resolved, makes one real LLM call to deepen that specific branch
//! with concrete detail, then persists the updated tree.
//!
//! This is the "as a project evolves, keep revisiting and deepening
//! already-found branches, not just building the AMT once and discarding
//! it" loop, mirroring the real methodology meta-loop's exact shape
//! (meta_loop.rs) — same RefinementConfig, same executor/store contracts,
//! same log-file-based candidate tracking as record_methodology_gap uses.
//!
//! Deliberately narrow: ONE real, well-defined trigger (an unverified node
//! on a project's persisted AMT) and ONE real re-expansion action (ask the
//! model for concrete sub-detail on that specific branch, add it as a new
//! child if genuinely substantive). A full "re-run intent extraction over
//! the whole project's accumulated history" loop is a much larger,
//! separate undertaking — not built here.

use crate::config::{AvailableModel, ModelFallbackConfig};
use crate::orchestrator::{AMTNode, ModelConfigOverride, PipelineExecutor, PromptOrchestrator, StoreAccess};
use crate::task::RefinementConfig;
use std::sync::Arc;

const PROMPT_PIPELINE_ID: u64 = 9;

/// Wake signal for the re-expansion loop — poked by the graph-ripple sync
/// whenever a relevant graph change appends candidates, so AMTs update
/// event-driven (instantly) with the interval as fallback, not instead.
static AMT_WAKE: std::sync::OnceLock<tokio::sync::Notify> = std::sync::OnceLock::new();

fn wake() -> &'static tokio::sync::Notify {
    AMT_WAKE.get_or_init(tokio::sync::Notify::new)
}

/// Subscribe the AMT re-expansion pipeline to the living-graph ripple
/// (src/graph_events.rs): graph writes scoped to a project (proj:<id>) or
/// touching files (file:<path>) convert into re-expansion candidates for
/// the project's AMT, and the loop wakes immediately. This is the
/// context-alignment seam — the AMT is the source of truth, and the ripple
/// is what keeps it truthful as the graph evolves.
pub fn spawn_graph_ripple_sync(store: Arc<dyn StoreAccess>, candidates_path: String) {
    let mut rx = crate::graph_events::GraphEventHub::global().subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(evt) => {
                    if let Err(e) = process_graph_event(&*store, &candidates_path, &evt).await {
                        tracing::warn!(error = %e, "AMT ripple sync: event processing failed");
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(lagged = n, "AMT ripple sync: missed graph events");
                }
                Err(_) => break,
            }
        }
    });
}

/// Convert one graph event into re-expansion work. Relevant: writes scoped
/// to a project (proj:<id>) or touching files (file:<path>). Returns the
/// AMT container id a candidate was recorded for, if any.
pub async fn process_graph_event(
    store: &dyn StoreAccess,
    candidates_path: &str,
    evt: &crate::graph_events::GraphEvent,
) -> Result<Option<u64>, String> {
    if evt.event == "deleted" {
        return Ok(None); // deletions don't deepen branches
    }
    let proj_id = evt
        .scope_keywords
        .iter()
        .find_map(|k| k.strip_prefix("proj:").and_then(|v| v.parse::<u64>().ok()));
    let touches_file = evt.scope_keywords.iter().any(|k| k.starts_with("file:"));
    if proj_id.is_none() && !touches_file {
        return Ok(None); // global-only coordination chatter doesn't drive AMTs
    }

    // Resolve the project's AMT container: the event's parent (or project)
    // container's children, first AMT-typed child wins.
    let anchor = proj_id.unwrap_or(evt.parent_id);
    let project_json = store
        .get_container(anchor)
        .await?
        .ok_or_else(|| format!("project container {} not found", anchor))?;
    let child_ids: Vec<u64> = project_json
        .get("global_state")
        .and_then(|g| g.get("child_ids"))
        .and_then(|c| c.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();

    let mut amt_child: Option<u64> = None;
    for child in child_ids {
        if let Some(c) = store.get_container(child).await? {
            let ctype = c
                .get("local_state")
                .and_then(|l| l.get("metadata"))
                .and_then(|m| m.get("container_type"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            // AMT containers persist as "Derived"-typed children of the
            // project (see persist_amt_container's JSON contract).
            if ctype == "Derived" || ctype.contains("AMT") {
                amt_child = Some(child);
                break;
            }
        }
    }
    let Some(amt_id) = amt_child else {
        return Ok(None); // project has no AMT yet — nothing to re-expand
    };

    append_reexpansion_candidate(
        candidates_path,
        amt_id,
        &format!(
            "graph-ripple: {} {} (container {})",
            evt.event, evt.container_type, evt.container_id
        ),
    )?;
    wake().notify_one();
    Ok(Some(amt_id))
}

/// Append a ripple-originated candidate through the unified expansion
/// store (route: GraphRipple). Deduped there: an unhandled candidate for
/// the same AMT is not duplicated.
fn append_reexpansion_candidate(candidates_path: &str, amt_id: u64, reason: &str) -> Result<(), String> {
    let _ = crate::orchestrator::amt_candidates::append_at(
        candidates_path, amt_id, None, "GraphRipple", Some(reason));
    Ok(())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn run_amt_reexpansion_loop(
    executor: Arc<dyn PipelineExecutor>,
    store: Arc<dyn StoreAccess>,
    config: RefinementConfig,
    available_models: Vec<AvailableModel>,
    meta_fallback: ModelFallbackConfig,
) {
    if !config.enabled {
        tracing::info!("AMT re-expansion loop disabled (RefinementConfig.enabled = false)");
        return;
    }

    tracing::info!(
        interval_secs = config.interval_secs,
        "AMT re-expansion loop starting"
    );

    loop {
        // Event-driven with interval fallback: a graph-ripple candidate
        // wakes the loop instantly; the interval guarantees periodic review
        // even when nothing rippled.
        let interval = tokio::time::sleep(std::time::Duration::from_secs(config.interval_secs));
        tokio::select! {
            _ = interval => {},
            _ = wake().notified() => {
                tracing::info!("AMT re-expansion loop: woken by graph ripple");
            }
        }

        tracing::info!("AMT re-expansion loop: starting review pass");
        if let Err(e) =
            review_amt_candidates_once(&executor, &store, &available_models, &meta_fallback).await
        {
            tracing::warn!(error = %e, "AMT re-expansion loop review pass failed");
        }
        tracing::info!("AMT re-expansion loop: review pass complete");
    }
}

fn candidates_log_path() -> String {
    let data_dir =
        std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    format!("{}/amt_reexpansion_candidates.json", data_dir)
}

async fn review_amt_candidates_once(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
) -> Result<(), String> {
    let mut candidates = crate::orchestrator::amt_candidates::load_all_at(&candidates_log_path());
    if candidates.is_empty() {
        return Ok(()); // nothing recorded yet — not an error
    }

    let mut any_change = false;

    for candidate in candidates.iter_mut() {
        if candidate.get("handled").and_then(|v| v.as_bool()) == Some(true) {
            continue;
        }
        let Some(container_id) = candidate.get("container_id").and_then(|v| v.as_u64()) else {
            continue;
        };
        let attempts_before = candidate.get("attempts").and_then(|a| a.as_u64()).unwrap_or(0);

        match try_reexpand_one(
            executor,
            store,
            container_id,
            attempts_before,
            available_models,
            meta_fallback,
        )
        .await
        {
            Ok(true) => {
                tracing::info!(container_id, "AMT re-expansion: branch deepened");
                candidate["handled"] = serde_json::json!(true);
                any_change = true;
                // MERGE-BACK V2 (main/fork islands): when a fork deepens,
                // notify the living graph so subscribers know the AMT
                // evolved. The Continues relation already links fork→main;
                // the ripple event lets downstream consumers know new
                // content exists to graft.
                let route = candidate.get("route").and_then(|r| r.as_str()).unwrap_or("UnknownNode");
                crate::graph_events::emit(
                    "updated",
                    container_id,
                    0,
                    "AMTExpansion".to_string(),
                    "amt-expansion",
                    vec![format!("route:{}", route)],
                );
            }
            Ok(false) => {
                // Container gone, node no longer unverified (resolved some
                // other way since), or the model had nothing substantive to
                // add — mark handled either way so this doesn't get retried
                // forever; a real new gap will get recorded fresh next time
                // this project's AMT is rebuilt if the branch is still thin.
                candidate["handled"] = serde_json::json!(true);
                any_change = true;
            }
            Err(e) => {
                // Retry cap — same reasoning as the methodology meta-loop's
                // identical fix: without one, a candidate whose LLM call
                // keeps failing (empty/invalid response, a real observed
                // OpenRouter behavior for some content) retries forever,
                // burning a real API call every pass with no escalation.
                // Persisted on the candidate itself so the count survives a
                // restart.
                const MAX_REEXPAND_ATTEMPTS: u64 = 10;
                let attempts = attempts_before + 1;
                candidate["attempts"] = serde_json::json!(attempts);
                any_change = true;
                if attempts >= MAX_REEXPAND_ATTEMPTS {
                    tracing::warn!(
                        container_id,
                        attempts,
                        error = %e,
                        "AMT re-expansion exceeded max attempts — marking failed, not retrying further"
                    );
                    candidate["handled"] = serde_json::json!(true);
                    candidate["outcome"] = serde_json::json!("failed_after_max_attempts");
                } else {
                    tracing::warn!(container_id, attempts, error = %e, "AMT re-expansion attempt failed — will retry next pass");
                }
            }
        }
    }

    if any_change {
        crate::orchestrator::amt_candidates::save_all_at(&candidates_log_path(), &candidates);
    }

    Ok(())
}

/// Returns Ok(true) if a branch was genuinely deepened, Ok(false) if there
/// was nothing to do (container missing, already resolved, or the model
/// declined to add anything substantive). Returns Err (retriable, see the
/// caller's attempt cap) for a real failure — including a response that
/// was empty or not valid JSON, which previously fell through to the
/// `{"details": []}` default and got silently treated as "the model
/// deliberately said no more detail needed," permanently marking the
/// candidate handled even though nothing had actually been checked.
async fn try_reexpand_one(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    container_id: u64,
    attempts_before: u64,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
) -> Result<bool, String> {
    let Some(container) = store.get_container(container_id).await? else {
        tracing::warn!(container_id, "AMT expansion: container missing (deleted since recording)");
        return Ok(false);
    };

    let object_store_path = container
        .get("local_state")
        .and_then(|ls| ls.get("storage"))
        .and_then(|s| s.get("object_store_path"))
        .and_then(|p| p.as_str())
        .ok_or_else(|| "container has no object_store_path".to_string())?;

    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    // Absolute object_store_path is used as-is; only relative paths join
    // the data dir (a bare format! garbage-joins absolute paths — found by
    // the ripple e2e test).
    let full_path = if std::path::Path::new(&object_store_path).is_absolute() {
        object_store_path.to_string()
    } else {
        format!("{}/{}", data_dir, object_store_path)
    };
    let raw = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
    let mut amt: AMTNode = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    let Some(target_node) = find_unverified_node(&amt) else {
        tracing::info!(container_id, "AMT expansion: branch already resolved since recording");
        return Ok(false); // already resolved since the candidate was recorded
    };
    let target_content = target_node.content.clone();
    let target_methodology_ids = target_node.methodology_ids.clone();
    let target_relationship_ids: Vec<u64> =
        target_node.relationships.iter().map(|r| r.target_id).collect();

    // GUIDANCE — real, not fabricated: this call previously deepened every
    // branch "naked" (root_content + target_content only), never reading the
    // real methodology_ids/relationships already populated on the node
    // being deepened, even though the exact mechanism to use them
    // (load_methodology_rules_text) already exists and is correctly wired
    // into regular orchestration steps (stages.rs). Confirmed live
    // 2026-09-15 as a real, previously-uncaptured gap (task 62) — this is
    // the fix: pull real decision_rules/heuristics text for every
    // methodology this node is tagged with, and pull in related-branch
    // content via its relationships, the same two signals stages.rs already
    // uses for a real orchestration step. Silently skips ids that don't
    // resolve to real content rather than failing the whole re-expansion —
    // guidance is additive, its absence isn't an error.
    let mut guidance_parts: Vec<String> = Vec::new();
    for &method_id in &target_methodology_ids {
        if let Ok(Some(container)) = store.get_container(method_id).await {
            if let Some(rules) = PromptOrchestrator::load_methodology_rules_text(&container) {
                guidance_parts.push(rules);
            }
        }
    }
    let methodology_guidance = if guidance_parts.is_empty() {
        None
    } else {
        Some(guidance_parts.join("\n"))
    };

    let mut related_parts: Vec<String> = Vec::new();
    for rel_id in &target_relationship_ids {
        if let Some(related) = find_node_by_id(&amt, *rel_id) {
            if related.content != target_content {
                related_parts.push(related.content.clone());
            }
        }
    }
    let related_branches = if related_parts.is_empty() {
        None
    } else {
        Some(related_parts.join("\n- "))
    };

    let root_content = amt.content.clone();
    let guidance_block = match &methodology_guidance {
        Some(text) => format!("\n\nRELEVANT GUIDANCE (apply these when deepening — do not restate them, use them):\n{}", text),
        None => String::new(),
    };
    let related_block = match &related_branches {
        Some(text) => format!("\n\nRELATED BRANCHES ALREADY IN THIS TREE (for context — stay consistent with these, don't duplicate them):\n- {}", text),
        None => String::new(),
    };
    let prompt = format!(
        r#"You are deepening one specific branch of an existing analysis tree.

OVERALL REQUEST: {}

BRANCH TO DEEPEN (currently has no concrete supporting detail): {}{}{}

Provide 1-3 concrete, specific details, requirements, or sub-points that would genuinely
strengthen this branch — not a restatement of the branch itself, not generic filler.
If you cannot add anything genuinely specific and useful, say so.

Return ONLY valid JSON:
{{
    "details": ["specific detail 1", "specific detail 2"]
}}
If nothing substantive can be added, return: {{"details": []}}"#,
        root_content, target_content, guidance_block, related_block
    );

    let mut input = serde_json::json!({
        "prompt": prompt,
        "max_tokens": 400,
        "temperature": 0.3,
        "system_context": "Deepen one analysis branch with concrete detail. Return only valid JSON."
    });

    // Model fallback escalation on retry — same reasoning and shape as the
    // methodology meta-loop's identical fix: attempt 1 (attempts_before ==
    // 0) uses the primary/default model, attempt 2 onward escalates
    // through the real meta_fallback chain rather than repeating a primary
    // that keeps failing. CYCLES through fallback candidates rather than
    // clamping to the last one — see meta_loop.rs's matching fix for the
    // real bug this corrects (clamping got stuck retrying a single broken
    // candidate forever instead of rotating through all of them).
    let fallback_candidates: Vec<&AvailableModel> = meta_fallback
        .order
        .iter()
        .filter_map(|id| available_models.iter().find(|m| &m.identifier == id))
        .filter(|m| !meta_fallback.free_only || m.is_free)
        .collect();
    if attempts_before > 0 && !fallback_candidates.is_empty() {
        let idx = ((attempts_before - 1) as usize) % fallback_candidates.len();
        let profile = fallback_candidates[idx];
        tracing::info!(container_id, fallback_model = %profile.identifier, "AMT re-expansion: escalating to fallback model");
        let override_cfg = ModelConfigOverride {
            model_type: Some(profile.model_type.clone()),
            model_identifier: Some(profile.identifier.clone()),
            max_tokens: None,
            temperature: None,
            context_length: Some(profile.context_length as u32),
            api_endpoint: profile.api_endpoint.clone(),
            api_key_env: profile.api_key_env.clone(),
            api_key: profile.api_key.clone(),
            wire_protocol: profile.wire_protocol.clone(),
            bitnet_cli_path: profile.bitnet_cli_path.clone(),
            local_model_path: profile.local_model_path.clone(),
        };
        if let Ok(v) = serde_json::to_value(&override_cfg) {
            input["model_override_config"] = v;
        }
    }

    let result = executor.execute(PROMPT_PIPELINE_ID, input).await?;
    let response = result.get("response").and_then(|r| r.as_str()).unwrap_or("");
    if response.trim().is_empty() {
        return Err("prompt pipeline returned an empty response".to_string());
    }
    let json_str = extract_json_object(response);
    let parsed: serde_json::Value = serde_json::from_str(json_str.trim())
        .map_err(|e| format!("draft response wasn't valid JSON: {}", e))?;

    let details: Vec<String> = parsed
        .get("details")
        .and_then(|d| d.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    if details.is_empty() {
        return Ok(false);
    }

    if !add_children_to_unverified(&mut amt, &target_content, &details) {
        tracing::warn!(container_id, target = %target_content, "AMT expansion: target branch not found (tree changed since detection)");
        return Ok(false);
    }

    // Overwrite the content file in place — the container's own record
    // (created_at/metadata) is left untouched; content-file-backed
    // containers in this codebase are read by following object_store_path,
    // not by re-fetching the container object itself (see
    // load_methodology_rules_text, persist_amt_container).
    let updated = serde_json::to_string_pretty(&amt).map_err(|e| e.to_string())?;
    std::fs::write(&full_path, updated).map_err(|e| e.to_string())?;
    tracing::info!(container_id, path = %full_path, "AMT expansion: branch deepened and persisted");

    Ok(true)
}

fn find_unverified_node(node: &AMTNode) -> Option<&AMTNode> {
    if !node.verified {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_unverified_node(child) {
            return Some(found);
        }
    }
    None
}

fn find_node_by_id(node: &AMTNode, target_id: u64) -> Option<&AMTNode> {
    if node.id == target_id {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node_by_id(child, target_id) {
            return Some(found);
        }
    }
    None
}

/// Find the node matching `target_content` and append new leaf children for
/// each detail — new children are honestly marked unverified too (this is
/// still model inference without new source-chunk evidence, not a claim of
/// provenance), but the branch is now genuinely deeper. Returns false if
/// the target node couldn't be found (tree changed since detection).
fn add_children_to_unverified(node: &mut AMTNode, target_content: &str, details: &[String]) -> bool {
    if node.content == target_content && !node.verified {
        let mut next_id = node
            .children
            .iter()
            .map(|c| c.id)
            .max()
            .unwrap_or(node.id)
            + 1;
        for detail in details {
            node.children.push(AMTNode {
                id: next_id,
                node_type: crate::orchestrator::AMTNodeType::Leaf,
                content: detail.clone(),
                source_chunk_indices: vec![],
                children: vec![],
                relationships: vec![],
                methodology_ids: vec![],
                metadata: Default::default(),
                depth: node.depth + 1,
                verified: false,
                confidence: 0.0,
            });
            next_id += 1;
        }
        return true;
    }
    for child in &mut node.children {
        if add_children_to_unverified(child, target_content, details) {
            return true;
        }
    }
    false
}

fn extract_json_object(response: &str) -> &str {
    let start = response.find('{');
    let end = response.rfind('}');
    match (start, end) {
        (Some(s), Some(e)) if e >= s => &response[s..=e],
        _ => "{}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_events::GraphEvent;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_path(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "amt_loop_{}_{}_{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("amt_reexpansion_candidates.json")
            .to_string_lossy()
            .into()
    }

    fn scoped_event(proj: Option<u64>, file: bool, event: &'static str) -> GraphEvent {
        let mut keywords = Vec::new();
        if let Some(p) = proj {
            keywords.push(format!("proj:{p}"));
        }
        if file {
            keywords.push("file:src/lib.rs".into());
        }
        GraphEvent {
            event,
            container_id: 30_001,
            parent_id: proj.unwrap_or(0),
            container_type: "FileGraph".into(),
            source: "test".into(),
            scope_keywords: keywords,
            timestamp: 0,
        }
    }

    // T-S1: a project-scoped graph write converts into a re-expansion
    // candidate for the project's AMT container.
    #[tokio::test]
    async fn ripple_event_appends_candidate_for_project_amt() {
        let path = temp_path("s1");
        let store = MockStore::with_project_amt(7, 500); // project 7, AMT child 500

        let got = process_graph_event(
            &store,
            &path,
            &scoped_event(Some(7), false, "updated"),
        )
        .await
        .unwrap();

        assert_eq!(got, Some(500), "candidate recorded for the project's AMT");
        let candidates: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0]["container_id"], 500);
        assert_eq!(candidates[0]["handled"], false);
        assert_eq!(candidates[0]["source"], "graph-ripple: updated FileGraph (container 30001)");
    }

    // T-S2: repeated events don't duplicate unhandled candidates.
    #[tokio::test]
    async fn ripple_events_dedupe_per_amt() {
        let path = temp_path("s2");
        let store = MockStore::with_project_amt(7, 500);
        for _ in 0..3 {
            process_graph_event(&store, &path, &scoped_event(Some(7), false, "created"))
                .await
                .unwrap();
        }
        let candidates: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(candidates.len(), 1, "one unhandled candidate per AMT");
    }

    // T-S3: scope filter — global-only coordination chatter doesn't drive AMTs.
    #[tokio::test]
    async fn global_only_event_creates_no_candidate() {
        let path = temp_path("s3");
        let store = MockStore::with_project_amt(7, 500);
        let evt = GraphEvent {
            event: "created",
            container_id: 1,
            parent_id: 8,
            container_type: "CoordinationEvent".into(),
            source: "test".into(),
            scope_keywords: vec!["scope:global".into()],
            timestamp: 0,
        };
        let got = process_graph_event(&store, &path, &evt).await.unwrap();
        assert_eq!(got, None);
        assert!(!std::path::Path::new(&path).exists(), "no candidates file written");
    }

    // T-S4: deletion events are ignored (deletions don't deepen branches).
    #[tokio::test]
    async fn delete_events_ignored() {
        let path = temp_path("s4");
        let store = MockStore::with_project_amt(7, 500);
        let got = process_graph_event(
            &store,
            &path,
            &scoped_event(Some(7), false, "deleted"),
        )
        .await
        .unwrap();
        assert_eq!(got, None);
    }

    // T-S5: full consumption — review pass deepens the AMT file through the
    // mock executor and marks the candidate handled.
    #[tokio::test]
    async fn review_pass_consumes_ripple_candidate_end_to_end() {
        let path = temp_path("s5");
        // review_amt_candidates_once resolves its candidates file from
        // OZONE_ZSEI_DATA_DIR — point it at this test's dir.
        let data_dir = std::path::Path::new(&path)
            .parent()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        std::env::set_var("OZONE_ZSEI_DATA_DIR", &data_dir);

        let store = MockStore::with_project_amt(7, 500);
        process_graph_event(&store, &path, &scoped_event(Some(7), false, "updated"))
            .await
            .unwrap();

        let executor: std::sync::Arc<dyn PipelineExecutor> =
            std::sync::Arc::new(MockExecutor::success(r#"{"details": ["concrete sub-detail"]}"#));
        let models = vec![];
        let fallback = ModelFallbackConfig::default();
        let (review_store, review_amt_file) = store_for_review();
        review_amt_candidates_once(&executor, &review_store, &models, &fallback)
            .await
            .unwrap();

        let candidates: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(candidates[0]["handled"], true);
        // The AMT file was genuinely deepened:
        // The review pass used the second MockStore instance (store_for_review).
        let amt: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&review_amt_file).unwrap(),
        )
        .unwrap();
        // The detail lands UNDER the thin branch (depth 2) — search the
        // whole tree, not just the root's direct children.
        fn count_content(node: &serde_json::Value, needle: &str) -> usize {
            let mut hits = if node["content"] == needle { 1 } else { 0 };
            for c in node["children"].as_array().unwrap_or(&vec![]) {
                hits += count_content(c, needle);
            }
            hits
        }
        assert_eq!(
            count_content(&amt, "concrete sub-detail"),
            1,
            "new unverified leaf added from ripple candidate"
        );
    }

    // T-A3: retry/fallback escalation genuinely CYCLES through every
    // candidate instead of clamping to the last one once attempts exceed
    // the candidate count — the exact bug class found and fixed earlier
    // this session in this same function (and meta_loop.rs's identical
    // pattern): `.min(len - 1)` got stuck on the final candidate forever;
    // `% len` (the live fix, already in try_reexpand_one above) wraps back
    // around. Calls try_reexpand_one directly with successive attempts_before
    // values and records which model_override_config was actually sent on
    // each call — attempt 0 uses no override (primary model), attempts 1/2
    // use fallback-a/fallback-b in order, and attempt 3 (with only 2
    // fallback candidates) must cycle back to fallback-a, not stay clamped
    // on fallback-b.
    #[tokio::test]
    async fn reexpansion_retry_cycles_through_fallback_candidates_not_clamps() {
        let store: Arc<dyn StoreAccess> = Arc::new(MockStore::with_project_amt(7, 600));
        let recorder = Arc::new(std::sync::Mutex::new(Vec::<Option<String>>::new()));
        let executor: Arc<dyn PipelineExecutor> =
            Arc::new(RecordingExecutor { recorder: recorder.clone() });
        let models = vec![
            model_profile("fallback-a"),
            model_profile("fallback-b"),
        ];
        let fallback = ModelFallbackConfig {
            order: vec!["fallback-a".to_string(), "fallback-b".to_string()],
            free_only: false,
        };

        for attempts_before in 0..4u64 {
            try_reexpand_one(&executor, &store, 600, attempts_before, &models, &fallback)
                .await
                .unwrap();
        }

        let used = recorder.lock().unwrap().clone();
        assert_eq!(
            used,
            vec![
                None,                              // attempt 0: primary model, no override
                Some("fallback-a".to_string()),     // attempt 1: first fallback
                Some("fallback-b".to_string()),     // attempt 2: second fallback
                Some("fallback-a".to_string()),     // attempt 3: CYCLES back, doesn't clamp on b
            ],
            "retry escalation must cycle through every fallback candidate, not clamp to the last one"
        );
    }

    fn model_profile(identifier: &str) -> AvailableModel {
        serde_json::from_value(serde_json::json!({
            "name": identifier,
            "model_type": "api",
            "identifier": identifier,
        }))
        .unwrap()
    }

    struct RecordingExecutor {
        recorder: Arc<std::sync::Mutex<Vec<Option<String>>>>,
    }

    #[async_trait::async_trait]
    impl PipelineExecutor for RecordingExecutor {
        async fn execute(
            &self,
            _pipeline_id: u64,
            input: serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            let model_id = input
                .get("model_override_config")
                .and_then(|c| c.get("model_identifier"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            self.recorder.lock().unwrap().push(model_id);
            Ok(serde_json::json!({ "response": r#"{"details": ["x"]}"# }))
        }
        async fn pipeline_exists(&self, _pipeline_id: u64) -> bool {
            true
        }
    }

    // T-62: the re-expansion prompt is genuinely guided by the target
    // node's real methodology_ids + relationships, not built "naked" from
    // just root_content + target_content (task 62 — confirmed live
    // 2026-09-15 that this call never read either field despite both
    // already existing on AMTNode and the guidance mechanism
    // (load_methodology_rules_text) already working correctly elsewhere).
    // A purpose-built store is used here rather than the shared MockStore —
    // that one hardcodes single project/AMT ids and can't represent a
    // methodology container at a third id, so reusing it would produce a
    // test that can't actually exercise this path.
    #[tokio::test]
    async fn reexpansion_prompt_includes_methodology_guidance_and_related_branches() {
        let dir = std::env::temp_dir().join(format!(
            "amt_loop_t62_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let amt_file = dir.join("amt.json").to_string_lossy().into_owned();
        let methodology_file = dir.join("methodology.json").to_string_lossy().into_owned();

        std::fs::write(
            &methodology_file,
            serde_json::to_string_pretty(&serde_json::json!({
                "decision_rules": [{"condition": "handling PII", "action": "encrypt at rest"}],
                "heuristics": []
            }))
            .unwrap(),
        )
        .unwrap();

        let amt = serde_json::json!({
            "content": "root", "node_type": "Root", "verified": true, "confidence": 1.0,
            "id": 0, "depth": 0, "source_chunk_indices": [], "relationships": [],
            "methodology_ids": [], "metadata": {},
            "children": [
                {
                    "content": "a related, already-verified branch about data retention",
                    "node_type": "Leaf", "verified": true, "confidence": 1.0, "id": 2, "depth": 1,
                    "children": [], "source_chunk_indices": [], "relationships": [],
                    "methodology_ids": [], "metadata": {}
                },
                {
                    "content": "thin branch needing detail",
                    "node_type": "Leaf", "verified": false, "confidence": 0.0, "id": 1, "depth": 1,
                    "children": [], "source_chunk_indices": [],
                    "relationships": [{"target_id": 2, "relation_type": "RelatesTo", "confidence": 0.8}],
                    "methodology_ids": [900], "metadata": {}
                }
            ]
        });
        std::fs::write(&amt_file, serde_json::to_string_pretty(&amt).unwrap()).unwrap();

        struct T62Store {
            amt_file: String,
            methodology_file: String,
        }
        #[async_trait::async_trait]
        impl StoreAccess for T62Store {
            async fn query(&self, _q: serde_json::Value) -> Result<serde_json::Value, String> {
                Ok(serde_json::json!({}))
            }
            async fn traverse(&self, _r: serde_json::Value) -> Result<serde_json::Value, String> {
                Ok(serde_json::json!({}))
            }
            async fn create_container(&self, _p: u64, _c: serde_json::Value) -> Result<u64, String> {
                Ok(999)
            }
            async fn update_container(&self, _id: u64, _u: serde_json::Value) -> Result<(), String> {
                Ok(())
            }
            async fn get_container(&self, id: u64) -> Result<Option<serde_json::Value>, String> {
                if id == 500 {
                    return Ok(Some(serde_json::json!({
                        "local_state": {
                            "metadata": { "container_type": "Derived" },
                            "storage": { "object_store_path": self.amt_file }
                        }
                    })));
                }
                if id == 900 {
                    return Ok(Some(serde_json::json!({
                        "local_state": { "storage": { "object_store_path": self.methodology_file } }
                    })));
                }
                Ok(None)
            }
            async fn search_by_keywords(&self, _k: &[String], _t: Option<&str>) -> Result<Vec<u64>, String> {
                Ok(Vec::new())
            }
            async fn get_categories(&self, _m: &str) -> Result<Vec<u64>, String> {
                Ok(Vec::new())
            }
        }

        let store: Arc<dyn StoreAccess> =
            Arc::new(T62Store { amt_file: amt_file.clone(), methodology_file: methodology_file.clone() });
        let (executor_impl, captured) = MockExecutor::success_capturing(r#"{"details": []}"#);
        let executor: Arc<dyn PipelineExecutor> = Arc::new(executor_impl);

        let models: Vec<AvailableModel> = vec![];
        let fallback = ModelFallbackConfig::default();
        try_reexpand_one(&executor, &store, 500, 0, &models, &fallback).await.unwrap();

        let prompt = captured.lock().unwrap().clone().expect("executor should have been called with a prompt");
        assert!(
            prompt.contains("encrypt at rest"),
            "methodology guidance text missing from prompt:\n{}",
            prompt
        );
        assert!(
            prompt.contains("a related, already-verified branch about data retention"),
            "related-branch content missing from prompt:\n{}",
            prompt
        );
    }

    // ── mocks ────────────────────────────────────────────────────────────

    struct MockStore {
        project_id: u64,
        amt_id: u64,
        calls: AtomicU32,
        amt_file: String,
    }

    impl MockStore {
        fn with_project_amt(project_id: u64, amt_id: u64) -> Self {
            // Unique file per instance — parallel tests must not share the
            // AMT content file (same isolation rule as the candidates path).
            static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let amt_file = format!(
                "{}_{}.json",
                MockStore::amt_file_base(),
                SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            );
            // Materialize the AMT content file the loop reads.
            let amt = serde_json::json!({
                "content": "project root analysis",
                "node_type": "Root",
                "verified": true,
                "confidence": 1.0,
                "id": 0,
                "depth": 0,
                "source_chunk_indices": [],
                "relationships": [],
                "methodology_ids": [],
                "metadata": {},
                "children": [{
                    "content": "thin branch",
                    "node_type": "Leaf",
                    "verified": false,
                    "confidence": 0.0,
                    "id": 1,
                    "depth": 1,
                    "children": [],
                    "source_chunk_indices": [],
                    "relationships": [],
                    "methodology_ids": [],
                    "metadata": {},
                }]
            });
            let path = std::path::PathBuf::from(&amt_file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, serde_json::to_string_pretty(&amt).unwrap()).unwrap();
            Self { project_id, amt_id, calls: AtomicU32::new(0), amt_file }
        }

        fn amt_file_base() -> String {
            let dir = std::env::temp_dir().join(format!("amt_loop_files_{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            dir.join("amt").to_string_lossy().into()
        }
    }

    impl MockStore {
        fn project_json(&self) -> serde_json::Value {
            serde_json::json!({
                "global_state": { "child_ids": [self.amt_id], "child_count": 1 },
                "local_state": { "metadata": { "container_type": "Project", "name": "p" } }
            })
        }
        fn amt_json(&self) -> serde_json::Value {
            serde_json::json!({
                "local_state": {
                    "metadata": { "container_type": "Derived" },
                    "storage": { "object_store_path": self.amt_file }
                }
            })
        }
    }

    #[async_trait::async_trait]
    impl StoreAccess for MockStore {
        async fn query(&self, _q: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({}))
        }
        async fn traverse(&self, _r: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({}))
        }
        async fn create_container(
            &self,
            _parent: u64,
            _c: serde_json::Value,
        ) -> Result<u64, String> {
            Ok(999)
        }
        async fn update_container(
            &self,
            _id: u64,
            _u: serde_json::Value,
        ) -> Result<(), String> {
            Ok(())
        }
        async fn get_container(
            &self,
            container_id: u64,
        ) -> Result<Option<serde_json::Value>, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if container_id == self.project_id {
                return Ok(Some(self.project_json()));
            }
            if container_id == self.amt_id {
                return Ok(Some(self.amt_json()));
            }
            Ok(None)
        }

        async fn search_by_keywords(
            &self,
            _keywords: &[String],
            _container_type: Option<&str>,
        ) -> Result<Vec<u64>, String> {
            Ok(Vec::new())
        }

        async fn get_categories(&self, _modality: &str) -> Result<Vec<u64>, String> {
            Ok(Vec::new())
        }
    }

    struct MockExecutor {
        response: String,
        captured_prompt: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    }

    impl MockExecutor {
        fn success(response: &str) -> Self {
            Self { response: response.to_string(), captured_prompt: Default::default() }
        }

        /// Same as `success`, but also records the real prompt text the
        /// caller sent — for tests asserting on prompt CONTENT (e.g. that
        /// real methodology guidance actually reached it), not just that a
        /// response came back.
        fn success_capturing(
            response: &str,
        ) -> (Self, std::sync::Arc<std::sync::Mutex<Option<String>>>) {
            let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
            (
                Self { response: response.to_string(), captured_prompt: captured.clone() },
                captured,
            )
        }
    }

    #[async_trait::async_trait]
    impl PipelineExecutor for MockExecutor {
        async fn execute(
            &self,
            _pipeline_id: u64,
            _input: serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            if let Some(p) = _input.get("prompt").and_then(|v| v.as_str()) {
                *self.captured_prompt.lock().unwrap() = Some(p.to_string());
            }
            Ok(serde_json::json!({ "response": self.response }))
        }
        async fn pipeline_exists(&self, _pipeline_id: u64) -> bool {
            true
        }
    }

    fn store_for_review() -> (Arc<dyn StoreAccess>, String) {
        // The review pass needs the AMT container via get_container(amt_id)
        // — same mock, fresh counters. Returns its AMT file path so the
        // test asserts the file the review actually wrote.
        let store = std::sync::Arc::new(MockStore::with_project_amt(7, 500));
        let file = store.amt_file.clone();
        (store, file)
    }
}

/// Bridge the graph ripple to the network layer's hook system.
/// Spawns a subscriber that converts GraphEvents into
/// `on_container_created` / `on_blueprint_created` /
/// `on_methodology_created` calls, finally connecting the network
/// transport to the graph writes (it was registered but never fed).
pub fn spawn_network_hook_bridge(
    hub: &'static crate::graph_events::GraphEventHub,
    network: std::sync::Arc<crate::network::NetworkManager>,
) {
    let mut rx = hub.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(evt) => {
                    if evt.event == "created" {
                        let data = serde_json::to_vec(&serde_json::json!({
                            "container_id": evt.container_id,
                            "parent_id": evt.parent_id,
                            "container_type": evt.container_type,
                            "source": evt.source,
                            "scope_keywords": evt.scope_keywords,
                        }))
                        .unwrap_or_default();
                        match evt.container_type.as_str() {
                            "Methodology" => {
                                let _ = network.on_methodology_created(evt.container_id, data).await;
                            }
                            "Blueprint" => {
                                let _ = network.on_blueprint_created(evt.container_id, data).await;
                            }
                            _ => {
                                let _ = network.on_container_created(evt.container_id, data).await;
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(lagged = n, "Network hook bridge: missed graph events");
                }
                Err(_) => break,
            }
        }
    });
}
