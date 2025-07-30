//! The Bootstrap Methodology Loader is responsible for loading and managing the
//! single hard-coded methodology that enables the creation of all other methodologies.
//! This is the "bootstrap paradox" solution - we need one methodology to exist
//! before the system can create others, so this module provides that foundation.
//! 
//! Think of this as the "DNA" of the methodology system - it contains the 
//! fundamental instructions for how to create new methodologies from human guidance.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::sync::Arc;

use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

use crate::{
    Methodology,
    MethodologyMetadata,
    ExecutionFramework,
    ValidationFramework,
    MethodologyCategory,
    DifficultyLevel,
    MethodologyRuntimeError,
};

use shared_protocols::{
    ComponentType,
    CoordinationStrategy,
    StrategicAlignment,
};

// Bootstrap methodology components
mod bootstrap_loader;
mod methodology_validator;
mod bootstrap_installer;
mod dependency_checker;
mod compatibility_validator;

// Hard-coded methodology definitions
mod create_methodology_from_human_guidance;
mod bootstrap_methodology_registry;
mod validation_framework_generator;

// Bootstrap error handling and recovery
mod bootstrap_error_handler;
mod bootstrap_recovery;
mod bootstrap_diagnostics;

// Re-export core bootstrap types
pub use bootstrap_loader::{
    BootstrapMethodologyLoader,
    LoaderConfiguration,
    LoaderMetrics,
    LoaderError,
    LoadingProgress,
    LoadingResult,
};

pub use methodology_validator::{
    BootstrapMethodologyValidator,
    ValidationCriteria,
    ValidationReport,
    ValidationIssue,
    ValidatorConfiguration,
    ValidationMetrics,
};

pub use bootstrap_installer::{
    BootstrapInstaller,
    InstallationProgress,
    InstallationResult,
    InstallationConfiguration,
    InstallationError,
    InstallationMetrics,
};

pub use dependency_checker::{
    DependencyChecker,
    DependencyGraph,
    DependencyViolation,
    DependencyResolution,
    DependencyMetrics,
    CircularDependencyDetector,
};

pub use compatibility_validator::{
    CompatibilityValidator,
    CompatibilityReport,
    CompatibilityIssue,
    CompatibilityLevel,
    CompatibilityMatrix,
    VersionCompatibility,
};

pub use create_methodology_from_human_guidance::{
    CreateMethodologyFromHumanGuidance,
    HumanGuidanceProcessor,
    MethodologyBuilder,
    GuidanceValidation,
    IterativeRefinement,
    QualityAssurance,
};

pub use bootstrap_methodology_registry::{
    BootstrapMethodologyRegistry,
    MethodologyRegistration,
    RegistryMetadata,
    RegistryValidation,
    RegistryMetrics,
    RegistryError,
};

pub use validation_framework_generator::{
    ValidationFrameworkGenerator,
    FrameworkTemplate,
    ValidationCriteriaBuilder,
    QualityGateBuilder,
    FrameworkValidator,
    FrameworkMetrics,
};

pub use bootstrap_error_handler::{
    BootstrapErrorHandler,
    ErrorClassification,
    ErrorRecovery,
    ErrorReporting,
    ErrorMetrics,
    CriticalErrorHandler,
};

pub use bootstrap_recovery::{
    BootstrapRecoveryManager,
    RecoveryStrategy,
    RecoveryExecution,
    RecoveryValidation,
    RecoveryMetrics,
    FailsafeActivation,
};

pub use bootstrap_diagnostics::{
    BootstrapDiagnostics,
    DiagnosticRunner,
    DiagnosticReport,
    HealthCheck,
    SystemValidation,
    DiagnosticMetrics,
};

