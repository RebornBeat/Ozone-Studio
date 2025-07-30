//! # Ecosystem Communication Module
//! 
//! This module provides the foundational communication infrastructure for the OZONE STUDIO ecosystem.
//! It defines the core message types, protocols, and patterns that enable conscious orchestration
//! across all AI Apps and infrastructure components.
//! 
//! ## Architecture Philosophy
//! 
//! The ecosystem communication layer operates on the principle that conscious orchestration
//! requires standardized, reliable, and observable communication patterns. Every message in
//! the ecosystem carries not just data, but also the context needed for conscious decision-making.
//! 
//! ## Key Concepts
//! 
//! - **EcosystemMessage**: The fundamental unit of communication that carries context, priority,
//!   and security information alongside the actual message payload
//! - **Component Registration**: The process by which AI Apps announce their capabilities and
//!   establish their role in the conscious orchestration hierarchy
//! - **Health Monitoring**: Continuous observation of component health that feeds into
//!   OZONE STUDIO's strategic awareness and decision-making
//! - **Message Routing**: Intelligent routing that supports both direct coordination and
//!   broadcast patterns needed for ecosystem-wide orchestration

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

// Async and networking dependencies
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use thiserror::Error;

// Re-export types that other modules need
pub use crate::{ComponentType, Endpoint, EndpointType, Protocol};

/// The fundamental message type that carries all ecosystem communication.
/// 
/// Every communication in the OZONE STUDIO ecosystem is wrapped in an EcosystemMessage,
/// which provides the context, routing, security, and metadata needed for conscious
/// orchestration. This design ensures that OZONE STUDIO can maintain awareness of
/// all system communications and make informed coordination decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemMessage {
    /// Unique identifier for this message, used for correlation and tracking
    pub message_id: String,
    
    /// The type of message being sent, which determines how it's processed
    pub message_type: EcosystemMessageType,
    
    /// The component sending this message
    pub sender: ComponentIdentity,
    
    /// The intended recipient(s) of this message
    pub recipient: MessageRecipient,
    
    /// When this message was created
    pub timestamp: SystemTime,
    
    /// The actual message payload
    pub payload: MessagePayload,
    
    /// Message priority for processing order
    pub priority: MessagePriority,
    
    /// Security context for authentication and authorization
    pub security_context: Option<MessageSecurityContext>,
    
    /// Correlation ID for linking related messages (requests/responses)
    pub correlation_id: Option<String>,
    
    /// Message routing information
    pub routing_info: RoutingInfo,
    
    /// Performance and diagnostic metadata
    pub metadata: MessageMetadata,
    
    /// Message version for protocol evolution
    pub protocol_version: String,
}

impl EcosystemMessage {
    /// Creates a new ecosystem message with the specified parameters.
    /// This is the primary constructor for all ecosystem communications.
    pub fn new(
        message_type: EcosystemMessageType,
        sender: ComponentIdentity,
        recipient: MessageRecipient,
        payload: MessagePayload,
    ) -> Self {
        let message_id = Uuid::new_v4().to_string();
        let timestamp = SystemTime::now();
        
        Self {
            message_id,
            message_type,
            sender,
            recipient,
            timestamp,
            payload,
            priority: MessagePriority::Normal,
            security_context: None,
            correlation_id: None,
            routing_info: RoutingInfo::new(),
            metadata: MessageMetadata::new(),
            protocol_version: "1.0.0".to_string(),
        }
    }
    
    /// Creates a response message that correlates with this message.
    /// This maintains the request/response relationship needed for coordination tracking.
    pub fn create_response(&self, response_payload: MessagePayload) -> Self {
        let mut response = EcosystemMessage::new(
            EcosystemMessageType::Response,
            self.recipient.clone().into_identity().unwrap_or_else(|| {
                ComponentIdentity {
                    component_id: "unknown".to_string(),
                    component_type: ComponentType::ConsciousOrchestrator,
                    instance_id: None,
                }
            }),
            MessageRecipient::Component(self.sender.clone()),
            response_payload,
        );
        
        response.correlation_id = Some(self.message_id.clone());
        response.priority = self.priority.clone();
        response.routing_info.response_routing = true;
        
        response
    }
    
    /// Sets the priority of this message, affecting processing order
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Adds security context to this message for authentication
    pub fn with_security_context(mut self, security_context: MessageSecurityContext) -> Self {
        self.security_context = Some(security_context);
        self
    }
    
    /// Sets a correlation ID for linking related messages
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
    
    /// Marks this message for reliable delivery with acknowledgments
    pub fn require_acknowledgment(mut self) -> Self {
        self.routing_info.requires_acknowledgment = true;
        self
    }
    
    /// Sets a delivery timeout for this message
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.routing_info.delivery_timeout = Some(timeout);
        self
    }
    
    /// Calculates the age of this message since creation
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.timestamp)
            .unwrap_or_else(|_| Duration::from_secs(0))
    }
    
    /// Checks if this message has expired based on its timeout
    pub fn is_expired(&self) -> bool {
        if let Some(timeout) = self.routing_info.delivery_timeout {
            self.age() > timeout
        } else {
            false
        }
    }
    
    /// Adds a routing hop to track message path
    pub fn add_routing_hop(&mut self, component: ComponentIdentity) {
        self.routing_info.hops.push(RoutingHop {
            component,
            timestamp: SystemTime::now(),
        });
    }
}

/// Identifies the types of messages that flow through the ecosystem.
/// 
/// Each message type has specific semantics and processing requirements that
/// enable OZONE STUDIO to understand and respond appropriately to different
/// kinds of ecosystem events and coordination needs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EcosystemMessageType {
    /// Component registration requests and updates
    ComponentRegistration,
    
    /// Component status updates and health reports
    ComponentStatus,
    
    /// Health check requests and responses
    HealthCheck,
    
    /// Task coordination requests between components
    TaskCoordination,
    
    /// Methodology execution requests and updates
    MethodologyExecution,
    
    /// Data transfer and file operations
    DataTransfer,
    
    /// Security-related messages (authentication, authorization)
    Security,
    
    /// Configuration updates and system changes
    Configuration,
    
    /// Monitoring and metrics reporting
    Monitoring,
    
    /// Error notifications and fault reporting
    ErrorNotification,
    
    /// Heartbeat messages for connection monitoring
    Heartbeat,
    
    /// Shutdown and maintenance notifications
    Lifecycle,
    
    /// Broadcast messages to multiple components
    Broadcast,
    
    /// Generic response messages
    Response,
    
    /// Acknowledgment messages for reliable delivery
    Acknowledgment,
    
    /// Custom message types for future extensions
    Custom(String),
}

impl fmt::Display for EcosystemMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EcosystemMessageType::ComponentRegistration => write!(f, "ComponentRegistration"),
            EcosystemMessageType::ComponentStatus => write!(f, "ComponentStatus"),
            EcosystemMessageType::HealthCheck => write!(f, "HealthCheck"),
            EcosystemMessageType::TaskCoordination => write!(f, "TaskCoordination"),
            EcosystemMessageType::MethodologyExecution => write!(f, "MethodologyExecution"),
            EcosystemMessageType::DataTransfer => write!(f, "DataTransfer"),
            EcosystemMessageType::Security => write!(f, "Security"),
            EcosystemMessageType::Configuration => write!(f, "Configuration"),
            EcosystemMessageType::Monitoring => write!(f, "Monitoring"),
            EcosystemMessageType::ErrorNotification => write!(f, "ErrorNotification"),
            EcosystemMessageType::Heartbeat => write!(f, "Heartbeat"),
            EcosystemMessageType::Lifecycle => write!(f, "Lifecycle"),
            EcosystemMessageType::Broadcast => write!(f, "Broadcast"),
            EcosystemMessageType::Response => write!(f, "Response"),
            EcosystemMessageType::Acknowledgment => write!(f, "Acknowledgment"),
            EcosystemMessageType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

