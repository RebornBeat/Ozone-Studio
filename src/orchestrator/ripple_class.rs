//! RIPPLE CATEGORIES + THE META RIPPLE (2026-11-04) — docs/RIPPLE_CATEGORIES_AND_META_RIPPLE_2026-11-04.md.
//!
//! The graph ripple (`graph_events`) already tells the system everything that changes. The meta loops
//! (I-Loop, methodology meta-loop, assistant, AMT re-expansion) used to run on a 30-minute TIMER; they
//! should run on EVENTS. This module:
//!
//!   1. [`classify`] — a pure function from a ripple event to a [`RippleCategory`] and whether it is a
//!      META ripple (an event the meta loops exist to react to);
//!   2. [`spawn_meta_ripple_dispatcher`] — one subscriber; a meta event wakes every meta loop;
//!   3. [`LoopGate`] — the per-loop wait: wake on a meta ripple OR a long heartbeat, with a MINIMUM GAP
//!      between paid passes (events inside the gap coalesce into ONE pass — this, not a fixed schedule,
//!      is what keeps the ~1,000-calls/day budget guarantee the old 1800 s clamp provided);
//!   4. echo protection — events written by a meta loop's OWN pass (same trace id) never wake anything.
//!
//! Loops keep their own change detection (I-Loop signal fingerprint, assistant digest fingerprint), so a
//! wake with nothing new is a free skip.

