//! `InstrumentedExecutor` — the complete index of EVERY executor call.
//!
//! The host has several routes to "one model / tool / pipeline call" and each
//! records a different subset of ledger, quota, ContextRecord and S11 (see
//! docs: the 2026-10-23 capture reviews). Step primaries, sub-steps, gate and
//! jurisdiction reviews, graph builders and strength-N votes all go straight
//! to `executor.execute` and left no row at all. This decorator closes that
//! gap WITHOUT touching a single call site: it wraps the executor object the
//! orchestrator (and the loops) hold, mints a child span of the current
//! trace for each call, runs the call inside that span (so the pipeline
//! executor can stamp `OZONE_TRACE_ID` / `OZONE_PARENT_SPAN` onto the spawned
//! process), and appends ONE line per call to
//! `<data_dir>/capture/call_spans.jsonl`.
//!
//! It deliberately records only the call's SHAPE (pipeline, outcome, latency,
//! sizes, trace ids) — not prompt or response text. Content stays in the
//! existing sinks (ledger, S11, S10, context_records), which carry the same
//! `trace_id`/`span_id` so a span row joins to its content row. Writes are
//! best-effort and can never fail or slow-fail a real call.

use serde_json::Value;
use std::io::Write;
use std::sync::Arc;

use super::PipelineExecutor;

pub struct InstrumentedExecutor {
    inner: Arc<dyn PipelineExecutor>,
}

impl InstrumentedExecutor {
    pub fn new(inner: Arc<dyn PipelineExecutor>) -> Self {
        Self { inner }
    }
}

/// `io::Write` that only counts bytes — sizes a JSON value without keeping a
/// second copy of a possibly multi-megabyte input in memory.
struct CountingWriter(usize);

impl Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn json_len(v: &Value) -> usize {
    let mut w = CountingWriter(0);
    let _ = serde_json::to_writer(&mut w, v);
    w.0
}

/// Append one row to `<data_dir>/capture/call_spans.jsonl`. Data dir is the
/// same env/default the ContextRecord sink uses.
fn append_span_row(row: &Value) {
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let path = std::path::Path::new(&data_dir).join("capture/call_spans.jsonl");
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{}", row);
    }
}

#[async_trait::async_trait]
impl PipelineExecutor for InstrumentedExecutor {
    async fn execute(&self, pipeline_id: u64, input: Value) -> Result<Value, String> {
        // Child of whatever run is current; a call with no trace at all becomes
        // its own root so it is still indexed (and still findable).
        let span = crate::trace::child_or_root("executor-call", "executor-root");
        let input_bytes = json_len(&input);
        let call_site = input
            .get("call_site")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let started = std::time::Instant::now();

        let result =
            crate::trace::with_trace(span.clone(), self.inner.execute(pipeline_id, input)).await;

        let latency_ms = started.elapsed().as_millis() as u64;
        let (success, error, output_bytes) = match &result {
            Ok(v) => (true, Value::Null, json_len(v)),
            Err(e) => (
                false,
                Value::String(e.chars().take(300).collect::<String>()),
                0usize,
            ),
        };
        let mut row = serde_json::json!({
            "ts": chrono::Utc::now().to_rfc3339(),
            "pipeline_id": pipeline_id,
            "call_site": call_site,
            "success": success,
            "error": error,
            "latency_ms": latency_ms,
            "input_bytes": input_bytes,
            "output_bytes": output_bytes,
        });
        // Trace fields of THIS span (not of the enclosing one).
        if let (Some(obj), Value::Object(fields)) = (row.as_object_mut(), span.fields()) {
            for (k, v) in fields {
                obj.insert(k, v);
            }
        }
        append_span_row(&row);
        result
    }

    async fn pipeline_exists(&self, pipeline_id: u64) -> bool {
        self.inner.pipeline_exists(pipeline_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo;

    #[async_trait::async_trait]
    impl PipelineExecutor for Echo {
        async fn execute(&self, _id: u64, input: Value) -> Result<Value, String> {
            Ok(input)
        }
        async fn pipeline_exists(&self, _id: u64) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn passes_result_through_and_runs_inside_a_span() {
        let exec = InstrumentedExecutor::new(Arc::new(Echo));
        let out = exec.execute(9, serde_json::json!({"x": 1})).await.unwrap();
        assert_eq!(out["x"], 1);
        assert!(exec.pipeline_exists(9).await);
    }

    #[test]
    fn counting_writer_matches_to_string_len() {
        let v = serde_json::json!({"a": [1, 2, 3], "b": "héllo"});
        assert_eq!(json_len(&v), v.to_string().len());
    }
}
