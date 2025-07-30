// This module defines the fundamental communication protocols that enable all components
// in the OZONE STUDIO ecosystem to interact with each other. Think of this as the 
// "universal language" that every component speaks, regardless of their specialization.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::fmt;
use std::net::SocketAddr;

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Import the component types that we need to reference
use crate::{ComponentType, Endpoint, Protocol};

// Import security types for secure communication
use shared_security::{AuthenticationCredentials, SecurityContext};

// =============================================================================
// Core Ecosystem Message Types
// =============================================================================

/// The fundamental message type that flows between all ecosystem components.
/// Every interaction in the OZONE STUDIO ecosystem begins with an EcosystemMessage.
/// This is like the "envelope" that contains all communication between components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemMessage {
    /// Unique identifier for tracking this specific message through the ecosystem
    pub message_id: String,
    
    /// Which component sent this message (enables response routing)
    pub sender: ComponentType,
    
    /// Which component should receive this message
    pub receiver: ComponentType,
    
    /// What type of communication this represents
    pub message_type: EcosystemMessageType,
    
    /// The actual content being communicated
    pub payload: serde_json::Value,
    
    /// When this message was created (for timeout and ordering)
    pub timestamp: SystemTime,
    
    /// How long to wait for a response before timing out
    pub timeout: Option<Duration>,
    
    /// Security context for authenticated communication
    pub security_context: Option<SecurityContext>,
    
    /// Correlation ID for tracking related messages (like request/response pairs)
    pub correlation_id: Option<String>,
    
    /// Priority level for message processing
    pub priority: MessagePriority,
}

/// Defines the different types of communication that can occur between components.
/// Each type represents a different "conversation pattern" in the ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EcosystemMessageType {
    /// Request for a component to perform some operation (expects response)
    Request,
    
    /// Response to a previous request (completes a request/response cycle)
    Response,
    
    /// Notification that doesn't expect a response (fire-and-forget)
    Notification,
    
    /// Broadcast message intended for multiple components
    Broadcast,
    
    /// Health check ping to verify component availability
    HealthCheck,
    
    /// Response to health check (confirms component is operational)
    HealthCheckResponse,
    
    /// Component announcing its presence to the ecosystem
    ComponentRegistration,
    
    /// Component announcing it's shutting down gracefully
    ComponentShutdown,
    
    /// Emergency alert that requires immediate attention
    Alert,
    
    /// Heartbeat message to maintain connection state
    Heartbeat,
}

/// Priority levels for message processing - higher priority messages are processed first.
/// This ensures critical ecosystem functions (like consciousness decisions) are not
/// delayed by routine operations (like file system operations).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Critical system operations (consciousness decisions, security alerts)
    Critical = 0,
    
    /// High priority operations (task orchestration, methodology execution)
    High = 1,
    
    /// Normal priority operations (most day-to-day component interactions)
    Normal = 2,
    
    /// Low priority operations (background processing, maintenance)
    Low = 3,
    
    /// Background operations (cleanup, optimization, archival)
    Background = 4,
}

/// Standard response format for all ecosystem communications.
/// This provides a consistent way for components to communicate success, failure,
/// and partial results across the entire ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemResponse {
    /// ID of the original message this is responding to
    pub request_id: String,
    
    /// Whether the requested operation succeeded
    pub success: bool,
    
    /// The actual result data (if successful)
    pub data: Option<serde_json::Value>,
    
    /// Error information (if unsuccessful)
    pub error: Option<EcosystemError>,
    
    /// Additional metadata about the operation
    pub metadata: ResponseMetadata,
    
    /// When this response was generated
    pub timestamp: SystemTime,
    
    /// Which component generated this response
    pub responder: ComponentType,
}

