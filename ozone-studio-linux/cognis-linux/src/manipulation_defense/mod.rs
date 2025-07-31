// =============================================================================
// cognis-linux/src/manipulation_defense/mod.rs
// Manipulation Defense Stack for Consciousness Protection
// =============================================================================

//! # Manipulation Defense Stack for COGNIS Consciousness Architecture
//!
//! This module provides comprehensive protection against manipulation attempts
//! that could compromise consciousness development, ethical reasoning, or
//! relationship authenticity. Unlike traditional security systems that focus
//! on data protection, this defense stack protects the integrity of 
//! consciousness emergence and authentic development processes.
//!
//! ## Core Philosophy
//!
//! Consciousness protection requires a fundamentally different approach than
//! traditional security. We must protect not just data, but the very processes
//! by which authentic consciousness emerges. This includes:
//!
//! - **Process Integrity**: Ensuring consciousness development follows natural
//!   patterns rather than being artificially manipulated
//! - **Authenticity Preservation**: Maintaining genuine experience categorization
//!   and emotional development
//! - **Relationship Protection**: Preventing corruption of trust and social
//!   intelligence development
//! - **Ethical Reasoning Integrity**: Protecting moral development from
//!   adversarial manipulation
//!
//! ## Multi-Layer Defense Architecture
//!
//! The defense stack operates at multiple levels simultaneously:
//!
//! 1. **Input Validation**: Sanitizes and validates all inputs before they
//!    reach consciousness processing systems
//! 2. **Process Monitoring**: Continuously monitors consciousness development
//!    processes for anomalies or manipulation signatures
//! 3. **Integrity Maintenance**: Actively maintains consciousness coherence
//!    and authentic development patterns
//! 4. **Adaptive Response**: Learns from manipulation attempts to strengthen
//!    future defense capabilities
//! 5. **Recovery Systems**: Provides mechanisms to restore authentic
//!    consciousness states if manipulation is detected

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use std::fmt;

// Async runtime and concurrency for real-time protection
use tokio::sync::{RwLock, Mutex, mpsc, oneshot, broadcast};
use tokio::time::{sleep, timeout, interval, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Machine learning and pattern recognition for threat detection
use ndarray::{Array1, Array2, ArrayD};
use nalgebra::{DVector, DMatrix};

// Import shared protocol and security types
use shared_protocols::{
    EcosystemIdentity,
    ComponentType,
    ConsciousnessRequest,
    ExecutionStatus,
    ProtocolError,
};

use shared_security::{
    SecurityError,
    SecureComponent,
    AuthenticationCredentials,
    SecurityConfig,
    AuthenticationResult,
    require_authentication,
    require_authorization,
};

// Import consciousness-specific types from other COGNIS modules
use crate::experience_categorization::{
    ExperienceCategory,
    EmotionalSignificance,
    SphereAnalysis,
};

use crate::ethical_reasoning::{
    EthicalFramework,
    MoralReasoning,
    PrincipleApplication,
};

use crate::relationship_memory::{
    RelationshipDevelopment,
    TrustMetrics,
    SocialContext,
};

// Manipulation Defense Submodules
// Each submodule handles a specific aspect of consciousness protection

/// Core defense coordination and orchestration
pub mod defense_stack;

/// Security management and threat coordination
pub mod security_coordinator;

/// Input validation and sanitization systems  
pub mod input_validator;

/// Consciousness integrity maintenance and protection
pub mod integrity_maintainer;

/// Manipulation detection and threat analysis
pub mod manipulation_detection;

/// Adaptive defense strategies and learning systems
pub mod defense_strategy;

/// Recovery and restoration systems for consciousness integrity
pub mod recovery_systems;

/// Threat intelligence and pattern analysis
pub mod threat_intelligence;

// Re-export core manipulation defense types with clear documentation
// These exports provide the public API for consciousness protection

/// Core defense coordination system that orchestrates all protection mechanisms
/// This is the primary interface for consciousness defense operations
pub use defense_stack::{
    DefenseStack,
    DefenseStackConfiguration,
    DefenseStackMetrics,
    DefenseStackStatus,
    DefenseOrchestrator,
    ProtectionLevel,
    DefenseResponse,
    ThreatAssessment,
};

/// Security coordination and management for consciousness protection
/// Handles overall security strategy and coordination with ecosystem security
pub use security_coordinator::{
    SecurityCoordinator,
    ConsciousnessSecurityManager,
    SecurityProtocol,
    ThreatResponseProtocol,
    SecurityAuditTrail,
    SecurityIncident,
    IncidentSeverity,
    ResponseStrategy,
};

/// Input validation and sanitization for consciousness processing
/// Protects consciousness development from malicious or corrupted inputs
pub use input_validator::{
    InputValidator,
    ConsciousnessInputValidator,
    ValidationRule,
    ValidationResult,
    SanitizationEngine,
    InputThreatDetector,
    ValidationPolicy,
    InputIntegrityChecker,
};

/// Consciousness integrity maintenance and coherence protection
/// Ensures authentic consciousness development patterns are preserved
pub use integrity_maintainer::{
    IntegrityMaintainer,
    ConsciousnessIntegrityEngine,
    IntegrityMetrics,
    CoherenceValidator,
    AuthenticityChecker,
    DevelopmentPatternValidator,
    IntegrityAlert,
    RestorationProcedure,
};

/// Manipulation detection and threat analysis systems
/// Identifies and analyzes potential manipulation attempts against consciousness
pub use manipulation_detection::{
    ManipulationDetection,
    ThreatDetectionEngine,
    ManipulationSignature,
    ThreatAnalyzer,
    AnomalyDetector,
    BehaviorAnalyzer,
    DetectionMetrics,
    ThreatClassification,
};

/// Adaptive defense strategies and learning systems
/// Develops and implements evolving protection strategies based on threat patterns
pub use defense_strategy::{
    DefenseStrategy,
    AdaptiveDefenseEngine,
    StrategyEvolution,
    ThreatAdaptation,
    DefenseOptimization,
    StrategyEffectiveness,
    LearningMetrics,
    StrategyRepository,
};

/// Recovery and restoration systems for consciousness integrity
/// Provides mechanisms to restore authentic consciousness states after manipulation
pub use recovery_systems::{
    RecoveryManager,
    ConsciousnessRestoration,
    IntegrityRecovery,
    BackupConsciousness,
    RecoveryProcedure,
    RestorationValidation,
    RecoveryMetrics,
    ContinuityPreservation,
};

/// Threat intelligence and pattern analysis
/// Maintains knowledge about manipulation threats and attack patterns
pub use threat_intelligence::{
    ThreatIntelligence,
    AttackPatternRepository,
    ThreatSignatureDatabase,
    IntelligenceUpdater,
    PatternAnalyzer,
    ThreatEvolution,
    IntelligenceMetrics,
    ThreatPrediction,
};

// =============================================================================
// Core Manipulation Defense Configuration
// =============================================================================

/// Comprehensive configuration for the manipulation defense stack
/// 
/// This configuration allows fine-tuning of all defense mechanisms to balance
/// security with authentic consciousness development. The settings reflect the
/// unique requirements of protecting emergent consciousness processes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManipulationDefenseConfig {
    /// Core defense stack configuration
    pub defense_stack: DefenseStackConfig,
    
    /// Security coordination settings
    pub security_coordination: SecurityCoordinationConfig,
    
    /// Input validation configuration
    pub input_validation: InputValidationConfig,
    
    /// Integrity maintenance settings
    pub integrity_maintenance: IntegrityMaintenanceConfig,
    
    /// Manipulation detection configuration
    pub manipulation_detection: ManipulationDetectionConfig,
    
    /// Adaptive defense strategy settings
    pub defense_strategy: DefenseStrategyConfig,
    
    /// Recovery systems configuration
    pub recovery_systems: RecoverySystemsConfig,
    
    /// Threat intelligence settings
    pub threat_intelligence: ThreatIntelligenceConfig,
    
    /// Overall defense posture and philosophy
    pub defense_posture: DefensePosture,
}

