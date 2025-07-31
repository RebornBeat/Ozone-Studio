// =============================================================================
// bridge-linux/src/task_interruption/mod.rs
// Universal Task Interruption and Override System for Human Authority
// =============================================================================

use std::collections::HashMap;
use std::time::{Duration, SystemTime, Instant};
use std::sync::Arc;
use std::fmt;

// Async runtime and concurrency - Essential for coordinating interruptions across the ecosystem
use tokio::sync::{RwLock, Mutex, mpsc, oneshot, broadcast};
use tokio::time::{sleep, timeout, interval};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Import shared protocol types for ecosystem communication
use shared_protocols::{
    EcosystemIdentity,
    ComponentType,
    TaskOrchestrationRequest,
    AIAppCoordinationRequest,
    AIAppCoordinationResponse,
    ExecutionStatus,
    HumanGuidance,
    AuthorityLevel,
    ProtocolError,
};

// Import security types for authority validation
use shared_security::{
    SecurityError,
    AuthenticationCredentials,
    AuthorizationProvider,
    Permission,
    SecurityResult,
};

// Import methodology runtime types for execution context
use methodology_runtime::{
    ExecutionContext,
    ExecutionResult,
    MethodologyRuntimeError,
    ExecutionStatus as MethodologyExecutionStatus,
};

// Core task interruption components
pub mod interruption_handler;
pub mod override_coordinator;
pub mod authority_validator;
pub mod safe_pause_manager;
pub mod resumption_coordinator;

// Supporting components for interruption workflow
pub mod operation_discovery;
pub mod context_preservation;
pub mod modification_integration;
pub mod safety_validation;
pub mod escalation_management;

// Re-export all core interruption types and functionality
pub use interruption_handler::{
    InterruptionHandler,
    InterruptionWorkflow,
    InterruptionResult,
    InterruptionMetrics,
    WorkflowState,
    InterruptionPriority,
};

pub use override_coordinator::{
    OverrideCoordinator,
    EcosystemOverride,
    OperationOverride,
    CoordinationResult,
    OverrideStrategy,
    SafetyProtocol,
};

pub use authority_validator::{
    AuthorityValidator,
    AuthorityValidation,
    PermissionCheck,
    AuthorityScope,
    ValidationResult,
    AuthorityLevel as TaskAuthorityLevel,
};

pub use safe_pause_manager::{
    SafePauseManager,
    PausePointIdentifier,
    StatePreservation,
    PauseStrategy,
    RecoveryInfo,
    PauseResult,
};

pub use resumption_coordinator::{
    ResumptionCoordinator,
    ResumptionWorkflow,
    ModificationIntegration,
    ResumptionValidation,
    ResumptionResult,
    ContinuationStrategy,
};

// Supporting component exports
pub use operation_discovery::{
    OperationDiscovery,
    ActiveOperation,
    OperationHierarchy,
    DependencyMapping,
    DiscoveryResult,
};

pub use context_preservation::{
    ContextPreservation,
    ExecutionSnapshot,
    StateCapture,
    ContextIntegrity,
    PreservationResult,
};

pub use modification_integration::{
    ModificationIntegration,
    HumanModification,
    IntegrationStrategy,
    CompatibilityCheck,
    IntegrationResult,
};

pub use safety_validation::{
    SafetyValidation,
    SafetyCheck,
    RiskAssessment,
    SafetyProtocol,
    ValidationOutcome,
};

pub use escalation_management::{
    EscalationManagement,
    EscalationTrigger,
    EscalationPath,
    ExpertNotification,
    EscalationResult,
};

// =============================================================================
// CORE INTERRUPTION REQUEST AND RESPONSE TYPES
// =============================================================================