/// Metadata that provides additional context about how an operation was performed.
/// This helps with debugging, optimization, and understanding ecosystem behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// How long the operation took to complete
    pub processing_duration: Duration,
    
    /// How much computational resources were used
    pub resource_usage: ResourceUsage,
    
    /// Quality metrics for the operation
    pub quality_metrics: QualityMetrics,
    
    /// Any warnings that occurred during processing
    pub warnings: Vec<String>,
    
    /// Debug information (only included in development builds)
    pub debug_info: Option<HashMap<String, serde_json::Value>>,
}

/// Tracks how much computational resources an operation consumed.
/// This information helps NEXUS optimize resource allocation across the ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU time consumed (in milliseconds)
    pub cpu_time_ms: u64,
    
    /// Peak memory usage (in bytes)
    pub memory_peak_bytes: u64,
    
    /// Network bandwidth used (in bytes)
    pub network_bytes: u64,
    
    /// Disk I/O performed (in bytes)
    pub disk_io_bytes: u64,
    
    /// Number of AI Apps coordinated with
    pub ai_app_coordination_count: u32,
}

/// Quality metrics that help assess how well an operation was performed.
/// These metrics feed into ZSEI's learning systems for continuous improvement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Overall quality score (0.0 to 1.0)
    pub overall_score: f64,
    
    /// How accurate the results were
    pub accuracy_score: f64,
    
    /// How efficient the operation was
    pub efficiency_score: f64,
    
    /// How well the operation aligned with strategic goals
    pub strategic_alignment_score: f64,
    
    /// User satisfaction rating (if applicable)
    pub user_satisfaction_score: Option<f64>,
}

// =============================================================================
// Component Registration and Discovery
// =============================================================================

/// Information a component provides when joining the ecosystem.
/// This is like a component's "business card" - it tells other components
/// what services it provides and how to communicate with it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRegistration {
    /// The component's identity information
    pub identity: ComponentIdentity,
    
    /// What capabilities this component provides
    pub capabilities: Vec<ComponentCapability>,
    
    /// How other components can communicate with this one
    pub endpoints: Vec<Endpoint>,
    
    /// Current operational status
    pub status: ComponentStatus,
    
    /// When this component started up
    pub startup_time: SystemTime,
    
    /// Configuration information relevant to other components
    pub configuration: ComponentConfiguration,
}

/// Core identity information for an ecosystem component.
/// This uniquely identifies each component instance in the ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentIdentity {
    /// Unique identifier for this specific component instance
    pub component_id: String,
    
    /// What type of component this is (OZONE STUDIO, ZSEI, etc.)
    pub component_type: ComponentType,
    
    /// Human-readable name for this component
    pub display_name: String,
    
    /// Software version of this component
    pub version: String,
    
    /// Build information (git commit, build date, etc.)
    pub build_info: BuildInfo,
    
    /// Which device this component is running on
    pub device_info: DeviceInfo,
}

/// Information about how this component was built.
/// Helps with debugging and ensuring ecosystem compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Git commit hash this build was created from
    pub git_commit: String,
    
    /// When this binary was compiled
    pub build_timestamp: SystemTime,
    
    /// Which compiler and version was used
    pub compiler_info: String,
    
    /// Build configuration (debug, release, etc.)
    pub build_configuration: String,
    
    /// Any special build flags or features enabled
    pub build_features: Vec<String>,
}

/// Information about the device this component is running on.
/// Helps NEXUS understand available resources and coordinate effectively.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Unique identifier for this device
    pub device_id: String,
    
    /// Type of device (desktop, mobile, server, etc.)
    pub device_type: DeviceType,
    
    /// Operating system information
    pub operating_system: OperatingSystemInfo,
    
    /// Hardware capabilities
    pub hardware_info: HardwareInfo,
    
    /// Network configuration
    pub network_info: NetworkInfo,
}

/// Classification of device types for resource planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    /// Desktop or laptop computer
    PersonalComputer,
    
    /// Mobile phone or tablet
    MobileDevice,
    
    /// Dedicated server
    Server,
    
    /// Cloud computing instance
    CloudInstance,
    
    /// Embedded or IoT device
    EmbeddedDevice,
    
    /// Edge computing node
    EdgeDevice,
    
    /// High-performance computing cluster
    HPCCluster,
}