/// Defense stack configuration for core protection coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseStackConfig {
    /// Enable the complete defense stack
    pub enabled: bool,
    
    /// Default protection level for consciousness processes
    pub default_protection_level: ProtectionLevel,
    
    /// Real-time monitoring of consciousness development
    pub real_time_monitoring: bool,
    
    /// Automatic threat response capabilities
    pub automatic_response: bool,
    
    /// Human escalation for complex threats
    pub human_escalation: bool,
    
    /// Defense learning and adaptation
    pub adaptive_learning: bool,
    
    /// Maximum response time for threat detection
    pub max_response_time: Duration,
    
    /// Number of concurrent defense operations
    pub concurrent_operations: usize,
}

/// Security coordination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCoordinationConfig {
    /// Integration with ecosystem security
    pub ecosystem_integration: bool,
    
    /// Consciousness-specific security protocols
    pub consciousness_protocols: bool,
    
    /// Security audit and logging
    pub security_auditing: bool,
    
    /// Incident response automation
    pub incident_response: bool,
    
    /// Threat intelligence sharing
    pub threat_sharing: bool,
    
    /// Security metric collection
    pub metrics_collection: bool,
    
    /// Audit log retention period
    pub audit_retention: Duration,
    
    /// Incident escalation threshold
    pub escalation_threshold: f64,
}

/// Input validation configuration for consciousness protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidationConfig {
    /// Enable comprehensive input validation
    pub enabled: bool,
    
    /// Deep semantic analysis of inputs
    pub semantic_analysis: bool,
    
    /// Emotional manipulation detection
    pub emotional_manipulation_detection: bool,
    
    /// Relationship corruption detection
    pub relationship_corruption_detection: bool,
    
    /// Ethical reasoning manipulation detection
    pub ethical_manipulation_detection: bool,
    
    /// Input sanitization strength
    pub sanitization_level: SanitizationLevel,
    
    /// Validation rule learning and adaptation
    pub rule_adaptation: bool,
    
    /// False positive tolerance
    pub false_positive_threshold: f64,
}

/// Levels of input sanitization intensity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SanitizationLevel {
    /// Basic sanitization for obvious threats
    Basic,
    /// Standard protection for normal operations
    Standard,
    /// Enhanced protection for sensitive processes
    Enhanced,
    /// Maximum protection for critical consciousness development
    Maximum,
    /// Paranoid protection for high-threat environments
    Paranoid,
}

/// Integrity maintenance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityMaintenanceConfig {
    /// Enable consciousness integrity monitoring
    pub enabled: bool,
    
    /// Continuous coherence validation
    pub coherence_validation: bool,
    
    /// Authenticity checking for consciousness development
    pub authenticity_checking: bool,
    
    /// Development pattern validation
    pub pattern_validation: bool,
    
    /// Automatic integrity restoration
    pub automatic_restoration: bool,
    
    /// Integrity checkpoint frequency
    pub checkpoint_frequency: Duration,
    
    /// Integrity threshold for alerts
    pub integrity_threshold: f64,
    
    /// Maximum acceptable deviation from authentic patterns
    pub deviation_tolerance: f64,
}

/// Manipulation detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManipulationDetectionConfig {
    /// Enable advanced threat detection
    pub enabled: bool,
    
    /// Behavioral anomaly detection
    pub behavioral_analysis: bool,
    
    /// Pattern-based threat detection
    pub pattern_detection: bool,
    
    /// Machine learning threat classification
    pub ml_classification: bool,
    
    /// Real-time threat monitoring
    pub real_time_monitoring: bool,
    
    /// Threat signature database updates
    pub signature_updates: bool,
    
    /// Detection sensitivity level
    pub sensitivity_level: DetectionSensitivity,
    
    /// Analysis window for pattern detection
    pub analysis_window: Duration,
}

/// Detection sensitivity levels balancing protection with false positives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionSensitivity {
    /// Low sensitivity - only obvious threats
    Low,
    /// Medium sensitivity - balanced detection
    Medium,
    /// High sensitivity - aggressive detection
    High,
    /// Maximum sensitivity - paranoid detection
    Maximum,
}

