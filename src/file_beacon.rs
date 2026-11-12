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
        let prev = cache.insert(container_id, state);
        let changed = matches!(prev, Some(p) if p != state);
        if prev.is_none() {
            // BASELINE (history review 2026-11-12): the first sighting of a file this boot records its CURRENT content as a version when it
            // differs from the newest recorded one, so (a) the very first change always has a "before" and (b) a change made while the host was
            // down is captured. An unchanged file is a no-op (history dedupes by hash). A failed write forgets the cache entry so the next poll
            // retries — nothing is skipped. No ripple on a baseline.
            if !record_history(&container_id.to_string(), &path, "baseline") {
                cache.remove(&container_id);
            }
        }
        if changed {
            // NATIVE HISTORY (src/history): one version row per DISTINCT content in history/versions/file/<container_id>.jsonl, content chunked
            // and de-duplicated in history/objects (no cap, no eviction). git-mcp reads/diffs/restores it. The GRAPH carries only the container
            // (ripple below). A failed write forgets the baseline so the next poll re-captures the file's then-current content.
            if !record_history(&container_id.to_string(), &path, "changed") {
                cache.remove(&container_id);
                continue;
            }
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

pub(crate) async fn discover_file_references(store: &dyn StoreAccess) -> Vec<(u64, String)> {
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

/// Record the file's content AT THIS MOMENT as a history version (src/history: chunked, de-duplicated, hash-verified on read). Returns false
/// when the write failed (caller retries on its next poll). A file that vanished between the stat and the read is not an error — the beacon
/// only reports files that still exist.
fn record_history(container_id: &str, path: &str, reason: &str) -> bool {
    let content = match std::fs::read(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return true,
        Err(e) => {
            tracing::error!(container_id, path, error = %e, "file beacon: cannot read file for history — will retry");
            return false;
        }
    };
    match crate::history::shared().record("file", container_id, &content, reason, Some(path)) {
        Ok(_) => true,
        Err(e) => {
            tracing::error!(container_id, path, error = %e, "file beacon: history write failed — will retry");
            false
        }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
