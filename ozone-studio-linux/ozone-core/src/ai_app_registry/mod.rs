use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;

use shared_protocols::{ComponentType, EcosystemIdentity, Endpoint};

// Registry management engine - maintains the authoritative registry of all AI Apps
// This is the single source of truth about what AI Apps exist and what they can do
pub mod registry_management_engine;

// Capability discovery system - automatically discovers and catalogs AI App capabilities  
// Like a talent scout who identifies and documents the skills of team members
pub mod capability_discovery_system;

// Health monitoring coordinator - tracks the operational status of all AI Apps
// Ensures the registry knows which AI Apps are available and performing well
pub mod health_monitoring_coordinator;

// Performance tracking system - monitors and analyzes AI App performance over time
// Helps optimize assignments by understanding each AI App's strengths and patterns
pub mod performance_tracking_system;

// Registration protocol manager - handles the secure registration of new AI Apps
// Manages the process by which new AI Apps join the ecosystem
pub mod registration_protocol_manager;

// Capability matching engine - intelligently matches tasks to AI App capabilities
// The "matchmaker" that finds the perfect AI App for each specific need
pub mod capability_matching_engine;

// Re-export registry types
pub use registry_management_engine::{
    AIAppRegistry,
    RegistryEngine,
    RegistryEntry,
    RegistryMaintenance,
    RegistryValidation,
};

pub use capability_discovery_system::{
    CapabilityDiscovery,
    CapabilityProfile,
    CapabilityCatalog,
    DiscoveryProtocol,
    CapabilityEvolution,
};

pub use health_monitoring_coordinator::{
    HealthMonitor,
    HealthCheck,
    HealthStatus,
    HealthMetrics,
    HealthTrends,
};

pub use performance_tracking_system::{
    PerformanceTracker,
    PerformanceMetrics,
    PerformanceHistory,
    BenchmarkComparison,
    PerformanceOptimization,
};

pub use registration_protocol_manager::{
    RegistrationProtocol,
    RegistrationRequest,
    RegistrationValidation,
    RegistrationResponse,
    RegistrationSecurity,
};

pub use capability_matching_engine::{
    CapabilityMatcher,
    MatchingCriteria,
    MatchingResult,
    MatchingOptimization,
    CapabilityScoring,
};

