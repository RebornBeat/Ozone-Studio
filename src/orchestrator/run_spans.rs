//! Run / span containers for the background loops (docs: trace spine design,
//! CHECKLIST 2026-10-23 "FINALIZED DESIGN REVIEW").
//!
//! One long-lived **Run root** container per loop (`amt`, `meta`, `i_loop`,
//! `assistant`) under the consciousness metacognition root, and ONE **span**
//! container per loop PASS (never per call) chained to the previous pass of the
//! same loop. Together they are the per-loop index of "what did this loop do,
//! when, under which trace id, consuming which candidates" that capture rows
//! (ledger / S11 / ContextRecord, all stamped with the trace id) point back to.
//!
//! FEEDBACK-LOOP SAFETY (verified by reading `zsei/mod.rs::ripple_info`,
//! `amt_loop::process_graph_event`, `actors.rs`): every container write emits a
//! `GraphEvent` whose scope keywords are the container's own keywords.
//! - The AMT ripple sync only converts events whose scope has `proj:<id>` or
//!   `file:<path>`; these containers never carry either.
//! - The ripple actors only react to `CoordinationEvent` (keywords `tool_call`
//!   / `finding`), `Task` and `FileReference` containers; these containers are
//!   `Derived` (same type the consciousness insights use) so no actor is even
//!   woken, and they carry neither keyword.
//! - `trace:<id>` / `loop:<name>` / `run-span` / `run-root` are interpreted by
//!   nothing in the host; they exist so the graph can be searched by trace id.
//!
//! Everything here is best-effort by contract: a failure logs and returns
//! `None`; it never panics and never fails the pass that called it.

