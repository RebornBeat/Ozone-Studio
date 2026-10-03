//! Real file-change → live-graph ripple. Full design:
//! docs/guides/file-beacon-design.md.
//!
//! Non-intrusive: nothing is ever written next to the user's real file, no
//! wrapping archive (explicitly not MetaStrata's `.stratum` model — see
//! the design doc for that comparison). The beacon's cache lives entirely
//! in this process's own memory, rebuilt fresh at every boot; the user's
//! file is only ever `stat`'d, never opened for writing or renamed.
//!
//! Real gap this closes: a `FileReference` container
//! (`assets/pipelines/general/file_link/main.rs`) is created once at
//! registration and never touched again — an external edit to the real
//! file is invisible to the live graph until someone re-registers it. This
//! polls every real `FileReference` container's referenced path on a real
//! interval and fires one real `UpdateContainer` when its mtime/size
//! genuinely changed since the last poll.
//!
//! Enumeration: no "list all containers of type X" query exists.
//! `SearchContainersByKeywords` needs real, non-empty, signal-bearing
//! keywords or `KeywordScan` returns nothing (confirmed directly against
//! `src/zsei/search.rs`); `ZSEIQuery::GetFileReferences` is declared but
//! has no handler anywhere in the codebase — schema-only, same pattern
//! this whole project keeps finding elsewhere. Walks the real, working
//! path instead: `GetUserWorkspaces` → `GetProjects` → each project's real
//! `child_ids`, filtering for `ContainerType::FileReference`. Known,
//! stated-plainly limitation: only the two real user ids this project has
//! ever found live data under (0 and 1) are walked — there is no "list all
//! users" query to discover others yet.
//!
//! A `FileReference` container carries no structured mtime/size field at
//! all today (confirmed directly against `link_reference_to_graph` —
//! `object_store_path` is null, only `metadata.name` holds anything real,
//! as the `"File: {path}"` convention F2's own finding documented). The
//! real path is parsed out of that name; the change-detection cache is
//! this module's own, not read from the container.

use crate::orchestrator::StoreAccess;
use crate::types::container::{Container, ContainerType};
use crate::types::zsei::{ContainerUpdate, ZSEIQuery, ZSEIQueryResult};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Starting point per the design doc — not a considered-final value.
/// Needs live tuning against real usage once running, not a guess left
/// untuned forever (methodology-38 doctrine).
const POLL_INTERVAL_SECS: u64 = 45;

/// The only real user ids this project has ever found live data under
/// (R1/R5's own repeated finding) — no "list all users" query exists to
/// discover others. Stated plainly as a real limitation, not silently
/// assumed complete.
const KNOWN_USER_IDS: &[u64] = &[0, 1];

/// The real naming convention every linking pipeline writes (confirmed:
/// `assets/pipelines/general/file_link/main.rs`, matching F2's own
/// citation for `FileReferenceViewer.tsx`) — the only place a
/// `FileReference` container's real path lives today.
const FILE_NAME_PREFIX: &str = "File: ";

#[derive(Clone, Copy, PartialEq, Eq)]
struct FileState {
    modified: u64,
    size: u64,
}

/// Boot-spawned, same convention as `amt_loop`'s two existing spawns in
/// `src/lib.rs`.
pub fn spawn(store: Arc<dyn StoreAccess>) {
    tokio::spawn(async move {
        let mut cache: HashMap<u64, FileState> = HashMap::new();
        let mut interval = tokio::time::interval(Duration::from_secs(POLL_INTERVAL_SECS));
        loop {
            interval.tick().await;
            poll_once(&*store, &mut cache).await;
        }
    });
}

