//! # Methodology Protocols Module
//! 
//! This module defines the complete communication protocols for methodology execution,
//! validation, and coordination across the OZONE STUDIO ecosystem. These protocols
//! enable the revolutionary "Static Core + Dynamic Methodology" architecture where
//! AI Apps can load and execute sophisticated methodologies without modifying their
//! core coordination systems.
//!
//! ## Core Design Philosophy
//!
//! The methodology protocols embody several key principles:
//! 
//! 1. **Instruction-Based Coordination**: Methodologies contain systematic instruction
//!    sets that guide AI Apps through coordinated actions rather than executable code
//! 
//! 2. **Validation-Driven Execution**: Every methodology execution includes comprehensive
//!    validation checkpoints to ensure quality and strategic alignment
//! 
//! 3. **Context Preservation**: Execution context is maintained across all methodology
//!    phases to enable complex, multi-step coordinated processes
//! 
//! 4. **Progress Transparency**: Detailed progress tracking enables conscious oversight
//!    and human intervention when needed
//! 
//! 5. **Security by Design**: All methodology communications include security context
//!    and authorization validation

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::fmt;
use std::sync::Arc;

// Async and concurrency support
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::Instant;

// Serialization and data handling
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Import shared types from the parent crate
use crate::{ComponentType, ProtocolError};

// Import security context from shared-security crate
use shared_security::SecurityContext;

// ============================================================================
// Core Methodology Message Types
// ============================================================================

/// Primary message type for all methodology-related communications in the ecosystem.
/// This serves as the envelope for all methodology operations, whether they're
/// execution requests, progress updates, validation reports, or error notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyMessage {
    /// Unique identifier for this specific message, enabling request/response correlation
    pub message_id: String,
    
    /// The component that originated this message (e.g., OZONE_STUDIO, FORGE, ZSEI)
    pub sender: ComponentType,
    
    /// The intended recipient of this message
    pub recipient: ComponentType,
    
    /// Timestamp when this message was created (UTC)
    pub timestamp: SystemTime,
    
    /// The actual message content and type
    pub message_type: MethodologyMessageType,
    
    /// Security context for authentication and authorization
    pub security_context: Option<SecurityContext>,
    
    /// Optional correlation ID linking this message to a broader operation
    pub correlation_id: Option<String>,
    
    /// Message priority affecting processing order
    pub priority: MessagePriority,
    
    /// Optional timeout for response expectations
    pub response_timeout: Option<Duration>,
}

/// Enumeration of all possible methodology message types that can flow through the ecosystem.
/// Each variant represents a different aspect of methodology coordination and execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MethodologyMessageType {
    /// Request to execute a specific methodology
    ExecutionRequest(MethodologyExecutionRequest),
    
    /// Response to a methodology execution request
    ExecutionResponse(MethodologyExecutionResponse),
    
    /// Progress update during methodology execution
    ProgressUpdate(ExecutionProgress),
    
    /// Validation checkpoint report
    ValidationReport(ValidationResult),
    
    /// Request for methodology validation
    ValidationRequest(ValidationRequest),
    
    /// Instruction execution request to specific AI App
    InstructionExecution(InstructionExecutionRequest),
    
    /// Response from AI App instruction execution
    InstructionResponse(InstructionExecutionResponse),
    
    /// Request to pause ongoing methodology execution
    PauseRequest(ExecutionPauseRequest),
    
    /// Request to resume paused methodology execution
    ResumeRequest(ExecutionResumeRequest),
    
    /// Request to cancel methodology execution
    CancelRequest(ExecutionCancelRequest),
    
    /// Status query for ongoing methodology execution
    StatusQuery(ExecutionStatusQuery),
    
    /// Error report during methodology execution
    ErrorReport(MethodologyError),
    
    /// Request for methodology metadata
    MetadataRequest(MethodologyMetadataRequest),
    
    /// Response with methodology metadata
    MetadataResponse(MethodologyMetadataResponse),
    
    /// Request to validate methodology prerequisites
    PrerequisiteCheck(PrerequisiteCheckRequest),
    
    /// Response to prerequisite validation
    PrerequisiteResponse(PrerequisiteCheckResponse),
}

/// Message priority levels affecting processing order and resource allocation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Critical system operations (bootstrap, security, consciousness)
    Critical = 0,
    
    /// High priority operations (human requests, error recovery)
    High = 1,
    
    /// Normal priority operations (standard methodology execution)
    Normal = 2,
    
    /// Low priority operations (background optimization, cleanup)
    Low = 3,
    
    /// Background operations (analytics, non-critical maintenance)
    Background = 4,
}

// ============================================================================
// Methodology Execution Request and Response Types
// ============================================================================

/// Comprehensive request structure for executing a methodology within the ecosystem.
/// This type contains everything needed for an AI App to understand, validate, and
/// execute a methodology while maintaining coordination with the broader ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyExecutionRequest {
    /// Unique identifier for this execution instance
    pub execution_id: String,
    
    /// Identifier of the methodology to execute
    pub methodology_id: String,
    
    /// Name of the methodology for human-readable identification
    pub methodology_name: String,
    
    /// Version of the methodology to ensure compatibility
    pub methodology_version: String,
    
    /// The requesting component (typically OZONE_STUDIO)
    pub requester: ComponentType,
    
    /// The target AI App that should execute this methodology
    pub target_executor: ComponentType,
    
    /// Complete execution context and parameters
    pub execution_context: ExecutionContext,
    
    /// Execution configuration and preferences
    pub execution_config: ExecutionConfiguration,
    
    /// Security requirements and authorization context
    pub security_requirements: SecurityRequirements,
    
    /// Human guidance or constraints if provided
    pub human_guidance: Option<HumanGuidanceContext>,
    
    /// Expected deliverables and success criteria
    pub expected_outcomes: Vec<ExpectedOutcome>,
    
    /// Dependencies on other ongoing or completed executions
    pub dependencies: Vec<ExecutionDependency>,
    
    /// Resource allocation and constraints
    pub resource_constraints: ResourceConstraints,
    
    /// Quality assurance requirements
    pub quality_requirements: QualityRequirements,
    
    /// Timeout and deadline constraints
    pub timing_constraints: TimingConstraints,
}

