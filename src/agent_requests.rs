//! Agent provider (docs/evidence/mcp-participants/REPORT.md): a walk step whose provider is a participating MCP
//! AGENT, not a process the host can call.
//!
//! A tool row with transport `agent` and endpoint `agent:<name>` (or `agent:*` = any participant of the walk) is
//! an agent provider. A provider step that calls it does not "delegate and move on": the call creates a durable
//! REQUEST and AWAITS the agent's report. The step completes when the agent reports; there is no timeout and no
//! fail-open — a request nobody answers keeps the step (and so the walk) waiting, visibly, exactly like any other
//! gate that awaits. Completed is final: a request answers once; a rerun of the task is a new task instance and
//! therefore a new request.
//!
//! Durability: `agent_requests.jsonl` is append-only (`request` rows, `report` rows). A restarted host re-reads it,
//! so a resumed walk that issues the same step again (same task_id + step_index + input) ADOPTS the earlier request
//! — and its answer if one was already reported — instead of asking twice.
//!
//! Authorisation of a report: the reporting agent must be the request's target, or — for `agent:*` — a participant
//! of the walk the request belongs to (`walk_feed`). The agent name is declared by the caller; the MCP server binds
//! it to the process identity (H1). The report records the validated device id when the route could establish one.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tokio::sync::watch;

const FILE: &str = "agent_requests.jsonl";

struct Entry {
    row: Value,
    tx: watch::Sender<Option<Value>>,
}

#[derive(Default)]
struct Store {
    loaded: bool,
    by_id: HashMap<String, Entry>,
}

fn store() -> &'static Mutex<Store> {
    static S: OnceLock<Mutex<Store>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Store::default()))
}

fn path() -> PathBuf {
    PathBuf::from(std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string()))
        .join(FILE)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn append(row: &Value) -> Result<(), String> {
    if let Some(dir) = path().parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("agent request store unavailable: {e}"))?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path())
        .map_err(|e| format!("agent request store unavailable: {e}"))?;
    writeln!(f, "{row}").map_err(|e| format!("agent request store write failed: {e}"))
}

fn ensure_loaded(s: &mut Store) {
    if s.loaded {
        return;
    }
    s.loaded = true;
    let Ok(text) = std::fs::read_to_string(path()) else {
        return;
    };
    for line in text.lines() {
        let Ok(r) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let id = r["id"].as_str().unwrap_or("").to_string();
        match r["event"].as_str() {
            Some("request") => {
                let (tx, _rx) = watch::channel(None);
                s.by_id.insert(id, Entry { row: r, tx });
            }
            Some("report") => {
                if let Some(e) = s.by_id.get_mut(&id) {
                    e.row["status"] = json!("answered");
                    e.row["result"] = r["result"].clone();
                    e.row["reported_by"] = r["by"].clone();
                    e.row["reported_at"] = r["at"].clone();
                    let _ = e.tx.send(Some(r["result"].clone()));
                }
            }
            _ => {}
        }
    }
}

/// Deterministic id: resume-safe when the step identity (task + step) is known, else scoped to the trace.
fn request_id(tool: &str, input: &Value, ctx: Option<&Value>) -> String {
    let c = ctx.cloned().unwrap_or(Value::Null);
    let scope = match (c["task_id"].as_u64(), c["step_index"].as_u64()) {
        (Some(t), Some(s)) => format!("task:{t}:step:{s}"),
        _ => format!(
            "trace:{}:task:{}",
            c["trace_id"].as_str().unwrap_or(""),
            c["task_id"]
        ),
    };
    let h = blake3::hash(format!("{tool}|{scope}|{input}").as_bytes())
        .to_hex()
        .to_string();
    format!("ar-{}", &h[..16])
}