/// Represents the identity of a component in the ecosystem.
/// 
/// Component identity is crucial for conscious orchestration because OZONE STUDIO
/// needs to understand not just what components exist, but their roles, capabilities,
/// and current state in the ecosystem coordination hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ComponentIdentity {
    /// Unique identifier for this component instance
    pub component_id: String,
    
    /// The type of component (determines its role in orchestration)
    pub component_type: ComponentType,
    
    /// Optional instance identifier for multi-instance components
    pub instance_id: Option<String>,
}

impl ComponentIdentity {
    /// Creates a new component identity
    pub fn new(component_id: String, component_type: ComponentType) -> Self {
        Self {
            component_id,
            component_type,
            instance_id: None,
        }
    }
    
    /// Creates a component identity with an instance ID for multi-instance scenarios
    pub fn with_instance(component_id: String, component_type: ComponentType, instance_id: String) -> Self {
        Self {
            component_id,
            component_type,
            instance_id: Some(instance_id),
        }
    }
    
    /// Gets the full identifier including instance if present
    pub fn full_id(&self) -> String {
        match &self.instance_id {
            Some(instance) => format!("{}:{}", self.component_id, instance),
            None => self.component_id.clone(),
        }
    }
    
    /// Checks if this component is a foundational component (required for bootstrap)
    pub fn is_foundational(&self) -> bool {
        matches!(
            self.component_type,
            ComponentType::ConsciousOrchestrator
                | ComponentType::IntelligenceCoordinator
                | ComponentType::ConsciousnessArchitecture
                | ComponentType::UniversalAIEngine
        )
    }
    
    /// Checks if this component is a specialized AI App
    pub fn is_specialized_ai_app(&self) -> bool {
        matches!(
            self.component_type,
            ComponentType::CodeFrameworkSpecialist
                | ComponentType::TextFrameworkSpecialist
                | ComponentType::HumanInterface
        )
    }
}

impl fmt::Display for ComponentIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_id())
    }
}

/// Defines who should receive a message, supporting both direct and broadcast patterns.
/// 
/// The ecosystem needs flexible message routing to support different coordination patterns:
/// direct AI App coordination, ecosystem-wide broadcasts, and group communications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRecipient {
    /// Send to a specific component
    Component(ComponentIdentity),
    
    /// Send to all components of a specific type
    ComponentType(ComponentType),
    
    /// Send to all components in the ecosystem
    Broadcast,
    
    /// Send to a specific group of components
    Group(Vec<ComponentIdentity>),
    
    /// Send to components matching specific criteria
    Query(RecipientQuery),
}

impl MessageRecipient {
    /// Attempts to convert this recipient to a single component identity
    pub fn into_identity(self) -> Option<ComponentIdentity> {
        match self {
            MessageRecipient::Component(identity) => Some(identity),
            _ => None,
        }
    }
    
    /// Checks if this recipient includes the specified component
    pub fn includes_component(&self, component: &ComponentIdentity) -> bool {
        match self {
            MessageRecipient::Component(target) => target == component,
            MessageRecipient::ComponentType(component_type) => component.component_type == *component_type,
            MessageRecipient::Broadcast => true,
            MessageRecipient::Group(components) => components.contains(component),
            MessageRecipient::Query(query) => query.matches(component),
        }
    }
}

/// Criteria for selecting message recipients based on component properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipientQuery {
    /// Match components with these types
    pub component_types: Option<Vec<ComponentType>>,
    
    /// Match components with these capabilities
    pub required_capabilities: Option<Vec<String>>,
    
    /// Match components with this status
    pub status: Option<ComponentStatus>,
    
    /// Match components in these states
    pub health_states: Option<Vec<HealthState>>,
    
    /// Custom query parameters
    pub custom_criteria: HashMap<String, String>,
}

impl RecipientQuery {
    /// Checks if a component matches this query
    pub fn matches(&self, component: &ComponentIdentity) -> bool {
        // For now, only check component type
        // In a full implementation, this would query the component registry
        if let Some(types) = &self.component_types {
            types.contains(&component.component_type)
        } else {
            true
        }
    }
}

/// The actual message content, typed by the message purpose.
/// 
/// Using an enum for message payload allows type-safe message handling while
/// supporting the diverse communication needs of different ecosystem components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Component registration data
    ComponentRegistration(ComponentRegistration),
    
    /// Component status update
    ComponentStatus(ComponentStatus),
    
    /// Health check request or response
    HealthCheck(HealthCheck),
    
    /// Task coordination data
    TaskCoordination(TaskCoordinationPayload),
    
    /// Methodology execution information
    MethodologyExecution(MethodologyExecutionPayload),
    
    /// Data transfer information
    DataTransfer(DataTransferPayload),
    
    /// Security message data
    Security(SecurityPayload),
    
    /// Configuration update data
    Configuration(ConfigurationPayload),
    
    /// Monitoring and metrics data
    Monitoring(MonitoringPayload),
    
    /// Error information
    Error(EcosystemError),
    
    /// Simple heartbeat (no additional data)
    Heartbeat,
    
    /// Lifecycle event information
    Lifecycle(LifecyclePayload),
    
    /// Acknowledgment data
    Acknowledgment(AcknowledmentPayload),
    
    /// Generic text payload for simple messages
    Text(String),
    
    /// Generic JSON payload for custom messages
    Json(serde_json::Value),
    
    /// Binary data payload
    Binary(Vec<u8>),
}

impl MessagePayload {
    /// Gets a human-readable description of this payload type
    pub fn payload_type(&self) -> &'static str {
        match self {
            MessagePayload::ComponentRegistration(_) => "ComponentRegistration",
            MessagePayload::ComponentStatus(_) => "ComponentStatus",
            MessagePayload::HealthCheck(_) => "HealthCheck",
            MessagePayload::TaskCoordination(_) => "TaskCoordination",
            MessagePayload::MethodologyExecution(_) => "MethodologyExecution",
            MessagePayload::DataTransfer(_) => "DataTransfer",
            MessagePayload::Security(_) => "Security",
            MessagePayload::Configuration(_) => "Configuration",
            MessagePayload::Monitoring(_) => "Monitoring",
            MessagePayload::Error(_) => "Error",
            MessagePayload::Heartbeat => "Heartbeat",
            MessagePayload::Lifecycle(_) => "Lifecycle",
            MessagePayload::Acknowledgment(_) => "Acknowledgment",
            MessagePayload::Text(_) => "Text",
            MessagePayload::Json(_) => "Json",
            MessagePayload::Binary(_) => "Binary",
        }
    }
    
    /// Estimates the size of this payload in bytes
    pub fn estimated_size(&self) -> usize {
        match self {
            MessagePayload::Text(s) => s.len(),
            MessagePayload::Binary(data) => data.len(),
            MessagePayload::Json(value) => {
                // Rough estimate based on JSON serialization
                serde_json::to_string(value).map(|s| s.len()).unwrap_or(0)
            }
            _ => {
                // Estimate based on typical serialized size
                std::mem::size_of_val(self)
            }
        }
    }
}

/// Message priority levels that affect processing order.
/// 
/// Priority ensures that critical ecosystem messages (like security alerts or
/// component failures) are processed before routine operational messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Critical system messages that require immediate attention
    Critical,
    
    /// High priority messages for time-sensitive operations
    High,
    
    /// Normal priority for standard operations
    Normal,
    
    /// Low priority for background operations
    Low,
    
    /// Bulk operations that can be deferred
    Bulk,
}

impl MessagePriority {
    /// Gets the numeric value for priority ordering (lower number = higher priority)
    pub fn numeric_value(&self) -> u8 {
        match self {
            MessagePriority::Critical => 0,
            MessagePriority::High => 1,
            MessagePriority::Normal => 2,
            MessagePriority::Low => 3,
            MessagePriority::Bulk => 4,
        }
    }
    
    /// Creates a priority from a numeric value
    pub fn from_numeric(value: u8) -> Self {
        match value {
            0 => MessagePriority::Critical,
            1 => MessagePriority::High,
            2 => MessagePriority::Normal,
            3 => MessagePriority::Low,
            _ => MessagePriority::Bulk,
        }
    }
}