/// Primary request structure for task interruption operations
/// This encapsulates all information needed to safely interrupt ecosystem operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionRequest {
    /// Unique identifier for this interruption request
    pub interruption_id: String,
    
    /// The type of interruption being requested (pause, modify, cancel, override)
    pub interruption_type: InterruptionType,
    
    /// Authority information about the requesting user
    pub user_authority: UserAuthority,
    
    /// Human-readable reason for the interruption
    pub reason: String,
    
    /// Optional target specification - if None, affects all active operations
    pub target_specification: Option<TargetSpecification>,
    
    /// Specific modification instructions if this is a modify operation
    pub modification_instructions: Option<ModificationInstructions>,
    
    /// Safety requirements that must be maintained during interruption
    pub safety_requirements: SafetyRequirements,
    
    /// Timeout for the interruption operation itself
    pub interruption_timeout: Duration,
    
    /// Priority level for this interruption request
    pub priority: InterruptionPriority,
    
    /// Whether to preserve operation context for potential resumption
    pub preserve_context: bool,
    
    /// Whether to wait for safe pause points or interrupt immediately
    pub force_immediate: bool,
}

/// The different types of interruptions that can be requested
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptionType {
    /// Temporarily pause operations - can be resumed later
    Pause,
    
    /// Modify ongoing operations with new instructions or parameters
    Modify,
    
    /// Completely cancel operations - cannot be resumed
    Cancel,
    
    /// Override current operations with completely new instructions
    Override,
    
    /// Emergency stop - immediate halt regardless of safety considerations
    EmergencyStop,
    
    /// Graceful shutdown - orderly termination of all operations
    GracefulShutdown,
}

/// Information about the user requesting the interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAuthority {
    /// User identifier from the authentication system
    pub user_id: String,
    
    /// The user's authority level for task interruption
    pub authority_level: AuthorityLevel,
    
    /// Specific permissions the user has
    pub permissions: Vec<Permission>,
    
    /// Authentication credentials for verification
    pub credentials: AuthenticationCredentials,
    
    /// Device from which the interruption request originated
    pub originating_device: String,
    
    /// Session information for audit trail
    pub session_context: SessionContext,
}

/// Session context for audit and security tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub session_id: String,
    pub session_start: SystemTime,
    pub previous_interactions: Vec<String>,
    pub security_level: String,
}

/// Specification of which operations to target for interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetSpecification {
    /// Target all operations across the entire ecosystem
    AllOperations,
    
    /// Target operations involving specific AI Apps
    SpecificAIApps(Vec<ComponentType>),
    
    /// Target specific task or methodology executions
    SpecificTasks(Vec<String>),
    
    /// Target operations matching certain criteria
    OperationCriteria(OperationCriteria),
    
    /// Target a specific hierarchy of related operations
    OperationHierarchy(String),
}

/// Criteria for selecting which operations to interrupt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationCriteria {
    /// Operations that have been running longer than this duration
    pub min_duration: Option<Duration>,
    
    /// Operations with resource usage above this threshold
    pub resource_threshold: Option<f64>,
    
    /// Operations with these priority levels
    pub priority_levels: Vec<TaskPriority>,
    
    /// Operations in these execution states
    pub execution_states: Vec<ExecutionStatus>,
    
    /// Operations involving these domains
    pub domains: Vec<String>,
}

/// Priority levels for tasks that can be interrupted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Background,
    Normal,
    High,
    Critical,
    Emergency,
}

/// Instructions for how to modify ongoing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationInstructions {
    /// New requirements to integrate into ongoing operations
    pub new_requirements: Option<String>,
    
    /// Changes to resource allocation or priorities
    pub priority_changes: HashMap<String, TaskPriority>,
    
    /// Adjustments to resource usage limits
    pub resource_adjustments: HashMap<String, f64>,
    
    /// New coordination approaches to apply
    pub approach_modifications: Option<String>,
    
    /// Additional constraints to enforce
    pub additional_constraints: Vec<String>,
    
    /// Parameters to modify in ongoing methodologies
    pub parameter_modifications: HashMap<String, serde_json::Value>,
}

/// Safety requirements that must be maintained during interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRequirements {
    /// Whether to preserve work progress during interruption
    pub preserve_progress: bool,
    
    /// Whether to maintain operational context for resumption
    pub maintain_context: bool,
    
    /// Whether to ensure data coherence is preserved
    pub ensure_coherence: bool,
    
    /// Whether resumption must be possible after interruption
    pub safe_resumption: bool,
    
    /// Whether to maintain relationships between interrupted operations
    pub preserve_relationships: bool,
    
    /// Maximum acceptable data loss tolerance
    pub data_loss_tolerance: DataLossTolerance,
    
    /// Required backup creation before interruption
    pub backup_requirements: BackupRequirements,
}

