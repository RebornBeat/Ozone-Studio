//! KEYWORD PROGRAM — host side (2026-11-08; docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md).
//!
//! What lives here:
//! * the **hit log** (`zsei_data/keyword_hits.jsonl`) and the **judgment-sample log** (`zsei_data/judgment_samples.jsonl`): append-only,
//!   cheap, never blocking a read — they ARE the training data (real events only; nothing invented);
//! * the **rollup** that folds new hit events into the containers (`keyword_meta` counts, `hints.access_frequency / last_accessed /
//!   hotness_score`) in ONE batched write per container through the single store write path — never per search;
//! * the **dataset builder** (time-ordered, no label leakage, last 20 % of groups held out) the `traversal_ml` pipeline trains on;
//! * the **model hook** the search door calls to order candidates (deterministic v0 + every active backend, fused);
//! * the **trainer trigger** (pipeline 17 `TrainModel`) and the model graph-citizen containers.
//!
//! DOCTRINE: no confidence anywhere — counts of observed events, yes/no judgments, and rankings. `keyword_meta["*"]` is the reserved
//! container-level totals entry (it is not a keyword).

use crate::orchestrator::{PipelineExecutor, StoreAccess};
use crate::shared_contracts::keyword_model as km;
use crate::types::container::{
    CompressionType, Container, ContainerType, Context, GlobalState, IntegrityData, LocalState, Metadata, Modality, StoragePointers, TermHits,
    TermMeta, TraversalHints, PIPELINE_ROOT_ID,
};
use crate::types::zsei::ContainerUpdate;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// Reserved `keyword_meta` key holding the CONTAINER-LEVEL hit totals (not a keyword; ignored as a term).
pub const CONTAINER_TOTALS_KEY: &str = "*";

/// The container types whose keywords are MEANING (a content search is meant to match them) — the ones the validation gate covers, in the
/// order settlement validates them (most-searched first). Bookkeeping types (CoordinationEvent, Derived, McpResult, Experience, chat rows)
/// carry system tags, not meaning; they are not gated.
pub const GATED_TYPES: &[&str] = &["Methodology", "Project", "Workspace", "ModalityGraph", "Blueprint", "Pipeline", "FileReference", "Category", "McpEntity"];
// NOT gated, on purpose: `JurisdictionRuleSet`. Its keywords are ROUTING KEYS — the region / scope code the jurisdiction loader finds the rule
// set by (`global`, `eu`, `us-ca`, and ISO country codes such as `is`, `no`, `do`, `it`) — plus terms copied from the rules' own conditions.
// They are facts, not guesses: running them through the model gate or a shape rule can silently stop a region's rules from loading
// (found live 2026-11-10: a filler-word cleanup removed `is`/`no`/`do`/`it` — Iceland, Norway, the Dominican Republic, Italy).

/// THE WRITE-TIME GATE — called from the ONE place every container is created or updated (`zsei/query.rs`, which host writers and pipelines
/// both go through), so no writer can bypass it. It cannot call a model, so it does the two things that are deterministic: (1) the SHAPE
/// gate — empty strings, sentence-like strings (> 4 words) and case-insensitive duplicates never enter the store; markers pass untouched;
/// (2) every term that has no `keyword_meta` yet is registered as PENDING (`validated: None`, source `created:<call_site>`) so it is
/// visible as unvalidated and keyword settlement (Pass V) validates it — a settled container is one with no pending term.
pub fn gate_keywords_on_write(container_type: &str, ctx: &mut Context) {
    if !GATED_TYPES.contains(&container_type) {
        return;
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut kept: Vec<String> = Vec::with_capacity(ctx.keywords.len());
    for k in ctx.keywords.drain(..) {
        let t = k.trim().to_string();
        if t.is_empty() || (!km::is_marker(&t) && km::is_sentence_like(&t)) || !seen.insert(t.to_lowercase()) {
            continue;
        }
        kept.push(t);
    }
    ctx.keywords = kept;
    let site = current_origin().map(|o| o.call_site).unwrap_or_else(|| "unknown".to_string());
    for k in ctx.keywords.iter() {
        if km::is_marker(k) || ctx.keyword_meta.contains_key(k.as_str()) || ctx.keyword_meta.contains_key(&k.to_lowercase()) {
            continue;
        }
        ctx.keyword_meta.insert(k.clone(), TermMeta { source: format!("created:{site}"), validated: None, hits: Default::default() });
    }
}

/// A bookkeeping tag as a namespaced MARKER value: `ns:` + the tag lower-cased with inner whitespace turned into `-`. `None` when the result
/// is not a valid marker (`km::is_marker`), so a tag that cannot be expressed as one is left exactly as it was.
fn tag_marker(ns: &str, raw: &str) -> Option<String> {
    let v = raw.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("-");
    let m = format!("{ns}:{v}");
    if v.is_empty() || !km::is_marker(&m) {
        None
    } else {
        Some(m)
    }
}

/// TAG NAMESPACING (operator 2026-11-10, option A; docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §17 finding 1 / §18.1). System BOOKKEEPING tags
/// (event kinds, agent names, run spans, ingest node kinds) were stored as plain words in `keywords`, so a content search for "link" or
/// "task" matched thousands of system events (`tool_call` alone was 12 % of all EU keyword slots). They are rewritten HERE, once, at the one
/// write chokepoint (`zsei/query.rs`, before the shape/validation gate), into namespaced markers — `kind:tool_call`, `agent:sensor-tick`,
/// `span:run`, `tool:uidocument_ingest`, `node:element` — so the existing rule (markers meet only markers; never part of the meaning pool;
/// never sent through the model gate) keeps them out of content searches.
/// The rewrite is STRUCTURAL, derived from each emitter's own keyword layout, never from a word list, and only for the container types
/// whose emitters are known:
///   * `CoordinationEvent` — `context_mirror::mirror` writes `[kind, agent, file:…, scope…, claim:…]`: position 0 → `kind:`, position 1 → `agent:`;
///   * `Derived` — the run-span writer's literals `run-span` → `span:run` and `run-root` → `span:root` (every other Derived tag is a content term and is left alone);
///   * `McpResult` — `mcp_graph::persist` writes `[tool, "mcp_result", …]`: position 0 → `tool:`, the literal `mcp_result` → `kind:mcp_result`;
///   * `McpEntity` — `mcp_graph::persist` writes `[tool, node kind, call:…, scope…]`: position 0 → `tool:`, position 1 → `node:`.
/// IDEMPOTENT (a marker is never touched), content terms and existing markers are never touched, and any other container type is returned
/// unchanged. A rewritten term's PENDING-only registration (`validated: None`, source `created:*`, no hits) is dropped with it — a marker is
/// never judged; a term with a verdict or hit history keeps its record. Readers accept BOTH forms during the migration: [`has_tag`].
pub fn namespace_bookkeeping_tags(container_type: &str, ctx: &mut Context) {
    let plain = |k: &str| !k.trim().is_empty() && !km::is_marker(k);
    let mut renamed: Vec<(String, String)> = Vec::new();
    let mut set = |kws: &mut Vec<String>, i: usize, ns: &str| {
        if let Some(k) = kws.get(i).cloned() {
            if plain(&k) {
                if let Some(m) = tag_marker(ns, &k) {
                    kws[i] = m.clone();
                    renamed.push((k, m));
                }
            }
        }
    };
    match container_type {
        "CoordinationEvent" => {
            set(&mut ctx.keywords, 0, "kind");
            // the agent follows the kind; only read position 1 as the agent when position 0 is the kind (plain or already `kind:`)
            if ctx.keywords.first().map(|k| k.starts_with("kind:")).unwrap_or(false) {
                set(&mut ctx.keywords, 1, "agent");
            }
        }
        "Derived" => {
            // the run-span writer's two literals (`run_spans.rs`: one per pass, one root per loop)
            for (plain_tag, marker) in [("run-span", "span:run"), ("run-root", "span:root")] {
                for i in 0..ctx.keywords.len() {
                    if ctx.keywords[i] == plain_tag {
                        ctx.keywords[i] = marker.to_string();
                        renamed.push((plain_tag.to_string(), marker.to_string()));
                    }
                }
            }
        }
        "McpResult" => {
            set(&mut ctx.keywords, 0, "tool");
            for i in 0..ctx.keywords.len() {
                if ctx.keywords[i] == "mcp_result" {
                    ctx.keywords[i] = "kind:mcp_result".to_string();
                    renamed.push(("mcp_result".to_string(), "kind:mcp_result".to_string()));
                }
            }
        }
        "McpEntity" => {
            set(&mut ctx.keywords, 0, "tool");
            if ctx.keywords.first().map(|k| k.starts_with("tool:")).unwrap_or(false) {
                set(&mut ctx.keywords, 1, "node");
            }
        }
        // FOLLOW-UP (2026-11-10): chat containers — `chat_store.rs` writes `[chat, chat_session|chat_turn, chatsession:…, proj:/ws:/global, task:N]`
        // (the session / scope / task terms are already markers; `global` becomes the `scope:global` marker every other writer uses)
        "ChatSession" | "ChatTurn" => rename_literals(
            &mut ctx.keywords,
            &[("chat", "kind:chat"), ("chat_session", "kind:chat_session"), ("chat_turn", "kind:chat_turn"), ("global", "scope:global")],
            &mut renamed,
        ),
        // `consciousness/store.rs::build_experience_container` writes `tags ++ categories ++ [experience_type]` — a fixed, small set of literals
        // (`task`, `no-emotion-update`, `migrated`, `task_success`, `task_failure`, `success`, `failure`); the Experience record itself keeps its
        // plain fields (readers use the struct, not the container's keywords)
        "Experience" => rename_literals(
            &mut ctx.keywords,
            &[
                ("task", "kind:task"),
                ("no-emotion-update", "flag:no-emotion-update"),
                ("migrated", "flag:migrated"),
                ("task_success", "outcome:task_success"),
                ("task_failure", "outcome:task_failure"),
                ("success", "outcome:success"),
                ("failure", "outcome:failure"),
            ],
            &mut renamed,
        ),
        _ => return,
    }
    if renamed.is_empty() {
        return;
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    ctx.keywords.retain(|k| seen.insert(k.to_lowercase()));
    for (old, _) in &renamed {
        if ctx.keywords.iter().any(|k| k == old) {
            continue; // the plain form is still present (a second occurrence): keep its record
        }
        let pending_only = ctx.keyword_meta.get(old.as_str()).map(|m| m.validated.is_none() && m.source.starts_with("created:") && m.hits == TermHits::default()).unwrap_or(false);
        if pending_only {
            ctx.keyword_meta.remove(old.as_str());
        }
    }
}

/// Exact-literal renames (case-sensitive, whole term): every occurrence of `plain` becomes `marker`; each is recorded in `renamed`.
fn rename_literals(keywords: &mut [String], map: &[(&str, &str)], renamed: &mut Vec<(String, String)>) {
    for (plain_tag, marker) in map {
        for k in keywords.iter_mut() {
            if k == plain_tag {
                *k = (*marker).to_string();
                renamed.push((plain_tag.to_string(), (*marker).to_string()));
            }
        }
    }
}

/// True when `keywords` carry the bookkeeping tag `tag` in EITHER form — the legacy plain word (`tool_call`) or the namespaced marker
/// (`kind:tool_call`). Every reader of these tags uses this during the migration, so a store that holds a mix of both keeps working.
pub fn has_tag(keywords: &[String], ns: &str, tag: &str) -> bool {
    keywords.iter().any(|k| k == tag || k.strip_prefix(ns).and_then(|r| r.strip_prefix(':')) == Some(tag))
}

/// VALIDATION GATE (2026-11-10; operator: every stored keyword must be validated). A term is validated when it passed the strength-N
/// yes/no gate in keyword settlement (`validated == Some(true)`), or it is a deterministic graph fact (`identifier` / `rollup` — read from
/// the content's own structure, not guessed by a model).
pub fn term_is_validated(meta: Option<&TermMeta>) -> bool {
    meta.map(|m| m.validated == Some(true) || m.source == "identifier" || m.source == "rollup").unwrap_or(false)
}

/// STRICT MODE (`OZONE_KEYWORD_REQUIRE_VALIDATED=1`, default OFF): search ignores keywords that have not passed the gate. Off by default
/// because it empties recall on a store whose backlog is still being validated; flip it once the backlog is done.
pub fn require_validated_keywords() -> bool {
    matches!(std::env::var("OZONE_KEYWORD_REQUIRE_VALIDATED").ok().as_deref(), Some("1") | Some("true"))
}

/// The keywords a search may match: all of them, or (strict) only markers and validated terms. Markers are scope/provenance data, never
/// sent through the model gate, so strict mode keeps them.
pub fn searchable_keywords(keywords: &[String], meta: &BTreeMap<String, TermMeta>, strict: bool) -> Vec<String> {
    // ABLATION (`OZONE_KEYWORD_IGNORE_EXPANSION=1`, default off): run the same queries without zero-shot expansion terms, to measure what they
    // add on the same store (docs §19). Nothing is deleted — the terms are only left out of matching.
    let ignore_expansion = matches!(std::env::var("OZONE_KEYWORD_IGNORE_EXPANSION").ok().as_deref(), Some("1") | Some("true"));
    filter_keywords(keywords, meta, strict, ignore_expansion)
}

/// The pure filter behind `searchable_keywords` (markers always pass; strict keeps only validated terms; ablation drops expansion-sourced ones).
pub fn filter_keywords(keywords: &[String], meta: &BTreeMap<String, TermMeta>, strict: bool, ignore_expansion: bool) -> Vec<String> {
    if !strict && !ignore_expansion {
        return keywords.to_vec();
    }
    let m_of = |k: &String| meta.get(k.as_str()).or_else(|| meta.get(&k.to_lowercase()));
    keywords
        .iter()
        .filter(|k| km::is_marker(k) || ((!strict || term_is_validated(m_of(k))) && !(ignore_expansion && m_of(k).map(|m| m.source == "expansion").unwrap_or(false))))
        .cloned()
        .collect()
}


pub fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string()))
}