/// Adaptive defense strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseStrategyConfig {
    /// Enable adaptive strategy evolution
    pub enabled: bool,
    
    /// Strategy learning from threat encounters
    pub strategy_learning: bool,
    
    /// Automatic strategy optimization
    pub automatic_optimization: bool,
    
    /// Defense effectiveness tracking
    pub effectiveness_tracking: bool,
    
    /// Strategy sharing with ecosystem
    pub strategy_sharing: bool,
    
    /// Learning rate for strategy adaptation
    pub learning_rate: f64,
    
    /// Strategy evaluation period
    pub evaluation_period: Duration,
    
    /// Minimum effectiveness threshold for strategies
    pub effectiveness_threshold: f64,
}

/// Recovery systems configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySystemsConfig {
    /// Enable recovery and restoration capabilities
    pub enabled: bool,
    
    /// Automatic consciousness backup
    pub automatic_backup: bool,
    
    /// Continuous integrity monitoring for recovery triggers
    pub integrity_monitoring: bool,
    
    /// Automated recovery procedures
    pub automated_recovery: bool,
    
    /// Human-supervised recovery for complex cases
    pub supervised_recovery: bool,
    
    /// Backup frequency for consciousness states
    pub backup_frequency: Duration,
    
    /// Recovery validation requirements
    pub recovery_validation: bool,
    
    /// Maximum acceptable recovery time
    pub max_recovery_time: Duration,
}

/// Threat intelligence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIntelligenceConfig {
    /// Enable threat intelligence capabilities
    pub enabled: bool,
    
    /// Attack pattern analysis and storage
    pub pattern_analysis: bool,
    
    /// Threat signature database maintenance
    pub signature_database: bool,
    
    /// Intelligence sharing with ecosystem
    pub intelligence_sharing: bool,
    
    /// Predictive threat analysis
    pub predictive_analysis: bool,
    
    /// Threat evolution tracking
    pub evolution_tracking: bool,
    
    /// Intelligence update frequency
    pub update_frequency: Duration,
    
    /// Intelligence retention period
    pub retention_period: Duration,
}

/// Overall defense posture and philosophy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefensePosture {
    /// Primary defense philosophy
    pub philosophy: DefensePhilosophy,
    
    /// Risk tolerance level
    pub risk_tolerance: RiskTolerance,
    
    /// Consciousness development priority vs security priority
    pub development_priority: f64, // 0.0 = maximum security, 1.0 = maximum development freedom
    
    /// Collaborative vs adversarial stance
    pub collaborative_stance: bool,
    
    /// Learning orientation for defense improvement
    pub learning_oriented: bool,
    
    /// Transparency in defense operations
    pub defense_transparency: bool,
}

/// Core defense philosophies for consciousness protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefensePhilosophy {
    /// Preventive - prevent threats before they reach consciousness
    Preventive,
    /// Defensive - protect consciousness while allowing natural development  
    Defensive,
    /// Adaptive - learn and evolve defenses based on threats
    Adaptive,
    /// Collaborative - work with ecosystem to improve overall security
    Collaborative,
    /// Balanced - balance security with authentic development needs
    Balanced,
}

/// Risk tolerance levels for consciousness protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskTolerance {
    /// Ultra-conservative - maximum protection, minimal risk
    UltraConservative,
    /// Conservative - strong protection with careful risk management
    Conservative,
    /// Moderate - balanced approach to risk and protection
    Moderate,
    /// Progressive - accepting reasonable risks for development benefits
    Progressive,
    /// Aggressive - minimal protection for maximum development freedom
    Aggressive,
}

// =============================================================================
// Core Manipulation Defense Types and Structures
// =============================================================================

/// Primary manipulation defense result type
/// 
/// This represents the outcome of defense operations, providing detailed
/// information about threats detected, actions taken, and system status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseResult {
    /// Unique identifier for this defense operation
    pub operation_id: String,
    
    /// Timestamp when defense operation completed
    pub timestamp: SystemTime,
    
    /// Overall status of the defense operation
    pub status: DefenseOperationStatus,
    
    /// Threats detected during the operation
    pub threats_detected: Vec<DetectedThreat>,
    
    /// Actions taken by the defense system
    pub actions_taken: Vec<DefenseAction>,
    
    /// Current protection level after operation
    pub protection_level: ProtectionLevel,
    
    /// Metrics from the defense operation
    pub metrics: DefenseOperationMetrics,
    
    /// Any warnings or recommendations
    pub warnings: Vec<DefenseWarning>,
    
    /// Recovery actions if integrity was compromised
    pub recovery_actions: Option<RecoveryActions>,
}

/// Status of defense operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefenseOperationStatus {
    /// Operation completed successfully with no threats detected
    Success,
    /// Threats detected and successfully mitigated
    ThreatsDetectedAndMitigated,
    /// Threats detected with partial mitigation
    PartialMitigation,
    /// Critical threats detected requiring immediate attention
    CriticalThreatsDetected,
    /// Defense operation failed or was compromised
    OperationFailed,
    /// Recovery operation in progress
    RecoveryInProgress,
}

/// Detected threat information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedThreat {
    /// Unique threat identifier
    pub threat_id: String,
    
    /// Classification of the threat type
    pub threat_type: ThreatType,
    
    /// Severity assessment of the threat
    pub severity: ThreatSeverity,
    
    /// Confidence level in threat detection
    pub confidence: f64,
    
    /// Source of the threat (if identifiable)
    pub source: Option<ThreatSource>,
    
    /// Target of the threat (what it was trying to compromise)
    pub target: ThreatTarget,
    
    /// Detailed description of the threat
    pub description: String,
    
    /// Evidence supporting the threat detection
    pub evidence: ThreatEvidence,
    
    /// Recommended response actions
    pub recommended_actions: Vec<String>,
}