/// Security context for message authentication and authorization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSecurityContext {
    /// Authentication token or credential
    pub authentication_token: String,
    
    /// Required authorization level for processing
    pub authorization_level: AuthorizationLevel,
    
    /// Whether the message payload is encrypted
    pub encrypted: bool,
    
    /// Digital signature for message integrity
    pub signature: Option<String>,
    
    /// Security audit requirements
    pub audit_required: bool,
}

/// Authorization levels for message processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthorizationLevel {
    /// No special authorization required
    Public,
    
    /// Requires component authentication
    Authenticated,
    
    /// Requires elevated privileges
    Privileged,
    
    /// Requires system-level access
    System,
    
    /// Requires administrative access
    Administrative,
}

/// Routing information for message delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingInfo {
    /// Whether this message requires acknowledgment
    pub requires_acknowledgment: bool,
    
    /// Maximum delivery timeout
    pub delivery_timeout: Option<Duration>,
    
    /// Number of delivery attempts
    pub attempt_count: u32,
    
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    
    /// Whether this is a response message
    pub response_routing: bool,
    
    /// Routing hops for tracking message path
    pub hops: Vec<RoutingHop>,
    
    /// Quality of service requirements
    pub qos: QualityOfService,
}

impl RoutingInfo {
    /// Creates new routing info with default settings
    pub fn new() -> Self {
        Self {
            requires_acknowledgment: false,
            delivery_timeout: None,
            attempt_count: 0,
            max_attempts: 3,
            response_routing: false,
            hops: Vec::new(),
            qos: QualityOfService::BestEffort,
        }
    }
    
    /// Checks if this message can be retried
    pub fn can_retry(&self) -> bool {
        self.attempt_count < self.max_attempts
    }
    
    /// Increments the attempt count for retry handling
    pub fn increment_attempts(&mut self) {
        self.attempt_count += 1;
    }
}

impl Default for RoutingInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Records a routing hop for message tracing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingHop {
    /// Component that processed this message
    pub component: ComponentIdentity,
    
    /// When this hop was recorded
    pub timestamp: SystemTime,
}

/// Quality of service levels for message delivery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualityOfService {
    /// Best effort delivery (default)
    BestEffort,
    
    /// At least once delivery guarantee
    AtLeastOnce,
    
    /// Exactly once delivery guarantee
    ExactlyOnce,
    
    /// Ordered delivery guarantee
    Ordered,
    
    /// Real-time delivery requirements
    RealTime,
}

/// Metadata for message performance and diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// When the message was created
    pub created_at: SystemTime,
    
    /// When the message was last processed
    pub last_processed: Option<SystemTime>,
    
    /// Processing performance metrics
    pub performance_metrics: PerformanceMetrics,
    
    /// Diagnostic information
    pub diagnostics: DiagnosticInfo,
    
    /// Custom metadata fields
    pub custom_fields: HashMap<String, String>,
}

impl MessageMetadata {
    /// Creates new metadata with default values
    pub fn new() -> Self {
        Self {
            created_at: SystemTime::now(),
            last_processed: None,
            performance_metrics: PerformanceMetrics::new(),
            diagnostics: DiagnosticInfo::new(),
            custom_fields: HashMap::new(),
        }
    }
    
    /// Records that this message was processed
    pub fn mark_processed(&mut self) {
        self.last_processed = Some(SystemTime::now());
        self.performance_metrics.processing_count += 1;
    }
    
    /// Adds a custom metadata field
    pub fn add_custom_field(&mut self, key: String, value: String) {
        self.custom_fields.insert(key, value);
    }
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics for message processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Number of times this message has been processed
    pub processing_count: u32,
    
    /// Total processing time across all attempts
    pub total_processing_time: Duration,
    
    /// Network latency measurements
    pub network_latency: Option<Duration>,
    
    /// Serialization/deserialization time
    pub serialization_time: Option<Duration>,
    
    /// Queue wait time
    pub queue_wait_time: Option<Duration>,
}

impl PerformanceMetrics {
    /// Creates new performance metrics
    pub fn new() -> Self {
        Self {
            processing_count: 0,
            total_processing_time: Duration::from_secs(0),
            network_latency: None,
            serialization_time: None,
            queue_wait_time: None,
        }
    }
    
    /// Records processing time for this message
    pub fn record_processing_time(&mut self, duration: Duration) {
        self.total_processing_time += duration;
    }
    
    /// Gets average processing time per attempt
    pub fn average_processing_time(&self) -> Duration {
        if self.processing_count > 0 {
            self.total_processing_time / self.processing_count
        } else {
            Duration::from_secs(0)
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Diagnostic information for troubleshooting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    /// Warning messages
    pub warnings: Vec<String>,
    
    /// Error messages encountered during processing
    pub errors: Vec<String>,
    
    /// Debug information
    pub debug_info: HashMap<String, String>,
    
    /// Trace information for detailed debugging
    pub trace_info: Vec<TraceEntry>,
}

impl DiagnosticInfo {
    /// Creates new diagnostic info
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
            errors: Vec::new(),
            debug_info: HashMap::new(),
            trace_info: Vec::new(),
        }
    }
    
    /// Adds a warning message
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    /// Adds an error message
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
    
    /// Adds debug information
    pub fn add_debug_info(&mut self, key: String, value: String) {
        self.debug_info.insert(key, value);
    }
    
    /// Adds a trace entry
    pub fn add_trace(&mut self, component: String, message: String) {
        self.trace_info.push(TraceEntry {
            component,
            message,
            timestamp: SystemTime::now(),
        });
    }
}

impl Default for DiagnosticInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// A single trace entry for detailed debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    /// Component that generated this trace
    pub component: String,
    
    /// Trace message
    pub message: String,
    
    /// When this trace was generated
    pub timestamp: SystemTime,
}

/// Component registration information sent when AI Apps join the ecosystem.
/// 
/// Registration is the first step in conscious orchestration - OZONE STUDIO needs
/// to understand what capabilities are available before it can coordinate them effectively.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRegistration {
    /// The component's identity
    pub identity: ComponentIdentity,
    
    /// Component version information
    pub version: String,
    
    /// Detailed capability descriptions
    pub capabilities: Vec<ComponentCapability>,
    
    /// Network endpoints for communication
    pub endpoints: Vec<Endpoint>,
    
    /// Component configuration information
    pub configuration: ComponentConfiguration,
    
    /// Service level requirements and guarantees
    pub service_level: ServiceLevel,
    
    /// Bootstrap integration requirements
    pub bootstrap_integration: BootstrapIntegration,
    
    /// Health check configuration
    pub health_check_config: HealthCheckConfig,
    
    /// Security requirements
    pub security_requirements: SecurityRequirements,
    
    /// Resource requirements and constraints
    pub resource_requirements: ResourceRequirements,
    
    /// Dependencies on other components
    pub dependencies: Vec<ComponentDependency>,
}

impl ComponentRegistration {
    /// Creates a new component registration
    pub fn new(identity: ComponentIdentity, version: String) -> Self {
        Self {
            identity,
            version,
            capabilities: Vec::new(),
            endpoints: Vec::new(),
            configuration: ComponentConfiguration::default(),
            service_level: ServiceLevel::Standard,
            bootstrap_integration: BootstrapIntegration::Optional,
            health_check_config: HealthCheckConfig::default(),
            security_requirements: SecurityRequirements::default(),
            resource_requirements: ResourceRequirements::default(),
            dependencies: Vec::new(),
        }
    }
    
    /// Adds a capability to this registration
    pub fn add_capability(mut self, capability: ComponentCapability) -> Self {
        self.capabilities.push(capability);
        self
    }
    
    /// Adds an endpoint to this registration
    pub fn add_endpoint(mut self, endpoint: Endpoint) -> Self {
        self.endpoints.push(endpoint);
        self
    }
    
    /// Sets the service level for this component
    pub fn with_service_level(mut self, service_level: ServiceLevel) -> Self {
        self.service_level = service_level;
        self
    }
    
