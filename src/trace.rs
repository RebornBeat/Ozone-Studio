//! Trace spine: one id that ties every model / tool / pipeline call, including
//! nested ones, back to the run that caused it.
//!
//! A *trace* is one run (an `/orchestrate` walk, one loop pass, one external
//! `/pipeline/execute` or `/mcp/call`). A *span* is one unit of work inside it
//! (a stage, a blueprint step, a fallback candidate, an executor call). Every
//! capture record carries `trace_id` / `span_id` / `parent_span` so a run's
//! calls can be listed from the run, and a call's run found from the call.
//!
//! Propagation:
//! - inside the host: a `tokio::task_local!` (same pattern and same limits as
//!   `pipeline::gate::with_priority` — a `tokio::spawn` does NOT inherit it, so
//!   every spawned future that makes calls must be wrapped with `with_trace`);
//! - across process boundaries: environment variables (`OZONE_TRACE_ID`,
//!   `OZONE_PARENT_SPAN`) set on the spawned pipeline command. The host never
//!   calls `env_clear`, so nested spawns (modality pipeline -> `prompt` binary)
//!   inherit them with no change at their call sites.
//!
//! Reading the trace is always optional: `current_trace()` is `None` outside a
//! scope, and every writer must treat that as "untraced", never as an error.

use serde::{Deserialize, Serialize};
use std::future::Future;

/// Env var carrying the trace id into spawned pipeline / tool processes.
pub const ENV_TRACE_ID: &str = "OZONE_TRACE_ID";
/// Env var carrying the span id of the caller (becomes the child's parent).
pub const ENV_PARENT_SPAN: &str = "OZONE_PARENT_SPAN";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceCtx {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span: Option<String>,
    /// What this span is: "orchestrate", "stage", "step", "loop-pass",
    /// "external-pipeline", "mcp-call", "executor-call", ...
    pub kind: String,
}

fn new_id() -> String {
    // 16 hex chars of a v4 uuid: short enough for jsonl rows, random enough to
    // be unique per run on one host.
    uuid::Uuid::new_v4().simple().to_string()[..16].to_string()
}

impl TraceCtx {
    /// A new root span (also starts a new trace).
    pub fn new_root(kind: &str) -> Self {
        TraceCtx {
            trace_id: new_id(),
            span_id: new_id(),
            parent_span: None,
            kind: kind.to_string(),
        }
    }

    /// A child span in the same trace.
    pub fn child(&self, kind: &str) -> Self {
        TraceCtx {
            trace_id: self.trace_id.clone(),
            span_id: new_id(),
            parent_span: Some(self.span_id.clone()),
            kind: kind.to_string(),
        }
    }

    /// Rebuild a context inherited through the environment of a spawned
    /// process (`OZONE_TRACE_ID` / `OZONE_PARENT_SPAN`). The returned span is a
    /// fresh child of the caller's span. `None` when no trace was inherited.
    pub fn from_env(kind: &str) -> Option<Self> {
        let trace_id = std::env::var(ENV_TRACE_ID).ok().filter(|s| !s.is_empty())?;
        let parent = std::env::var(ENV_PARENT_SPAN).ok().filter(|s| !s.is_empty());
        Some(TraceCtx {
            trace_id,
            span_id: new_id(),
            parent_span: parent,
            kind: kind.to_string(),
        })
    }

    /// Environment pairs to set on a spawned command so the child (and its own
    /// nested spawns) stay in this trace. The child's parent span is THIS span.
    pub fn env_pairs(&self) -> Vec<(String, String)> {
        vec![
            (ENV_TRACE_ID.to_string(), self.trace_id.clone()),
            (ENV_PARENT_SPAN.to_string(), self.span_id.clone()),
        ]
    }

    /// The three correlation fields as a JSON object, ready to merge into a
    /// capture record.
    pub fn fields(&self) -> serde_json::Value {
        serde_json::json!({
            "trace_id": self.trace_id,
            "span_id": self.span_id,
            "parent_span": self.parent_span,
            "span_kind": self.kind,
        })
    }
}

tokio::task_local! {
    static TRACE: TraceCtx;
}

/// Run `fut` with `ctx` as the current trace context.
pub fn with_trace<F: Future>(ctx: TraceCtx, fut: F) -> impl Future<Output = F::Output> {
    TRACE.scope(ctx, fut)
}

/// The current trace context, or `None` when called outside any scope.
pub fn current_trace() -> Option<TraceCtx> {
    TRACE.try_with(|t| t.clone()).ok()
}

/// A child of the current span, or a fresh root of `root_kind` when there is
/// no current trace. Does NOT install the new context — wrap the work in
/// `with_trace(ctx.clone(), ...)` to do that.
pub fn child_or_root(kind: &str, root_kind: &str) -> TraceCtx {
    match current_trace() {
        Some(t) => t.child(kind),
        None => TraceCtx::new_root(root_kind),
    }
}

/// Correlation fields of the current trace for a capture record. Always an
/// object: nulls when untraced, so record shapes stay stable.
pub fn current_fields() -> serde_json::Value {
    match current_trace() {
        Some(t) => t.fields(),
        None => serde_json::json!({
            "trace_id": serde_json::Value::Null,
            "span_id": serde_json::Value::Null,
            "parent_span": serde_json::Value::Null,
            "span_kind": serde_json::Value::Null,
        }),
    }
}

/// Merge the current trace fields into a JSON object record in place. A
/// non-object `record` is left untouched.
pub fn stamp(record: &mut serde_json::Value) {
    if let (Some(obj), serde_json::Value::Object(fields)) = (record.as_object_mut(), current_fields()) {
        for (k, v) in fields {
            obj.entry(k).or_insert(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn child_shares_trace_and_links_parent() {
        let root = TraceCtx::new_root("orchestrate");
        let child = root.child("stage");
        assert_eq!(root.trace_id, child.trace_id);
        assert_eq!(child.parent_span.as_deref(), Some(root.span_id.as_str()));
        assert_ne!(root.span_id, child.span_id);
    }

    #[tokio::test]
    async fn current_trace_is_scoped() {
        assert!(current_trace().is_none());
        let root = TraceCtx::new_root("t");
        let id = root.trace_id.clone();
        with_trace(root, async move {
            assert_eq!(current_trace().map(|t| t.trace_id), Some(id));
        })
        .await;
        assert!(current_trace().is_none());
    }

    #[tokio::test]
    async fn stamp_adds_fields_without_overwriting() {
        let mut rec = serde_json::json!({"call_site": "x", "trace_id": "keep"});
        let root = TraceCtx::new_root("t");
        with_trace(root, async {
            stamp(&mut rec);
        })
        .await;
        assert_eq!(rec["trace_id"], "keep");
        assert!(rec["span_id"].is_string());
    }
}