/// Tolerance levels for data loss during interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataLossTolerance {
    /// No data loss acceptable - must find perfect pause points
    Zero,
    
    /// Minimal data loss acceptable - can lose recent uncommitted work
    Minimal,
    
    /// Moderate data loss acceptable - can lose current operation results
    Moderate,
    
    /// High data loss acceptable - can lose significant progress if necessary
    High,
    
    /// Any data loss acceptable - emergency situations only
    Any,
}

/// Requirements for backup creation during interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupRequirements {
    /// No backup required
    None,
    
    /// Create backup of current state only
    StateOnly,
    
    /// Create backup of state and partial results
    StateAndResults,
    
    /// Create comprehensive backup of all operation data
    Comprehensive,
    
    /// Create backup and verify integrity before proceeding
    VerifiedBackup,
}

// =============================================================================
// INTERRUPTION RESPONSE AND RESULT TYPES
// =============================================================================

/// Response structure for interruption requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionResponse {
    /// The original interruption request ID
    pub interruption_id: String,
    
    /// Current status of the interruption operation
    pub interruption_status: InterruptionStatus,
    
    /// Information about affected operations and components
    pub affected_operations: Vec<AffectedOperation>,
    
    /// State preservation information if context was saved
    pub preserved_state: Option<PreservedState>,
    
    /// Information needed for potential resumption
    pub resumption_info: Option<ResumptionInfo>,
    
    /// Any errors or warnings that occurred during interruption
    pub execution_feedback: ExecutionFeedback,
    
    /// Metrics about the interruption operation performance
    pub interruption_metrics: InterruptionMetrics,
    
    /// Estimated time until resumption is possible (if applicable)
    pub estimated_resume_time: Option<Duration>,
}

/// Status of an interruption operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptionStatus {
    /// Request received and being processed
    Received,
    
    /// Currently discovering and analyzing affected operations
    Analyzing,
    
    /// Validating user authority for the requested interruption
    ValidatingAuthority,
    
    /// Identifying safe pause points in active operations
    FindingPausePoints,
    
    /// Actually executing the interruption across affected components
    Interrupting,
    
    /// Operations successfully paused and state preserved
    Paused,
    
    /// Operations modified according to instructions
    Modified,
    
    /// Operations cancelled and resources cleaned up
    Cancelled,
    
    /// Override completed with new operations started
    Overridden,
    
    /// Interruption completed successfully
    Completed,
    
    /// Interruption failed - operations may still be running
    Failed,
    
    /// Partial success - some operations interrupted, others continue
    PartialSuccess,
}

/// Information about an operation affected by interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedOperation {
    /// Unique identifier for this operation
    pub operation_id: String,
    
    /// Type of AI App or component running this operation
    pub component: ComponentType,
    
    /// Description of what this operation was doing
    pub operation_description: String,
    
    /// How this operation was affected by the interruption
    pub interruption_impact: InterruptionImpact,
    
    /// Progress lost or preserved during interruption
    pub progress_impact: ProgressImpact,
    
    /// Whether this operation can be resumed later
    pub resumable: bool,
    
    /// Dependencies that were affected by interrupting this operation
    pub dependency_impact: Vec<String>,
}

/// How an interruption affected a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptionImpact {
    /// Operation was cleanly paused at a safe point
    CleanlyPaused,
    
    /// Operation was forcibly interrupted with some state loss
    ForciblyInterrupted,
    
    /// Operation was modified and continued with new parameters
    ModifiedAndContinued,
    
    /// Operation was completely cancelled and cleaned up
    CancelledAndCleaned,
    
    /// Operation was replaced with new instructions
    ReplacedWithOverride,
    
    /// Operation was unaffected by the interruption
    Unaffected,
    
    /// Operation failed during interruption attempt
    FailedDuringInterruption,
}

/// Impact on operation progress during interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressImpact {
    /// Percentage of work completed before interruption
    pub completion_percentage: f64,
    
    /// Amount of progress that was preserved
    pub preserved_progress: f64,
    
    /// Amount of work that was lost during interruption
    pub lost_progress: f64,
    
    /// Estimated time to recover lost progress if resumed
    pub recovery_time_estimate: Option<Duration>,
    
    /// Description of what work was preserved or lost
    pub impact_description: String,
}

