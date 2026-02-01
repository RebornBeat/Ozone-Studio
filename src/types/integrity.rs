//! Integrity types - Section 25 of the specification

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use super::{ContainerID, Severity};

/// Storage integrity system (§25.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageIntegritySystem {
    pub integrity_checks: Vec<IntegrityCheck>,
    pub alerts: Vec<IntegrityAlert>,
    pub verification_schedule: VerificationSchedule,
    pub rollback_system: RollbackSystem,
}

/// Integrity check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheck {
    pub check_id: u64,
    pub check_type: IntegrityCheckType,
    pub target: ContainerID,
    pub timestamp: u64,
    pub result: IntegrityCheckResult,
}

/// Integrity check types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrityCheckType {
    Full,
    ContentHash,
    SemanticFingerprint,
    ChunkBoundary,
    CrossChunkRelationship,
    CodeDocSync,
    ReconstructionTest,
}

/// Integrity check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheckResult {
    pub check_type: IntegrityCheckType,
    pub passed: bool,
    pub score: f32,
    pub timestamp: u64,
    pub containers_checked: u32,
    pub issues_found: u32,
    pub repairs_made: u32,
    pub issues: Vec<IntegrityIssue>,
}

/// Integrity issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityIssue {
    pub issue_type: IntegrityIssueType,
    pub description: String,
    pub severity: Severity,
    pub auto_repairable: bool,
}

/// Integrity issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrityIssueType {
    ChunkBoundaryBreak,
    OrphanedReference,
    SemanticDrift,
    VersionMismatch,
    ReconstructionFailure,
    RelationshipBroken,
    HashMismatch,
}

/// Integrity alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityAlert {
    pub alert_id: u64,
    pub alert_type: IntegrityIssueType,
    pub severity: Severity,
    pub affected_containers: Vec<ContainerID>,
    pub timestamp: u64,
    pub resolved: bool,
    pub resolution: Option<String>,
}

/// Verification schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSchedule {
    pub full_scan_interval_secs: u64,
    pub chunk_verification_interval_secs: u64,
    pub reference_validation_interval_secs: u64,
    pub external_ref_check_interval_secs: u64,
}

impl Default for VerificationSchedule {
    fn default() -> Self {
        Self {
            full_scan_interval_secs: 86400,          // 1 day
            chunk_verification_interval_secs: 3600,   // 1 hour
            reference_validation_interval_secs: 7200, // 2 hours
            external_ref_check_interval_secs: 21600,  // 6 hours
        }
    }
}

/// Rollback system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackSystem {
    pub max_versions_retained: u32,
    pub auto_checkpoint_interval_secs: u64,
    pub rollback_data_path: String,
}

impl Default for RollbackSystem {
    fn default() -> Self {
        Self {
            max_versions_retained: 100,
            auto_checkpoint_interval_secs: 3600,
            rollback_data_path: "zsei_data/integrity/rollback".into(),
        }
    }
}

/// Rollback request (§25.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRequest {
    pub container_id: ContainerID,
    pub target_version: Option<u32>,
    pub target: RollbackTarget,
    pub to_version: u64,
    pub reason: String,
    pub impact_analysis: Option<ImpactAnalysis>,
    pub user_confirmed: bool,
    pub dry_run_first: bool,
    pub preserve_newer: bool,
}

/// Rollback target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackTarget {
    Entity(ContainerID),
    Transaction(u64),
    Checkpoint(u64),
    FullSystem,
}

/// Impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub affected_containers: Vec<ContainerID>,
    pub affected_relationships: Vec<ContainerID>,
    pub dependent_tasks: Vec<u64>,
    pub estimated_data_loss: u64,
    pub data_loss_risk: f32,
    pub warnings: Vec<String>,
    pub recommendation: String,
}