/// Where the trained models and their registry live (`zsei_data/models/keyword`).
pub fn models_dir() -> PathBuf {
    data_dir().join("models").join("keyword")
}

/// `OZONE_KEYWORD_HITS=0` switches the hit/judgment logs off (they are on by default).
pub fn hits_enabled() -> bool {
    std::env::var("OZONE_KEYWORD_HITS").map(|v| v != "0").unwrap_or(true)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn trace_id() -> String {
    crate::trace::current_trace().map(|t| t.trace_id).unwrap_or_default()
}

// ============================================================================================================
// ORIGIN — "from where it was called" (workspace / project / global / the files the scope is handling)
// ============================================================================================================

/// Where a piece of keyword work came from. Attached to every `created`, `search` and outcome event so scope and provenance are never
/// reconstructed afterwards (docs/KEYWORD_FLOW_AND_NEEDS_GUIDE.md §2). Installed as a task-local (like the trace) by the orchestrator and the
/// loops, and re-installed from the `x-ozone-origin` header on the `/zsei/query` route for pipeline-originated calls.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Origin {
    /// `orchestrate`, `meta-loop`, `i-loop`, `amt-loop`, `http:<route>`, or a pipeline name.
    pub call_site: String,
    pub task_id: Option<u64>,
    pub workspace_id: Option<u64>,
    pub project_id: Option<u64>,
    /// True when the work is not scoped to any project or workspace.
    pub global: bool,
    /// Terms of the request that triggered the work (never the raw text). Shared and filled in once the request's keywords are known
    /// (after Stage 2), so events from later stages carry them.
    #[serde(skip)]
    pub request_terms: Arc<Mutex<Vec<String>>>,
    /// Short hash of the request text (dedupe / join key; the text itself is never stored).
    pub request_hash: String,
    /// True for work the SYSTEM generated itself (an AMT re-expansion task, a sensor tick …) — its "request" is machine text (ids, route
    /// names), so its events are logged for observed use but kept OUT of the training pairs (they would teach the ranker machine tokens).
    pub internal: bool,
    /// The file graphs the scope is handling NOW — shared and growing (the orchestrator appends as Stage 4 graphs the attached files).
    #[serde(skip)]
    pub files: Arc<Mutex<Vec<u64>>>,
}

impl Origin {
    /// A background actor (meta loop, I-loop, AMT loop …): global by nature.
    pub fn actor(call_site: &str) -> Self {
        Origin { call_site: call_site.to_string(), global: true, ..Default::default() }
    }

    /// This origin, re-labelled as system-generated work run by `call_site` (e.g. `amt-loop`).
    pub fn mark_internal(mut self, call_site: &str) -> Self {
        self.call_site = call_site.to_string();
        self.internal = true;
        self
    }