// Core registry types that define how AI Apps are managed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAppRegistration {
    pub registration_id: String,
    pub app_identity: EcosystemIdentity,
    pub capabilities: Vec<AIAppCapability>,
    pub service_level: ServiceLevel,
    pub availability: AvailabilityProfile,
    pub bootstrap_integration: BootstrapIntegration,
    pub registration_timestamp: SystemTime,
    pub security_credentials: SecurityCredentials,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAppCapability {
    pub capability_id: String,
    pub capability_name: String,
    pub capability_type: CapabilityType,
    pub description: String,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
    pub quality_metrics: Vec<QualityMetric>,
    pub performance_characteristics: PerformanceCharacteristics,
    pub resource_requirements: CapabilityResourceRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapabilityType {
    Analysis,
    Generation,
    Modification,
    Validation,
    Coordination,
    Integration,
    Optimization,
    Communication,
    Storage,
    Processing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetric {
    pub metric_name: String,
    pub measurement_method: String,
    pub typical_score: f64,
    pub best_case_score: f64,
    pub worst_case_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCharacteristics {
    pub typical_response_time: Duration,
    pub throughput_capacity: f64,
    pub accuracy_rating: f64,
    pub reliability_rating: f64,
    pub scalability_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResourceRequirements {
    pub cpu_usage_typical: f64,
    pub memory_usage_typical: f64,
    pub network_usage_typical: f64,
    pub storage_usage_typical: f64,
    pub coordination_overhead: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceLevel {
    Foundational,    // Critical to ecosystem operation (like SPARK)
    Specialized,     // Domain-specific expert capabilities  
    Enhanced,        // Advanced or premium capabilities
    Experimental,    // New or unproven capabilities
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityProfile {
    pub availability_type: AvailabilityType,
    pub expected_uptime: f64,
    pub maintenance_windows: Vec<MaintenanceWindow>,
    pub failure_recovery_time: Duration,
    pub load_handling_capacity: LoadHandlingCapacity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AvailabilityType {
    Required,        // Must be available for ecosystem to function
    Optional,        // Enhances capabilities but not required
    Conditional,     // Available under certain conditions
    OnDemand,        // Available when explicitly requested
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceWindow {
    pub window_id: String,
    pub start_time: SystemTime,
    pub duration: Duration,
    pub maintenance_type: MaintenanceType,
    pub impact_assessment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaintenanceType {
    Routine,
    Emergency,
    Upgrade,
    Performance,
    Security,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadHandlingCapacity {
    pub max_concurrent_requests: usize,
    pub queue_capacity: usize,
    pub load_shedding_threshold: f64,
    pub scaling_capability: ScalingCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingCapability {
    None,
    Vertical,        // Can handle more load on same instance
    Horizontal,      // Can spawn multiple instances
    Elastic,         // Automatically scales based on demand
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BootstrapIntegration {
    Essential,       // Required during ecosystem bootstrap
    Standard,        // Standard integration during normal operation
    Optional,        // Enhanced capabilities, not required for basic operation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCredentials {
    pub certificate_id: String,
    pub public_key_fingerprint: String,
    pub authorization_level: AuthorizationLevel,
    pub access_permissions: Vec<Permission>,
    pub security_clearance: SecurityClearance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationLevel {
    Basic,
    Standard,
    Elevated,
    Administrative,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub permission_type: String,
    pub scope: String,
    pub operations: Vec<String>,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityClearance {
    Public,
    Internal,
    Restricted,
    Confidential,
    TopSecret,
}

// Registry operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityQuery {
    pub query_id: String,
    pub required_capabilities: Vec<String>,
    pub quality_requirements: Vec<QualityRequirement>,
    pub performance_requirements: Vec<PerformanceRequirement>,
    pub availability_requirements: AvailabilityRequirement,
    pub resource_constraints: Vec<ResourceConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRequirement {
    pub quality_aspect: String,
    pub minimum_score: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirement {
    pub performance_metric: String,
    pub minimum_value: f64,
    pub maximum_value: Option<f64>,
    pub criticality: RequirementCriticality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequirementCriticality {
    Optional,
    Preferred,
    Required,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityRequirement {
    pub minimum_uptime: f64,
    pub max_response_time: Duration,
    pub load_tolerance: f64,
    pub failure_handling: FailureHandlingRequirement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureHandlingRequirement {
    GracefulDegradation,
    FailOver,
    Retry,
    HumanEscalation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraint {
    pub resource_type: String,
    pub maximum_usage: f64,
    pub sharing_acceptable: bool,
}

// Registry response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityQueryResponse {
    pub query_id: String,
    pub matching_apps: Vec<AIAppMatch>,
    pub recommendation_confidence: f64,
    pub alternative_suggestions: Vec<AlternativeSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAppMatch {
    pub app_type: ComponentType,
    pub match_score: f64,
    pub capability_alignment: Vec<CapabilityAlignment>,
    pub performance_prediction: PerformancePrediction,
    pub availability_status: CurrentAvailabilityStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityAlignment {
    pub required_capability: String,
    pub app_capability: String,
    pub alignment_score: f64,
    pub gap_analysis: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePrediction {
    pub predicted_response_time: Duration,
    pub predicted_accuracy: f64,
    pub predicted_resource_usage: ResourceUsagePrediction,
    pub confidence_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsagePrediction {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub network_usage: f64,
    pub duration_estimate: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentAvailabilityStatus {
    pub available: bool,
    pub current_load: f64,
    pub estimated_wait_time: Duration,
    pub queue_position: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeSuggestion {
    pub suggestion_type: SuggestionType,
    pub description: String,
    pub trade_offs: Vec<String>,
    pub implementation_effort: ImplementationEffort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    DifferentApproach,
    CombinedCapabilities,
    QualityTradeOff,
    TimelineAdjustment,
    ResourceReallocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationEffort {
    Minimal,
    Low,
    Moderate,
    High,
    Significant,
}

// Registry traits
pub trait RegistryInterface {
    fn register_ai_app(&mut self, registration: AIAppRegistration) -> Result<RegistrationResult>;
    fn query_capabilities(&self, query: CapabilityQuery) -> Result<CapabilityQueryResponse>;
    fn update_app_status(&mut self, app_type: ComponentType, status: AIAppStatus) -> Result<()>;
    fn get_app_health(&self, app_type: ComponentType) -> Result<HealthStatus>;
    fn deregister_ai_app(&mut self, app_type: ComponentType) -> Result<DeregistrationResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResult {
    pub registration_id: String,
    pub status: RegistrationStatus,
    pub assigned_endpoints: Vec<String>,
    pub integration_instructions: Vec<String>,
    pub next_health_check: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationStatus {
    Successful,
    Pending,
    Failed,
    RequiresApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeregistrationResult {
    pub deregistration_id: String,
    pub status: DeregistrationStatus,
    pub cleanup_required: Vec<String>,
    pub data_retention_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeregistrationStatus {
    Completed,
    InProgress,
    Failed,
    RequiresManualIntervention,
}
