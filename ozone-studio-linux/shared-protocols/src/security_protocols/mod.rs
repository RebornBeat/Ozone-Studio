// =============================================================================
// shared-protocols/src/security_protocols.rs
// Security Protocols for OZONE STUDIO Ecosystem Communication
// 
// This module implements comprehensive security protocols that ensure all
// communication within the OZONE STUDIO ecosystem is authenticated, authorized,
// and encrypted. It provides the foundation for secure AI App coordination,
// user authentication, and data protection across all ecosystem components.
// =============================================================================

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use std::fmt;

// Cryptographic and security dependencies
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, SharedSecret};
use aes_gcm::{Aes256Gcm, Key, Nonce, NewAead, Aead};
use rand::{rngs::OsRng, RngCore};
use sha2::{Sha256, Digest};
use blake3::Hasher as Blake3Hasher;

// Async and serialization
use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;
use thiserror::Error;

// Import from shared-security for core security types
use shared_security::{
    SecurityError,
    AuthenticationCredentials,
    AuthenticationResult,
    Permission,
    EncryptedMessage,
    EcosystemCertificate,
    AIAppCertificate,
    UserCertificate,
};

// Import component types from our protocol layer
use crate::{ComponentType, EcosystemIdentity};

// =============================================================================
// CORE SECURITY MESSAGE TYPES
// =============================================================================

/// SecureMessage is the foundational encrypted communication envelope used for
/// all secure communications within the OZONE STUDIO ecosystem. Every message
/// between components is wrapped in this structure to ensure confidentiality,
/// integrity, and authenticity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureMessage {
    /// Unique identifier for this secure message
    pub message_id: String,
    
    /// Type of secure message being transmitted
    pub message_type: SecureMessageType,
    
    /// Sender component identification and verification
    pub sender: ComponentIdentity,
    
    /// Recipient component identification
    pub recipient: ComponentIdentity,
    
    /// Encrypted payload containing the actual message content
    pub encrypted_payload: EncryptedPayload,
    
    /// Digital signature for message authentication and non-repudiation
    pub signature: MessageSignature,
    
    /// Security metadata including encryption parameters
    pub security_metadata: SecurityMetadata,
    
    /// Timestamp when the message was created
    pub timestamp: SystemTime,
    
    /// Message expiration time for replay attack prevention
    pub expires_at: SystemTime,
    
    /// Nonce for preventing replay attacks
    pub nonce: Vec<u8>,
}

/// Defines the different types of secure messages in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecureMessageType {
    /// Authentication request or response
    Authentication,
    
    /// Authorization request or response  
    Authorization,
    
    /// Key exchange for establishing secure channels
    KeyExchange,
    
    /// Certificate exchange and validation
    CertificateExchange,
    
    /// Encrypted data payload
    EncryptedData,
    
    /// Security handshake initiation or response
    SecurityHandshake,
    
    /// Heartbeat for connection liveness
    Heartbeat,
    
    /// Security audit log entry
    AuditLog,
    
    /// Emergency security alert
    SecurityAlert,
}

/// Component identity for secure message routing and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentIdentity {
    /// Component type (OZONE_STUDIO, BRIDGE, FORGE, etc.)
    pub component_type: ComponentType,
    
    /// Unique component instance identifier
    pub instance_id: String,
    
    /// Component's public key for verification
    pub public_key: Vec<u8>,
    
    /// Component's certificate for authentication
    pub certificate: Option<Vec<u8>>,
    
    /// Component version information
    pub version: String,
}

/// Encrypted payload container with integrity protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// Encryption algorithm used
    pub algorithm: EncryptionAlgorithm,
    
    /// Encrypted data
    pub ciphertext: Vec<u8>,
    
    /// Initialization vector or nonce
    pub iv: Vec<u8>,
    
    /// Authentication tag for integrity verification
    pub auth_tag: Vec<u8>,
    
    /// Additional authenticated data
    pub aad: Vec<u8>,
}

/// Supported encryption algorithms in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256 in Galois/Counter Mode
    AES256GCM,
    
    /// ChaCha20-Poly1305 AEAD
    ChaCha20Poly1305,
    
    /// XSalsa20-Poly1305
    XSalsa20Poly1305,
}

/// Digital signature for message authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSignature {
    /// Signature algorithm used
    pub algorithm: SignatureAlgorithm,
    
    /// Digital signature bytes
    pub signature: Vec<u8>,
    
    /// Hash of the signed content
    pub content_hash: Vec<u8>,
    
    /// Signer's public key for verification
    pub signer_public_key: Vec<u8>,
}

/// Supported signature algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    /// Ed25519 signature scheme
    Ed25519,
    
    /// ECDSA with P-256 curve
    ECDSAp256,
    
    /// RSA-PSS with SHA-256
    RSAPSS,
}

/// Security metadata for message processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetadata {
    /// Security protocol version
    pub protocol_version: String,
    
    /// Encryption parameters and settings
    pub encryption_params: EncryptionParameters,
    
    /// Key derivation information
    pub key_derivation: KeyDerivationInfo,
    
    /// Security context reference
    pub security_context_id: String,
    
    /// Priority level for security processing
    pub priority: SecurityPriority,
}

/// Encryption parameters for message processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionParameters {
    /// Key size in bits
    pub key_size: u32,
    
    /// Nonce/IV size in bytes
    pub nonce_size: u32,
    
    /// Authentication tag size in bytes
    pub tag_size: u32,
    
    /// Additional security flags
    pub flags: Vec<String>,
}

/// Key derivation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationInfo {
    /// Key derivation function used
    pub kdf: KeyDerivationFunction,
    
    /// Salt used in derivation
    pub salt: Vec<u8>,
    
    /// Iteration count for KDF
    pub iterations: u32,
    
    /// Derived key length
    pub key_length: u32,
}

/// Supported key derivation functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyDerivationFunction {
    /// PBKDF2 with HMAC-SHA256
    PBKDF2HMACSHA256,
    
    /// Scrypt key derivation
    Scrypt,
    
    /// Argon2id key derivation
    Argon2id,
    
    /// HKDF with SHA-256
    HKDFSHA256,
}

/// Security priority for message processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityPriority {
    /// Low priority, background processing
    Low,
    
    /// Normal priority
    Normal,
    
    /// High priority, expedited processing
    High,
    
    /// Critical priority, immediate processing
    Critical,
    
    /// Emergency priority, highest precedence
    Emergency,
}

// =============================================================================
// AUTHENTICATION PROTOCOLS
// =============================================================================

/// Authentication message for component and user authentication flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationMessage {
    /// Unique authentication request identifier
    pub auth_id: String,
    
    /// Type of authentication being performed
    pub auth_type: AuthenticationType,
    
    /// Authentication method being used
    pub auth_method: AuthenticationMethod,
    
    /// Authentication challenge or response data
    pub auth_data: AuthenticationData,
    
    /// Timestamp of authentication attempt
    pub timestamp: SystemTime,
    
    /// IP address of authentication origin (for audit)
    pub source_ip: Option<String>,
    
    /// Additional authentication context
    pub context: AuthenticationContext,
}

/// Types of authentication in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationType {
    /// AI App authentication with OZONE STUDIO
    AIAppAuthentication,
    
    /// User authentication with BRIDGE
    UserAuthentication,
    
    /// Service-to-service authentication
    ServiceAuthentication,
    
    /// Device authentication for cross-device coordination
    DeviceAuthentication,
    
    /// Emergency authentication for security incidents
    EmergencyAuthentication,
}

/// Authentication methods supported by the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    /// Certificate-based authentication
    Certificate {
        certificate: Vec<u8>,
        signature: Vec<u8>,
    },
    
    /// API key authentication
    APIKey {
        key_id: String,
        key_hash: Vec<u8>,
    },
    
    /// Multi-factor authentication
    MultiFactorAuth {
        factors: Vec<AuthenticationFactor>,
    },
    
    /// Biometric authentication
    Biometric {
        biometric_type: BiometricType,
        biometric_data: Vec<u8>,
    },
    
    /// Challenge-response authentication
    ChallengeResponse {
        challenge: Vec<u8>,
        response: Vec<u8>,
    },
}