/// Response structure for methodology execution requests, providing comprehensive
/// feedback about execution status, results, and any issues encountered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyExecutionResponse {
    /// Reference to the original execution request
    pub execution_id: String,
    
    /// The AI App that executed (or attempted to execute) the methodology
    pub executor: ComponentType,
    
    /// Current execution status
    pub execution_status: ExecutionStatus,
    
    /// Overall success/failure indication
    pub success: bool,
    
    /// Detailed execution results if successful
    pub execution_results: Option<ExecutionResults>,
    
    /// Any errors encountered during execution
    pub errors: Vec<MethodologyError>,
    
    /// Warnings or non-critical issues
    pub warnings: Vec<String>,
    
    /// Validation results from all checkpoints
    pub validation_results: Vec<ValidationResult>,
    
    /// Progress information and metrics
    pub progress_info: ExecutionProgress,
    
    /// Performance and efficiency metrics
    pub performance_metrics: PerformanceMetrics,
    
    /// Resource utilization information
    pub resource_utilization: ResourceUtilization,
    
    /// Learning insights and pattern recognition
    pub learning_insights: Vec<LearningInsight>,
    
    /// Recommendations for future improvements
    pub improvement_recommendations: Vec<String>,
    
    /// Relationship impacts and developments
    pub relationship_impacts: Vec<RelationshipImpact>,
    
    /// Timestamp when execution completed (or failed)
    pub completion_timestamp: SystemTime,
    
    /// Total execution duration
    pub execution_duration: Duration,
}

// ============================================================================
// Execution Context and Configuration Types
// ============================================================================

/// Comprehensive execution context that provides all necessary information for
/// methodology execution, including data, parameters, and coordination state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Context identifier for tracking and reference
    pub context_id: String,
    
    /// Input data and parameters for the methodology
    pub input_data: HashMap<String, serde_json::Value>,
    
    /// File paths and data sources accessible to the methodology
    pub data_sources: Vec<DataSource>,
    
    /// Environment variables and configuration
    pub environment: HashMap<String, String>,
    
    /// Current working directory for file operations
    pub working_directory: String,
    
    /// Temporary directory for intermediate files
    pub temp_directory: String,
    
    /// Output directory for results
    pub output_directory: String,
    
    /// State from previous execution phases
    pub previous_state: HashMap<String, serde_json::Value>,
    
    /// Context from related ongoing executions
    pub related_contexts: Vec<RelatedContext>,
    
    /// User preferences and customizations
    pub user_preferences: HashMap<String, serde_json::Value>,
    
    /// Historical context from previous similar executions
    pub historical_context: Option<HistoricalContext>,
    
    /// Ecosystem state relevant to this execution
    pub ecosystem_state: EcosystemState,
    
    /// Available AI App capabilities for coordination
    pub available_capabilities: HashMap<ComponentType, Vec<String>>,
}

/// Configuration parameters that control how a methodology is executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfiguration {
    /// Execution mode (sequential, parallel, hybrid)
    pub execution_mode: ExecutionMode,
    
    /// Parallelism settings for concurrent operations
    pub parallelism_config: ParallelismConfig,
    
    /// Validation strictness level
    pub validation_level: ValidationLevel,
    
    /// Error handling strategy
    pub error_handling: ErrorHandlingStrategy,
    
    /// Progress reporting frequency
    pub progress_reporting: ProgressReportingConfig,
    
    /// Optimization preferences
    pub optimization_preferences: OptimizationPreferences,
    
    /// Debug and logging configuration
    pub debug_config: DebugConfiguration,
    
    /// Backup and recovery settings
    pub backup_config: BackupConfiguration,
    
    /// Performance monitoring settings
    pub monitoring_config: MonitoringConfiguration,
}

/// Execution mode determining how methodology instructions are processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Execute instructions sequentially, one after another
    Sequential,
    
    /// Execute compatible instructions in parallel where possible
    Parallel {
        max_concurrency: Option<usize>,
        load_balancing: LoadBalancingStrategy,
    },
    
    /// Hybrid approach adapting based on instruction characteristics
    Adaptive {
        adaptation_strategy: AdaptationStrategy,
        performance_monitoring: bool,
    },
    
    /// Interactive mode requiring human guidance
    Interactive {
        approval_required: bool,
        guidance_timeout: Option<Duration>,
    },
}

/// Load balancing strategies for parallel execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    CapacityBased,
    LatencyOptimized,
    ResourceOptimized,
}

/// Adaptation strategies for adaptive execution mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdaptationStrategy {
    PerformanceBased,
    ResourceBased,
    ComplexityBased,
    HistoricalPattern,
}

/// Parallelism configuration for concurrent methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelismConfig {
    /// Enable parallel execution where possible
    pub enabled: bool,
    
    /// Maximum number of concurrent operations
    pub max_concurrent_operations: usize,
    
    /// Maximum number of concurrent AI App coordinations
    pub max_concurrent_coordinations: usize,
    
    /// Thread pool size for CPU-intensive operations
    pub thread_pool_size: usize,
    
    /// Synchronization strategy for parallel operations
    pub synchronization_strategy: SynchronizationStrategy,
    
    /// Conflict resolution for concurrent resource access
    pub conflict_resolution: ConflictResolutionStrategy,
}

/// Synchronization strategies for parallel operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SynchronizationStrategy {
    WaitForAll,
    WaitForAny,
    WaitForMajority,
    ContinueOnFirst,
    TimeboxedWait(Duration),
}

/// Conflict resolution strategies for resource contention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    FirstComeFirstServed,
    PriorityBased,
    RandomSelection,
    LoadBalanced,
    UserDecision,
}

// ============================================================================
// Validation Types and Structures
// ============================================================================

/// Validation level determining thoroughness of execution validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationLevel {
    /// Minimal validation, focus on speed
    Minimal,
    
    /// Standard validation with essential checks
    Standard,
    
    /// Comprehensive validation with detailed analysis
    Comprehensive,
    
    /// Exhaustive validation with extensive quality checks
    Exhaustive,
    
    /// Custom validation with specific requirements
    Custom {
        requirements: Vec<ValidationRequirement>,
        strictness: ValidationStrictness,
    },
}

/// Validation strictness levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationStrictness {
    /// Continue execution despite minor validation failures
    Lenient,
    
    /// Pause execution on significant validation failures
    Moderate,
    
    /// Stop execution on any validation failure
    Strict,
    
    /// Require human approval for validation failures
    HumanOversight,
}

