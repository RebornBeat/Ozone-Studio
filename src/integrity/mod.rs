//! Integrity monitoring and rollback system
//!
//! Based on Section 25 of the specification.
//!
//! Ensures no information is ever lost and provides rollback capability.

use crate::config::IntegrityConfig;
use crate::types::{ContainerID, OzoneError, OzoneResult, Blake3Hash};
use crate::types::integrity::{
    IntegrityCheck, IntegrityCheckType, IntegrityCheckResult,
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
        let mut repairs_made: Vec<String> = Vec::new();
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
        let mut affected_containers = vec![request.container_id];
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
        
        let data_loss_risk = if estimated_data_loss > 0 { 0.5 } else { 0.0 };
        let recommendation = if warnings.is_empty() {
            "Safe to proceed with rollback".to_string()
        } else {
            format!("Review warnings before proceeding: {} issues", warnings.len())
        };
        
        Ok(ImpactAnalysis {
            affected_containers: affected_containers.clone(),
            affected_relationships: Vec::new(),
            dependent_tasks: Vec::new(), // Would need task manager integration
            estimated_data_loss,
            data_loss_risk,
            warnings,
            recommendation,
        })
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