/// Individual authentication factor for MFA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationFactor {
    /// Type of authentication factor
    pub factor_type: FactorType,
    
    /// Factor-specific data
    pub factor_data: Vec<u8>,
    
    /// Factor verification status
    pub verified: bool,
    
    /// Factor strength/confidence level
    pub strength: f64,
}

/// Types of authentication factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FactorType {
    /// Something you know (password, PIN)
    Knowledge,
    
    /// Something you have (token, certificate)
    Possession,
    
    /// Something you are (biometric)
    Inherence,
    
    /// Somewhere you are (location)
    Location,
    
    /// Something you do (behavioral pattern)
    Behavior,
}

/// Biometric authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiometricType {
    /// Fingerprint recognition
    Fingerprint,
    
    /// Facial recognition
    FaceRecognition,
    
    /// Voice recognition
    VoiceRecognition,
    
    /// Iris recognition
    IrisRecognition,
    
    /// Behavioral patterns
    BehavioralPattern,
}

/// Authentication context and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationContext {
    /// Session identifier if applicable
    pub session_id: Option<String>,
    
    /// Device information
    pub device_info: Option<DeviceInfo>,
    
    /// Geographic location if available
    pub location: Option<LocationInfo>,
    
    /// Risk assessment factors
    pub risk_factors: Vec<RiskFactor>,
    
    /// Previous authentication history
    pub auth_history: Vec<AuthenticationEvent>,
}

/// Device information for authentication context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device unique identifier
    pub device_id: String,
    
    /// Device type and model
    pub device_type: String,
    
    /// Operating system information
    pub os_info: String,
    
    /// Device fingerprint
    pub fingerprint: Vec<u8>,
    
    /// Device trust level
    pub trust_level: TrustLevel,
}

/// Geographic location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    /// Latitude coordinate
    pub latitude: f64,
    
    /// Longitude coordinate
    pub longitude: f64,
    
    /// Accuracy of location in meters
    pub accuracy: f64,
    
    /// Country code
    pub country: String,
    
    /// City or region
    pub city: String,
}

/// Risk factors for authentication assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Type of risk factor
    pub factor_type: RiskFactorType,
    
    /// Risk level (0.0 to 1.0)
    pub risk_level: f64,
    
    /// Description of the risk
    pub description: String,
    
    /// Mitigation recommendations
    pub mitigations: Vec<String>,
}

/// Types of authentication risk factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskFactorType {
    /// Unusual location access
    UnusualLocation,
    
    /// Unusual time access
    UnusualTime,
    
    /// Multiple failed attempts
    FailedAttempts,
    
    /// Compromised credentials detected
    CompromisedCredentials,
    
    /// Suspicious device characteristics
    SuspiciousDevice,
    
    /// Network-based risks
    NetworkRisk,
}

/// Authentication event for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationEvent {
    /// Event unique identifier
    pub event_id: String,
    
    /// Event timestamp
    pub timestamp: SystemTime,
    
    /// Authentication outcome
    pub outcome: AuthenticationOutcome,
    
    /// Method used for authentication
    pub method: AuthenticationMethod,
    
    /// Risk assessment at time of authentication
    pub risk_assessment: f64,
    
    /// Additional event metadata
    pub metadata: HashMap<String, String>,
}

/// Authentication outcome types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationOutcome {
    /// Authentication successful
    Success,
    
    /// Authentication failed
    Failed { reason: String },
    
    /// Authentication denied due to policy
    Denied { policy: String },
    
    /// Authentication blocked due to security concerns
    Blocked { reason: String },
    
    /// Authentication requires additional verification
    RequiresAdditionalAuth { required_factors: Vec<FactorType> },
}

/// Trust levels for devices and entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Untrusted entity
    Untrusted,
    
    /// Low trust level
    Low,
    
    /// Medium trust level
    Medium,
    
    /// High trust level
    High,
    
    /// Fully trusted entity
    Trusted,
}

/// Authentication data container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationData {
    /// Certificate authentication data
    Certificate {
        certificate_chain: Vec<Vec<u8>>,
        signature: Vec<u8>,
        challenge: Vec<u8>,
    },
    
    /// API key authentication data
    APIKey {
        key_identifier: String,
        timestamp: SystemTime,
        signature: Vec<u8>,
    },
    
    /// Challenge-response data
    ChallengeResponse {
        challenge: Vec<u8>,
        response: Vec<u8>,
        proof: Vec<u8>,
    },
    
    /// Biometric authentication data
    Biometric {
        template: Vec<u8>,
        confidence: f64,
        quality_score: f64,
    },
    
    /// Session continuation data
    SessionContinuation {
        session_token: String,
        refresh_token: String,
        last_activity: SystemTime,
    },
}

// =============================================================================
// AUTHORIZATION PROTOCOLS
// =============================================================================

/// Authorization message for permission verification and access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationMessage {
    /// Unique authorization request identifier
    pub authz_id: String,
    
    /// Type of authorization being requested
    pub authz_type: AuthorizationType,
    
    /// Subject requesting authorization
    pub subject: AuthorizationSubject,
    
    /// Resource being accessed
    pub resource: AuthorizationResource,
    
    /// Action being performed on the resource
    pub action: AuthorizationAction,
    
    /// Context for authorization decision
    pub context: AuthorizationContext,
    
    /// Timestamp of authorization request
    pub timestamp: SystemTime,
    
    /// Authorization decision if this is a response
    pub decision: Option<AuthorizationDecision>,
}

/// Types of authorization in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationType {
    /// API operation authorization
    APIOperation,
    
    /// Resource access authorization
    ResourceAccess,
    
    /// Administrative operation authorization
    Administrative,
    
    /// Emergency operation authorization
    Emergency,
    
    /// Cross-component operation authorization
    CrossComponent,
}

/// Subject requesting authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationSubject {
    /// Subject type (user, AI app, service)
    pub subject_type: SubjectType,
    
    /// Subject identifier
    pub subject_id: String,
    
    /// Subject's roles
    pub roles: Vec<String>,
    
    /// Subject's permissions
    pub permissions: Vec<Permission>,
    
    /// Subject's authentication context
    pub auth_context: String,
}

/// Types of authorization subjects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubjectType {
    /// Human user
    User,
    
    /// AI App component
    AIApp,
    
    /// System service
    Service,
    
    /// Device
    Device,
    
    /// External system
    External,
}

/// Resource being accessed in authorization request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationResource {
    /// Resource type
    pub resource_type: ResourceType,
    
    /// Resource identifier
    pub resource_id: String,
    
    /// Resource attributes for policy evaluation
    pub attributes: HashMap<String, String>,
    
    /// Resource owner
    pub owner: Option<String>,
    
    /// Resource classification level
    pub classification: ClassificationLevel,
}

/// Types of resources in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    /// File system resource
    File,
    
    /// AI App functionality
    AIAppFunction,
    
    /// Methodology execution
    Methodology,
    
    /// Ecosystem configuration
    Configuration,
    
    /// Consciousness data
    Consciousness,
    
    /// Intelligence data
    Intelligence,
    
    /// Administrative function
    Administrative,
}

/// Classification levels for resource protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassificationLevel {
    /// Public information
    Public,
    
    /// Internal use only
    Internal,
    
    /// Restricted access
    Restricted,
    
    /// Confidential information
    Confidential,
    
    /// Top secret information
    TopSecret,
}

/// Action being performed on the resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationAction {
    /// Action type
    pub action_type: ActionType,
    
    /// Specific operation being performed
    pub operation: String,
    
    /// Action parameters
    pub parameters: HashMap<String, String>,
    
    /// Action impact level
    pub impact_level: ImpactLevel,
}

/// Types of actions that can be authorized
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    /// Read operation
    Read,
    
    /// Write operation
    Write,
    
    /// Execute operation
    Execute,
    
    /// Delete operation
    Delete,
    
    /// Create operation
    Create,
    
    /// Modify operation
    Modify,
    
    /// Administrative operation
    Administer,
}

/// Impact level of the action being performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    /// Low impact operation
    Low,
    
    /// Medium impact operation
    Medium,
    
    /// High impact operation
    High,
    
    /// Critical impact operation
    Critical,
}