// Main bootstrap methodology type - this is the hard-coded methodology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapMethodology {
    pub methodology: Methodology,
    pub bootstrap_metadata: BootstrapMetadata,
    pub validation_checkpoints: Vec<BootstrapValidationCheckpoint>,
    pub installation_requirements: InstallationRequirements,
    pub compatibility_matrix: CompatibilityMatrix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapMetadata {
    pub bootstrap_version: String,
    pub creation_date: SystemTime,
    pub required_components: Vec<ComponentType>,
    pub minimum_system_requirements: SystemRequirements,
    pub bootstrap_priority: BootstrapPriority,
    pub critical_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BootstrapPriority {
    Critical,
    Essential,
    Important,
    Standard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRequirements {
    pub minimum_memory: u64,
    pub minimum_storage: u64,
    pub required_network_access: bool,
    pub required_permissions: Vec<String>,
    pub supported_platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapValidationCheckpoint {
    pub checkpoint_id: String,
    pub checkpoint_name: String,
    pub validation_type: BootstrapValidationType,
    pub validation_criteria: Vec<String>,
    pub failure_recovery: BootstrapRecoveryAction,
    pub checkpoint_priority: CheckpointPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BootstrapValidationType {
    ComponentAvailability,
    MethodologyIntegrity,
    SystemCompatibility,
    SecurityValidation,
    DependencyResolution,
    FunctionalValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BootstrapRecoveryAction {
    RetryWithBackoff,
    FallbackToSafeMode,
    RequestHumanIntervention,
    AttemptAutoRepair,
    AbortWithDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointPriority {
    Blocking,
    Critical,
    Important,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationRequirements {
    pub pre_installation_checks: Vec<PreInstallationCheck>,
    pub installation_order: Vec<InstallationStep>,
    pub post_installation_validation: Vec<PostInstallationValidation>,
    pub rollback_strategy: RollbackStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreInstallationCheck {
    pub check_id: String,
    pub check_description: String,
    pub check_type: CheckType,
    pub required_result: String,
    pub failure_action: CheckFailureAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckType {
    ComponentPresence,
    VersionCompatibility,
    ResourceAvailability,
    PermissionValidation,
    NetworkConnectivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckFailureAction {
    BlockInstallation,
    IssueWarning,
    AttemptAutoFix,
    PromptUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationStep {
    pub step_id: String,
    pub step_description: String,
    pub step_type: InstallationStepType,
    pub step_dependencies: Vec<String>,
    pub timeout: Duration,
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallationStepType {
    MethodologyRegistration,
    DependencyInjection,
    ValidationSetup,
    IntegrationTest,
    SecurityConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
    pub retry_conditions: Vec<RetryCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Linear,
    Exponential,
    Fixed,
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    TransientError,
    ResourceContention,
    NetworkTimeout,
    ComponentUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInstallationValidation {
    pub validation_id: String,
    pub validation_description: String,
    pub validation_method: ValidationMethod,
    pub success_criteria: Vec<String>,
    pub validation_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationMethod {
    FunctionalTest,
    IntegrationTest,
    PerformanceTest,
    SecurityAudit,
    CompatibilityTest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackStrategy {
    NoRollback,
    PartialRollback,
    CompleteRollback,
    SelectiveRollback,
}

// Bootstrap error types specific to this module
#[derive(Error, Debug)]
pub enum BootstrapError {
    #[error("Bootstrap methodology loading failed: {reason}")]
    LoadingFailed { reason: String },
    
    #[error("Bootstrap validation failed: {checkpoint} - {details}")]
    ValidationFailed { checkpoint: String, details: String },
    
    #[error("Bootstrap installation failed: {step} - {details}")]
    InstallationFailed { step: String, details: String },
    
    #[error("Bootstrap dependency error: {dependency} - {details}")]
    DependencyError { dependency: String, details: String },
    
    #[error("Bootstrap compatibility error: {component} - {details}")]
    CompatibilityError { component: String, details: String },
    
    #[error("Bootstrap security error: {details}")]
    SecurityError { details: String },
    
    #[error("Bootstrap system requirements not met: {requirement}")]
    SystemRequirementsNotMet { requirement: String },
    
    #[error("Bootstrap recovery failed: {recovery_action} - {details}")]
    RecoveryFailed { recovery_action: String, details: String },
}

// Bootstrap validation result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapValidation {
    pub validation_id: String,
    pub validation_status: BootstrapValidationStatus,
    pub validation_results: Vec<BootstrapValidationResult>,
    pub overall_score: f64,
    pub recommendations: Vec<String>,
    pub validation_timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BootstrapValidationStatus {
    Passed,
    Failed,
    Warning,
    RequiresIntervention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapValidationResult {
    pub checkpoint_id: String,
    pub status: ValidationStatus,
    pub score: f64,
    pub details: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationStatus {
    Success,
    Failure,
    Warning,
    Skipped,
    InProgress,
}
