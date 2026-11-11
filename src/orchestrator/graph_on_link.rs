//! GRAPH ON LINK — the full graph creation process runs by itself when a file is linked, changes, or the workspace loads (operator 2026-11-11:
//! "rerun through me the full graph creation process on file link itself and when a file is linked and folder and etc"; "every modality should have
//! something like a call graph working with entrypoints").
//!
//! WHAT IT RUNS: exactly what `/orchestrate` runs for an attached file (`orchestrator/mod.rs` STEP 0, `graphs.rs::process_modality`):
//!   modality detection (`ModalityType::from_path`, the one shared extension table) → the shared reader (`file_reader.rs`: whole text for text-shaped
//!   modalities, the typed by-path input for the rest, a recorded reason when a file needs parameters it does not state) → the modality pipeline's `Analyze` of the WHOLE file
//!   (never cut, never refused for size) → the same pipeline's `CreateGraph` (`link_to_existing: true` where the action has it — text / code / math; the durable per-file ModalityGraph
//!   container, linked at creation, keywords/validation through the ordinary write path) → the graph's ENTRIES (`graph_fanout::digest_graph`: explicit
//!   `analysis.entry_points` for code, otherwise the roots — nodes with no inbound structural edge — for every other modality).
//!   There is no prompt, so the walk's prompt-relative file-role classification does not apply; what applies is recorded (see REPORT).
//!
//! TRIGGERS (all land in ONE queue, processed one at a time):
//!   * a `FileReference` container is CREATED (file link; a folder link creates one per direct child file, so a folder is covered file by file);
//!   * a `FileReference` is UPDATED (the file beacon rippled a real mtime/size change — `file_beacon.rs`);
//!   * WORKSPACE LOAD: one sweep at boot over every registered `FileReference` (a file edited while the host was down; a link made before this
//!     actor existed). The sweep is the same code path as the events.
//!
//! IDEMPOTENT BY CONTENT: the file is hashed (blake3, streamed — no size ceiling) and the result is recorded in `graph_on_link_ledger.json`
//! (data dir). The same content is never processed twice; a changed file is a NEW task (`rerun_of` the previous one) — a completed task is final.
//!
//! WORK-ORDER SEMANTICS: each (file, content) is a `wo_graph` task in the existing task system (`task/mod.rs create_work_task`); its checklist is
//!   `classify` (modality + pipeline identified) · `analyze` (the pipeline analysed the file) · `graph` (the ModalityGraph container was created) ·
//!   `entries` (the graph's entries were read back). Each item is ticked ONLY by the real event that completes it; the task completes exactly when
//!   all four are done (`TaskManager::settle`). A failed step leaves the task OPEN with the error recorded in the ledger and logged; the next trigger for
//!   the same content resumes from the failed step (failure pauses and resumes; nothing is dropped, nothing is fabricated).
//!
//! GUARDS: a host-memory floor only (`OZONE_MIN_AVAIL_MB`, default 1536 — the same silent-death guard the task executor uses): below it the worker
//!   WAITS, it never drops a job. No cap on files, queue length or file size. Runs at `CallPriority::Loop`, labelled origin `graph-on-link`.
//! PRIVACY: reads `FileReference` containers and the files they point at; announces nothing (no marketplace/peer publication, nothing from ui-ingest
//!   or McpEntity); the ledger and tasks stay in the local data dir.
//! SWITCH: `OZONE_GRAPH_ON_LINK` — ON unless set to `off`, `0` or `false` (a host opts out in its own config).
//!
//! LINK ↔ WALK ALIGNMENT (operator 2026-11-11: file link ALWAYS runs first and captures the non-semantic / structural graph; the walk adds the intelligence
//! on top; both know the other's state — docs/evidence/graph-link-walk-alignment/REPORT.md). The ledger row of a file is the one state both read:
//!   `hash` (the content), `graph_id`, `structural_done` (the pipeline's native graph exists), `created_by` (`link` | `walk`), `task_id` (the wo_graph task),
//!   `semantic_layers` (append-only: what the walk later added, e.g. its prompt-relative file role — it never replaces or edits the structural graph).
//! Both paths take the SAME per-path async lock (`claim`) before they look at the ledger, so there is never a parallel second graph for one content:
//!   * the worker runs first (normal case): the walk finds `graph_id` for the current content hash and REUSES it;
//!   * the worker is mid-run: the walk AWAITS the lock (it never fails open and never builds a second graph), then reuses;
//!   * nothing exists yet (queue not reached / switch off / earlier failure): the walk builds the graph through its own pipeline calls and records it in the
//!     ledger (`record_created`); the worker later finds `graph_id` for that hash, skips analyze/create and completes the remaining checklist items;
//!   * the file changed since: the hash differs, so nothing is reused — a NEW graph and a NEW wo_graph task (`rerun_of` the previous one, completed is final).