/// Context for authorization decision making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationContext {
    /// Current time of request
    pub request_time: SystemTime,
    
    /// Source location of request
    pub source_location: Option<LocationInfo>,
    
    /// Risk assessment of the request
    pub risk_assessment: RiskAssessment,
    
    /// Environmental factors
    pub environment: EnvironmentContext,
    
    /// Policy context
    pub policy_context: PolicyContext,
}

/// Risk assessment for authorization request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk score (0.0 to 1.0)
    pub risk_score: f64,
    
    /// Individual risk factors
    pub risk_factors: Vec<RiskFactor>,
    
    /// Risk mitigation strategies
    pub mitigations: Vec<String>,
    
    /// Risk tolerance threshold
    pub tolerance_threshold: f64,
}

/// Environmental context for authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentContext {
    /// System load and performance
    pub system_load: f64,
    
    /// Network conditions
    pub network_conditions: NetworkConditions,
    
    /// Security posture
    pub security_posture: SecurityPosture,
    
    /// Operational mode
    pub operational_mode: OperationalMode,
}

/// Network conditions affecting authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConditions {
    /// Network latency in milliseconds
    pub latency: f64,
    
    /// Network bandwidth utilization
    pub bandwidth_utilization: f64,
    
    /// Packet loss rate
    pub packet_loss: f64,
    
    /// Network security status
    pub security_status: NetworkSecurityStatus,
}

/// Network security status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkSecurityStatus {
    /// Network is secure
    Secure,
    
    /// Network has minor security concerns
    MinorConcerns,
    
    /// Network has significant security concerns
    SignificantConcerns,
    
    /// Network is under attack
    UnderAttack,
    
    /// Network security status unknown
    Unknown,
}

/// System security posture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityPosture {
    /// Normal security posture
    Normal,
    
    /// Elevated security posture
    Elevated,
    
    /// High security posture
    High,
    
    /// Maximum security posture
    Maximum,
    
    /// Emergency security posture
    Emergency,
}

/// System operational mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationalMode {
    /// Normal operations
    Normal,
    
    /// Maintenance mode
    Maintenance,
    
    /// Degraded operations
    Degraded,
    
    /// Emergency operations
    Emergency,
    
    /// Shutdown mode
    Shutdown,
}

/// Policy context for authorization decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    /// Applicable policies
    pub policies: Vec<PolicyReference>,
    
    /// Policy evaluation mode
    pub evaluation_mode: PolicyEvaluationMode,
    
    /// Override policies if any
    pub overrides: Vec<PolicyOverride>,
    
    /// Policy version information
    pub policy_version: String,
}

/// Reference to a security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyReference {
    /// Policy identifier
    pub policy_id: String,
    
    /// Policy name
    pub policy_name: String,
    
    /// Policy version
    pub version: String,
    
    /// Policy priority
    pub priority: u32,
}

/// Policy evaluation modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEvaluationMode {
    /// Strict policy enforcement
    Strict,
    
    /// Permissive evaluation
    Permissive,
    
    /// Advisory mode (log only)
    Advisory,
    
    /// Emergency bypass mode
    EmergencyBypass,
}

/// Policy override for special circumstances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyOverride {
    /// Override identifier
    pub override_id: String,
    
    /// Override reason
    pub reason: String,
    
    /// Override authority
    pub authority: String,
    
    /// Override expiration
    pub expires_at: SystemTime,
    
    /// Override scope
    pub scope: OverrideScope,
}

/// Scope of policy override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverrideScope {
    /// Single request override
    SingleRequest,
    
    /// Session-based override
    Session,
    
    /// Time-based override
    TimeBased,
    
    /// Subject-based override
    SubjectBased,
    
    /// Global override
    Global,
}

/// Authorization decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationDecision {
    /// Decision result
    pub decision: DecisionResult,
    
    /// Reasoning for the decision
    pub reasoning: Vec<String>,
    
    /// Applicable policies
    pub applied_policies: Vec<PolicyReference>,
    
    /// Decision confidence level
    pub confidence: f64,
    
    /// Additional obligations or conditions
    pub obligations: Vec<Obligation>,
}

/// Authorization decision results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionResult {
    /// Access granted
    Permit,
    
    /// Access denied
    Deny { reason: String },
    
    /// Decision indeterminate
    Indeterminate { reason: String },
    
    /// Not applicable
    NotApplicable,
    
    /// Conditional permit with obligations
    ConditionalPermit { conditions: Vec<String> },
}

/// Obligations that must be fulfilled for authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    /// Obligation type
    pub obligation_type: ObligationType,
    
    /// Obligation description
    pub description: String,
    
    /// Fulfillment deadline
    pub deadline: Option<SystemTime>,
    
    /// Fulfillment status
    pub status: ObligationStatus,
}

/// Types of authorization obligations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObligationType {
    /// Audit logging required
    AuditLogging,
    
    /// Notification required
    Notification,
    
    /// Additional approval required
    AdditionalApproval,
    
    /// Data encryption required
    DataEncryption,
    
    /// Access time limits
    TimeLimit,
    
    /// Usage monitoring
    UsageMonitoring,
}

/// Status of obligation fulfillment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObligationStatus {
    /// Obligation not yet fulfilled
    Pending,
    
    /// Obligation fulfilled
    Fulfilled,
    
    /// Obligation failed to be fulfilled
    Failed,
    
    /// Obligation not applicable
    NotApplicable,
}

// =============================================================================
// ENCRYPTION PROTOCOLS
// =============================================================================

/// Encryption protocol configuration and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionProtocol {
    /// Protocol identifier
    pub protocol_id: String,
    
    /// Protocol name and version
    pub protocol_name: String,
    
    /// Protocol version
    pub version: String,
    
    /// Supported encryption algorithms
    pub supported_algorithms: Vec<EncryptionAlgorithm>,
    
    /// Key exchange methods
    pub key_exchange_methods: Vec<KeyExchangeMethod>,
    
    /// Protocol parameters
    pub parameters: EncryptionProtocolParameters,
    
    /// Security properties
    pub security_properties: SecurityProperties,
}

/// Key exchange methods supported by the protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyExchangeMethod {
    /// Elliptic Curve Diffie-Hellman
    ECDH,
    
    /// X25519 key agreement
    X25519,
    
    /// RSA key transport
    RSA,
    
    /// Pre-shared key
    PreSharedKey,
    
    /// Static key pair
    StaticKeyPair,
}

/// Encryption protocol parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionProtocolParameters {
    /// Key sizes for different algorithms
    pub key_sizes: HashMap<String, u32>,
    
    /// IV/Nonce sizes
    pub nonce_sizes: HashMap<String, u32>,
    
    /// Authentication tag sizes
    pub tag_sizes: HashMap<String, u32>,
    
    /// Key derivation parameters
    pub key_derivation_params: KeyDerivationParams,
    
    /// Forward secrecy configuration
    pub forward_secrecy: ForwardSecrecyConfig,
}

/// Key derivation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationParams {
    /// Key derivation function
    pub kdf: KeyDerivationFunction,
    
    /// Salt length
    pub salt_length: u32,
    
    /// Iteration count
    pub iterations: u32,
    
    /// Memory cost (for memory-hard functions)
    pub memory_cost: Option<u32>,
    
    /// Parallelism factor
    pub parallelism: Option<u32>,
}

/// Forward secrecy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardSecrecyConfig {
    /// Forward secrecy enabled
    pub enabled: bool,
    
    /// Key rotation interval
    pub key_rotation_interval: Duration,
    
    /// Ephemeral key generation method
    pub ephemeral_key_method: EphemeralKeyMethod,
    
    /// Perfect forward secrecy
    pub perfect_forward_secrecy: bool,
}

/// Methods for generating ephemeral keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EphemeralKeyMethod {
    /// Generate new ephemeral key for each session
    PerSession,
    
    /// Generate new ephemeral key for each message
    PerMessage,
    
    /// Time-based key rotation
    TimeBased,
    
    /// Message count-based rotation
    MessageCountBased,
}

/// Security properties of the encryption protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProperties {
    /// Confidentiality provided
    pub confidentiality: ConfidentialityLevel,
    
    /// Integrity protection
    pub integrity: IntegrityLevel,
    
    /// Authentication provided
    pub authentication: AuthenticationLevel,
    
    /// Non-repudiation support
    pub non_repudiation: bool,
    
    /// Forward secrecy support
    pub forward_secrecy: bool,
    
    /// Post-quantum security
    pub post_quantum_secure: bool,
}