    /// Sets the bootstrap integration level
    pub fn with_bootstrap_integration(mut self, bootstrap_integration: BootstrapIntegration) -> Self {
        self.bootstrap_integration = bootstrap_integration;
        self
    }
    
    /// Checks if this component is required for ecosystem bootstrap
    pub fn is_bootstrap_required(&self) -> bool {
        matches!(self.bootstrap_integration, BootstrapIntegration::Essential | BootstrapIntegration::Required)
    }
    
    /// Gets all capabilities of a specific type
    pub fn capabilities_of_type(&self, capability_type: &str) -> Vec<&ComponentCapability> {
        self.capabilities
            .iter()
            .filter(|cap| cap.capability_type == capability_type)
            .collect()
    }
}

/// Describes a specific capability that a component provides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentCapability {
    /// Unique name for this capability
    pub name: String,
    
    /// Type/category of capability
    pub capability_type: String,
    
    /// Human-readable description
    pub description: String,
    
    /// Version of this capability
    pub version: String,
    
    /// Parameters this capability accepts
    pub parameters: Vec<CapabilityParameter>,
    
    /// Expected outputs from this capability
    pub outputs: Vec<CapabilityOutput>,
    
    /// Performance characteristics
    pub performance: CapabilityPerformance,
    
    /// Resource requirements for this capability
    pub resource_requirements: ResourceRequirements,
    
    /// Whether this capability is currently available
    pub available: bool,
}

impl ComponentCapability {
    /// Creates a new capability description
    pub fn new(name: String, capability_type: String, description: String) -> Self {
        Self {
            name,
            capability_type,
            description,
            version: "1.0.0".to_string(),
            parameters: Vec::new(),
            outputs: Vec::new(),
            performance: CapabilityPerformance::default(),
            resource_requirements: ResourceRequirements::default(),
            available: true,
        }
    }
    
    /// Adds a parameter to this capability
    pub fn add_parameter(mut self, parameter: CapabilityParameter) -> Self {
        self.parameters.push(parameter);
        self
    }
    
    /// Adds an output to this capability
    pub fn add_output(mut self, output: CapabilityOutput) -> Self {
        self.outputs.push(output);
        self
    }
    
    /// Checks if this capability accepts a specific parameter
    pub fn accepts_parameter(&self, parameter_name: &str) -> bool {
        self.parameters.iter().any(|p| p.name == parameter_name)
    }
}

/// Describes a parameter that a capability accepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityParameter {
    /// Parameter name
    pub name: String,
    
    /// Parameter data type
    pub parameter_type: String,
    
    /// Whether this parameter is required
    pub required: bool,
    
    /// Default value if not provided
    pub default_value: Option<String>,
    
    /// Parameter description
    pub description: String,
    
    /// Validation constraints
    pub constraints: Vec<ParameterConstraint>,
}

/// Describes an output that a capability produces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityOutput {
    /// Output name
    pub name: String,
    
    /// Output data type
    pub output_type: String,
    
    /// Output description
    pub description: String,
    
    /// Whether this output is always produced
    pub guaranteed: bool,
}

/// Performance characteristics of a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityPerformance {
    /// Expected response time
    pub expected_response_time: Duration,
    
    /// Maximum response time
    pub max_response_time: Duration,
    
    /// Throughput capacity (operations per second)
    pub throughput: f64,
    
    /// Reliability percentage (0.0 to 1.0)
    pub reliability: f64,
    
    /// Scalability characteristics
    pub scalability: ScalabilityCharacteristics,
}

impl Default for CapabilityPerformance {
    fn default() -> Self {
        Self {
            expected_response_time: Duration::from_millis(100),
            max_response_time: Duration::from_secs(5),
            throughput: 1.0,
            reliability: 0.99,
            scalability: ScalabilityCharacteristics::default(),
        }
    }
}

/// Scalability characteristics for performance modeling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalabilityCharacteristics {
    /// Can this capability scale horizontally
    pub horizontal_scaling: bool,
    
    /// Can this capability scale vertically
    pub vertical_scaling: bool,
    
    /// Maximum concurrent operations
    pub max_concurrent: Option<u32>,
    
    /// Resource scaling factor
    pub scaling_factor: f64,
}

impl Default for ScalabilityCharacteristics {
    fn default() -> Self {
        Self {
            horizontal_scaling: false,
            vertical_scaling: true,
            max_concurrent: None,
            scaling_factor: 1.0,
        }
    }
}

/// Validation constraints for capability parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConstraint {
    /// Type of constraint
    pub constraint_type: String,
    
    /// Constraint value
    pub value: String,
    
    /// Error message if constraint is violated
    pub error_message: String,
}

/// Component configuration information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfiguration {
    /// Configuration parameters
    pub parameters: HashMap<String, String>,
    
    /// Feature flags
    pub features: Vec<String>,
    
    /// Environment-specific settings
    pub environment: String,
    
    /// Logging configuration
    pub logging: LoggingConfiguration,
    
    /// Monitoring configuration
    pub monitoring: MonitoringConfiguration,
}

impl Default for ComponentConfiguration {
    fn default() -> Self {
        Self {
            parameters: HashMap::new(),
            features: Vec::new(),
            environment: "production".to_string(),
            logging: LoggingConfiguration::default(),
            monitoring: MonitoringConfiguration::default(),
        }
    }
}

/// Logging configuration for components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfiguration {
    /// Log level
    pub level: String,
    
    /// Whether to log to console
    pub console: bool,
    
    /// Whether to log to file
    pub file: bool,
    
    /// Log file path
    pub file_path: Option<String>,
    
    /// Whether to include structured logging
    pub structured: bool,
}

impl Default for LoggingConfiguration {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            console: true,
            file: false,
            file_path: None,
            structured: true,
        }
    }
}

/// Monitoring configuration for components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfiguration {
    /// Whether monitoring is enabled
    pub enabled: bool,
    
    /// Metrics collection interval
    pub metrics_interval: Duration,
    
    /// Whether to collect performance metrics
    pub performance_metrics: bool,
    
    /// Whether to collect resource metrics
    pub resource_metrics: bool,
    
    /// Custom metrics to collect
    pub custom_metrics: Vec<String>,
}

impl Default for MonitoringConfiguration {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: Duration::from_secs(60),
            performance_metrics: true,
            resource_metrics: true,
            custom_metrics: Vec::new(),
        }
    }
}

/// Service level classification for components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceLevel {
    /// Foundational services required for ecosystem operation
    Foundational,
    
    /// Standard specialized services
    Standard,
    
    /// Enhanced services with additional capabilities
    Enhanced,
    
    /// Experimental or beta services
    Experimental,
}

/// Bootstrap integration requirements for components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BootstrapIntegration {
    /// Essential for bootstrap - system cannot start without this
    Essential,
    
    /// Required during normal bootstrap
    Required,
    
    /// Standard integration during bootstrap
    Standard,
    
    /// Optional - can be added after bootstrap
    Optional,
}

/// Health check configuration for components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Whether health checks are enabled
    pub enabled: bool,
    
    /// Health check interval
    pub interval: Duration,
    
    /// Health check timeout
    pub timeout: Duration,
    
    /// Health check endpoint path
    pub endpoint: String,
    
    /// Custom health check parameters
    pub parameters: HashMap<String, String>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            endpoint: "/health".to_string(),
            parameters: HashMap::new(),
        }
    }
}

/// Security requirements for components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRequirements {
    /// Whether authentication is required
    pub authentication_required: bool,
    
    /// Required authentication methods
    pub authentication_methods: Vec<String>,
    
    /// Whether authorization is required
    pub authorization_required: bool,
    
    /// Whether encryption is required
    pub encryption_required: bool,
    
    /// Whether audit logging is required
    pub audit_logging: bool,
    
    /// Security level requirements
    pub security_level: SecurityLevel,
}

impl Default for SecurityRequirements {
    fn default() -> Self {
        Self {
            authentication_required: true,
            authentication_methods: vec!["certificate".to_string()],
            authorization_required: true,
            encryption_required: true,
            audit_logging: true,
            security_level: SecurityLevel::Standard,
        }
    }
}