/// Information about preserved operation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreservedState {
    /// Unique identifier for this preserved state
    pub state_id: String,
    
    /// Location where the state information is stored
    pub storage_location: String,
    
    /// Timestamp when the state was preserved
    pub preservation_timestamp: SystemTime,
    
    /// Size of the preserved state data
    pub state_size: u64,
    
    /// Integrity verification information
    pub integrity_hash: String,
    
    /// Expiration time for the preserved state
    pub expiration_time: SystemTime,
    
    /// Operations included in this preserved state
    pub included_operations: Vec<String>,
    
    /// Additional metadata about the preserved state
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Information needed to resume interrupted operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumptionInfo {
    /// Whether resumption is currently possible
    pub resumption_ready: bool,
    
    /// Conditions that must be met before resumption
    pub resumption_requirements: Vec<ResumptionRequirement>,
    
    /// Estimated time needed to prepare for resumption
    pub preparation_time: Duration,
    
    /// Modified expectations due to the interruption
    pub modified_expectations: ModifiedExpectations,
    
    /// Resources that need to be reallocated for resumption
    pub resource_reallocation: ResourceReallocation,
    
    /// Steps required to resume each affected operation
    pub resumption_steps: Vec<ResumptionStep>,
}

/// Requirements that must be met before operations can be resumed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResumptionRequirement {
    /// User must provide explicit authorization to resume
    UserAuthorization,
    
    /// System resources must be available at specified levels
    ResourceAvailability(ResourceRequirement),
    
    /// Dependent operations must be in specific states
    DependencyReadiness(Vec<String>),
    
    /// Data integrity must be verified before resumption
    IntegrityVerification,
    
    /// Backup must be completed before resumption
    BackupCompletion,
    
    /// Modification integration must be validated
    ModificationValidation,
}

/// Specific resource requirements for resumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirement {
    pub resource_type: String,
    pub minimum_level: f64,
    pub preferred_level: f64,
    pub duration_needed: Duration,
}

/// How expectations have changed due to the interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifiedExpectations {
    /// New estimated completion times
    pub completion_estimates: HashMap<String, Duration>,
    
    /// Changed resource requirements
    pub resource_changes: HashMap<String, f64>,
    
    /// Modified quality expectations
    pub quality_adjustments: HashMap<String, f64>,
    
    /// Updated success criteria
    pub success_modifications: Vec<String>,
}

/// Resource reallocation needed for resumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReallocation {
    /// CPU allocation changes needed
    pub cpu_changes: HashMap<String, f64>,
    
    /// Memory allocation changes needed
    pub memory_changes: HashMap<String, f64>,
    
    /// Storage allocation changes needed
    pub storage_changes: HashMap<String, f64>,
    
    /// Network bandwidth changes needed
    pub network_changes: HashMap<String, f64>,
}

/// Individual steps required to resume a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumptionStep {
    /// Unique identifier for this resumption step
    pub step_id: String,
    
    /// Description of what this step accomplishes
    pub step_description: String,
    
    /// Component responsible for executing this step
    pub responsible_component: ComponentType,
    
    /// Prerequisites that must be completed before this step
    pub prerequisites: Vec<String>,
    
    /// Estimated time to complete this step
    pub estimated_duration: Duration,
    
    /// Risk level associated with this resumption step
    pub risk_level: RiskLevel,
}

/// Risk levels for resumption steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Feedback about the execution of the interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionFeedback {
    /// Operations that were successfully interrupted
    pub successful_interruptions: Vec<String>,
    
    /// Operations that failed to interrupt properly
    pub failed_interruptions: Vec<FailedInterruption>,
    
    /// Warnings about potential issues
    pub warnings: Vec<String>,
    
    /// Informational messages about the interruption process
    pub information: Vec<String>,
    
    /// Recommendations for future interruption attempts
    pub recommendations: Vec<String>,
}

/// Information about a failed interruption attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedInterruption {
    /// Operation that failed to interrupt
    pub operation_id: String,
    
    /// Reason the interruption failed
    pub failure_reason: String,
    
    /// Current state of the failed operation
    pub current_state: String,
    
    /// Recommended actions to handle this failure
    pub recommended_actions: Vec<String>,
    
    /// Whether manual intervention is required
    pub manual_intervention_required: bool,
}