use crate::orchestrator::StoreAccess;
use crate::trace::TraceCtx;
use crate::types::container::{
    CompressionType, Container, ContainerType, Context, DiscoveryMethod, GlobalState,
    IntegrityData, LocalState, Metadata, Modality, Relation, RelationType, StoragePointers,
    TraversalHints, CONSCIOUSNESS_METACOGNITION_ROOT_ID,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// What one loop pass did — the counters that go into its span body.
#[derive(Debug, Clone, Default)]
pub struct PassStats {
    /// Candidates / gaps / signals the pass looked at.
    pub candidates_consumed: u64,
    /// Model calls actually made.
    pub calls_made: u64,
    /// Calls (or candidates) that produced a usable outcome.
    pub successes: u64,
    /// Calls (or candidates) that failed this pass.
    pub failures: u64,
    /// Container ids of the candidates handled (AMT containers etc.).
    pub candidate_container_ids: Vec<u64>,
    /// Free-form honest notes ("per-pass bound hit; 4 left for next tick").
    pub notes: Vec<String>,
}

impl PassStats {
    /// Whether the pass did anything worth a span. Idle passes (nothing to
    /// consume, no call made) are logged at debug and not recorded — an idle
    /// loop must not write ~48 empty containers a day.
    pub fn did_work(&self) -> bool {
        self.candidates_consumed > 0 || self.calls_made > 0
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "candidates_consumed": self.candidates_consumed,
            "calls_made": self.calls_made,
            "successes": self.successes,
            "failures": self.failures,
            "candidate_container_ids": self.candidate_container_ids,
            "notes": self.notes,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RunState {
    root_id: u64,
    #[serde(default)]
    last_span_id: Option<u64>,
}

static RUNS: OnceLock<Mutex<HashMap<String, RunState>>> = OnceLock::new();

fn data_dir() -> String {
    std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string())
}

fn state_path() -> std::path::PathBuf {
    std::path::PathBuf::from(data_dir()).join("consciousness/run_roots.json")
}

fn load_persisted() -> HashMap<String, RunState> {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn persist(map: &HashMap<String, RunState>) {
    let path = state_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(map) {
        // temp + rename: a crash mid-write must not leave a truncated map.
        let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

fn runs() -> &'static Mutex<HashMap<String, RunState>> {
    RUNS.get_or_init(|| Mutex::new(load_persisted()))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Write a JSON body next to the other consciousness bodies and return its
/// data-dir-relative path (the codebase's content-pointer convention).
fn write_body(rel: &str, body: &serde_json::Value) -> Option<String> {
    let abs = std::path::PathBuf::from(data_dir()).join(rel);
    if let Some(dir) = abs.parent() {
        std::fs::create_dir_all(dir).ok()?;
    }
    let json = serde_json::to_string_pretty(body).ok()?;
    std::fs::write(&abs, json).ok()?;
    Some(rel.to_string())
}

fn build_container(
    parent_id: u64,
    name: String,
    materialized_path: String,
    provenance: &str,
    keywords: Vec<String>,
    relationships: Vec<Relation>,
    object_store_path: Option<String>,
    now: u64,
) -> Container {
    Container {
        global_state: GlobalState {
            container_id: 0, // assigned by the store on create
            parent_id,
            child_ids: vec![],
            child_count: 0,
            version: 1,
        },
        local_state: LocalState {
            metadata: Metadata {
                container_type: ContainerType::Derived,
                modality: Modality::Unknown,
                created_at: now,
                updated_at: now,
                provenance: provenance.to_string(),
                permissions: 0,
                owner_id: 0,
                name: Some(name),
                materialized_path: Some(materialized_path),
            },
            context: Context {
                categories: vec![],
                methodologies: vec![],
                keywords,
                topics: vec!["consciousness".to_string()],
                relationships,
                learned_associations: vec![],
                embedding: None,
            },
            storage: StoragePointers {
                db_shard_id: None,
                vector_index_ref: None,
                object_store_path,
                compression_type: CompressionType::None,
            },
            hints: TraversalHints::default(),
            integrity: IntegrityData::default(),
            file_context: None,
            code_context: None,
            text_context: None,
            external_ref: None,
        },
    }
}

fn relation(target_id: u64, relation_type: RelationType) -> Relation {
    Relation {
        target_id,
        relation_type,
        confidence: 1.0,
        discovered_via: DiscoveryMethod::Manual,
        graph_hops: None,
    }
}

async fn create_run_root(store: &Arc<dyn StoreAccess>, loop_name: &str, now: u64) -> Option<u64> {
    let container = build_container(
        CONSCIOUSNESS_METACOGNITION_ROOT_ID,
        format!("Run: {}", loop_name),
        format!("/Consciousness/Metacognition/Runs/{}", loop_name),
        "run-spans",
        vec!["run-root".to_string(), format!("loop:{}", loop_name)],
        vec![],
        None,
        now,
    );
    let json = serde_json::to_value(&container).ok()?;
    match store
        .create_container(CONSCIOUSNESS_METACOGNITION_ROOT_ID, json)
        .await
    {
        Ok(id) => {
            tracing::info!(loop_name, run_root_id = id, "run spans: created Run root for loop");
            Some(id)
        }
        Err(e) => {
            tracing::warn!(loop_name, error = %e, "run spans: Run root creation failed (best-effort)");
            None
        }
    }
}

/// Record one loop PASS as a span container under that loop's long-lived Run
/// root, chained to the loop's previous pass span.
///
/// - Finds the Run root from an in-memory map persisted at
///   `<data_dir>/consciousness/run_roots.json` (verified against the store; a
///   missing/deleted root is recreated).
/// - The span gets `PartOf` -> Run root and `Follows` -> the previous span
///   (reverse `Precedes` edges are NOT written: that would need an
///   UpdateContainer on every earlier span; the forward chain is walkable
///   backwards by traversal).
/// - The span body (`consciousness/run_span_<loop>_<span_id>.json`) carries the
///   trace fields and `summary` (see [`PassStats::to_json`]).
/// - Returns the span container id, or `None` on any failure (logged).
pub async fn record_pass_span(
    store: &Arc<dyn StoreAccess>,
    loop_name: &str,
    ctx: &TraceCtx,
    summary: serde_json::Value,
) -> Option<u64> {
    let now = now_secs();

    let existing = runs().lock().ok().and_then(|m| m.get(loop_name).cloned());
    let mut root_id: Option<u64> = None;
    let mut prev: Option<u64> = None;
    if let Some(st) = existing {
        if matches!(store.get_container(st.root_id).await, Ok(Some(_))) {
            root_id = Some(st.root_id);
            prev = st.last_span_id;
        }
    }
    let root_id = match root_id {
        Some(id) => id,
        None => create_run_root(store, loop_name, now).await?,
    };
    let prev = match prev {
        Some(p) if matches!(store.get_container(p).await, Ok(Some(_))) => Some(p),
        _ => None,
    };

    let body = serde_json::json!({
        "kind": "run_span",
        "loop": loop_name,
        "trace": ctx.fields(),
        "recorded_at": now,
        "summary": summary,
    });
    let rel_path = write_body(
        &format!("consciousness/run_span_{}_{}.json", loop_name, ctx.span_id),
        &body,
    );

    let mut relationships = vec![relation(root_id, RelationType::PartOf)];
    if let Some(p) = prev {
        relationships.push(relation(p, RelationType::Follows));
    }
    let container = build_container(
        root_id,
        format!("Pass: {} {}", loop_name, now),
        format!("/Consciousness/Metacognition/Runs/{}/{}", loop_name, now),
        "run-spans",
        vec![
            "run-span".to_string(),
            format!("loop:{}", loop_name),
            format!("trace:{}", ctx.trace_id),
        ],
        relationships,
        rel_path,
        now,
    );
    let json = match serde_json::to_value(&container) {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!(loop_name, error = %e, "run spans: span serialize failed (best-effort)");
            return None;
        }
    };
    match store.create_container(root_id, json).await {
        Ok(span_id) => {
            if let Ok(mut m) = runs().lock() {
                m.insert(
                    loop_name.to_string(),
                    RunState { root_id, last_span_id: Some(span_id) },
                );
                persist(&m);
            }
            tracing::debug!(loop_name, span_id, trace_id = %ctx.trace_id, "run spans: pass span recorded");
            Some(span_id)
        }
        Err(e) => {
            tracing::warn!(loop_name, error = %e, "run spans: span creation failed (best-effort)");
            None
        }
    }
}

// ── context-budget helpers shared by the loops ─────────────────────────────

/// Window of the model a loop call will use when no override is sent: the
/// host's configured context length (`OZONE_CONTEXT_LENGTH`, exported at boot
/// from `models.context_length`). 8192 labelled "unknown" when unset — a
/// conservative budget, never a claimed measurement.
pub fn default_model_window() -> (usize, &'static str) {
    match std::env::var("OZONE_CONTEXT_LENGTH")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
    {
        Some(w) if w > 0 => (w, "configured"),
        _ => (8192, "unknown"),
    }
}

/// Window of an explicit model profile (`AvailableModel.context_length`);
/// falls back to [`default_model_window`] when the profile carries 0.
pub fn profile_window(context_length: usize) -> (usize, &'static str) {
    if context_length > 0 {
        (context_length, "configured")
    } else {
        default_model_window()
    }
}

/// Input budget (tokens) for a prompt: window minus the output the call asks
/// for minus a fixed scaffold/safety reserve, floored at 1,000 so a tiny
/// window never produces a zero budget (same never-a-regression floor the
/// assistant digest uses).
pub fn input_budget_tokens(window_tokens: usize, want_output_tokens: usize) -> usize {
    window_tokens
        .saturating_sub(want_output_tokens + 512)
        .max(1_000)
}

/// Record the trims of a loop prompt's assembled view as a ContextRecord.
/// `capture_loop_model_call` writes the per-call ContextRecord (served model,
/// served window, prompt size) but takes no trims, so when `assemble` had to
/// cut anything this writes one extra record, `call_site = loop:<site>:view`,
/// carrying the window used for budgeting and every `TrimRecord` (which also
/// emits the `context_trimmed` graph markers). Writes nothing when nothing
/// was trimmed — no duplicate rows on the common path.
pub fn record_view_trims(
    call_site: &str,
    model: &str,
    window_tokens: usize,
    window_source: &'static str,
    want_output_tokens: usize,
    assembled: &crate::context_budget::AssembledContext,
) {
    if assembled.trims.is_empty() {
        return;
    }
    let mut rec = crate::context_budget::ContextRecord::new(format!("loop:{call_site}:view"), model);
    rec.window_tokens = window_tokens;
    rec.window_source = window_source;
    rec.want_output_tokens = want_output_tokens;
    rec.prompt_tokens = assembled.estimated_tokens;
    rec.usable = true;
    rec.trims = assembled.trims.clone();
    crate::context_budget::record_call(rec);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_pass_is_not_recorded() {
        assert!(!PassStats::default().did_work());
        let mut s = PassStats::default();
        s.calls_made = 1;
        assert!(s.did_work());
    }

    #[test]
    fn budget_floor_never_zero() {
        assert_eq!(input_budget_tokens(0, 400), 1_000);
        assert_eq!(input_budget_tokens(32_768, 400), 32_768 - 912);
    }

    #[test]
    fn stats_serialize_all_counters() {
        let mut s = PassStats::default();
        s.candidates_consumed = 3;
        s.candidate_container_ids = vec![7, 9];
        let j = s.to_json();
        assert_eq!(j["candidates_consumed"], 3);
        assert_eq!(j["candidate_container_ids"][1], 9);
    }
}