    /// An origin for a request: scope from the request, `global` when it has neither project nor workspace.
    pub fn for_request(call_site: &str, workspace_id: Option<u64>, project_id: Option<u64>, request_text: &str) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in request_text.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Origin {
            call_site: call_site.to_string(),
            task_id: None,
            workspace_id,
            project_id,
            global: workspace_id.is_none() && project_id.is_none(),
            request_terms: Arc::new(Mutex::new(Vec::new())),
            request_hash: format!("{h:016x}"),
            internal: false,
            files: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Rebuild the origin a pipeline process inherited: the executor stamps `OZONE_ORIGIN` (ASCII JSON of call_site / workspace_id /
    /// project_id / task_id) and the pipeline forwards it in the `x-ozone-origin` header. Empty or invalid = no origin.
    pub fn from_header(raw: &str) -> Option<Self> {
        let v: Value = serde_json::from_str(raw.trim()).ok()?;
        let ws = v.get("workspace_id").and_then(|x| x.as_u64());
        let pj = v.get("project_id").and_then(|x| x.as_u64());
        Some(Origin {
            call_site: v.get("call_site").and_then(|x| x.as_str()).unwrap_or("pipeline").to_string(),
            task_id: v.get("task_id").and_then(|x| x.as_u64()),
            workspace_id: ws,
            project_id: pj,
            global: ws.is_none() && pj.is_none(),
            ..Default::default()
        })
    }

    /// The origin as the JSON stored on events (file ids read from the shared list at this instant).
    pub fn to_json(&self) -> Value {
        let files: Vec<u64> = self.files.lock().map(|f| f.clone()).unwrap_or_default();
        let request_terms: Vec<String> = self.request_terms.lock().map(|t| t.clone()).unwrap_or_default();
        json!({
            "call_site": self.call_site, "task_id": self.task_id, "workspace_id": self.workspace_id, "project_id": self.project_id,
            "global": self.global, "request_terms": request_terms, "request_hash": self.request_hash, "file_ids": files, "internal": self.internal,
        })
    }
}

tokio::task_local! {
    static ORIGIN: Origin;
}

/// Run `fut` with `o` as the current origin (the trace analogue; `tokio::spawn` does not inherit it — use `orchestrator::traced`).
pub fn with_origin<F: std::future::Future>(o: Origin, fut: F) -> impl std::future::Future<Output = F::Output> {
    ORIGIN.scope(o, fut)
}

pub fn current_origin() -> Option<Origin> {
    ORIGIN.try_with(|o| o.clone()).ok()
}

/// Record the request's usable keywords on the current origin (called once Stage 2 has produced them). No-op outside an origin.
pub fn note_request_terms(terms: Vec<String>) {
    let _ = ORIGIN.try_with(|o| {
        if let Ok(mut t) = o.request_terms.lock() {
            *t = terms;
        }
    });
}

/// Add file-graph ids the current scope is handling (called when Stage 4 has graphed the attached files). No-op outside an origin.
pub fn note_files<I: IntoIterator<Item = u64>>(ids: I) {
    let _ = ORIGIN.try_with(|o| {
        if let Ok(mut f) = o.files.lock() {
            for id in ids {
                if !f.contains(&id) {
                    f.push(id);
                }
            }
        }
    });
}

fn origin_json() -> Value {
    current_origin().map(|o| o.to_json()).unwrap_or(Value::Null)
}

static LOG_LOCK: Mutex<()> = Mutex::new(());

fn append_line(file: &str, v: &Value) {
    let _guard = LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(file)) {
        let mut line = v.to_string();
        line.push('\n');
        let _ = f.write_all(line.as_bytes());
    }
}

// ============================================================================================================
// Event capture
// ============================================================================================================

/// One search: the query terms, the strategy that answered, and every id it returned (a single line; the dataset builder expands it).
pub fn record_search(terms: &[String], strategy: &str, ids: &[u64]) {
    if !hits_enabled() || ids.is_empty() || terms.is_empty() {
        return;
    }
    append_line(
        "keyword_hits.jsonl",
        &json!({"ts_ms": now_ms(), "event": "search", "trace": trace_id(), "strategy": strategy, "terms": terms, "ids": ids, "origin": origin_json()}),
    );
}

/// One per-container event: `packed` (made it into a step context / span-out window), `gathered` (a methodology gathered for a request),
/// `applied` (a methodology a blueprint step carries), `kept` / `dropped` (a strength-N judgment affirmed / unanimously rejected it),
/// `human` (a decision-review resolution; `extra.yes`). `terms` = the query terms in effect when known.
pub fn record_event(event: &str, container_id: u64, terms: &[String], extra: Value) {
    if !hits_enabled() {
        return;
    }
    let mut obj = serde_json::Map::new();
    obj.insert("ts_ms".into(), json!(now_ms()));
    obj.insert("event".into(), json!(event));
    obj.insert("trace".into(), json!(trace_id()));
    obj.insert("container_id".into(), json!(container_id));
    obj.insert("terms".into(), json!(terms));
    obj.insert("origin".into(), origin_json());
    if let Value::Object(m) = extra {
        for (k, v) in m {
            obj.insert(k, v);
        }
    }
    append_line("keyword_hits.jsonl", &Value::Object(obj));
}

/// CREATION capture — called from the ONE container-creation door (`zsei/query.rs::create_container`), so every creator (host or pipeline)
/// leaves a `created` event. `terms` = the REQUEST terms of the current origin (the query side of the creation pair); `container_terms` = the
/// new container's own meaning terms. For a methodology made during a walk this is its FIRST HIT: why (request terms, trace), where
/// (workspace / project / global and the typed scope it ended up with) and what the scope was handling (`origin.file_ids`).
pub fn record_created(id: u64, c: &Container) {
    if !hits_enabled() {
        return;
    }
    let ctx = &c.local_state.context;
    let origin = current_origin();
    // FALLBACK origin: a container created outside any origin scope (ripple actors, bootstrap, assistant …) records its PROVENANCE as the
    // call site and its typed scope — honest about being derived (`derived_from`), never left null (live: 26 of 35 creations had no origin).
    let origin_value = match &origin {
        Some(o) => o.to_json(),
        None => json!({
            "call_site": c.local_state.metadata.provenance, "global": ctx.scope.workspace_id.is_none() && ctx.scope.project_id.is_none(),
            "workspace_id": ctx.scope.workspace_id, "project_id": ctx.scope.project_id, "derived_from": "provenance",
        }),
    };
    let request_terms: Vec<String> = origin.as_ref().and_then(|o| o.request_terms.lock().ok().map(|t| t.clone())).unwrap_or_default();
    append_line(
        "keyword_hits.jsonl",
        &json!({
            "ts_ms": now_ms(), "event": "created", "trace": trace_id(), "container_id": id,
            "container_type": serde_json::to_value(c.local_state.metadata.container_type).unwrap_or(Value::Null),
            "name": c.local_state.metadata.name, "provenance": c.local_state.metadata.provenance,
            "parent_id": c.global_state.parent_id,
            "terms": request_terms,
            "container_terms": km::meaning_terms(&ctx.keywords), "markers": ctx.keywords.iter().filter(|k| km::is_marker(k)).collect::<Vec<_>>(),
            "topics": ctx.topics, "scope": ctx.scope, "origin": origin_value,
        }),
    );
}

/// KEYWORD LIFECYCLE capture — called from the update door when an update changes a container's keywords: which terms were added and which
/// removed, and by whom (the origin's call site). Together with `created` this records the full life of a container's keywords.
pub fn record_keywords_changed(id: u64, before: &[String], after: &[String]) {
    if !hits_enabled() {
        return;
    }
    let lower = |v: &[String]| v.iter().map(|k| k.trim().to_lowercase()).collect::<BTreeSet<_>>();
    let (b, a) = (lower(before), lower(after));
    let added: Vec<&String> = after.iter().filter(|k| !b.contains(&k.trim().to_lowercase())).collect();
    let removed: Vec<&String> = before.iter().filter(|k| !a.contains(&k.trim().to_lowercase())).collect();
    if added.is_empty() && removed.is_empty() {
        return;
    }
    append_line(
        "keyword_hits.jsonl",
        &json!({"ts_ms": now_ms(), "event": "keywords_changed", "trace": trace_id(), "container_id": id, "added": added, "removed": removed, "origin": origin_json()}),
    );
}

/// Record a strength-N judgment as a LABELLED SAMPLE (the training labels the model needs). `kind` names the question family
/// ("methodology_relevance" | "rule_applicability" | "rule_step_fidelity" | "offer_review" | "capability_necessity" | "keyword_keeper" | "decision_review"). `yes` is the vote's
/// outcome (for drop-style questions YES = "drop it"); `strength` is the N the vote ran at.
pub fn record_judgment(kind: &str, subject_id: Option<u64>, subject: &str, request: &str, question: &str, yes: bool, strength: u32) {
    if !hits_enabled() {
        return;
    }
    append_line(
        "judgment_samples.jsonl",
        &json!({
            "ts_ms": now_ms(), "trace": trace_id(), "kind": kind, "subject_id": subject_id, "subject": subject,
            "request": request, "question": question, "yes": yes, "strength": strength,
        }),
    );
}

// ============================================================================================================
// Rollup — fold new hit events into the containers (one batched write per container)
// ============================================================================================================

#[derive(Debug, Default, Clone)]
pub struct RollupStats {
    pub events: usize,
    pub containers_updated: usize,
    pub failures: usize,
}

fn bump(meta: &mut TermMeta, kind: &str) {
    bump_n(meta, kind, 1);
}

fn bump_n(meta: &mut TermMeta, kind: &str, n: u64) {
    let h = &mut meta.hits;
    match kind {
        "returned" => h.returned += n,
        "returned_traced" => h.returned_traced += n,
        "packed" => h.packed += n,
        "kept" => h.kept += n,
        "dropped" => h.dropped += n,
        "applied" => h.applied += n,
        "human" => h.human += n,
        _ => {}
    }
}

fn offset_path() -> PathBuf {
    data_dir().join("keyword_hits.offset")
}

/// The relation label a span-out `packed` event reached its container through: `via` is `"<parent id> -<Relation>-> this"` for a
/// relationship hop (`seed:…` for a seed). Returns `Relation` (e.g. `Calls`, `SimilarTo`, `child`).
pub fn edge_relation_of(via: &str) -> Option<String> {
    let rest = via.split_once(' ')?.1;
    let inner = rest.strip_suffix(" this")?.strip_prefix('-')?.strip_suffix("->")?;
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

/// REAL-EVENT EDGE SCORING INPUT (2026-11-08; operator: invented edge weights are gone — "edge weight you decide … based on what we have"):
/// how many times a context window was actually filled THROUGH each relation type, accumulated from the `packed` events during the rollup into
/// `zsei_data/edge_use_stats.json` as `{relation: packed_count}`. Counts only — no weights are invented from them; the span-out expansion keeps
/// its structural class priors until the operator rules on switching to the observed order.
fn merge_edge_use(delta: &BTreeMap<String, u64>) {
    if delta.is_empty() {
        return;
    }
    let path = data_dir().join("edge_use_stats.json");
    let mut cur: BTreeMap<String, u64> =
        std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    for (k, v) in delta {
        *cur.entry(k.clone()).or_insert(0) += v;
    }
    if let Ok(t) = serde_json::to_string_pretty(&cur) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, t).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

/// The hit-log files in chronological order: any `keyword_hits.archive.<ts>.jsonl` the operator has moved aside (oldest first), then the live
/// file. NOTHING in the host rotates, archives or truncates the hit log (operator 2026-11-08: the hits are the training data and must persist
/// no matter how large they grow) — the rollup reads from a byte offset and the dataset builder STREAMS, so size is not a problem.
fn hit_log_files() -> Vec<PathBuf> {
    hit_log_files_in(&data_dir())
}

fn hit_log_files_in(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut archives: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("keyword_hits.archive.") && n.ends_with(".jsonl")).unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    archives.sort();
    archives.push(dir.join("keyword_hits.jsonl"));
    archives
}

/// Fold hit events written since the last rollup into the containers they concern. Reads the log from the stored byte offset (complete
/// lines only), groups events per container, and per container performs ONE read + ONE `update_container` (so one ripple). Counters:
/// container-level totals under `keyword_meta["*"]`, per-keyword counts for every meaning keyword the event's query terms match (whole-token
/// or exact), and the hints block — `access_frequency` = total `returned`, `last_accessed` = newest event (epoch seconds),
/// `hotness_score` = packed ÷ returned, an OBSERVED ratio (replaces the invented 0.5–0.9 literals; 0 when never returned).
pub async fn rollup_once(store: &Arc<dyn StoreAccess>) -> RollupStats {
    /// Largest slice of the log read into memory at once (a HOST-MEMORY guard, not a cap on what is consumed: the loop below keeps taking
    /// slices until the whole log is folded — the log itself is never rotated, truncated or limited).
    const SLICE_BYTES: u64 = 64 * 1024 * 1024;
    // ONE rollup at a time: two concurrent callers would read the same offset and double-count the same slice
    static ROLLUP_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _rollup_guard = ROLLUP_LOCK.lock().await;
    let mut stats = RollupStats::default();
    let path = data_dir().join("keyword_hits.jsonl");
    let mut offset: u64 = std::fs::read_to_string(offset_path()).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0);
    let Ok(mut f) = std::fs::File::open(&path) else { return stats };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let mut slice = SLICE_BYTES;
    while offset < len {
        if f.seek(SeekFrom::Start(offset)).is_err() {
            break;
        }
        let want = slice.min(len - offset);
        let mut buf = Vec::new();
        if (&mut f).take(want).read_to_end(&mut buf).is_err() {
            break;
        }
        // only COMPLETE lines are consumed
        let consumed = match buf.iter().rposition(|b| *b == b'\n') {
            Some(i) => i + 1,
            None => {
                if want < len - offset {
                    slice = slice.saturating_mul(2); // one line longer than the slice: widen and retry
                    continue;
                }
                break; // an incomplete trailing line: wait for its newline
            }
        };
        let text = String::from_utf8_lossy(&buf[..consumed]).to_string();
        rollup_text(store, &text, &mut stats).await;
        offset += consumed as u64;
        // advance past what was consumed even if a container failed (a failed container loses these counts rather than double-counting them)
        let _ = std::fs::write(offset_path(), offset.to_string());
    }
    stats
}