// =============================================================================
// CORE TRAIT DEFINITIONS FOR INTERRUPTION SYSTEM
// =============================================================================

/// Main trait for handling task interruption requests
/// This defines the primary interface that BRIDGE uses to interrupt ecosystem operations
pub trait TaskInterruptionInterface {
    /// The type of error that can occur during interruption operations
    type Error;
    
    /// Process a task interruption request
    /// This is the main entry point for all interruption functionality
    async fn interrupt_task(
        &mut self,
        request: InterruptionRequest,
    ) -> Result<InterruptionResponse, Self::Error>;
    
    /// Resume previously interrupted operations
    /// Used to continue operations that were paused rather than cancelled
    async fn resume_operations(
        &mut self,
        resumption_request: ResumptionRequest,
    ) -> Result<ResumptionResult, Self::Error>;
    
    /// Get the current status of an interruption operation
    /// Allows tracking the progress of complex interruption workflows
    async fn get_interruption_status(
        &self,
        interruption_id: &str,
    ) -> Result<InterruptionStatus, Self::Error>;
    
    /// Cancel an in-progress interruption request
    /// Allows aborting interruption attempts that are taking too long
    async fn cancel_interruption(
        &mut self,
        interruption_id: &str,
    ) -> Result<(), Self::Error>;
    
    /// List all current interruption operations
    /// Provides visibility into active interruption workflows
    async fn list_active_interruptions(&self) -> Result<Vec<InterruptionSummary>, Self::Error>;
    
    /// Validate that a user has authority to perform a specific interruption
    /// Pre-validation before attempting expensive interruption operations
    async fn validate_interruption_authority(
        &self,
        user_authority: &UserAuthority,
        target: &TargetSpecification,
    ) -> Result<AuthorityValidation, Self::Error>;
}

/// Trait for coordinating with OZONE STUDIO during interruptions
/// This handles the complex ecosystem-wide coordination needed for safe interruptions
pub trait EcosystemInterruptionCoordinator {
    type Error;
    
    /// Discover all currently active operations in the ecosystem
    async fn discover_active_operations(&self) -> Result<Vec<ActiveOperation>, Self::Error>;
    
    /// Request OZONE STUDIO to pause specific operations
    async fn request_operation_pause(
        &mut self,
        operations: Vec<String>,
        safety_requirements: SafetyRequirements,
    ) -> Result<PauseResult, Self::Error>;
    
    /// Request OZONE STUDIO to modify ongoing operations
    async fn request_operation_modification(
        &mut self,
        modifications: Vec<OperationModification>,
    ) -> Result<ModificationResult, Self::Error>;
    
    /// Request OZONE STUDIO to cancel specific operations
    async fn request_operation_cancellation(
        &mut self,
        operations: Vec<String>,
        cleanup_requirements: CleanupRequirements,
    ) -> Result<CancellationResult, Self::Error>;
    
    /// Coordinate resumption of previously paused operations
    async fn coordinate_operation_resumption(
        &mut self,
        resumption_plan: ResumptionPlan,
    ) -> Result<CoordinationResult, Self::Error>;
}

/// Trait for validating user authority for interruption operations
/// Ensures only authorized users can interrupt specific types of operations
pub trait InterruptionAuthorityValidator {
    type Error;
    
    /// Validate that a user can interrupt operations of a specific type
    async fn validate_operation_authority(
        &self,
        user: &UserAuthority,
        operation_type: &str,
    ) -> Result<bool, Self::Error>;
    
    /// Validate that a user can interrupt operations involving specific AI Apps
    async fn validate_component_authority(
        &self,
        user: &UserAuthority,
        components: &[ComponentType],
    ) -> Result<bool, Self::Error>;
    
    /// Validate that a user can perform a specific type of interruption
    async fn validate_interruption_type_authority(
        &self,
        user: &UserAuthority,
        interruption_type: &InterruptionType,
    ) -> Result<bool, Self::Error>;
    
    /// Get the maximum scope of operations a user can interrupt
    async fn get_user_interruption_scope(
        &self,
        user: &UserAuthority,
    ) -> Result<InterruptionScope, Self::Error>;
}