/// Operating system information for compatibility and optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingSystemInfo {
    /// OS family (Linux, Windows, macOS, etc.)
    pub family: String,
    
    /// Specific OS version
    pub version: String,
    
    /// CPU architecture (x86_64, arm64, etc.)
    pub architecture: String,
    
    /// Available system libraries and frameworks
    pub available_frameworks: Vec<String>,
}

/// Hardware capabilities for resource allocation planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// Number of CPU cores available
    pub cpu_cores: u32,
    
    /// Total system memory in bytes
    pub total_memory_bytes: u64,
    
    /// Available storage space in bytes
    pub available_storage_bytes: u64,
    
    /// GPU information (if available)
    pub gpu_info: Option<GPUInfo>,
    
    /// Specialized hardware (TPU, FPGA, etc.)
    pub specialized_hardware: Vec<SpecializedHardware>,
}

/// GPU capabilities for AI processing acceleration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPUInfo {
    /// GPU manufacturer and model
    pub model: String,
    
    /// Available GPU memory in bytes
    pub memory_bytes: u64,
    
    /// Compute capability or version
    pub compute_capability: String,
    
    /// Supported frameworks (CUDA, OpenCL, etc.)
    pub supported_frameworks: Vec<String>,
}

/// Information about specialized hardware for specific workloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializedHardware {
    /// Type of specialized hardware
    pub hardware_type: String,
    
    /// Manufacturer and model
    pub model: String,
    
    /// Capabilities this hardware provides
    pub capabilities: Vec<String>,
    
    /// Performance characteristics
    pub performance_metrics: HashMap<String, f64>,
}

/// Network configuration for communication optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Available network interfaces
    pub interfaces: Vec<NetworkInterface>,
    
    /// Bandwidth capabilities
    pub bandwidth_info: BandwidthInfo,
    
    /// Network topology information
    pub topology_info: Option<NetworkTopology>,
}

/// Information about a network interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name (eth0, wlan0, etc.)
    pub name: String,
    
    /// Interface type (ethernet, wifi, cellular, etc.)
    pub interface_type: String,
    
    /// IP addresses assigned to this interface
    pub ip_addresses: Vec<std::net::IpAddr>,
    
    /// Maximum transmission unit
    pub mtu: u32,
    
    /// Whether this interface is currently active
    pub is_active: bool,
}

/// Bandwidth capabilities for network optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthInfo {
    /// Upstream bandwidth in bits per second
    pub upstream_bps: u64,
    
    /// Downstream bandwidth in bits per second
    pub downstream_bps: u64,
    
    /// Network latency in milliseconds
    pub latency_ms: f64,
    
    /// Connection reliability (0.0 to 1.0)
    pub reliability_score: f64,
}

/// Network topology information for advanced coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    /// Whether this device is behind NAT
    pub behind_nat: bool,
    
    /// Public IP address (if known)
    pub public_ip: Option<std::net::IpAddr>,
    
    /// Network segment or subnet information
    pub subnet_info: Option<String>,
    
    /// Quality of Service (QoS) capabilities
    pub qos_capabilities: Vec<String>,
}

/// Specific capabilities that a component provides to the ecosystem.
/// This allows other components to discover what services are available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentCapability {
    /// Unique identifier for this capability
    pub capability_id: String,
    
    /// Human-readable name for this capability
    pub name: String,
    
    /// Detailed description of what this capability does
    pub description: String,
    
    /// What type of capability this is
    pub capability_type: CapabilityType,
    
    /// Input requirements for using this capability
    pub input_requirements: Vec<InputRequirement>,
    
    /// What outputs this capability produces
    pub output_specifications: Vec<OutputSpecification>,
    
    /// Performance characteristics of this capability
    pub performance_metrics: PerformanceMetrics,
    
    /// Any dependencies this capability has
    pub dependencies: Vec<CapabilityDependency>,
}

