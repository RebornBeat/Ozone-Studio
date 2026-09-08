//! K-registry `store` family — task persistence backend contract.
//!
//! The TaskManager owns task STATE (the queues, locks, and logic); this trait
//! owns only PERSISTENCE: loading a snapshot at boot, saving one on change.
//! The default backend is the JSON file store (tasks.json in the configured
//! directory). Any other backend — SQLite, remote store, ZSEI containers —
//! implements the same two methods and is selected by config. Nothing in the
//! TaskManager changes when the backend does.

use crate::types::{OzoneResult, TaskID};
use std::path::{Path, PathBuf};

/// A full persistence snapshot: everything needed to restore task state.
/// Crate-internal — the snapshot types live in task/mod.rs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct TaskSnapshot {
    pub(crate) tasks: HashMap<TaskID, super::StoredTask>,
    pub(crate) logs: HashMap<TaskID, Vec<super::LogEntry>>,
    pub(crate) next_id: TaskID,
}

use std::collections::HashMap;

/// The task persistence contract ("store of the store" — the TaskManager is
/// swappable over this). Crate-internal.
pub(crate) trait TaskStoreBackend: Send + Sync {
    /// Load the persisted snapshot. `Ok(None)` = nothing persisted yet
    /// (fresh start) — never an error.
    fn load(&self) -> OzoneResult<Option<TaskSnapshot>>;

    /// Persist the full snapshot (idempotent full-write).
    fn save(&self, snapshot: &TaskSnapshot) -> OzoneResult<()>;
}

/// DEFAULT backend — pretty JSON file (`tasks.json`) in the configured dir.
/// Behavior-identical to the original inline save/load.
pub struct JsonFileStore {
    dir: PathBuf,
}

impl JsonFileStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }
}

impl TaskStoreBackend for JsonFileStore {
    fn load(&self) -> OzoneResult<Option<TaskSnapshot>> {
        let path = Path::new(&self.dir).join("tasks.json");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| crate::types::OzoneError::StorageError(format!("Failed to read tasks: {e}")))?;
        if content.trim().is_empty() {
            return Ok(None);
        }
        let snapshot: TaskSnapshot = serde_json::from_str(&content)
            .map_err(|e| crate::types::OzoneError::StorageError(format!("Failed to parse tasks: {e}")))?;
        Ok(Some(snapshot))
    }

    fn save(&self, snapshot: &TaskSnapshot) -> OzoneResult<()> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| crate::types::OzoneError::StorageError(format!("Failed to create task dir: {e}")))?;
        let content = serde_json::to_string_pretty(snapshot)
            .map_err(|e| crate::types::OzoneError::StorageError(format!("Failed to serialize tasks: {e}")))?;
        std::fs::write(Path::new(&self.dir).join("tasks.json"), content)
            .map_err(|e| crate::types::OzoneError::StorageError(format!("Failed to write tasks: {e}")))?;
        Ok(())
    }
}

/// Select a backend by config name. Unknown names fall back to the JSON file
/// store with a warning — selection failures never block the task queue.
pub(crate) fn select_backend(name: &str, storage_path: &str) -> std::sync::Arc<dyn TaskStoreBackend> {
    match name {
        "json_file" | "" => std::sync::Arc::new(JsonFileStore::new(storage_path)),
        other => {
            tracing::warn!(
                "Unknown task store backend '{other}' — falling back to json_file"
            );
            std::sync::Arc::new(JsonFileStore::new(storage_path))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_file_round_trip() {
        let dir = std::env::temp_dir().join(format!("oz-taskstore-{}", std::process::id()));
        let store = JsonFileStore::new(&dir);

        // Fresh start: no snapshot.
        assert!(store.load().unwrap().is_none());

        let snapshot_json = r#"{"tasks":{},"logs":{},"next_id":7}"#;
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tasks.json"), snapshot_json).unwrap();

        let snap = store.load().unwrap().expect("snapshot after write");
        assert_eq!(snap.next_id, 7);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_backend_falls_back() {
        // select_backend never panics on unknown names.
        let _ = select_backend("warp-drive", "/tmp/oz-unused");
        let _ = select_backend("", "/tmp/oz-unused");
    }
}