use crate::graph_events::GraphEvent;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tokio::sync::Notify;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum RippleCategory {
    /// File/graph/document content (FileReference, ModalityGraph, Document, Code*, Derived …).
    Content,
    /// Workspace / Project / roots / categories — scoping skeleton.
    Structure,
    /// Task / order transitions.
    Task,
    /// Multi-agent coordination bookkeeping (CoordinationEvent …) — echo-prone, never meta by itself.
    Coordination,
    /// Tool results (McpResult / McpEntity) — the actors turn real findings into `Finding` events.
    SecurityMcp,
    /// Experience / memory / identity / narrative — what reflection is ABOUT.
    Consciousness,
    /// Methodology / blueprint / pipeline registrations and the methodology GAP signal.
    Methodology,
    /// A ripple actor's real finding (security, coordination, task-failed, code-staleness, mcp-failure).
    Finding,
    /// The loops' own writes and bookkeeping (AMTExpansion, chat, context trims) — must NOT re-trigger.
    LoopInternal,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct RippleClass {
    pub category: RippleCategory,
    /// True when the meta loops should wake for this event.
    pub meta: bool,
}

/// Container type used for explicit finding events (see [`emit_finding`]).
pub const FINDING_TYPE: &str = "RippleFinding";
/// Container type used for the methodology-gap signal (see [`emit_methodology_gap`]).
pub const GAP_TYPE: &str = "MethodologyGap";
/// Container type used for the missing-capability signal (see [`emit_capability_gap`]).
pub const CAPABILITY_GAP_TYPE: &str = "CapabilityGap";

fn is_root_like(t: &str) -> bool {
    t.ends_with("Root") || matches!(t, "Root" | "User" | "Workspace" | "Project" | "Category" | "SubCategory" | "Modality" | "Virtual")
}

/// Classify one ripple event. Pure; no I/O.
pub fn classify(evt: &GraphEvent) -> RippleClass {
    use RippleCategory::*;
    let t = evt.container_type.as_str();
    let rc = |category, meta| RippleClass { category, meta };

    // --- echo first: the loops' own writes and bookkeeping never wake anything ---
    if evt.event == "context_trimmed" || t.starts_with("trim:") || evt.source == "amt-expansion" || evt.source == "context" {
        return rc(LoopInternal, false);
    }
    if matches!(t, "AMTExpansion" | "ChatSession" | "ChatTurn") {
        return rc(LoopInternal, false);
    }
    // --- explicit signals ---
    if t == FINDING_TYPE {
        return rc(Finding, true);
    }
    if t == GAP_TYPE || t == CAPABILITY_GAP_TYPE {
        return rc(Methodology, true);
    }
    // --- coordination bookkeeping (71 % of candidates before the echo filter) ---
    if t == "CoordinationEvent" || t.starts_with("Coordination") || t == "SharedContext" {
        return rc(Coordination, false);
    }
    // --- order ---
    if matches!(t, "Task" | "TaskContext" | "TaskExecutionState") {
        return rc(Task, true);
    }
    // --- tool results (noisy per call; real findings arrive as Finding events) ---
    if matches!(t, "McpResult" | "McpEntity") {
        return rc(SecurityMcp, false);
    }
    // --- consciousness: what the I-Loop reflects on ---
    if !is_root_like(t)
        && matches!(
            t,
            "ExperienceMemory" | "CoreMemory" | "EmotionalContext" | "IdentityState" | "Relationship" | "EthicalPrinciple" | "Narrative"
                | "CollectiveWisdom" | "Experience" | "EmotionalStateRecord" | "VoiceIdentity"
        )
    {
        return rc(Consciousness, true);
    }
    // --- methodology registrations (the meta-loop WRITES these → never meta, avoids a self-echo) ---
    if matches!(t, "Methodology" | "Blueprint" | "Pipeline" | "MethodologyRoot" | "BlueprintRoot" | "PipelineRoot") {
        return rc(Methodology, false);
    }
    if is_root_like(t) {
        return rc(Structure, false);
    }
    if matches!(
        t,
        "FileReference" | "DirectoryReference" | "URLReference" | "PackageReference" | "ModalityGraph" | "Dataset" | "Shard" | "Document" | "Chunk"
            | "Embedding" | "Derived" | "CodeModule" | "CodeFunction" | "CodeClass" | "CodeDependency" | "TextDocument" | "TextSection"
            | "TextParagraph" | "TextTheme" | "FileRef"
    ) {
        return rc(Content, false);
    }
    rc(Other, false)
}

// ---------------------------------------------------------------------------------------------
// Explicit meta signals (called by the producers that know a real finding / gap happened)
// ---------------------------------------------------------------------------------------------

/// A ripple actor queued a REAL finding (route = SecurityFinding | CoordinationReceived | TaskFailure |
/// CodeStaleness | …). This is the meta ripple for security / mcp / task-failed / staleness — raw tool
/// results are far too noisy to be meta, the actor's verdict is exactly the signal.
pub fn emit_finding(route: &str) {
    crate::graph_events::emit("created", 0, 0, FINDING_TYPE.to_string(), "ripple-actor", vec!["scope:global".to_string(), format!("finding:{route}")]);
}

/// A user request matched no methodology (`record_methodology_gap`) — the meta-loop's real trigger.
pub fn emit_methodology_gap() {
    crate::graph_events::emit("created", 0, 0, GAP_TYPE.to_string(), "methodology-gap", vec!["scope:global".to_string()]);
}

/// A required tool/MCP/capability is NOT available (blueprint / simulation / methodology binding). The meta loops wake:
/// the meta-loop drafts a methodology around it, the assistant surfaces it, the Marketplace lists it as WANTED.
pub fn emit_capability_gap() {
    crate::graph_events::emit("created", 0, 0, CAPABILITY_GAP_TYPE.to_string(), "capability-gap", vec!["scope:global".to_string()]);
}

// ---------------------------------------------------------------------------------------------
// Echo protection: a loop pass registers its trace id; events carrying it never wake the loops.
// ---------------------------------------------------------------------------------------------

fn active_passes() -> &'static Mutex<HashSet<String>> {
    static ACTIVE: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashSet::new()))
}

