//! Personal-Assistant proposals — the PROPOSE half of the acting chain,
//! riding the EXISTING review infrastructure (operator correction 2026-10-29:
//! the assistant works alongside the system like every other component — it
//! does not get a private acting path; proposals are REVIEW rows like every
//! other "needs a human" item).
//!
//! The system's one acting chain, which this module joins:
//!   sense (findings) → propose (a review row) → operator disposition
//!   (the EXISTING /consciousness/reviews/resolve route) → gated execution
//!   (the EXISTING mcp::call_global door — metered, jurisdiction-gated,
//!   graph-rippling) → capture (an S10 result row).
//!
//! What this module contributes:
//! 1. `derive_proposals` — deterministic proposal rules over the same
//!    machine-computed findings the digest sees. A meeting with a real URL
//!    becomes "open it"; a slippage cascade becomes "show the Order board";
//!    an overdue item with an external link becomes "open that link". A
//!    finding without concrete data proposes NOTHING.
//! 2. `record_pending` — each proposal becomes a REAL final S10 row
//!    (`model_calls/decision_review.jsonl`, `review_kind: assistant-proposal`,
//!    `decision: review_pending`) — the exact shape the existing
//!    `/consciousness/reviews/pending` route lists and the resolve route
//!    keys on. Stable kind-scoped review ids; deduped while unresolved.
//! 3. `execute_resolved` — called by the resolve route when an
//!    assistant-proposal review is resolved "Proceed": validates the tool
//!    against the whitelist, runs it through `call_global` under its own
//!    trace root, and appends an honest `executed`/`failed` S10 row with the
//!    real outcome. "Decline" needs nothing extra — the resolution row IS
//!    the record.
//!
//! No new routes, no private store: the review system is the proposal
//! system. No invented confidence anywhere — every row carries real values
//! (tokens_used: 0, model_used: "none (deterministic derivation)").