/// Levels of confidentiality protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfidentialityLevel {
    /// No confidentiality protection
    None,
    
    /// Basic confidentiality
    Basic,
    
    /// Strong confidentiality
    Strong,
    
    /// Military-grade confidentiality
    MilitaryGrade,
    
    /// Top secret confidentiality
    TopSecret,
}

/// Levels of integrity protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrityLevel {
    /// No integrity protection
    None,
    
    /// Basic checksum
    BasicChecksum,
    
    /// Cryptographic hash
    CryptographicHash,
    
    /// HMAC protection
    HMAC,
    
    /// Digital signature
    DigitalSignature,
}

/// Levels of authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationLevel {
    /// No authentication
    None,
    
    /// Basic authentication
    Basic,
    
    /// Strong authentication
    Strong,
    
    /// Multi-factor authentication
    MultiFactor,
    
    /// Continuous authentication
    Continuous,
}

// =============================================================================
// KEY EXCHANGE PROTOCOLS
// =============================================================================

/// Key exchange message for establishing secure communication channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyExchangeMessage {
    /// Exchange identifier
    pub exchange_id: String,
    
    /// Key exchange protocol being used
    pub protocol: KeyExchangeProtocol,
    
    /// Exchange phase (initiation, response, confirmation)
    pub phase: KeyExchangePhase,
    
    /// Public key data for exchange
    pub public_key_data: PublicKeyData,
    
    /// Proof of possession or authentication data
    pub proof_of_possession: Option<ProofOfPossession>,
    
    /// Supported algorithms and parameters
    pub supported_params: SupportedParameters,
    
    /// Exchange timestamp
    pub timestamp: SystemTime,
    
    /// Exchange expiration
    pub expires_at: SystemTime,
}

/// Key exchange protocols supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyExchangeProtocol {
    /// X25519 Elliptic Curve Diffie-Hellman
    X25519ECDH,
    
    /// P-256 Elliptic Curve Diffie-Hellman
    P256ECDH,
    
    /// RSA key transport
    RSAKeyTransport,
    
    /// Pre-shared key agreement
    PreSharedKey,
    
    /// Signal Protocol X3DH
    SignalX3DH,
}

/// Phases of key exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyExchangePhase {
    /// Initial key exchange request
    Initiation,
    
    /// Response to key exchange
    Response,
    
    /// Final confirmation
    Confirmation,
    
    /// Key derivation complete
    Complete,
    
    /// Key exchange failed
    Failed { reason: String },
}

/// Public key data for exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyData {
    /// Key algorithm
    pub algorithm: String,
    
    /// Public key bytes
    pub key_bytes: Vec<u8>,
    
    /// Key usage constraints
    pub usage: Vec<KeyUsage>,
    
    /// Key validity period
    pub validity: KeyValidity,
    
    /// Key parameters
    pub parameters: KeyParameters,
}

/// Key usage constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyUsage {
    /// Key agreement/exchange
    KeyAgreement,
    
    /// Digital signature
    DigitalSignature,
    
    /// Data encryption
    DataEncryption,
    
    /// Key encipherment
    KeyEncipherment,
    
    /// Certificate signing
    CertSigning,
}

/// Key validity period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValidity {
    /// Valid from timestamp
    pub valid_from: SystemTime,
    
    /// Valid until timestamp
    pub valid_until: SystemTime,
    
    /// Key revocation status
    pub revocation_status: RevocationStatus,
}

/// Key revocation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevocationStatus {
    /// Key is valid
    Valid,
    
    /// Key is revoked
    Revoked { reason: String, timestamp: SystemTime },
    
    /// Key revocation status unknown
    Unknown,
    
    /// Key is suspended
    Suspended { until: SystemTime },
}

/// Key algorithm parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyParameters {
    /// Curve name for ECC keys
    pub curve: Option<String>,
    
    /// Key size in bits
    pub key_size: u32,
    
    /// Hash algorithm used
    pub hash_algorithm: Option<String>,
    
    /// Additional algorithm-specific parameters
    pub additional_params: HashMap<String, String>,
}

/// Proof of possession for key exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfPossession {
    /// Proof method used
    pub method: ProofMethod,
    
    /// Proof data
    pub proof_data: Vec<u8>,
    
    /// Challenge used for proof
    pub challenge: Vec<u8>,
    
    /// Timestamp of proof generation
    pub timestamp: SystemTime,
}

/// Methods for proving key possession
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofMethod {
    /// Digital signature proof
    DigitalSignature,
    
    /// HMAC proof
    HMAC,
    
    /// Challenge-response proof
    ChallengeResponse,
    
    /// Zero-knowledge proof
    ZeroKnowledge,
}

/// Supported parameters for key exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedParameters {
    /// Supported key exchange methods
    pub key_exchange_methods: Vec<KeyExchangeMethod>,
    
    /// Supported encryption algorithms
    pub encryption_algorithms: Vec<EncryptionAlgorithm>,
    
    /// Supported signature algorithms
    pub signature_algorithms: Vec<SignatureAlgorithm>,
    
    /// Supported hash algorithms
    pub hash_algorithms: Vec<HashAlgorithm>,
    
    /// Key size preferences
    pub key_size_preferences: HashMap<String, Vec<u32>>,
}

/// Supported hash algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithm {
    /// SHA-256 hash function
    SHA256,
    
    /// SHA-384 hash function
    SHA384,
    
    /// SHA-512 hash function
    SHA512,
    
    /// BLAKE3 hash function
    BLAKE3,
    
    /// SHA-3 (Keccak) hash function
    SHA3_256,
}

// =============================================================================
// CERTIFICATE EXCHANGE PROTOCOLS
// =============================================================================

/// Certificate exchange message for trust establishment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateExchangeMessage {
    /// Exchange identifier
    pub exchange_id: String,
    
    /// Type of certificate exchange
    pub exchange_type: CertificateExchangeType,
    
    /// Certificate chain being exchanged
    pub certificate_chain: Vec<Certificate>,
    
    /// Certificate validation request/response
    pub validation_request: Option<CertificateValidationRequest>,
    
    /// Certificate validation result
    pub validation_result: Option<CertificateValidationResult>,
    
    /// Trust anchor information
    pub trust_anchors: Vec<TrustAnchor>,
    
    /// Exchange timestamp
    pub timestamp: SystemTime,
}

/// Types of certificate exchange operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertificateExchangeType {
    /// Certificate presentation for authentication
    CertificatePresentation,
    
    /// Certificate validation request
    ValidationRequest,
    
    /// Certificate validation response
    ValidationResponse,
    
    /// Trust anchor distribution
    TrustAnchorDistribution,
    
    /// Certificate revocation notification
    RevocationNotification,
    
    /// Certificate renewal notification
    RenewalNotification,
}

/// Certificate representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    /// Certificate format
    pub format: CertificateFormat,
    
    /// Certificate data
    pub certificate_data: Vec<u8>,
    
    /// Certificate type
    pub certificate_type: CertificateType,
    
    /// Subject information
    pub subject: CertificateSubject,
    
    /// Issuer information
    pub issuer: CertificateIssuer,
    
    /// Certificate validity period
    pub validity: CertificateValidity,
    
    /// Certificate extensions
    pub extensions: Vec<CertificateExtension>,
}

/// Certificate formats supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertificateFormat {
    /// X.509 DER encoded
    X509DER,
    
    /// X.509 PEM encoded
    X509PEM,
    
    /// OpenPGP certificate
    OpenPGP,
    
    /// Custom ecosystem format
    EcosystemFormat,
}

/// Types of certificates in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertificateType {
    /// AI App certificate
    AIApp,
    
    /// User certificate
    User,
    
    /// Device certificate
    Device,
    
    /// Service certificate
    Service,
    
    /// Certificate Authority certificate
    CA,
    
    /// Root certificate
    Root,
}

/// Certificate subject information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateSubject {
    /// Common name
    pub common_name: String,
    
    /// Organization
    pub organization: Option<String>,
    
    /// Organizational unit
    pub organizational_unit: Option<String>,
    
    /// Country
    pub country: Option<String>,
    
    /// State or province
    pub state: Option<String>,
    
    /// Locality
    pub locality: Option<String>,
    
    /// Email address
    pub email: Option<String>,
    
    /// Subject alternative names
    pub subject_alt_names: Vec<SubjectAlternativeName>,
}