use super::{PipelineExecutor, StoreAccess};
use crate::task::TaskManager;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::RwLock;

const FILE_NAME_PREFIX: &str = "File: ";
const LEDGER_FILE: &str = "graph_on_link_ledger.json";
const TASK_KIND: &str = "wo_graph";

/// ON unless explicitly switched off (operator ruling 2026-11-11: every switch defaults ON).
pub fn enabled() -> bool {
    !matches!(
        std::env::var("OZONE_GRAPH_ON_LINK").ok().map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("off") | Some("0") | Some("false")
    )
}

fn data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string()))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ── ledger ────────────────────────────────────────────────────────────────────────────────────────────

fn ledger_path() -> std::path::PathBuf {
    data_dir().join(LEDGER_FILE)
}

fn load_ledger() -> serde_json::Map<String, Value> {
    std::fs::read_to_string(ledger_path())
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn save_ledger(rows: &serde_json::Map<String, Value>) {
    let path = ledger_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // write-then-rename: a crash never leaves a half-written ledger
    let tmp = path.with_extension("json.tmp");
    if let Ok(text) = serde_json::to_string_pretty(&Value::Object(rows.clone())) {
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

/// One lock around every read-modify-write of the ledger file (the worker and the walk both write it).
static LEDGER_LOCK: Mutex<()> = Mutex::new(());

fn put_row(container_id: u64, row: Value) {
    put_row_key(&container_id.to_string(), row);
}

/// Write one ledger row. For the SAME content the keys the walk owns (`semantic_layers`, `previous_task_id`) and who created the graph (`created_by`) are
/// carried over from the old row, so a worker update never erases what the walk recorded (and the other way round).
fn put_row_key(key: &str, mut row: Value) {
    let _g = LEDGER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut rows = load_ledger();
    let old = rows.get(key).cloned().unwrap_or(Value::Null);
    let same = old["hash"].as_str().is_some() && old["hash"] == row["hash"];
    if let Some(r) = row.as_object_mut() {
        let created_by = if same { old["created_by"].as_str().unwrap_or("link").to_string() } else { "link".to_string() };
        r.entry("created_by".to_string()).or_insert(json!(created_by));
        if same {
            for k in ["semantic_layers", "previous_task_id"] {
                if let Some(v) = old.get(k) {
                    r.entry(k.to_string()).or_insert(v.clone());
                }
            }
        }
    }
    rows.insert(key.to_string(), row);
    save_ledger(&rows);
}

// ── link ↔ walk: the shared claim ─────────────────────────────────────────────────────────────────────

static PATH_LOCKS: OnceLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> = OnceLock::new();

/// The async lock of one file path (held across the whole structural-graph job by whoever builds it; never held across the std ledger lock).
fn path_lock(path: &str) -> Arc<tokio::sync::Mutex<()>> {
    let m = PATH_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut g = m.lock().unwrap_or_else(|e| e.into_inner());
    g.entry(path.to_string()).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
}

/// Which ledger row belongs to this (path, content) and its current value. Rows are keyed by the FileReference container id; the walk may not know it,
/// so a row for the same path is found by scanning, and with no row at all the walk's row is keyed `path:<path>` — the worker adopts it (same content)
/// into the container-id row the first time it claims the path.
fn resolve_row(path: &str, hash: &str, container_id: Option<u64>) -> (String, Value) {
    let _g = LEDGER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut rows = load_ledger();
    let alias = format!("path:{path}");
    let key = match container_id {
        Some(c) => c.to_string(),
        None => rows
            .iter()
            .filter(|(_, v)| v["path"].as_str() == Some(path))
            .max_by_key(|(_, v)| (v["hash"].as_str() == Some(hash), v["at"].as_u64().unwrap_or(0)))
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| alias.clone()),
    };
    let mut prev = rows.get(&key).cloned().unwrap_or(Value::Null);
    if container_id.is_some() && prev["hash"].as_str() != Some(hash) {
        if let Some(mut a) = rows.get(&alias).cloned() {
            if a["hash"].as_str() == Some(hash) {
                // the walk built this content's graph before the link row existed: the container-id row adopts it
                if a.get("previous_task_id").map(|v| v.is_null()).unwrap_or(true) {
                    if let Some(t) = prev["task_id"].as_u64() {
                        a["previous_task_id"] = json!(t);
                    }
                }
                rows.remove(&alias);
                rows.insert(key.clone(), a.clone());
                save_ledger(&rows);
                prev = a;
            }
        }
    }
    (key, prev)
}

/// The holder of a file path's lock plus what the ledger said when it was taken. Dropping it releases the path.
pub(crate) struct Claim {
    _guard: tokio::sync::OwnedMutexGuard<()>,
    key: String,
    path: String,
    hash: String,
    prev: Value,
    existing: Option<u64>,
}

impl Claim {
    /// The graph already built for THIS content (by the link worker or an earlier walk), when its container still exists.
    pub(crate) fn existing_graph(&self) -> Option<u64> {
        self.existing
    }
    /// The ledger row as it was when the claim was taken.
    pub(crate) fn prev(&self) -> &Value {
        &self.prev
    }
    /// The walk built the structural graph itself: record it so the link worker (and every later walk) sees it. Same content → same row; a changed file
    /// keeps the old task as `previous_task_id` so the worker's new task is `rerun_of` it.
    pub(crate) fn record_created(&self, graph_id: u64, modality: &str, pipeline_id: u64, project_id: u64) {
        let mut row = json!({
            "path": self.path, "project_id": project_id, "hash": self.hash, "modality": modality, "pipeline_id": pipeline_id,
            "status": "graph_created", "trigger": "walk", "created_by": "walk", "graph_id": graph_id, "structural_done": true, "at": now_secs(),
            "note": "graph built by the walk before the link worker reached this file — the link task completes the remaining items",
        });
        let same = self.prev["hash"].as_str() == Some(self.hash.as_str());
        if let Some(r) = row.as_object_mut() {
            if same {
                if let Some(t) = self.prev.get("task_id").filter(|v| !v.is_null()) {
                    r.insert("task_id".to_string(), t.clone());
                }
            } else if let Some(t) = self.prev["task_id"].as_u64() {
                r.insert("previous_task_id".to_string(), json!(t));
            }
        }
        put_row_key(&self.key, row);
    }
}

/// Take the path's lock (AWAITING a worker or walk already building this file — never failing open, never building a second graph), then read the
/// ledger. `container_id` is the FileReference id when the caller knows it (the worker); the walk passes `None`.
pub(crate) async fn claim(store: &dyn StoreAccess, path: &str, hash: &str, container_id: Option<u64>) -> Claim {
    let guard = path_lock(path).lock_owned().await;
    let (key, prev) = resolve_row(path, hash, container_id);
    let built = matches!(prev["status"].as_str(), Some("graph_created") | Some("complete"));
    let existing = match prev["graph_id"].as_u64() {
        Some(g) if built && prev["hash"].as_str() == Some(hash) && matches!(store.get_container(g).await, Ok(Some(_))) => Some(g),
        _ => None,
    };
    Claim { _guard: guard, key, path: path.to_string(), hash: hash.to_string(), prev, existing }
}

/// The walk's entry: hash the file as it is now and take its claim. `None` when the file cannot be hashed (missing / unreadable): the walk then proceeds
/// exactly as before and records nothing.
pub(crate) async fn claim_for_file(store: &dyn StoreAccess, path: &str) -> Option<Claim> {
    let p = path.to_string();
    let hash = tokio::task::spawn_blocking(move || hash_file(&p)).await.ok()?.ok()?;
    Some(claim(store, path, &hash, None).await)
}

/// The walk's intelligence about a file's graph (its prompt-relative role, …): appended to the row's `semantic_layers`, never touching the structural
/// fields. The row is found by graph id. Append-only (each walk adds its own entry).
pub(crate) fn record_semantic_layer(graph_id: u64, kind: &str, detail: Value) {
    let _g = LEDGER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut rows = load_ledger();
    let mut hit = false;
    for (_, v) in rows.iter_mut() {
        if v["graph_id"].as_u64() != Some(graph_id) {
            continue;
        }
        if let Some(o) = v.as_object_mut() {
            let layers = o.entry("semantic_layers".to_string()).or_insert_with(|| json!([]));
            if let Some(a) = layers.as_array_mut() {
                a.push(json!({"kind": kind, "by": "walk", "at": now_secs(), "detail": detail.clone()}));
                hit = true;
            }
        }
    }
    if hit {
        save_ledger(&rows);
    }
}

/// Read-only view for `GET /callgraph/files`: the ledger rows (optionally of one project), as stored, plus each row's task state.
pub fn ledger_rows(project_id: Option<u64>) -> Vec<Value> {
    let mut out: Vec<Value> = load_ledger()
        .into_iter()
        .map(|(k, mut v)| {
            v["file_reference_id"] = json!(k.parse::<u64>().ok());
            v
        })
        .filter(|v| project_id.map(|p| v["project_id"].as_u64() == Some(p)).unwrap_or(true))
        .collect();
    out.sort_by_key(|v| std::cmp::Reverse(v["at"].as_u64().unwrap_or(0)));
    out
}

// ── entry concept per modality ────────────────────────────────────────────────────────────────────────

/// What an ENTRY means for each modality's graph. The traversal rule is generic (`graph_fanout.rs`): explicit `analysis.entry_points` when the
/// pipeline recorded them (code today), else the ROOTS — nodes with no inbound structural edge, which for every modality pipeline is its root node
/// type (verified against each pipeline's `*NodeType` enum, see docs/evidence/graph-on-link/REPORT.md). `explicit` says which case applies.
pub fn entry_concepts() -> Value {
    let rows: &[(&str, bool, &str)] = &[
        ("code", true, "functions with no caller in the file (main, exports, handlers) — recorded by the pipeline as analysis.entry_points; the call sequence follows Calls edges"),
        ("text", false, "the document's top-level sentences / clauses (roots of the grammar structure); reading order follows the order edges"),
        ("math", false, "the root expression or proof; steps follow the derivation edges"),
        ("image", false, "the image root node; objects, regions, text and faces hang under it"),
        ("audio", false, "the audio root node; segments, speakers and transcribed words follow in time order"),
        ("video", false, "the video root node; scenes → shots → keyframes follow in time order"),
        ("chemistry", false, "the molecule / reaction roots; atoms, bonds, groups and pathways hang under them"),
        ("dna", false, "the sequence root; genes, exons, variants and motifs hang under it"),
        ("eeg", false, "the recording root; channels, epochs, events and bands hang under it"),
        ("3d", false, "the scene root; collections, objects, meshes and armatures hang under it"),
        ("sound", false, "the sound-scene root; detected events and vocalizations hang under it"),
        ("biology", false, "the biology-scene root; pathways, cells and signalling events hang under it"),
        ("proteomics", false, "the proteome root; proteins, domains and structures hang under it"),
        ("haptic", false, "the haptic-scene root; contact events and force profiles hang under it"),
        ("thermal", false, "the thermal-scene root; frames, hotspots and contours hang under it"),
        ("depth", false, "the depth-scene root; point clouds, maps, planes and meshes hang under it"),
        ("imu", false, "the IMU-scene root; sample windows, attitude estimates and trajectory segments hang under it"),
        ("geospatial", false, "the geo-scene root; locations, regions, routes and trajectories hang under it"),
        ("electromagnetic", false, "the EM-scene root; spectrum snapshots, bands and detected signals hang under it"),
        ("bci", false, "the BCI session root; signals, units and epochs hang under it"),
        ("parametric_cad", false, "the CAD document root; parts, assemblies, features and sketches follow the parametric driver chain"),
        ("kinematics", false, "the kinematic system root; chains run base → end effector"),
        ("control_systems", false, "the control-system root; plant, controller, sensor and actuator form the feedback structure"),
        ("network_topology", false, "the network-scene root; hosts, subnets, zones and services hang under it"),
        ("radar", false, "the radar-scene root; range-Doppler cells, targets and tracks hang under it"),
        ("sonar", false, "the sonar-scene root; echoes, targets and tracks hang under it"),
        ("hyperspectral", false, "the hyperspectral-scene root; signatures, endmembers and abundances hang under it"),
        ("uidocument", false, "no modality pipeline binary exists for this modality yet (a library crate) — no graph is created"),
    ];
    Value::Array(rows.iter().map(|(m, explicit, text)| json!({"modality": m, "explicit_entry_points": explicit, "entry": text})).collect())
}

// ── the job queue ─────────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Job {
    container_id: u64,
    path: String,
    project_id: u64,
    reason: &'static str,
}

/// Boot spawn (lib.rs, beside the ripple actors): the event subscriber, the workspace-load sweep and the single worker.
pub fn spawn(store: Arc<dyn StoreAccess>, executor: Arc<dyn PipelineExecutor>, tasks: Arc<RwLock<TaskManager>>) {
    if !enabled() {
        tracing::info!("graph-on-link: OFF (OZONE_GRAPH_ON_LINK) — file links will not create graphs on their own");
        return;
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Job>();

    // 1. events: a file linked / a file changed
    {
        let store = store.clone();
        let tx = tx.clone();
        let mut events = crate::graph_events::GraphEventHub::global().subscribe();
        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(evt) => {
                        if evt.container_type != "FileReference" || !matches!(evt.event, "created" | "updated" | "linked") || evt.container_id == 0 {
                            continue;
                        }
                        if let Some(job) = job_for(&*store, evt.container_id, if evt.event == "updated" { "file-changed" } else { "file-linked" }).await {
                            let _ = tx.send(job);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(lagged = n, "graph-on-link: missed graph events — the next boot sweep covers them");
                    }
                    Err(_) => break,
                }
            }
        });
    }

    // 2. workspace load: one sweep over every registered FileReference (the same queue, the same idempotent check)
    {
        let store = store.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            // let the store and the workspace graph come up first; a sweep that finds nothing is not an error
            tokio::time::sleep(Duration::from_secs(20)).await;
            let refs = crate::file_beacon::discover_file_references(&*store).await;
            tracing::info!(files = refs.len(), "graph-on-link: workspace-load sweep");
            for (container_id, _path) in refs {
                if let Some(job) = job_for(&*store, container_id, "workspace-load").await {
                    let _ = tx.send(job);
                }
            }
        });
    }

    // 3. the worker: one job at a time, never dropping, waiting while host memory is below the floor
    tokio::spawn(crate::pipeline::gate::with_priority(
        crate::pipeline::gate::CallPriority::Loop,
        crate::keyword_program::with_origin(crate::keyword_program::Origin::actor("graph-on-link"), async move {
            while let Some(job) = rx.recv().await {
                wait_for_memory().await;
                if let Err(e) = process_job(&*store, &*executor, &tasks, &job).await {
                    tracing::warn!(container_id = job.container_id, path = %job.path, reason = job.reason, error = %e, "graph-on-link: job did not finish (recorded; the next trigger resumes it)");
                }
            }
        }),
    ));
}