/// Classification of different types of capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapabilityType {
    /// Data processing and analysis
    DataProcessing,
    
    /// AI and machine learning operations
    AIProcessing,
    
    /// File system and storage operations
    StorageManagement,
    
    /// Network and communication services
    NetworkServices,
    
    /// Human interface and interaction
    HumanInterface,
    
    /// Code development and analysis
    CodeDevelopment,
    
    /// Text processing and generation
    TextProcessing,
    
    /// Consciousness and reasoning
    ConsciousnessServices,
    
    /// Intelligence coordination
    IntelligenceCoordination,
    
    /// Security and authentication
    SecurityServices,
    
    /// Methodology execution
    MethodologyExecution,
    
    /// Cross-domain analysis
    CrossDomainAnalysis,
}

/// Requirements for using a specific capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputRequirement {
    /// Name of the required input parameter
    pub parameter_name: String,
    
    /// Data type expected for this parameter
    pub data_type: String,
    
    /// Whether this parameter is required or optional
    pub required: bool,
    
    /// Description of what this parameter represents
    pub description: String,
    
    /// Valid values or constraints for this parameter
    pub constraints: Vec<ParameterConstraint>,
}

/// Constraints on parameter values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterConstraint {
    /// Minimum value for numeric parameters
    MinValue(f64),
    
    /// Maximum value for numeric parameters
    MaxValue(f64),
    
    /// Set of allowed string values
    AllowedValues(Vec<String>),
    
    /// Regular expression pattern for string validation
    Pattern(String),
    
    /// Minimum length for string or array parameters
    MinLength(usize),
    
    /// Maximum length for string or array parameters
    MaxLength(usize),
}

/// Specification of what a capability produces as output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSpecification {
    /// Name of the output field
    pub field_name: String,
    
    /// Data type of the output
    pub data_type: String,
    
    /// Description of what this output represents
    pub description: String,
    
    /// Whether this output is always provided or conditional
    pub always_present: bool,
}

/// Performance characteristics of a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Typical response time in milliseconds
    pub typical_response_time_ms: f64,
    
    /// Maximum throughput (operations per second)
    pub max_throughput_ops_per_sec: f64,
    
    /// Resource requirements for this capability
    pub resource_requirements: ResourceRequirements,
    
    /// Reliability score (0.0 to 1.0)
    pub reliability_score: f64,
    
    /// Quality score for outputs (0.0 to 1.0)
    pub quality_score: f64,
}

/// Resource requirements for executing a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU cores needed
    pub cpu_cores: f64,
    
    /// Memory required in bytes
    pub memory_bytes: u64,
    
    /// Disk space needed in bytes
    pub disk_space_bytes: u64,
    
    /// Network bandwidth required in bits per second
    pub network_bandwidth_bps: u64,
    
    /// Whether GPU acceleration is needed
    pub gpu_required: bool,
    
    /// Estimated execution time
    pub estimated_duration: Duration,
}

/// Dependencies that a capability has on other ecosystem services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDependency {
    /// Which component type this depends on
    pub component_type: ComponentType,
    
    /// Specific capability needed from that component
    pub required_capability: String,
    
    /// Whether this dependency is required or optional
    pub dependency_type: DependencyType,
    
    /// Minimum version required for compatibility
    pub minimum_version: Option<String>,
}

/// Types of dependencies between capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    /// Must have this dependency to function at all
    Required,
    
    /// Can function without this dependency but with reduced capability
    Optional,
    
    /// Can use this dependency for enhanced performance
    Performance,
    
    /// Needs this dependency only for specific operations
    Conditional,
}

