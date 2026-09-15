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
use crate::orchestrator::{AMTNode, ModelConfigOverride, PipelineExecutor, StoreAccess};
use crate::task::RefinementConfig;
use std::sync::Arc;

const PROMPT_PIPELINE_ID: u64 = 9;

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
        tokio::time::sleep(std::time::Duration::from_secs(config.interval_secs)).await;

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
    let path = candidates_log_path();

    let mut candidates: Vec<serde_json::Value> = match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("parse {}: {}", path, e))?,
        Err(_) => return Ok(()), // nothing recorded yet — not an error
    };

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
        if let Ok(json) = serde_json::to_string_pretty(&candidates) {
            let _ = std::fs::write(&path, json);
        }
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
        return Ok(false);
    };

    let object_store_path = container
        .get("local_state")
        .and_then(|ls| ls.get("storage"))
        .and_then(|s| s.get("object_store_path"))
        .and_then(|p| p.as_str())
        .ok_or_else(|| "container has no object_store_path".to_string())?;

    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let full_path = format!("{}/{}", data_dir, object_store_path);
    let raw = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
    let mut amt: AMTNode = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    let Some(target_content) = find_unverified_content(&amt) else {
        return Ok(false); // already resolved since the candidate was recorded
    };

    let root_content = amt.content.clone();
    let prompt = format!(
        r#"You are deepening one specific branch of an existing analysis tree.

OVERALL REQUEST: {}

BRANCH TO DEEPEN (currently has no concrete supporting detail): {}

Provide 1-3 concrete, specific details, requirements, or sub-points that would genuinely
strengthen this branch — not a restatement of the branch itself, not generic filler.
If you cannot add anything genuinely specific and useful, say so.

Return ONLY valid JSON:
{{
    "details": ["specific detail 1", "specific detail 2"]
}}
If nothing substantive can be added, return: {{"details": []}}"#,
        root_content, target_content
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
        return Ok(false);
    }

    // Overwrite the content file in place — the container's own record
    // (created_at/metadata) is left untouched; content-file-backed
    // containers in this codebase are read by following object_store_path,
    // not by re-fetching the container object itself (see
    // load_methodology_rules_text, persist_amt_container).
    let updated = serde_json::to_string_pretty(&amt).map_err(|e| e.to_string())?;
    std::fs::write(&full_path, updated).map_err(|e| e.to_string())?;

    Ok(true)
}

fn find_unverified_content(node: &AMTNode) -> Option<String> {
    if !node.verified {
        return Some(node.content.clone());
    }
    for child in &node.children {
        if let Some(found) = find_unverified_content(child) {
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
