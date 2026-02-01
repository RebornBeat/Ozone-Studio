//! Authentication types - Section 4 of the specification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{UserID, DeviceID, WorkspaceID, PublicKey};

/// User account (§4.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: UserID,
    pub public_key: PublicKey,
    pub private_key_hash: Vec<u8>,
    pub registered_devices: Vec<DeviceRegistration>,
    pub devices: Vec<DeviceRegistration>, // Alias for compatibility
    pub workspaces: Vec<WorkspaceID>,
    pub permissions: Permissions,
    pub contribution_score: f32,
    pub contribution_rank: Option<u32>,
    pub contribution_count: u64,
    pub reputation: f64,
    pub created_at: u64,
    pub last_login: u64,
    pub last_seen: u64,
}

impl Default for User {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            user_id: 0,
            public_key: Vec::new(),
            private_key_hash: Vec::new(),
            registered_devices: Vec::new(),
            devices: Vec::new(),
            workspaces: Vec::new(),
            permissions: Permissions::default(),
            contribution_score: 0.0,
            contribution_rank: None,
            contribution_count: 0,
            reputation: 0.5,
            created_at: now,
            last_login: now,
            last_seen: now,
        }
    }
}

/// Device registration (§4.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistration {
    pub device_id: DeviceID,
    pub device_name: String,
    pub device_type: DeviceType,
    pub public_key: PublicKey,
    pub registered_at: u64,
    pub last_seen: u64,
    pub status: DeviceStatus,
    pub resource_contribution: ResourceAllocation,
    pub capabilities: Vec<String>,
}

impl Default for DeviceRegistration {
    fn default() -> Self {
        Self {
            device_id: 0,
            device_name: String::new(),
            device_type: DeviceType::Desktop,
            public_key: Vec::new(),
            registered_at: 0,
            last_seen: 0,
            status: DeviceStatus::Offline,
            resource_contribution: ResourceAllocation::default(),
            capabilities: Vec::new(),
        }
    }
}

/// Device types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Desktop,
    Laptop,
    Mobile,
    Server,
    Custom(String),
}

/// Device status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Online,
    Offline,
    Busy,
    Suspended,
}

/// Resource allocation for a device
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceAllocation {
    pub cpu_cores: u8,
    pub memory_gb: f32,
    pub disk_gb: f32,
    pub gpu_available: bool,
    pub gpu_memory_gb: Option<f32>,
}

/// Session information (§4.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: u128,
    pub user_id: UserID,
    pub device_id: DeviceID,
    pub token: Vec<u8>,
    pub session_token: Vec<u8>,
    pub expires_at: u64,
    pub created_at: u64,
    pub last_activity: u64,
    pub active_workspace: Option<WorkspaceID>,
    pub active_project: Option<u64>,
}

impl Default for Session {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            session_id: 0,
            user_id: 0,
            device_id: 0,
            token: Vec::new(),
            session_token: Vec::new(),
            expires_at: now + 3600,
            created_at: now,
            last_activity: now,
            active_workspace: None,
            active_project: None,
        }
    }
}

/// User permissions (§4.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub can_create_pipelines: bool,
    pub can_modify_global: bool,
    pub can_access_distributed: bool,
    pub can_propose_consensus: bool,
    pub max_concurrent_tasks: u32,
    pub workspace_permissions: HashMap<WorkspaceID, WorkspacePermission>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            can_create_pipelines: true,
            can_modify_global: false,
            can_access_distributed: true,
            can_propose_consensus: true,
            max_concurrent_tasks: 10,
            workspace_permissions: HashMap::new(),
        }
    }
}

/// Workspace-specific permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePermission {
    pub can_read: bool,
    pub can_write: bool,
    pub can_delete: bool,
    pub can_share: bool,
}

impl Default for WorkspacePermission {
    fn default() -> Self {
        Self {
            can_read: true,
            can_write: true,
            can_delete: true,
            can_share: false,
        }
    }
}

/// Authentication challenge for login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub challenge_id: u64,
    pub challenge: Vec<u8>,
    pub nonce: [u8; 32],
    pub public_key: Vec<u8>,
    pub created_at: u64,
    pub expires_at: u64,
}

/// Authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub challenge_id: u64,
    pub signature: Vec<u8>,
    pub public_key: PublicKey,
}

/// Permission enum for explicit permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Delete,
    Execute,
    Admin,
}
