//! Durable awaiting items for meta / consciousness gates (2026-11-01 doctrine).
//!
//! Consciousness and meta gates are decided ONLY by meta/consciousness. When
//! they cannot judge (rate limit, empty reply, confetti, all models failed →
//! `ReviewPending` / `ReviewFailed`), the walk must AWAIT — never push
//! through, never fail open, never ask a human. Humans decide only
//! jurisdiction `RequireConfirmation` (kind `jurisdiction_confirm`).
//!
//! Persistence: append-only JSONL at `<data_dir>/awaiting_items.jsonl`; the
//! latest row per `await_id` wins. Items survive restart; a driver may list
//! `due_items` and resume them.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::OnceLock;
use tokio::sync::Notify;

pub const KIND_CONSCIOUSNESS_GATE: &str = "consciousness_gate";
pub const KIND_META_GATE: &str = "meta_gate";
pub const KIND_JURISDICTION_CONFIRM: &str = "jurisdiction_confirm";

pub const STATUS_AWAITING: &str = "awaiting";
pub const STATUS_RESOLVED: &str = "resolved";
pub const STATUS_CANCELLED: &str = "cancelled";

/// Backoff ceiling between meta re-judgment attempts.
pub const BACKOFF_CEILING_SECS: u64 = 30 * 60;
pub const BACKOFF_FLOOR_SECS: u64 = 15;

static AWAIT_WAKE: OnceLock<Notify> = OnceLock::new();

/// Notified whenever an item is resolved/cancelled/added so waiters re-check.
pub fn await_wake() -> &'static Notify {
    AWAIT_WAKE.get_or_init(Notify::new)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AwaitingItem {
    pub await_id: String,
    pub kind: String,
    pub status: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub review_id: Option<String>,
    pub subject: String,
    #[serde(default)]
    pub reason: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(default)]
    pub attempts: u32,
    pub next_attempt_at_ms: u64,
    #[serde(default)]
    pub resolution: Option<String>,
    /// Set when a resolution has been written and the owner should continue.
    #[serde(default)]
    pub resume_requested: bool,
    #[serde(default)]
    pub resume_applied: bool,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn path(data_dir: &str) -> String {
    format!("{}/awaiting_items.jsonl", data_dir.trim_end_matches('/'))
}

/// Exponential backoff: 15s, 30s, 60s … capped at 30 minutes.
pub fn backoff_secs(attempts: u32) -> u64 {
    let shift = attempts.min(10);
    (BACKOFF_FLOOR_SECS.saturating_mul(1u64 << shift)).min(BACKOFF_CEILING_SECS)
}

pub fn make_await_id(kind: &str, subject: &str, task_id: Option<&str>) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    kind.hash(&mut h);
    subject.hash(&mut h);
    task_id.hash(&mut h);
    format!("aw-{:016x}", h.finish())
}

fn append_row(data_dir: &str, item: &AwaitingItem) -> Result<(), String> {
    let p = path(data_dir);
    if let Some(parent) = std::path::Path::new(&p).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let line = serde_json::to_string(item).map_err(|e| e.to_string())?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
        .map_err(|e| format!("awaiting open {}: {}", p, e))?;
    writeln!(f, "{}", line).map_err(|e| e.to_string())?;
    await_wake().notify_waiters();
    Ok(())
}

/// Latest row per await_id.
pub fn load_all(data_dir: &str) -> Vec<AwaitingItem> {
    let Ok(text) = std::fs::read_to_string(path(data_dir)) else {
        return Vec::new();
    };
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::HashMap<String, AwaitingItem> = Default::default();
    for line in text.lines() {
        if let Ok(item) = serde_json::from_str::<AwaitingItem>(line) {
            if !map.contains_key(&item.await_id) {
                order.push(item.await_id.clone());
            }
            map.insert(item.await_id.clone(), item);
        }
    }
    order.into_iter().filter_map(|id| map.remove(&id)).collect()
}

pub fn get(data_dir: &str, await_id: &str) -> Option<AwaitingItem> {
    load_all(data_dir).into_iter().find(|i| i.await_id == await_id)
}