/// Subject alternative name types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubjectAlternativeName {
    /// DNS name
    DNSName(String),
    
    /// IP address
    IPAddress(String),
    
    /// Email address
    Email(String),
    
    /// URI
    URI(String),
    
    /// Component identifier
    ComponentID(String),
}

/// Certificate issuer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateIssuer {
    /// Issuer common name
    pub common_name: String,
    
    /// Issuer organization
    pub organization: Option<String>,
    
    /// Issuer country
    pub country: Option<String>,
    
    /// Issuer key identifier
    pub key_identifier: Vec<u8>,
    
    /// Issuer certificate fingerprint
    pub fingerprint: Vec<u8>,
}

/// Certificate validity period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidity {
    /// Valid from date
    pub not_before: SystemTime,
    
    /// Valid until date
    pub not_after: SystemTime,
    
    /// Certificate revocation status
    pub revocation_status: CertificateRevocationStatus,
    
    /// OCSP responder URL
    pub ocsp_responder: Option<String>,
}

/// Certificate revocation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertificateRevocationStatus {
    /// Certificate is valid
    Valid,
    
    /// Certificate is revoked
    Revoked {
        reason: RevocationReason,
        revocation_time: SystemTime,
    },
    
    /// Certificate is suspended
    Suspended {
        until: SystemTime,
        reason: String,
    },
    
    /// Revocation status unknown
    Unknown,
}

/// Reasons for certificate revocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevocationReason {
    /// Key compromise
    KeyCompromise,
    
    /// CA compromise
    CACompromise,
    
    /// Affiliation changed
    AffiliationChanged,
    
    /// Certificate superseded
    Superseded,
    
    /// Cessation of operation
    CessationOfOperation,
    
    /// Certificate hold
    CertificateHold,
    
    /// Privilege withdrawn
    PrivilegeWithdrawn,
    
    /// AA compromise
    AACompromise,
}

/// Certificate extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateExtension {
    /// Extension OID
    pub oid: String,
    
    /// Extension criticality
    pub critical: bool,
    
    /// Extension value
    pub value: Vec<u8>,
    
    /// Extension description
    pub description: String,
}

/// Certificate validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidationRequest {
    /// Validation request ID
    pub request_id: String,
    
    /// Certificate to validate
    pub certificate: Certificate,
    
    /// Validation policy
    pub validation_policy: ValidationPolicy,
    
    /// Validation time
    pub validation_time: SystemTime,
    
    /// Additional validation parameters
    pub parameters: ValidationParameters,
}

/// Certificate validation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationPolicy {
    /// Trust anchor certificates
    pub trust_anchors: Vec<TrustAnchor>,
    
    /// Validation flags
    pub validation_flags: Vec<ValidationFlag>,
    
    /// Policy constraints
    pub policy_constraints: Vec<PolicyConstraint>,
    
    /// Name constraints
    pub name_constraints: Option<NameConstraints>,
}

/// Trust anchor for certificate validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAnchor {
    /// Trust anchor certificate
    pub certificate: Certificate,
    
    /// Trust level
    pub trust_level: TrustLevel,
    
    /// Trust constraints
    pub constraints: Vec<TrustConstraint>,
    
    /// Trust anchor name
    pub name: String,
}

/// Trust constraints for trust anchors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustConstraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    
    /// Constraint value
    pub value: String,
    
    /// Constraint criticality
    pub critical: bool,
}

/// Types of trust constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Name constraint
    NameConstraint,
    
    /// Policy constraint
    PolicyConstraint,
    
    /// Usage constraint
    UsageConstraint,
    
    /// Time constraint
    TimeConstraint,
    
    /// Custom constraint
    CustomConstraint(String),
}

/// Certificate validation flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationFlag {
    /// Check certificate validity period
    CheckValidity,
    
    /// Check certificate revocation status
    CheckRevocation,
    
    /// Check certificate chain
    CheckChain,
    
    /// Check certificate usage
    CheckUsage,
    
    /// Check certificate policies
    CheckPolicies,
    
    /// Check name constraints
    CheckNameConstraints,
}

/// Policy constraints for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConstraint {
    /// Policy OID
    pub policy_oid: String,
    
    /// Constraint type
    pub constraint_type: String,
    
    /// Constraint value
    pub value: String,
    
    /// Constraint criticality
    pub critical: bool,
}

/// Name constraints for certificate validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameConstraints {
    /// Permitted subtrees
    pub permitted_subtrees: Vec<GeneralSubtree>,
    
    /// Excluded subtrees
    pub excluded_subtrees: Vec<GeneralSubtree>,
}

/// General subtree for name constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSubtree {
    /// Base name
    pub base: SubjectAlternativeName,
    
    /// Minimum distance
    pub minimum: Option<u32>,
    
    /// Maximum distance
    pub maximum: Option<u32>,
}

/// Validation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationParameters {
    /// Maximum chain length
    pub max_chain_length: Option<u32>,
    
    /// OCSP checking enabled
    pub ocsp_checking: bool,
    
    /// CRL checking enabled
    pub crl_checking: bool,
    
    /// Policy checking enabled
    pub policy_checking: bool,
    
    /// Name checking enabled
    pub name_checking: bool,
}

/// Certificate validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidationResult {
    /// Validation request ID
    pub request_id: String,
    
    /// Validation result
    pub result: ValidationResult,
    
    /// Validation path
    pub validation_path: Vec<Certificate>,
    
    /// Validation errors
    pub errors: Vec<ValidationError>,
    
    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
    
    /// Validation timestamp
    pub timestamp: SystemTime,
}

/// Certificate validation result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationResult {
    /// Certificate is valid
    Valid,
    
    /// Certificate is invalid
    Invalid { reasons: Vec<String> },
    
    /// Validation indeterminate
    Indeterminate { reasons: Vec<String> },
    
    /// Certificate revoked
    Revoked { reason: RevocationReason },
}

/// Validation error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub error_code: String,
    
    /// Error message
    pub message: String,
    
    /// Error severity
    pub severity: ErrorSeverity,
    
    /// Certificate causing the error
    pub certificate: Option<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Low severity
    Low,
    
    /// Medium severity
    Medium,
    
    /// High severity
    High,
    
    /// Critical severity
    Critical,
}

/// Validation warning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub warning_code: String,
    
    /// Warning message
    pub message: String,
    
    /// Certificate causing the warning
    pub certificate: Option<String>,
}

// =============================================================================
// SECURITY HANDSHAKE PROTOCOLS
// =============================================================================

/// Security handshake for establishing secure communication sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHandshake {
    /// Handshake identifier
    pub handshake_id: String,
    
    /// Handshake type
    pub handshake_type: HandshakeType,
    
    /// Current handshake state
    pub state: HandshakeState,
    
    /// Handshake messages exchanged
    pub messages: Vec<HandshakeMessage>,
    
    /// Negotiated security parameters
    pub negotiated_params: Option<NegotiatedParameters>,
    
    /// Handshake timestamp
    pub timestamp: SystemTime,
    
    /// Handshake timeout
    pub timeout: Duration,
}

/// Types of security handshakes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandshakeType {
    /// AI App to OZONE STUDIO handshake
    AIAppHandshake,
    
    /// User to BRIDGE handshake
    UserHandshake,
    
    /// Cross-device handshake
    CrossDeviceHandshake,
    
    /// Service-to-service handshake
    ServiceHandshake,
    
    /// Emergency handshake
    EmergencyHandshake,
}

/// States of security handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandshakeState {
    /// Handshake initiated
    Initiated,
    
    /// Client hello sent/received
    ClientHello,
    
    /// Server hello sent/received
    ServerHello,
    
    /// Certificate exchange
    CertificateExchange,
    
    /// Key exchange
    KeyExchange,
    
    /// Authentication
    Authentication,
    
    /// Handshake complete
    Complete,
    
    /// Handshake failed
    Failed { reason: String },
    
    /// Handshake aborted
    Aborted,
}