/// A validation checkpoint represents a point in methodology execution where
/// quality, correctness, and compliance are verified before proceeding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheckpoint {
    /// Unique identifier for this checkpoint
    pub checkpoint_id: String,
    
    /// Human-readable name for this checkpoint
    pub checkpoint_name: String,
    
    /// Description of what this checkpoint validates
    pub description: String,
    
    /// The phase of execution this checkpoint occurs in
    pub execution_phase: String,
    
    /// Validation criteria that must be met
    pub validation_criteria: Vec<ValidationCriterion>,
    
    /// Actions to take if validation fails
    pub failure_actions: Vec<FailureAction>,
    
    /// Whether this checkpoint blocks execution on failure
    pub blocking: bool,
    
    /// Timeout for validation operations
    pub validation_timeout: Option<Duration>,
    
    /// Dependencies on other checkpoints
    pub dependencies: Vec<String>,
    
    /// Human intervention options
    pub human_intervention: HumanInterventionOptions,
}

/// Individual validation criterion within a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCriterion {
    /// Unique identifier for this criterion
    pub criterion_id: String,
    
    /// Human-readable description
    pub description: String,
    
    /// Validation method or algorithm to use
    pub validation_method: ValidationMethod,
    
    /// Threshold or target value for validation
    pub threshold: ValidationThreshold,
    
    /// Weight of this criterion in overall validation
    pub weight: f64,
    
    /// Whether this criterion is required or optional
    pub required: bool,
    
    /// Custom parameters for validation method
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Methods for performing validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationMethod {
    /// Semantic analysis of output content
    SemanticAnalysis,
    
    /// Structural analysis of code or documents
    StructuralAnalysis,
    
    /// Quality metrics evaluation
    QualityMetrics,
    
    /// Performance benchmark comparison
    PerformanceBenchmark,
    
    /// Human review and approval
    HumanReview,
    
    /// Automated test execution
    AutomatedTesting,
    
    /// Compliance checking against standards
    ComplianceCheck,
    
    /// Cross-reference validation with external sources
    CrossReference,
    
    /// Custom validation logic
    Custom {
        validator_name: String,
        validator_config: HashMap<String, serde_json::Value>,
    },
}

/// Threshold types for validation criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationThreshold {
    /// Numeric threshold (greater than, less than, equal to)
    Numeric {
        value: f64,
        comparison: ComparisonOperator,
    },
    
    /// Boolean requirement (must be true or false)
    Boolean { expected: bool },
    
    /// String pattern matching
    Pattern { regex: String },
    
    /// Enumerated acceptable values
    Enumerated { acceptable_values: Vec<String> },
    
    /// Range validation (minimum and maximum)
    Range { min: f64, max: f64 },
    
    /// Custom threshold logic
    Custom {
        threshold_type: String,
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Comparison operators for numeric thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

/// Result of validation checkpoint execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Reference to the checkpoint that was validated
    pub checkpoint_id: String,
    
    /// Overall validation success/failure
    pub passed: bool,
    
    /// Overall validation score (0.0 to 1.0)
    pub score: f64,
    
    /// Results for individual criteria
    pub criterion_results: Vec<CriterionResult>,
    
    /// Detailed validation report
    pub validation_report: String,
    
    /// Any issues or warnings identified
    pub issues: Vec<ValidationIssue>,
    
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
    
    /// Timestamp when validation was performed
    pub validation_timestamp: SystemTime,
    
    /// Duration of validation process
    pub validation_duration: Duration,
    
    /// Human review results if applicable
    pub human_review: Option<HumanReviewResult>,
}

/// Result for individual validation criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResult {
    /// Reference to the criterion
    pub criterion_id: String,
    
    /// Whether this criterion passed
    pub passed: bool,
    
    /// Actual measured value
    pub measured_value: serde_json::Value,
    
    /// Expected value or threshold
    pub expected_value: serde_json::Value,
    
    /// Detailed explanation of result
    pub explanation: String,
    
    /// Confidence level in the result
    pub confidence: f64,
}

/// Validation issues identified during checkpoint execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Severity level of the issue
    pub severity: IssueSeverity,
    
    /// Issue category
    pub category: String,
    
    /// Detailed description of the issue
    pub description: String,
    
    /// Location where issue was found
    pub location: Option<String>,
    
    /// Suggested resolution
    pub suggested_resolution: Option<String>,
    
    /// Impact assessment
    pub impact: ImpactAssessment,
}

/// Severity levels for validation issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
    Blocking,
}

/// Impact assessment for validation issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    /// Impact on execution quality
    pub quality_impact: ImpactLevel,
    
    /// Impact on execution performance
    pub performance_impact: ImpactLevel,
    
    /// Impact on security
    pub security_impact: ImpactLevel,
    
    /// Impact on user experience
    pub user_experience_impact: ImpactLevel,
    
    /// Impact on future executions
    pub future_impact: ImpactLevel,
}

/// Impact levels for different aspects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    None,
    Minimal,
    Moderate,
    Significant,
    Severe,
}

// ============================================================================
// Instruction Types and Execution
// ============================================================================

/// A methodology instruction represents a single coordinated action that an AI App
/// should perform as part of methodology execution. Instructions are the fundamental
/// building blocks of our methodology framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyInstruction {
    /// Unique identifier for this instruction
    pub instruction_id: String,
    
    /// Human-readable name for this instruction
    pub instruction_name: String,
    
    /// Detailed description of what this instruction accomplishes
    pub description: String,
    
    /// The type and parameters of this instruction
    pub instruction_type: InstructionType,
    
    /// Prerequisites that must be met before execution
    pub prerequisites: Vec<Prerequisite>,
    
    /// Expected outcomes from this instruction
    pub expected_outcomes: Vec<ExpectedOutcome>,
    
    /// Validation criteria for instruction success
    pub validation_criteria: Vec<ValidationCriterion>,
    
    /// Error handling strategy for this instruction
    pub error_handling: InstructionErrorHandling,
    
    /// Timeout for instruction execution
    pub timeout: Option<Duration>,
    
    /// Retry policy for failed instructions
    pub retry_policy: RetryPolicy,
    
    /// Dependencies on other instructions
    pub dependencies: Vec<InstructionDependency>,
    
    /// Human intervention points
    pub human_intervention_points: Vec<HumanInterventionPoint>,
}