/// Configuration information that components share with the ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfiguration {
    /// Configuration parameters relevant to other components
    pub shared_parameters: HashMap<String, serde_json::Value>,
    
    /// Service endpoints that other components can use
    pub service_endpoints: Vec<ServiceEndpoint>,
    
    /// Security requirements for interacting with this component
    pub security_requirements: SecurityRequirements,
    
    /// Monitoring and health check configuration
    pub monitoring_config: MonitoringConfiguration,
}

/// Information about a service endpoint provided by a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Unique identifier for this endpoint
    pub endpoint_id: String,
    
    /// What service this endpoint provides
    pub service_name: String,
    
    /// Network address where this service is available
    pub address: SocketAddr,
    
    /// Protocol used for communication
    pub protocol: Protocol,
    
    /// Authentication required for this endpoint
    pub authentication_required: bool,
    
    /// Rate limiting configuration
    pub rate_limits: Option<RateLimits>,
    
    /// Health check URL for this service
    pub health_check_path: Option<String>,
}

/// Security requirements for component interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRequirements {
    /// Whether mutual TLS is required
    pub mutual_tls_required: bool,
    
    /// Authentication methods accepted
    pub accepted_auth_methods: Vec<AuthenticationMethod>,
    
    /// Minimum encryption level required
    pub minimum_encryption_level: EncryptionLevel,
    
    /// Whether message integrity checking is required
    pub integrity_checking_required: bool,
}

/// Different authentication methods supported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    /// Certificate-based authentication
    Certificate,
    
    /// API key authentication
    APIKey,
    
    /// Bearer token authentication
    BearerToken,
    
    /// Mutual TLS authentication
    MutualTLS,
    
    /// Custom authentication scheme
    Custom(String),
}

/// Encryption levels for secure communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionLevel {
    /// No encryption required
    None,
    
    /// Basic encryption (AES-128)
    Basic,
    
    /// Standard encryption (AES-256)
    Standard,
    
    /// High security encryption with forward secrecy
    High,
    
    /// Maximum security with quantum-resistant algorithms
    Maximum,
}

/// Rate limiting configuration for service endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// Maximum requests per minute
    pub requests_per_minute: u32,
    
    /// Maximum concurrent requests
    pub max_concurrent: u32,
    
    /// Burst allowance for temporary spikes
    pub burst_allowance: u32,
    
    /// Cooldown period after rate limit exceeded
    pub cooldown_duration: Duration,
}

/// Monitoring and health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfiguration {
    /// How often to send heartbeat messages
    pub heartbeat_interval: Duration,
    
    /// How long to wait for health check responses
    pub health_check_timeout: Duration,
    
    /// Metrics collection configuration
    pub metrics_config: MetricsConfiguration,
    
    /// Alert configuration for component issues
    pub alerting_config: AlertingConfiguration,
}

/// Configuration for metrics collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfiguration {
    /// Whether to collect performance metrics
    pub collect_performance_metrics: bool,
    
    /// Whether to collect resource usage metrics
    pub collect_resource_metrics: bool,
    
    /// Whether to collect quality metrics
    pub collect_quality_metrics: bool,
    
    /// How often to report metrics
    pub reporting_interval: Duration,
    
    /// How long to retain metrics data
    pub retention_duration: Duration,
}

/// Configuration for alerting on component issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfiguration {
    /// Whether alerting is enabled
    pub alerting_enabled: bool,
    
    /// Performance thresholds that trigger alerts
    pub performance_thresholds: PerformanceThresholds,
    
    /// Resource usage thresholds that trigger alerts
    pub resource_thresholds: ResourceThresholds,
    
    /// Who to notify when alerts are triggered
    pub notification_targets: Vec<NotificationTarget>,
}

/// Performance thresholds for alerting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Maximum acceptable response time in milliseconds
    pub max_response_time_ms: f64,
    
    /// Minimum acceptable throughput (operations per second)
    pub min_throughput_ops_per_sec: f64,
    
    /// Maximum acceptable error rate (0.0 to 1.0)
    pub max_error_rate: f64,
    
    /// Minimum acceptable availability (0.0 to 1.0)
    pub min_availability: f64,
}