use crate::consciousness::assistant::AssistantFinding;
use crate::task::TaskData;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Review kind every assistant proposal row carries (kind-scoped, so the
/// resolve route and readers can tell proposals from model judgment reviews).
pub const PROPOSAL_REVIEW_KIND: &str = "assistant-proposal";

/// Tools a proposal may ever name. The derive rules only ever emit
/// `browser_open` today; this list bounds what a Proceed on a (tampered or
/// future) review row can reach. Execution still goes through the registry,
/// the usage meter, and the jurisdiction gate. `farm_command` is
/// deliberately NOT here: it drives a live profile through the Command
/// Center channel (arbitrary multi-step behaviour) — an operator/CC act, not
/// something an assistant proposal may name.
pub const PROPOSABLE_TOOLS: &[&str] = &[
    "browser_open",
    "browser_navigate",
    "browser_screenshot",
    "farm_screenshot",
];

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn s10_dir() -> String {
    let data_dir =
        std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    format!("{}/model_calls", data_dir)
}

/// Normalize a URL for dedupe: drop the fragment, lower-case scheme+host,
/// trim a trailing slash — so `HTTP://Host/x/#a` and `http://host/x` are the
/// same ask. Path/query case is preserved (it can be significant).
fn normalize_url(u: &str) -> String {
    let u = u.trim();
    let u = u.split('#').next().unwrap_or(u);
    let (head, rest) = match u.find("://") {
        Some(i) => {
            let after = &u[i + 3..];
            let host_end = after.find('/').unwrap_or(after.len());
            (
                format!("{}://{}", u[..i].to_lowercase(), after[..host_end].to_lowercase()),
                after[host_end..].to_string(),
            )
        }
        None => (String::new(), u.to_string()),
    };
    let out = format!("{}{}", head, rest);
    out.trim_end_matches('/').to_string()
}

/// The normalized TARGET of a proposal input: a `url` string is URL-
/// normalized; anything else falls back to its compact JSON.
fn normalize_input(input: &serde_json::Value) -> String {
    match input.get("url").and_then(|v| v.as_str()) {
        Some(u) => normalize_url(u),
        None => input.to_string(),
    }
}

/// Normalized DETAIL: lower-cased, digits dropped, whitespace collapsed — so
/// "3 items overdue" and "4 items overdue" are one ask (a cascade that grows
/// is the same alert, not a new one every check-up).
fn normalize_detail(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_ascii_digit())
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Dedupe digest over (kind, class, tool, normalized target, normalized
/// detail) — the identity of an ASK, not of a byte-exact payload.
fn fingerprint(kind: &str, class: &str, tool: &str, target: &str, detail: &str) -> u64 {
    let mut h = DefaultHasher::new();
    kind.hash(&mut h);
    class.hash(&mut h);
    tool.hash(&mut h);
    target.hash(&mut h);
    detail.hash(&mut h);
    h.finish()
}

/// Cooldown window for re-proposing an ask that already reached a terminal
/// state (resolved / acknowledged / executed / failed). Env-tunable:
/// `OZONE_PROPOSAL_COOLDOWN_SECS`, default 6h.
fn cooldown_secs() -> i64 {
    std::env::var("OZONE_PROPOSAL_COOLDOWN_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|v| *v >= 0)
        .unwrap_or(REPROPOSE_COOLDOWN_SECS)
}

/// An ask the proposal system may be asked to execute only if it names a
/// whitelisted gated tool; everything else is a notification (R1).
fn normalize_kind(d: &mut ProposalDraft) {
    if d.kind == ProposalKind::Action && !PROPOSABLE_TOOLS.contains(&d.tool.as_str()) {
        d.kind = ProposalKind::Notification;
    }
}

/// Decisions that mean "this proposal reached a terminal state".
fn is_terminal_decision(d: &str) -> bool {
    matches!(d, "acknowledged" | "executed" | "failed")
}

/// What kind of proposal this is. Most are NOTIFICATIONS (operator
/// directive 2026-10-29: "most of the times we may just be notifying and
/// awaiting clarification — such as reminders"): the review row IS the
/// notification, Proceed acknowledges it, and nothing executes. ACTION
/// proposals are the minority that name a concrete gated tool act; only
/// those execute on Proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalKind {
    Notification,
    Action,
}

/// Notification is the DEFAULT kind: reminders and awaiting-clarification
/// items are the common case; an Action must be earned (named gated tool on
/// the whitelist — see `normalize_kind`).
impl Default for ProposalKind {
    fn default() -> Self {
        ProposalKind::Notification
    }
}

impl ProposalKind {
    fn as_str(self) -> &'static str {
        match self {
            ProposalKind::Notification => "notification",
            ProposalKind::Action => "action",
        }
    }
}

/// The derived-proposal shape before persistence.
#[derive(Debug, Clone)]
pub struct ProposalDraft {
    pub kind: ProposalKind,
    pub class: String,
    pub tool: String,
    pub input: serde_json::Value,
    pub rationale: String,
    pub task_id: u64,
}

/// Deterministic proposal rules — pure function of real tasks + findings.
/// One proposal per finding, never more.
pub fn derive_proposals(
    tasks: &[TaskData],
    findings: &[AssistantFinding],
    now: u64,
) -> Vec<ProposalDraft> {
    let _ = now;
    let mut out = Vec::new();
    for f in findings {
        let task = tasks.iter().find(|t| t.task_id == f.task_id);
        match f.class {
            // A meeting with a real link: open it for the operator.
            "meeting-soon" => {
                if let Some(url) = task.and_then(|t| t.meeting_url.clone()) {
                    if url.starts_with("http://") || url.starts_with("https://") {
                        out.push(ProposalDraft {
                            kind: ProposalKind::Notification,
                            class: f.class.to_string(),
                            // Notification, not an action: the review row is
                            // the reminder; the link rides along as context.
                            tool: "browser_open".to_string(),
                            input: serde_json::json!({ "url": url }),
                            rationale: format!(
                                "meeting \"{}\" starts soon — link for reference",
                                f.title.trim_start_matches("Meeting soon: ")
                            ),
                            task_id: f.task_id,
                        });
                    }
                }
            }
            // A slippage cascade: a NOTIFICATION (R1, review 2026-10-29). A
            // headless browser_open cannot make anything visible to the
            // operator, so executing it on Proceed was an action that could
            // never do what its rationale said. The row is the alert; the
            // Order board link rides along for reference; Proceed
            // acknowledges and nothing executes.
            "slippage-cascade" => {
                let base = std::env::var("OZONE_UI_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:8767/".to_string());
                out.push(ProposalDraft {
                    kind: ProposalKind::Notification,
                    class: f.class.to_string(),
                    tool: "browser_open".to_string(),
                    input: serde_json::json!({ "url": base }),
                    rationale: format!(
                        "{} — see the Order board (link for reference)",
                        f.detail
                    ),
                    task_id: f.task_id,
                });
            }
            // An overdue item with an external reference: open the reference.
            "overdue" if f.severity >= 3 => {
                let url = task.and_then(|t| t.external_ref.clone()).and_then(|v| v.as_str().map(String::from));
                if let Some(url) = url {
                    if url.starts_with("http://") || url.starts_with("https://") {
                        out.push(ProposalDraft {
                            kind: ProposalKind::Notification,
                            class: f.class.to_string(),
                            tool: "browser_open".to_string(),
                            input: serde_json::json!({ "url": url }),
                            rationale: format!(
                                "overdue item {} links out — for reference",
                                f.task_id
                            ),
                            task_id: f.task_id,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// After a proposal is resolved (or failed), the same ask may be
/// re-derived by a later check-up — but not on EVERY ripple wake (which
/// would flood the S10 store with duplicate rows). This cooldown bounds
/// re-proposals: one per id per window.
const REPROPOSE_COOLDOWN_SECS: i64 = 6 * 3600;

fn row_ts_secs(row: &serde_json::Value) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(row.get("ts").and_then(|t| t.as_str())?)
        .ok()
        .map(|t| t.timestamp())
}

/// Stable review id for a proposal: kind-prefixed + fingerprint — the same
/// derivation always lands on the same id, so dedupe and resolution key on
/// identity, not on a mutable list.
fn proposal_review_id(d: &ProposalDraft) -> String {
    format!(
        "{}-{}-{:016x}",
        PROPOSAL_REVIEW_KIND,
        d.class,
        fingerprint(
            d.kind.as_str(),
            &d.class,
            &d.tool,
            &normalize_input(&d.input),
            &normalize_detail(&d.rationale),
        )
    )
}

fn append_s10_row(dir: &str, record: serde_json::Value) -> bool {
    let _ = std::fs::create_dir_all(dir);
    let path = format!("{}/decision_review.jsonl", dir);
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut f) => {
            use std::io::Write;
            match serde_json::to_string(&record) {
                Ok(line) => f.write_all(format!("{}\n", line).as_bytes()).is_ok(),
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

fn read_s10_rows(dir: &str) -> Vec<serde_json::Value> {
    let path = format!("{}/decision_review.jsonl", dir);
    match std::fs::read_to_string(&path) {
        Ok(s) => s
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Is this proposal already sitting UNRESOLVED in the S10 store? (Resolved
/// or re-derivable-after-decline proposals may be recorded again — a decline
/// of "open X" does not forever silence the finding; the next check-up may
/// re-propose and the operator may change their mind.)
/// Pure suppression decision (R2): should a draft with this `review_id` be
/// SKIPPED rather than recorded? `rows` is the whole S10 decision_review
/// store in file order; `resolved` = the id appears in the operator
/// resolutions file.
///
/// - An OPEN proposal (the latest relevant row is a pending/review-failed
///   row, no resolution, no terminal row after it) always suppresses.
/// - A proposal that reached ANY terminal state — operator-resolved,
///   acknowledged, executed, or failed — suppresses for the cooldown window
///   measured from the id's most recent row. Previously only the resolutions
///   file was consulted, so acknowledged/failed outcomes that never hit it
///   (or hit a different data dir) were re-recorded on every check-up.
fn suppressed_in(
    rows: &[serde_json::Value],
    review_id: &str,
    resolved: bool,
    now: i64,
    cooldown: i64,
) -> bool {
    let mine: Vec<&serde_json::Value> = rows
        .iter()
        .filter(|r| r.get("review_id").and_then(|v| v.as_str()) == Some(review_id))
        .collect();
    fn dec(r: &serde_json::Value) -> &str {
        r.get("decision").and_then(|d| d.as_str()).unwrap_or("")
    }
    fn is_final(r: &serde_json::Value) -> bool {
        r.get("final").and_then(|f| f.as_bool()) == Some(true)
    }
    let last_pending = mine
        .iter()
        .rposition(|r| is_final(r) && matches!(dec(r), "review_pending" | "review-failed"));
    let last_terminal = mine.iter().rposition(|r| is_final(r) && is_terminal_decision(dec(r)));
    let open = !resolved
        && match (last_pending, last_terminal) {
            (Some(_), None) => true,
            (Some(p), Some(t)) => p > t,
            _ => false,
        };
    if open {
        return true;
    }
    if resolved || last_terminal.is_some() {
        let last = mine.iter().filter_map(|r| row_ts_secs(r)).max();
        return last.map_or(false, |t| now - t < cooldown);
    }
    false
}

/// Is this proposal already open, or still inside its cooldown after a
/// terminal state? Reads the same S10 store the pending route reads.
fn is_open(dir: &str, review_id: &str, resolutions: &std::collections::HashSet<String>) -> bool {
    suppressed_in(
        &read_s10_rows(dir),
        review_id,
        resolutions.contains(review_id),
        chrono::Utc::now().timestamp(),
        cooldown_secs(),
    )
}

fn operator_resolved_ids() -> std::collections::HashSet<String> {
    let path = format!("{}/decision_review_resolutions.jsonl", s10_dir());
    match std::fs::read_to_string(&path) {
        Ok(s) => s
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter_map(|r| {
                r.get("review_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect(),
        Err(_) => std::collections::HashSet::new(),
    }
}

/// Persist new drafts as pending proposal reviews in the S10 store — the
/// EXISTING review system's store, listed by the EXISTING pending route.
/// Deduped while unresolved. Returns how many NEW proposals were recorded.
/// Free — no model call anywhere in this path.
pub fn record_pending(drafts: Vec<ProposalDraft>, notes: &mut Vec<String>) -> usize {
    let resolutions = operator_resolved_ids();
    let mut added = 0usize;
    for mut d in drafts {
        // R1: only a whitelisted gated tool may stay an Action; anything
        // else degrades to a notification BEFORE the id is derived.
        normalize_kind(&mut d);
        let review_id = proposal_review_id(&d);
        if is_open(&s10_dir(), &review_id, &resolutions) {
            continue; // already open — one open proposal per distinct ask
        }
        let payload = serde_json::json!({
            "kind": d.kind.as_str(),
            "tool": d.tool,
            "input": d.input,
            "class": d.class,
            "task_id": d.task_id,
        });
        // The proposal's actionable half rides in reasoning_preview as a
        // JSON object: the resolve side parses it back out of the row. The
        // previews are 500-char-cut; a proposal payload stays well under.
        let record = serde_json::json!({
            "ts": chrono::Utc::now().to_rfc3339(),
            "model_used": "none (deterministic derivation)",
            "tokens_used": 0,
            "decision": "review_pending",
            "confidence": null,
            "task_summary_preview": d.rationale,
            "reasoning_preview": payload.to_string(),
            "raw_response_preview": "",
            "review_id": review_id,
            "review_kind": PROPOSAL_REVIEW_KIND,
            "final": true,
        });
        let mut stamped = record;
        crate::trace::stamp(&mut stamped);
        if append_s10_row(&s10_dir(), stamped) {
            added += 1;
        } else {
            notes.push("proposal S10 row write failed — proposal NOT recorded".to_string());
        }
    }
    if added > 0 {
        notes.push(format!("{} new proposal review(s) recorded", added));
    }
    added
}

/// The review row behind a pending proposal (final + matching id). Reads the
/// same store `list_pending_reviews` reads — one source of truth.
fn latest_row_for(dir: &str, review_id: &str) -> Option<serde_json::Value> {
    // The PROPOSAL row specifically (decision review_pending) — not just any
    // latest final row: the id also accrues acknowledged/executed/failed
    // outcome rows after a resolve, and their reasoning_preview is the
    // outcome, not the action payload.
    read_s10_rows(dir)
        .into_iter()
        .filter(|r| r.get("review_id").and_then(|v| v.as_str()) == Some(review_id))
        .filter(|r| r.get("final").and_then(|f| f.as_bool()) == Some(true))
        .filter(|r| r.get("decision").and_then(|d| d.as_str()) == Some("review_pending"))
        .last()
}

/// The already-recorded outcome of a proposal, if it has one: the id's
/// latest terminal row (acknowledged / executed / failed) that comes AFTER
/// its latest pending row, shaped exactly like a fresh `execute_resolved`
/// answer plus `already_resolved: true`. `None` = never resolved (or
/// re-proposed since, i.e. a newer pending row follows the last outcome).
fn recorded_outcome(rows: &[serde_json::Value], review_id: &str) -> Option<serde_json::Value> {
    let mine: Vec<&serde_json::Value> = rows
        .iter()
        .filter(|r| r.get("review_id").and_then(|v| v.as_str()) == Some(review_id))
        .filter(|r| r.get("final").and_then(|f| f.as_bool()) == Some(true))
        .collect();
    let last_pending = mine
        .iter()
        .rposition(|r| r.get("decision").and_then(|d| d.as_str()) == Some("review_pending"));
    let last_terminal = mine.iter().rposition(|r| {
        r.get("decision")
            .and_then(|d| d.as_str())
            .map_or(false, is_terminal_decision)
    });
    let t = last_terminal?;
    if let Some(p) = last_pending {
        if p > t {
            return None;
        }
    }
    let row = mine[t];
    let status = row.get("decision").and_then(|d| d.as_str()).unwrap_or("").to_string();
    let outcome = row
        .get("reasoning_preview")
        .and_then(|r| r.as_str())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .unwrap_or(serde_json::Value::Null);
    Some(serde_json::json!({
        "review_id": review_id,
        "status": status,
        "outcome": outcome,
        "already_resolved": true,
    }))
}

/// In-process single-flight set for `execute_resolved` (one execution per
/// review id at a time). Released on drop, including on early return/panic.
fn inflight() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    static SET: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::OnceLock::new();
    SET.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
}

struct FlightGuard(String);

impl FlightGuard {
    fn acquire(id: &str) -> Option<FlightGuard> {
        let mut set = inflight().lock().unwrap_or_else(|e| e.into_inner());
        if set.insert(id.to_string()) {
            Some(FlightGuard(id.to_string()))
        } else {
            None
        }
    }
}

impl Drop for FlightGuard {
    fn drop(&mut self) {
        let mut set = inflight().lock().unwrap_or_else(|e| e.into_inner());
        set.remove(&self.0);
    }
}

/// The resolve route's execution half: called when an assistant-proposal
/// review is resolved "Proceed". Parses the action back out of the row,
/// enforces the whitelist, runs it through `call_global` under its own trace
/// root, and appends an honest executed/failed S10 row with the real
/// outcome. Errors are returned for the HTTP response AND recorded.
pub async fn execute_resolved(data_dir: &str, review_id: &str) -> Result<serde_json::Value, String> {
    // The route's config-driven data_dir is authoritative for the resolve
    // side; fall back to the env convention only when callers pass empty.
    let dir = if data_dir.is_empty() { s10_dir() } else { format!("{}/model_calls", data_dir) };
    // R3 — IDEMPOTENT Proceed. A second Proceed (double click, retry, a
    // client replaying the resolve) must neither re-execute the action nor
    // append another outcome row: if the proposal already has a terminal
    // row after its pending row, return that recorded outcome as-is.
    if let Some(prev) = recorded_outcome(&read_s10_rows(&dir), review_id) {
        return Ok(prev);
    }
    // In-process single-flight: two concurrent Proceeds on the same id
    // both pass the check above before either has appended its outcome row.
    let _flight = match FlightGuard::acquire(review_id) {
        Some(g) => g,
        None => {
            return Err(format!(
                "review {} is already being executed — retry to read the recorded outcome",
                review_id
            ))
        }
    };
    // Re-check under the guard (the winner may have finished between the
    // first check and acquiring).
    if let Some(prev) = recorded_outcome(&read_s10_rows(&dir), review_id) {
        return Ok(prev);
    }
    let row = latest_row_for(&dir, review_id)
        .ok_or_else(|| format!("review {} not found in the S10 store", review_id))?;
    let kind = row.get("review_kind").and_then(|k| k.as_str()).unwrap_or("");
    if kind != PROPOSAL_REVIEW_KIND {
        return Err(format!(
            "review {} is kind '{}' — execution applies only to '{}' rows",
            review_id, kind, PROPOSAL_REVIEW_KIND
        ));
    }
    let payload: serde_json::Value = serde_json::from_str(
        row.get("reasoning_preview")
            .and_then(|r| r.as_str())
            .unwrap_or(""),
    )
    .map_err(|e| format!("proposal row payload unreadable: {}", e))?;
    // NOTIFICATION proposals never execute: Proceed is the operator's
    // acknowledgment (the row was the reminder); the resolution row is the
    // record. Refuse loudly if execution is somehow asked anyway.
    if payload.get("kind").and_then(|k| k.as_str()) == Some("notification") {
        let outcome = serde_json::json!({
            "acknowledged_at": now_secs(),
            "note": "notification proposal — acknowledged, nothing to execute",
        });
        append_exec_row(&dir, review_id, "acknowledged", &outcome);
        return Ok(serde_json::json!({
            "review_id": review_id,
            "status": "acknowledged",
            "outcome": outcome,
        }));
    }
    let tool = payload
        .get("tool")
        .and_then(|t| t.as_str())
        .ok_or("proposal row has no tool")?
        .to_string();
    let input = payload.get("input").cloned().unwrap_or(serde_json::json!({}));
    if !PROPOSABLE_TOOLS.contains(&tool.as_str()) {
        let err = format!("tool '{}' is not on the proposable whitelist", tool);
        append_exec_row(&dir, review_id, "failed", &serde_json::json!({ "error": err }));
        return Err(err);
    }
    let call = crate::mcp::McpCall {
        tool,
        // Ledger honesty: the actor is the operator's approval of an
        // assistant proposal — the assistant itself never calls tools
        // (ACTING_FACTORS_REGISTRY §0).
        agent: "assistant-proposal (operator-approved)".to_string(),
        input,
        context: Some(serde_json::json!({
            "source": "assistant-proposal",
            "proposal_review": review_id,
            "approved_by": "operator",
        })),
        session_token: None,
    };
    let ctx = crate::trace::TraceCtx::new_root("assistant-proposal-exec");
    let result = crate::trace::with_trace(ctx, async { crate::mcp::call_global(call).await }).await;
    let outcome = match &result {
        r if r.success => serde_json::json!({
            "executed_at": now_secs(),
            "output": r.output,
            "usage": r.usage,
        }),
        r => serde_json::json!({
            "failed_at": now_secs(),
            "error": r.error,
        }),
    };
    let status = if result.success { "executed" } else { "failed" };
    append_exec_row(&dir, review_id, status, &outcome);
    Ok(serde_json::json!({
        "review_id": review_id,
        "status": status,
        "outcome": outcome,
    }))
}

/// The honest follow-up row: `executed` or `failed` with the REAL outcome —
/// never a fabricated success.
fn append_exec_row(dir: &str, review_id: &str, status: &str, outcome: &serde_json::Value) {
    let record = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "model_used": "none (gated tool execution)",
        "tokens_used": 0,
        "decision": status,
        "confidence": null,
        "task_summary_preview": format!("proposal {} {}", review_id, status),
        "reasoning_preview": outcome.to_string(),
        "raw_response_preview": "",
        "review_id": review_id,
        "review_kind": PROPOSAL_REVIEW_KIND,
        "final": true,
    });
    let mut stamped = record;
    crate::trace::stamp(&mut stamped);
    let _ = append_s10_row(dir, stamped);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: u64) -> TaskData {
        TaskData {
            task_id: id,
            blueprint_id: None,
            status: "running".into(),
            progress: 0.5,
            created_at: 0,
            started_at: Some(0),
            completed_at: None,
            user_id: 0,
            device_id: 0,
            workspace_id: Some(1),
            project_id: Some(3),
            parent_task_id: None,
            child_count: 0,
            error: None,
            inputs: Some(serde_json::json!({"name": format!("item {}", id)})),
            outputs: None,
            steps: vec![],
            total_tokens: 0,
            gate_result: None,
            thinking_log: vec![],
            amt_summary: None,
            kind: "todo".into(),
            due_at: None,
            remind_at: None,
            recurrence: None,
            meeting_url: None,
            external_ref: None,
            checklist: vec![],
            note_body: None,
        }
    }

    fn finding(class: &'static str, severity: u8, task_id: u64) -> AssistantFinding {
        AssistantFinding {
            class: class,
            severity,
            title: format!("{} on task {}", class, task_id),
            detail: format!("{} detail", class),
            task_id,
            project_id: Some(3),
            workspace_id: Some(1),
            due_at: None,
        }
    }

    #[test]
    fn meeting_with_url_proposes_open() {
        let now = now_secs();
        let mut t = task(7);
        t.kind = "meeting".into();
        t.remind_at = Some(now + 1800);
        t.meeting_url = Some("https://meet.example.com/x".into());
        let findings = crate::consciousness::assistant::compute_findings(&[t.clone()], now);
        let props = derive_proposals(&[t], &findings, now);
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].tool, "browser_open");
        assert_eq!(props[0].input["url"], "https://meet.example.com/x");
    }

    #[test]
    fn meeting_without_url_proposes_nothing() {
        let now = now_secs();
        let mut t = task(8);
        t.kind = "meeting".into();
        t.remind_at = Some(now + 1800);
        let findings = crate::consciousness::assistant::compute_findings(&[t.clone()], now);
        assert!(derive_proposals(&[t], &findings, now).is_empty());
    }

    #[test]
    fn overdue_with_external_ref_proposes_open() {
        let now = now_secs();
        let mut t = task(9);
        t.due_at = Some(now - 3 * 24 * 3600);
        t.external_ref = Some("https://tracker.example.com/9".into());
        let findings = crate::consciousness::assistant::compute_findings(&[t.clone()], now);
        let props = derive_proposals(&[t], &findings, now);
        assert!(props
            .iter()
            .any(|p| p.tool == "browser_open" && p.input["url"] == "https://tracker.example.com/9"));
    }

    #[test]
    fn cascade_proposes_the_order_board() {
        let now = now_secs();
        let tasks: Vec<TaskData> = (10..13)
            .map(|i| {
                let mut t = task(i);
                t.due_at = Some(now - 2 * 3600);
                t
            })
            .collect();
        let findings = crate::consciousness::assistant::compute_findings(&tasks, now);
        let props = derive_proposals(&tasks, &findings, now);
        assert!(props.iter().any(|p| p.class == "slippage-cascade"));
    }

    #[test]
    fn whitelist_rejects_unknown_tools() {
        assert!(!PROPOSABLE_TOOLS.contains(&"shell_exec"));
        assert!(PROPOSABLE_TOOLS.contains(&"browser_open"));
    }

    #[test]
    fn notifications_carry_kind_and_url_for_reference() {
        let now = now_secs();
        let mut t = task(21);
        t.kind = "meeting".into();
        t.remind_at = Some(now + 1800);
        t.meeting_url = Some("https://meet.example.com/y".into());
        let findings = crate::consciousness::assistant::compute_findings(&[t.clone()], now);
        let props = derive_proposals(&[t], &findings, now);
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].kind, ProposalKind::Notification);
        // R1: the cascade is a NOTIFICATION too — a headless browser_open
        // could never make the Order board visible, so it must not execute.
        let tasks: Vec<TaskData> = (30..33)
            .map(|i| {
                let mut t = task(i);
                t.due_at = Some(now - 2 * 3600);
                t
            })
            .collect();
        let findings = crate::consciousness::assistant::compute_findings(&tasks, now);
        let props = derive_proposals(&tasks, &findings, now);
        let cascade: Vec<_> = props.iter().filter(|p| p.class == "slippage-cascade").collect();
        assert!(!cascade.is_empty());
        assert!(cascade.iter().all(|p| p.kind == ProposalKind::Notification));
    }

    #[test]
    fn derive_never_emits_an_action_today() {
        let now = now_secs();
        let tasks: Vec<TaskData> = (40..44)
            .map(|i| {
                let mut t = task(i);
                t.due_at = Some(now - 3 * 24 * 3600);
                t.external_ref = Some(format!("https://tracker.example.com/{}", i).into());
                t
            })
            .collect();
        let findings = crate::consciousness::assistant::compute_findings(&tasks, now);
        let props = derive_proposals(&tasks, &findings, now);
        assert!(!props.is_empty());
        assert!(props.iter().all(|p| p.kind == ProposalKind::Notification));
    }

    #[test]
    fn default_kind_is_notification() {
        assert_eq!(ProposalKind::default(), ProposalKind::Notification);
    }

    #[test]
    fn farm_command_is_not_proposable() {
        assert!(!PROPOSABLE_TOOLS.contains(&"farm_command"));
    }

    #[test]
    fn action_on_a_non_whitelisted_tool_degrades_to_notification() {
        let mut d = ProposalDraft {
            kind: ProposalKind::Action,
            class: "x".into(),
            tool: "farm_command".into(),
            input: serde_json::json!({}),
            rationale: "r".into(),
            task_id: 1,
        };
        normalize_kind(&mut d);
        assert_eq!(d.kind, ProposalKind::Notification);
        let mut ok = ProposalDraft { tool: "browser_open".into(), kind: ProposalKind::Action, ..d.clone() };
        normalize_kind(&mut ok);
        assert_eq!(ok.kind, ProposalKind::Action);
    }

    #[test]
    fn review_id_ignores_url_noise_and_digit_churn() {
        let a = ProposalDraft {
            kind: ProposalKind::Notification,
            class: "slippage-cascade".into(),
            tool: "browser_open".into(),
            input: serde_json::json!({"url": "HTTP://Localhost:8767/#board"}),
            rationale: "3 items slipped — see the Order board".into(),
            task_id: 1,
        };
        let b = ProposalDraft {
            input: serde_json::json!({"url": "http://localhost:8767"}),
            rationale: "4 items slipped — see the Order board".into(),
            ..a.clone()
        };
        assert_eq!(proposal_review_id(&a), proposal_review_id(&b));
        // kind is part of the identity
        let c = ProposalDraft { kind: ProposalKind::Action, ..a.clone() };
        assert_ne!(proposal_review_id(&a), proposal_review_id(&c));
    }

    fn row(id: &str, decision: &str, ts_secs_ago: i64, now: i64, preview: &str) -> serde_json::Value {
        let ts = chrono::DateTime::<chrono::Utc>::from_timestamp(now - ts_secs_ago, 0)
            .unwrap()
            .to_rfc3339();
        serde_json::json!({
            "ts": ts,
            "decision": decision,
            "review_id": id,
            "review_kind": PROPOSAL_REVIEW_KIND,
            "reasoning_preview": preview,
            "final": true,
        })
    }

    #[test]
    fn open_proposal_suppresses_redraft() {
        let now = 1_800_000_000;
        let rows = vec![row("p1", "review_pending", 60, now, "{}")];
        assert!(suppressed_in(&rows, "p1", false, now, 6 * 3600));
        assert!(!suppressed_in(&rows, "other", false, now, 6 * 3600));
    }

    #[test]
    fn acknowledged_proposal_is_not_rerecorded_within_cooldown_even_if_unlisted() {
        let now = 1_800_000_000;
        // acknowledged row exists but the id never reached the resolutions
        // file (resolved=false): the terminal row alone must suppress.
        let rows = vec![
            row("p1", "review_pending", 600, now, "{}"),
            row("p1", "acknowledged", 300, now, "{\"note\":\"n\"}"),
        ];
        assert!(suppressed_in(&rows, "p1", false, now, 6 * 3600));
    }

    #[test]
    fn terminal_proposal_may_return_after_cooldown() {
        let now = 1_800_000_000;
        let rows = vec![
            row("p1", "review_pending", 8 * 3600, now, "{}"),
            row("p1", "failed", 7 * 3600, now, "{\"error\":\"e\"}"),
        ];
        assert!(!suppressed_in(&rows, "p1", false, now, 6 * 3600));
        assert!(!suppressed_in(&rows, "p1", true, now, 6 * 3600));
        // inside the window it is held back
        assert!(suppressed_in(&rows, "p1", true, now, 8 * 3600));
    }

    #[test]
    fn newer_pending_after_outcome_is_open_again() {
        let now = 1_800_000_000;
        let rows = vec![
            row("p1", "review_pending", 20 * 3600, now, "{}"),
            row("p1", "executed", 19 * 3600, now, "{}"),
            row("p1", "review_pending", 60, now, "{}"),
        ];
        assert!(suppressed_in(&rows, "p1", false, now, 6 * 3600));
    }

    #[test]
    fn zero_cooldown_never_holds_back_a_terminal_ask() {
        let now = 1_800_000_000;
        let rows = vec![
            row("p1", "review_pending", 10, now, "{}"),
            row("p1", "acknowledged", 5, now, "{}"),
        ];
        assert!(!suppressed_in(&rows, "p1", true, now, 0));
    }

    #[test]
    fn proceed_is_idempotent_returns_recorded_outcome() {
        let now = 1_800_000_000;
        let rows = vec![
            row("p1", "review_pending", 100, now, "{\"tool\":\"browser_open\"}"),
            row("p1", "failed", 50, now, "{\"error\":\"connection refused\"}"),
        ];
        let prev = recorded_outcome(&rows, "p1").expect("recorded");
        assert_eq!(prev["status"], "failed");
        assert_eq!(prev["already_resolved"], true);
        assert_eq!(prev["outcome"]["error"], "connection refused");
        // never-resolved and re-proposed ids have no recorded outcome
        assert!(recorded_outcome(&rows, "nope").is_none());
        let reopened = vec![
            row("p1", "review_pending", 100, now, "{}"),
            row("p1", "executed", 50, now, "{}"),
            row("p1", "review_pending", 5, now, "{}"),
        ];
        assert!(recorded_outcome(&reopened, "p1").is_none());
    }

    #[test]
    fn flight_guard_is_single_flight_and_releases() {
        let g = FlightGuard::acquire("flight-test-id").expect("first acquire");
        assert!(FlightGuard::acquire("flight-test-id").is_none());
        drop(g);
        assert!(FlightGuard::acquire("flight-test-id").is_some());
    }

    #[test]
    fn review_id_is_stable_per_ask() {
        let d1 = ProposalDraft {
            kind: ProposalKind::Notification,
            class: "meeting-soon".into(),
            tool: "browser_open".into(),
            input: serde_json::json!({"url": "https://m"}),
            rationale: "r".into(),
            task_id: 1,
        };
        let d2 = d1.clone();
        assert_eq!(proposal_review_id(&d1), proposal_review_id(&d2));
        let d3 = ProposalDraft {
            input: serde_json::json!({"url": "https://other"}),
            ..d1.clone()
        };
        assert_ne!(proposal_review_id(&d1), proposal_review_id(&d3));
        assert!(proposal_review_id(&d1).starts_with(PROPOSAL_REVIEW_KIND));
    }
}
