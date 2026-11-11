//! Walk feed + walk participants (docs/evidence/mcp-participants/REPORT.md).
//!
//! An MCP agent can BE the prompt (it calls the orchestrate tool) and the walk then runs internally. The agents that
//! started or joined a walk are its PARTICIPANTS; this module is what makes them aware of the walk they are making:
//!   * `walk_participants.jsonl` — append-only: one `walk` row per submitted walk (run_token, trace_id, caller,
//!     participants, scope) and `bind` rows that attach the walk's task id to its trace once the task exists;
//!   * `walk_feed.jsonl` — append-only live feed: every orchestration stage event (`OrchestrationEventHub`) as one
//!     row with a monotonically increasing `seq`, so a reader asks "everything after seq N" and never polls raw HTTP
//!     endpoints for the same information. A lagging receiver writes a `lagged` row with the missed count — a gap is
//!     never silent.
//! Never rotated and never capped (the same doctrine as the hit and judgment logs); readers stream the file.
//! Per-node graph events are NOT mirrored here (they are voluminous and already served by the graph endpoints).

use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const FEED_FILE: &str = "walk_feed.jsonl";
const PARTICIPANTS_FILE: &str = "walk_participants.jsonl";

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string()))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn write_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

fn seq_counter() -> &'static AtomicU64 {
    static S: OnceLock<AtomicU64> = OnceLock::new();
    S.get_or_init(|| {
        let n = std::fs::File::open(data_dir().join(FEED_FILE))
            .map(|f| {
                std::io::BufReader::new(f)
                    .lines()
                    .filter(|l| l.is_ok())
                    .count() as u64
            })
            .unwrap_or(0);
        AtomicU64::new(n)
    })
}

fn append_line(file: &str, row: &Value) -> std::io::Result<()> {
    let _g = write_lock().lock().unwrap_or_else(|e| e.into_inner());
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(file))?;
    writeln!(f, "{}", row)
}

/// Append one feed row; returns its `seq`. A failed write is logged, never swallowed silently and never fatal.
pub fn append_row(mut row: Value) -> u64 {
    let seq = seq_counter().fetch_add(1, Ordering::SeqCst);
    if let Some(o) = row.as_object_mut() {
        o.insert("seq".to_string(), json!(seq));
        o.entry("ts".to_string())
            .or_insert_with(|| json!(now_secs()));
    }
    if let Err(e) = append_line(FEED_FILE, &row) {
        tracing::warn!("walk_feed: could not append feed row {seq}: {e}");
    }
    seq
}

/// Record a submitted walk and who is in it. `participants` always includes the caller.
#[allow(clippy::too_many_arguments)]
pub fn record_walk(
    run_token: Option<&str>,
    trace_id: &str,
    caller: &str,
    participants: &[String],
    user_id: u64,
    device_id: u64,
    project_id: Option<u64>,
    workspace_id: Option<u64>,
) {
    let mut all: Vec<String> = vec![caller.to_string()];
    for p in participants {
        if !all.iter().any(|a| a == p) {
            all.push(p.clone());
        }
    }
    let row = json!({
        "kind": "walk", "run_token": run_token, "trace_id": trace_id, "caller": caller, "participants": all,
        "user_id": user_id, "device_id": device_id, "project_id": project_id, "workspace_id": workspace_id, "at": now_secs(),
    });
    if let Err(e) = append_line(PARTICIPANTS_FILE, &row) {
        tracing::warn!("walk_feed: could not record walk participants: {e}");
    }
    append_row(
        json!({ "kind": "walk_submitted", "trace_id": trace_id, "run_token": run_token, "caller": caller, "participants": all }),
    );
}

/// Attach the walk's task id to its trace (called when the task exists — at the latest when the walk finishes).
pub fn bind_task(trace_id: &str, task_id: u64) {
    let row = json!({ "kind": "bind", "trace_id": trace_id, "task_id": task_id, "at": now_secs() });
    if let Err(e) = append_line(PARTICIPANTS_FILE, &row) {
        tracing::warn!("walk_feed: could not bind task {task_id}: {e}");
    }
    append_row(json!({ "kind": "task_bound", "trace_id": trace_id, "task_id": task_id }));
}

