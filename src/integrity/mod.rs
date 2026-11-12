//! Integrity monitoring and rollback system
//!
//! Based on Section 25 of the specification.
//!
//! Ensures no information is ever lost and provides rollback capability.

use crate::config::IntegrityConfig;
use crate::types::{ContainerID, OzoneError, OzoneResult};
use crate::types::integrity::{
    IntegrityCheckType, IntegrityCheckResult,
    RollbackRequest, ImpactAnalysis,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Integrity monitor. Snapshots live in the native history store (`src/history`): content-chunked, de-duplicated, durably indexed, shared
/// with the file beacon and git-mcp (operator 2026-11-12). Version numbers are the store's per-container `v`.
pub struct IntegrityMonitor {
    /// Configuration
    config: IntegrityConfig,

    /// Rollback storage root (the history store root)
    rollback_path: PathBuf,

    /// The shared history store (scope `container`)
    history: Arc<crate::history::History>,

    /// Last check results
    last_check: Arc<RwLock<Option<IntegrityCheckResult>>>,

    /// Running flag
    running: Arc<RwLock<bool>>,
}

const SCOPE: &str = "container";

impl IntegrityMonitor {
    /// The shared history store this monitor writes to.
    pub fn history_handle(&self) -> Arc<crate::history::History> {
        self.history.clone()
    }

    /// Create new integrity monitor
    pub fn new(config: &IntegrityConfig) -> OzoneResult<Self> {
        let rollback_path = PathBuf::from(&config.rollback_path);

        // Ensure rollback directory exists
        std::fs::create_dir_all(&rollback_path)
            .map_err(|e| OzoneError::IntegrityError(format!("Failed to create rollback dir: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            history: crate::history::open_shared(rollback_path.clone()),
            rollback_path,
            last_check: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start integrity monitoring
    pub async fn start_monitoring(&self) -> OzoneResult<()> {
        if !self.config.enabled {
            tracing::info!("Integrity monitoring disabled");
            return Ok(());
        }
        
        {
            let mut running = self.running.write().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }
        
        tracing::info!("Starting integrity monitoring");
        
        let check_interval = std::time::Duration::from_secs(self.config.check_interval_secs);
        
        loop {
            // Check if we should stop
            if !*self.running.read().await {
                break;
            }
            
            // Run integrity check
            if let Err(e) = self.run_check().await {
                tracing::error!("Integrity check failed: {}", e);
            }
            
            // Wait for next check
            tokio::time::sleep(check_interval).await;
        }
        
        Ok(())
    }
    
    /// Stop monitoring
    pub async fn stop_monitoring(&self) {
        *self.running.write().await = false;
    }
    
    /// Run an integrity check
    pub async fn run_check(&self) -> OzoneResult<IntegrityCheckResult> {
        tracing::debug!("Running integrity check");
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut issues_found: Vec<String> = Vec::new();
        let repairs_made: Vec<String> = Vec::new();
        let mut containers_checked = 0u32;
        
        // Verify the newest version of every container that has history: every chunk and the whole content are blake3-checked.
        let history = self.history.clone();
        let (containers_checked_n, found) = tokio::task::spawn_blocking(move || {
            let mut checked = 0u32;
            let mut issues: Vec<String> = Vec::new();
            for (scope, id, _n) in history.scopes() {
                if scope != SCOPE {
                    continue;
                }
                checked += 1;
                match history.verify_latest(&scope, &id) {
                    Ok(_) => {}
                    Err(e) => issues.push(format!("Container {} latest snapshot failed verification: {}", id, e)),
                }
            }
            (checked, issues)
        })
        .await
        .map_err(|e| OzoneError::IntegrityError(format!("integrity check task failed: {}", e)))?;
        containers_checked = containers_checked_n;
        issues_found.extend(found);

        let result = IntegrityCheckResult {
            check_type: IntegrityCheckType::Full,
            passed: issues_found.is_empty(),
            score: if issues_found.is_empty() { 1.0 } else { 1.0 - (issues_found.len() as f32 / containers_checked as f32).min(1.0) },
            timestamp: now,
            containers_checked,
            issues_found: issues_found.len() as u32,
            repairs_made: repairs_made.len() as u32,
            issues: Vec::new(),
        };
        
        *self.last_check.write().await = Some(result.clone());
        
        Ok(result)
    }
    
    /// Create a snapshot for rollback: record `data` as the container's newest history version (identical to the latest = no new row).
    pub async fn create_snapshot(&self, container_id: ContainerID, data: &[u8]) -> OzoneResult<()> {
        let history = self.history.clone();
        let bytes = data.to_vec();
        let id = container_id.to_string();
        tokio::task::spawn_blocking(move || history.record(SCOPE, &id, &bytes, "pre-write", None))
            .await
            .map_err(|e| OzoneError::IntegrityError(format!("snapshot task failed: {}", e)))?
            .map(|_| ())
            .map_err(|e| OzoneError::IntegrityError(format!("Failed to record snapshot: {}", e)))
    }

    /// Rollback a container to a previous version: returns that version's bytes (chunk- and whole-hash verified). The caller restores them
    /// (`ZSEI::restore_container_version` / `POST /history/restore`).
    pub async fn rollback(&self, request: RollbackRequest) -> OzoneResult<Vec<u8>> {
        let history = self.history.clone();
        let id = request.container_id.to_string();
        let target = request.target_version;
        let container_id = request.container_id;
        tokio::task::spawn_blocking(move || -> OzoneResult<Vec<u8>> {
            let rows = history.list(SCOPE, &id).map_err(|e| OzoneError::IntegrityError(e.to_string()))?;
            if rows.is_empty() {
                return Err(OzoneError::NotFound(format!("No versions for container {}", container_id)));
            }
            let row = match target {
                Some(v) => rows.iter().find(|r| r.v == v as u64),
                // previous version = the one before the newest
                None => rows.iter().rev().nth(1),
            }
            .ok_or_else(|| OzoneError::NotFound("Target version not found".into()))?;
            tracing::info!("Rolling back container {} to version {}", container_id, row.v);
            history.rebuild(row).map_err(|e| OzoneError::IntegrityError(format!("Failed to read snapshot: {}", e)))
        })
        .await
        .map_err(|e| OzoneError::IntegrityError(format!("rollback task failed: {}", e)))?
    }

    /// Analyze impact of a rollback
    pub async fn analyze_impact(&self, request: &RollbackRequest) -> OzoneResult<ImpactAnalysis> {
        let rows = self.history.list(SCOPE, &request.container_id.to_string()).unwrap_or_default();
        let affected_containers = vec![request.container_id];
        let mut warnings = Vec::new();
        let mut estimated_data_loss = 0u64;
        
        // Find the target version
        if !rows.is_empty() {
            let current_version = rows.last().map(|v| v.v as u32).unwrap_or(0);
            let target_version = request.target_version.unwrap_or(current_version.saturating_sub(1));
            
            // Count versions that would be lost
            let versions_lost = current_version.saturating_sub(target_version);
            estimated_data_loss = versions_lost as u64;
            
            if versions_lost > 1 {
                warnings.push(format!(
                    "Rolling back {} versions (from {} to {})",
                    versions_lost, current_version, target_version
                ));
            }
            
            // Check if target version exists
            if !rows.iter().any(|v| v.v == target_version as u64) {
                warnings.push(format!("Target version {} not found in history", target_version));
            }
        } else {
            warnings.push("No version history available for this container".to_string());
        }
        
        // Check for dependent containers (simplified - in production would query ZSEI)
        // Containers that have this container as parent would be affected
        
        // Find dependent tasks by scanning task store
        let dependent_tasks = self.find_dependent_tasks(request.container_id).await;
        if !dependent_tasks.is_empty() {
            warnings.push(format!("{} active tasks may be affected", dependent_tasks.len()));
        }
        
        // DERIVED, never invented (doctrine audit H3, 2026-11-04: was a literal 0.5 whenever any loss was estimated): the share of the affected
        // containers that carry data loss — a measured ratio, 0.0 when nothing is affected.
        let data_loss_risk = if affected_containers.is_empty() {
            0.0
        } else {
            (estimated_data_loss as f32 / affected_containers.len() as f32).min(1.0)
        };
        let recommendation = if warnings.is_empty() {
            "Safe to proceed with rollback".to_string()
        } else {
            format!("Review warnings before proceeding: {} issues", warnings.len())
        };
        
        Ok(ImpactAnalysis {
            affected_containers: affected_containers.clone(),
            affected_relationships: Vec::new(),
            dependent_tasks,
            estimated_data_loss,
            data_loss_risk,
            warnings,
            recommendation,
        })
    }
    
    /// Find tasks that depend on a container
    async fn find_dependent_tasks(&self, container_id: ContainerID) -> Vec<u64> {
        let tasks_dir = std::env::var("OZONE_TASKS_PATH")
            .unwrap_or_else(|_| "./ozone_tasks".to_string());
        
        let mut dependent_tasks = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(&tasks_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(task) = serde_json::from_str::<serde_json::Value>(&content) {
                            // Check if task is active and references this container
                            let status = task.get("status").and_then(|s| s.as_str()).unwrap_or("");
                            let is_active = status == "queued" || status == "running";
                            
                            if is_active {
                                // Check if container_id is in task inputs
                                if let Some(inputs) = task.get("inputs") {
                                    let inputs_str = serde_json::to_string(inputs).unwrap_or_default();
                                    if inputs_str.contains(&container_id.to_string()) {
                                        // Extract task_id from filename
                                        if let Some(task_id) = path.file_stem()
                                            .and_then(|s| s.to_str())
                                            .and_then(|s| s.strip_prefix("task_"))
                                            .and_then(|s| s.parse::<u64>().ok()) {
                                            dependent_tasks.push(task_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        dependent_tasks
    }
    /// Get version history for a container: `(version, unix seconds)`.
    pub async fn get_version_history(&self, container_id: ContainerID) -> Vec<(u32, u64)> {
        self.history
            .list(SCOPE, &container_id.to_string())
            .unwrap_or_default()
            .into_iter()
            .map(|r| (r.v as u32, r.ts / 1000))
            .collect()
    }

    /// Get last check result
    pub async fn get_last_check(&self) -> Option<IntegrityCheckResult> {
        self.last_check.read().await.clone()
    }
    
    /// Verify a specific container's integrity: does `data` equal the newest recorded version?
    pub async fn verify_container(&self, container_id: ContainerID, data: &[u8]) -> OzoneResult<bool> {
        match self.history.latest(SCOPE, &container_id.to_string()) {
            Ok(Some(latest)) => Ok(blake3::hash(data).to_hex().to_string() == latest.hash),
            // No version history - assume valid
            Ok(None) => Ok(true),
            Err(e) => Err(OzoneError::IntegrityError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Local container fixture (same shape as graph_events' tiny_container —
    /// kept local because that one is cfg(test)-private to its module).
    fn tiny_fixture(keywords: Vec<String>) -> crate::types::container::Container {
        use crate::types::container::*;
        Container {
            global_state: GlobalState {
                container_id: 0,
                parent_id: 0,
                child_ids: vec![],
                child_count: 0,
                version: 1,
            },
            local_state: LocalState {
                metadata: Metadata {
                    container_type: ContainerType::CoordinationEvent,
                    modality: Modality::Unknown,
                    created_at: 0,
                    updated_at: 0,
                    provenance: "test".into(),
                    permissions: 0,
                    owner_id: 0,
                    name: None,
                    materialized_path: None,
                },
                context: Context {
                    categories: vec![],
                    methodologies: vec![],
                    keywords,
                    topics: vec![],
                    relationships: vec![],
                    learned_associations: vec![],
                    keyword_meta: Default::default(),
                    scope: Default::default(),
                    embedding: None,
                },
                storage: StoragePointers {
                    db_shard_id: None,
                    vector_index_ref: None,
                    object_store_path: None,
                    compression_type: CompressionType::None,
                },
                hints: Default::default(),
                integrity: IntegrityData::default(),
                file_context: None,
                code_context: None,
                text_context: None,
                external_ref: None,
            },
        }
    }

    fn test_config(tag: &str) -> IntegrityConfig {
        let dir = std::env::temp_dir().join(format!(
            "ozone_integrity_{}_{}",
            std::process::id(),
            tag
        ));
        std::fs::create_dir_all(&dir).unwrap();
        IntegrityConfig {
            enabled: true,
            check_interval_secs: 3600,
            rollback_path: dir.to_string_lossy().into(),
            max_versions: 0,
        }
    }

    /// T-I4 (GRAPH_TEST_PLAN §10) — blake3 tampering detection on a real
    /// snapshot: a clean snapshot passes the periodic check; flipping one
    /// byte in the snapshot file must surface as a real integrity issue.
    #[tokio::test]
    async fn snapshot_tampering_detected_by_run_check() {
        let config = test_config("tamper");
        let monitor = IntegrityMonitor::new(&config).unwrap();

        monitor
            .create_snapshot(4242, b"original container content")
            .await
            .unwrap();

        // Clean state: the check must pass with the container counted.
        let clean = monitor.run_check().await.unwrap();
        assert!(clean.passed, "untampered snapshot must pass: {:?}", clean.issues);
        assert_eq!(clean.containers_checked, 1);

        // Tamper: rewrite the snapshot file with different content — the
        // recorded blake3 hash no longer matches.
        let latest = monitor.history.latest(SCOPE, "4242").unwrap().unwrap();
        let object = monitor.history.root().join("objects").join(&latest.chunks[0][..2]).join(&latest.chunks[0]);
        std::fs::write(&object, b"tampered container content").unwrap();

        let result = monitor.run_check().await.unwrap();
        assert!(!result.passed, "tampered snapshot must fail the check");
        assert!(result.issues_found >= 1, "tampering must be reported");
    }

    /// T-I4 call-site proof — the ZSEI write choke point now feeds the
    /// integrity monitor: an UpdateContainer must leave a PRE-WRITE
    /// snapshot of the container's OLD content in the rollback layer.
    /// (Until 2026-09-19 create_snapshot had no caller at all.)
    #[tokio::test]
    async fn zsei_update_snapshots_pre_write_content() {
        use crate::types::zsei::ZSEIQuery;

        // ZSEI with its own temp store + monitor with its own rollback dir.
        let zdir = std::env::temp_dir().join(format!("ozone_ti4_zsei_{}", std::process::id()));
        std::fs::create_dir_all(&zdir).unwrap();
        let zsei_config = crate::config::ZSEIConfig {
            global_path: zdir.join("global.mmap").to_string_lossy().into(),
            local_path: zdir.join("local").to_string_lossy().into(),
            cache_path: zdir.join("cache").to_string_lossy().into(),
            ml_path: zdir.join("ml").to_string_lossy().into(),
            max_containers_in_memory: 100,
            mmap_enabled: false,
            embedding_dimension: 64,
            pipeline_index_path: zdir.join("pi.json").to_string_lossy().into(),
            methodology_index_path: zdir.join("mi.json").to_string_lossy().into(),
            blueprint_index_path: zdir.join("bi.json").to_string_lossy().into(),
        };
        let mut zsei = crate::zsei::ZSEI::new(&zsei_config).unwrap();

        let rollback_dir = std::env::temp_dir().join(format!("ozone_ti4_rb_{}", std::process::id()));
        let monitor = IntegrityMonitor::new(&IntegrityConfig {
            enabled: true,
            check_interval_secs: 3600,
            rollback_path: rollback_dir.to_string_lossy().into(),
            max_versions: 0,
        })
        .unwrap();
        zsei.set_integrity(Arc::new(tokio::sync::RwLock::new(monitor)));

        // Create a named container, then update its name.
        let mut container = tiny_fixture(vec!["ti4".to_string()]);
        container.local_state.metadata.name = Some("pre-update name".to_string());
        let new_id = match zsei
            .query(ZSEIQuery::CreateContainer { parent_id: 0, container })
            .await
            .unwrap()
        {
            crate::types::zsei::ZSEIQueryResult::ContainerID(id) => id,
            other => panic!("expected ContainerID, got {:?}", other),
        };

        let mut updates = crate::types::zsei::ContainerUpdate::default();
        updates.metadata = Some(crate::types::container::Metadata {
            container_type: crate::types::container::ContainerType::CoordinationEvent,
            modality: crate::types::container::Modality::Unknown,
            created_at: 0,
            updated_at: 1,
            provenance: "test-update".into(),
            permissions: 0,
            owner_id: 0,
            name: Some("post-update name".to_string()),
            materialized_path: None,
        });
        zsei.query(ZSEIQuery::UpdateContainer { container_id: new_id, updates })
            .await
            .unwrap();

        // The rollback layer must hold the PRE-WRITE content.
        let h = crate::history::open_shared(rollback_dir.clone());
        let rows = h.list("container", &new_id.to_string()).unwrap();
        assert!(!rows.is_empty(), "UpdateContainer must leave a pre-write snapshot");
        let raw = String::from_utf8(h.read_version("container", &new_id.to_string(), rows[0].v).unwrap()).unwrap();
        assert!(raw.contains("pre-update name"), "snapshot must capture the OLD content, not the new");
        assert!(!raw.contains("post-update name"));

        let _ = std::fs::remove_dir_all(&zdir);
        let _ = std::fs::remove_dir_all(&rollback_dir);
    }
}
