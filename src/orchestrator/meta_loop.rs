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

use crate::orchestrator::{PipelineExecutor, StoreAccess};
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
        if let Err(e) = review_methodology_gaps_once(&executor, &store).await {
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

        tracing::info!(?keywords, "Methodology meta-loop: drafting a new methodology for a real gap");

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

        let draft_input = serde_json::json!({
            "prompt": draft_prompt,
            "max_tokens": 800,
            "temperature": 0.3,
            "system_context": "Draft real, practical methodologies only when the pattern genuinely warrants one. Return only valid JSON."
        });

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