async fn wait_for_memory() {
    let floor: u64 = std::env::var("OZONE_MIN_AVAIL_MB").ok().and_then(|v| v.parse().ok()).unwrap_or(1536);
    let mut warned = false;
    while crate::mem_available_mb() < floor {
        if !warned {
            tracing::warn!(floor, "graph-on-link: memory floor — waiting (jobs are never dropped)");
            warned = true;
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

/// A job for one FileReference container: its real path (the `File: <path>` naming every linking pipeline writes) and its project.
async fn job_for(store: &dyn StoreAccess, container_id: u64, reason: &'static str) -> Option<Job> {
    let c = store.get_container(container_id).await.ok()??;
    let path = c["local_state"]["metadata"]["name"].as_str()?.strip_prefix(FILE_NAME_PREFIX)?.to_string();
    // the project is the parent container when that parent is a Project (a no-project ad-hoc link has project 0, like the walk)
    let parent_id = c["global_state"]["parent_id"].as_u64().unwrap_or(0);
    let mut project_id = 0;
    if parent_id != 0 {
        if let Ok(Some(p)) = store.get_container(parent_id).await {
            if p["local_state"]["metadata"]["container_type"].as_str() == Some("Project") {
                project_id = parent_id;
            }
        }
    }
    Some(Job { container_id, path, project_id, reason })
}

// ── hashing ───────────────────────────────────────────────────────────────────────────────────────────

/// blake3 of the whole file, streamed (no size ceiling, no whole-file buffer).
fn hash_file(path: &str) -> Result<String, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize().as_bytes()))
}