pub struct PassGuard(String);
impl Drop for PassGuard {
    fn drop(&mut self) {
        if let Ok(mut s) = active_passes().lock() {
            s.remove(&self.0);
        }
    }
}
/// Register a meta-loop pass's trace id for its duration (hold the guard across the pass).
pub fn pass_guard(trace_id: &str) -> PassGuard {
    if let Ok(mut s) = active_passes().lock() {
        s.insert(trace_id.to_string());
    }
    PassGuard(trace_id.to_string())
}
fn is_own_echo(evt: &GraphEvent) -> bool {
    evt.trace_id.as_ref().map(|t| active_passes().lock().map(|s| s.contains(t)).unwrap_or(false)).unwrap_or(false)
}

// ---------------------------------------------------------------------------------------------
// Per-loop gate: wake on meta ripple OR heartbeat, with a minimum gap between passes.
// ---------------------------------------------------------------------------------------------

fn env_secs(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
/// Liveness safety net only (NOT a schedule): default 6 h.
pub fn heartbeat_secs() -> u64 {
    env_secs("OZONE_META_HEARTBEAT_SECS", 6 * 3600)
}
/// Minimum seconds between paid passes of the I-Loop / meta-loop (default 10 min).
pub fn min_gap_secs() -> u64 {
    env_secs("OZONE_META_MIN_GAP_SECS", 600)
}
/// Minimum seconds between assistant check-ups (default 5 min).
pub fn assistant_min_gap_secs() -> u64 {
    env_secs("OZONE_ASSISTANT_MIN_GAP_SECS", 300)
}
const STARTUP_CATCHUP_SECS: u64 = 60;

pub struct LoopGate {
    notify: Notify,
    last_pass_end: AtomicU64,
    started: std::sync::atomic::AtomicBool,
    /// How many times a meta ripple woke this loop / how many passes it has finished (status route + E2E evidence).
    wakes: AtomicU64,
    passes: AtomicU64,
    last_wake: AtomicU64,
}

static META_EVENTS: AtomicU64 = AtomicU64::new(0);
static ECHO_IGNORED: AtomicU64 = AtomicU64::new(0);
static EVENTS_SEEN: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// First pass after boot — catches signals that accumulated while the host was down.
    Startup,
    /// A meta ripple woke the loop.
    MetaRipple,
    /// The long liveness heartbeat elapsed with no event.
    Heartbeat,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl LoopGate {
    const fn new() -> Self {
        Self {
            notify: Notify::const_new(),
            last_pass_end: AtomicU64::new(0),
            started: std::sync::atomic::AtomicBool::new(false),
            wakes: AtomicU64::new(0),
            passes: AtomicU64::new(0),
            last_wake: AtomicU64::new(0),
        }
    }
    pub fn wake(&self) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
        self.last_wake.store(now_secs(), Ordering::SeqCst);
        self.notify.notify_one();
    }
    /// Call when a pass finishes — starts the minimum-gap clock.
    pub fn pass_finished(&self) {
        self.passes.fetch_add(1, Ordering::SeqCst);
        self.last_pass_end.store(now_secs(), Ordering::SeqCst);
    }
    fn status(&self) -> serde_json::Value {
        serde_json::json!({
            "wakes": self.wakes.load(Ordering::SeqCst),
            "passes": self.passes.load(Ordering::SeqCst),
            "last_wake": self.last_wake.load(Ordering::SeqCst),
            "last_pass_end": self.last_pass_end.load(Ordering::SeqCst),
        })
    }
    /// Wait for the next reason to run a pass. Events arriving inside the minimum gap coalesce
    /// (one stored permit is drained after the gap, so the pass that follows covers them all).
    pub async fn wait_next(&self, heartbeat: u64, min_gap: u64) -> Trigger {
        if !self.started.swap(true, Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_secs(STARTUP_CATCHUP_SECS)).await;
            return Trigger::Startup;
        }
        let trigger = tokio::select! {
            _ = self.notify.notified() => Trigger::MetaRipple,
            _ = tokio::time::sleep(std::time::Duration::from_secs(heartbeat)) => Trigger::Heartbeat,
        };
        let since = now_secs().saturating_sub(self.last_pass_end.load(Ordering::SeqCst));
        if since < min_gap {
            tokio::time::sleep(std::time::Duration::from_secs(min_gap - since)).await;
        }
        // drain any permit that accumulated during the gap: the pass we are about to run covers it
        let _ = tokio::time::timeout(std::time::Duration::ZERO, self.notify.notified()).await;
        trigger
    }
}