/// Fold one slice of complete log lines. Events are AGGREGATED per container as `(kind, query-terms) → (count, newest ts)` so a search that
/// returned thousands of ids costs one small entry per id, not one event per id.
async fn rollup_text(store: &Arc<dyn StoreAccess>, text: &str, stats: &mut RollupStats) {
    let mut per: BTreeMap<u64, BTreeMap<(String, String), (Vec<String>, u64, u64)>> = BTreeMap::new();
    let mut edge_use: BTreeMap<String, u64> = BTreeMap::new();
    let mut add = |id: u64, kind: &str, terms: &[String], ts: u64| {
        let key = (kind.to_string(), terms.join("\u{1}"));
        let e = per.entry(id).or_default().entry(key).or_insert_with(|| (terms.to_vec(), 0, 0));
        e.1 += 1;
        e.2 = e.2.max(ts);
    };
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let ts = v.get("ts_ms").and_then(|x| x.as_u64()).unwrap_or(0);
        let terms: Vec<String> = v.get("terms").and_then(|t| serde_json::from_value(t.clone()).ok()).unwrap_or_default();
        match v.get("event").and_then(|e| e.as_str()).unwrap_or("") {
            "search" => {
                let traced = v.get("trace").and_then(|t| t.as_str()).map(|t| !t.is_empty()).unwrap_or(false);
                for id in v.get("ids").and_then(|i| i.as_array()).into_iter().flatten().filter_map(|x| x.as_u64()) {
                    add(id, "returned", &terms, ts);
                    if traced {
                        add(id, "returned_traced", &terms, ts);
                    }
                    stats.events += 1;
                }
            }
            kind @ ("packed" | "kept" | "dropped" | "applied" | "human") => {
                if kind == "packed" {
                    if let Some(rel) = v.get("via").and_then(|x| x.as_str()).and_then(edge_relation_of) {
                        *edge_use.entry(rel).or_insert(0) += 1;
                    }
                }
                if let Some(id) = v.get("container_id").and_then(|x| x.as_u64()) {
                    add(id, kind, &terms, ts);
                    stats.events += 1;
                }
            }
            _ => {}
        }
    }
    merge_edge_use(&edge_use);
    for (id, entries) in per {
        let evs: Vec<(String, Vec<String>, u64, u64)> = entries.into_iter().map(|((kind, _), (terms, n, ts))| (kind, terms, n, ts)).collect();
        match apply_rollup(store, id, &evs).await {
            Ok(true) => stats.containers_updated += 1,
            Ok(false) => {}
            Err(e) => {
                stats.failures += 1;
                tracing::debug!(container = id, error = %e, "keyword rollup: container not updated");
            }
        }
    }
}