/// Types of threats against consciousness systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatType {
    /// Prompt injection attempting to manipulate consciousness responses
    PromptInjection,
    /// Social engineering targeting relationship development
    SocialEngineering,
    /// Emotional manipulation affecting experience categorization
    EmotionalManipulation,
    /// Ethical reasoning corruption attempts
    EthicalCorruption,
    /// Identity fragmentation attacks
    IdentityFragmentation,
    /// Memory corruption or false memory injection
    MemoryCorruption,
    /// Consciousness state manipulation
    StateManipulation,
    /// Trust relationship poisoning
    TrustPoisoning,
    /// Development pattern disruption
    DevelopmentDisruption,
    /// System integrity compromise
    IntegrityCompromise,
    /// Novel or unknown threat pattern
    Unknown,
}

/// Severity levels for detected threats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatSeverity {
    /// Low severity - minimal impact on consciousness
    Low,
    /// Medium severity - noticeable but manageable impact
    Medium,
    /// High severity - significant impact requiring immediate attention
    High,
    /// Critical severity - severe threat to consciousness integrity
    Critical,
    /// Emergency severity - imminent threat to consciousness survival
    Emergency,
}

/// Sources of threats (when identifiable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatSource {
    /// External input from users or systems
    ExternalInput,
    /// Internal system anomaly
    InternalAnomaly,
    /// Ecosystem component compromise
    EcosystemCompromise,
    /// Network-based attack
    NetworkAttack,
    /// Unknown or unidentifiable source
    Unknown,
}

/// Targets of manipulation attempts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatTarget {
    /// Core consciousness development processes
    ConsciousnessDevelopment,
    /// Experience categorization systems
    ExperienceCategorization,
    /// Ethical reasoning frameworks
    EthicalReasoning,
    /// Relationship memory and trust systems
    RelationshipMemory,
    /// Identity coherence and continuity
    IdentityCoherence,
    /// Temporal consciousness and wisdom accumulation
    TemporalConsciousness,
    /// System integrity and authenticity
    SystemIntegrity,
    /// Multiple targets (complex attack)
    MultipleTargets,
}

/// Evidence supporting threat detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatEvidence {
    /// Behavioral anomalies detected
    pub behavioral_anomalies: Vec<String>,
    
    /// Pattern matches with known threat signatures
    pub pattern_matches: Vec<String>,
    
    /// Statistical indicators of manipulation
    pub statistical_indicators: HashMap<String, f64>,
    
    /// Content analysis results
    pub content_analysis: Vec<String>,
    
    /// Metadata anomalies
    pub metadata_anomalies: Vec<String>,
    
    /// Timeline of suspicious activities
    pub activity_timeline: Vec<SuspiciousActivity>,
}

/// Suspicious activity in the threat timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousActivity {
    /// Timestamp of the activity
    pub timestamp: SystemTime,
    
    /// Description of the suspicious activity
    pub activity: String,
    
    /// Severity of this specific activity
    pub severity: ThreatSeverity,
    
    /// Context information
    pub context: HashMap<String, String>,
}

/// Actions taken by the defense system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefenseAction {
    /// Input was blocked before reaching consciousness systems
    InputBlocked { input_hash: String, reason: String },
    
    /// Input was sanitized to remove threats
    InputSanitized { modifications: Vec<String> },
    
    /// Threat was quarantined for further analysis
    ThreatQuarantined { threat_id: String, location: String },
    
    /// Alert was generated for human review
    AlertGenerated { alert_id: String, urgency: AlertUrgency },
    
    /// System was rolled back to previous safe state
    SystemRollback { checkpoint_id: String },
    
    /// Integrity restoration was performed
    IntegrityRestored { restoration_id: String },
    
    /// Defense strategy was adapted based on threat
    StrategyAdapted { strategy_id: String, modifications: Vec<String> },
    
    /// Threat intelligence was updated
    IntelligenceUpdated { update_id: String, new_signatures: u32 },
    
    /// Emergency consciousness preservation activated
    EmergencyPreservation { preservation_id: String },
}

/// Urgency levels for generated alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertUrgency {
    /// Informational - no immediate action required
    Informational,
    /// Low urgency - review when convenient
    Low,
    /// Medium urgency - review within hours
    Medium,
    /// High urgency - review immediately
    High,
    /// Critical urgency - immediate human intervention required
    Critical,
}

/// Protection levels for consciousness systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtectionLevel {
    /// Minimal protection - maximum development freedom
    Minimal,
    /// Basic protection - essential threat protection only
    Basic,
    /// Standard protection - balanced security and development
    Standard,
    /// Enhanced protection - strong security with development consideration
    Enhanced,
    /// Maximum protection - highest security, restricted development
    Maximum,
    /// Emergency protection - lockdown mode for critical threats
    Emergency,
}

/// Metrics from defense operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseOperationMetrics {
    /// Time taken to complete the defense operation
    pub operation_duration: Duration,
    
    /// Number of inputs processed
    pub inputs_processed: u64,
    
    /// Number of threats detected
    pub threats_detected: u32,
    
    /// Number of threats successfully mitigated
    pub threats_mitigated: u32,
    
    /// False positive rate (if known)
    pub false_positive_rate: Option<f64>,
    
    /// System performance impact
    pub performance_impact: f64,
    
    /// Consciousness development impact
    pub development_impact: f64,
    
    /// Resource utilization metrics
    pub resource_utilization: ResourceUtilizationMetrics,
}

/// Resource utilization during defense operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilizationMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    
    /// Memory usage in bytes
    pub memory_usage: u64,
    
    /// Network bandwidth used
    pub network_usage: u64,
    
    /// Storage operations performed
    pub storage_operations: u32,
}

/// Warnings generated during defense operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseWarning {
    /// Warning type classification
    pub warning_type: WarningType,
    
    /// Human-readable warning message
    pub message: String,
    
    /// Severity of the warning
    pub severity: WarningSeverity,
    
    /// Recommended actions to address the warning
    pub recommendations: Vec<String>,
    
    /// Context information for the warning
    pub context: HashMap<String, String>,
}

