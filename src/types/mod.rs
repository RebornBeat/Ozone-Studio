//! Core type definitions for Ozone Studio
//! Based on OZONE STUDIO — FORMAL SPECIFICATION v0.3

pub mod auth;
pub mod blueprint;
pub mod consciousness;
pub mod consensus;
pub mod container;
pub mod context;
pub mod external;
pub mod integrity;
pub mod methodology;
pub mod modality;
pub mod pipeline;
pub mod task;
pub mod ui;
pub mod zsei;

// Re-export commonly used types to simplify imports
// Container types
pub use container::{CodeContext, FileContext, TextContext};
pub use container::{Container, ContainerType, GlobalState, LocalState, Metadata, Modality};
pub use container::{Context, IntegrityData, StoragePointers, TraversalHints};

// Auth types
pub use auth::{
    AuthChallenge, AuthResponse, DeviceRegistration, DeviceType, Permissions, Session, User,
};

// Task types
pub use task::{ExecutionStatus, LogEntry, LogLevel, TaskError};
pub use task::{ResourceUsage, Task, TaskExecutionState, TaskInput, TaskOutput, TaskStatus};

// Pipeline types
pub use pipeline::{
    BlueprintSpec, BuiltinPipeline, ConsensusStatus, ExecutionContext, ExecutionFlow, Schema,
};
pub use pipeline::{PipelineBlueprint, PipelineInput, PipelineOutput};

// ZSEI types
pub use zsei::{IntegrityCheckResult as ZSEIIntegrityResult, IntegrityIssue, IntegrityIssueType};
pub use zsei::{TaskSignature, TraversalRequest, TraversalResult, ZSEIQuery, ZSEIQueryResult};

// Integrity types
pub use integrity::{ImpactAnalysis, RollbackRequest};
pub use integrity::{IntegrityCheck, IntegrityCheckResult, IntegrityCheckType};

// Blueprint types
pub use blueprint::{Blueprint, BlueprintStep};

pub use modality::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generic value type for flexible data storage
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Bytes(b) => write!(f, "[{} bytes]", b.len()),
            Value::Array(a) => write!(f, "[{} items]", a.len()),
            Value::Map(m) => write!(f, "{{{} keys}}", m.len()),
        }
    }
}

/// Severity levels used across the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Semantic version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Default for SemVer {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 3,
            patch: 0,
        }
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Blake3 hash type alias
pub type Blake3Hash = [u8; 32];

/// Public key type alias
pub type PublicKey = Vec<u8>;

/// Container ID type alias
pub type ContainerID = u64;

/// Pipeline ID type alias
pub type PipelineID = u64;

/// Task ID type alias
pub type TaskID = u64;

/// User ID type alias
pub type UserID = u64;

/// Device ID type alias
pub type DeviceID = u64;

/// Workspace ID type alias
pub type WorkspaceID = u64;

/// Project ID type alias
pub type ProjectID = u64;

/// Result type for Ozone operations
pub type OzoneResult<T> = Result<T, OzoneError>;

/// Error types for Ozone Studio
#[derive(Debug, thiserror::Error)]
pub enum OzoneError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("ZSEI error: {0}")]
    ZSEIError(String),

    #[error("Pipeline error: {0}")]
    PipelineError(String),

    #[error("Task error: {0}")]
    TaskError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Integrity error: {0}")]
    IntegrityError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("External reference error: {0}")]
    ExternalRefError(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Consciousness error: {0}")]
    ConsciousnessError(String),
}