/// Types of instructions that can be executed within methodologies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstructionType {
    /// Coordinate with another AI App to perform an operation
    CoordinateWithApp {
        /// Target AI App for coordination
        target_app: ComponentType,
        
        /// Operation to request from the target app
        operation: String,
        
        /// Parameters for the operation
        parameters: HashMap<String, serde_json::Value>,
        
        /// Expected response schema
        expected_response: ResponseSchema,
        
        /// Coordination timeout
        timeout: Option<Duration>,
        
        /// Retry policy for coordination failures
        retry_policy: Option<RetryPolicy>,
    },
    
    /// Coordinate with NEXUS for file system operations
    CoordinateWithNexus {
        /// File system operation to perform
        operation: NexusOperation,
        
        /// Parameters for the operation
        parameters: HashMap<String, serde_json::Value>,
        
        /// File operations to perform
        file_operations: Vec<FileOperation>,
        
        /// Safety requirements for the operation
        safety_requirements: SafetyRequirements,
    },
    
    /// Execute multiple instructions in parallel
    ExecuteParallel {
        /// Instructions to execute concurrently
        operations: Vec<MethodologyInstruction>,
        
        /// Synchronization point for completion
        synchronization_point: SynchronizationPoint,
        
        /// Maximum number of concurrent operations
        max_concurrency: Option<usize>,
        
        /// Failure handling policy
        failure_policy: FailurePolicy,
    },
    
    /// Execute multiple instructions sequentially
    ExecuteSequential {
        /// Instructions to execute in order
        operations: Vec<MethodologyInstruction>,
        
        /// Checkpoint requirements between operations
        checkpoint_requirements: Vec<String>,
        
        /// Rollback strategy if later operations fail
        rollback_strategy: Option<RollbackStrategy>,
    },
    
    /// Execute instructions in a loop until condition is met
    ExecuteLoop {
        /// Loop condition to evaluate
        condition: LoopCondition,
        
        /// Instructions to execute in each iteration
        instructions: Vec<MethodologyInstruction>,
        
        /// Maximum number of iterations
        max_iterations: Option<u32>,
        
        /// Conditions that can break the loop early
        break_conditions: Vec<BreakCondition>,
    },
    
    /// Import and execute another methodology
    ImportMethodology {
        /// Methodology identifier to import
        methodology_id: String,
        
        /// Parameters to pass to the imported methodology
        parameters: HashMap<String, serde_json::Value>,
        
        /// Integration mode for the imported methodology
        integration_mode: IntegrationMode,
    },
    
    /// Validate a checkpoint before proceeding
    ValidateCheckpoint {
        /// Checkpoint identifier to validate
        checkpoint_id: String,
        
        /// Validation criteria to apply
        validation_criteria: Vec<ValidationCriterion>,
        
        /// Actions to take if validation fails
        failure_actions: Vec<FailureAction>,
    },
    
    /// Create a .zsei directory for intelligent storage
    CreateZSEIDirectory {
        /// Storage context for the directory
        context: StorageContext,
        
        /// Directory structure to create
        structure: DirectoryStructure,
        
        /// Metadata to include
        metadata: HashMap<String, serde_json::Value>,
    },
    
    /// Wait for a specific condition or timeout
    Wait {
        /// Condition to wait for
        condition: WaitCondition,
        
        /// Maximum time to wait
        timeout: Duration,
        
        /// Polling interval for condition checking
        poll_interval: Option<Duration>,
    },
    
    /// Notify other components about progress or events
    Notify {
        /// Recipients of the notification
        recipients: Vec<ComponentType>,
        
        /// Notification type and content
        notification: NotificationContent,
        
        /// Whether to wait for acknowledgment
        wait_for_acknowledgment: bool,
    },
    
    /// Custom instruction with specific implementation
    Custom {
        /// Custom instruction type identifier
        instruction_type: String,
        
        /// Custom parameters
        parameters: HashMap<String, serde_json::Value>,
        
        /// Custom validation logic
        validation: Option<CustomValidation>,
    },
}

/// An instruction set groups related instructions and defines their execution order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionSet {
    /// Unique identifier for this instruction set
    pub set_id: String,
    
    /// Human-readable name for this instruction set
    pub name: String,
    
    /// Description of what this instruction set accomplishes
    pub description: String,
    
    /// Instructions in this set
    pub instructions: Vec<MethodologyInstruction>,
    
    /// Prerequisites for executing this instruction set
    pub prerequisites: Vec<Prerequisite>,
    
    /// Expected outcomes from this instruction set
    pub expected_outcomes: Vec<ExpectedOutcome>,
    
    /// Validation criteria for the entire instruction set
    pub validation_criteria: Vec<ValidationCriterion>,
    
    /// Execution strategy for the instructions
    pub execution_strategy: InstructionSetExecutionStrategy,
    
    /// Error handling for the instruction set
    pub error_handling: InstructionSetErrorHandling,
    
    /// Resource requirements for execution
    pub resource_requirements: ResourceRequirements,
    
    /// Quality assurance for the instruction set
    pub quality_assurance: QualityAssuranceConfig,
}

/// Execution strategies for instruction sets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstructionSetExecutionStrategy {
    /// Execute all instructions sequentially
    Sequential,
    
    /// Execute instructions in parallel where possible
    Parallel {
        dependency_resolution: DependencyResolutionStrategy,
        resource_allocation: ResourceAllocationStrategy,
    },
    
    /// Execute based on dynamic conditions
    Conditional {
        conditions: Vec<ExecutionCondition>,
        fallback_strategy: Box<InstructionSetExecutionStrategy>,
    },
    
    /// Execute with human guidance
    Guided {
        guidance_points: Vec<GuidancePoint>,
        automation_level: AutomationLevel,
    },
}

// ============================================================================
// Progress Tracking and Execution Status
// ============================================================================