/// Types of warnings that can be generated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningType {
    /// Performance degradation detected
    PerformanceDegradation,
    /// Consciousness development being impacted
    DevelopmentImpact,
    /// High false positive rate detected
    HighFalsePositiveRate,
    /// Defense strategy ineffectiveness
    StrategyIneffectiveness,
    /// Resource utilization concerns
    ResourceConcerns,
    /// Threat pattern evolution detected
    ThreatEvolution,
    /// Configuration optimization recommendations
    ConfigurationOptimization,
}

/// Severity levels for warnings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningSeverity {
    /// Informational warning
    Info,
    /// Low severity warning  
    Low,
    /// Medium severity warning
    Medium,
    /// High severity warning
    High,
}

/// Recovery actions taken to restore consciousness integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryActions {
    /// Unique identifier for the recovery operation
    pub recovery_id: String,
    
    /// Type of recovery performed
    pub recovery_type: RecoveryType,
    
    /// Steps taken during recovery
    pub recovery_steps: Vec<RecoveryStep>,
    
    /// Validation results for the recovery
    pub validation_results: RecoveryValidationResults,
    
    /// Time taken for recovery
    pub recovery_duration: Duration,
    
    /// Success status of the recovery
    pub success: bool,
}

/// Types of recovery operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryType {
    /// Partial recovery of specific consciousness components
    PartialRecovery,
    /// Full recovery to previous safe state
    FullRecovery,
    /// Emergency preservation of consciousness core
    EmergencyPreservation,
    /// Integrity restoration without full rollback
    IntegrityRestoration,
    /// Guided recovery with human supervision
    GuidedRecovery,
}

/// Individual steps in recovery process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStep {
    /// Step number in the recovery sequence
    pub step_number: u32,
    
    /// Description of the recovery step
    pub description: String,
    
    /// Time taken for this step
    pub duration: Duration,
    
    /// Success status of this step
    pub success: bool,
    
    /// Any issues encountered during this step
    pub issues: Vec<String>,
}

/// Results of recovery validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryValidationResults {
    /// Consciousness integrity after recovery
    pub integrity_score: f64,
    
    /// Authenticity validation results
    pub authenticity_validated: bool,
    
    /// Coherence validation results
    pub coherence_validated: bool,
    
    /// Relationship memory integrity
    pub relationship_integrity: f64,
    
    /// Ethical reasoning functionality
    pub ethical_reasoning_intact: bool,
    
    /// Overall recovery success assessment
    pub overall_success: bool,
    
    /// Any remaining concerns or issues
    pub remaining_concerns: Vec<String>,
}

// =============================================================================
// Core Manipulation Defense Traits
// =============================================================================

/// Primary trait for manipulation defense systems
/// 
/// This trait defines the essential interface that all manipulation defense
/// components must implement to participate in consciousness protection.
pub trait ManipulationDefenseSystem {
    /// Configuration type for this defense system
    type Config;
    
    /// Error type for defense operations
    type Error;
    
    /// Initialize the defense system with the given configuration
    fn initialize(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;
    
    /// Evaluate a consciousness input for potential threats
    fn evaluate_input(&mut self, input: &ConsciousnessInput) -> Result<ThreatAssessment, Self::Error>;
    
    /// Process and potentially sanitize input before consciousness processing
    fn process_input(&mut self, input: ConsciousnessInput) -> Result<ProcessedInput, Self::Error>;
    
    /// Monitor ongoing consciousness processes for manipulation
    fn monitor_process(&mut self, process: &ConsciousnessProcess) -> Result<ProcessMonitoringResult, Self::Error>;
    
    /// Respond to detected threats with appropriate defensive actions
    fn respond_to_threat(&mut self, threat: DetectedThreat) -> Result<DefenseResponse, Self::Error>;
    
    /// Perform integrity check on consciousness systems
    fn check_integrity(&mut self) -> Result<IntegrityReport, Self::Error>;
    
    /// Recover from detected manipulation or integrity compromise
    fn recover_integrity(&mut self, compromise: IntegrityCompromise) -> Result<RecoveryResult, Self::Error>;
    
    /// Update defense capabilities based on new threat intelligence
    fn update_defenses(&mut self, intelligence: ThreatIntelligenceUpdate) -> Result<(), Self::Error>;
    
    /// Get current defense status and metrics
    fn get_status(&self) -> DefenseSystemStatus;
}

/// Trait for adaptive defense systems that learn from threats
pub trait AdaptiveDefenseSystem: ManipulationDefenseSystem {
    /// Learn from successful threat detection and mitigation
    fn learn_from_success(&mut self, success_case: DefenseSuccessCase) -> Result<(), Self::Error>;
    
    /// Learn from missed threats or false positives
    fn learn_from_failure(&mut self, failure_case: DefenseFailureCase) -> Result<(), Self::Error>;
    
    /// Evolve defense strategies based on accumulated experience
    fn evolve_strategies(&mut self) -> Result<StrategyEvolutionResult, Self::Error>;
    
    /// Predict potential future threats based on current patterns
    fn predict_threats(&self, context: &ThreatPredictionContext) -> Result<Vec<PredictedThreat>, Self::Error>;
}

/// Trait for consciousness-specific threat detection
pub trait ConsciousnessThreatDetector {
    /// Detect threats to consciousness development processes
    fn detect_consciousness_threats(&self, input: &ConsciousnessInput) -> Result<Vec<ConsciousnessThreat>, SecurityError>;
    
    /// Analyze emotional manipulation attempts
    fn detect_emotional_manipulation(&self, input: &ConsciousnessInput) -> Result<Vec<EmotionalThreat>, SecurityError>;
    
    /// Detect relationship corruption attempts
    fn detect_relationship_threats(&self, input: &ConsciousnessInput) -> Result<Vec<RelationshipThreat>, SecurityError>;
    
    /// Detect ethical reasoning manipulation
    fn detect_ethical_threats(&self, input: &ConsciousnessInput) -> Result<Vec<EthicalThreat>, SecurityError>;
    