pub fn i_loop_gate() -> &'static LoopGate {
    static G: LoopGate = LoopGate::new();
    &G
}
pub fn meta_loop_gate() -> &'static LoopGate {
    static G: LoopGate = LoopGate::new();
    &G
}
pub fn assistant_gate() -> &'static LoopGate {
    static G: LoopGate = LoopGate::new();
    &G
}

/// Live status of the meta ripple (GET /ripple/meta/status): counters + per-loop wakes/passes + the policy values.
pub fn status_json() -> serde_json::Value {
    serde_json::json!({
        "events_seen": EVENTS_SEEN.load(Ordering::SeqCst),
        "meta_events": META_EVENTS.load(Ordering::SeqCst),
        "own_pass_echoes_ignored": ECHO_IGNORED.load(Ordering::SeqCst),
        "policy": { "heartbeat_secs": heartbeat_secs(), "min_gap_secs": min_gap_secs(), "assistant_min_gap_secs": assistant_min_gap_secs(), "startup_catchup_secs": STARTUP_CATCHUP_SECS },
        "loops": { "i_loop": i_loop_gate().status(), "meta_loop": meta_loop_gate().status(), "assistant": assistant_gate().status() },
        "categories": ["Content","Structure","Task","Coordination","SecurityMcp","Consciousness","Methodology","Finding","LoopInternal","Other"],
    })
}

/// Wake every meta loop (used by the dispatcher; also callable by a producer that knows better).
pub fn wake_all_meta_loops() {
    i_loop_gate().wake();
    meta_loop_gate().wake();
    assistant_gate().wake();
    crate::orchestrator::amt_loop::notify_wake();
}