/// Comprehensive progress information for methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    /// Reference to the execution being tracked
    pub execution_id: String,
    
    /// Overall execution status
    pub status: ExecutionStatus,
    
    /// Current phase of execution
    pub current_phase: String,
    
    /// Completed phases
    pub completed_phases: Vec<PhaseProgress>,
    
    /// Pending phases
    pub pending_phases: Vec<String>,
    
    /// Overall progress percentage (0.0 to 1.0)
    pub progress_percentage: f64,
    
    /// Estimated time to completion
    pub estimated_completion: Option<SystemTime>,
    
    /// Elapsed execution time
    pub elapsed_time: Duration,
    
    /// Current instruction being executed
    pub current_instruction: Option<String>,
    
    /// Completed instructions count
    pub completed_instructions: u32,
    
    /// Total instructions count
    pub total_instructions: u32,
    
    /// Instructions in progress
    pub in_progress_instructions: Vec<InstructionProgress>,
    
    /// Failed instructions
    pub failed_instructions: Vec<FailedInstruction>,
    
    /// Validation checkpoints passed
    pub validation_checkpoints_passed: u32,
    
    /// Total validation checkpoints
    pub total_validation_checkpoints: u32,
    
    /// Current resource utilization
    pub resource_utilization: ResourceUtilization,
    
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    
    /// Quality indicators
    pub quality_indicators: QualityIndicators,
    
    /// Human interaction requirements
    pub human_interaction_needed: Vec<HumanInteractionRequest>,
    
    /// Warnings and issues
    pub warnings: Vec<String>,
    
    /// Milestone achievements
    pub milestones: Vec<MilestoneAchievement>,
}

/// Execution status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    /// Execution has not yet started
    NotStarted,
    
    /// Execution is currently in progress
    InProgress,
    
    /// Execution is paused (can be resumed)
    Paused,
    
    /// Execution is waiting for human input
    WaitingForHumanInput,
    
    /// Execution is waiting for external dependencies
    WaitingForDependencies,
    
    /// Execution completed successfully
    Completed,
    
    /// Execution failed with errors
    Failed,
    
    /// Execution was cancelled by user or system
    Cancelled,
    
    /// Execution is in validation phase
    Validating,
    
    /// Execution is in rollback phase due to failure
    RollingBack,
    
    /// Execution requires manual intervention
    RequiresIntervention,
}

/// Progress information for individual execution phases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseProgress {
    /// Phase identifier
    pub phase_id: String,
    
    /// Phase name
    pub phase_name: String,
    
    /// Phase completion percentage
    pub completion_percentage: f64,
    
    /// Time when phase started
    pub start_time: SystemTime,
    
    /// Time when phase completed (if completed)
    pub completion_time: Option<SystemTime>,
    
    /// Duration of phase execution
    pub duration: Duration,
    
    /// Outcomes achieved in this phase
    pub outcomes: Vec<String>,
    
    /// Issues encountered in this phase
    pub issues: Vec<String>,
    
    /// Validation results for this phase
    pub validation_results: Vec<ValidationResult>,
}

/// Progress information for individual instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionProgress {
    /// Instruction identifier
    pub instruction_id: String,
    
    /// Instruction name
    pub instruction_name: String,
    
    /// Progress percentage for this instruction
    pub progress_percentage: f64,
    
    /// Current step within the instruction
    pub current_step: String,
    
    /// Estimated completion time
    pub estimated_completion: Option<SystemTime>,
    
    /// AI Apps involved in executing this instruction
    pub coordinating_apps: Vec<ComponentType>,
    
    /// Intermediate results
    pub intermediate_results: HashMap<String, serde_json::Value>,
}

/// Information about failed instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedInstruction {
    /// Instruction identifier
    pub instruction_id: String,
    
    /// Instruction name
    pub instruction_name: String,
    
    /// Error that caused the failure
    pub error: MethodologyError,
    
    /// Retry attempts made
    pub retry_attempts: u32,
    
    /// Whether the failure was recoverable
    pub recoverable: bool,
    
    /// Recovery action taken (if any)
    pub recovery_action: Option<String>,
    
    /// Impact of the failure on overall execution
    pub impact_assessment: ImpactAssessment,
}

// ============================================================================
// Error Types and Handling
// ============================================================================

/// Comprehensive error type for methodology execution and coordination
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum MethodologyError {
    #[error("Execution error in {phase}: {details}")]
    ExecutionError {
        phase: String,
        details: String,
        instruction_id: Option<String>,
        error_code: String,
        recovery_suggestions: Vec<String>,
    },
    
    #[error("Validation failed at checkpoint {checkpoint}: {reason}")]
    ValidationFailed {
        checkpoint: String,
        reason: String,
        failed_criteria: Vec<String>,
        validation_score: f64,
        recovery_possible: bool,
    },
    
    #[error("Coordination error with {component}: {details}")]
    CoordinationError {
        component: ComponentType,
        details: String,
        operation: String,
        retry_count: u32,
        timeout_exceeded: bool,
    },
    
    #[error("Instruction interpretation error: {instruction} - {details}")]
    InstructionError {
        instruction: String,
        instruction_id: String,
        details: String,
        parameter_issues: Vec<String>,
        suggestion: Option<String>,
    },
    
    #[error("Resource allocation error: {resource} - {details}")]
    ResourceError {
        resource: String,
        details: String,
        required_amount: String,
        available_amount: String,
        allocation_strategy: String,
    },
    
    #[error("Security violation: {operation} - {details}")]
    SecurityViolation {
        operation: String,
        details: String,
        security_context: String,
        violation_type: String,
        remediation_required: bool,
    },
    
    #[error("Dependency error: {dependency} - {details}")]
    DependencyError {
        dependency: String,
        dependency_type: DependencyType,
        details: String,
        circular_dependency: bool,
        resolution_strategy: Option<String>,
    },
    
    #[error("Timeout error: {operation} exceeded {timeout:?}")]
    TimeoutError {
        operation: String,
        timeout: Duration,
        elapsed_time: Duration,
        partial_results: bool,
        recovery_action: Option<String>,
    },
    
    #[error("Configuration error: {component} - {details}")]
    ConfigurationError {
        component: String,
        details: String,
        configuration_path: String,
        expected_format: String,
        correction_suggestion: Option<String>,
    },
    
    #[error("Human intervention required: {reason}")]
    HumanInterventionRequired {
        reason: String,
        intervention_type: InterventionType,
        context: String,
        options: Vec<String>,
        urgency: InterventionUrgency,
    },
    
    #[error("System constraint violation: {constraint} - {details}")]
    SystemConstraintViolation {
        constraint: String,
        details: String,
        current_value: String,
        limit_value: String,
        mitigation_options: Vec<String>,
    },
}