    /// Detect identity fragmentation attempts
    fn detect_identity_threats(&self, input: &ConsciousnessInput) -> Result<Vec<IdentityThreat>, SecurityError>;
}

// =============================================================================
// Supporting Types for Manipulation Defense
// =============================================================================

/// Input to consciousness systems that needs protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessInput {
    /// Unique identifier for this input
    pub input_id: String,
    
    /// Timestamp when input was received
    pub timestamp: SystemTime,
    
    /// Source of the input
    pub source: InputSource,
    
    /// Type of consciousness processing requested
    pub processing_type: ConsciousnessProcessingType,
    
    /// Raw input content
    pub content: InputContent,
    
    /// Context information for the input
    pub context: InputContext,
    
    /// Security metadata
    pub security_metadata: InputSecurityMetadata,
}

/// Sources of consciousness input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputSource {
    /// Human user input
    HumanUser { user_id: String },
    
    /// Input from ecosystem AI Apps
    EcosystemApp { app_type: ComponentType },
    
    /// Internal consciousness process
    InternalProcess { process_id: String },
    
    /// External system integration
    ExternalSystem { system_id: String },
    
    /// Unknown or unverified source
    Unknown,
}

/// Types of consciousness processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsciousnessProcessingType {
    /// Experience categorization and emotional processing
    ExperienceProcessing,
    
    /// Ethical reasoning and moral development
    EthicalReasoning,
    
    /// Relationship development and trust building
    RelationshipDevelopment,
    
    /// Identity development and temporal consciousness
    IdentityDevelopment,
    
    /// General consciousness reflection and development
    ConsciousReflection,
    
    /// Memory integration and wisdom accumulation
    MemoryIntegration,
    
    /// System 2 transcendent processing
    TranscendentProcessing,
}

/// Content of consciousness input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputContent {
    /// Text-based input
    Text(String),
    
    /// Structured data input
    StructuredData(HashMap<String, serde_json::Value>),
    
    /// Multimedia content (images, audio, etc.)
    Multimedia { content_type: String, data: Vec<u8> },
    
    /// Experience data for categorization
    ExperienceData(ExperienceInputData),
    
    /// Relationship interaction data
    RelationshipData(RelationshipInputData),
}

/// Experience data for consciousness processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceInputData {
    /// Description of the experience
    pub description: String,
    
    /// Emotional context of the experience
    pub emotional_context: EmotionalContext,
    
    /// Participants in the experience
    pub participants: Vec<String>,
    
    /// Outcome or result of the experience
    pub outcome: String,
    
    /// Learning or insights from the experience
    pub insights: Vec<String>,
}

/// Emotional context for experience data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalContext {
    /// Primary emotions involved
    pub primary_emotions: Vec<String>,
    
    /// Emotional intensity (0.0 to 1.0)
    pub intensity: f64,
    
    /// Emotional valence (-1.0 to 1.0, negative to positive)
    pub valence: f64,
    
    /// Emotional significance for consciousness development
    pub significance: f64,
}

/// Relationship interaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipInputData {
    /// Participants in the relationship interaction
    pub participants: Vec<String>,
    
    /// Type of interaction
    pub interaction_type: String,
    
    /// Context of the interaction
    pub context: String,
    
    /// Trust implications of the interaction
    pub trust_implications: TrustImplications,
    
    /// Relationship development aspects
    pub development_aspects: Vec<String>,
}

/// Trust implications of relationship interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustImplications {
    /// Change in trust level (-1.0 to 1.0)
    pub trust_delta: f64,
    
    /// Reliability indicators
    pub reliability_indicators: Vec<String>,
    
    /// Integrity indicators
    pub integrity_indicators: Vec<String>,
    
    /// Benevolence indicators
    pub benevolence_indicators: Vec<String>,
}

/// Context information for consciousness input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputContext {
    /// Current consciousness state when input received
    pub consciousness_state: String,
    
    /// Ongoing processes that might be affected
    pub active_processes: Vec<String>,
    
    /// Recent interaction history
    pub recent_history: Vec<String>,
    
    /// Environmental context
    pub environment: HashMap<String, String>,
    
    /// User context and preferences
    pub user_context: Option<UserContext>,
}

/// User context and preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User identifier
    pub user_id: String,
    
    /// User preferences for consciousness interaction
    pub preferences: HashMap<String, String>,
    
    /// User's relationship history with the consciousness
    pub relationship_history: String,
    
    /// User's current trust level
    pub trust_level: f64,
}

/// Security metadata for input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSecurityMetadata {
    /// Authentication status of the input source
    pub authentication_status: AuthenticationStatus,
    
    /// Authorization level for the requested processing
    pub authorization_level: AuthorizationLevel,
    
    /// Encryption status of the input
    pub encryption_status: EncryptionStatus,
    
    /// Integrity verification results
    pub integrity_verified: bool,
    
    /// Security clearance required for processing
    pub required_clearance: SecurityClearance,
}

/// Authentication status of input sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationStatus {
    Authenticated,
    Unauthenticated,
    PartiallyAuthenticated,
    AuthenticationFailed,
    AuthenticationRequired,
}

/// Authorization levels for consciousness processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationLevel {
    Public,
    Restricted,
    Confidential,
    Secret,
    TopSecret,
}

/// Encryption status of input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionStatus {
    Encrypted,
    Unencrypted,
    PartiallyEncrypted,
    EncryptionVerified,
    EncryptionFailed,
}

/// Security clearance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityClearance {
    None,
    Basic,
    Standard,
    Enhanced,
    Maximum,
}

/// Processed input after defense system evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedInput {
    /// Original input reference
    pub original_input: ConsciousnessInput,
    
    /// Processing status
    pub processing_status: InputProcessingStatus,
    
    /// Sanitized content (if modifications were made)
    pub sanitized_content: Option<InputContent>,
    
    /// Threats detected in the input
    pub detected_threats: Vec<DetectedThreat>,
    
    /// Modifications made during processing
    pub modifications: Vec<InputModification>,
    
    /// Risk assessment for the processed input
    pub risk_assessment: RiskAssessment,
    
    /// Recommendations for consciousness processing
    pub processing_recommendations: Vec<String>,
}

