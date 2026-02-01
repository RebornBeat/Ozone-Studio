//! Context types - derived from sections 7 and 34

use serde::{Deserialize, Serialize};
use super::{ContainerID, Blake3Hash};

/// Package context for dependency tracking (§8.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageContext {
    pub package_manager: PackageManager,
    pub lock_file_path: Option<String>,
    pub packages: Vec<PackageInfo>,
    pub last_updated: u64,
}

/// Package managers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Cargo,
    Pip,
    Poetry,
    Maven,
    Gradle,
    Go,
    Gem,
    Composer,
    NuGet,
    Custom(String),
}

/// Package info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub version_constraint: String,
    pub is_outdated: bool,
    pub breaking_changes: Vec<BreakingChange>,
    pub deprecations: Vec<String>,
    pub documentation_url: Option<String>,
    pub package_ref_id: Option<ContainerID>,
}

/// Breaking change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    pub from_version: String,
    pub to_version: String,
    pub description: String,
    pub migration_guide: Option<String>,
}

/// Resource capacity for devices
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceCapacity {
    pub cpu_cores: u8,
    pub memory_gb: f32,
    pub disk_gb: f32,
    pub gpu_available: bool,
    pub gpu_memory_gb: Option<f32>,
}

// ============================================================================
// ML TRAVERSAL TYPES (§16)
// ============================================================================

/// ML Traversal Model for optimized ZSEI navigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalMLModel {
    pub model_id: u64,
    pub layer_depth: u16,
    pub modality: Option<String>,
    pub category: Option<u64>,
    
    pub model_file: String,
    pub version: super::SemVer,
    
    pub trained_on_samples: u64,
    pub training_date: u64,
    
    pub accuracy: f32,
    pub precision: f32,
    pub recall: f32,
    
    pub status: MLModelStatus,
    pub confidence_threshold: f32,
}

/// ML Model Status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MLModelStatus {
    Training,
    Validating,
    Active,
    Inactive,
    Deprecated,
}

impl Default for MLModelStatus {
    fn default() -> Self {
        Self::Inactive
    }
}

/// Traversal training data sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalTrainingSample {
    pub sample_id: u64,
    pub timestamp: u64,
    pub start_container: u64,
    pub query_context: String,
    pub query_embedding: Vec<f32>,
    pub path_taken: Vec<u64>,
    pub containers_visited: Vec<u64>,
    pub containers_relevant: Vec<u64>,  // Confirmed by zero-shot
    pub final_results: Vec<u64>,
}

/// Traversal training dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalTrainingDataset {
    pub dataset_id: u64,
    pub samples: Vec<TraversalTrainingSample>,
    pub categories_covered: Vec<u64>,
    pub total_samples: u64,
    pub created_at: u64,
    pub last_updated: u64,
}

// ============================================================================
// MULTI-DEVICE TYPES (§18)
// ============================================================================

/// Device registry for multi-device resource pooling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistry {
    pub user_id: u64,
    pub devices: Vec<RegisteredDevice>,
}

/// Registered device for task distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredDevice {
    pub device_id: u64,
    pub device_name: String,
    pub device_type: super::auth::DeviceType,
    pub public_key: Vec<u8>,
    
    pub address: String,
    pub port: u16,
    pub last_seen: u64,
    pub status: super::auth::DeviceStatus,
    
    pub total_resources: ResourceCapacity,
    pub available_resources: ResourceCapacity,
    
    pub running_tasks: Vec<u64>,
    pub queued_tasks: Vec<u64>,
}

/// Resource view for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceView {
    pub user_id: u64,
    pub total_resources: ResourceCapacity,
    pub used_resources: ResourceCapacity,
    pub devices: Vec<DeviceResourceView>,
}

/// Device resource view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceResourceView {
    pub device_id: u64,
    pub device_name: String,
    pub status: super::auth::DeviceStatus,
    pub resources: ResourceCapacity,
    pub running_tasks: Vec<TaskSummary>,
}

/// Task summary for resource view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub task_id: u64,
    pub pipeline_name: String,
    pub progress: f32,
    pub started_at: u64,
}