// ── payloads (the same shapes the walk sends; see graphs.rs::process_modality / mod.rs STEP 0) ────────

use super::graphs::file_reader::{self, Prepared};

// ── one job ───────────────────────────────────────────────────────────────────────────────────────────

fn ledger_row(job: &Job, hash: &str, modality: &str, pipeline_id: u64, status: &str, extra: Value) -> Value {
    let mut row = json!({
        "path": job.path, "project_id": job.project_id, "hash": hash, "modality": modality, "pipeline_id": pipeline_id,
        "status": status, "trigger": job.reason, "at": now_secs(),
        "structural_done": matches!(status, "graph_created" | "complete"),
    });
    if let (Some(r), Some(e)) = (row.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            r.insert(k.clone(), v.clone());
        }
    }
    row
}

async fn process_job(store: &dyn StoreAccess, executor: &dyn PipelineExecutor, tasks: &Arc<RwLock<TaskManager>>, job: &Job) -> Result<(), String> {
    // 1. the content as it is NOW
    let hash_path = job.path.clone();
    let hashed = tokio::task::spawn_blocking(move || hash_file(&hash_path)).await.map_err(|e| e.to_string()).and_then(|r| r);
    let hash = match hashed {
        Ok(h) => h,
        Err(e) => {
            // a linked file that is gone / unreadable: recorded, not retried on its own and not an error of this actor
            let prev = load_ledger().get(&job.container_id.to_string()).cloned().unwrap_or(Value::Null);
            if prev["status"].as_str() != Some("file_unreadable") {
                put_row(job.container_id, ledger_row(job, "", "", 0, "file_unreadable", json!({"error": e})));
            }
            return Ok(());
        }
    };
    // the path's lock: if the walk is building this file's graph right now we wait for it (and then find it in the ledger); the walk waits for us the same way
    let claim = claim(store, &job.path, &hash, Some(job.container_id)).await;
    let prev = claim.prev().clone();
    let same_content = prev["hash"].as_str() == Some(hash.as_str());
    if same_content && matches!(prev["status"].as_str(), Some("complete") | Some("unreadable") | Some("not_text_readable")) {
        return Ok(()); // this content was already processed: the graph exists (or the file has no reader without invented parameters, recorded)
    }

    // 2. classify: the one shared extension table → the modality pipeline (unknown → text, exactly as the walk treats it)
    let modality = crate::types::modality::ModalityType::from_path(&job.path).path_name().to_string();
    let named_pipeline = super::graphs::pipeline_id_for_modality(&modality);
    let pipeline_id = if named_pipeline == 0 { 100 } else { named_pipeline };

    // 3. read through the ONE shared reader (file_reader.rs): text-shaped modalities as the WHOLE text (never cut), the rest as the typed input their
    //    pipeline takes (it opens the file by path itself). A file with no reader that avoids inventing parameters is recorded with the reason, not fed
    //    to a pipeline that cannot take it and not given made-up values.
    let prepared = file_reader::prepare(&job.path, pipeline_id).await;
    if let Prepared::Unreadable(reason) = &prepared {
        put_row(job.container_id, ledger_row(job, &hash, &modality, pipeline_id, "unreadable", json!({"error": reason})));
        return Ok(());
    }

    // 4. the work-order task for this (file, content): a changed file is a NEW task, `rerun_of` the previous one (completed is final)
    let key = format!("file:{}:{}", job.container_id, &hash[..16.min(hash.len())]);
    let title = format!("Graph creation for {} ({})", job.path, modality);
    let items: Vec<(String, String)> = [
        ("classify", "the file's modality and modality pipeline are identified"),
        ("analyze", "the modality pipeline analysed the whole file"),
        ("graph", "the ModalityGraph container was created and linked"),
        ("entries", "the graph's entries (call-graph entry points / roots) were read back"),
    ]
    .iter()
    .map(|(k, t)| (k.to_string(), t.to_string()))
    .collect();
    let task_id = {
        let tm = tasks.read().await;
        // idempotent on (parent, kind, key) for the same content; `rerun_of` only when the content changed since the last task
        // same content: no rerun — unless the walk built the graph first and left the previous content's task for us (`previous_task_id`) and no task exists yet
        let rerun_of = if same_content {
            if prev["task_id"].is_null() { prev["previous_task_id"].as_u64() } else { None }
        } else {
            prev["task_id"].as_u64()
        };
        tm.create_work_task(None, TASK_KIND, &key, &title, &items, rerun_of, None).await
    };
    let Some(task_id) = task_id else { return Err("could not create the work-order task".into()) };
    let tm = tasks.read().await;
    tm.complete_item(task_id, "classify", &format!("modality {modality}, pipeline {pipeline_id}"), "graph-on-link").await;

    // resume point: a graph already created for THIS content (an earlier run stopped before `entries`)
    let mut graph_id = if same_content { claim.existing_graph() } else { None };

    // 5. analyze + create the graph (skipped when this content already has its graph)
    if graph_id.is_none() {
        let analyze_in = match prepared {
            Prepared::Text(text) => file_reader::text_analyze_input(pipeline_id, &text, &[modality.clone()], &job.path),
            Prepared::Action(a) => a,
            Prepared::Unreadable(_) => return Ok(()), // handled above
        };
        let analysis = match executor.execute(pipeline_id, analyze_in).await {
            Ok(a) => a,
            Err(e) => {
                put_row(job.container_id, ledger_row(job, &hash, &modality, pipeline_id, "failed_analyze", json!({"task_id": task_id, "error": e.clone()})));
                return Err(format!("analyze failed: {e}"));
            }
        };
        tm.complete_item(task_id, "analyze", "pipeline Analyze returned", "graph-on-link").await;
        let created = match executor.execute(pipeline_id, file_reader::create_graph_input(pipeline_id, &analysis, job.project_id)).await {
            Ok(g) => g,
            Err(e) => {
                put_row(job.container_id, ledger_row(job, &hash, &modality, pipeline_id, "failed_create_graph", json!({"task_id": task_id, "error": e.clone()})));
                return Err(format!("CreateGraph failed: {e}"));
            }
        };
        let Some(gid) = file_reader::graph_id_of(pipeline_id, &created) else {
            let msg = "CreateGraph returned no graph_id".to_string();
            put_row(job.container_id, ledger_row(job, &hash, &modality, pipeline_id, "failed_create_graph", json!({"task_id": task_id, "error": msg.clone()})));
            return Err(msg);
        };
        tm.complete_item(task_id, "graph", &format!("ModalityGraph container {gid} created (link_to_existing)"), "graph-on-link").await;
        graph_id = Some(gid);
        put_row(job.container_id, ledger_row(job, &hash, &modality, pipeline_id, "graph_created", json!({"task_id": task_id, "graph_id": gid})));
    } else {
        // resumed: the earlier run already ticked analyze/graph; complete_item is idempotent and keeps the original evidence
        let by = if prev["created_by"].as_str() == Some("walk") { "built by the walk's attached-file step (same pipeline calls)" } else { "created in an earlier run of this content" };
        tm.complete_item(task_id, "analyze", &format!("analysed — {by}"), "graph-on-link").await;
        tm.complete_item(task_id, "graph", &format!("ModalityGraph container {} {by}", graph_id.unwrap_or(0)), "graph-on-link").await;
    }
    let gid = graph_id.unwrap_or(0);

    // 6. entries: read the graph back and digest it by the modality's rule (explicit entry points for code, else the roots)
    let digest = read_digest(store, gid).await;
    match digest {
        Some(d) => {
            let ev = format!("{} nodes, {} edges; entries: {}", d.nodes, d.edges, if d.entries.is_empty() { "none found".to_string() } else { d.entries.join(", ") });
            tm.complete_item(task_id, "entries", &ev, "graph-on-link").await;
            put_row(
                job.container_id,
                ledger_row(job, &hash, &modality, pipeline_id, "complete", json!({"task_id": task_id, "graph_id": gid, "nodes": d.nodes, "edges": d.edges, "entries": d.entries})),
            );
        }
        None => {
            // the graph container exists but its graph file could not be read back yet: the task stays OPEN, honestly; the next trigger retries only this step
            put_row(job.container_id, ledger_row(job, &hash, &modality, pipeline_id, "graph_created", json!({"task_id": task_id, "graph_id": gid, "note": "graph file not readable yet — entries pending"})));
        }
    }
    Ok(())
}