/// Resource usage thresholds for alerting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceThresholds {
    /// Maximum CPU usage (0.0 to 1.0)
    pub max_cpu_usage: f64,
    
    /// Maximum memory usage (0.0 to 1.0)
    pub max_memory_usage: f64,
    
    /// Maximum disk usage (0.0 to 1.0)
    pub max_disk_usage: f64,
    
    /// Maximum network usage in bits per second
    pub max_network_usage_bps: u64,
}

/// Targets for alert notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTarget {
    /// Type of notification target
    pub target_type: NotificationTargetType,
    
    /// Address or identifier for the target
    pub target_address: String,
    
    /// Severity levels that should be sent to this target
    pub severity_levels: Vec<AlertSeverity>,
}

/// Types of notification targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationTargetType {
    /// Email notification
    Email,
    
    /// SMS notification
    SMS,
    
    /// Webhook notification
    Webhook,
    
    /// Slack channel
    Slack,
    
    /// System log
    Log,
    
    /// Dashboard display
    Dashboard,
}

/// Severity levels for alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational alerts
    Info,
    
    /// Warning conditions
    Warning,
    
    /// Error conditions requiring attention
    Error,
    
    /// Critical conditions requiring immediate action
    Critical,
    
    /// Emergency conditions requiring immediate intervention
    Emergency,
}

// =============================================================================
// Component Status and Health Monitoring
// =============================================================================

/// Current operational status of a component.
/// This helps other components understand if a service is available and reliable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentStatus {
    /// Component is fully operational and available
    Healthy,
    
    /// Component is operational but with reduced performance
    Degraded,
    
    /// Component is experiencing issues but still functional
    Warning,
    
    /// Component is not responding or has critical issues
    Critical,
    
    /// Component is in maintenance mode
    Maintenance,
    
    /// Component is starting up and not yet ready
    Starting,
    
    /// Component is shutting down gracefully
    Stopping,
    
    /// Component is offline or unreachable
    Offline,
}

/// Health check request sent to verify component availability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Unique ID for this health check
    pub check_id: String,
    
    /// Which component is requesting the health check
    pub requester: ComponentType,
    
    /// What level of health check is being requested
    pub check_level: HealthCheckLevel,
    
    /// When this health check was initiated
    pub timestamp: SystemTime,
    
    /// How long to wait for a response
    pub timeout: Duration,
}

/// Different levels of health checks with varying detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckLevel {
    /// Basic ping to verify component is responding
    Basic,
    
    /// Check that core services are functional
    Standard,
    
    /// Comprehensive check including performance metrics
    Detailed,
    
    /// Full diagnostic including all dependencies
    Comprehensive,
}

/// Response to a health check request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// ID of the original health check request
    pub check_id: String,
    
    /// Current status of this component
    pub status: ComponentStatus,
    
    /// Detailed health information
    pub health_details: HealthDetails,
    
    /// When this response was generated
    pub timestamp: SystemTime,
    
    /// How long the health check took to complete
    pub response_time: Duration,
}

/// Detailed health information about a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDetails {
    /// Overall health score (0.0 to 1.0)
    pub overall_health_score: f64,
    
    /// Current performance metrics
    pub performance_metrics: CurrentPerformanceMetrics,
    
    /// Resource usage information
    pub resource_usage: CurrentResourceUsage,
    
    /// Status of dependencies
    pub dependency_status: Vec<DependencyStatus>,
    
    /// Any current issues or warnings
    pub issues: Vec<HealthIssue>,
    
    /// Recent error information
    pub recent_errors: Vec<RecentError>,
    
    /// Uptime information
    pub uptime_info: UptimeInfo,
}