/// Individual handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    /// Message type
    pub message_type: HandshakeMessageType,
    
    /// Message data
    pub data: Vec<u8>,
    
    /// Message sender
    pub sender: ComponentIdentity,
    
    /// Message recipient
    pub recipient: ComponentIdentity,
    
    /// Message timestamp
    pub timestamp: SystemTime,
    
    /// Message hash for integrity
    pub hash: Vec<u8>,
}

/// Types of handshake messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandshakeMessageType {
    /// Client hello message
    ClientHello,
    
    /// Server hello message
    ServerHello,
    
    /// Certificate message
    Certificate,
    
    /// Certificate request
    CertificateRequest,
    
    /// Certificate verify
    CertificateVerify,
    
    /// Client key exchange
    ClientKeyExchange,
    
    /// Server key exchange
    ServerKeyExchange,
    
    /// Finished message
    Finished,
    
    /// Hello request
    HelloRequest,
    
    /// Alert message
    Alert,
}

/// Negotiated security parameters from handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiatedParameters {
    /// Protocol version
    pub protocol_version: String,
    
    /// Cipher suite
    pub cipher_suite: CipherSuite,
    
    /// Compression method
    pub compression: CompressionMethod,
    
    /// Session keys
    pub session_keys: SessionKeys,
    
    /// Security properties
    pub security_properties: NegotiatedSecurityProperties,
}

/// Cipher suite negotiated for the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CipherSuite {
    /// Cipher suite identifier
    pub suite_id: String,
    
    /// Encryption algorithm
    pub encryption: EncryptionAlgorithm,
    
    /// Authentication algorithm
    pub authentication: AuthenticationAlgorithm,
    
    /// Key exchange algorithm
    pub key_exchange: KeyExchangeAlgorithm,
    
    /// Hash algorithm
    pub hash: HashAlgorithm,
}

/// Authentication algorithms for cipher suites
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationAlgorithm {
    /// HMAC with SHA-256
    HMACSHA256,
    
    /// HMAC with SHA-384
    HMACSHA384,
    
    /// Poly1305 authenticator
    Poly1305,
    
    /// GCM authentication
    GCM,
}

/// Key exchange algorithms for cipher suites
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyExchangeAlgorithm {
    /// RSA key exchange
    RSA,
    
    /// Elliptic Curve Diffie-Hellman
    ECDH,
    
    /// Ephemeral Elliptic Curve Diffie-Hellman
    ECDHE,
    
    /// Pre-shared key
    PSK,
    
    /// X25519 key agreement
    X25519,
}

/// Compression methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionMethod {
    /// No compression
    None,
    
    /// DEFLATE compression
    Deflate,
    
    /// LZ4 compression
    LZ4,
    
    /// Brotli compression
    Brotli,
}

/// Session keys established during handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKeys {
    /// Client write key
    pub client_write_key: Vec<u8>,
    
    /// Server write key
    pub server_write_key: Vec<u8>,
    
    /// Client MAC key
    pub client_mac_key: Vec<u8>,
    
    /// Server MAC key
    pub server_mac_key: Vec<u8>,
    
    /// Client IV
    pub client_iv: Vec<u8>,
    
    /// Server IV
    pub server_iv: Vec<u8>,
}

/// Negotiated security properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiatedSecurityProperties {
    /// Encryption strength
    pub encryption_strength: u32,
    
    /// Authentication strength
    pub authentication_strength: u32,
    
    /// Forward secrecy provided
    pub forward_secrecy: bool,
    
    /// Perfect forward secrecy
    pub perfect_forward_secrecy: bool,
    
    /// Post-quantum security
    pub post_quantum_secure: bool,
}

// =============================================================================
// SECURITY CONTEXT MANAGEMENT
// =============================================================================

/// Security context for maintaining session security state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Context identifier
    pub context_id: String,
    
    /// Context type
    pub context_type: SecurityContextType,
    
    /// Associated session information
    pub session_info: SessionInfo,
    
    /// Security parameters
    pub security_params: SecurityParameters,
    
    /// Authentication state
    pub auth_state: AuthenticationState,
    
    /// Authorization cache
    pub authz_cache: AuthorizationCache,
    
    /// Context creation time
    pub created_at: SystemTime,
    
    /// Context expiration time
    pub expires_at: SystemTime,
    
    /// Last activity time
    pub last_activity: SystemTime,
}

/// Types of security contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityContextType {
    /// AI App communication context
    AIAppContext,
    
    /// User session context
    UserSession,
    
    /// Device coordination context
    DeviceContext,
    
    /// Service communication context
    ServiceContext,
    
    /// Administrative context
    AdministrativeContext,
}

/// Session information for security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session identifier
    pub session_id: String,
    
    /// Session type
    pub session_type: SessionType,
    
    /// Participant identities
    pub participants: Vec<ParticipantInfo>,
    
    /// Session state
    pub state: SessionState,
    
    /// Session attributes
    pub attributes: HashMap<String, String>,
}

/// Types of security sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    /// Interactive user session
    Interactive,
    
    /// Batch processing session
    Batch,
    
    /// API session
    API,
    
    /// Administrative session
    Administrative,
    
    /// Emergency session
    Emergency,
}

/// Participant information in security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    /// Participant identifier
    pub participant_id: String,
    
    /// Participant type
    pub participant_type: ParticipantType,
    
    /// Participant roles
    pub roles: Vec<String>,
    
    /// Participant permissions
    pub permissions: Vec<Permission>,
    
    /// Authentication level achieved
    pub auth_level: AuthenticationLevel,
}

/// Types of session participants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantType {
    /// Human user
    HumanUser,
    
    /// AI App component
    AIAppComponent,
    
    /// System service
    SystemService,
    
    /// External entity
    ExternalEntity,
}

/// Security session states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionState {
    /// Session active
    Active,
    
    /// Session suspended
    Suspended,
    
    /// Session expired
    Expired,
    
    /// Session terminated
    Terminated,
    
    /// Session invalidated
    Invalidated { reason: String },
}

/// Security parameters for the context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityParameters {
    /// Encryption configuration
    pub encryption_config: EncryptionConfiguration,
    
    /// Authentication requirements
    pub auth_requirements: AuthenticationRequirements,
    
    /// Authorization policy
    pub authz_policy: AuthorizationPolicy,
    
    /// Security controls
    pub security_controls: Vec<SecurityControl>,
}

/// Encryption configuration for security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfiguration {
    /// Algorithm in use
    pub algorithm: EncryptionAlgorithm,
    
    /// Key material
    pub key_material: KeyMaterial,
    
    /// Encryption parameters
    pub parameters: EncryptionParams,
    
    /// Key rotation policy
    pub key_rotation: KeyRotationPolicy,
}

/// Key material for encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMaterial {
    /// Encryption key identifier
    pub key_id: String,
    
    /// Key derivation info
    pub derivation_info: KeyDerivationInfo,
    
    /// Key usage constraints
    pub usage_constraints: Vec<KeyUsageConstraint>,
    
    /// Key lifecycle state
    pub lifecycle_state: KeyLifecycleState,
}

/// Key usage constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyUsageConstraint {
    /// Constraint type
    pub constraint_type: String,
    
    /// Constraint value
    pub value: String,
    
    /// Constraint enforcement
    pub enforcement: ConstraintEnforcement,
}

/// Constraint enforcement levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintEnforcement {
    /// Advisory constraint
    Advisory,
    
    /// Warning constraint
    Warning,
    
    /// Enforced constraint
    Enforced,
    
    /// Strict constraint
    Strict,
}

/// Key lifecycle states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyLifecycleState {
    /// Key is active
    Active,
    
    /// Key is pending activation
    PendingActivation,
    
    /// Key is deactivated
    Deactivated,
    
    /// Key is compromised
    Compromised,
    
    /// Key is destroyed
    Destroyed,
}

/// Encryption parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionParams {
    /// IV generation method
    pub iv_generation: IVGenerationMethod,
    
    /// Padding scheme
    pub padding: PaddingScheme,
    
    /// Additional authenticated data
    pub aad_policy: AADPolicy,
}

/// IV generation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IVGenerationMethod {
    /// Random IV generation
    Random,
    
    /// Counter-based IV
    Counter,
    
    /// Timestamp-based IV
    Timestamp,
    
    /// Deterministic IV
    Deterministic,
}

