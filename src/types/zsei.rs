//! ZSEI traversal types - Section 6.7 of the specification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{ContainerID, Value};
use super::container::{Container, Modality};

/// Traversal request (§6.7)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalRequest {
    pub start_container: ContainerID,
    pub mode: TraversalMode,
    pub filters: Vec<Filter>,
    pub max_depth: u16,
    pub max_results: u32,
    pub budget: TraversalBudget,
    pub use_ml: bool,
    pub include_methodologies: bool,
    pub include_external_refs: bool,
    pub keyword_filter: Option<Vec<String>>,
    pub topic_filter: Option<Vec<String>>,
}

impl Default for TraversalRequest {
    fn default() -> Self {
        Self {
            start_container: 0,
            mode: TraversalMode::Hybrid,
            filters: Vec::new(),
            max_depth: 10,
            max_results: 100,
            budget: TraversalBudget::default(),
            use_ml: false,
            include_methodologies: true,
            include_external_refs: true,
            keyword_filter: None,
            topic_filter: None,
        }
    }
}

/// Traversal modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalMode {
    Structural,
    Semantic,
    Contextual,
    Hybrid,
    MLGuided,
    BruteForce,
}

/// Filter for traversal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub operator: Operator,
    pub value: Value,
}

/// Filter operators
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    In,
    HasKeyword,
    HasTopic,
    Custom(String),
}

/// Traversal budget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalBudget {
    pub max_hops: u16,
    pub max_containers: u32,
    pub max_latency_ms: u32,
}

impl Default for TraversalBudget {
    fn default() -> Self {
        Self {
            max_hops: 10,
            max_containers: 1000,
            max_latency_ms: 5000,
        }
    }
}

/// Traversal result
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraversalResult {
    pub containers: Vec<ContainerID>,
    pub distances: Vec<f32>,
    pub paths: Vec<Path>,
    pub stats: TraversalStats,
    pub methodologies: Vec<ContainerID>,
    pub external_refs: Vec<ContainerID>,
    pub keywords_found: Vec<String>,
    pub topics_found: Vec<String>,
}

/// Path in traversal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
    pub hops: Vec<ContainerID>,
    pub total_distance: f32,
}

/// Traversal statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraversalStats {
    pub containers_visited: u32,
    pub hops_taken: u16,
    pub latency_ms: u32,
    pub cache_hits: u32,
    pub ml_predictions_used: u32,
    pub brute_force_fallback: bool,
}

/// ZSEI query types (§6.8)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZSEIQuery {
    // User Organization
    GetUserWorkspaces { user_id: u64 },
    GetProjects { workspace_id: ContainerID },
    GetProjectContext { project_id: ContainerID },
    GetFileReferences { project_id: ContainerID },
    GetExternalReferences { project_id: ContainerID },
    
    // Category/Methodology
    GetCategories { modality: Modality, parent_category: Option<ContainerID> },
    GetMethodologies { category_ids: Vec<ContainerID> },
    GetMethodologiesByKeywords { keywords: Vec<String> },
    GetMethodologiesByTopics { topics: Vec<String> },
    
    // Blueprint
    SearchBlueprints { task_signature: TaskSignature },
    SearchBlueprintsByKeywords { keywords: Vec<String> },
    
    // External References
    GetPackageInfo { registry: super::container::PackageRegistry, name: String },
    GetURLContext { url: String },
    
    // Semantic
    SemanticSearch { embedding: Vec<f32>, top_k: u32, filters: Vec<Filter> },
    
    // Context
    GetContextForTask { task_id: u64, token_budget: u32 },
    GetWorkspaceContext { workspace_id: ContainerID },
    
    // Traversal
    Traverse(TraversalRequest),
    
    // Write Operations
    CreateContainer { parent_id: ContainerID, container: Container },
    UpdateContainer { container_id: ContainerID, updates: ContainerUpdate },
    DeleteContainer { container_id: ContainerID },
    LinkFile { project_id: ContainerID, file_path: String },
    LinkURL { project_id: ContainerID, url: String },
    LinkPackage { 
        project_id: ContainerID, 
        registry: super::container::PackageRegistry, 
        name: String, 
        version: String 
    },
    UnlinkFile { project_id: ContainerID, file_ref_id: ContainerID },
    
    // Integrity
    VerifyIntegrity { container_id: ContainerID },
    GetVersionHistory { container_id: ContainerID },
    Rollback { container_id: ContainerID, to_version: u64 },
}

/// Task signature for blueprint matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSignature {
    pub input_types: Vec<String>,
    pub output_type: String,
    pub constraints: Vec<String>,
    pub hash: super::Blake3Hash,
}

/// Container update for partial updates
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerUpdate {
    pub metadata: Option<super::container::Metadata>,
    pub context: Option<super::container::Context>,
    pub storage: Option<super::container::StoragePointers>,
    pub hints: Option<super::container::TraversalHints>,
}

/// ZSEI query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZSEIQueryResult {
    Containers(Vec<ContainerID>),
    Container(Container),
    ContainerID(ContainerID),
    TraversalResult(TraversalResult),
    Success,
    VersionHistory(Vec<super::container::VersionRecord>),
    IntegrityResult(IntegrityCheckResult),
    Error(String),
}

/// Integrity check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheckResult {
    pub container_id: ContainerID,
    pub passed: bool,
    pub issues: Vec<IntegrityIssue>,
    pub checked_at: u64,
}

/// Integrity issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityIssue {
    pub issue_type: IntegrityIssueType,
    pub description: String,
    pub severity: super::Severity,
    pub auto_repairable: bool,
}

/// Integrity issue types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrityIssueType {
    ChunkBoundaryBreak,
    OrphanedReference,
    SemanticDrift,
    VersionMismatch,
    ReconstructionFailure,
    RelationshipBroken,
    HashMismatch,
}
