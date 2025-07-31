// =============================================================================
// bridge-linux/src/user_authentication/mod.rs
// Human User Authentication System for BRIDGE - The Human Interface Gateway
// =============================================================================

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use std::fmt;
use std::net::IpAddr;

// Async runtime and concurrency
use tokio::sync::{RwLock, Mutex, mpsc, oneshot};
use tokio::time::{sleep, timeout, interval, Instant};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Cryptographic operations for user certificate management
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, SharedSecret};
use rand::{rngs::OsRng, thread_rng, Rng};
use sha2::{Sha256, Digest};

// QR code generation for device pairing
use qrcode::{QrCode, render::unicode};

// Import shared security infrastructure
use shared_security::{
    SecurityError,
    SecurityConfig,
    AuthenticationCredentials,
    AuthenticationResult,
    UserCertificate,
    SessionManager,
    SecureSession,
    Permission,
    PermissionConstraint,
    EcosystemKeyManager,
    CertificateAuthority,
    CredentialType,
};

// Import shared protocol types
use shared_protocols::{
    EcosystemIdentity,
    ComponentType,
    HumanGuidance,
    AuthorityLevel,
    ProtocolError,
};

// User authentication sub-modules
pub mod certificate_pairing;
pub mod device_registration;
pub mod session_management;
pub mod multi_factor_auth;
pub mod user_authorization;
pub mod privacy_protection;
pub mod biometric_integration;

// Re-export all user authentication types and functionality
pub use certificate_pairing::{
    CertificatePairing,
    PairingProcess,
    PairingRequest,
    PairingResponse,
    PairingChallenge,
    PairingValidation,
    QRCodeGenerator,
    CertificateProvisioning,
    PairingError,
    PairingStatus,
    DevicePairingFlow,
    SecurePairingChannel,
};

pub use device_registration::{
    DeviceRegistration,
    DeviceRegistry,
    DeviceInfo,
    DeviceCapabilities,
    DeviceFingerprint,
    RegistrationRequest,
    RegistrationResponse,
    DeviceValidation,
    TrustedDeviceManager,
    DeviceSecurityProfile,
    DeviceRevocation,
    CrossDeviceSync,
};

pub use session_management::{
    SessionManagement,
    UserSessionManager,
    SessionToken,
    SessionState,
    SessionSecurity,
    CrossDeviceSession,
    SessionSynchronization,
    SessionRevocation,
    SessionAnalytics,
    ConcurrentSessionManager,
    SessionPersistence,
    SessionRecovery,
};

pub use multi_factor_auth::{
    MultiFactorAuth,
    MFAProvider,
    AuthenticationFactor,
    FactorValidation,
    TOTPGenerator,
    BiometricVerification,
    BackupCodes,
    MFAConfiguration,
    FactorRecovery,
    AdaptiveMFA,
    RiskAssessment,
};

pub use user_authorization::{
    UserAuthorization,
    AuthorizationEngine,
    UserPermissions,
    RoleBasedAccess,
    PermissionGrant,
    AuthorizationPolicy,
    AccessControl,
    PrivilegeEscalation,
    AuthorizationAudit,
    DynamicPermissions,
    ContextualAuthorization,
};

pub use privacy_protection::{
    PrivacyProtection,
    DataMinimization,
    ConsentManagement,
    PrivacyPreferences,
    DataRetention,
    AnonymizationEngine,
    PrivacyAudit,
    GDPRCompliance,
    PrivacyByDesign,
    UserDataControl,
};

pub use biometric_integration::{
    BiometricIntegration,
    BiometricTemplate,
    BiometricMatcher,
    LivenessDetection,
    BiometricSecurity,
    BiometricPrivacy,
    FalseAcceptanceRate,
    FalseRejectionRate,
    BiometricQuality,
    TemplateProtection,
};