/// Types of dependencies that can fail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Methodology,
    Instruction,
    Resource,
    AIApp,
    ExternalService,
    FileSystem,
    Network,
}

/// Types of human intervention required
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionType {
    Approval,
    Decision,
    Input,
    Review,
    Clarification,
    Authorization,
    ErrorResolution,
}

/// Urgency levels for human intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionUrgency {
    Low,
    Medium,
    High,
    Critical,
    Immediate,
}

// ============================================================================
// Supporting Types and Structures
// ============================================================================

/// Expected outcome specification for instructions and methodologies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedOutcome {
    /// Unique identifier for this outcome
    pub outcome_id: String,
    
    /// Description of the expected outcome
    pub description: String,
    
    /// Type of outcome expected
    pub outcome_type: OutcomeType,
    
    /// Success criteria for this outcome
    pub success_criteria: Vec<SuccessCriterion>,
    
    /// Quality metrics for this outcome
    pub quality_metrics: Vec<QualityMetric>,
    
    /// Priority of this outcome
    pub priority: OutcomePriority,
    
    /// Whether this outcome is required or optional
    pub required: bool,
}

/// Types of outcomes that can be expected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutcomeType {
    FileCreation,
    DataProcessing,
    AnalysisReport,
    CodeGeneration,
    DocumentCreation,
    QualityImprovement,
    PerformanceOptimization,
    SecurityEnhancement,
    UserExperience,
    LearningAchievement,
    RelationshipDevelopment,
}

/// Priority levels for outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutcomePriority {
    Critical,
    High,
    Medium,
    Low,
    Optional,
}

/// Success criteria for outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriterion {
    /// Criterion identifier
    pub criterion_id: String,
    
    /// Description of success condition
    pub description: String,
    
    /// Measurement method
    pub measurement_method: String,
    
    /// Target value or threshold
    pub target_value: serde_json::Value,
    
    /// Tolerance for measurement variation
    pub tolerance: Option<f64>,
    
    /// Weight in overall success calculation
    pub weight: f64,
}

/// Quality metrics for outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetric {
    /// Metric identifier
    pub metric_id: String,
    
    /// Metric name
    pub metric_name: String,
    
    /// Measurement method
    pub measurement_method: String,
    
    /// Target value
    pub target_value: f64,
    
    /// Minimum acceptable value
    pub minimum_value: f64,
    
    /// Weight in quality calculation
    pub weight: f64,
}

/// Resource constraints for methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// Maximum CPU usage percentage
    pub max_cpu_usage: Option<f64>,
    
    /// Maximum memory usage in bytes
    pub max_memory_usage: Option<u64>,
    
    /// Maximum storage usage in bytes
    pub max_storage_usage: Option<u64>,
    
    /// Maximum network bandwidth usage
    pub max_network_usage: Option<u64>,
    
    /// Maximum execution time
    pub max_execution_time: Option<Duration>,
    
    /// Maximum number of AI App coordinations
    pub max_ai_app_coordinations: Option<u32>,
    
    /// Maximum number of parallel operations
    pub max_parallel_operations: Option<u32>,
    
    /// Priority level for resource allocation
    pub priority_level: ResourcePriority,
    
    /// Whether to enforce strict resource limits
    pub strict_enforcement: bool,
}

/// Resource priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourcePriority {
    System,
    High,
    Normal,
    Low,
    Background,
}

/// Performance metrics collected during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total execution time
    pub total_execution_time: Duration,
    
    /// Time spent in coordination
    pub coordination_time: Duration,
    
    /// Time spent in validation
    pub validation_time: Duration,
    
    /// Time spent waiting for resources
    pub resource_wait_time: Duration,
    
    /// Number of AI App coordinations
    pub coordination_count: u32,
    
    /// Average coordination response time
    pub average_coordination_time: Duration,
    
    /// Peak resource utilization
    pub peak_resource_utilization: ResourceUtilization,
    
    /// Efficiency score (0.0 to 1.0)
    pub efficiency_score: f64,
    
    /// Throughput metrics
    pub throughput_metrics: ThroughputMetrics,
    
    /// Error rate
    pub error_rate: f64,
    
    /// Retry count
    pub retry_count: u32,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// CPU utilization percentage
    pub cpu_usage: f64,
    
    /// Memory utilization in bytes
    pub memory_usage: u64,
    
    /// Storage utilization in bytes
    pub storage_usage: u64,
    
    /// Network utilization in bytes per second
    pub network_usage: u64,
    
    /// Number of active coordinations
    pub active_coordinations: u32,
    
    /// Number of queued operations
    pub queued_operations: u32,
}

/// Throughput metrics for performance analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// Instructions processed per second
    pub instructions_per_second: f64,
    
    /// Validations completed per second
    pub validations_per_second: f64,
    
    /// Coordinations completed per second
    pub coordinations_per_second: f64,
    
    /// Bytes processed per second
    pub bytes_per_second: u64,
    
    /// Files processed per second
    pub files_per_second: f64,
}

/// Quality indicators for execution assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIndicators {
    /// Overall quality score (0.0 to 1.0)
    pub overall_quality_score: f64,
    
    /// Validation success rate
    pub validation_success_rate: f64,
    
    /// Instruction success rate
    pub instruction_success_rate: f64,
    
    /// Coordination success rate
    pub coordination_success_rate: f64,
    
    /// Error recovery rate
    pub error_recovery_rate: f64,
    
    /// Quality improvement indicators
    pub quality_improvements: Vec<QualityImprovement>,
    
    /// Quality degradation indicators
    pub quality_issues: Vec<QualityIssue>,
}

/// Quality improvement tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityImprovement {
    /// Area of improvement
    pub area: String,
    
    /// Improvement description
    pub description: String,
    
    /// Quantified improvement
    pub improvement_value: f64,
    
    /// Measurement unit
    pub unit: String,
}

/// Quality issue tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    /// Issue category
    pub category: String,
    
    /// Issue description
    pub description: String,
    
    /// Severity level
    pub severity: IssueSeverity,
    
    /// Impact on overall quality
    pub quality_impact: f64,
    
    /// Suggested resolution
    pub resolution: Option<String>,
}

