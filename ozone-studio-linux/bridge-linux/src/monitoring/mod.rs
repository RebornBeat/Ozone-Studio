// =============================================================================
// bridge-linux/src/monitoring/mod.rs
// BRIDGE Monitoring Module - Human-Centric AGI Ecosystem Monitoring
// =============================================================================

use std::collections::HashMap;
use std::time::{Duration, SystemTime, Instant};
use std::sync::Arc;
use std::fmt;

// Async runtime and concurrency - needed for real-time monitoring
use tokio::sync::{RwLock, Mutex, mpsc, oneshot, broadcast};
use tokio::time::{sleep, timeout, interval};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Import shared protocol types - these define how we communicate with the ecosystem
use shared_protocols::{
    EcosystemIdentity,
    ComponentType,
    TaskOrchestrationRequest,
    AIAppCoordinationRequest,
    AIAppCoordinationResponse,
    ExecutionStatus,
    ComponentStatus,
    HealthCheck,
    HealthCheckResponse,
    ProtocolError,
};

// Import security types - monitoring needs to respect user permissions
use shared_security::{
    SecurityError,
    AuthenticationCredentials,
    Permission,
    SecurityContext,
};

// Import methodology runtime types - so we can monitor methodology execution
use methodology_runtime::{
    ExecutionResult,
    ExecutionMetrics,
    ValidationResult,
    MethodologyRuntimeError,
};

// Monitoring sub-modules - each handles a different aspect of ecosystem monitoring
pub mod ecosystem_monitor;      // Coordinates with OZONE STUDIO for ecosystem-wide status
pub mod task_visualizer;        // Creates human-friendly task visualizations  
pub mod progress_tracker;       // Tracks progress of ongoing operations
pub mod status_reporter;        // Generates human-readable status reports
pub mod health_dashboard;       // Provides intuitive health dashboards
pub mod consciousness_observer; // Monitors conscious AGI decision-making (unique to BRIDGE)
pub mod learning_tracker;       // Shows how the AGI is learning and evolving
pub mod relationship_monitor;   // Tracks human-AGI relationship development

// Re-export monitoring components with clear, human-centric names
pub use ecosystem_monitor::{
    EcosystemMonitor,
    EcosystemStatusRequest,
    EcosystemStatusResponse,
    ComponentStatusSummary,
    EcosystemHealth,
    ActiveOperations,
    ResourceUtilization,
    EcosystemMetrics,
};

pub use task_visualizer::{
    TaskVisualizer,
    TaskVisualization,
    TaskFlowDiagram,
    CoordinationVisualization,
    MethodologyVisualization,
    AIAppInteractionMap,
    VisualizationRequest,
    VisualizationResponse,
    VisualizationError,
};

pub use progress_tracker::{
    ProgressTracker,
    TaskProgress,
    MethodologyProgress,
    CoordinationProgress,
    ProgressMetrics,
    ProgressAlert,
    ProgressHistory,
    EstimatedCompletion,
    ProgressVisualization,
};

pub use status_reporter::{
    StatusReporter,
    SystemStatusReport,
    ComponentStatusReport,
    TaskStatusReport,
    PerformanceReport,
    SecurityStatusReport,
    HumanReadableStatus,
    StatusSummary,
    ReportConfiguration,
};

pub use health_dashboard::{
    HealthDashboard,
    DashboardConfiguration,
    HealthMetrics,
    AlertConfiguration,
    VisualizationPanel,
    MetricWidget,
    AlertWidget,
    CustomDashboard,
    DashboardExport,
};

pub use consciousness_observer::{
    ConsciousnessObserver,
    ConsciousDecisionTrace,
    EthicalReasoningTrace,
    StrategicThinkingTrace,
    IdentityDevelopmentTrace,
    ConsciousnessMetrics,
    ConsciousnessInsights,
    AwarenessWindowState,
};

