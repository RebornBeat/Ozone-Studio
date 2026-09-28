//! Integrity monitoring and rollback system
//!
//! Based on Section 25 of the specification.
//!
//! Ensures no information is ever lost and provides rollback capability.

use crate::config::IntegrityConfig;
use crate::types::{ContainerID, OzoneError, OzoneResult, Blake3Hash};
use crate::types::integrity::{
    IntegrityCheckType, IntegrityCheckResult,
    RollbackRequest, ImpactAnalysis,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Integrity monitor
pub struct IntegrityMonitor {
    /// Configuration
    config: IntegrityConfig,
    
    /// Rollback storage path
    rollback_path: PathBuf,
    
    /// Container version history
    versions: Arc<RwLock<HashMap<ContainerID, Vec<ContainerVersion>>>>,
    
    /// Last check results
    last_check: Arc<RwLock<Option<IntegrityCheckResult>>>,
    
    /// Running flag
    running: Arc<RwLock<bool>>,
}

/// Container version for rollback
#[derive(Debug, Clone)]
struct ContainerVersion {
    version: u32,
    timestamp: u64,
    hash: Blake3Hash,
    snapshot_path: PathBuf,
}

impl IntegrityMonitor {
    /// Create new integrity monitor
    pub fn new(config: &IntegrityConfig) -> OzoneResult<Self> {
        let rollback_path = PathBuf::from(&config.rollback_path);
        
        // Ensure rollback directory exists
        std::fs::create_dir_all(&rollback_path)
            .map_err(|e| OzoneError::IntegrityError(format!("Failed to create rollback dir: {}", e)))?;
        
        Ok(Self {
            config: config.clone(),
            rollback_path,
            versions: Arc::new(RwLock::new(HashMap::new())),
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
        
        // Check all versioned containers
        let versions = self.versions.read().await;
        for (container_id, container_versions) in versions.iter() {
            containers_checked += 1;
            
            // Verify latest snapshot exists and is valid
            if let Some(latest) = container_versions.last() {
                if !latest.snapshot_path.exists() {
                    issues_found.push(format!(
                        "Container {} version {} snapshot missing at {:?}",
                        container_id, latest.version, latest.snapshot_path
                    ));
                } else {
                    // Verify hash
                    if let Ok(data) = std::fs::read(&latest.snapshot_path) {
                        let hash = blake3::hash(&data);
                        if hash.as_bytes() != &latest.hash {
                            issues_found.push(format!(
                                "Container {} version {} hash mismatch - possible corruption",
                                container_id, latest.version
                            ));
                        }
                    }
                }
            }
        }
        
        // Check rollback directory health
        if let Ok(entries) = std::fs::read_dir(&self.rollback_path) {
            let snapshot_count = entries.filter(|e| e.is_ok()).count();
            tracing::debug!("Found {} snapshots in rollback directory", snapshot_count);
        }
        
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
    
    /// Create a snapshot for rollback
    pub async fn create_snapshot(&self, container_id: ContainerID, data: &[u8]) -> OzoneResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Calculate hash
        let hash = blake3::hash(data);
        let hash_bytes: Blake3Hash = *hash.as_bytes();
        
        // Get current version number
        let mut versions = self.versions.write().await;
        let container_versions = versions.entry(container_id).or_insert_with(Vec::new);
        let version = container_versions.last()
            .map(|v| v.version + 1)
            .unwrap_or(1);
        
        // Save snapshot
        let snapshot_filename = format!("{}_{}.snapshot", container_id, version);
        let snapshot_path = self.rollback_path.join(&snapshot_filename);
        
        std::fs::write(&snapshot_path, data)
            .map_err(|e| OzoneError::IntegrityError(format!("Failed to write snapshot: {}", e)))?;
        
        // Record version
        let container_version = ContainerVersion {
            version,
            timestamp: now,
            hash: hash_bytes,
            snapshot_path,
        };
        
        container_versions.push(container_version);
        
        // Trim old versions if needed
        while container_versions.len() > self.config.max_versions as usize {
            let old = container_versions.remove(0);
            let _ = std::fs::remove_file(&old.snapshot_path);
        }
        
        Ok(())
    }
    
    /// Rollback a container to a previous version
    pub async fn rollback(&self, request: RollbackRequest) -> OzoneResult<Vec<u8>> {
        let versions = self.versions.read().await;
        let container_versions = versions.get(&request.container_id)
            .ok_or_else(|| OzoneError::NotFound(
                format!("No versions for container {}", request.container_id)
            ))?;
        
        // Find the requested version
        let target_version = if let Some(version) = request.target_version {
            container_versions.iter().find(|v| v.version == version)
        } else {
            // Rollback to previous version
            container_versions.iter().rev().nth(1)
        };
        
        let target = target_version
            .ok_or_else(|| OzoneError::NotFound("Target version not found".into()))?;
        
        // Read snapshot
        let data = std::fs::read(&target.snapshot_path)
            .map_err(|e| OzoneError::IntegrityError(format!("Failed to read snapshot: {}", e)))?;
        
        // Verify hash
        let hash = blake3::hash(&data);
        if hash.as_bytes() != &target.hash {
            return Err(OzoneError::IntegrityError("Snapshot hash mismatch".into()));
        }
        
        tracing::info!(
            "Rolling back container {} to version {}",
            request.container_id,
            target.version
        );
        
        Ok(data)
    }
    
    /// Analyze impact of a rollback
    pub async fn analyze_impact(&self, request: &RollbackRequest) -> OzoneResult<ImpactAnalysis> {
        let versions = self.versions.read().await;
        let affected_containers = vec![request.container_id];
        let mut warnings = Vec::new();
        let mut estimated_data_loss = 0u64;
        
        // Find the target version
        if let Some(container_versions) = versions.get(&request.container_id) {
            let current_version = container_versions.last().map(|v| v.version).unwrap_or(0);
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
            if !container_versions.iter().any(|v| v.version == target_version) {
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
        
        let data_loss_risk = if estimated_data_loss > 0 { 0.5 } else { 0.0 };
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
    /// Get version history for a container
    pub async fn get_version_history(&self, container_id: ContainerID) -> Vec<(u32, u64)> {
        self.versions.read().await
            .get(&container_id)
            .map(|versions| {
                versions.iter()
                    .map(|v| (v.version, v.timestamp))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get last check result
    pub async fn get_last_check(&self) -> Option<IntegrityCheckResult> {
        self.last_check.read().await.clone()
    }
    
    /// Verify a specific container's integrity
    pub async fn verify_container(&self, container_id: ContainerID, data: &[u8]) -> OzoneResult<bool> {
        let versions = self.versions.read().await;
        
        if let Some(container_versions) = versions.get(&container_id) {
            if let Some(latest) = container_versions.last() {
                let hash = blake3::hash(data);
                return Ok(hash.as_bytes() == &latest.hash);
            }
        }
        
        // No version history - assume valid
        Ok(true)
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
            max_versions: 10,
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
        let snapshot_path = {
            let versions = monitor.versions.read().await;
            versions.get(&4242).unwrap().last().unwrap().snapshot_path.clone()
        };
        std::fs::write(&snapshot_path, b"tampered container content").unwrap();

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
            max_versions: 10,
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
        let snapshots: Vec<_> = std::fs::read_dir(&rollback_dir)
            .expect("rollback dir exists")
            .flatten()
            .collect();
        assert!(
            !snapshots.is_empty(),
            "UpdateContainer must leave a pre-write snapshot"
        );
        let raw = std::fs::read_to_string(snapshots[0].path()).unwrap();
        assert!(
            raw.contains("pre-update name"),
            "snapshot must capture the OLD content, not the new"
        );
        assert!(!raw.contains("post-update name"));

        let _ = std::fs::remove_dir_all(&zdir);
        let _ = std::fs::remove_dir_all(&rollback_dir);
    }
}