/// Create (or re-open) an awaiting item. Idempotent per await_id: an existing
/// non-terminal item is returned unchanged.
pub fn open(
    data_dir: &str,
    kind: &str,
    subject: &str,
    task_id: Option<&str>,
    review_id: Option<&str>,
    reason: &str,
) -> Result<AwaitingItem, String> {
    let await_id = make_await_id(kind, subject, task_id);
    if let Some(existing) = get(data_dir, &await_id) {
        if existing.status == STATUS_AWAITING {
            return Ok(existing);
        }
    }
    let now = now_ms();
    let item = AwaitingItem {
        await_id,
        kind: kind.to_string(),
        status: STATUS_AWAITING.to_string(),
        task_id: task_id.map(|s| s.to_string()),
        review_id: review_id.map(|s| s.to_string()),
        // NO CAP (2026-11-04 doctrine audit): the operator-visible subject/reason are stored whole
        subject: subject.to_string(),
        reason: reason.to_string(),
        created_at_ms: now,
        updated_at_ms: now,
        attempts: 0,
        next_attempt_at_ms: now + backoff_secs(0) * 1000,
        ..Default::default()
    };
    append_row(data_dir, &item)?;
    Ok(item)
}

/// Record a failed re-judgment attempt and push the next attempt out.
pub fn record_attempt(data_dir: &str, await_id: &str, reason: &str) -> Option<AwaitingItem> {
    let mut item = get(data_dir, await_id)?;
    item.attempts += 1;
    item.updated_at_ms = now_ms();
    item.next_attempt_at_ms = item.updated_at_ms + backoff_secs(item.attempts) * 1000;
    item.reason = reason.to_string();
    append_row(data_dir, &item).ok()?;
    Some(item)
}

/// Meta/consciousness produced a judgment. Marks resolved and requests resume.
pub fn mark_resolved(data_dir: &str, await_id: &str, resolution: &str) -> Option<AwaitingItem> {
    let mut item = get(data_dir, await_id)?;
    item.status = STATUS_RESOLVED.to_string();
    item.resolution = Some(resolution.to_string());
    item.resume_requested = true;
    item.updated_at_ms = now_ms();
    append_row(data_dir, &item).ok()?;
    Some(item)
}

pub fn mark_cancelled(data_dir: &str, await_id: &str, why: &str) -> Option<AwaitingItem> {
    let mut item = get(data_dir, await_id)?;
    item.status = STATUS_CANCELLED.to_string();
    item.resolution = Some(why.to_string());
    item.updated_at_ms = now_ms();
    append_row(data_dir, &item).ok()?;
    Some(item)
}

pub fn mark_applied(data_dir: &str, await_id: &str) -> Option<AwaitingItem> {
    let mut item = get(data_dir, await_id)?;
    item.resume_applied = true;
    item.resume_requested = false;
    item.updated_at_ms = now_ms();
    append_row(data_dir, &item).ok()?;
    Some(item)
}

/// Items still awaiting (what the Order / UI shows).
pub fn pending_actions(data_dir: &str) -> Vec<AwaitingItem> {
    load_all(data_dir)
        .into_iter()
        .filter(|i| i.status == STATUS_AWAITING)
        .collect()
}

/// Awaiting items whose backoff has elapsed, bounded by `limit`.
pub fn due_items(data_dir: &str, limit: usize) -> Vec<AwaitingItem> {
    let now = now_ms();
    pending_actions(data_dir)
        .into_iter()
        .filter(|i| i.kind != KIND_JURISDICTION_CONFIRM && i.next_attempt_at_ms <= now)
        .take(limit)
        .collect()
}

/// Resolved items whose owner has not yet applied the resume.
pub fn take_resume_requests(data_dir: &str) -> Vec<AwaitingItem> {
    load_all(data_dir)
        .into_iter()
        .filter(|i| i.resume_requested && !i.resume_applied)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_capped() {
        assert_eq!(backoff_secs(0), 15);
        assert_eq!(backoff_secs(1), 30);
        assert_eq!(backoff_secs(20), BACKOFF_CEILING_SECS);
    }

    #[test]
    fn open_resolve_roundtrip() {
        let dir = std::env::temp_dir().join(format!("aw-test-{}", now_ms()));
        let d = dir.to_str().unwrap();
        let a = open(d, KIND_CONSCIOUSNESS_GATE, "plan", Some("t1"), None, "429").unwrap();
        assert_eq!(pending_actions(d).len(), 1);
        mark_resolved(d, &a.await_id, "Proceed").unwrap();
        assert!(pending_actions(d).is_empty());
        assert_eq!(take_resume_requests(d).len(), 1);
        mark_applied(d, &a.await_id).unwrap();
        assert!(take_resume_requests(d).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