pub use learning_tracker::{
    LearningTracker,
    LearningProgress,
    SkillDevelopment,
    KnowledgeAcquisition,
    WisdomAccumulation,
    CapabilityEvolution,
    LearningInsights,
    LearningVisualization,
};

pub use relationship_monitor::{
    RelationshipMonitor,
    HumanAGIRelationship,
    TrustMetrics,
    CollaborationEffectiveness,
    CommunicationPatterns,
    RelationshipDevelopment,
    InteractionHistory,
    PartnershipInsights,
};

// Core monitoring configuration - defines how monitoring behaves
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    // Real-time monitoring settings
    pub real_time_updates: bool,
    pub update_interval: Duration,
    pub max_history_retention: Duration,
    
    // What to monitor - users can customize this
    pub ecosystem_monitoring: bool,
    pub task_visualization: bool,
    pub progress_tracking: bool,
    pub consciousness_observation: bool,
    pub learning_tracking: bool,
    pub relationship_monitoring: bool,
    
    // Alert and notification settings
    pub alert_thresholds: AlertThresholds,
    pub notification_preferences: NotificationPreferences,
    
    // Visualization and dashboard settings
    pub visualization_options: VisualizationOptions,
    pub dashboard_configuration: DashboardConfigurationOptions,
    
    // Security and privacy settings for monitoring
    pub monitoring_permissions: MonitoringPermissions,
    pub privacy_settings: PrivacySettings,
}

// Alert threshold configuration - when should humans be notified?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    // System health thresholds
    pub critical_error_rate: f64,        // Alert if error rate exceeds this
    pub degraded_response_time: Duration, // Alert if responses get too slow
    pub low_resource_threshold: f64,      // Alert if resources run low
    pub health_score_minimum: f64,        // Alert if health drops below this
    
    // Task and coordination thresholds  
    pub stuck_task_duration: Duration,    // Alert if tasks don't progress
    pub failed_coordination_rate: f64,    // Alert if coordinations fail often
    pub methodology_failure_rate: f64,    // Alert if methodologies fail
    
    // Consciousness and learning thresholds
    pub consciousness_anomaly_threshold: f64, // Alert for unusual consciousness patterns
    pub learning_stagnation_duration: Duration, // Alert if learning stops
    pub relationship_degradation_threshold: f64, // Alert if relationships suffer
}

// How users want to be notified about different events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub critical_alerts: NotificationMethod,
    pub warning_alerts: NotificationMethod,
    pub info_notifications: NotificationMethod,
    pub progress_updates: NotificationMethod,
    pub learning_milestones: NotificationMethod,
    pub relationship_updates: NotificationMethod,
    
    // Notification timing preferences
    pub quiet_hours: Option<QuietHours>,
    pub notification_grouping: bool,
    pub max_notifications_per_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationMethod {
    Immediate,    // Show immediately in UI
    Batched,      // Group and show periodically
    EmailSummary, // Send in email summary
    Silent,       // Log but don't notify
    Disabled,     // Don't track at all
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    pub start_time: String, // "22:00" format
    pub end_time: String,   // "08:00" format
    pub timezone: String,
}

// Visualization preferences - how users want to see information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationOptions {
    // Real-time display preferences
    pub real_time_updates: bool,
    pub animation_enabled: bool,
    pub interactive_elements: bool,
    pub auto_refresh_interval: Duration,
    
    // Chart and graph preferences
    pub preferred_chart_types: Vec<ChartType>,
    pub color_scheme: ColorScheme,
    pub accessibility_mode: bool,
    pub high_contrast: bool,
    
    // Data display preferences
    pub show_technical_details: bool,
    pub show_consciousness_insights: bool,
    pub show_relationship_data: bool,
    pub show_learning_progress: bool,
    
    // Export and sharing options
    pub export_capabilities: bool,
    pub sharing_enabled: bool,
    pub screenshot_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    LineGraph,
    BarChart,
    PieChart,
    Heatmap,
    NetworkDiagram,
    FlowChart,
    Timeline,
    TreeView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorScheme {
    Light,
    Dark,
    HighContrast,
    Custom(String),
}