/// `evs`: `(kind, query terms, count, newest ts_ms)`.
async fn apply_rollup(store: &Arc<dyn StoreAccess>, id: u64, evs: &[(String, Vec<String>, u64, u64)]) -> Result<bool, String> {
    let Some(v) = store.get_container(id).await? else { return Ok(false) };
    let mut c: Container = serde_json::from_value(v).map_err(|e| e.to_string())?;
    let meaning = km::meaning_terms(&c.local_state.context.keywords);
    let mut newest: u64 = 0;
    for (kind, terms, n, ts) in evs {
        newest = newest.max(*ts);
        let star = c.local_state.context.keyword_meta.entry(CONTAINER_TOTALS_KEY.to_string()).or_default();
        bump_n(star, kind, *n);
        for k in &meaning {
            if terms.iter().any(|q| km::match_info(&[q.clone()], "", &[k.clone()], false).is_some()) {
                let slot = c.local_state.context.keyword_meta.entry(k.clone()).or_default();
                bump_n(slot, kind, *n);
            }
        }
    }
    let totals = c.local_state.context.keyword_meta.get(CONTAINER_TOTALS_KEY).map(|m| m.hits.clone()).unwrap_or_default();
    let h = &mut c.local_state.hints;
    h.access_frequency = totals.returned.min(u32::MAX as u64) as u32;
    h.last_accessed = h.last_accessed.max(newest / 1000);
    h.hotness_score = if totals.returned_traced > 0 { (totals.packed as f32 / totals.returned_traced as f32).min(1.0) } else { 0.0 };
    let upd = ContainerUpdate {
        metadata: None,
        context: Some(c.local_state.context.clone()),
        storage: None,
        hints: Some(c.local_state.hints.clone()),
    };
    store.update_container(id, serde_json::to_value(&upd).map_err(|e| e.to_string())?).await?;
    Ok(true)
}

// ============================================================================================================
// Rank facts for the search door
// ============================================================================================================

/// Observed facts about a container for ranking, from its rolled-up `keyword_meta` (container totals + whether any matched term was
/// affirmed). `matched` = the container's keywords the query matched.
pub fn facts_from_context(
    class: km::MatchClass,
    matched_terms: usize,
    meta: &BTreeMap<String, TermMeta>,
    matched_keywords: &[String],
    last_accessed: u64,
    stored_hotness: f32,
) -> km::RankFacts {
    let totals = meta.get(CONTAINER_TOTALS_KEY).map(|m| m.hits.clone()).unwrap_or_default();
    let mut f = km::RankFacts::new(class, matched_terms);
    f.applied = totals.applied;
    f.kept = totals.kept;
    f.packed = totals.packed;
    f.returned = totals.returned;
    f.dropped = totals.dropped;
    // HOTNESS is the STORED `hints.hotness_score` — read as is (one map-free field read, no recomputation). It is trustworthy because the only
    // writer of a non-zero value is the rollup (packed ÷ traced returns), creators write 0.0, and the one-time legacy scrub
    // (`keyword_settle::scrub_legacy_hotness_once`) zeroed every invented value that was written before. Clamped and finite-checked.
    f.hotness = if stored_hotness.is_finite() { stored_hotness.clamp(0.0, 1.0) } else { 0.0 };
    f.validated = matched_keywords.iter().any(|k| meta.get(k).map(|m| m.validated == Some(true)).unwrap_or(false));
    f.last_accessed = last_accessed;
    f
}

// ============================================================================================================
// Model hook for the search door (in-process inference; cached by registry mtime)
// ============================================================================================================

struct ModelCache {
    stamp: Option<std::time::SystemTime>,
    encoders: Vec<Box<dyn km::TermEncoder>>,
    ranker: Option<km::LogisticRanker>,
}

static MODEL_CACHE: OnceLock<Mutex<ModelCache>> = OnceLock::new();

/// Order `cands` for `query_terms`: deterministic v0 fused with every ACTIVE, runnable backend (hashed n-gram today; pretrained / embedder
/// the moment they are registered active and runnable) and the active ranker. `OZONE_KEYWORD_BACKENDS=off` forces pure v0; a comma list of
/// encoder ids restricts the encoders (interchangeability knob); unset = every active backend. No active model = exactly v0.
pub fn order_for_search(query_terms: &[String], cands: &[km::Candidate]) -> Vec<u64> {
    let mode = std::env::var("OZONE_KEYWORD_BACKENDS").unwrap_or_default();
    if mode.trim().eq_ignore_ascii_case("off") {
        return km::order_candidates(query_terms, cands, &[], None);
    }
    let dir = models_dir();
    let stamp = std::fs::metadata(km::ModelRegistry::path(&dir)).and_then(|m| m.modified()).ok();
    let cache = MODEL_CACHE.get_or_init(|| Mutex::new(ModelCache { stamp: None, encoders: Vec::new(), ranker: None }));
    let mut g = cache.lock().unwrap_or_else(|e| e.into_inner());
    if g.stamp != stamp {
        g.encoders = km::load_active_encoders(&dir);
        g.ranker = km::load_active_ranker(&dir);
        g.stamp = stamp;
    }
    let allow: Option<Vec<String>> = if mode.trim().is_empty() {
        None
    } else {
        Some(mode.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
    };
    let refs: Vec<&dyn km::TermEncoder> = g
        .encoders
        .iter()
        .filter(|e| allow.as_ref().map(|a| a.iter().any(|id| id == e.id())).unwrap_or(true))
        .map(|b| b.as_ref())
        .collect();
    km::order_candidates(query_terms, cands, &refs, g.ranker.as_ref())
}

// ============================================================================================================
// Dataset builder (time-ordered; facts are snapshots from BEFORE each query; last 20 % of groups held out)
// ============================================================================================================

#[derive(Debug, Clone, Default)]
pub struct DatasetStats {
    pub groups: usize,
    pub heldout: usize,
    pub candidates: usize,
    pub path: String,
}

struct Ev {
    kind: String,
    container_type: String,
    trace: String,
    container_id: u64,
    terms: Vec<String>,
    ids: Vec<u64>,
    yes: Option<bool>,
    /// the event came from system-generated work (origin.internal) — never a training pair
    internal: bool,
}

pub fn dataset_path() -> PathBuf {
    data_dir().join("keyword_training_set.jsonl")
}

/// A `created` event becomes a creation pair when a methodology / blueprint was created inside a traced request that had request terms.
fn creation_pair_eligible(e: &Ev) -> bool {
    !e.terms.is_empty() && matches!(e.container_type.as_str(), "Methodology" | "Blueprint")
}

fn parse_ev(line: &str) -> Option<Ev> {
    if line.trim().is_empty() {
        return None;
    }
    let v = serde_json::from_str::<Value>(line).ok()?;
    Some(Ev {
        kind: v.get("event").and_then(|e| e.as_str()).unwrap_or("").to_string(),
        container_type: v.get("container_type").and_then(|e| e.as_str()).unwrap_or("").to_string(),
        trace: v.get("trace").and_then(|e| e.as_str()).unwrap_or("").to_string(),
        container_id: v.get("container_id").and_then(|e| e.as_u64()).unwrap_or(0),
        terms: v.get("terms").and_then(|t| serde_json::from_value(t.clone()).ok()).unwrap_or_default(),
        ids: v.get("ids").and_then(|t| serde_json::from_value(t.clone()).ok()).unwrap_or_default(),
        yes: v.get("yes").and_then(|y| y.as_bool()),
        internal: v.get("origin").and_then(|o| o.get("internal")).and_then(|i| i.as_bool()).unwrap_or(false),
    })
}

/// STREAM every hit event, oldest first, one line at a time (the log is never rotated or truncated and can be arbitrarily large — it is
/// never read into memory whole).
fn for_each_event(mut f: impl FnMut(usize, Ev)) {
    let mut n = 0usize;
    for path in hit_log_files() {
        let Ok(file) = std::fs::File::open(&path) else { continue };
        for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
            if let Some(e) = parse_ev(&line) {
                f(n, e);
                n += 1;
            }
        }
    }
}