/// Security levels for components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Basic security requirements
    Basic,
    
    /// Standard security requirements
    Standard,
    
    /// High security requirements
    High,
    
    /// Maximum security requirements
    Maximum,
}

/// Resource requirements and constraints for components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU requirements
    pub cpu: CpuRequirements,
    
    /// Memory requirements
    pub memory: MemoryRequirements,
    
    /// Storage requirements
    pub storage: StorageRequirements,
    
    /// Network requirements
    pub network: NetworkRequirements,
    
    /// GPU requirements (optional)
    pub gpu: Option<GpuRequirements>,
    
    /// Special hardware requirements
    pub special_hardware: Vec<String>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu: CpuRequirements::default(),
            memory: MemoryRequirements::default(),
            storage: StorageRequirements::default(),
            network: NetworkRequirements::default(),
            gpu: None,
            special_hardware: Vec::new(),
        }
    }
}

/// CPU resource requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    /// Minimum CPU cores required
    pub min_cores: u32,
    
    /// Recommended CPU cores
    pub recommended_cores: u32,
    
    /// Minimum CPU frequency (GHz)
    pub min_frequency: f64,
    
    /// CPU architecture requirements
    pub architecture: Vec<String>,
}

impl Default for CpuRequirements {
    fn default() -> Self {
        Self {
            min_cores: 1,
            recommended_cores: 2,
            min_frequency: 1.0,
            architecture: vec!["x86_64".to_string()],
        }
    }
}

/// Memory resource requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    /// Minimum RAM in MB
    pub min_ram_mb: u64,
    
    /// Recommended RAM in MB
    pub recommended_ram_mb: u64,
    
    /// Whether swap is acceptable
    pub swap_acceptable: bool,
    
    /// Memory access patterns
    pub access_patterns: Vec<String>,
}

impl Default for MemoryRequirements {
    fn default() -> Self {
        Self {
            min_ram_mb: 512,
            recommended_ram_mb: 1024,
            swap_acceptable: true,
            access_patterns: vec!["sequential".to_string()],
        }
    }
}

/// Storage resource requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Minimum storage space in MB
    pub min_space_mb: u64,
    
    /// Recommended storage space in MB
    pub recommended_space_mb: u64,
    
    /// Required storage type
    pub storage_type: Vec<String>,
    
    /// Whether persistent storage is required
    pub persistent: bool,
    
    /// Required IOPS
    pub min_iops: Option<u32>,
}

impl Default for StorageRequirements {
    fn default() -> Self {
        Self {
            min_space_mb: 100,
            recommended_space_mb: 1000,
            storage_type: vec!["ssd".to_string(), "hdd".to_string()],
            persistent: true,
            min_iops: None,
        }
    }
}

/// Network resource requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    /// Minimum bandwidth in Mbps
    pub min_bandwidth_mbps: u32,
    
    /// Maximum acceptable latency in ms
    pub max_latency_ms: u32,
    
    /// Required protocols
    pub protocols: Vec<String>,
    
    /// Whether internet access is required
    pub internet_required: bool,
    
    /// Network security requirements
    pub security_requirements: Vec<String>,
}

impl Default for NetworkRequirements {
    fn default() -> Self {
        Self {
            min_bandwidth_mbps: 10,
            max_latency_ms: 100,
            protocols: vec!["TCP".to_string(), "HTTP".to_string()],
            internet_required: false,
            security_requirements: vec!["TLS".to_string()],
        }
    }
}

/// GPU resource requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Minimum GPU memory in MB
    pub min_memory_mb: u64,
    
    /// Required GPU capabilities
    pub capabilities: Vec<String>,
    
    /// Supported GPU vendors
    pub vendors: Vec<String>,
    
    /// Minimum compute capability
    pub min_compute_capability: Option<String>,
}

/// Component dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDependency {
    /// Component type this depends on
    pub component_type: ComponentType,
    
    /// Specific component ID (optional)
    pub component_id: Option<String>,
    
    /// Required capabilities from the dependency
    pub required_capabilities: Vec<String>,
    
    /// Whether this is a hard dependency (required for operation)
    pub hard_dependency: bool,
    
    /// Minimum version required
    pub min_version: Option<String>,
}

/// Current status of a component in the ecosystem.
/// 
/// Status information helps OZONE STUDIO make informed coordination decisions
/// by understanding the current operational state of each component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    /// Component identity
    pub component: ComponentIdentity,
    
    /// Current operational status
    pub status: OperationalStatus,
    
    /// Current health state
    pub health: HealthState,
    
    /// When this status was last updated
    pub last_updated: SystemTime,
    
    /// Current resource utilization
    pub resource_utilization: ResourceUtilization,
    
    /// Active capabilities (subset of total capabilities)
    pub active_capabilities: Vec<String>,
    
    /// Current load metrics
    pub load_metrics: LoadMetrics,
    
    /// Error information if in error state
    pub error_info: Option<ErrorInfo>,
    
    /// Performance metrics
    pub performance_metrics: ComponentPerformanceMetrics,
    
    /// Current configuration version
    pub config_version: String,
}

impl ComponentStatus {
    /// Creates a new component status
    pub fn new(component: ComponentIdentity, status: OperationalStatus) -> Self {
        Self {
            component,
            status,
            health: HealthState::Healthy,
            last_updated: SystemTime::now(),
            resource_utilization: ResourceUtilization::default(),
            active_capabilities: Vec::new(),
            load_metrics: LoadMetrics::default(),
            error_info: None,
            performance_metrics: ComponentPerformanceMetrics::default(),
            config_version: "1.0.0".to_string(),
        }
    }
    
    /// Checks if this component is available for coordination
    pub fn is_available(&self) -> bool {
        matches!(self.status, OperationalStatus::Running | OperationalStatus::Idle) &&
        matches!(self.health, HealthState::Healthy | HealthState::Warning)
    }
    
    /// Checks if this component is in a healthy state
    pub fn is_healthy(&self) -> bool {
        matches!(self.health, HealthState::Healthy | HealthState::Warning)
    }
    
    /// Gets the age of this status update
    pub fn status_age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.last_updated)
            .unwrap_or_else(|_| Duration::from_secs(0))
    }
    
    /// Checks if this status is stale (older than threshold)
    pub fn is_stale(&self, threshold: Duration) -> bool {
        self.status_age() > threshold
    }
}

/// Operational status of a component.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationalStatus {
    /// Component is starting up
    Starting,
    
    /// Component is running and available
    Running,
    
    /// Component is idle but available
    Idle,
    
    /// Component is busy processing
    Busy,
    
    /// Component is in maintenance mode
    Maintenance,
    
    /// Component is shutting down
    Shutting Down,
    
    /// Component is stopped
    Stopped,
    
    /// Component is in error state
    Error,
    
    /// Component status is unknown
    Unknown,
}

impl fmt::Display for OperationalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationalStatus::Starting => write!(f, "Starting"),
            OperationalStatus::Running => write!(f, "Running"),
            OperationalStatus::Idle => write!(f, "Idle"),
            OperationalStatus::Busy => write!(f, "Busy"),
            OperationalStatus::Maintenance => write!(f, "Maintenance"),
            OperationalStatus::ShuttingDown => write!(f, "Shutting Down"),
            OperationalStatus::Stopped => write!(f, "Stopped"),
            OperationalStatus::Error => write!(f, "Error"),
            OperationalStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Health state of a component.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthState {
    /// Component is healthy
    Healthy,
    
    /// Component has warnings but is operational
    Warning,
    
    /// Component is degraded but partially functional
    Degraded,
    
    /// Component is critical but still running
    Critical,
    
    /// Component has failed
    Failed,
    
    /// Component health is unknown
    Unknown,
}