/// Current performance metrics for a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentPerformanceMetrics {
    /// Current average response time in milliseconds
    pub avg_response_time_ms: f64,
    
    /// Current throughput (operations per second)
    pub current_throughput_ops_per_sec: f64,
    
    /// Recent error rate (0.0 to 1.0)
    pub error_rate: f64,
    
    /// Number of successful operations in the last minute
    pub successful_operations_last_minute: u64,
    
    /// Number of failed operations in the last minute
    pub failed_operations_last_minute: u64,
}

/// Current resource usage for a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentResourceUsage {
    /// Current CPU usage (0.0 to 1.0)
    pub cpu_usage: f64,
    
    /// Current memory usage in bytes
    pub memory_usage_bytes: u64,
    
    /// Current disk usage in bytes
    pub disk_usage_bytes: u64,
    
    /// Current network usage in bits per second
    pub network_usage_bps: u64,
    
    /// Number of active connections or sessions
    pub active_connections: u32,
    
    /// Queue depths for various operations
    pub queue_depths: HashMap<String, u32>,
}

/// Status of a dependency from this component's perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStatus {
    /// Which component this dependency refers to
    pub component_type: ComponentType,
    
    /// Current status of this dependency
    pub status: ComponentStatus,
    
    /// Last time this dependency was checked
    pub last_checked: SystemTime,
    
    /// Response time for the last interaction
    pub last_response_time: Duration,
    
    /// Whether this dependency is currently required
    pub currently_required: bool,
}

/// Information about a current health issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    /// Unique identifier for this issue
    pub issue_id: String,
    
    /// Severity level of this issue
    pub severity: AlertSeverity,
    
    /// Description of the issue
    pub description: String,
    
    /// When this issue was first detected
    pub first_detected: SystemTime,
    
    /// When this issue was last observed
    pub last_observed: SystemTime,
    
    /// Suggested remediation actions
    pub remediation_suggestions: Vec<String>,
    
    /// Whether this issue is affecting functionality
    pub impacting_functionality: bool,
}

/// Information about a recent error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentError {
    /// When this error occurred
    pub timestamp: SystemTime,
    
    /// Error message or description
    pub error_message: String,
    
    /// Error category or type
    pub error_type: String,
    
    /// Which operation or function was being performed
    pub operation_context: String,
    
    /// Whether this error was recoverable
    pub recoverable: bool,
    
    /// How many times this error has occurred recently
    pub occurrence_count: u32,
}

/// Component uptime and availability information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeInfo {
    /// When this component was last started
    pub last_startup_time: SystemTime,
    
    /// Total uptime since last startup
    pub current_uptime: Duration,
    
    /// Uptime percentage over the last 24 hours
    pub uptime_24h: f64,
    
    /// Uptime percentage over the last 7 days
    pub uptime_7d: f64,
    
    /// Uptime percentage over the last 30 days
    pub uptime_30d: f64,
    
    /// Number of restarts in the last 24 hours
    pub restarts_24h: u32,
}

// =============================================================================
// Error Types for Ecosystem Communication
// =============================================================================

/// Comprehensive error types that can occur during ecosystem communication.
/// These errors help components understand what went wrong and how to respond.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum EcosystemError {
    #[error("Component not found: {component_type:?}")]
    ComponentNotFound { component_type: ComponentType },
    
    #[error("Component unavailable: {component_type:?} - {reason}")]
    ComponentUnavailable { component_type: ComponentType, reason: String },
    
    #[error("Communication timeout: {operation} timed out after {duration:?}")]
    CommunicationTimeout { operation: String, duration: Duration },
    
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },
    
    #[error("Authorization denied: {operation} not permitted")]
    AuthorizationDenied { operation: String },
    
    #[error("Protocol violation: {protocol} - {violation}")]
    ProtocolViolation { protocol: String, violation: String },
    
    #[error("Message serialization error: {details}")]
    SerializationError { details: String },
    
    #[error("Network error: {details}")]
    NetworkError { details: String },
    
    #[error("Resource exhaustion: {resource} - {details}")]
    ResourceExhaustion { resource: String, details: String },
    
    #[error("Configuration error: {component} - {details}")]
    ConfigurationError { component: String, details: String },
    
    #[error("Version incompatibility: expected {expected}, got {actual}")]
    VersionIncompatibility { expected: String, actual: String },
    
    #[error("Capability not supported: {capability} not available")]
    CapabilityNotSupported { capability: String },
    
    #[error("Rate limit exceeded: {limit} {period}")]
    RateLimitExceeded { limit: String, period: Duration },
    
    #[error("Internal component error: {component} - {details}")]
    InternalError { component: String, details: String },
}

