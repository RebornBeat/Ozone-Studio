//! Entry gate for EXTERNAL pipeline dispatch (the gated-entry decorator,
//! finalized design review 2026-10-23; this module materializes it).
//!
//! The orchestrator's walk has its own gates (jurisdiction at stage 0/8b,
//! consciousness at stage 8) — but `/pipeline/execute` and `rerun_step`
//! dispatch a pipeline DIRECTLY, outside any walk. Until this module those
//! raw entries left no trace row at all: no trace id minted, no span
//! recorded, nothing joining the call to anything. `gated` closes that:
//!
//! 1. mints a trace root (kind `external-pipeline`) — or a child span when
//!    the caller is already inside a trace (e.g. `file_link`'s HTTP
//!    re-entry from inside a walk, which is exactly how the walk's nested
//!    HTTP dispatch stays JOINED to the walk's trace instead of forking a
//!    new one);
//! 2. wraps the dispatch future in `with_trace` so the pipeline executor's
//!    own env stamping (`OZONE_TRACE_ID` / `OZONE_PARENT_SPAN`) carries the
//!    ids into the spawned process and its nested spawns;
//! 3. appends ONE shape row to the same `capture/call_spans.jsonl` sink
//!    `instrumented.rs` uses — entry fact, pipeline id, trace ids, latency.
//!    No prompt/response content (that lives in the content sinks, joined
//!    by the same trace ids).
//!
//! Jurisdiction at external entries is deliberately NOT reimplemented here:
//! `/pipeline/execute` already enforces session + consciousness-category
//! rejection at its handler, `/mcp/call` runs the real jurisdiction match
//! before invoke, and the walk's stages own the plan-level checks. What was
//! genuinely missing was the TRACE — that is what this module adds, with no
//! behavior change to the dispatch itself.

use std::future::Future;
use std::io::Write;
use std::time::Instant;

/// Append one row to `<OZONE_ZSEI_DATA_DIR|zsei_data>/capture/call_spans.jsonl`
/// — the same sink `instrumented.rs` writes, same best-effort discipline: a
/// capture failure is logged and swallowed, never propagated into the call.
fn append_entry_row(row: &serde_json::Value) {
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let dir = format!("{data_dir}/capture");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!("entry gate: cannot create {dir}: {e}");
        return;
    }
    let path = format!("{dir}/call_spans.jsonl");
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut f) => {
            if let Err(e) = writeln!(f, "{row}") {
                tracing::warn!("entry gate: span row write failed: {e}");
            }
        }
        Err(e) => tracing::warn!("entry gate: cannot open {path}: {e}"),
    }
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Run an EXTERNAL dispatch under the entry gate: trace root (or joined
/// child), the dispatch future executed inside that trace scope, and one
/// span row recording the entry. Generic over the dispatch result — the
/// outcome itself belongs to the type-specific sinks; this row carries the
/// entry FACT and the correlation ids.
pub async fn gated<F, T>(pipeline_id: u64, fut: F) -> T
where
    F: Future<Output = T>,
{
    let started = Instant::now();
    let ctx = crate::trace::child_or_root("executor-call", "external-pipeline");

    let mut row = serde_json::json!({
        "ts_ms": now_millis(),
        "span_kind": "external-pipeline",
        "pipeline_id": pipeline_id,
    });
    if let (Some(obj), serde_json::Value::Object(fields)) =
        (row.as_object_mut(), ctx.fields())
    {
        for (k, v) in fields {
            obj.insert(k, v);
        }
    }

    let out = crate::trace::with_trace(ctx, fut).await;

    if let Some(obj) = row.as_object_mut() {
        obj.insert(
            "latency_ms".to_string(),
            serde_json::json!(started.elapsed().as_millis() as u64),
        );
    }
    append_entry_row(&row);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate must install the trace for the wrapped future (a nested
    /// `current_trace()` sees the entry root) and record one row.
    #[tokio::test]
    async fn gated_installs_trace_and_records_row() {
        let dir = std::env::temp_dir().join(format!("entry_gate_test_{}", std::process::id()));
        std::env::set_var("OZONE_ZSEI_DATA_DIR", &dir);
        let seen = gated(9, async {
            crate::trace::current_trace().map(|t| t.trace_id)
        })
        .await;
        assert!(seen.is_some(), "the trace must be installed inside gated");
        let path = dir.join("capture/call_spans.jsonl");
        let rows = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            rows.contains("\"span_kind\":\"external-pipeline\""),
            "one entry row expected, got: {rows}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Re-entry from INSIDE a walk joins the walk's trace instead of forking
    /// a new one (the file_link-over-HTTP case).
    #[tokio::test]
    async fn gated_joins_an_existing_trace() {
        let root = crate::trace::TraceCtx::new_root("orchestrate");
        let root_id = root.trace_id.clone();
        crate::trace::with_trace(root, async {
            let inner_id = gated(21, async {
                crate::trace::current_trace().map(|t| t.trace_id)
            })
            .await;
            assert_eq!(inner_id, Some(root_id));
        })
        .await;
    }
}