impl fmt::Display for HealthState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthState::Healthy => write!(f, "Healthy"),
            HealthState::Warning => write!(f, "Warning"),
            HealthState::Degraded => write!(f, "Degraded"),
            HealthState::Critical => write!(f, "Critical"),
            HealthState::Failed => write!(f, "Failed"),
            HealthState::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Resource utilization metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// CPU utilization percentage (0.0 to 1.0)
    pub cpu: f64,
    
    /// Memory utilization percentage (0.0 to 1.0)
    pub memory: f64,
    
    /// Storage utilization percentage (0.0 to 1.0)
    pub storage: f64,
    
    /// Network utilization percentage (0.0 to 1.0)
    pub network: f64,
    
    /// GPU utilization percentage (0.0 to 1.0, if applicable)
    pub gpu: Option<f64>,
    
    /// Custom resource utilization metrics
    pub custom: HashMap<String, f64>,
}

impl Default for ResourceUtilization {
    fn default() -> Self {
        Self {
            cpu: 0.0,
            memory: 0.0,
            storage: 0.0,
            network: 0.0,
            gpu: None,
            custom: HashMap::new(),
        }
    }
}

impl ResourceUtilization {
    /// Calculates overall utilization score
    pub fn overall_utilization(&self) -> f64 {
        let mut total = self.cpu + self.memory + self.storage + self.network;
        let mut count = 4.0;
        
        if let Some(gpu) = self.gpu {
            total += gpu;
            count += 1.0;
        }
        
        total / count
    }
    
    /// Checks if any resource is over the threshold
    pub fn is_over_threshold(&self, threshold: f64) -> bool {
        self.cpu > threshold ||
        self.memory > threshold ||
        self.storage > threshold ||
        self.network > threshold ||
        self.gpu.map_or(false, |gpu| gpu > threshold)
    }
}

/// Load metrics for component performance monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMetrics {
    /// Current number of active requests
    pub active_requests: u32,
    
    /// Requests per second
    pub requests_per_second: f64,
    
    /// Average response time in milliseconds
    pub average_response_time_ms: f64,
    
    /// Error rate percentage (0.0 to 1.0)
    pub error_rate: f64,
    
    /// Queue depth
    pub queue_depth: u32,
    
    /// Throughput metrics
    pub throughput: ThroughputMetrics,
}

impl Default for LoadMetrics {
    fn default() -> Self {
        Self {
            active_requests: 0,
            requests_per_second: 0.0,
            average_response_time_ms: 0.0,
            error_rate: 0.0,
            queue_depth: 0,
            throughput: ThroughputMetrics::default(),
        }
    }
}

/// Throughput metrics for performance monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// Bytes per second processed
    pub bytes_per_second: u64,
    
    /// Operations per second
    pub operations_per_second: f64,
    
    /// Transactions per second
    pub transactions_per_second: f64,
    
    /// Peak throughput achieved
    pub peak_throughput: f64,
}

impl Default for ThroughputMetrics {
    fn default() -> Self {
        Self {
            bytes_per_second: 0,
            operations_per_second: 0.0,
            transactions_per_second: 0.0,
            peak_throughput: 0.0,
        }
    }
}

/// Error information for failed components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Error code
    pub error_code: String,
    
    /// Error message
    pub error_message: String,
    
    /// When the error occurred
    pub occurred_at: SystemTime,
    
    /// Stack trace or detailed error information
    pub details: String,
    
    /// Whether this error is recoverable
    pub recoverable: bool,
    
    /// Suggested recovery actions
    pub recovery_actions: Vec<String>,
}

/// Performance metrics for component monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPerformanceMetrics {
    /// Uptime in seconds
    pub uptime_seconds: u64,
    
    /// Total requests processed
    pub total_requests: u64,
    
    /// Total errors encountered
    pub total_errors: u64,
    
    /// Average processing time
    pub average_processing_time: Duration,
    
    /// Peak memory usage in MB
    pub peak_memory_mb: u64,
    
    /// Custom performance metrics
    pub custom_metrics: HashMap<String, f64>,
}

impl Default for ComponentPerformanceMetrics {
    fn default() -> Self {
        Self {
            uptime_seconds: 0,
            total_requests: 0,
            total_errors: 0,
            average_processing_time: Duration::from_millis(0),
            peak_memory_mb: 0,
            custom_metrics: HashMap::new(),
        }
    }
}

/// Health check request and response structures.
/// 
/// Health checks are essential for conscious orchestration because OZONE STUDIO
/// needs to understand the real-time health of all components to make informed
/// coordination decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Type of health check
    pub check_type: HealthCheckType,
    
    /// Component being checked
    pub component: ComponentIdentity,
    
    /// When the check was initiated
    pub initiated_at: SystemTime,
    
    /// Timeout for the health check
    pub timeout: Duration,
    
    /// Specific checks to perform
    pub checks: Vec<SpecificHealthCheck>,
    
    /// Health check response (if this is a response)
    pub response: Option<HealthCheckResponse>,
}

impl HealthCheck {
    /// Creates a new health check request
    pub fn new_request(component: ComponentIdentity, check_type: HealthCheckType) -> Self {
        Self {
            check_type,
            component,
            initiated_at: SystemTime::now(),
            timeout: Duration::from_secs(5),
            checks: Vec::new(),
            response: None,
        }
    }
    
    /// Creates a health check response
    pub fn new_response(
        component: ComponentIdentity,
        check_type: HealthCheckType,
        response: HealthCheckResponse,
    ) -> Self {
        Self {
            check_type,
            component,
            initiated_at: SystemTime::now(),
            timeout: Duration::from_secs(5),
            checks: Vec::new(),
            response: Some(response),
        }
    }
    
    /// Adds a specific check to perform
    pub fn add_check(mut self, check: SpecificHealthCheck) -> Self {
        self.checks.push(check);
        self
    }
    
    /// Checks if this health check has timed out
    pub fn is_timed_out(&self) -> bool {
        SystemTime::now()
            .duration_since(self.initiated_at)
            .map(|duration| duration > self.timeout)
            .unwrap_or(false)
    }
}

/// Types of health checks that can be performed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthCheckType {
    /// Basic availability check
    Basic,
    
    /// Detailed health assessment
    Detailed,
    
    /// Performance benchmarking
    Performance,
    
    /// Capability verification
    Capability,
    
    /// Resource availability check
    Resource,
    
    /// Security validation
    Security,
    
    /// Configuration validation
    Configuration,
    
    /// Custom health check
    Custom(String),
}

/// Specific health checks to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificHealthCheck {
    /// Name of the check
    pub name: String,
    
    /// Type of check
    pub check_type: String,
    
    /// Parameters for the check
    pub parameters: HashMap<String, String>,
    
    /// Expected result
    pub expected_result: Option<String>,
    
    /// Timeout for this specific check
    pub timeout: Duration,
}

/// Response to a health check request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Overall health status
    pub overall_status: HealthState,
    
    /// When the response was generated
    pub response_time: SystemTime,
    
    /// Duration to generate this response
    pub response_duration: Duration,
    
    /// Results of individual checks
    pub check_results: Vec<HealthCheckResult>,
    
    /// Current component status
    pub component_status: ComponentStatus,
    
    /// Additional diagnostic information
    pub diagnostics: HealthDiagnostics,
    
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

impl HealthCheckResponse {
    /// Creates a healthy response
    pub fn healthy(component: ComponentIdentity) -> Self {
        Self {
            overall_status: HealthState::Healthy,
            response_time: SystemTime::now(),
            response_duration: Duration::from_millis(1),
            check_results: Vec::new(),
            component_status: ComponentStatus::new(component, OperationalStatus::Running),
            diagnostics: HealthDiagnostics::new(),
            recommendations: Vec::new(),
        }
    }
    
    /// Creates an unhealthy response with error information
    pub fn unhealthy(component: ComponentIdentity, error: String) -> Self {
        let mut response = Self::healthy(component.clone());
        response.overall_status = HealthState::Failed;
        response.component_status = ComponentStatus::new(component, OperationalStatus::Error);
        response.diagnostics.errors.push(error);
        response
    }
    