/// Padding schemes for encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaddingScheme {
    /// No padding
    None,
    
    /// PKCS#7 padding
    PKCS7,
    
    /// OAEP padding
    OAEP,
    
    /// PSS padding
    PSS,
}

/// Additional authenticated data policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AADPolicy {
    /// AAD required
    pub required: bool,
    
    /// AAD sources
    pub sources: Vec<AADSource>,
    
    /// AAD validation
    pub validation: AADValidation,
}

/// Sources of additional authenticated data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AADSource {
    /// Message headers
    MessageHeaders,
    
    /// Timestamp
    Timestamp,
    
    /// Sender identity
    SenderIdentity,
    
    /// Custom data
    CustomData(String),
}

/// AAD validation requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AADValidation {
    /// No validation
    None,
    
    /// Basic validation
    Basic,
    
    /// Strict validation
    Strict,
    
    /// Custom validation
    Custom(String),
}

/// Key rotation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    /// Rotation enabled
    pub enabled: bool,
    
    /// Rotation interval
    pub interval: Duration,
    
    /// Rotation trigger
    pub trigger: RotationTrigger,
    
    /// Grace period for old keys
    pub grace_period: Duration,
}

/// Key rotation triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationTrigger {
    /// Time-based rotation
    TimeBased,
    
    /// Usage-based rotation
    UsageBased { max_uses: u64 },
    
    /// Data-based rotation
    DataBased { max_bytes: u64 },
    
    /// Event-based rotation
    EventBased { events: Vec<String> },
}

/// Authentication requirements for security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationRequirements {
    /// Required authentication level
    pub required_level: AuthenticationLevel,
    
    /// Required factors
    pub required_factors: Vec<FactorType>,
    
    /// Re-authentication policy
    pub reauth_policy: ReauthenticationPolicy,
    
    /// Session management
    pub session_management: SessionManagementPolicy,
}

/// Re-authentication policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReauthenticationPolicy {
    /// Re-authentication required
    pub required: bool,
    
    /// Re-authentication interval
    pub interval: Duration,
    
    /// Re-authentication triggers
    pub triggers: Vec<ReauthTrigger>,
    
    /// Grace period
    pub grace_period: Duration,
}

/// Re-authentication triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReauthTrigger {
    /// Time-based re-authentication
    TimeBased,
    
    /// Risk-based re-authentication
    RiskBased { threshold: f64 },
    
    /// Operation-based re-authentication
    OperationBased { operations: Vec<String> },
    
    /// Location-based re-authentication
    LocationBased,
}

/// Session management policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManagementPolicy {
    /// Session timeout
    pub timeout: Duration,
    
    /// Idle timeout
    pub idle_timeout: Duration,
    
    /// Maximum sessions per user
    pub max_sessions: u32,
    
    /// Concurrent session policy
    pub concurrent_sessions: ConcurrentSessionPolicy,
}

/// Concurrent session policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConcurrentSessionPolicy {
    /// Allow concurrent sessions
    Allow,
    
    /// Deny concurrent sessions
    Deny,
    
    /// Limit concurrent sessions
    Limit { max_concurrent: u32 },
    
    /// Replace existing sessions
    Replace,
}

/// Authorization policy for security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationPolicy {
    /// Policy identifier
    pub policy_id: String,
    
    /// Policy rules
    pub rules: Vec<AuthorizationRule>,
    
    /// Default action
    pub default_action: DefaultAction,
    
    /// Policy enforcement mode
    pub enforcement_mode: PolicyEnforcementMode,
}

/// Authorization rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRule {
    /// Rule identifier
    pub rule_id: String,
    
    /// Rule conditions
    pub conditions: Vec<RuleCondition>,
    
    /// Rule actions
    pub actions: Vec<RuleAction>,
    
    /// Rule priority
    pub priority: u32,
    
    /// Rule enabled
    pub enabled: bool,
}

/// Rule condition for authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    /// Condition type
    pub condition_type: ConditionType,
    
    /// Condition operator
    pub operator: ConditionOperator,
    
    /// Condition value
    pub value: String,
    
    /// Condition negated
    pub negated: bool,
}

/// Types of rule conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    /// Subject-based condition
    Subject,
    
    /// Resource-based condition
    Resource,
    
    /// Action-based condition
    Action,
    
    /// Time-based condition
    Time,
    
    /// Location-based condition
    Location,
    
    /// Risk-based condition
    Risk,
    
    /// Custom condition
    Custom(String),
}

/// Condition operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    /// Equals
    Equals,
    
    /// Not equals
    NotEquals,
    
    /// Contains
    Contains,
    
    /// Starts with
    StartsWith,
    
    /// Ends with
    EndsWith,
    
    /// Greater than
    GreaterThan,
    
    /// Less than
    LessThan,
    
    /// In list
    In,
    
    /// Not in list
    NotIn,
    
    /// Matches regex
    Regex,
}

/// Rule action for authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// Allow access
    Allow,
    
    /// Deny access
    Deny,
    
    /// Require additional authentication
    RequireAuth,
    
    /// Log access
    Log,
    
    /// Send alert
    Alert,
    
    /// Custom action
    Custom(String),
}

/// Default authorization action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefaultAction {
    /// Allow by default
    Allow,
    
    /// Deny by default
    Deny,
    
    /// Audit by default
    Audit,
}

/// Policy enforcement modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEnforcementMode {
    /// Enforcing mode
    Enforcing,
    
    /// Permissive mode
    Permissive,
    
    /// Audit mode
    Audit,
    
    /// Disabled
    Disabled,
}

/// Security control for security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityControl {
    /// Control identifier
    pub control_id: String,
    
    /// Control type
    pub control_type: SecurityControlType,
    
    /// Control configuration
    pub configuration: ControlConfiguration,
    
    /// Control enabled
    pub enabled: bool,
    
    /// Control priority
    pub priority: u32,
}

/// Types of security controls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityControlType {
    /// Access control
    AccessControl,
    
    /// Audit logging
    AuditLogging,
    
    /// Intrusion detection
    IntrusionDetection,
    
    /// Rate limiting
    RateLimiting,
    
    /// Data loss prevention
    DataLossPrevention,
    
    /// Threat detection
    ThreatDetection,
    
    /// Custom control
    Custom(String),
}

/// Control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlConfiguration {
    /// Configuration parameters
    pub parameters: HashMap<String, String>,
    
    /// Control thresholds
    pub thresholds: HashMap<String, f64>,
    
    /// Control actions
    pub actions: Vec<ControlAction>,
    
    /// Control reporting
    pub reporting: ControlReporting,
}

/// Control action configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlAction {
    /// Action type
    pub action_type: String,
    
    /// Action trigger
    pub trigger: ActionTrigger,
    
    /// Action parameters
    pub parameters: HashMap<String, String>,
    
    /// Action enabled
    pub enabled: bool,
}

/// Action triggers for security controls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionTrigger {
    /// Threshold exceeded
    ThresholdExceeded { threshold: String },
    
    /// Event detected
    EventDetected { event_type: String },
    
    /// Time-based trigger
    TimeBased { interval: Duration },
    
    /// Manual trigger
    Manual,
}

/// Control reporting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlReporting {
    /// Reporting enabled
    pub enabled: bool,
    
    /// Reporting frequency
    pub frequency: Duration,
    
    /// Report recipients
    pub recipients: Vec<String>,
    
    /// Report format
    pub format: ReportFormat,
}

/// Report formats for security controls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportFormat {
    /// JSON format
    JSON,
    
    /// XML format
    XML,
    
    /// Plain text
    Text,
    
    /// HTML format
    HTML,
    
    /// Custom format
    Custom(String),
}

/// Authentication state for security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationState {
    /// Authentication status
    pub status: AuthenticationStatus,
    
    /// Authenticated identity
    pub identity: Option<AuthenticatedIdentity>,
    
    /// Authentication methods used
    pub methods_used: Vec<AuthenticationMethod>,
    
    /// Authentication timestamp
    pub auth_timestamp: SystemTime,
    
    /// Authentication expires at
    pub expires_at: SystemTime,
    
    /// Last verification
    pub last_verification: SystemTime,
}

