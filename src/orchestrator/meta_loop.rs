//! Real methodology meta-loop: periodically reviews methodology gaps
//! recorded live during orchestration (see amt.rs's
//! PromptOrchestrator::record_methodology_gap, called from
//! enrich_with_zsei_knowledge whenever a real request's keyword signal
//! matched zero methodologies) and, for a gap not already covered by
//! something created since, drafts and persists a real new methodology.
//!
//! Lives here (not on TaskManager, where the pre-existing but never-started
//! `start_refinement_daemon` lives) because drafting a genuine methodology
//! requires a real LLM call, and TaskManager only has ZSEI access — no
//! pipeline-executing capability. This loop takes the same PipelineExecutor/
//! StoreAccess contracts the rest of the orchestrator already uses, and
//! respects the same RefinementConfig (enabled/interval_secs) the dormant
//! daemon does rather than inventing a second config surface.
//!
//! Cross-referencing against duplication is real but intentionally simple:
//! before drafting anything, it re-runs the same relevance-filtered
//! search_by_keywords (zsei/query.rs's find_methodologies_by_keywords) the
//! live request path uses — if that now returns a match (created since the
//! gap was recorded, by this loop or anything else), the gap is considered
//! covered and skipped, not duplicated. A full pairwise similarity pass
//! across the whole methodology store is a larger, separate undertaking,
//! not built here.

use crate::config::{AvailableModel, ModelFallbackConfig};
use crate::orchestrator::{ModelConfigOverride, PipelineExecutor, PromptOrchestrator, StoreAccess};
use crate::task::RefinementConfig;
use std::sync::Arc;

const PROMPT_PIPELINE_ID: u64 = 9;
const METHODOLOGY_CREATE_PIPELINE_ID: u64 = 12;

/// Start the meta-loop. Returns immediately if disabled; otherwise runs
/// forever on `config.interval_secs`, matching start_refinement_daemon's own
/// cadence convention (that daemon sleeps in 60s increments and only acts
/// once the full interval has elapsed — this loop sleeps the full interval
/// directly since it has no other per-minute work to interleave with).
pub async fn run_methodology_meta_loop(
    executor: Arc<dyn PipelineExecutor>,
    store: Arc<dyn StoreAccess>,
    config: RefinementConfig,
    available_models: Vec<AvailableModel>,
    meta_fallback: ModelFallbackConfig,
) {
    if !config.enabled {
        tracing::info!("Methodology meta-loop disabled (RefinementConfig.enabled = false)");
        return;
    }

    tracing::info!(
        interval_secs = config.interval_secs,
        "Methodology meta-loop starting"
    );

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(config.interval_secs)).await;

        tracing::info!("Methodology meta-loop: starting review pass");
        if let Err(e) =
            review_methodology_gaps_once(&executor, &store, &available_models, &meta_fallback).await
        {
            tracing::warn!(error = %e, "Methodology meta-loop review pass failed");
        }
        tracing::info!("Methodology meta-loop: review pass complete");
    }
}

fn gaps_log_path() -> String {
    let data_dir =
        std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    format!("{}/methodology_gaps.json", data_dir)
}