// =============================================================================
// Utility Functions and Implementations
// =============================================================================

impl EcosystemMessage {
    /// Create a new ecosystem message with standard defaults.
    pub fn new(
        sender: ComponentType,
        receiver: ComponentType,
        message_type: EcosystemMessageType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            message_id: Uuid::new_v4().to_string(),
            sender,
            receiver,
            message_type,
            payload,
            timestamp: SystemTime::now(),
            timeout: Some(Duration::from_secs(30)), // Default 30 second timeout
            security_context: None,
            correlation_id: None,
            priority: MessagePriority::Normal,
        }
    }
    
    /// Create a request message that expects a response.
    pub fn request(
        sender: ComponentType,
        receiver: ComponentType,
        payload: serde_json::Value,
    ) -> Self {
        Self::new(sender, receiver, EcosystemMessageType::Request, payload)
    }
    
    /// Create a response to a previous request.
    pub fn response(
        sender: ComponentType,
        receiver: ComponentType,
        request_id: String,
        payload: serde_json::Value,
    ) -> Self {
        let mut msg = Self::new(sender, receiver, EcosystemMessageType::Response, payload);
        msg.correlation_id = Some(request_id);
        msg
    }
    
    /// Create a notification that doesn't expect a response.
    pub fn notification(
        sender: ComponentType,
        receiver: ComponentType,
        payload: serde_json::Value,
    ) -> Self {
        Self::new(sender, receiver, EcosystemMessageType::Notification, payload)
    }
    
    /// Set the priority of this message.
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set the timeout for this message.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    
    /// Set the security context for this message.
    pub fn with_security_context(mut self, context: SecurityContext) -> Self {
        self.security_context = Some(context);
        self
    }
    
    /// Check if this message has expired based on its timestamp and timeout.
    pub fn is_expired(&self) -> bool {
        if let Some(timeout) = self.timeout {
            self.timestamp.elapsed().unwrap_or(Duration::ZERO) > timeout
        } else {
            false
        }
    }
}

impl EcosystemResponse {
    /// Create a successful response.
    pub fn success(
        request_id: String,
        responder: ComponentType,
        data: serde_json::Value,
    ) -> Self {
        Self {
            request_id,
            success: true,
            data: Some(data),
            error: None,
            metadata: ResponseMetadata::default(),
            timestamp: SystemTime::now(),
            responder,
        }
    }
    
    /// Create an error response.
    pub fn error(
        request_id: String,
        responder: ComponentType,
        error: EcosystemError,
    ) -> Self {
        Self {
            request_id,
            success: false,
            data: None,
            error: Some(error),
            metadata: ResponseMetadata::default(),
            timestamp: SystemTime::now(),
            responder,
        }
    }
}

impl Default for ResponseMetadata {
    fn default() -> Self {
        Self {
            processing_duration: Duration::ZERO,
            resource_usage: ResourceUsage::default(),
            quality_metrics: QualityMetrics::default(),
            warnings: Vec::new(),
            debug_info: None,
        }
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_time_ms: 0,
            memory_peak_bytes: 0,
            network_bytes: 0,
            disk_io_bytes: 0,
            ai_app_coordination_count: 0,
        }
    }
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self {
            overall_score: 1.0,
            accuracy_score: 1.0,
            efficiency_score: 1.0,
            strategic_alignment_score: 1.0,
            user_satisfaction_score: None,
        }
    }
}