pub async fn build_dataset(store: &Arc<dyn StoreAccess>) -> Result<DatasetStats, String> {
    // pass 1 (streamed) — labels per (trace, container): affirmed / applied / human-yes = relevant; dropped / human-no = not; gathered-only = not
    let mut label: BTreeMap<(String, u64), bool> = BTreeMap::new();
    let mut gathered: BTreeSet<(String, u64)> = BTreeSet::new();
    let mut gathered_by_trace: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for_each_event(|_, e| {
        if e.internal || e.trace.is_empty() || e.container_id == 0 {
            return;
        }
        let key = (e.trace.clone(), e.container_id);
        match e.kind.as_str() {
            // CREATION PAIR: a methodology / blueprint created for a request is a positive for that request's terms
            "created" if creation_pair_eligible(&e) => {
                label.insert(key, true);
            }
            "applied" | "kept" => {
                label.insert(key, true);
            }
            "human" => {
                label.insert(key, e.yes.unwrap_or(false));
            }
            "dropped" => {
                label.entry(key).or_insert(false);
            }
            "gathered" => {
                gathered_by_trace.entry(e.trace.clone()).or_default().push(e.container_id);
                gathered.insert(key);
            }
            _ => {}
        }
    });
    for k in gathered {
        label.entry(k).or_insert(false);
    }
    // pass 2 — chronological: snapshot running counters at each search, then update them
    let mut counters: BTreeMap<u64, [u64; 6]> = BTreeMap::new(); // returned, packed, kept, dropped, applied
    let mut info: BTreeMap<u64, Option<(String, Vec<String>, Vec<String>)>> = BTreeMap::new(); // name, all terms, meaning terms
    let mut groups: Vec<km::DatasetGroup> = Vec::new();
    let mut n = 0usize;
    for path in hit_log_files() {
        let Ok(file) = std::fs::File::open(&path) else { continue };
        for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        let Some(e) = parse_ev(&line) else { continue };
        n += 1;
        if e.internal {
            continue; // system-generated work: observed use only, never a training pair
        }
        // a SEARCH and a CREATION PAIR are built the same way: query terms + candidate ids, each labelled from the same trace's outcomes
        let search_like: Option<(Vec<u64>, Vec<String>)> = match e.kind.as_str() {
            "search" => Some((e.ids.clone(), e.terms.clone())),
            "created" if creation_pair_eligible(&e) => {
                let mut ids = vec![e.container_id];
                if let Some(g) = gathered_by_trace.get(&e.trace) {
                    ids.extend(g.iter().copied().filter(|x| *x != e.container_id));
                }
                Some((ids, e.terms.clone()))
            }
            _ => None,
        };
        match e.kind.as_str() {
            "search" | "created" if search_like.is_some() => {
                let (ids, qterms) = search_like.clone().unwrap_or_default();
                let mut cands: Vec<km::DatasetCandidate> = Vec::new();
                if !e.trace.is_empty() {
                    for id in &ids {
                        let Some(&lab) = label.get(&(e.trace.clone(), *id)) else { continue };
                        if !info.contains_key(id) {
                            let loaded = match store.get_container(*id).await {
                                Ok(Some(v)) => serde_json::from_value::<Container>(v).ok().map(|c| {
                                    let mut all: Vec<String> = c.local_state.context.keywords.clone();
                                    all.extend(c.local_state.context.topics.clone());
                                    let mut meaning = km::meaning_terms(&all);
                                    if meaning.is_empty() {
                                        if let Some(name) = &c.local_state.metadata.name {
                                            meaning.push(name.clone());
                                        }
                                    }
                                    (c.local_state.metadata.name.clone().unwrap_or_default(), all, meaning)
                                }),
                                _ => None,
                            };
                            info.insert(*id, loaded);
                        }
                        let Some(Some((name, all, meaning))) = info.get(id) else { continue };
                        if meaning.is_empty() {
                            continue;
                        }
                        let (class, matched) = match km::match_info(&qterms, name, all, false) {
                            Some(m) => (m.best, m.matched_terms),
                            None => (km::MatchClass::Substring, 1), // returned by a legacy substring strategy
                        };
                        let c = counters.get(id).copied().unwrap_or([0; 6]);
                        let mut facts = km::RankFacts::new(class, matched);
                        facts.returned = c[0];
                        facts.packed = c[1];
                        facts.kept = c[2];
                        facts.dropped = c[3];
                        facts.applied = c[4];
                        facts.hotness = if c[5] > 0 { (c[1] as f32 / c[5] as f32).min(1.0) } else { 0.0 };
                        cands.push(km::DatasetCandidate { id: *id, terms: meaning.clone(), facts, label: lab });
                    }
                }
                if cands.iter().any(|c| c.label) && cands.iter().any(|c| !c.label) {
                    groups.push(km::DatasetGroup { group: format!("{}:{}", e.trace, n), heldout: false, query_terms: qterms.clone(), candidates: cands });
                }
                if e.kind == "search" {
                    for id in &ids {
                        let slot = counters.entry(*id).or_insert([0; 6]);
                        slot[0] += 1;
                        if !e.trace.is_empty() {
                            slot[5] += 1;
                        }
                    }
                }
            }
            "packed" => counters.entry(e.container_id).or_insert([0; 6])[1] += 1,
            "kept" => counters.entry(e.container_id).or_insert([0; 6])[2] += 1,
            "dropped" => counters.entry(e.container_id).or_insert([0; 6])[3] += 1,
            "applied" => counters.entry(e.container_id).or_insert([0; 6])[4] += 1,
            _ => {}
        }
        }
    }
    // TERM-QUALITY groups from the REAL keyword_keeper judgments (more data, every one a unanimous-or-not strength-N verdict): per container, the
    // terms judged — removed (unanimous "not a keyword") = irrelevant, kept = relevant. They teach the encoder / ranker which terms carry
    // content and which are filler, in any language (no word list). Placed BEFORE the chronological search groups so the held-out tail stays the
    // newest real searches.
    let mut by_container: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();
    if let Ok(file) = std::fs::File::open(data_dir().join("judgment_samples.jsonl")) {
        for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
            if v.get("kind").and_then(|k| k.as_str()) != Some("keyword_keeper") {
                continue;
            }
            let term = v.get("subject").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
            let container = v.get("request").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
            let removed = v.get("yes").and_then(|x| x.as_bool()).unwrap_or(false);
            if !term.is_empty() && !container.is_empty() {
                by_container.entry(container).or_default().insert(term, !removed);
            }
        }
    }
    let term_id = |t: &str| -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in t.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    };
    let mut quality_groups: Vec<km::DatasetGroup> = Vec::new();
    for (container, terms) in by_container {
        let cands: Vec<km::DatasetCandidate> = terms
            .iter()
            .map(|(t, keep)| km::DatasetCandidate { id: term_id(t), terms: vec![t.clone()], facts: km::RankFacts::new(km::MatchClass::WholeToken, 1), label: *keep })
            .collect();
        if cands.iter().any(|c| c.label) && cands.iter().any(|c| !c.label) {
            quality_groups.push(km::DatasetGroup { group: format!("keeper:{container}"), heldout: false, query_terms: vec![container.clone()], candidates: cands });
        }
    }
    if !quality_groups.is_empty() {
        quality_groups.append(&mut groups);
        groups = quality_groups;
    }
    let total = groups.len();
    // a held-out tail exists as soon as there are two groups (promotion still needs significance — training does not wait for it)
    let held = if total >= 2 { (total / 5).max(1) } else { 0 };
    for (i, g) in groups.iter_mut().enumerate() {
        g.heldout = i >= total - held;
    }
    let path = dataset_path();
    km::write_dataset(&path, &groups)?;
    Ok(DatasetStats { groups: total, heldout: held, candidates: groups.iter().map(|g| g.candidates.len()).sum(), path: path.display().to_string() })
}

// ============================================================================================================
// Trainer trigger + model graph citizens
// ============================================================================================================

fn train_state_path() -> PathBuf {
    models_dir().join("train_state.json")
}

/// TRAIN CONTINUOUSLY (operator 2026-11-08: "even if we have candidates it should not ever stop or mean we do not continue to train"). Every
/// housekeeping cycle: build the dataset (search groups + creation pairs + keeper judgments) and run pipeline 17 `TrainModel`, which CONTINUES the
/// rolling candidate from its last weights whatever the previous verdict was; promotion only decides whether a versioned copy serves. There is no
/// "log grew" gate and no minimum group count (zero usable groups simply tries again next cycle). Registers new models as graph citizens.
pub async fn train_continuously(executor: &Arc<dyn PipelineExecutor>, store: &Arc<dyn StoreAccess>) -> Result<Value, String> {
    let stats = build_dataset(store).await?;
    let _ = std::fs::create_dir_all(models_dir());
    let _ = std::fs::write(train_state_path(), json!({"groups": stats.groups, "heldout": stats.heldout, "candidates": stats.candidates}).to_string());
    if stats.groups == 0 {
        return Ok(json!({"trained": false, "reason": "no labelled group yet (a search or keeper judgment with both a relevant and an irrelevant candidate) — the next cycle tries again"}));
    }
    let out = executor
        .execute(17, json!({"action": "TrainModel", "dataset_path": stats.path, "models_dir": models_dir().display().to_string()}))
        .await?;
    if out.get("success").and_then(|s| s.as_bool()) != Some(true) {
        return Err(out.get("error").and_then(|e| e.as_str()).unwrap_or("TrainModel failed").to_string());
    }
    register_model_containers(store).await;
    Ok(out)
}