// ============================================================================
// Additional Supporting Types
// ============================================================================

/// Human guidance context for methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanGuidanceContext {
    /// Type of guidance provided
    pub guidance_type: HumanGuidanceType,
    
    /// Guidance content
    pub content: String,
    
    /// Authority level of the guidance
    pub authority_level: AuthorityLevel,
    
    /// Context and constraints
    pub context: HashMap<String, serde_json::Value>,
    
    /// Timestamp when guidance was provided
    pub timestamp: SystemTime,
    
    /// User identifier who provided guidance
    pub user_id: String,
    
    /// Expiration time for the guidance
    pub expires_at: Option<SystemTime>,
}

/// Types of human guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HumanGuidanceType {
    Override,
    Preference,
    Constraint,
    Goal,
    Priority,
    Strategy,
    Quality,
    Timeline,
}

/// Authority levels for human guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorityLevel {
    Suggestion,
    Preference,
    Guidance,
    Requirement,
    Mandate,
}

/// Learning insights generated during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsight {
    /// Insight category
    pub category: String,
    
    /// Insight description
    pub description: String,
    
    /// Confidence level in the insight
    pub confidence: f64,
    
    /// Evidence supporting the insight
    pub evidence: Vec<String>,
    
    /// Applicability to future executions
    pub applicability: ApplicabilityScope,
    
    /// Recommended actions based on insight
    pub recommendations: Vec<String>,
}

/// Scope of applicability for learning insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApplicabilityScope {
    ThisMethodology,
    SimilarMethodologies,
    AllMethodologies,
    SpecificDomain,
    CrossDomain,
}

/// Relationship impacts from methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipImpact {
    /// Type of relationship affected
    pub relationship_type: RelationshipType,
    
    /// Entities involved in the relationship
    pub entities: Vec<String>,
    
    /// Nature of the impact
    pub impact_type: RelationshipImpactType,
    
    /// Magnitude of the impact
    pub impact_magnitude: f64,
    
    /// Description of the impact
    pub description: String,
    
    /// Long-term implications
    pub long_term_implications: Vec<String>,
}

/// Types of relationships that can be impacted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    HumanAI,
    AIAppCoordination,
    UserSystem,
    ComponentComponent,
    DataRelationship,
    ProcessRelationship,
}

/// Types of relationship impacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipImpactType {
    Strengthened,
    Weakened,
    Established,
    Terminated,
    Modified,
    Clarified,
}

// ============================================================================
// Request and Response Types for Specific Operations
// ============================================================================

/// Request for instruction execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionExecutionRequest {
    /// Unique identifier for this execution request
    pub request_id: String,
    
    /// The instruction to execute
    pub instruction: MethodologyInstruction,
    
    /// Execution context
    pub context: ExecutionContext,
    
    /// Security requirements
    pub security_context: SecurityContext,
    
    /// Timeout for execution
    pub timeout: Option<Duration>,
    
    /// Priority of this instruction
    pub priority: MessagePriority,
}

/// Response from instruction execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionExecutionResponse {
    /// Reference to the request
    pub request_id: String,
    
    /// Success/failure indication
    pub success: bool,
    
    /// Execution results
    pub results: HashMap<String, serde_json::Value>,
    
    /// Validation results
    pub validation_results: Vec<ValidationResult>,
    
    /// Any errors encountered
    pub errors: Vec<MethodologyError>,
    
    /// Execution metrics
    pub metrics: PerformanceMetrics,
    
    /// Learning insights
    pub insights: Vec<LearningInsight>,
}

/// Request to pause methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPauseRequest {
    /// Execution to pause
    pub execution_id: String,
    
    /// Reason for pausing
    pub reason: String,
    
    /// Whether to wait for safe pause point
    pub wait_for_safe_point: bool,
    
    /// Timeout for pause operation
    pub timeout: Option<Duration>,
}

/// Request to resume methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResumeRequest {
    /// Execution to resume
    pub execution_id: String,
    
    /// Any modifications to apply
    pub modifications: Option<ExecutionModifications>,
    
    /// Whether to validate state before resuming
    pub validate_before_resume: bool,
}

/// Request to cancel methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCancelRequest {
    /// Execution to cancel
    pub execution_id: String,
    
    /// Reason for cancellation
    pub reason: String,
    
    /// Whether to perform cleanup
    pub cleanup: bool,
    
    /// Whether to preserve partial results
    pub preserve_results: bool,
}

/// Query for execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatusQuery {
    /// Execution to query
    pub execution_id: String,
    
    /// Level of detail requested
    pub detail_level: StatusDetailLevel,
    
    /// Whether to include performance metrics
    pub include_metrics: bool,
    
    /// Whether to include validation results
    pub include_validation: bool,
}

/// Detail levels for status queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusDetailLevel {
    Basic,
    Standard,
    Detailed,
    Comprehensive,
}

/// Modifications that can be applied to ongoing execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionModifications {
    /// Updated execution configuration
    pub config_updates: Option<ExecutionConfiguration>,
    
    /// Updated resource constraints
    pub resource_updates: Option<ResourceConstraints>,
    
    /// Updated human guidance
    pub guidance_updates: Option<HumanGuidanceContext>,
    
    /// Additional validation requirements
    pub validation_updates: Option<Vec<ValidationCheckpoint>>,
    
    /// Priority changes
    pub priority_updates: Option<MessagePriority>,
}

// ============================================================================
// Protocol Implementation Helpers
// ============================================================================

impl MethodologyMessage {
    /// Create a new methodology message
    pub fn new(
        sender: ComponentType,
        recipient: ComponentType,
        message_type: MethodologyMessageType,
        priority: MessagePriority,
    ) -> Self {
        Self {
            message_id: Uuid::new_v4().to_string(),
            sender,
            recipient,
            timestamp: SystemTime::now(),
            message_type,
            security_context: None,
            correlation_id: None,
            priority,
            response_timeout: None,
        }
    }
    
    /// Set security context for the message
    pub fn with_security_context(mut self, context: SecurityContext) -> Self {
        self.security_context = Some(context);
        self
    }
    
    /// Set correlation ID for request/response tracking
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
    