async fn review_methodology_gaps_once(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
) -> Result<(), String> {
    let path = gaps_log_path();

    let mut gaps: Vec<serde_json::Value> = match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("parse {}: {}", path, e))?,
        Err(_) => return Ok(()), // nothing recorded yet — not an error
    };

    let mut any_change = false;

    for gap in gaps.iter_mut() {
        if gap.get("handled").and_then(|h| h.as_bool()).unwrap_or(false) {
            continue;
        }

        let keywords: Vec<String> = gap
            .get("keywords")
            .and_then(|k| serde_json::from_value(k.clone()).ok())
            .unwrap_or_default();
        let topics: Vec<String> = gap
            .get("topics")
            .and_then(|t| serde_json::from_value(t.clone()).ok())
            .unwrap_or_default();

        if keywords.is_empty() {
            continue;
        }

        // Cross-reference for duplication: has this gap already been
        // covered since it was recorded (by this loop on a prior cycle, or
        // by anything else)?
        let existing = store
            .search_by_keywords(&keywords, Some("Methodology"))
            .await
            .unwrap_or_default();
        if !existing.is_empty() {
            tracing::info!(
                ?keywords,
                covered_by = ?existing,
                "Methodology gap now covered by an existing methodology — marking handled"
            );
            gap["handled"] = serde_json::json!(true);
            any_change = true;
            continue;
        }

        // Retry cap — confirmed live this session that without one, a gap
        // whose draft call keeps coming back empty/invalid (a real,
        // observed OpenRouter behavior for some content, not a bug in this
        // loop) gets retried forever: every interval, indefinitely, burning
        // a real API call each time with no escalation and no visible
        // signal anything is actually wrong. Track attempts on the gap
        // record itself (persisted, so this survives a restart — the
        // count is never lost/reset just because the process was). After
        // MAX_DRAFT_ATTEMPTS, stop retrying and mark it failed (not
        // silently "handled" as if resolved) so it's visibly inspectable
        // rather than either retried forever or quietly dropped.
        const MAX_DRAFT_ATTEMPTS: u64 = 10;
        let attempts = gap.get("attempts").and_then(|a| a.as_u64()).unwrap_or(0);
        if attempts >= MAX_DRAFT_ATTEMPTS {
            tracing::warn!(
                ?keywords,
                attempts,
                "Methodology meta-loop: gap exceeded max draft attempts — marking failed, not retrying further"
            );
            gap["handled"] = serde_json::json!(true);
            gap["outcome"] = serde_json::json!("failed_after_max_attempts");
            any_change = true;
            continue;
        }
        gap["attempts"] = serde_json::json!(attempts + 1);
        any_change = true;

        // Model fallback escalation on retry — the first-ever attempt
        // (attempts == 0) uses the primary/default model as before;
        // attempt 2 onward escalates through the real meta_fallback chain
        // (config.toml's [models.meta_fallback] — this loop is genuinely
        // detached background work, the exact case that config was always
        // reserved for) rather than blindly repeating a primary that keeps
        // returning empty/invalid responses. CYCLES through the fallback
        // candidates (never back to the primary, which has already failed
        // at least once by then) rather than clamping to the last one —
        // confirmed live 2026-09-15: with candidates [bitnet-i2_s,
        // openrouter/free], clamping meant every attempt from #4 onward
        // retried the SAME broken openrouter/free (daily quota exhausted)
        // forever, never giving bitnet-i2_s — a real, working fallback —
        // another turn. That wasted 6 of the 10-attempt retry budget on a
        // guaranteed-fail target for one real gap. Cycling gives every
        // fallback a fair rotation on every retry instead.
        let fallback_candidates: Vec<&AvailableModel> = meta_fallback
            .order
            .iter()
            .filter_map(|id| available_models.iter().find(|m| &m.identifier == id))
            .filter(|m| !meta_fallback.free_only || m.is_free)
            .collect();
        let escalated_model = if attempts > 0 && !fallback_candidates.is_empty() {
            let idx = ((attempts - 1) as usize) % fallback_candidates.len();
            Some(fallback_candidates[idx])
        } else {
            None
        };

        tracing::info!(
            ?keywords,
            attempt = attempts + 1,
            fallback_model = ?escalated_model.map(|m| m.identifier.as_str()),
            "Methodology meta-loop: drafting a new methodology for a real gap"
        );

        let draft_prompt = format!(
            r#"A real user request's keyword signal repeatedly matched no existing methodology:

KEYWORDS: {}
TOPICS: {}

If — and only if — this is a genuinely coherent, real, methodology-worthy
pattern (specific enough to give concrete, actionable guidance, not vague or
overly broad), draft ONE real methodology for it. Every decision rule must be
a concrete "if X then do Y" a person or system could actually follow — not a
restatement of the keywords. Return ONLY valid JSON:
{{
    "name": "short methodology name",
    "description": "one sentence description",
    "keywords": ["...", "..."],
    "topics": ["...", "..."],
    "principles": [{{"name": "...", "description": "...", "priority": 1}}],
    "heuristics": [{{"condition": "...", "action": "...", "confidence": 0.8}}],
    "decision_rules": [{{"name": "...", "condition": "...", "outcome": "..."}}]
}}
If this keyword set is too vague, too broad, or not actually a coherent
methodology-worthy pattern, return exactly: {{"skip": true}}"#,
            keywords.join(", "),
            topics.join(", ")
        );

        let mut draft_input = serde_json::json!({
            "prompt": draft_prompt,
            "max_tokens": 800,
            "temperature": 0.3,
            "system_context": "Draft real, practical methodologies only when the pattern genuinely warrants one. Return only valid JSON."
        });
        if let Some(profile) = escalated_model {
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
                draft_input["model_override_config"] = v;
            }
        }

        let result = match executor.execute(PROMPT_PIPELINE_ID, draft_input).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, ?keywords, "Methodology meta-loop: draft LLM call failed, will retry next cycle");
                continue;
            }
        };

        let response_text = result.get("response").and_then(|r| r.as_str()).unwrap_or("");
        let json_str = extract_json_object(response_text);
        let parsed: serde_json::Value = match serde_json::from_str(json_str.trim()) {
            Ok(v) => v,
            Err(_) => {
                tracing::warn!(raw = %response_text, ?keywords, "Methodology meta-loop: draft response wasn't valid JSON, will retry next cycle");
                continue;
            }
        };

        if parsed.get("skip").and_then(|s| s.as_bool()).unwrap_or(false) {
            tracing::info!(?keywords, "Methodology meta-loop: model judged this not methodology-worthy — marking handled");
            gap["handled"] = serde_json::json!(true);
            any_change = true;
            continue;
        }

        // Require at least one real decision_rule or heuristic before
        // persisting — a "methodology" with only principles and no
        // actionable rule is exactly the low-content shell this whole
        // effort exists to avoid creating more of.
        let has_decision_rules = parsed
            .get("decision_rules")
            .and_then(|d| d.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        let has_heuristics = parsed
            .get("heuristics")
            .and_then(|h| h.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if !has_decision_rules && !has_heuristics {
            tracing::warn!(?keywords, "Methodology meta-loop: draft had no real decision_rules or heuristics — discarding rather than persisting a shell");
            continue;
        }

        let create_input = serde_json::json!({
            "action": "Create",
            "name": parsed.get("name").and_then(|n| n.as_str()).unwrap_or("Untitled Methodology"),
            "description": parsed.get("description").and_then(|d| d.as_str()).unwrap_or(""),
            "category_id": 0,
            "principles": parsed.get("principles").cloned().unwrap_or(serde_json::json!([])),
            "heuristics": parsed.get("heuristics").cloned().unwrap_or(serde_json::json!([])),
            "decision_rules": parsed.get("decision_rules").cloned().unwrap_or(serde_json::json!([])),
            "keywords": parsed.get("keywords").cloned().unwrap_or(serde_json::json!(keywords)),
            "topics": parsed.get("topics").cloned().unwrap_or(serde_json::json!(topics)),
        });

        match executor
            .execute(METHODOLOGY_CREATE_PIPELINE_ID, create_input)
            .await
        {
            Ok(create_result) => {
                let new_id = create_result.get("methodology_id").and_then(|m| m.as_u64());
                tracing::info!(
                    methodology_id = ?new_id,
                    ?keywords,
                    "Methodology meta-loop: created a new methodology for a real gap"
                );
                gap["handled"] = serde_json::json!(true);
                any_change = true;
            }
            Err(e) => {
                tracing::warn!(error = %e, ?keywords, "Methodology meta-loop: create call failed, will retry next cycle");
            }
        }
    }

    if any_change {
        let json = serde_json::to_string_pretty(&gaps).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| format!("write {}: {}", path, e))?;
    }

    Ok(())
}

fn extract_json_object(s: &str) -> &str {
    let trimmed = s.trim();
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return &trimmed[start..=end];
            }
        }
    }
    trimmed
}