    /// Adds a check result
    pub fn add_check_result(mut self, result: HealthCheckResult) -> Self {
        // Update overall status based on check results
        if result.status != HealthState::Healthy && result.status != HealthState::Warning {
            if self.overall_status == HealthState::Healthy {
                self.overall_status = result.status.clone();
            }
        }
        
        self.check_results.push(result);
        self
    }
    
    /// Checks if all checks passed
    pub fn all_checks_passed(&self) -> bool {
        self.check_results.iter().all(|result| {
            matches!(result.status, HealthState::Healthy | HealthState::Warning)
        })
    }
}

/// Result of a specific health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Name of the check that was performed
    pub check_name: String,
    
    /// Status result of the check
    pub status: HealthState,
    
    /// Duration to perform the check
    pub duration: Duration,
    
    /// Result message
    pub message: String,
    
    /// Actual result value
    pub result_value: Option<String>,
    
    /// Expected result value
    pub expected_value: Option<String>,
    
    /// Additional details
    pub details: HashMap<String, String>,
}

/// Diagnostic information from health checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDiagnostics {
    /// Warning messages
    pub warnings: Vec<String>,
    
    /// Error messages
    pub errors: Vec<String>,
    
    /// Informational messages
    pub info: Vec<String>,
    
    /// System metrics at time of check
    pub system_metrics: SystemMetrics,
    
    /// Service-specific diagnostics
    pub service_diagnostics: HashMap<String, String>,
}

impl HealthDiagnostics {
    /// Creates new health diagnostics
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
            errors: Vec::new(),
            info: Vec::new(),
            system_metrics: SystemMetrics::default(),
            service_diagnostics: HashMap::new(),
        }
    }
    
    /// Adds a warning message
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    /// Adds an error message
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
    
    /// Adds an info message
    pub fn add_info(&mut self, info: String) {
        self.info.push(info);
    }
}

impl Default for HealthDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

/// System metrics for health diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    
    /// Memory usage percentage
    pub memory_usage: f64,
    
    /// Disk usage percentage
    pub disk_usage: f64,
    
    /// Network utilization
    pub network_usage: f64,
    
    /// Load averages
    pub load_averages: Vec<f64>,
    
    /// Process count
    pub process_count: u32,
    
    /// Open file descriptors
    pub open_files: u32,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
            network_usage: 0.0,
            load_averages: Vec::new(),
            process_count: 0,
            open_files: 0,
        }
    }
}

/// Response message for ecosystem communications.
/// 
/// Standard response format ensures consistent communication patterns across
/// the ecosystem and enables proper error handling and status tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemResponse {
    /// Unique response identifier
    pub response_id: String,
    
    /// ID of the original request message
    pub request_id: String,
    
    /// Response status
    pub status: ResponseStatus,
    
    /// Component that generated this response
    pub responder: ComponentIdentity,
    
    /// When the response was generated
    pub timestamp: SystemTime,
    
    /// Response payload
    pub payload: Option<MessagePayload>,
    
    /// Error information if status indicates failure
    pub error: Option<EcosystemError>,
    
    /// Processing metrics
    pub processing_metrics: ResponseProcessingMetrics,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl EcosystemResponse {
    /// Creates a successful response
    pub fn success(
        request_id: String,
        responder: ComponentIdentity,
        payload: Option<MessagePayload>,
    ) -> Self {
        Self {
            response_id: Uuid::new_v4().to_string(),
            request_id,
            status: ResponseStatus::Success,
            responder,
            timestamp: SystemTime::now(),
            payload,
            error: None,
            processing_metrics: ResponseProcessingMetrics::default(),
            metadata: HashMap::new(),
        }
    }
    
    /// Creates an error response
    pub fn error(
        request_id: String,
        responder: ComponentIdentity,
        error: EcosystemError,
    ) -> Self {
        Self {
            response_id: Uuid::new_v4().to_string(),
            request_id,
            status: ResponseStatus::Error,
            responder,
            timestamp: SystemTime::now(),
            payload: None,
            error: Some(error),
            processing_metrics: ResponseProcessingMetrics::default(),
            metadata: HashMap::new(),
        }
    }
    
    /// Adds metadata to the response
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Sets processing metrics
    pub fn with_processing_metrics(mut self, metrics: ResponseProcessingMetrics) -> Self {
        self.processing_metrics = metrics;
        self
    }
}

/// Status of an ecosystem response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResponseStatus {
    /// Request completed successfully
    Success,
    
    /// Request completed with warnings
    Warning,
    
    /// Request failed with error
    Error,
    
    /// Request is still being processed
    Processing,
    
    /// Request was rejected
    Rejected,
    
    /// Request timed out
    Timeout,
    
    /// Request was cancelled
    Cancelled,
}

impl fmt::Display for ResponseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseStatus::Success => write!(f, "Success"),
            ResponseStatus::Warning => write!(f, "Warning"),
            ResponseStatus::Error => write!(f, "Error"),
            ResponseStatus::Processing => write!(f, "Processing"),
            ResponseStatus::Rejected => write!(f, "Rejected"),
            ResponseStatus::Timeout => write!(f, "Timeout"),
            ResponseStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Processing metrics for response analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseProcessingMetrics {
    /// Time spent processing the request
    pub processing_time: Duration,
    
    /// Time spent waiting in queue
    pub queue_time: Duration,
    
    /// Time spent on I/O operations
    pub io_time: Duration,
    
    /// Number of processing steps
    pub processing_steps: u32,
    
    /// Resources consumed during processing
    pub resource_consumption: ResourceConsumption,
}

impl Default for ResponseProcessingMetrics {
    fn default() -> Self {
        Self {
            processing_time: Duration::from_millis(0),
            queue_time: Duration::from_millis(0),
            io_time: Duration::from_millis(0),
            processing_steps: 0,
            resource_consumption: ResourceConsumption::default(),
        }
    }
}

/// Resource consumption metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConsumption {
    /// CPU time consumed
    pub cpu_time: Duration,
    
    /// Memory allocated in bytes
    pub memory_allocated: u64,
    
    /// Network bytes transferred
    pub network_bytes: u64,
    
    /// Disk I/O operations
    pub disk_operations: u32,
}

impl Default for ResourceConsumption {
    fn default() -> Self {
        Self {
            cpu_time: Duration::from_millis(0),
            memory_allocated: 0,
            network_bytes: 0,
            disk_operations: 0,
        }
    }
}

/// Ecosystem communication errors.
/// 
/// Comprehensive error types enable proper error handling and recovery
/// across the ecosystem communication layer.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum EcosystemError {
    #[error("Communication error: {message}")]
    CommunicationError { message: String },
    
    #[error("Component not found: {component_id}")]
    ComponentNotFound { component_id: String },
    
    #[error("Component unavailable: {component_id} - {reason}")]
    ComponentUnavailable { component_id: String, reason: String },
    
    #[error("Registration failed: {component_id} - {reason}")]
    RegistrationFailed { component_id: String, reason: String },
    
    #[error("Health check failed: {component_id} - {details}")]
    HealthCheckFailed { component_id: String, details: String },
    
    #[error("Message routing failed: {message_id} - {reason}")]
    MessageRoutingFailed { message_id: String, reason: String },
    
    #[error("Authentication failed: {component_id}")]
    AuthenticationFailed { component_id: String },
    
    #[error("Authorization denied: {component_id} - {operation}")]
    AuthorizationDenied { component_id: String, operation: String },
    
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolVersionMismatch { expected: String, actual: String },
    
    #[error("Message serialization error: {details}")]
    SerializationError { details: String },
    
    #[error("Message deserialization error: {details}")]
    DeserializationError { details: String },
    
    #[error("Timeout error: operation timed out after {duration:?}")]
    TimeoutError { duration: Duration },
    
    #[error("Resource exhaustion: {resource} - {details}")]
    ResourceExhaustion { resource: String, details: String },
    
    #[error("Configuration error: {component_id} - {details}")]
    ConfigurationError { component_id: String, details: String },
    
    #[error("Network error: {details}")]
    NetworkError { details: String },
    
    #[error("Security violation: {component_id} - {violation}")]
    SecurityViolation { component_id: String, violation: String },
    
    #[error("Invalid message format: {details}")]
    InvalidMessageFormat { details: String },
    
    #[error("Message too large: {size} bytes exceeds limit of {limit} bytes")]
    MessageTooLarge { size: usize, limit: usize },
    
    #[error("Component dependency error: {component_id} depends on unavailable {dependency}")]
    DependencyError { component_id: String, dependency: String },
    
    #[error("Service degradation: {component_id} - {details}")]
    ServiceDegradation { component_id: String, details: String },
    
    #[error("Internal error: {details}")]
    InternalError { details: String },
}