// =============================================================================
// SUPPORTING TYPES AND STRUCTURES
// =============================================================================

/// Request structure for resuming interrupted operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumptionRequest {
    pub resumption_id: String,
    pub interruption_id: String,
    pub user_authority: UserAuthority,
    pub resumption_strategy: ResumptionStrategy,
    pub modification_integration: Option<ModificationInstructions>,
    pub resource_allocation: Option<ResourceReallocation>,
    pub safety_validation: bool,
}

/// Strategies for resuming interrupted operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResumptionStrategy {
    /// Resume exactly where operations left off
    ExactResumption,
    
    /// Resume with modifications integrated
    ModifiedResumption,
    
    /// Resume with updated resource allocation
    ResourceOptimizedResumption,
    
    /// Resume with enhanced safety checks
    SafetyEnhancedResumption,
    
    /// Resume with full revalidation of all conditions
    FullValidationResumption,
}

/// Summary information about an interruption operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionSummary {
    pub interruption_id: String,
    pub interruption_type: InterruptionType,
    pub requesting_user: String,
    pub start_time: SystemTime,
    pub current_status: InterruptionStatus,
    pub affected_operations_count: usize,
    pub estimated_completion: Option<SystemTime>,
}

/// Modification to apply to an ongoing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationModification {
    pub operation_id: String,
    pub modification_type: ModificationType,
    pub new_parameters: HashMap<String, serde_json::Value>,
    pub resource_adjustments: Option<ResourceAdjustment>,
    pub priority_change: Option<TaskPriority>,
}

/// Types of modifications that can be applied to operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationType {
    ParameterUpdate,
    ResourceReallocation,
    PriorityAdjustment,
    ApproachModification,
    ConstraintAddition,
    RequirementUpdate,
}

/// Adjustment to resource allocation for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAdjustment {
    pub cpu_adjustment: Option<f64>,
    pub memory_adjustment: Option<f64>,
    pub storage_adjustment: Option<f64>,
    pub network_adjustment: Option<f64>,
}

/// Requirements for cleaning up cancelled operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRequirements {
    pub cleanup_temporary_files: bool,
    pub release_resources: bool,
    pub notify_dependencies: bool,
    pub preserve_partial_results: bool,
    pub create_cancellation_report: bool,
}

/// Result of a modification operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationResult {
    pub modification_id: String,
    pub successful_modifications: Vec<String>,
    pub failed_modifications: Vec<FailedModification>,
    pub integration_status: IntegrationStatus,
}

/// Information about a failed modification attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedModification {
    pub operation_id: String,
    pub failure_reason: String,
    pub rollback_status: RollbackStatus,
}

/// Status of modification integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationStatus {
    FullyIntegrated,
    PartiallyIntegrated,
    IntegrationPending,
    IntegrationFailed,
}

/// Status of rollback operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackStatus {
    NotRequired,
    Completed,
    InProgress,
    Failed,
}

/// Result of a cancellation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationResult {
    pub cancellation_id: String,
    pub cancelled_operations: Vec<String>,
    pub cleanup_status: CleanupStatus,
    pub preserved_results: Vec<String>,
}

/// Status of cleanup operations after cancellation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanupStatus {
    Complete,
    InProgress,
    Partial,
    Failed,
}

/// Plan for resuming operations after interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumptionPlan {
    pub plan_id: String,
    pub operations_to_resume: Vec<String>,
    pub resumption_order: Vec<ResumptionPhase>,
    pub resource_requirements: ResourceRequirements,
    pub safety_validations: Vec<SafetyCheck>,
    pub success_criteria: Vec<String>,
}

/// Phase in the resumption process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumptionPhase {
    pub phase_id: String,
    pub phase_description: String,
    pub operations_in_phase: Vec<String>,
    pub phase_dependencies: Vec<String>,
    pub estimated_duration: Duration,
}

/// Scope of operations a user is authorized to interrupt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionScope {
    pub authorized_components: Vec<ComponentType>,
    pub authorized_operation_types: Vec<String>,
    pub authorized_interruption_types: Vec<InterruptionType>,
    pub authority_limitations: Vec<AuthorityLimitation>,
}

/// Limitations on a user's interruption authority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityLimitation {
    pub limitation_type: String,
    pub limitation_description: String,
    pub affected_operations: Vec<String>,
}

