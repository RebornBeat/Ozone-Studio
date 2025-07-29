// External dependencies for cryptographic operations and security
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use std::fmt;
use std::error::Error as StdError;

// Cryptographic dependencies
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, SharedSecret};
use aes_gcm::{Aes256Gcm, Key, Nonce, NewAead, Aead};
use rand::{rngs::OsRng, RngCore};
use sha2::{Sha256, Digest};

// Async runtime and networking
use tokio::sync::{RwLock, Mutex};
use tokio::time::Instant;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

// Certificate and TLS handling
use rustls::{Certificate, PrivateKey, ServerConfig, ClientConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};

// Error handling
use thiserror::Error;
use anyhow::{Result, Context};

// Certificate authority and PKI management
pub mod certificate_authority;
pub mod key_management;
pub mod encryption;
pub mod protocols;

// Re-export core security types and traits
pub use certificate_authority::{
    CertificateAuthority,
    CertificateIssuer,
    CertificateValidator,
    EcosystemCertificate,
    AIAppCertificate,
    UserCertificate,
    CertificateValidationResult,
    CertificateIssuanceRequest,
    CertificateRevocationList,
};

pub use key_management::{
    EcosystemKeyManager,
    AIAppKeyManager,
    UserKeyManager,
    MasterKeyPair,
    AIAppKeyPair,
    UserKeyPair,
    KeyRotationManager,
    KeyDerivationEngine,
    SecureKeyStorage,
};

pub use encryption::{
    MessageEncryption,
    KeyExchange,
    ForwardSecrecy,
    EncryptedMessage,
    EncryptionContext,
    DecryptionContext,
    EncryptionAlgorithm,
    KeyAgreementProtocol,
};

pub use protocols::{
    MutualTLS,
    APIAuthentication,
    UserPairing,
    DevicePairing,
    AuthenticationProtocol,
    AuthorizationProtocol,
    SecureChannelEstablishment,
};

// Core security error types
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },
    
    #[error("Authorization denied: {operation} not permitted for {principal}")]
    AuthorizationDenied { operation: String, principal: String },
    
    #[error("Certificate validation failed: {details}")]
    CertificateValidationFailed { details: String },
    
    #[error("Encryption operation failed: {details}")]
    EncryptionFailed { details: String },
    
    #[error("Key management error: {details}")]
    KeyManagementError { details: String },
    
    #[error("Protocol violation: {protocol} - {details}")]
    ProtocolViolation { protocol: String, details: String },
    
    #[error("Security configuration error: {details}")]
    ConfigurationError { details: String },
    
    #[error("Network security error: {details}")]
    NetworkSecurityError { details: String },
}

// Security configuration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub certificate_authority: CAConfig,
    pub encryption: EncryptionConfig,
    pub authentication: AuthenticationConfig,
    pub authorization: AuthorizationConfig,
    pub network_security: NetworkSecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CAConfig {
    pub ca_certificate_path: String,
    pub ca_private_key_path: String,
    pub certificate_validity_duration: Duration,
    pub auto_renewal_enabled: bool,
    pub revocation_list_update_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub message_encryption_algorithm: EncryptionAlgorithm,
    pub key_exchange_protocol: KeyAgreementProtocol,
    pub forward_secrecy_enabled: bool,
    pub key_rotation_interval: Duration,
    pub encryption_at_rest_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    pub mutual_tls_required: bool,
    pub api_key_authentication: bool,
    pub certificate_authentication: bool,
    pub session_timeout: Duration,
    pub max_failed_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    pub role_based_access_control: bool,
    pub fine_grained_permissions: bool,
    pub audit_logging_enabled: bool,
    pub privilege_escalation_detection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurityConfig {
    pub tls_version: String,
    pub cipher_suites: Vec<String>,
    pub certificate_pinning: bool,
    pub rate_limiting: RateLimitConfig,
    pub ddos_protection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub burst_limit: u32,
    pub ban_duration: Duration,
}

// Core security traits that all components must implement
pub trait SecureComponent {
    fn initialize_security(&mut self, config: &SecurityConfig) -> Result<(), SecurityError>;
    fn authenticate_peer(&self, peer_certificate: &Certificate) -> Result<bool, SecurityError>;
    fn authorize_operation(&self, operation: &str, principal: &str) -> Result<bool, SecurityError>;
    fn encrypt_message(&self, message: &[u8], recipient: &PublicKey) -> Result<EncryptedMessage, SecurityError>;
    fn decrypt_message(&self, encrypted_message: &EncryptedMessage) -> Result<Vec<u8>, SecurityError>;
}

pub trait AuthenticationProvider {
    fn authenticate(&self, credentials: &AuthenticationCredentials) -> Result<AuthenticationResult, SecurityError>;
    fn validate_session(&self, session_token: &str) -> Result<SessionValidationResult, SecurityError>;
    fn refresh_session(&self, refresh_token: &str) -> Result<AuthenticationResult, SecurityError>;
}

pub trait AuthorizationProvider {
    fn authorize(&self, principal: &str, resource: &str, operation: &str) -> Result<bool, SecurityError>;
    fn get_permissions(&self, principal: &str) -> Result<Vec<Permission>, SecurityError>;
    fn check_role(&self, principal: &str, role: &str) -> Result<bool, SecurityError>;
}

// Authentication and authorization types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationCredentials {
    pub credential_type: CredentialType,
    pub principal_id: String,
    pub credential_data: Vec<u8>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialType {
    Certificate,
    APIKey,
    SessionToken,
    BiometricData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationResult {
    pub success: bool,
    pub principal_id: String,
    pub session_token: String,
    pub refresh_token: String,
    pub expires_at: SystemTime,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionValidationResult {
    pub valid: bool,
    pub principal_id: String,
    pub expires_at: SystemTime,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub resource: String,
    pub operations: Vec<String>,
    pub constraints: Vec<PermissionConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConstraint {
    pub constraint_type: String,
    pub value: String,
}

// Session management types
#[derive(Debug, Clone)]
pub struct SecureSession {
    pub session_id: String,
    pub principal_id: String,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub permissions: Vec<Permission>,
    pub encryption_key: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, SecureSession>>>,
    config: AuthenticationConfig,
}

// Network security types
#[derive(Debug, Clone)]
pub struct SecureChannel {
    pub channel_id: String,
    pub local_peer: String,
    pub remote_peer: String,
    pub encryption_context: EncryptionContext,
    pub established_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct TLSConfiguration {
    pub server_config: Arc<ServerConfig>,
    pub client_config: Arc<ClientConfig>,
    pub certificate_chain: Vec<Certificate>,
    pub private_key: PrivateKey,
}

// Security result types
pub type SecurityResult<T> = Result<T, SecurityError>;

// Helper macros for security validation
#[macro_export]
macro_rules! require_authentication {
    ($auth_result:expr) => {
        match $auth_result {
            Ok(result) if result.success => result,
            Ok(_) => return Err(SecurityError::AuthenticationFailed { 
                reason: "Authentication validation failed".to_string() 
            }),
            Err(e) => return Err(e),
        }
    };
}

#[macro_export]
macro_rules! require_authorization {
    ($auth_check:expr, $operation:expr, $principal:expr) => {
        match $auth_check {
            Ok(true) => {},
            Ok(false) => return Err(SecurityError::AuthorizationDenied { 
                operation: $operation.to_string(),
                principal: $principal.to_string(),
            }),
            Err(e) => return Err(e),
        }
    };
}