/// Create (or adopt) the request for one agent-provider call and wait for the agent's report. Never times out.
pub async fn request_and_await(
    tool: &str,
    endpoint: &str,
    caller: &str,
    input: &Value,
    ctx: Option<&Value>,
) -> Result<Value, String> {
    let target = endpoint
        .strip_prefix("agent:")
        .unwrap_or(endpoint)
        .to_string();
    let id = request_id(tool, input, ctx);
    // G4: content leaving the host for an agent passes the instance's outbound jurisdiction policy FIRST. A request that already exists
    // (a resumed walk adopting it) was cleared when it was created. A hold AWAITS — retried with a doubling backoff (2 s .. 60 s) until the
    // operator's decision clears it; the content is never handed to the agent while held, and nothing fails open.
    let already_requested = {
        let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
        ensure_loaded(&mut s);
        s.by_id.contains_key(&id)
    };
    if !already_requested {
        let mut wait = std::time::Duration::from_secs(2);
        loop {
            let verdict = crate::orchestrator::jurisdiction::policy::check_outbound_to_agent(endpoint, input, None).await;
            if verdict.allow {
                break;
            }
            tracing::warn!(tool, endpoint, reason = %verdict.reason, "agent provider call held by the jurisdiction outbound policy — awaiting the operator");
            tokio::time::sleep(wait).await;
            wait = (wait * 2).min(std::time::Duration::from_secs(60));
        }
    }
    let mut rx = {
        let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
        ensure_loaded(&mut s);
        if let Some(e) = s.by_id.get(&id) {
            e.tx.subscribe()
        } else {
            let trace_id = ctx
                .and_then(|c| c["trace_id"].as_str())
                .unwrap_or("")
                .to_string();
            let row = json!({
                "event": "request", "id": id, "tool": tool, "target": target, "caller": caller,
                "input": input, "context": ctx, "trace_id": trace_id, "status": "pending", "at": now_secs(),
            });
            // a request that is not durably recorded must not exist: fail loudly instead of awaiting a phantom
            append(&row)?;
            let (tx, rx) = watch::channel(None);
            s.by_id.insert(id.clone(), Entry { row, tx });
            rx
        }
    };
    loop {
        if let Some(v) = rx.borrow().clone() {
            return Ok(v);
        }
        if rx.changed().await.is_err() {
            return Err(format!("agent request {id} was dropped before an answer"));
        }
    }
}

/// Record an agent's report. Completed is final: a second report is refused.
pub fn report(id: &str, by: &str, result: Value, device_id: Option<u64>) -> Result<Value, String> {
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut s);
    let e = s
        .by_id
        .get_mut(id)
        .ok_or_else(|| format!("no agent request '{id}'"))?;
    if e.row["status"] == "answered" {
        return Err(format!(
            "agent request {id} is already answered (completed is final)"
        ));
    }
    let target = e.row["target"].as_str().unwrap_or("").to_string();
    let trace_id = e.row["trace_id"].as_str().unwrap_or("").to_string();
    let allowed =
        target == by || (target == "*" && crate::walk_feed::is_participant(&trace_id, by));
    if !allowed {
        return Err(format!(
            "agent '{by}' may not report request {id}: it is addressed to '{target}'"
        ));
    }
    let row = json!({ "event": "report", "id": id, "by": by, "device_id": device_id, "result": result, "at": now_secs() });
    append(&row)?;
    e.row["status"] = json!("answered");
    e.row["result"] = result.clone();
    e.row["reported_by"] = json!(by);
    e.row["reported_at"] = row["at"].clone();
    let _ = e.tx.send(Some(result));
    Ok(json!({ "id": id, "status": "answered", "reported_by": by }))
}

/// Pending requests an agent may answer (addressed to it, or `*` for a walk it participates in).
pub fn pending_for(agent: &str) -> Vec<Value> {
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut s);
    let mut out: Vec<Value> = s
        .by_id
        .values()
        .filter(|e| e.row["status"] == "pending")
        .filter(|e| {
            let t = e.row["target"].as_str().unwrap_or("");
            t == agent
                || (t == "*"
                    && crate::walk_feed::is_participant(
                        e.row["trace_id"].as_str().unwrap_or(""),
                        agent,
                    ))
        })
        .map(|e| e.row.clone())
        .collect();
    out.sort_by_key(|r| r["at"].as_u64().unwrap_or(0));
    out
}

/// EVERY open (pending) request, oldest first — the operator's view ("which walk steps are waiting on which agent"), unlike `pending_for`
/// which is one agent's inbox. Each row carries `age_secs`. There is no timeout: a pending request stays pending until its agent reports.
pub fn open_requests() -> Vec<Value> {
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut s);
    let now = now_secs();
    let mut out: Vec<Value> = s
        .by_id
        .values()
        .filter(|e| e.row["status"] == "pending")
        .map(|e| {
            let mut r = e.row.clone();
            let at = r["at"].as_u64().unwrap_or(0);
            if let Some(o) = r.as_object_mut() {
                o.insert("age_secs".into(), json!(now.saturating_sub(at)));
            }
            r
        })
        .collect();
    out.sort_by_key(|r| r["at"].as_u64().unwrap_or(0));
    out
}

/// One request by id (any status).
pub fn get(id: &str) -> Option<Value> {
    let mut s = store().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded(&mut s);
    s.by_id.get(id).map(|e| e.row.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_resume_safe_for_the_same_step_and_differs_across_tasks() {
        let ctx = json!({ "task_id": 7, "step_index": 2, "trace_id": "a" });
        let resumed = json!({ "task_id": 7, "step_index": 2, "trace_id": "b" });
        let rerun = json!({ "task_id": 8, "step_index": 2, "trace_id": "c" });
        let input = json!({ "query": "review this" });
        assert_eq!(
            request_id("t", &input, Some(&ctx)),
            request_id("t", &input, Some(&resumed))
        );
        assert_ne!(
            request_id("t", &input, Some(&ctx)),
            request_id("t", &input, Some(&rerun))
        );
    }
}
