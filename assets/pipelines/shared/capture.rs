//! Pipeline-side zero-shot call capture (C6-minimal, 2026-09-27).
//!
//! THE GAP THIS CLOSES (found by the E7 fork's audit, operator-directed):
//! the host's `zero_shot_calls.jsonl` (S11) only exists in the main
//! binary — the separately-executed modality pipelines never routed
//! their pipeline-9 model calls through any capture at all, so a failed
//! or confetti extraction left NOTHING durable: one ephemeral stderr
//! line, an empty keyword/topic list in the persisted graph, and a
//! downstream isolation failure (e.g. math's `link_related_containers`
//! finding nothing) with no record of the upstream cause. This helper
//! gives every pipeline binary the same append-only capture discipline
//! as the host's `capture_zero_shot_call` (methodology 43: every call
//! leaves a truthful record), with NO shared state and NO host
//! dependency — plain file append, safe from concurrent processes.
//!
//! WIRING (single choke point per pipeline):
//! - text: inside `llm_execute` (ALL 13 extractor/vote sites flow
//!   through it — one capture covers every call, including each
//!   individual confetti-retry attempt).
//! - math: the `resolve_implicit_step_references` call (dead code until
//!   E7 wires it — capture lands with it, not after).
//! - code: zero model calls today (regex-only extraction) — the mod is
//!   declared dormant and lights up with the first real call site.
//!
//! DATA-DIR RESOLUTION (honest note on the store split): the host's
//! capture routes read `{general.data_dir}/model_calls/`; pipelines
//! resolve `OZONE_CAPTURE_DATA_DIR` → `OZONE_ZSEI_DATA_DIR` →
//! `"zsei_data"`. When the host runs from `target/release` (its cwd,
//! `general.data_dir = "zsei_data"` relative) and pipelines inherit
//! that cwd (they do — subprocess spawn), both resolve to the SAME
//! directory and the file lands beside the host's stores. If an
//! operator ever splits those config values, this file follows
//! `OZONE_ZSEI_DATA_DIR` and the split is visible in the path itself —
//! never silently merged or silently lost.
//!
//! FIELD DIVERGENCE vs S11 (deliberate, documented): this store records
//! `pipeline` (which binary made the call) and omits the host-only
//! context markers (`amt_container_id`/`blueprint_id`/`project_id` —
//! pipeline binaries don't hold OrchestrationState). `ts` is rfc3339
//! via the same chrono dependency every pipeline crate already carries,
//! matching the host stores' format so one reader handles all three
//! files. Attempts are captured INDIVIDUALLY (retry loops sit above the
//! choke point), so `retry_count`/`used_fallback` have no meaning here —
//! count the rows instead.

use std::io::Write;

/// Data-dir resolution: explicit capture dir wins, then the ZSEI data dir
/// the pipeline already uses for graph persistence, then the relative
/// default (matches the host's `general.data_dir = "zsei_data"` when cwds
/// align — the normal case).
pub fn capture_data_dir() -> String {
    std::env::var("OZONE_CAPTURE_DATA_DIR")
        .or_else(|_| std::env::var("OZONE_ZSEI_DATA_DIR"))
        .unwrap_or_else(|_| "zsei_data".to_string())
}

/// Append one record per model call. `result` is the pipeline executor's
/// own outcome — `Err` (subprocess failed / non-zero exit) and `Ok` with
/// an empty `response` are BOTH captured, with `success` false; that is
/// exactly the invisible-failure class this exists to make visible.
pub fn capture_zero_shot_call(
    pipeline: &str,
    call_site: &str,
    prompt_preview: &str,
    result: &Result<serde_json::Value, String>,
) {
    let dir = format!("{}/model_calls", capture_data_dir());
    if std::fs::create_dir_all(&dir).is_err() {
        return; // capture must never break the caller — same discipline as the host
    }
    let path = format!("{}/pipeline_zero_shot_calls.jsonl", dir);
    let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    let cut = |s: &str, n: usize| s.chars().take(n).collect::<String>();
    let (success, model_used, tokens_used, response_preview) = match result {
        Ok(v) => {
            let resp = v
                .get("response")
                .and_then(|r| r.as_str())
                .unwrap_or("");
            (
                !resp.trim().is_empty(),
                v.get("model_used")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string(),
                v.get("tokens_used").and_then(|t| t.as_u64()).unwrap_or(0),
                cut(resp, 500),
            )
        }
        Err(e) => (false, String::new(), 0, cut(e, 300)),
    };
    let record = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "pipeline": pipeline,
        "call_site": call_site,
        "model_used": model_used,
        "tokens_used": tokens_used,
        "success": success,
        "response_preview": response_preview,
        "prompt_preview": cut(prompt_preview, 200),
    });
    let _ = writeln!(f, "{}", record);
}
