//! Task types - Sections 11-12 of the specification

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use super::{TaskID, PipelineID, ContainerID, UserID, DeviceID, Value};

/// Task record (§11.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: TaskID,
    pub status: TaskStatus,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    
    // Pipeline linkage
    pub pipeline_used: PipelineID,
    pub pipeline_container: Option<ContainerID>,
    
    // Hierarchy
    pub parent_task_id: Option<TaskID>,
    pub child_tasks: Vec<TaskID>,
    
    // Data
    pub inputs: Vec<TaskInput>,
    pub outputs: Vec<TaskOutput>,
    
    // Execution context
    pub user_id: UserID,
    pub device_id: DeviceID,
    pub workspace_id: Option<ContainerID>,
    pub project_id: Option<ContainerID>,
    pub task_context_id: Option<ContainerID>,
    
    // Execution state (per-run data)
    pub execution_state_id: Option<u64>,
    
    // Observability
    pub logs: Vec<LogEntry>,
    pub error: Option<TaskError>,
    pub progress: f32,
    
    // Resources
    pub resource_usage: ResourceUsage,
    
    // Consciousness (if enabled)
    
    pub consciousness_observed: bool,
    
    pub consciousness_intervened: bool,
    
    pub intervention_type: Option<InterventionType>,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            task_id: 0,
            status: TaskStatus::Queued,
            created_at: 0,
            started_at: None,
            completed_at: None,
            pipeline_used: 0,
            pipeline_container: None,
            parent_task_id: None,
            child_tasks: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            user_id: 0,
            device_id: 0,
            workspace_id: None,
            project_id: None,
            task_context_id: None,
            execution_state_id: None,
            logs: Vec::new(),
            error: None,
            progress: 0.0,
            resource_usage: ResourceUsage::default(),
            
            consciousness_observed: false,
            
            consciousness_intervened: false,
            
            intervention_type: None,
        }
    }
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
    
    AwaitingClarification,
}

/// Task input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInput {
    pub name: String,
    pub value: Value,
    pub source: Option<ContainerID>,
}

/// Task output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    pub name: String,
    pub value: Value,
    pub stored_at: Option<ContainerID>,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Task error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskError {
    pub error_type: String,
    pub message: String,
    pub stack_trace: Option<String>,
    pub recoverable: bool,
}

/// Resource usage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_peak_mb: u64,
    pub disk_read_mb: u64,
    pub disk_write_mb: u64,
    pub network_sent_mb: u64,
    pub network_recv_mb: u64,
}

/// Intervention type (consciousness)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterventionType {
    Clarification,
    Modification,
    Pause,
    Cancel,
}

/// Task Execution State (§12.1) - tracks runtime data for a specific execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionState {
    pub execution_state_id: u64,
    pub task_id: TaskID,
    pub blueprint_id: u64,
    
    // Step tracking
    pub current_step: u32,
    pub total_steps: u32,
    pub step_states: Vec<StepExecutionState>,
    
    // Data per this execution
    pub step_inputs: HashMap<u64, Value>,
    pub step_outputs: HashMap<u64, Value>,
    pub intermediate_results: Vec<IntermediateResult>,
    
    // Execution flow
    pub execution_path: Vec<u64>,
    pub skipped_steps: Vec<u64>,
    
    // State
    pub started_at: u64,
    pub last_updated: u64,
    pub status: ExecutionStatus,
    
    // Preservation options
    pub preserve_for_learning: bool,
    pub drop_on_completion: bool,
}

/// Step execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionState {
    pub step_id: u64,
    pub blueprint_step_id: u64,
    pub status: StepStatus,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub error: Option<String>,
}

/// Step status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Intermediate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermediateResult {
    pub after_step: u64,
    pub result_type: String,
    pub value: Value,
    pub timestamp: u64,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Initializing,
    Running,
    Paused,
    AwaitingInput,
    Completed,
    Failed,
    Cancelled,
}

/// Task graph for visualization (§11.4)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskGraph {
    pub nodes: Vec<TaskGraphNode>,
    pub edges: Vec<TaskGraphEdge>,
    pub layout: GraphLayout,
}

/// Task graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraphNode {
    pub task_id: TaskID,
    pub label: String,
    pub status: TaskStatus,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub color: Color,
}

/// Color for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Task graph edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGraphEdge {
    pub from_task: TaskID,
    pub to_task: TaskID,
    pub edge_type: TaskEdgeType,
}

/// Task edge types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskEdgeType {
    ParentChild,
    DataDependency,
    Sequential,
}

/// Graph layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GraphLayout {
    #[default]
    Hierarchical,
    ForceDirected,
    Timeline,
    Gantt,
}

/// Task context for knowledge aggregation (§7.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub task_id: TaskID,
    
    // Linked resources
    pub workspace_context: ContainerID,
    pub project_context: Option<ContainerID>,
    
    // Aggregated context for this task
    pub relevant_files: Vec<ContainerID>,
    pub relevant_chunks: Vec<ContainerID>,
    pub relevant_external_refs: Vec<ContainerID>,
    pub methodologies_used: Vec<ContainerID>,
    pub blueprint_id: Option<ContainerID>,
    
    // Context window management
    pub token_budget: u32,
    pub current_tokens: u32,
    
    // Multi-pass organization
    pub passes_completed: u32,
    pub organization_state: OrganizationState,
    
    // Context blueprint (how to chunk/organize)
    pub context_blueprint: TaskContextBlueprint,
    
    // Consciousness context (if enabled)
    
    pub consciousness_context: Option<TaskConsciousnessContext>,
}