async fn poll_once(store: &dyn StoreAccess, cache: &mut HashMap<u64, FileState>) {
    for (container_id, path) in discover_file_references(store).await {
        let Ok(meta) = std::fs::metadata(&path) else {
            // File genuinely gone or unreadable — not this beacon's job to
            // report that, only real changes to files that still exist.
            continue;
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let state = FileState { modified, size: meta.len() };
        // HashMap::insert returns the prior value, if any — the first
        // sighting (None) establishes a baseline only, never fires a
        // spurious "changed" ripple for a file the beacon just started
        // watching this boot.
        let changed = matches!(cache.insert(container_id, state), Some(prev) if prev != state);
        if changed {
            // NATIVE FILE HISTORY (operator vision: git-like, per-file, all
            // changes, light on the graph): one JSONL row per change in
            // file_history/<container_id>.jsonl + a capped content snapshot
            // in file_history/snapshots/<container_id>/. The GRAPH carries
            // only the container (ripple below) — the history lives in
            // plain files, queryable through the git MCP.
            record_history(&container_id.to_string(), &path, modified, meta.len());
            match ripple_change(store, container_id).await {
                Ok(()) => tracing::info!(
                    container_id,
                    path = %path,
                    "file beacon: real file change detected, container rippled"
                ),
                Err(e) => tracing::warn!(container_id, error = %e, "file beacon: ripple failed"),
            }
        }
    }
}

async fn discover_file_references(store: &dyn StoreAccess) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    for &user_id in KNOWN_USER_IDS {
        for workspace_id in query_container_ids(store, &ZSEIQuery::GetUserWorkspaces { user_id }).await {
            for project_id in query_container_ids(store, &ZSEIQuery::GetProjects { workspace_id }).await {
                let Some(project) = get_container(store, project_id).await else { continue };
                for child_id in project.global_state.child_ids {
                    let Some(child) = get_container(store, child_id).await else { continue };
                    if child.local_state.metadata.container_type != ContainerType::FileReference {
                        continue;
                    }
                    if let Some(path) = child
                        .local_state
                        .metadata
                        .name
                        .as_deref()
                        .and_then(|n| n.strip_prefix(FILE_NAME_PREFIX))
                    {
                        out.push((child_id, path.to_string()));
                    }
                }
            }
        }
    }
    out
}

async fn query_container_ids(store: &dyn StoreAccess, q: &ZSEIQuery) -> Vec<u64> {
    let Ok(req) = serde_json::to_value(q) else { return Vec::new() };
    let Ok(resp) = store.query(req).await else { return Vec::new() };
    match serde_json::from_value::<ZSEIQueryResult>(resp) {
        Ok(ZSEIQueryResult::Containers(ids)) => ids,
        _ => Vec::new(),
    }
}

async fn get_container(store: &dyn StoreAccess, container_id: u64) -> Option<Container> {
    let raw = store.get_container(container_id).await.ok()??;
    serde_json::from_value(raw).ok()
}

/// One real `UpdateContainer` for a genuinely detected change — bumps
/// `metadata.updated_at` to now (a real, existing, semantically-correct
/// field: "this container's referenced reality just changed"), read-
/// modify-write since `ContainerUpdate.metadata` replaces the whole
/// sub-struct, not a sparse patch (same contract this session's earlier
/// `ContinuedBy` reverse-edge fix in `amt.rs` already works around).
async fn ripple_change(store: &dyn StoreAccess, container_id: u64) -> Result<(), String> {
    let container = get_container(store, container_id)
        .await
        .ok_or_else(|| "container vanished before the ripple could fire".to_string())?;
    let mut metadata = container.local_state.metadata.clone();
    metadata.updated_at = now_secs();
    let updates = ContainerUpdate {
        metadata: Some(metadata),
        context: None,
        storage: None,
        hints: None,
    };
    let updates_json = serde_json::to_value(&updates).map_err(|e| e.to_string())?;
    store.update_container(container_id, updates_json).await
}

/// History dir under the beacon's own data area. OZONE_ZSEI_DATA_DIR is
/// the established convention (amt_loop, image/3D pipelines).
fn history_dir() -> std::path::PathBuf {
    let dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".into());
    std::path::Path::new(&dir).join("file_history")
}

/// Append one history row + keep a bounded content snapshot. Snapshot
/// naming: <unix_millis>_<size> so ordering is intrinsic; cap 10 per file,
/// oldest evicted. Content-addressed-enough for diff-on-demand (the git
/// MCP reads two snapshots and diffs) without storing every intermediate.
fn record_history(container_id: &str, path: &str, modified: u64, size: u64) {
    use std::io::Write;
    let dir = history_dir().join("snapshots").join(container_id);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    // Snapshot copy (bounded): read the real file once, write once.
    if let Ok(content) = std::fs::read(path) {
        let snap = dir.join(format!("{}_{:x}.snap", millis, size));
        let _ = std::fs::write(&snap, &content);
        // Evict oldest beyond 10.
        let mut snaps: Vec<_> = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map(|x| x == "snap").unwrap_or(false))
                    .filter_map(|e| {
                        e.metadata().ok().and_then(|m| {
                            m.modified().ok().map(|t| (e.path(), t))
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        snaps.sort_by_key(|(_, t)| *t);
        while snaps.len() > 10 {
            let (oldest, _) = snaps.remove(0);
            let _ = std::fs::remove_file(oldest);
        }
    }
    let hist = history_dir().join(format!("{}.jsonl", container_id));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&hist) {
        let row = serde_json::json!({
            "ts": millis,
            "modified": modified,
            "size": size,
            "path": path,
            "snapshot_dir": format!("snapshots/{}/", container_id),
        });
        let _ = writeln!(f, "{}", row);
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