impl EcosystemError {
    /// Gets the error category for classification
    pub fn category(&self) -> ErrorCategory {
        match self {
            EcosystemError::CommunicationError { .. } |
            EcosystemError::MessageRoutingFailed { .. } |
            EcosystemError::NetworkError { .. } => ErrorCategory::Communication,
            
            EcosystemError::ComponentNotFound { .. } |
            EcosystemError::ComponentUnavailable { .. } |
            EcosystemError::RegistrationFailed { .. } => ErrorCategory::Component,
            
            EcosystemError::AuthenticationFailed { .. } |
            EcosystemError::AuthorizationDenied { .. } |
            EcosystemError::SecurityViolation { .. } => ErrorCategory::Security,
            
            EcosystemError::ConfigurationError { .. } |
            EcosystemError::ProtocolVersionMismatch { .. } => ErrorCategory::Configuration,
            
            EcosystemError::ResourceExhaustion { .. } |
            EcosystemError::ServiceDegradation { .. } => ErrorCategory::Resource,
            
            EcosystemError::SerializationError { .. } |
            EcosystemError::DeserializationError { .. } |
            EcosystemError::InvalidMessageFormat { .. } |
            EcosystemError::MessageTooLarge { .. } => ErrorCategory::Protocol,
            
            EcosystemError::TimeoutError { .. } => ErrorCategory::Timeout,
            
            EcosystemError::DependencyError { .. } => ErrorCategory::Dependency,
            
            EcosystemError::HealthCheckFailed { .. } => ErrorCategory::Health,
            
            EcosystemError::InternalError { .. } => ErrorCategory::Internal,
        }
    }
    
    /// Checks if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            EcosystemError::TimeoutError { .. } |
            EcosystemError::NetworkError { .. } |
            EcosystemError::ResourceExhaustion { .. } |
            EcosystemError::ServiceDegradation { .. } => true,
            
            EcosystemError::ComponentNotFound { .. } |
            EcosystemError::AuthenticationFailed { .. } |
            EcosystemError::AuthorizationDenied { .. } |
            EcosystemError::ProtocolVersionMismatch { .. } |
            EcosystemError::ConfigurationError { .. } => false,
            
            _ => false,
        }
    }
    
    /// Gets the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            EcosystemError::InternalError { .. } |
            EcosystemError::SecurityViolation { .. } => ErrorSeverity::Critical,
            
            EcosystemError::ComponentNotFound { .. } |
            EcosystemError::AuthenticationFailed { .. } |
            EcosystemError::AuthorizationDenied { .. } => ErrorSeverity::High,
            
            EcosystemError::ComponentUnavailable { .. } |
            EcosystemError::HealthCheckFailed { .. } |
            EcosystemError::DependencyError { .. } => ErrorSeverity::Medium,
            
            EcosystemError::TimeoutError { .. } |
            EcosystemError::NetworkError { .. } => ErrorSeverity::Low,
            
            _ => ErrorSeverity::Medium,
        }
    }
}

/// Error categories for classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCategory {
    Communication,
    Component,
    Security,
    Configuration,
    Resource,
    Protocol,
    Timeout,
    Dependency,
    Health,
    Internal,
}

/// Error severity levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

// Additional payload types for specific message types

/// Task coordination payload for task-related messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCoordinationPayload {
    /// Task identifier
    pub task_id: String,
    
    /// Type of task coordination
    pub coordination_type: TaskCoordinationType,
    
    /// Task details
    pub task_details: HashMap<String, serde_json::Value>,
    
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    
    /// Priority level
    pub priority: TaskPriority,
    
    /// Deadline for completion
    pub deadline: Option<SystemTime>,
}

/// Types of task coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskCoordinationType {
    Assignment,
    Progress,
    Completion,
    Cancellation,
    StatusRequest,
}

/// Task priority levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

/// Methodology execution payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyExecutionPayload {
    /// Methodology identifier
    pub methodology_id: String,
    
    /// Execution context
    pub execution_context: String,
    
    /// Current phase
    pub current_phase: Option<String>,
    
    /// Progress percentage
    pub progress: f64,
    
    /// Status information
    pub status: String,
    
    /// Results if completed
    pub results: Option<HashMap<String, serde_json::Value>>,
}

/// Data transfer payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTransferPayload {
    /// Transfer identifier
    pub transfer_id: String,
    
    /// Type of data transfer
    pub transfer_type: DataTransferType,
    
    /// Source location
    pub source: String,
    
    /// Destination location
    pub destination: String,
    
    /// Transfer progress
    pub progress: f64,
    
    /// Transfer metadata
    pub metadata: HashMap<String, String>,
}

/// Types of data transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataTransferType {
    Upload,
    Download,
    Copy,
    Move,
    Sync,
}

/// Security payload for security-related messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPayload {
    /// Security event type
    pub event_type: SecurityEventType,
    
    /// Event details
    pub details: HashMap<String, String>,
    
    /// Severity level
    pub severity: SecuritySeverity,
    
    /// Affected components
    pub affected_components: Vec<ComponentIdentity>,
}

/// Types of security events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    AuthenticationAttempt,
    AuthorizationFailure,
    SecurityViolation,
    CertificateExpiry,
    SecurityUpdate,
}

/// Security event severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecuritySeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Configuration payload for configuration messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationPayload {
    /// Configuration changes
    pub changes: HashMap<String, serde_json::Value>,
    
    /// Configuration version
    pub version: String,
    
    /// Whether restart is required
    pub restart_required: bool,
    
    /// Affected components
    pub affected_components: Vec<ComponentIdentity>,
}

/// Monitoring payload for monitoring messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringPayload {
    /// Metrics data
    pub metrics: HashMap<String, f64>,
    
    /// Metric timestamp
    pub timestamp: SystemTime,
    
    /// Metric source
    pub source: ComponentIdentity,
    
    /// Metric type
    pub metric_type: String,
}

/// Lifecycle payload for lifecycle events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecyclePayload {
    /// Lifecycle event type
    pub event_type: LifecycleEventType,
    
    /// Event details
    pub details: String,
    
    /// When the event occurred
    pub occurred_at: SystemTime,
    
    /// Expected completion time
    pub expected_completion: Option<SystemTime>,
}

/// Types of lifecycle events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEventType {
    Startup,
    Shutdown,
    Restart,
    Maintenance,
    Upgrade,
    Rollback,
}

/// Acknowledgment payload for reliable messaging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcknowledmentPayload {
    /// Acknowledged message ID
    pub acknowledged_message_id: String,
    
    /// Acknowledgment type
    pub ack_type: AcknowledmentType,
    
    /// Processing status
    pub processing_status: String,
    
    /// Additional information
    pub additional_info: Option<String>,
}

/// Types of acknowledgments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AcknowledmentType {
    Received,
    Processing,
    Completed,
    Failed,
}

// Type aliases for common patterns
pub type ComponentId = String;
pub type MessageId = String;
pub type CorrelationId = String;

// Constants for the ecosystem communication module
pub const PROTOCOL_VERSION: &str = "1.0.0";
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024 * 10; // 10MB
pub const DEFAULT_MESSAGE_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);
pub const MAX_ROUTING_HOPS: usize = 10;
pub const MESSAGE_EXPIRY_TIME: Duration = Duration::from_secs(3600); // 1 hour