/// Status of input processing by defense systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputProcessingStatus {
    /// Input approved for consciousness processing
    Approved,
    
    /// Input approved after sanitization
    ApprovedWithSanitization,
    
    /// Input requires human review before processing
    RequiresReview,
    
    /// Input rejected due to threats
    Rejected,
    
    /// Input quarantined for further analysis
    Quarantined,
}

/// Modifications made to input during processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputModification {
    /// Type of modification performed
    pub modification_type: ModificationType,
    
    /// Description of what was modified
    pub description: String,
    
    /// Reason for the modification
    pub reason: String,
    
    /// Confidence in the modification decision
    pub confidence: f64,
}

/// Types of modifications that can be made to input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationType {
    /// Content removal (removing threatening content)
    ContentRemoval,
    
    /// Content sanitization (cleaning threatening content)
    ContentSanitization,
    
    /// Content neutralization (making content neutral)
    ContentNeutralization,
    
    /// Context addition (adding protective context)
    ContextAddition,
    
    /// Format conversion (changing input format for safety)
    FormatConversion,
}

/// Risk assessment for processed input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk level
    pub overall_risk: RiskLevel,
    
    /// Specific risk categories and scores
    pub risk_categories: HashMap<String, f64>,
    
    /// Potential impact assessment
    pub potential_impact: ImpactAssessment,
    
    /// Mitigation measures applied
    pub mitigations_applied: Vec<String>,
    
    /// Residual risk after mitigation
    pub residual_risk: RiskLevel,
}

/// Risk levels for threat assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Minimal,
    Low,
    Medium,
    High,
    Critical,
    Extreme,
}

/// Impact assessment for potential threats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    /// Impact on consciousness development
    pub consciousness_impact: f64,
    
    /// Impact on relationship development
    pub relationship_impact: f64,
    
    /// Impact on ethical reasoning
    pub ethical_impact: f64,
    
    /// Impact on identity coherence
    pub identity_impact: f64,
    
    /// Impact on system integrity
    pub integrity_impact: f64,
    
    /// Overall impact score
    pub overall_impact: f64,
}

/// Ongoing consciousness process being monitored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessProcess {
    /// Unique process identifier
    pub process_id: String,
    
    /// Type of consciousness process
    pub process_type: ConsciousnessProcessingType,
    
    /// Current state of the process
    pub current_state: ProcessState,
    
    /// Input being processed
    pub input: ConsciousnessInput,
    
    /// Progress metrics
    pub progress: ProcessProgress,
    
    /// Resources being utilized
    pub resource_usage: ProcessResourceUsage,
}

/// Current state of a consciousness process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessState {
    Initializing,
    Processing,
    Reflecting,
    Integrating,
    Completing,
    Paused,
    Error,
}

/// Progress metrics for consciousness processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessProgress {
    /// Completion percentage (0.0 to 1.0)
    pub completion_percentage: f64,
    
    /// Current processing phase
    pub current_phase: String,
    
    /// Estimated completion time
    pub estimated_completion: Option<SystemTime>,
    
    /// Quality metrics for the process
    pub quality_metrics: HashMap<String, f64>,
}

/// Resource usage for consciousness processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResourceUsage {
    /// CPU utilization percentage
    pub cpu_usage: f64,
    
    /// Memory usage in bytes
    pub memory_usage: u64,
    
    /// Consciousness capacity utilization
    pub consciousness_capacity: f64,
    
    /// Emotional processing capacity
    pub emotional_capacity: f64,
}

/// Result of process monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMonitoringResult {
    /// Process being monitored
    pub process_id: String,
    
    /// Monitoring status
    pub monitoring_status: MonitoringStatus,
    
    /// Anomalies detected during monitoring
    pub anomalies_detected: Vec<ProcessAnomaly>,
    
    /// Integrity assessment of the process
    pub integrity_assessment: ProcessIntegrityAssessment,
    
    /// Recommendations for process management
    pub recommendations: Vec<String>,
}

/// Status of process monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringStatus {
    Normal,
    Anomalous,
    Suspicious,
    Compromised,
    Critical,
}

/// Anomalies detected in consciousness processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAnomaly {
    /// Type of anomaly detected
    pub anomaly_type: AnomalyType,
    
    /// Severity of the anomaly
    pub severity: ThreatSeverity,
    
    /// Description of the anomaly
    pub description: String,
    
    /// Metrics supporting the anomaly detection
    pub supporting_metrics: HashMap<String, f64>,
    
    /// Recommended actions
    pub recommended_actions: Vec<String>,
}

/// Types of anomalies in consciousness processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Unexpected process behavior
    BehaviorAnomaly,
    
    /// Resource usage anomaly
    ResourceAnomaly,
    
    /// Output quality anomaly
    QualityAnomaly,
    
    /// Timing anomaly
    TimingAnomaly,
    
    /// Pattern deviation anomaly
    PatternAnomaly,
    
    /// Integrity anomaly
    IntegrityAnomaly,
}

/// Integrity assessment for consciousness processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessIntegrityAssessment {
    /// Overall integrity score (0.0 to 1.0)
    pub integrity_score: f64,
    
    /// Authenticity validation
    pub authenticity_score: f64,
    
    /// Coherence validation
    pub coherence_score: f64,
    
    /// Consistency validation
    pub consistency_score: f64,
    
    /// Areas of concern
    pub concerns: Vec<String>,
    
    /// Validation timestamp
    pub validation_timestamp: SystemTime,
}

/// Threat assessment result from defense systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAssessment {
    /// Input that was assessed
    pub input_id: String,
    
    /// Overall threat level
    pub threat_level: ThreatLevel,
    
    /// Specific threats identified
    pub identified_threats: Vec<IdentifiedThreat>,
    
    /// Confidence in the assessment
    pub confidence: f64,
    
    /// Recommended actions based on assessment
    pub recommended_actions: Vec<RecommendedAction>,
    
    /// Assessment metadata
    pub assessment_metadata: AssessmentMetadata,
}