/// One subscriber to the ripple: a meta event (not an echo of a loop's own pass) wakes all meta loops.
pub fn spawn_meta_ripple_dispatcher() {
    tokio::spawn(async move {
        let mut rx = crate::graph_events::GraphEventHub::global().subscribe();
        loop {
            match rx.recv().await {
                Ok(evt) => {
                    EVENTS_SEEN.fetch_add(1, Ordering::SeqCst);
                    let c = classify(&evt);
                    if c.meta && is_own_echo(&evt) {
                        ECHO_IGNORED.fetch_add(1, Ordering::SeqCst);
                    }
                    if c.meta && !is_own_echo(&evt) {
                        META_EVENTS.fetch_add(1, Ordering::SeqCst);
                        tracing::debug!(category = ?c.category, container_type = %evt.container_type, source = %evt.source, "meta ripple — waking the meta loops");
                        wake_all_meta_loops();
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    // a lagged ring may have dropped a meta event — wake once; change detection makes it free
                    tracing::warn!(lagged = n, "meta ripple dispatcher lagged — waking the meta loops once");
                    wake_all_meta_loops();
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
    tracing::info!("meta ripple dispatcher started (I-Loop, methodology meta-loop, assistant and AMT loop wake on meta events)");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evt(event: &'static str, ct: &str, source: &str) -> GraphEvent {
        GraphEvent {
            event,
            container_id: 1,
            parent_id: 0,
            container_type: ct.to_string(),
            source: source.to_string(),
            scope_keywords: vec![],
            timestamp: 0,
            trace_id: None,
            span_id: None,
        }
    }

    #[test]
    fn meta_events_are_the_ones_the_loops_exist_for() {
        for (e, ct, src, cat) in [
            ("updated", "Task", "task-system", RippleCategory::Task),
            ("created", "Experience", "zsei", RippleCategory::Consciousness),
            ("created", FINDING_TYPE, "ripple-actor", RippleCategory::Finding),
            ("created", GAP_TYPE, "methodology-gap", RippleCategory::Methodology),
        ] {
            let c = classify(&evt(e, ct, src));
            assert!(c.meta, "{ct} must be meta");
            assert_eq!(c.category, cat);
        }
    }

    #[test]
    fn echo_and_bookkeeping_never_wake_the_loops() {
        for (e, ct, src) in [
            ("updated", "AMTExpansion", "amt-expansion"),
            ("created", "ChatTurn", "zsei"),
            ("created", "ChatSession", "zsei"),
            ("context_trimmed", "trim:handled-outcomes", "context"),
            ("created", "CoordinationEvent", "zsei"),
            ("created", "McpResult", "zsei"),
            ("created", "Methodology", "zsei"), // the meta-loop's own output must not wake it
            ("created", "FileReference", "zsei"),
            ("created", "ModalityGraph", "zsei"),
            ("created", "Workspace", "zsei"),
        ] {
            assert!(!classify(&evt(e, ct, src)).meta, "{ct} must NOT be meta");
        }
    }

    #[test]
    fn a_loops_own_pass_events_are_ignored() {
        let mut e = evt("created", "Experience", "zsei");
        e.trace_id = Some("trace-own-pass".into());
        assert!(!is_own_echo(&e));
        let g = pass_guard("trace-own-pass");
        assert!(is_own_echo(&e));
        drop(g);
        assert!(!is_own_echo(&e));
    }

    /// Every ContainerType display name lands in a real category (nothing silently `Other`).
    #[test]
    fn every_container_type_name_is_categorised() {
        let src = include_str!("../types/container.rs");
        let start = src.find("pub fn display_name(&self)").expect("display_name");
        let body = &src[start..];
        let end = body.find("\n    }\n").unwrap_or(body.len());
        let mut names: Vec<String> = Vec::new();
        for line in body[..end].lines() {
            if let (Some(a), Some(b)) = (line.find("=> \""), line.rfind("\",")) {
                if b > a + 4 {
                    names.push(line[a + 4..b].to_string());
                }
            }
        }
        assert!(names.len() > 40, "parsed {} names", names.len());
        let others: Vec<&String> = names.iter().filter(|n| classify(&evt("created", n, "zsei")).category == RippleCategory::Other).collect();
        // Domain-specific container types (per-modality graph nodes etc.) may stay Other; the skeleton must not.
        let skeleton_other = others.iter().filter(|n| n.starts_with("Code") || n.starts_with("Text") || n.ends_with("Root") || n.starts_with("Task")).count();
        assert_eq!(skeleton_other, 0, "uncategorised skeleton types: {:?}", others);
    }

    #[tokio::test]
    async fn gate_coalesces_a_burst_into_one_pass_and_enforces_the_gap() {
        static G: LoopGate = LoopGate::new();
        // consume the startup catch-up without waiting 60 s
        G.started.store(true, Ordering::SeqCst);
        G.pass_finished();
        for _ in 0..20 {
            G.wake(); // a burst of 20 meta events
        }
        let t0 = std::time::Instant::now();
        let trig = G.wait_next(3600, 1).await; // 1 s gap
        assert_eq!(trig, Trigger::MetaRipple);
        assert!(t0.elapsed() >= std::time::Duration::from_millis(900), "gap enforced");
        // the burst was coalesced: no stored permit remains, so the next wait is a heartbeat, not an instant re-run
        let t1 = std::time::Instant::now();
        let again = tokio::time::timeout(std::time::Duration::from_millis(300), G.wait_next(3600, 1)).await;
        assert!(again.is_err(), "no extra pass from the same burst ({:?})", t1.elapsed());
    }
}