/// The digest of a ModalityGraph container's graph FILE (`storage.object_store_path`, relative to the data dir — the same read `span_out` does).
pub(crate) async fn read_digest(store: &dyn StoreAccess, graph_container_id: u64) -> Option<super::graph_fanout::GraphDigest> {
    let c = store.get_container(graph_container_id).await.ok()??;
    let rel = c["local_state"]["storage"]["object_store_path"].as_str()?.to_string();
    let base = data_dir().to_string_lossy().to_string();
    super::span_out::read_graph_digest(&base, &rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_defaults_on() {
        // the default (unset) is ON; only an explicit off/0/false turns it off
        std::env::remove_var("OZONE_GRAPH_ON_LINK");
        assert!(enabled());
    }

    #[test]
    fn payloads_match_the_walk_shapes() {
        let a = file_reader::text_analyze_input(101, "fn main(){}", &["code".to_string()], "/x/a.rs");
        assert_eq!(a["action"]["type"], "Analyze");
        assert_eq!(a["action"]["code"], "fn main(){}");
        assert!(a["action"]["language"].is_null());
        assert_eq!(a["action"]["file_path"], "/x/a.rs");
        let m = file_reader::text_analyze_input(105, "x+1", &["math".to_string()], "/x/a.tex");
        assert_eq!(m["action"]["type"], "ParseExpression");
        let g = file_reader::create_graph_input(100, &json!({"analysis": {"k": 1}}), 7);
        assert_eq!(g["action"]["link_to_existing"], true);
        assert_eq!(g["action"]["project_id"], 7);
    }

    #[test]
    fn graph_id_is_never_minted() {
        assert_eq!(file_reader::graph_id_of(101, &json!({"graph_id": 42})), Some(42));
        assert_eq!(file_reader::graph_id_of(105, &json!({"result": {"graph_id": 9}})), Some(9));
        assert_eq!(file_reader::graph_id_of(101, &json!({})), None);
    }

    #[test]
    fn every_modality_has_an_entry_concept() {
        let v = entry_concepts();
        assert!(v.as_array().map(|a| a.len() >= 28).unwrap_or(false));
    }
}