/// Resource requirements for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_percentage: f64,
    pub memory_gb: f64,
    pub storage_gb: f64,
    pub network_mbps: f64,
    pub duration: Duration,
}

// =============================================================================
// ERROR TYPES FOR TASK INTERRUPTION
// =============================================================================

/// Comprehensive error types for task interruption operations
#[derive(Error, Debug)]
pub enum TaskInterruptionError {
    #[error("Authority validation failed: {user} lacks permission for {operation}")]
    InsufficientAuthority { user: String, operation: String },
    
    #[error("Operation discovery failed: {details}")]
    OperationDiscoveryFailed { details: String },
    
    #[error("Safe pause point not found: {operation} cannot be safely interrupted")]
    NoPausePointFound { operation: String },
    
    #[error("Interruption coordination failed: {component} - {details}")]
    CoordinationFailed { component: String, details: String },
    
    #[error("State preservation failed: {operation} - {details}")]
    StatePreservationFailed { operation: String, details: String },
    
    #[error("Modification integration failed: {modification} - {details}")]
    ModificationFailed { modification: String, details: String },
    
    #[error("Resumption validation failed: {operation} - {details}")]
    ResumptionFailed { operation: String, details: String },
    
    #[error("Safety requirements not met: {requirement} - {details}")]
    SafetyViolation { requirement: String, details: String },
    
    #[error("Interruption timeout: operation took longer than {timeout:?}")]
    InterruptionTimeout { timeout: Duration },
    
    #[error("Ecosystem communication error: {details}")]
    EcosystemCommunicationError { details: String },
    
    #[error("Resource allocation error: {resource} - {details}")]
    ResourceAllocationError { resource: String, details: String },
    
    #[error("Configuration error: {component} - {details}")]
    ConfigurationError { component: String, details: String },
    
    #[error("Security error: {operation} - {details}")]
    SecurityError { operation: String, details: String },
    
    #[error("Partial interruption failure: {successful} succeeded, {failed} failed")]
    PartialFailure { successful: usize, failed: usize },
}

// =============================================================================
// RESULT TYPE AND UTILITY MACROS
// =============================================================================

/// Result type for task interruption operations
pub type InterruptionResult<T> = Result<T, TaskInterruptionError>;

/// Macro for validating user authority before performing interruption operations
#[macro_export]
macro_rules! validate_interruption_authority {
    ($validator:expr, $user:expr, $operation:expr) => {
        match $validator.validate_operation_authority($user, $operation).await {
            Ok(true) => {},
            Ok(false) => return Err(TaskInterruptionError::InsufficientAuthority {
                user: $user.user_id.clone(),
                operation: $operation.to_string(),
            }),
            Err(e) => return Err(TaskInterruptionError::SecurityError {
                operation: "authority_validation".to_string(),
                details: e.to_string(),
            }),
        }
    };
}

/// Macro for handling interruption timeouts
#[macro_export]
macro_rules! with_interruption_timeout {
    ($timeout:expr, $operation:expr) => {
        match timeout($timeout, $operation).await {
            Ok(result) => result,
            Err(_) => return Err(TaskInterruptionError::InterruptionTimeout { 
                timeout: $timeout 
            }),
        }
    };
}

// =============================================================================
// CONSTANTS AND DEFAULTS
// =============================================================================

/// Default timeout for interruption operations
pub const DEFAULT_INTERRUPTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum number of operations that can be interrupted simultaneously
pub const MAX_SIMULTANEOUS_INTERRUPTIONS: usize = 100;

/// Default data loss tolerance for interruptions
pub const DEFAULT_DATA_LOSS_TOLERANCE: DataLossTolerance = DataLossTolerance::Minimal;

/// Default backup requirements for interruptions
pub const DEFAULT_BACKUP_REQUIREMENTS: BackupRequirements = BackupRequirements::StateOnly;

/// Maximum time to preserve interrupted operation state
pub const MAX_STATE_PRESERVATION_TIME: Duration = Duration::from_secs(3600); // 1 hour

/// Default priority for user-initiated interruptions
pub const DEFAULT_USER_INTERRUPTION_PRIORITY: InterruptionPriority = InterruptionPriority::High;