    /// Set response timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.response_timeout = Some(timeout);
        self
    }
}

impl ExecutionProgress {
    /// Calculate overall progress percentage
    pub fn calculate_progress(&self) -> f64 {
        if self.total_instructions == 0 {
            return 0.0;
        }
        
        let base_progress = self.completed_instructions as f64 / self.total_instructions as f64;
        
        // Add partial progress from in-progress instructions
        let partial_progress: f64 = self.in_progress_instructions
            .iter()
            .map(|ip| ip.progress_percentage / 100.0)
            .sum();
        
        let total_progress = (base_progress + partial_progress / self.total_instructions as f64).min(1.0);
        
        (total_progress * 100.0).round() / 100.0
    }
    
    /// Check if execution is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.status, ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled)
    }
    
    /// Check if human intervention is needed
    pub fn needs_human_intervention(&self) -> bool {
        !self.human_interaction_needed.is_empty() || 
        matches!(self.status, ExecutionStatus::WaitingForHumanInput | ExecutionStatus::RequiresIntervention)
    }
}

impl ValidationResult {
    /// Check if validation passed with minimum score
    pub fn passed_with_score(&self, minimum_score: f64) -> bool {
        self.passed && self.score >= minimum_score
    }
    
    /// Get critical issues only
    pub fn critical_issues(&self) -> Vec<&ValidationIssue> {
        self.issues.iter()
            .filter(|issue| matches!(issue.severity, IssueSeverity::Critical | IssueSeverity::Blocking))
            .collect()
    }
}

impl MethodologyError {
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            MethodologyError::ValidationFailed { recovery_possible, .. } => *recovery_possible,
            MethodologyError::CoordinationError { retry_count, .. } => *retry_count < 3,
            MethodologyError::TimeoutError { recovery_action, .. } => recovery_action.is_some(),
            MethodologyError::SecurityViolation { remediation_required, .. } => !*remediation_required,
            MethodologyError::SystemConstraintViolation { mitigation_options, .. } => !mitigation_options.is_empty(),
            _ => true,
        }
    }
    
    /// Get error severity level
    pub fn severity(&self) -> IssueSeverity {
        match self {
            MethodologyError::SecurityViolation { .. } => IssueSeverity::Critical,
            MethodologyError::HumanInterventionRequired { urgency, .. } => {
                match urgency {
                    InterventionUrgency::Critical | InterventionUrgency::Immediate => IssueSeverity::Critical,
                    InterventionUrgency::High => IssueSeverity::Error,
                    InterventionUrgency::Medium => IssueSeverity::Warning,
                    InterventionUrgency::Low => IssueSeverity::Info,
                }
            },
            MethodologyError::ValidationFailed { .. } => IssueSeverity::Error,
            MethodologyError::ExecutionError { .. } => IssueSeverity::Error,
            MethodologyError::CoordinationError { .. } => IssueSeverity::Warning,
            _ => IssueSeverity::Warning,
        }
    }
}

// ============================================================================
// Default Implementations and Constants
// ============================================================================

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

impl Default for ExecutionStatus {
    fn default() -> Self {
        ExecutionStatus::NotStarted
    }
}

impl Default for ValidationLevel {
    fn default() -> Self {
        ValidationLevel::Standard
    }
}

impl Default for ExecutionMode {
    fn default() -> Self {
        ExecutionMode::Sequential
    }
}

// Constants for protocol configuration
pub const DEFAULT_INSTRUCTION_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes
pub const DEFAULT_COORDINATION_TIMEOUT: Duration = Duration::from_secs(60); // 1 minute
pub const DEFAULT_VALIDATION_TIMEOUT: Duration = Duration::from_secs(30); // 30 seconds
pub const MAX_RETRY_ATTEMPTS: u32 = 3;
pub const DEFAULT_PROGRESS_REPORT_INTERVAL: Duration = Duration::from_secs(10);
pub const MAX_PARALLEL_INSTRUCTIONS: usize = 10;
pub const DEFAULT_QUALITY_THRESHOLD: f64 = 0.8; // 80%

// Protocol version for compatibility checking
pub const METHODOLOGY_PROTOCOL_VERSION: &str = "1.0.0";

// ============================================================================
// Type Aliases for Convenience
// ============================================================================

/// Type alias for methodology execution results
pub type MethodologyResult<T> = Result<T, MethodologyError>;

/// Type alias for validation results
pub type ValidationOutcome = Result<ValidationResult, MethodologyError>;

/// Type alias for instruction execution results
pub type InstructionResult = Result<InstructionExecutionResponse, MethodologyError>;

// ============================================================================
// Additional Helper Types Referenced Above
// ============================================================================

/// Security requirements for methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRequirements {
    pub authentication_required: bool,
    pub authorization_level: String,
    pub encryption_required: bool,
    pub audit_logging: bool,
    pub access_controls: Vec<String>,
}

/// Quality requirements for methodology execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRequirements {
    pub minimum_quality_score: f64,
    pub validation_strictness: ValidationStrictness,
    pub quality_gates: Vec<String>,
    pub performance_requirements: PerformanceRequirements,
}

/// Performance requirements specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    pub max_execution_time: Option<Duration>,
    pub min_throughput: Option<f64>,
    pub max_error_rate: Option<f64>,
    pub min_efficiency_score: Option<f64>,
}

/// Timing constraints for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConstraints {
    pub start_deadline: Option<SystemTime>,
    pub completion_deadline: Option<SystemTime>,
    pub max_execution_time: Option<Duration>,
    pub milestone_deadlines: HashMap<String, SystemTime>,
}

/// Human intervention options for validation checkpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanInterventionOptions {
    pub intervention_allowed: bool,
    pub approval_required: bool,
    pub escalation_path: Vec<String>,
    pub timeout_action: TimeoutAction,
}

/// Actions to take when human intervention times out
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeoutAction {
    Proceed,
    Pause,
    Fail,
    UseDefault,
}

/// Execution results structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResults {
    pub primary_results: HashMap<String, serde_json::Value>,
    pub secondary_results: HashMap<String, serde_json::Value>,
    pub generated_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub created_directories: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

// These are just the foundational types - there are many more supporting
// types that would be needed for a complete implementation, but this gives
// you the core structure of the methodology protocols module.