/// Organization state for multi-pass context building
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrganizationState {
    pub reviewed_files: HashSet<ContainerID>,
    pub reviewed_chunks: HashSet<ContainerID>,
    pub relevance_scores: HashMap<ContainerID, f32>,
    pub included_items: Vec<ContextItem>,
}

/// Context item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub container_id: ContainerID,
    pub chunk_id: Option<u64>,
    pub relevance_score: f32,
    pub token_count: u32,
}

/// Task context blueprint (§34)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContextBlueprint {
    pub blueprint_id: u64,
    pub task_type: ContextTaskType,
    pub name: String,
    pub description: String,
    pub context_structure: ContextStructure,
    pub chunking_strategy: ChunkingStrategy,
    pub priority_rules: Vec<PriorityRule>,
    pub token_allocation: TokenAllocation,
    pub overflow_strategy: OverflowStrategy,
    pub integrity_constraints: IntegrityConstraints,
    
    // Consciousness additions
    
    pub emotional_context_weight: f32,
    
    pub experience_context_weight: f32,
    
    pub relationship_context_weight: f32,
}

/// Context task types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextTaskType {
    CodeGeneration,
    CodeAnalysis,
    TextGeneration,
    TextAnalysis,
    DataProcessing,
    Research,
    Conversation,
    CreativeWriting,
    ProblemSolving,
    Custom(String),
}

/// Context structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextStructure {
    pub sections: Vec<ContextSection>,
    pub section_order: Vec<u64>,
    pub required_sections: Vec<u64>,
    pub optional_sections: Vec<u64>,
}

/// Context section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSection {
    pub section_id: u64,
    pub name: String,
    pub section_type: SectionType,
    pub max_tokens: Option<u32>,
    pub min_tokens: Option<u32>,
    pub priority: u8,
}

/// Section types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionType {
    // Standard
    SystemPrompt,
    TaskDescription,
    RelevantFiles,
    RelevantChunks,
    ExternalReferences,
    Methodologies,
    Examples,
    Constraints,
    
    // Consciousness
    
    EmotionalContext,
    
    RetrievedExperiences,
    
    RelationshipHistory,
    
    EthicalGuidelines,
    
    IdentityReminders,
    
    NarrativeContext,
}

/// Chunking strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingStrategy {
    pub strategy_type: ChunkingType,
    pub max_chunk_size: u32,
    pub overlap_tokens: u32,
    pub boundary_preservation: BoundaryPreservation,
}

/// Chunking types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkingType {
    Fixed,
    Semantic,
    Structural,
    Adaptive,
}

/// Boundary preservation settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoundaryPreservation {
    pub preserve_paragraphs: bool,
    pub preserve_sentences: bool,
    pub preserve_code_blocks: bool,
    pub preserve_function_boundaries: bool,
    pub preserve_class_boundaries: bool,
}

/// Priority rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityRule {
    pub condition: String,
    pub priority_boost: i8,
    pub reason: String,
}

/// Token allocation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenAllocation {
    pub total_budget: u32,
    pub section_allocations: HashMap<u64, u32>,
    pub dynamic_reallocation: bool,
    
    pub consciousness_reserve: u32,
}

/// Overflow strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverflowStrategy {
    Truncate,
    Summarize,
    Split,
    Error,
}

/// Integrity constraints
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrityConstraints {
    pub verify_chunk_boundaries: bool,
    pub verify_semantic_coherence: bool,
    pub verify_relationship_preservation: bool,
    pub max_information_loss: f32,
    pub require_reconstruction_test: bool,
}

/// Task consciousness context (consciousness feature)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConsciousnessContext {
    pub emotional_state_at_start: super::consciousness::EmotionalState,
    pub emotional_state_current: super::consciousness::EmotionalState,
    pub retrieved_experiences: Vec<ContainerID>,
    pub identity_implications: Option<String>,
    pub relationship_context: Option<super::consciousness::RelationshipContext>,
    pub decision_gate_result: Option<super::consciousness::ConsciousnessDecisionGate>,
    pub narrative_fragments: Vec<crate::consciousness::NarrativeFragment>,
}

// ============================================================================
// EXECUTION ENVIRONMENT TYPES (§22)
// ============================================================================

/// Execution environment for isolated task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionEnvironment {
    /// Direct execution in current process
    Native,
    /// Isolated subprocess
    Process,
    /// Container isolation (Docker/OCI)
    Container,
}

impl Default for ExecutionEnvironment {
    fn default() -> Self {
        Self::Process
    }
}

/// Environment configuration for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEnvironmentConfig {
    pub environment_type: ExecutionEnvironment,
    pub resource_allocation: TaskResourceAllocation,
    pub isolation_level: IsolationLevel,
    pub timeout_sec: u64,
    pub zsei_read_containers: Vec<u64>,
    pub zsei_write_containers: Vec<u64>,
}

/// Task-specific resource allocation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskResourceAllocation {
    pub cpu_cores: u8,
    pub memory_mb: u64,
    pub disk_mb: u64,
    pub gpu_access: bool,
    pub network_access: bool,
    pub max_duration_sec: u64,
}

/// Isolation level for task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// No isolation (shared process)
    None,
    /// Process-level isolation
    Process,
    /// Filesystem isolation
    Filesystem,
    /// Full container isolation
    Full,
}

impl Default for IsolationLevel {
    fn default() -> Self {
        Self::Process
    }
}