fn read_rows(file: &str) -> Vec<Value> {
    match std::fs::File::open(data_dir().join(file)) {
        Ok(f) => std::io::BufReader::new(f)
            .lines()
            .map_while(Result::ok)
            .filter_map(|l| serde_json::from_str::<Value>(&l).ok())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Resolve a walk by run_token, trace_id or task_id. Returns `{trace_id, run_token, caller, participants, task_id, ...}`
/// merged from the `walk` row and any `bind` row, or `None` when nothing matches.
pub fn resolve(
    run_token: Option<&str>,
    trace_id: Option<&str>,
    task_id: Option<u64>,
) -> Option<Value> {
    resolve_rows(&read_rows(PARTICIPANTS_FILE), run_token, trace_id, task_id)
}

fn resolve_rows(
    rows: &[Value],
    run_token: Option<&str>,
    trace_id: Option<&str>,
    task_id: Option<u64>,
) -> Option<Value> {
    // task_id -> trace via bind rows
    let mut trace: Option<String> = trace_id.map(String::from);
    if trace.is_none() {
        if let Some(rt) = run_token {
            trace = rows
                .iter()
                .find(|r| r["kind"] == "walk" && r["run_token"] == rt)
                .and_then(|r| r["trace_id"].as_str().map(String::from));
        }
    }
    if trace.is_none() {
        if let Some(t) = task_id {
            trace = rows
                .iter()
                .find(|r| r["kind"] == "bind" && r["task_id"] == t)
                .and_then(|r| r["trace_id"].as_str().map(String::from));
        }
    }
    let trace = trace?;
    let mut out = rows
        .iter()
        .find(|r| r["kind"] == "walk" && r["trace_id"] == trace.as_str())?
        .clone();
    if let Some(b) = rows
        .iter()
        .rev()
        .find(|r| r["kind"] == "bind" && r["trace_id"] == trace.as_str())
    {
        out["task_id"] = b["task_id"].clone();
    }
    Some(out)
}

/// Is `agent` a participant of the walk that `trace_id` identifies?
pub fn is_participant(trace_id: &str, agent: &str) -> bool {
    resolve(None, Some(trace_id), None)
        .and_then(|w| {
            w["participants"]
                .as_array()
                .map(|a| a.iter().any(|p| p == agent))
        })
        .unwrap_or(false)
}

/// Walks an agent participates in (newest first) — so an agent can discover the walks it is part of.
pub fn walks_for(agent: &str) -> Vec<Value> {
    let rows = read_rows(PARTICIPANTS_FILE);
    let mut out: Vec<Value> = Vec::new();
    for r in rows.iter().filter(|r| r["kind"] == "walk") {
        if r["participants"]
            .as_array()
            .map(|a| a.iter().any(|p| p == agent))
            .unwrap_or(false)
        {
            let mut w = r.clone();
            if let Some(b) = rows
                .iter()
                .rev()
                .find(|b| b["kind"] == "bind" && b["trace_id"] == r["trace_id"])
            {
                w["task_id"] = b["task_id"].clone();
            }
            out.push(w);
        }
    }
    out.reverse();
    out
}

/// Feed rows with `seq >= after` for one walk (matched by trace id), plus the cursor to ask for next. Rows carry the
/// raw stage data the host recorded; nothing is summarised away and nothing is capped.
pub fn read_feed(after: u64, trace_id: &str) -> (Vec<Value>, u64) {
    let mut next = after;
    let mut out = Vec::new();
    for r in read_rows(FEED_FILE) {
        let seq = r["seq"].as_u64().unwrap_or(0);
        if seq + 1 > next {
            next = seq + 1;
        }
        if seq < after {
            continue;
        }
        // lagged rows are global (a gap affects every walk): always shown to readers.
        if r["trace_id"] == trace_id || r["kind"] == "lagged" {
            out.push(r);
        }
    }
    (out, next)
}

/// Start the collector: every orchestration stage event becomes a feed row.
pub fn spawn_collector() {
    let mut rx = crate::orchestration_events::OrchestrationEventHub::global().subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(e) => {
                    append_row(json!({
                        "kind": "stage", "trace_id": e.trace_id, "user_id": e.user_id, "device_id": e.device_id,
                        "project_id": e.project_id, "workspace_id": e.workspace_id,
                        "stage": e.stage, "stage_name": e.stage_name, "success": e.success,
                        "summary": e.summary, "duration_ms": e.duration_ms, "ts": e.timestamp,
                    }));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    append_row(json!({ "kind": "lagged", "missed": n }));
                }
                Err(_) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_resolves_by_token_and_task_with_bind_row() {
        let rows = vec![
            json!({ "kind": "walk", "run_token": "run-1", "trace_id": "tr1", "caller": "claude-code", "participants": ["claude-code", "zcode"] }),
            json!({ "kind": "bind", "trace_id": "tr1", "task_id": 42 }),
        ];
        let by_token =
            resolve_rows(&rows, Some("run-1"), None, None).expect("resolves by run_token");
        assert_eq!(by_token["participants"], json!(["claude-code", "zcode"]));
        assert_eq!(by_token["task_id"], json!(42));
        assert_eq!(
            resolve_rows(&rows, None, None, Some(42)).unwrap()["trace_id"],
            "tr1"
        );
        assert!(resolve_rows(&rows, Some("run-2"), None, None).is_none());
    }
}