/// Authentication status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationStatus {
    /// Not authenticated
    NotAuthenticated,
    
    /// Partially authenticated
    PartiallyAuthenticated,
    
    /// Fully authenticated
    FullyAuthenticated,
    
    /// Authentication expired
    Expired,
    
    /// Authentication suspended
    Suspended,
}

/// Authenticated identity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedIdentity {
    /// Principal identifier
    pub principal_id: String,
    
    /// Principal type
    pub principal_type: PrincipalType,
    
    /// Principal attributes
    pub attributes: HashMap<String, String>,
    
    /// Authentication assurance level
    pub assurance_level: AssuranceLevel,
    
    /// Identity verification status
    pub verification_status: VerificationStatus,
}

/// Types of authenticated principals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrincipalType {
    /// Human user principal
    HumanUser,
    
    /// AI App principal
    AIApp,
    
    /// Service principal
    Service,
    
    /// Device principal
    Device,
    
    /// Anonymous principal
    Anonymous,
}

/// Authentication assurance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssuranceLevel {
    /// Low assurance
    Low,
    
    /// Medium assurance
    Medium,
    
    /// High assurance
    High,
    
    /// Very high assurance
    VeryHigh,
}

/// Identity verification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Identity verified
    Verified,
    
    /// Identity not verified
    NotVerified,
    
    /// Identity partially verified
    PartiallyVerified,
    
    /// Identity verification expired
    VerificationExpired,
}

/// Authorization cache for security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCache {
    /// Cache entries
    pub entries: HashMap<String, CacheEntry>,
    
    /// Cache configuration
    pub configuration: CacheConfiguration,
    
    /// Cache statistics
    pub statistics: CacheStatistics,
    
    /// Last cleanup timestamp
    pub last_cleanup: SystemTime,
}

/// Authorization cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Entry key
    pub key: String,
    
    /// Authorization decision
    pub decision: AuthorizationDecision,
    
    /// Entry timestamp
    pub timestamp: SystemTime,
    
    /// Entry expires at
    pub expires_at: SystemTime,
    
    /// Entry access count
    pub access_count: u32,
    
    /// Last access time
    pub last_access: SystemTime,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfiguration {
    /// Cache enabled
    pub enabled: bool,
    
    /// Maximum cache size
    pub max_size: usize,
    
    /// Default TTL
    pub default_ttl: Duration,
    
    /// Cleanup interval
    pub cleanup_interval: Duration,
    
    /// Cache eviction policy
    pub eviction_policy: EvictionPolicy,
}

/// Cache eviction policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least recently used
    LRU,
    
    /// Least frequently used
    LFU,
    
    /// First in, first out
    FIFO,
    
    /// Time-based expiration
    TTL,
    
    /// Random eviction
    Random,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Cache hits
    pub hits: u64,
    
    /// Cache misses
    pub misses: u64,
    
    /// Cache evictions
    pub evictions: u64,
    
    /// Cache size
    pub size: usize,
    
    /// Hit ratio
    pub hit_ratio: f64,
}

// =============================================================================
// SECURITY PROTOCOL ERRORS
// =============================================================================

/// Security protocol errors
#[derive(Error, Debug)]
pub enum SecurityProtocolError {
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },
    
    #[error("Authorization denied: {operation} not permitted for {principal}")]
    AuthorizationDenied { operation: String, principal: String },
    
    #[error("Encryption error: {details}")]
    EncryptionError { details: String },
    
    #[error("Key exchange failed: {reason}")]
    KeyExchangeFailed { reason: String },
    
    #[error("Certificate validation failed: {details}")]
    CertificateValidationFailed { details: String },
    
    #[error("Handshake failed: {phase} - {reason}")]
    HandshakeFailed { phase: String, reason: String },
    
    #[error("Security context error: {context_id} - {details}")]
    SecurityContextError { context_id: String, details: String },
    
    #[error("Protocol violation: {protocol} - {violation}")]
    ProtocolViolation { protocol: String, violation: String },
    
    #[error("Security policy violation: {policy} - {violation}")]
    PolicyViolation { policy: String, violation: String },
    
    #[error("Cryptographic error: {operation} - {details}")]
    CryptographicError { operation: String, details: String },
    
    #[error("Invalid security configuration: {component} - {details}")]
    InvalidConfiguration { component: String, details: String },
    
    #[error("Security audit failure: {audit_type} - {details}")]
    AuditFailure { audit_type: String, details: String },
}

// =============================================================================
// UTILITY FUNCTIONS AND TRAITS
// =============================================================================

/// Trait for secure message processing
pub trait SecureMessageProcessor {
    /// Process a secure message
    fn process_secure_message(&mut self, message: SecureMessage) -> Result<SecureMessage, SecurityProtocolError>;
    
    /// Validate message security
    fn validate_message_security(&self, message: &SecureMessage) -> Result<bool, SecurityProtocolError>;
    
    /// Extract decrypted payload
    fn extract_payload(&self, message: &SecureMessage) -> Result<Vec<u8>, SecurityProtocolError>;
}

/// Trait for authentication providers
pub trait AuthenticationProvider {
    /// Authenticate a principal
    fn authenticate(&mut self, auth_message: AuthenticationMessage) -> Result<AuthenticationResult, SecurityProtocolError>;
    
    /// Validate authentication
    fn validate_authentication(&self, identity: &AuthenticatedIdentity) -> Result<bool, SecurityProtocolError>;
    
    /// Refresh authentication
    fn refresh_authentication(&mut self, context: &SecurityContext) -> Result<AuthenticationResult, SecurityProtocolError>;
}

/// Trait for authorization providers
pub trait AuthorizationProvider {
    /// Authorize an operation
    fn authorize(&mut self, authz_message: AuthorizationMessage) -> Result<AuthorizationDecision, SecurityProtocolError>;
    
    /// Check permissions
    fn check_permissions(&self, subject: &AuthorizationSubject, resource: &AuthorizationResource, action: &AuthorizationAction) -> Result<bool, SecurityProtocolError>;
    
    /// Update authorization cache
    fn update_cache(&mut self, cache_entry: CacheEntry) -> Result<(), SecurityProtocolError>;
}

/// Trait for encryption providers
pub trait EncryptionProvider {
    /// Encrypt a message
    fn encrypt_message(&mut self, plaintext: &[u8], context: &SecurityContext) -> Result<EncryptedPayload, SecurityProtocolError>;
    
    /// Decrypt a message
    fn decrypt_message(&mut self, encrypted: &EncryptedPayload, context: &SecurityContext) -> Result<Vec<u8>, SecurityProtocolError>;
    
    /// Generate session keys
    fn generate_session_keys(&mut self, key_exchange: &KeyExchangeMessage) -> Result<SessionKeys, SecurityProtocolError>;
}

/// Trait for certificate validators
pub trait CertificateValidator {
    /// Validate a certificate
    fn validate_certificate(&mut self, cert: &Certificate, validation_request: &CertificateValidationRequest) -> Result<CertificateValidationResult, SecurityProtocolError>;
    
    /// Check certificate revocation
    fn check_revocation(&self, cert: &Certificate) -> Result<RevocationStatus, SecurityProtocolError>;
    
    /// Build certificate chain
    fn build_chain(&self, cert: &Certificate, trust_anchors: &[TrustAnchor]) -> Result<Vec<Certificate>, SecurityProtocolError>;
}

/// Result type for security protocol operations
pub type SecurityProtocolResult<T> = Result<T, SecurityProtocolError>;

// =============================================================================
// CONSTANTS AND DEFAULTS
// =============================================================================

/// Security protocol version
pub const SECURITY_PROTOCOL_VERSION: &str = "1.0.0";

/// Default message timeout
pub const DEFAULT_MESSAGE_TIMEOUT: Duration = Duration::from_secs(30);

/// Default handshake timeout
pub const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);

/// Default key rotation interval
pub const DEFAULT_KEY_ROTATION_INTERVAL: Duration = Duration::from_secs(86400); // 24 hours

/// Default session timeout
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour

/// Default cache TTL
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// Maximum message size
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16MB

/// Maximum handshake messages
pub const MAX_HANDSHAKE_MESSAGES: usize = 10;

/// Default nonce size
pub const DEFAULT_NONCE_SIZE: usize = 12;

/// Default authentication tag size
pub const DEFAULT_AUTH_TAG_SIZE: usize = 16;