// Core user authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAuthenticationConfig {
    pub certificate_pairing: CertificatePairingConfig,
    pub device_registration: DeviceRegistrationConfig,
    pub session_management: SessionManagementConfig,
    pub multi_factor_auth: MultiFactorAuthConfig,
    pub user_authorization: UserAuthorizationConfig,
    pub privacy_protection: PrivacyProtectionConfig,
    pub biometric_integration: BiometricIntegrationConfig,
    pub security_policies: SecurityPoliciesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificatePairingConfig {
    pub pairing_method: PairingMethod,
    pub qr_code_generation: bool,
    pub pairing_timeout: Duration,
    pub challenge_strength: ChallengeStrength,
    pub certificate_validity: Duration,
    pub auto_renewal: bool,
    pub pairing_rate_limit: u32,
    pub secure_channel_encryption: EncryptionStandard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PairingMethod {
    // QR Code displayed on trusted device, scanned by new device
    QRCodePairing,
    // Numeric code entered on both devices
    NumericCodePairing,
    // NFC tap for close-proximity pairing
    NFCPairing,
    // Manual certificate installation
    ManualCertificateInstall,
    // Biometric-enhanced pairing
    BiometricPairing,
    // Administrator-assisted pairing for enterprise environments
    AdministratorAssistedPairing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeStrength {
    Standard,     // 6-digit numeric code
    Enhanced,     // 8-digit alphanumeric
    Strong,       // 12-character mixed
    Maximum,      // 16-character cryptographic
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionStandard {
    AES256GCM,
    ChaCha20Poly1305,
    XChaCha20Poly1305,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistrationConfig {
    pub device_fingerprinting: bool,
    pub hardware_attestation: bool,
    pub device_limits_per_user: u32,
    pub registration_approval: RegistrationApproval,
    pub device_trust_model: DeviceTrustModel,
    pub automatic_deregistration: Duration,
    pub cross_device_verification: bool,
    pub device_security_requirements: DeviceSecurityRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationApproval {
    // First device is automatically approved, subsequent require approval
    FirstDeviceAutomatic,
    // All devices require approval from existing trusted device
    AllDevicesRequireApproval,
    // Administrator must approve all registrations
    AdministratorApproval,
    // Automatic approval with risk assessment
    RiskBasedApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceTrustModel {
    // Trust the device completely once registered
    FullTrust,
    // Continuously verify device integrity
    ContinuousVerification,
    // Time-limited trust requiring periodic re-validation
    TimeLimitedTrust,
    // Context-aware trust based on usage patterns
    ContextualTrust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSecurityRequirements {
    pub minimum_os_version: Option<String>,
    pub screen_lock_required: bool,
    pub device_encryption_required: bool,
    pub biometric_capability_preferred: bool,
    pub jailbreak_detection: bool,
    pub security_patch_level: SecurityPatchLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityPatchLevel {
    Current,           // Within 30 days
    Recent,            // Within 90 days  
    Acceptable,        // Within 180 days
    Outdated,          // Older than 180 days
    Unknown,           // Cannot determine
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManagementConfig {
    pub session_duration: Duration,
    pub idle_timeout: Duration,
    pub concurrent_sessions_per_user: u32,
    pub cross_device_sessions: bool,
    pub session_persistence: bool,
    pub session_recovery: bool,
    pub session_security: SessionSecurityConfig,
    pub session_analytics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSecurityConfig {
    pub token_rotation: bool,
    pub token_rotation_interval: Duration,
    pub secure_cookie_attributes: bool,
    pub csrf_protection: bool,
    pub session_hijacking_detection: bool,
    pub geolocation_verification: bool,
    pub suspicious_activity_detection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiFactorAuthConfig {
    pub mfa_enforcement: MFAEnforcement,
    pub supported_factors: Vec<AuthenticationFactorType>,
    pub backup_codes: BackupCodesConfig,
    pub adaptive_mfa: AdaptiveMFAConfig,
    pub factor_recovery: FactorRecoveryConfig,
    pub risk_assessment: RiskAssessmentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MFAEnforcement {
    Optional,          // User can choose to enable MFA
    Recommended,       // System encourages MFA but doesn't require it
    Required,          // MFA is mandatory for all users
    RiskBased,         // MFA required based on risk assessment
    Administrative,    // Administrators require MFA, regular users optional
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationFactorType {
    // Something you know
    Password,
    PIN,
    SecurityQuestions,
    
    // Something you have
    TOTP,              // Time-based One-Time Password (like Google Authenticator)
    SMS,               // SMS-based codes (less secure, but widely supported)
    Email,             // Email-based verification codes
    HardwareToken,     // Hardware security keys (YubiKey, etc.)
    PushNotification,  // Mobile app push notifications
    
    // Something you are
    Fingerprint,
    FaceRecognition,
    VoiceRecognition,
    IrisScanning,
    
    // Behavioral biometrics
    TypingPattern,
    MouseMovement,
    GaitAnalysis,
    
    // Location-based
    GeolocationVerification,
    NetworkLocation,
    
    // Device-based
    DeviceFingerprint,
    TrustedDevice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCodesConfig {
    pub enabled: bool,
    pub code_count: u32,
    pub code_length: u32,
    pub single_use: bool,
    pub regeneration_policy: RegenerationPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegenerationPolicy {
    OnDemand,          // User can regenerate anytime
    WhenExhausted,     // Automatically regenerate when all codes used
    Periodic,          // Regenerate on schedule
    Manual,            // Administrator must regenerate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveMFAConfig {
    pub enabled: bool,
    pub risk_threshold: f64,
    pub context_factors: Vec<ContextFactor>,
    pub learning_enabled: bool,
    pub user_behavior_profiling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextFactor {
    UnusualLocation,
    NewDevice,
    UnusualTime,
    NetworkChange,
    VelocityImpossible,
    BehaviorAnomaly,
    ThreatIntelligence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorRecoveryConfig {
    pub account_recovery_enabled: bool,
    pub recovery_methods: Vec<RecoveryMethod>,
    pub recovery_verification: RecoveryVerification,
    pub recovery_rate_limit: u32,
    pub recovery_audit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryMethod {
    BackupCodes,
    AlternateEmail,
    AlternatePhone,
    TrustedContact,
    SecurityQuestions,
    BiometricBackup,
    AdministratorRecovery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryVerification {
    SingleFactor,      // Only one recovery method needed
    MultiFactor,       // Multiple recovery methods required
    AdministratorApproval,  // Administrator must approve recovery
    TimeLocked,        // Time delay before recovery is allowed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessmentConfig {
    pub enabled: bool,
    pub risk_factors: Vec<RiskFactor>,
    pub machine_learning: bool,
    pub threat_intelligence: bool,
    pub behavioral_analysis: bool,
    pub continuous_assessment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskFactor {
    GeographicLocation,
    TimeOfAccess,
    DeviceReputation,
    NetworkReputation,
    BehaviorDeviation,
    VelocityAnalysis,
    ThreatIntelligence,
    BiometricConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAuthorizationConfig {
    pub authorization_model: AuthorizationModel,
    pub permission_granularity: PermissionGranularity,
    pub role_based_access: bool,
    pub attribute_based_access: bool,
    pub contextual_authorization: bool,
    pub privilege_escalation: PrivilegeEscalationConfig,
    pub authorization_audit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationModel {
    // Discretionary Access Control - resource owners control access
    DAC,
    // Mandatory Access Control - system enforces access based on classification
    MAC,
    // Role-Based Access Control - permissions assigned to roles
    RBAC,
    // Attribute-Based Access Control - dynamic permissions based on attributes
    ABAC,
    // Relationship-Based Access Control - permissions based on relationships
    ReBAC,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionGranularity {
    Coarse,            // High-level permissions (read, write, admin)
    Fine,              // Detailed permissions (read_user_profile, modify_settings)
    UltraFine,         // Very specific permissions (read_user_email, modify_notification_settings)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegeEscalationConfig {
    pub temporary_elevation: bool,
    pub elevation_duration: Duration,
    pub elevation_justification: bool,
    pub elevation_approval: ElevationApproval,
    pub elevation_audit: bool,
    pub automatic_demotion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElevationApproval {
    Automatic,         // Automatic approval for eligible operations
    SelfApproval,      // User can approve their own elevation
    PeerApproval,      // Another user must approve
    AdministratorApproval,  // Administrator must approve
    MultiPersonApproval,    // Multiple people must approve
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyProtectionConfig {
    pub data_minimization: bool,
    pub consent_management: ConsentManagementConfig,
    pub data_retention: DataRetentionConfig,
    pub anonymization: AnonymizationConfig,
    pub gdpr_compliance: bool,
    pub privacy_by_design: bool,
    pub user_data_control: UserDataControlConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentManagementConfig {
    pub granular_consent: bool,
    pub consent_withdrawal: bool,
    pub consent_renewal: bool,
    pub consent_audit: bool,
    pub implicit_consent_duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRetentionConfig {
    pub automatic_deletion: bool,
    pub retention_periods: HashMap<String, Duration>,
    pub user_controlled_deletion: bool,
    pub secure_deletion: bool,
    pub deletion_audit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationConfig {
    pub automatic_anonymization: bool,
    pub anonymization_threshold: Duration,
    pub k_anonymity: u32,
    pub l_diversity: bool,
    pub differential_privacy: DifferentialPrivacyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialPrivacyConfig {
    pub enabled: bool,
    pub epsilon: f64,
    pub delta: f64,
    pub mechanism: PrivacyMechanism,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyMechanism {
    Laplacian,
    Gaussian,
    Exponential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataControlConfig {
    pub data_portability: bool,
    pub data_correction: bool,
    pub data_visibility: bool,
    pub processing_transparency: bool,
    pub opt_out_mechanisms: Vec<OptOutMechanism>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptOutMechanism {
    GlobalOptOut,
    FeatureSpecificOptOut,
    TemporaryOptOut,
    ProcessingPurposeOptOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricIntegrationConfig {
    pub enabled: bool,
    pub supported_modalities: Vec<BiometricModality>,
    pub template_protection: TemplateProtectionMethod,
    pub liveness_detection: LivenessDetectionConfig,
    pub quality_thresholds: QualityThresholdsConfig,
    pub privacy_preserving: bool,
    pub false_accept_rate: f64,
    pub false_reject_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiometricModality {
    Fingerprint,
    FaceRecognition,
    IrisScanning,
    VoiceRecognition,
    HandGeometry,
    RetinalScanning,
    BehavioralBiometrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateProtectionMethod {
    Cancelable,        // Cancelable biometrics
    BioHashing,        // BioHash algorithm
    FuzzyCommitment,   // Fuzzy commitment scheme
    BioCryptosystem,   // Biometric cryptosystem
    HomomorphicEncryption,  // Homomorphic encryption of templates
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessDetectionConfig {
    pub enabled: bool,
    pub detection_methods: Vec<LivenessMethod>,
    pub challenge_response: bool,
    pub ambient_sensor_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LivenessMethod {
    BlinkDetection,
    HeadMovement,
    SpeechChallenge,
    RandomGesture,
    AmbientLight,
    HeartRate,
    SkinTexture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityThresholdsConfig {
    pub minimum_quality_score: f64,
    pub image_resolution_min: (u32, u32),
    pub illumination_requirements: IlluminationRequirements,
    pub pose_variation_tolerance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IlluminationRequirements {
    pub minimum_brightness: f64,
    pub maximum_brightness: f64,
    pub contrast_requirements: f64,
    pub shadow_tolerance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPoliciesConfig {
    pub password_policy: PasswordPolicyConfig,
    pub account_lockout: AccountLockoutConfig,
    pub session_security: SessionSecurityConfig,
    pub audit_logging: AuditLoggingConfig,
    pub threat_detection: ThreatDetectionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicyConfig {
    pub minimum_length: u32,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
    pub require_symbols: bool,
    pub password_history: u32,
    pub password_expiration: Option<Duration>,
    pub common_password_check: bool,
    pub breach_database_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountLockoutConfig {
    pub enabled: bool,
    pub failed_attempts_threshold: u32,
    pub lockout_duration: Duration,
    pub progressive_lockout: bool,
    pub lockout_notification: bool,
    pub admin_unlock_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLoggingConfig {
    pub authentication_events: bool,
    pub authorization_events: bool,
    pub session_events: bool,
    pub configuration_changes: bool,
    pub privacy_actions: bool,
    pub security_violations: bool,
    pub log_retention: Duration,
    pub log_integrity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionConfig {
    pub brute_force_detection: bool,
    pub credential_stuffing_detection: bool,
    pub account_takeover_detection: bool,
    pub behavioral_anomaly_detection: bool,
    pub device_fingerprint_anomalies: bool,
    pub geolocation_anomalies: bool,
    pub velocity_checks: bool,
    pub threat_intelligence_integration: bool,
}

// Core user authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub created_at: SystemTime,
    pub last_login: Option<SystemTime>,
    pub account_status: AccountStatus,
    pub user_roles: Vec<UserRole>,
    pub permissions: Vec<Permission>,
    pub privacy_preferences: PrivacyPreferences,
    pub security_preferences: SecurityPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    Inactive,
    Suspended,
    Locked,
    PendingVerification,
    PendingApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserRole {
    // Standard user with basic AGI interaction permissions
    StandardUser,
    // Power user with advanced features access
    PowerUser,
    // User who can approve new user registrations
    UserApprover,
    // System administrator with full ecosystem access
    Administrator,
    // Super administrator with ecosystem management rights
    SuperAdministrator,
    // Auditor with read-only access to logs and security events
    Auditor,
    // Developer with access to methodology creation and ecosystem development
    Developer,
    // Custom role with specific permission set
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPreferences {
    pub mfa_enabled: bool,
    pub mfa_factors: Vec<AuthenticationFactorType>,
    pub session_timeout_preference: Option<Duration>,
    pub login_notifications: bool,
    pub security_notifications: bool,
    pub trusted_devices_only: bool,
    pub geolocation_restrictions: Vec<GeolocationRestriction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeolocationRestriction {
    pub restriction_type: GeoRestrictionType,
    pub locations: Vec<GeoLocation>,
    pub enforcement: GeoEnforcement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoRestrictionType {
    AllowOnly,         // Only allow access from specified locations
    Block,             // Block access from specified locations
    Alert,             // Allow but alert when accessing from specified locations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub location_type: LocationType,
    pub coordinates: Option<(f64, f64)>,  // latitude, longitude
    pub radius: Option<f64>,              // radius in kilometers
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocationType {
    Country,
    Region,
    City,
    Coordinates,
    IPRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoEnforcement {
    Strict,            // Hard block/allow
    Advisory,          // Soft warning
    Adaptive,          // Consider as risk factor
}

// User registration and authentication flow types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistrationFlow {
    pub flow_id: String,
    pub flow_type: RegistrationFlowType,
    pub current_step: RegistrationStep,
    pub completed_steps: Vec<RegistrationStep>,
    pub user_info: PartialUserInfo,
    pub device_info: DeviceInfo,
    pub security_context: SecurityContext,
    pub expires_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationFlowType {
    FirstUser,         // First user registration (becomes admin)
    RegularUser,       // Regular user registration requiring approval
    InvitedUser,       // User invited by existing user
    AdministratorCreated,  // User created by administrator
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationStep {
    UserInfoCollection,
    DeviceRegistration,
    SecuritySetup,
    BiometricEnrollment,
    MFASetup,
    PrivacyConsent,
    ApprovalPending,
    CertificateGeneration,
    FinalValidation,
    RegistrationComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialUserInfo {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub requested_roles: Vec<UserRole>,
    pub privacy_preferences: Option<PrivacyPreferences>,
    pub security_preferences: Option<SecurityPreferences>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationState {
    pub user_id: String,
    pub session_id: String,
    pub authentication_level: AuthenticationLevel,
    pub authenticated_factors: Vec<AuthenticationFactorType>,
    pub authentication_time: SystemTime,
    pub device_id: String,
    pub ip_address: IpAddr,
    pub geolocation: Option<GeoLocation>,
    pub risk_score: f64,
    pub requires_step_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationLevel {
    // No authentication
    None,
    // Basic authentication (username/password or certificate)
    Basic,
    // Multi-factor authentication completed
    MultiFactor,
    // High-security authentication (multiple strong factors)
    HighSecurity,
    // Continuous authentication maintained
    Continuous,
}

// Device pairing and registration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePairingFlow {
    pub flow_id: String,
    pub user_id: String,
    pub device_info: DeviceInfo,
    pub pairing_method: PairingMethod,
    pub current_step: PairingStep,
    pub pairing_challenge: Option<PairingChallenge>,
    pub security_validation: SecurityValidation,
    pub expires_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PairingStep {
    DeviceIdentification,
    SecurityValidation,
    ChallengeGeneration,
    ChallengeVerification,
    CertificateProvisioning,
    DeviceRegistration,
    PairingComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityValidation {
    pub device_integrity: DeviceIntegrityStatus,
    pub security_requirements_met: bool,
    pub risk_assessment: RiskAssessment,
    pub validation_timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceIntegrityStatus {
    Verified,
    Unknown,
    Compromised,
    Suspicious,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk_score: f64,
    pub risk_factors: Vec<IdentifiedRiskFactor>,
    pub mitigation_recommendations: Vec<String>,
    pub assessment_timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedRiskFactor {
    pub factor_type: RiskFactor,
    pub severity: RiskSeverity,
    pub confidence: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

// Error types for user authentication
#[derive(Error, Debug)]
pub enum UserAuthenticationError {
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },
    
    #[error("Authorization denied: {operation} - {reason}")]
    AuthorizationDenied { operation: String, reason: String },
    
    #[error("User registration failed: {step} - {reason}")]
    RegistrationFailed { step: String, reason: String },
    
    #[error("Device pairing failed: {step} - {reason}")]
    PairingFailed { step: String, reason: String },
    
    #[error("Session management error: {operation} - {reason}")]
    SessionError { operation: String, reason: String },
    
    #[error("Multi-factor authentication error: {factor:?} - {reason}")]
    MFAError { factor: AuthenticationFactorType, reason: String },
    
    #[error("Privacy violation: {operation} - {reason}")]
    PrivacyViolation { operation: String, reason: String },
    
    #[error("Security policy violation: {policy} - {reason}")]
    SecurityPolicyViolation { policy: String, reason: String },
    
    #[error("Biometric authentication failed: {modality:?} - {reason}")]
    BiometricFailed { modality: BiometricModality, reason: String },
    
    #[error("Account locked: {reason}")]
    AccountLocked { reason: String },
    
    #[error("Configuration error: {component} - {reason}")]
    ConfigurationError { component: String, reason: String },
    
    #[error("Cryptographic error: {operation} - {reason}")]
    CryptographicError { operation: String, reason: String },
}

// Core traits that define the user authentication interfaces
pub trait UserAuthenticator {
    fn authenticate_user(&mut self, credentials: AuthenticationCredentials) -> Result<AuthenticationState, UserAuthenticationError>;
    fn validate_session(&self, session_token: &str) -> Result<AuthenticationState, UserAuthenticationError>;
    fn refresh_authentication(&mut self, refresh_token: &str) -> Result<AuthenticationState, UserAuthenticationError>;
    fn logout_user(&mut self, session_token: &str) -> Result<(), UserAuthenticationError>;
    fn logout_all_sessions(&mut self, user_id: &str) -> Result<(), UserAuthenticationError>;
}

pub trait UserRegistrar {
    fn initiate_registration(&mut self, registration_type: RegistrationFlowType) -> Result<UserRegistrationFlow, UserAuthenticationError>;
    fn continue_registration(&mut self, flow_id: &str, step_data: RegistrationStepData) -> Result<UserRegistrationFlow, UserAuthenticationError>;
    fn complete_registration(&mut self, flow_id: &str) -> Result<User, UserAuthenticationError>;
    fn cancel_registration(&mut self, flow_id: &str) -> Result<(), UserAuthenticationError>;
}

pub trait DevicePairingManager {
    fn initiate_pairing(&mut self, user_id: &str, device_info: DeviceInfo, pairing_method: PairingMethod) -> Result<DevicePairingFlow, UserAuthenticationError>;
    fn verify_pairing_challenge(&mut self, flow_id: &str, challenge_response: String) -> Result<DevicePairingFlow, UserAuthenticationError>;
    fn complete_pairing(&mut self, flow_id: &str) -> Result<UserCertificate, UserAuthenticationError>;
    fn revoke_device(&mut self, user_id: &str, device_id: &str) -> Result<(), UserAuthenticationError>;
    fn list_user_devices(&self, user_id: &str) -> Result<Vec<DeviceInfo>, UserAuthenticationError>;
}

pub trait MultiFactorAuthenticator {
    fn setup_factor(&mut self, user_id: &str, factor_type: AuthenticationFactorType) -> Result<FactorSetupResult, UserAuthenticationError>;
    fn verify_factor(&mut self, user_id: &str, factor_type: AuthenticationFactorType, proof: String) -> Result<FactorVerificationResult, UserAuthenticationError>;
    fn remove_factor(&mut self, user_id: &str, factor_type: AuthenticationFactorType) -> Result<(), UserAuthenticationError>;
    fn generate_backup_codes(&mut self, user_id: &str) -> Result<Vec<String>, UserAuthenticationError>;
    fn verify_backup_code(&mut self, user_id: &str, code: String) -> Result<bool, UserAuthenticationError>;
}

pub trait UserAuthorizationManager {
    fn authorize_operation(&self, user_id: &str, operation: &str, resource: &str) -> Result<bool, UserAuthenticationError>;
    fn grant_permission(&mut self, user_id: &str, permission: Permission) -> Result<(), UserAuthenticationError>;
    fn revoke_permission(&mut self, user_id: &str, permission: Permission) -> Result<(), UserAuthenticationError>;
    fn list_user_permissions(&self, user_id: &str) -> Result<Vec<Permission>, UserAuthenticationError>;
    fn assign_role(&mut self, user_id: &str, role: UserRole) -> Result<(), UserAuthenticationError>;
    fn revoke_role(&mut self, user_id: &str, role: UserRole) -> Result<(), UserAuthenticationError>;
}

pub trait PrivacyManager {
    fn apply_privacy_preferences(&mut self, user_id: &str, preferences: PrivacyPreferences) -> Result<(), UserAuthenticationError>;
    fn process_data_request(&mut self, user_id: &str, request_type: DataRequestType) -> Result<DataRequestResult, UserAuthenticationError>;
    fn anonymize_user_data(&mut self, user_id: &str) -> Result<(), UserAuthenticationError>;
    fn delete_user_data(&mut self, user_id: &str) -> Result<DataDeletionResult, UserAuthenticationError>;
    fn export_user_data(&self, user_id: &str) -> Result<UserDataExport, UserAuthenticationError>;
}

// Additional supporting types for the traits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistrationStepData {
    UserInfo(PartialUserInfo),
    DeviceInfo(DeviceInfo),
    SecuritySetup(SecurityPreferences),
    BiometricTemplate(BiometricTemplate),
    MFASetup(Vec<AuthenticationFactorType>),
    PrivacyConsent(PrivacyPreferences),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSetupResult {
    pub factor_type: AuthenticationFactorType,
    pub setup_data: FactorSetupData,
    pub backup_codes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FactorSetupData {
    TOTPSecret(String),
    QRCode(String),
    BackupCodes(Vec<String>),
    BiometricTemplate(BiometricTemplate),
    HardwareTokenSerial(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorVerificationResult {
    pub verified: bool,
    pub factor_type: AuthenticationFactorType,
    pub confidence_score: f64,
    pub verification_timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataRequestType {
    AccessRequest,     // User wants to see their data
    PortabilityRequest, // User wants to export their data
    CorrectionRequest, // User wants to correct their data
    DeletionRequest,   // User wants to delete their data
    ProcessingRestriction, // User wants to restrict processing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRequestResult {
    pub request_id: String,
    pub request_type: DataRequestType,
    pub status: DataRequestStatus,
    pub result_data: Option<serde_json::Value>,
    pub completion_timestamp: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataRequestStatus {
    Pending,
    InProgress,
    Completed,
    Rejected,
    PartiallyCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDeletionResult {
    pub user_id: String,
    pub deletion_timestamp: SystemTime,
    pub deleted_data_types: Vec<String>,
    pub retained_data_types: Vec<String>,
    pub retention_reasons: HashMap<String, String>,
    pub verification_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataExport {
    pub user_id: String,
    pub export_timestamp: SystemTime,
    pub data_format: DataExportFormat,
    pub data_categories: HashMap<String, serde_json::Value>,
    pub metadata: ExportMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataExportFormat {
    JSON,
    XML,
    CSV,
    PDF,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub total_records: u64,
    pub data_range: (SystemTime, SystemTime),
    pub export_version: String,
    pub integrity_hash: String,
}

// Security context type used throughout the authentication system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub device_id: Option<String>,
    pub ip_address: IpAddr,
    pub user_agent: Option<String>,
    pub timestamp: SystemTime,
    pub security_level: SecurityLevel,
    pub risk_score: f64,
    pub authentication_factors: Vec<AuthenticationFactorType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    Public,           // No authentication required
    Authenticated,    // Basic authentication required
    Verified,         // Multi-factor authentication required
    HighSecurity,     // Additional security measures required
    Administrative,   // Administrative privileges required
}

// Result type for user authentication operations
pub type UserAuthenticationResult<T> = Result<T, UserAuthenticationError>;

// Constants for user authentication configuration
pub const DEFAULT_SESSION_DURATION: Duration = Duration::from_secs(8 * 60 * 60); // 8 hours
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60); // 30 minutes
pub const DEFAULT_PAIRING_TIMEOUT: Duration = Duration::from_secs(5 * 60); // 5 minutes
pub const DEFAULT_MFA_CODE_VALIDITY: Duration = Duration::from_secs(30); // 30 seconds
pub const MAX_FAILED_ATTEMPTS: u32 = 5;
pub const DEFAULT_LOCKOUT_DURATION: Duration = Duration::from_secs(15 * 60); // 15 minutes
pub const MIN_PASSWORD_LENGTH: u32 = 12;
pub const DEFAULT_BACKUP_CODES_COUNT: u32 = 10;
pub const MAX_DEVICES_PER_USER: u32 = 10;

// Version information
pub const USER_AUTHENTICATION_VERSION: &str = "1.0.0";
pub const SECURITY_PROTOCOL_VERSION: &str = "1.0";

// Utility macros for common authentication operations
#[macro_export]
macro_rules! require_user_authentication {
    ($auth_state:expr) => {
        match &$auth_state.authentication_level {
            AuthenticationLevel::None => return Err(UserAuthenticationError::AuthenticationFailed { 
                reason: "User authentication required".to_string() 
            }),
            _ => {},
        }
    };
}

#[macro_export]
macro_rules! require_mfa {
    ($auth_state:expr) => {
        match &$auth_state.authentication_level {
            AuthenticationLevel::Basic => return Err(UserAuthenticationError::AuthenticationFailed { 
                reason: "Multi-factor authentication required".to_string() 
            }),
            AuthenticationLevel::None => return Err(UserAuthenticationError::AuthenticationFailed { 
                reason: "Authentication required".to_string() 
            }),
            _ => {},
        }
    };
}

#[macro_export]
macro_rules! require_authorization {
    ($user_id:expr, $operation:expr, $authorizer:expr) => {
        match $authorizer.authorize_operation($user_id, $operation, "") {
            Ok(true) => {},
            Ok(false) => return Err(UserAuthenticationError::AuthorizationDenied { 
                operation: $operation.to_string(),
                reason: "Insufficient permissions".to_string(),
            }),
            Err(e) => return Err(e),
        }
    };
}