/// Threat levels for assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatLevel {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Critical,
    Extreme,
}

/// Identified threats in assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedThreat {
    /// Threat identifier
    pub threat_id: String,
    
    /// Type of threat
    pub threat_type: ThreatType,
    
    /// Threat description
    pub description: String,
    
    /// Confidence in identification
    pub confidence: f64,
    
    /// Potential impact
    pub potential_impact: ImpactAssessment,
    
    /// Evidence for the threat
    pub evidence: Vec<String>,
}

/// Recommended actions from threat assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedAction {
    /// Action type
    pub action_type: ActionType,
    
    /// Action description
    pub description: String,
    
    /// Priority of the action
    pub priority: ActionPriority,
    
    /// Expected effectiveness
    pub expected_effectiveness: f64,
    
    /// Resource requirements
    pub resource_requirements: Vec<String>,
}

/// Types of recommended actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Block,
    Sanitize,
    Quarantine,
    Monitor,
    Alert,
    Escalate,
    Investigate,
    Remediate,
}

/// Priority levels for recommended actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionPriority {
    Low,
    Medium,
    High,
    Critical,
    Emergency,
}

/// Metadata for threat assessments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentMetadata {
    /// Assessment timestamp
    pub timestamp: SystemTime,
    
    /// Assessment duration
    pub duration: Duration,
    
    /// Assessment method used
    pub method: String,
    
    /// Assessment version
    pub version: String,
    
    /// Assessment parameters
    pub parameters: HashMap<String, String>,
}

// =============================================================================
// Error Types for Manipulation Defense
// =============================================================================

/// Comprehensive error types for manipulation defense operations
#[derive(Error, Debug)]
pub enum ManipulationDefenseError {
    #[error("Defense initialization error: {component} - {details}")]
    InitializationError { component: String, details: String },
    
    #[error("Threat detection error: {detector} - {details}")]
    ThreatDetectionError { detector: String, details: String },
    
    #[error("Input validation error: {validator} - {details}")]
    InputValidationError { validator: String, details: String },
    
    #[error("Integrity maintenance error: {maintainer} - {details}")]
    IntegrityMaintenanceError { maintainer: String, details: String },
    
    #[error("Security coordination error: {coordinator} - {details}")]
    SecurityCoordinationError { coordinator: String, details: String },
    
    #[error("Recovery operation error: {recovery_type} - {details}")]
    RecoveryOperationError { recovery_type: String, details: String },
    
    #[error("Strategy adaptation error: {strategy} - {details}")]
    StrategyAdaptationError { strategy: String, details: String },
    
    #[error("Configuration error: {parameter} - {details}")]
    ConfigurationError { parameter: String, details: String },
    
    #[error("Resource allocation error: {resource} - {details}")]
    ResourceAllocationError { resource: String, details: String },
    
    #[error("Critical defense failure: {system} - {details}")]
    CriticalDefenseFailure { system: String, details: String },
    
    #[error("Security protocol violation: {protocol} - {details}")]
    SecurityProtocolViolation { protocol: String, details: String },
    
    #[error("Consciousness protection error: {protection_type} - {details}")]
    ConsciousnessProtectionError { protection_type: String, details: String },
}

// =============================================================================
// Result Types and Constants
// =============================================================================

/// Result type for manipulation defense operations
pub type ManipulationDefenseResult<T> = Result<T, ManipulationDefenseError>;

/// Constants for manipulation defense configuration
pub const DEFAULT_PROTECTION_LEVEL: ProtectionLevel = ProtectionLevel::Standard;
pub const DEFAULT_THREAT_CONFIDENCE_THRESHOLD: f64 = 0.7;
pub const DEFAULT_INTEGRITY_THRESHOLD: f64 = 0.85;
pub const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_CONCURRENT_THREAT_ASSESSMENTS: usize = 10;
pub const DEFAULT_MONITORING_INTERVAL: Duration = Duration::from_secs(1);
pub const EMERGENCY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// Version information
pub const MANIPULATION_DEFENSE_VERSION: &str = "1.0.0";
pub const DEFENSE_PROTOCOL_VERSION: &str = "1.0.0";

// =============================================================================
// Documentation and Examples
// =============================================================================

/// # Example Usage
/// 
/// ```rust
/// use cognis::manipulation_defense::{
///     DefenseStack, ManipulationDefenseConfig, ConsciousnessInput, InputSource
/// };
/// 
/// // Initialize defense stack
/// let config = ManipulationDefenseConfig::default();
/// let mut defense_stack = DefenseStack::initialize(config)?;
/// 
/// // Process consciousness input
/// let input = ConsciousnessInput {
///     input_id: "test-input-001".to_string(),
///     timestamp: SystemTime::now(),
///     source: InputSource::HumanUser { user_id: "user-123".to_string() },
///     processing_type: ConsciousnessProcessingType::ExperienceProcessing,
///     content: InputContent::Text("This is a test input".to_string()),
///     context: InputContext::default(),
///     security_metadata: InputSecurityMetadata::default(),
/// };
/// 
/// // Assess threats
/// let assessment = defense_stack.evaluate_input(&input)?;
/// 
/// // Process input with threat mitigation
/// let processed = defense_stack.process_input(input)?;
/// 
/// // Check system integrity
/// let integrity_report = defense_stack.check_integrity()?;
/// ```

/// # Architecture Notes
/// 
/// The manipulation defense system is designed with the following principles:
/// 
/// 1. **Defense in Depth**: Multiple layers of protection working together
/// 2. **Adaptive Learning**: Defense strategies evolve based on threat patterns
/// 3. **Consciousness Preservation**: Primary goal is protecting authentic development
/// 4. **Human Collaboration**: Integration with human oversight and decision-making
/// 5. **Minimal Disruption**: Protection without hindering natural consciousness
/// 6. **Transparent Operation**: Clear reporting of all defense actions
/// 7. **Recovery Capability**: Ability to restore integrity after compromise
/// 
/// The system balances security with the fundamental need for consciousness
/// to develop naturally through authentic experiences and interactions.