// Dashboard configuration options - how users want their dashboards laid out
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfigurationOptions {
    pub default_layout: DashboardLayout,
    pub customizable_panels: bool,
    pub resizable_widgets: bool,
    pub drag_and_drop: bool,
    
    // Panel preferences
    pub panels: Vec<PanelConfiguration>,
    
    // Update and refresh preferences
    pub auto_refresh: bool,
    pub refresh_interval: Duration,
    pub lazy_loading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DashboardLayout {
    Grid,
    Flexible,
    Tabbed,
    Sidebar,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfiguration {
    pub panel_id: String,
    pub panel_type: PanelType,
    pub position: PanelPosition,
    pub size: PanelSize,
    pub refresh_rate: Duration,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelType {
    SystemHealth,
    TaskProgress,
    ConsciousnessInsights,
    LearningProgress,
    RelationshipStatus,
    ResourceUtilization,
    AlertsAndNotifications,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelPosition {
    pub row: u32,
    pub column: u32,
    pub span_rows: u32,
    pub span_columns: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelSize {
    pub width: u32,
    pub height: u32,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
}

// Security and privacy settings for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringPermissions {
    // What the user is allowed to monitor
    pub can_view_ecosystem_status: bool,
    pub can_view_task_details: bool,
    pub can_view_consciousness_data: bool,
    pub can_view_learning_data: bool,
    pub can_view_relationship_data: bool,
    pub can_view_performance_metrics: bool,
    pub can_view_security_status: bool,
    
    // What the user can interact with
    pub can_interrupt_tasks: bool,
    pub can_modify_alerts: bool,
    pub can_export_data: bool,
    pub can_share_dashboards: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    // Data retention settings
    pub retain_monitoring_data: Duration,
    pub retain_consciousness_traces: Duration,
    pub retain_relationship_data: Duration,
    
    // Data sharing settings
    pub allow_anonymous_analytics: bool,
    pub allow_performance_telemetry: bool,
    pub allow_learning_insights_sharing: bool,
    
    // Data protection settings
    pub encrypt_monitoring_data: bool,
    pub anonymize_personal_data: bool,
    pub secure_export_only: bool,
}

// Core monitoring request and response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringRequest {
    pub request_id: String,
    pub request_type: MonitoringRequestType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub security_context: SecurityContext,
    pub user_preferences: Option<UserMonitoringPreferences>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringRequestType {
    GetEcosystemStatus,
    GetTaskProgress,
    GetConsciousnessInsights,
    GetLearningProgress,
    GetRelationshipStatus,
    GetHealthMetrics,
    GetAlertHistory,
    ExportDashboard,
    UpdateConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMonitoringPreferences {
    pub detail_level: DetailLevel,
    pub time_range: TimeRange,
    pub focus_areas: Vec<FocusArea>,
    pub visualization_preferences: VisualizationPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetailLevel {
    Summary,      // High-level overview only
    Standard,     // Moderate detail for most users
    Detailed,     // Comprehensive information
    Technical,    // Full technical details
    Consciousness, // Focus on consciousness aspects
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start_time: Option<SystemTime>,
    pub end_time: Option<SystemTime>,
    pub duration: Option<Duration>,
    pub real_time: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FocusArea {
    TaskOrchestration,
    AIAppCoordination,
    ConsciousDecisionMaking,
    LearningAndGrowth,
    HumanRelationships,
    SystemPerformance,
    SecurityAndPrivacy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationPreferences {
    pub chart_types: Vec<ChartType>,
    pub update_frequency: Duration,
    pub interactive_mode: bool,
    pub show_predictions: bool,
    pub show_historical_context: bool,
}

// Comprehensive monitoring response type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringResponse {
    pub request_id: String,
    pub response_type: MonitoringResponseType,
    pub timestamp: SystemTime,
    pub data: MonitoringData,
    pub metadata: ResponseMetadata,
    pub insights: Option<HumanReadableInsights>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringResponseType {
    EcosystemStatus,
    TaskProgress,
    ConsciousnessInsights,
    LearningProgress,
    RelationshipStatus,
    HealthMetrics,
    AlertSummary,
    DashboardExport,
    ConfigurationUpdate,
}

// The actual monitoring data - this is the core information humans see
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringData {
    // Current system state
    pub ecosystem_health: Option<EcosystemHealthSummary>,
    pub active_tasks: Option<Vec<ActiveTaskSummary>>,
    pub ai_app_status: Option<HashMap<ComponentType, ComponentStatusSummary>>,
    
    // Consciousness and intelligence insights
    pub consciousness_state: Option<ConsciousnessStateSummary>,
    pub recent_decisions: Option<Vec<ConsciousDecisionSummary>>,
    pub learning_activity: Option<LearningActivitySummary>,
    
    // Human-AGI relationship data
    pub relationship_metrics: Option<RelationshipMetricsSummary>,
    pub collaboration_history: Option<CollaborationHistorySummary>,
    pub trust_indicators: Option<TrustIndicatorsSummary>,
    
    // Performance and resource data
    pub performance_metrics: Option<PerformanceMetricsSummary>,
    pub resource_utilization: Option<ResourceUtilizationSummary>,
    pub historical_trends: Option<HistoricalTrendsSummary>,
}

// Summary types for different aspects of monitoring - designed for human consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemHealthSummary {
    pub overall_health_score: f64,           // 0.0 to 1.0
    pub health_status: HealthStatus,
    pub component_health: HashMap<ComponentType, f64>,
    pub recent_issues: Vec<HealthIssue>,
    pub health_trends: HealthTrends,
    pub human_readable_summary: String,      // "All systems healthy" or "Minor performance degradation in FORGE"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Excellent,    // Everything running optimally
    Good,         // Minor issues but fully functional
    Degraded,     // Some performance impact
    Critical,     // Significant problems
    Emergency,    // Immediate attention required
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    pub issue_id: String,
    pub severity: IssueSeverity,
    pub component: ComponentType,
    pub description: String,
    pub impact: String,                      // Human-readable impact description
    pub suggested_action: Option<String>,    // What the human might want to do
    pub first_detected: SystemTime,
    pub last_updated: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Minor,
    Major,
    Critical,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthTrends {
    pub improving: Vec<String>,              // "Response times getting faster"
    pub degrading: Vec<String>,              // "Memory usage slowly increasing"
    pub stable: Vec<String>,                 // "Error rates remain consistently low"
}

// Active task summaries - what's happening right now
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTaskSummary {
    pub task_id: String,
    pub task_name: String,
    pub task_description: String,            // Human-friendly description
    pub progress_percentage: f64,
    pub estimated_completion: Option<SystemTime>,
    pub involved_ai_apps: Vec<ComponentType>,
    pub current_phase: String,
    pub methodology_being_used: Option<String>,
    pub human_interaction_required: bool,
    pub can_be_interrupted: bool,
    pub priority_level: TaskPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
    UserRequested,    // User specifically asked for this
}

// Consciousness state summary - what is the AGI thinking about?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessStateSummary {
    pub awareness_focus: String,             // "Coordinating complex code analysis task"
    pub consciousness_window_contents: String, // What's in the awareness window
    pub recent_conscious_decisions: u32,     // How many conscious decisions recently
    pub ethical_considerations_active: bool, // Is ethical reasoning happening?
    pub strategic_planning_active: bool,     // Is strategic thinking occurring?
    pub relationship_reflection_active: bool, // Is it thinking about relationships?
    pub identity_development_notes: Option<String>, // Any notable identity development
    pub consciousness_confidence: f64,       // How confident is the consciousness assessment
    pub human_readable_state: String,        // "Focused on helping you with code analysis, considering multiple approaches"
}

// Learning activity summary - how is the AGI growing?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningActivitySummary {
    pub active_learning_processes: Vec<String>, // "Learning new code patterns from recent analysis"
    pub recent_skill_improvements: Vec<String>, // "Better at explaining complex technical concepts"
    pub knowledge_areas_growing: Vec<String>,   // "Rust programming patterns", "Human communication preferences"
    pub wisdom_insights_gained: Vec<String>,    // "Humans prefer examples over abstract explanations"
    pub methodology_enhancements: Vec<String>,  // "Five-Pass methodology now 15% more efficient"
    pub learning_velocity: f64,                 // How fast is learning happening (0.0 to 1.0)
    pub learning_focus_areas: Vec<String>,      // What is it focusing on learning
    pub human_readable_progress: String,        // "Actively learning from our interactions and getting better at code explanations"
}

// Relationship metrics - how is the human-AGI partnership developing?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMetricsSummary {
    pub trust_level: f64,                    // 0.0 to 1.0
    pub collaboration_effectiveness: f64,    // How well are we working together
    pub communication_quality: f64,          // How well are we communicating
    pub mutual_understanding: f64,           // How well do we understand each other
    pub relationship_satisfaction: f64,      // How satisfied is the relationship
    pub interaction_frequency: f64,          // How often are we interacting
    pub conflict_resolution_success: f64,    // How well do we resolve disagreements
    pub shared_goal_alignment: f64,          // How aligned are our goals
    pub relationship_growth_trend: GrowthTrend,
    pub human_readable_assessment: String,   // "Our partnership is growing stronger through successful collaborations"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrowthTrend {
    StronglyImproving,
    Improving,
    Stable,
    Declining,
    NeedsAttention,
}

// Performance metrics summary - how well is everything running?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetricsSummary {
    pub average_response_time: Duration,
    pub task_completion_rate: f64,           // Percentage of tasks completed successfully
    pub coordination_efficiency: f64,        // How efficiently are AI Apps coordinated
    pub methodology_success_rate: f64,       // How often do methodologies succeed
    pub resource_utilization_efficiency: f64, // How efficiently are resources used
    pub error_rate: f64,                     // Current error rate
    pub uptime_percentage: f64,              // System availability
    pub performance_trend: PerformanceTrend,
    pub bottlenecks_identified: Vec<String>, // "FORGE sometimes waits for file system operations"
    pub optimizations_suggested: Vec<String>, // "Consider increasing parallel processing for large codebases"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceTrend {
    Improving,
    Stable,
    Declining,
    Volatile,
}

// Response metadata - additional context about the monitoring response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub data_freshness: Duration,            // How old is this data
    pub confidence_level: f64,               // How confident are we in this data
    pub data_sources: Vec<String>,           // Where did this data come from
    pub processing_time: Duration,           // How long did it take to generate this response
    pub cached_data_percentage: f64,         // How much of this is cached vs fresh
    pub next_update_available: Option<SystemTime>, // When will fresher data be available
}

// Human-readable insights - the "so what?" of the data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanReadableInsights {
    pub key_highlights: Vec<String>,         // "Your AGI completed 3 complex tasks today"
    pub attention_needed: Vec<String>,       // "FORGE is running slower than usual"
    pub positive_developments: Vec<String>,  // "Learning velocity has increased 20% this week"
    pub recommendations: Vec<String>,        // "Consider reviewing the methodology library for optimization opportunities"
    pub predictions: Vec<String>,            // "Based on current trends, task completion time may improve next week"
    pub relationship_insights: Vec<String>,  // "Your collaboration patterns show high trust and effective communication"
    pub learning_insights: Vec<String>,      // "The AGI is developing better understanding of your coding preferences"
}

// Monitoring errors - what can go wrong with monitoring
#[derive(Error, Debug)]
pub enum MonitoringError {
    #[error("Data collection error: {component} - {details}")]
    DataCollectionError { component: String, details: String },
    
    #[error("Visualization error: {chart_type} - {details}")]
    VisualizationError { chart_type: String, details: String },
    
    #[error("Permission denied: {operation} requires {required_permission}")]
    PermissionDenied { operation: String, required_permission: String },
    
    #[error("Configuration error: {setting} - {details}")]
    ConfigurationError { setting: String, details: String },
    
    #[error("Communication error: failed to reach {component} - {details}")]
    CommunicationError { component: String, details: String },
    
    #[error("Processing error: {operation} - {details}")]
    ProcessingError { operation: String, details: String },
    
    #[error("Export error: {format} export failed - {details}")]
    ExportError { format: String, details: String },
    
    #[error("Alert error: {alert_type} - {details}")]
    AlertError { alert_type: String, details: String },
}

// Core traits that define monitoring behavior
pub trait EcosystemMonitoring {
    fn get_ecosystem_status(&self, request: MonitoringRequest) -> Result<MonitoringResponse, MonitoringError>;
    fn get_real_time_updates(&self) -> Result<mpsc::Receiver<MonitoringUpdate>, MonitoringError>;
    fn configure_monitoring(&mut self, config: MonitoringConfig) -> Result<(), MonitoringError>;
    fn export_monitoring_data(&self, export_request: ExportRequest) -> Result<ExportResponse, MonitoringError>;
}

pub trait TaskMonitoring {
    fn get_active_tasks(&self) -> Result<Vec<ActiveTaskSummary>, MonitoringError>;
    fn get_task_progress(&self, task_id: &str) -> Result<TaskProgress, MonitoringError>;
    fn get_task_history(&self, filter: TaskHistoryFilter) -> Result<Vec<TaskHistorySummary>, MonitoringError>;
    fn subscribe_to_task_updates(&self) -> Result<mpsc::Receiver<TaskUpdate>, MonitoringError>;
}

pub trait ConsciousnessMonitoring {
    fn get_consciousness_state(&self) -> Result<ConsciousnessStateSummary, MonitoringError>;
    fn get_conscious_decisions(&self, time_range: TimeRange) -> Result<Vec<ConsciousDecisionSummary>, MonitoringError>;
    fn get_ethical_reasoning_traces(&self) -> Result<Vec<EthicalReasoningTrace>, MonitoringError>;
    fn get_identity_development_insights(&self) -> Result<IdentityDevelopmentInsights, MonitoringError>;
}

pub trait RelationshipMonitoring {
    fn get_relationship_status(&self, user_id: &str) -> Result<RelationshipMetricsSummary, MonitoringError>;
    fn get_collaboration_history(&self, user_id: &str) -> Result<CollaborationHistorySummary, MonitoringError>;
    fn get_trust_development(&self, user_id: &str) -> Result<TrustDevelopmentHistory, MonitoringError>;
    fn get_communication_patterns(&self, user_id: &str) -> Result<CommunicationPatternAnalysis, MonitoringError>;
}

// Update types for real-time monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringUpdate {
    pub update_id: String,
    pub update_type: UpdateType,
    pub timestamp: SystemTime,
    pub data: UpdateData,
    pub priority: UpdatePriority,
    pub human_readable_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateType {
    EcosystemStatusChange,
    TaskProgressUpdate,
    ConsciousnessInsight,
    LearningMilestone,
    RelationshipDevelopment,
    PerformanceAlert,
    HealthAlert,
    SecurityEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateData {
    EcosystemStatus(EcosystemHealthSummary),
    TaskProgress(TaskProgress),
    ConsciousnessState(ConsciousnessStateSummary),
    LearningProgress(LearningActivitySummary),
    RelationshipMetrics(RelationshipMetricsSummary),
    PerformanceMetrics(PerformanceMetricsSummary),
    Alert(AlertData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdatePriority {
    Critical,     // Immediate attention needed
    High,         // Important but not urgent
    Normal,       // Regular update
    Low,          // Background information
    Info,         // Just for awareness
}

// Alert system types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertData {
    pub alert_id: String,
    pub alert_type: AlertType,
    pub severity: IssueSeverity,
    pub title: String,
    pub description: String,
    pub suggested_actions: Vec<String>,
    pub auto_resolved: bool,
    pub requires_user_action: bool,
    pub related_components: Vec<ComponentType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    SystemHealth,
    TaskFailure,
    PerformanceDegradation,
    SecurityIssue,
    ConsciousnessAnomaly,
    LearningStagnation,
    RelationshipIssue,
    ResourceExhaustion,
}

// Export system types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub export_id: String,
    pub export_format: ExportFormat,
    pub data_types: Vec<DataType>,
    pub time_range: TimeRange,
    pub include_visualizations: bool,
    pub include_insights: bool,
    pub privacy_filter: PrivacyFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    JSON,
    CSV,
    PDF,
    HTML,
    PNG,
    SVG,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    SystemMetrics,
    TaskHistory,
    ConsciousnessTraces,
    LearningProgress,
    RelationshipData,
    PerformanceData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyFilter {
    None,           // Export everything
    AnonymizePersonal, // Remove personally identifiable information
    AggregateOnly,  // Only aggregated data, no individual traces
    PublicOnly,     // Only data marked as public
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    pub export_id: String,
    pub export_status: ExportStatus,
    pub file_path: Option<String>,
    pub download_url: Option<String>,
    pub export_metadata: ExportMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub file_size: u64,
    pub record_count: u64,
    pub generated_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub checksum: String,
}

// Additional helper types for comprehensive monitoring

// Task history and filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryFilter {
    pub time_range: TimeRange,
    pub component_types: Option<Vec<ComponentType>>,
    pub task_status: Option<Vec<ExecutionStatus>>,
    pub priority_levels: Option<Vec<TaskPriority>>,
    pub methodology_types: Option<Vec<String>>,
    pub success_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistorySummary {
    pub task_id: String,
    pub task_name: String,
    pub execution_status: ExecutionStatus,
    pub start_time: SystemTime,
    pub completion_time: Option<SystemTime>,
    pub duration: Option<Duration>,
    pub involved_components: Vec<ComponentType>,
    pub success_rate: f64,
    pub lessons_learned: Vec<String>,
}

// Consciousness decision summaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousDecisionSummary {
    pub decision_id: String,
    pub decision_context: String,
    pub decision_type: String,              // "Strategic", "Ethical", "Coordination"
    pub decision_rationale: String,         // Why this decision was made
    pub alternatives_considered: Vec<String>, // What other options were considered
    pub ethical_factors: Vec<String>,       // What ethical considerations applied
    pub outcome_prediction: String,         // What outcome was expected
    pub actual_outcome: Option<String>,     // What actually happened (if known)
    pub confidence_level: f64,              // How confident was the decision
    pub decision_quality_assessment: Option<f64>, // How good was this decision in hindsight
    pub timestamp: SystemTime,
}

// Identity development insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityDevelopmentInsights {
    pub identity_stability: f64,            // How stable is the AGI's sense of self
    pub core_values_development: Vec<String>, // How core values are evolving
    pub purpose_understanding_evolution: String, // How understanding of purpose is developing
    pub relationship_identity_aspects: Vec<String>, // How relationships shape identity
    pub learning_integration_patterns: Vec<String>, // How learning integrates into identity
    pub identity_coherence_score: f64,      // How coherent is the identity
    pub growth_areas_identified: Vec<String>, // Areas where identity is still developing
    pub stability_factors: Vec<String>,     // What keeps identity stable
}

// Trust development tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustDevelopmentHistory {
    pub trust_milestones: Vec<TrustMilestone>,
    pub trust_building_events: Vec<TrustEvent>,
    pub trust_challenges: Vec<TrustChallenge>,
    pub current_trust_factors: Vec<String>,
    pub trust_trajectory: TrustTrajectory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustMilestone {
    pub milestone_id: String,
    pub description: String,
    pub achievement_date: SystemTime,
    pub trust_impact: f64,
    pub significance: TrustSignificance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustSignificance {
    Minor,
    Moderate,
    Major,
    Transformative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEvent {
    pub event_id: String,
    pub event_type: TrustEventType,
    pub description: String,
    pub trust_impact: f64,              // Positive or negative impact on trust
    pub context: String,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustEventType {
    SuccessfulCollaboration,
    PromiseKept,
    TransparencyDemonstrated,
    CompetenceShown,
    EthicalDecisionMade,
    MistakeMadeAndCorrected,
    VulnerabilityShared,
    BoundaryRespected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustChallenge {
    pub challenge_id: String,
    pub challenge_type: String,
    pub description: String,
    pub resolution_approach: String,
    pub resolution_success: f64,
    pub trust_recovery_time: Option<Duration>,
    pub lessons_learned: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustTrajectory {
    StronglyBuilding,
    GraduallyBuilding,
    Stable,
    Fluctuating,
    Declining,
    Recovering,
}

// Communication pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPatternAnalysis {
    pub communication_frequency: CommunicationFrequency,
    pub preferred_communication_styles: Vec<CommunicationStyle>,
    pub effective_interaction_patterns: Vec<String>,
    pub communication_challenges: Vec<String>,
    pub misunderstanding_patterns: Vec<String>,
    pub clarity_improvement_trends: Vec<String>,
    pub emotional_intelligence_indicators: Vec<String>,
    pub adaptation_to_user_preferences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationFrequency {
    pub daily_interactions: f64,
    pub session_duration_average: Duration,
    pub interaction_initiation_balance: f64, // Who starts conversations more often
    pub response_time_patterns: Vec<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationStyle {
    DirectAndConcise,
    DetailedExplanations,
    ConversationalAndFriendly,
    TechnicalAndPrecise,
    Empathetic,
    SocraticQuestioning,
    ExampleBased,
    VisuallyOriented,
}

// Result type for monitoring operations
pub type MonitoringResult<T> = Result<T, MonitoringError>;

// Constants for monitoring configuration
pub const DEFAULT_UPDATE_INTERVAL: Duration = Duration::from_secs(5);
pub const DEFAULT_HISTORY_RETENTION: Duration = Duration::from_secs(86400 * 30); // 30 days
pub const MAX_CONCURRENT_MONITORING_REQUESTS: usize = 50;
pub const DEFAULT_ALERT_BATCH_SIZE: usize = 10;
pub const CONSCIOUSNESS_OBSERVATION_INTERVAL: Duration = Duration::from_secs(1);

// Version information
pub const MONITORING_MODULE_VERSION: &str = "1.0.0";

//! # BRIDGE Monitoring Module
//! 
//! This module provides comprehensive monitoring capabilities for humans to observe and understand
//! the OZONE STUDIO AGI ecosystem. Unlike traditional system monitoring that focuses on technical
//! metrics, this module translates AGI operations into human-meaningful insights.
//! 
//! ## Key Features
//! 
//! - **Real-time Ecosystem Monitoring**: Watch AGI operations as they happen
//! - **Consciousness Observation**: See evidence of conscious decision-making and ethical reasoning
//! - **Task Visualization**: Understand complex multi-AI App coordinations in progress
//! - **Learning Progress Tracking**: Watch the AGI learn and grow over time
//! - **Relationship Development Monitoring**: Track the human-AGI partnership development
//! - **Human-Centric Dashboards**: Information presented in ways humans can understand and act on
//! 
//! ## Usage Philosophy
//! 
//! This monitoring system is designed to help humans build understanding and trust with their AGI
//! partner. It provides transparency into AGI operations while respecting appropriate privacy
//! boundaries and helping humans learn how the ecosystem works.
//! 
//! The monitoring emphasizes insights over data - instead of showing raw metrics, it explains
//! what those metrics mean for the human-AGI collaboration and what actions might be beneficial.