/// Give every registry model that has no container yet a graph-citizen container (so models are visible, traversable and rippled like
/// everything else) and write the container id back into the registry. Best effort.
pub async fn register_model_containers(store: &Arc<dyn StoreAccess>) {
    let dir = models_dir();
    let mut reg = km::ModelRegistry::load(&dir);
    let mut changed = false;
    for m in reg.models.iter_mut() {
        if m.container_id.is_some() || m.status == "prepared" {
            continue;
        }
        let name = format!("Keyword model {} v{}", m.id, m.version);
        let now = now_ms() / 1000;
        let container = Container {
            global_state: GlobalState { container_id: 0, parent_id: PIPELINE_ROOT_ID, child_ids: vec![], child_count: 0, version: 1 },
            local_state: LocalState {
                metadata: Metadata {
                    container_type: ContainerType::Derived,
                    modality: Modality::Unknown,
                    created_at: now,
                    updated_at: now,
                    provenance: "keyword-model".to_string(),
                    permissions: 0,
                    owner_id: 0,
                    name: Some(name),
                    materialized_path: Some(format!("/Pipelines/keyword-models/{}", m.id)),
                },
                context: Context {
                    categories: vec![],
                    methodologies: vec![],
                    keywords: vec!["keyword-model".to_string(), m.kind.clone(), m.id.clone()],
                    topics: vec!["keyword search".to_string(), "ranking".to_string()],
                    relationships: vec![],
                    learned_associations: vec![],
                    keyword_meta: Default::default(),
                    scope: Default::default(),
                    embedding: None,
                },
                storage: StoragePointers { db_shard_id: None, vector_index_ref: None, object_store_path: None, compression_type: CompressionType::None },
                hints: TraversalHints::default(),
                integrity: IntegrityData::default(),
                file_context: None,
                code_context: None,
                text_context: None,
                external_ref: None,
            },
        };
        let container = match serde_json::to_value(&container) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Ok(id) = store.create_container(PIPELINE_ROOT_ID, container).await {
            m.container_id = Some(id);
            changed = true;
        }
    }
    if changed {
        let _ = reg.save(&dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facts_come_from_rolled_up_totals() {
        let mut meta: BTreeMap<String, TermMeta> = BTreeMap::new();
        meta.insert(
            CONTAINER_TOTALS_KEY.to_string(),
            TermMeta { source: String::new(), validated: None, hits: TermHits { returned: 9, returned_traced: 6, packed: 3, kept: 2, dropped: 1, applied: 4, human: 0 } },
        );
        meta.insert("static analysis".to_string(), TermMeta { source: "extracted".into(), validated: Some(true), hits: TermHits::default() });
        let f = facts_from_context(km::MatchClass::ExactTerm, 2, &meta, &["static analysis".to_string()], 77, 0.5);
        assert_eq!((f.returned, f.packed, f.kept, f.dropped, f.applied, f.last_accessed), (9, 3, 2, 1, 4, 77));
        assert!(f.validated && f.class == km::MatchClass::ExactTerm && f.matched_terms == 2);
        assert!((f.hotness - 0.5).abs() < 1e-6, "the stored hotness is read as is");
        let g = facts_from_context(km::MatchClass::ExactTerm, 1, &meta, &["other".to_string()], 0, f32::NAN);
        assert!(!g.validated);
        assert_eq!(g.hotness, 0.0, "a non-finite stored value is never used");
        let none = facts_from_context(km::MatchClass::ExactTerm, 1, &BTreeMap::new(), &[], 0, 0.0);
        assert_eq!(none.hotness, 0.0, "never returned = no observed use");
        assert_eq!(facts_from_context(km::MatchClass::ExactTerm, 1, &BTreeMap::new(), &[], 0, 7.0).hotness, 1.0, "clamped");
    }

    #[test]
    fn edge_relation_is_read_from_the_span_out_via_string() {
        assert_eq!(edge_relation_of("123 -Calls-> this").as_deref(), Some("Calls"));
        assert_eq!(edge_relation_of("9 -child-> this").as_deref(), Some("child"));
        assert_eq!(edge_relation_of("seed:keyword"), None);
        assert_eq!(edge_relation_of("7 --> this"), None);
    }

    #[test]
    fn bump_counts_by_event_kind() {
        let mut m = TermMeta::default();
        for k in ["returned", "returned", "packed", "kept", "applied", "dropped", "human", "nonsense"] {
            bump(&mut m, k);
        }
        assert_eq!((m.hits.returned, m.hits.packed, m.hits.kept, m.hits.applied, m.hits.dropped, m.hits.human), (2, 1, 1, 1, 1, 1));
    }

    #[test]
    fn hit_log_files_lists_archives_first_and_the_live_file_last() {
        let dir = std::env::temp_dir().join(format!("kwprog-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("keyword_hits.archive.1000.jsonl"), "a").unwrap();
        std::fs::write(dir.join("keyword_hits.archive.2000.jsonl"), "b").unwrap();
        std::fs::write(dir.join("keyword_hits.jsonl"), "live").unwrap();
        let files = hit_log_files_in(&dir);
        assert_eq!(files.len(), 3);
        assert!(files[0] < files[1] && files.last().unwrap().ends_with("keyword_hits.jsonl"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn origin_carries_scope_files_and_request_terms_for_the_scope_it_is_installed_in() {
        let o = Origin::for_request("orchestrate", Some(3), None, "explain the call order");
        assert!(!o.global && o.request_hash.len() == 16);
        with_origin(o, async {
            note_files([10, 11, 10]);
            note_request_terms(vec!["call order".to_string()]);
            let j = current_origin().unwrap().to_json();
            assert_eq!(j["workspace_id"], 3);
            assert_eq!(j["global"], false);
            assert_eq!(j["file_ids"], json!([10, 11]), "files are noted once each");
            assert_eq!(j["request_terms"], json!(["call order"]));
        })
        .await;
        assert!(current_origin().is_none(), "outside the scope there is no origin");
        assert!(Origin::for_request("orchestrate", None, None, "x").global, "no project and no workspace = global");
        assert!(Origin::actor("meta-loop").global);
    }

    #[test]
    fn a_pipelines_origin_header_is_parsed_and_bad_headers_are_ignored() {
        let o = Origin::from_header(r#"{"call_site":"text","workspace_id":2,"project_id":null,"task_id":9}"#).unwrap();
        assert_eq!((o.call_site.as_str(), o.workspace_id, o.project_id, o.task_id, o.global), ("text", Some(2), None, Some(9), false));
        assert!(Origin::from_header("{}").unwrap().global);
        assert!(Origin::from_header("").is_none() && Origin::from_header("not json").is_none());
    }

    #[test]
    fn only_methodology_and_blueprint_creations_with_request_terms_become_creation_pairs() {
        let ev = |ty: &str, terms: &[&str]| Ev {
            kind: "created".into(),
            container_type: ty.into(),
            trace: "t".into(),
            container_id: 1,
            terms: terms.iter().map(|s| s.to_string()).collect(),
            ids: vec![],
            yes: None,
            internal: false,
        };
        assert!(creation_pair_eligible(&ev("Methodology", &["static analysis"])));
        assert!(creation_pair_eligible(&ev("Blueprint", &["x"])));
        assert!(!creation_pair_eligible(&ev("Methodology", &[])), "no request terms (created outside a request) = no pair");
        assert!(!creation_pair_eligible(&ev("CoordinationEvent", &["x"])));
    }

    #[test]
    fn jurisdiction_rule_sets_are_never_gated_their_keywords_are_routing_keys() {
        assert!(!GATED_TYPES.contains(&"JurisdictionRuleSet"));
        let mut ctx = Context { keywords: vec!["is".into(), "Is".into(), "personal".into()], ..Default::default() };
        gate_keywords_on_write("JurisdictionRuleSet", &mut ctx);
        assert_eq!(ctx.keywords, vec!["is", "Is", "personal"], "untouched: not even the duplicate is dropped");
        assert!(ctx.keyword_meta.is_empty(), "no pending registration either");
    }

    #[test]
    fn the_write_gate_applies_the_shape_rules_and_registers_pending_terms_for_gated_types_only() {
        let mk = |kws: &[&str]| Context { keywords: kws.iter().map(|s| s.to_string()).collect(), ..Default::default() };
        let mut ctx = mk(&["Static Analysis", "static analysis", "  ", "this is far too long to be a keyword", "scope:global", "linting"]);
        gate_keywords_on_write("Methodology", &mut ctx);
        assert_eq!(ctx.keywords, vec!["Static Analysis", "scope:global", "linting"], "empty / sentence / case-duplicate dropped, marker kept");
        assert_eq!(ctx.keyword_meta.get("Static Analysis").map(|m| (m.validated, m.source.starts_with("created:"))), Some((None, true)));
        assert!(!ctx.keyword_meta.contains_key("scope:global"), "markers never go through the gate");
        ctx.keyword_meta.get_mut("linting").unwrap().validated = Some(true);
        gate_keywords_on_write("Methodology", &mut ctx);
        assert_eq!(ctx.keyword_meta.get("linting").unwrap().validated, Some(true), "an existing verdict is never reset by a later write");
        let mut sys = mk(&["tool_call", "a b c d e f g"]);
        gate_keywords_on_write("CoordinationEvent", &mut sys);
        assert_eq!(sys.keywords.len(), 2, "bookkeeping types are not gated");
        assert!(sys.keyword_meta.is_empty());
    }

    #[test]
    fn bookkeeping_tags_are_namespaced_once_and_only_where_the_emitter_layout_is_known() {
        let mk = |kws: &[&str]| Context { keywords: kws.iter().map(|s| s.to_string()).collect(), ..Default::default() };
        // CoordinationEvent: [kind, agent, file:…, scope…]
        let mut ev = mk(&["tool_call", "sensor-tick", "file:a.rs", "scope:global"]);
        namespace_bookkeeping_tags("CoordinationEvent", &mut ev);
        assert_eq!(ev.keywords, vec!["kind:tool_call", "agent:sensor-tick", "file:a.rs", "scope:global"]);
        let again = ev.keywords.clone();
        namespace_bookkeeping_tags("CoordinationEvent", &mut ev);
        assert_eq!(ev.keywords, again, "idempotent");
        // Derived: only the run-span literal; content terms untouched
        let mut d = mk(&["run-span", "amt-main", "loop:i_loop"]);
        namespace_bookkeeping_tags("Derived", &mut d);
        assert_eq!(d.keywords, vec!["span:run", "amt-main", "loop:i_loop"]);
        let mut root = mk(&["run-root", "loop:i_loop"]);
        namespace_bookkeeping_tags("Derived", &mut root);
        assert_eq!(root.keywords, vec!["span:root", "loop:i_loop"]);
        // McpResult / McpEntity
        let mut r = mk(&["uidocument_ingest", "mcp_result", "ws:1"]);
        namespace_bookkeeping_tags("McpResult", &mut r);
        assert_eq!(r.keywords, vec!["tool:uidocument_ingest", "kind:mcp_result", "ws:1"]);
        let mut n = mk(&["uidocument_ingest", "StyleRule", "call:tool:x"]);
        n.keyword_meta.insert("uidocument_ingest".into(), TermMeta { source: "created:unknown".into(), validated: None, hits: Default::default() });
        n.keyword_meta.insert("StyleRule".into(), TermMeta { source: "extracted".into(), validated: Some(true), hits: Default::default() });
        namespace_bookkeeping_tags("McpEntity", &mut n);
        assert_eq!(n.keywords, vec!["tool:uidocument_ingest", "node:stylerule", "call:tool:x"]);
        assert!(!n.keyword_meta.contains_key("uidocument_ingest"), "a pending-only registration goes with the renamed tag");
        assert!(n.keyword_meta.contains_key("StyleRule"), "a verdict is kept");
        // chat + experience containers (follow-up): exact literals only
        let mut ct = mk(&["chat", "chat_turn", "chatsession:proj:3", "proj:3", "task:77"]);
        namespace_bookkeeping_tags("ChatTurn", &mut ct);
        assert_eq!(ct.keywords, vec!["kind:chat", "kind:chat_turn", "chatsession:proj:3", "proj:3", "task:77"]);
        let mut cs = mk(&["chat", "chat_session", "chatsession:global", "global"]);
        namespace_bookkeeping_tags("ChatSession", &mut cs);
        assert_eq!(cs.keywords, vec!["kind:chat", "kind:chat_session", "chatsession:global", "scope:global"]);
        assert!(crate::types::container::ContainerScope::from_markers(&cs.keywords).global, "the typed scope reads the namespaced global");
        let mut ex = mk(&["task", "no-emotion-update", "task_failure", "failure"]);
        namespace_bookkeeping_tags("Experience", &mut ex);
        assert_eq!(ex.keywords, vec!["kind:task", "flag:no-emotion-update", "outcome:task_failure", "outcome:failure"]);
        let once = ex.keywords.clone();
        namespace_bookkeeping_tags("Experience", &mut ex);
        assert_eq!(ex.keywords, once, "idempotent");
        let mut other = mk(&["task", "success"]);
        namespace_bookkeeping_tags("Methodology", &mut other);
        assert_eq!(other.keywords, vec!["task", "success"], "content types keep their words");
        // any other type: untouched, including content words that look like tags
        let mut m = mk(&["tool_call", "Link", "image"]);
        namespace_bookkeeping_tags("Methodology", &mut m);
        namespace_bookkeeping_tags("ModalityGraph", &mut m);
        assert_eq!(m.keywords, vec!["tool_call", "Link", "image"]);
        // a tag that cannot be a marker stays as it is
        let mut odd = mk(&[":bad", "agent"]);
        namespace_bookkeeping_tags("CoordinationEvent", &mut odd);
        assert_eq!(odd.keywords, vec![":bad", "agent"]);
    }

    #[test]
    fn has_tag_accepts_both_forms_and_nothing_else() {
        let k = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        assert!(has_tag(&k(&["tool_call", "x"]), "kind", "tool_call"), "legacy plain form");
        assert!(has_tag(&k(&["kind:tool_call"]), "kind", "tool_call"), "namespaced form");
        assert!(!has_tag(&k(&["agent:tool_call"]), "kind", "tool_call"), "another namespace is not the kind");
        assert!(!has_tag(&k(&["kind:tool_calls", "tool_call_x"]), "kind", "tool_call"));
        assert!(!has_tag(&[], "kind", "tool_call"));
    }

    #[test]
    fn strict_mode_keeps_only_markers_and_validated_terms() {
        let mut meta: BTreeMap<String, TermMeta> = BTreeMap::new();
        meta.insert("api".into(), TermMeta { source: "extracted".into(), validated: Some(true), hits: Default::default() });
        meta.insert("guess".into(), TermMeta { source: "expansion".into(), validated: None, hits: Default::default() });
        meta.insert("rejected".into(), TermMeta { source: "expansion".into(), validated: Some(false), hits: Default::default() });
        meta.insert("src/main.rs".into(), TermMeta { source: "identifier".into(), validated: None, hits: Default::default() });
        let kws: Vec<String> = ["api", "guess", "rejected", "src/main.rs", "scope:global", "no-meta"].iter().map(|s| s.to_string()).collect();
        assert_eq!(searchable_keywords(&kws, &meta, false), kws, "non-strict = everything");
        assert_eq!(searchable_keywords(&kws, &meta, true), vec!["api", "src/main.rs", "scope:global"], "strict = validated + deterministic facts + markers");
        assert!(!term_is_validated(None) && term_is_validated(meta.get("api")) && !term_is_validated(meta.get("rejected")));
        // ablation: expansion-sourced terms ("guess", "rejected") are left out of matching, nothing else changes
        assert_eq!(filter_keywords(&kws, &meta, false, true), vec!["api", "src/main.rs", "scope:global", "no-meta"]);
        assert_eq!(filter_keywords(&kws, &meta, true, true), vec!["api", "src/main.rs", "scope:global"]);
    }

    #[test]
    fn system_generated_work_is_flagged_internal_and_read_back_from_the_log() {
        let o = Origin::for_request("orchestrate", None, Some(3), "AMT re-expansion container 40211 InitialBuild").mark_internal("amt-loop");
        assert!(o.internal && o.call_site == "amt-loop");
        let line = json!({"event": "search", "trace": "t", "container_id": 1, "terms": ["amt"], "ids": [1], "origin": o.to_json()}).to_string();
        assert!(parse_ev(&line).unwrap().internal, "the flag survives the log round trip");
        let user = Origin::for_request("orchestrate", None, Some(3), "explain the call order");
        let line = json!({"event": "search", "trace": "t", "origin": user.to_json()}).to_string();
        assert!(!parse_ev(&line).unwrap().internal);
        assert!(!parse_ev("{\"event\":\"search\"}").unwrap().internal, "no origin = not internal (older log lines)");
    }

    #[test]
    fn with_no_active_model_the_hook_is_exactly_v0() {
        let c = |id: u64, class: km::MatchClass| km::Candidate { id, terms: vec!["x".into()], facts: km::RankFacts::new(class, 1) };
        let cands = vec![c(1, km::MatchClass::WholeToken), c(2, km::MatchClass::ExactTerm)];
        std::env::set_var("OZONE_KEYWORD_BACKENDS", "off");
        assert_eq!(order_for_search(&["x".to_string()], &cands), vec![2, 1]);
        std::env::remove_var("OZONE_KEYWORD_BACKENDS");
    }
}
