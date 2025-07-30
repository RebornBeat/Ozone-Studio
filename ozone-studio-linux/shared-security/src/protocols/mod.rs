use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use std::net::SocketAddr;

// TLS and networking for secure protocols
use rustls::{ServerConfig, ClientConfig, Certificate, PrivateKey, Session};
use tokio_rustls::{TlsAcceptor, TlsConnector, server::TlsStream, client::TlsStream as ClientTlsStream};
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::{RwLock, Mutex};

// Cryptographic operations for protocol implementation
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, SharedSecret};
use rand::{rngs::OsRng, RngCore};

// Serialization and async support
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;

// Import core security types
use crate::{SecurityError, SecurityResult};

// Protocol implementation submodules
pub mod mutual_tls;           // Mutual TLS for AI App authentication
pub mod api_authentication;   // API-based authentication protocols
pub mod user_pairing;         // User device pairing protocols
pub mod device_discovery;     // Secure device discovery and registration
pub mod session_management;   // Secure session establishment and management
pub mod protocol_negotiation; // Protocol version and capability negotiation

// Re-export protocol types that other security modules need
pub use mutual_tls::{
    MutualTLS,
    TLSConfiguration,
    TLSHandshake,
    TLSSession,
    CertificateVerification,
    TLSSecurityPolicy,
};

pub use api_authentication::{
    APIAuthentication,
    APICredentials,
    APIToken,
    APISecurityContext,
    APIAuthenticationPolicy,
    TokenValidation,
};

pub use user_pairing::{
    UserPairing,
    PairingRequest,
    PairingResponse,
    PairingChallenge,
    DevicePairing,
    PairingVerification,
};

pub use device_discovery::{
    DeviceDiscovery,
    DiscoveryBeacon,
    DiscoveryResponse,
    DeviceCapabilities,
    SecurityCapabilities,
    TrustEstablishment,
};

pub use session_management::{
    SessionManagement,
    SecureSessionProtocol,
    SessionEstablishment,
    SessionMaintenance,
    SessionTermination,
    SessionSecurity,
};

pub use protocol_negotiation::{
    ProtocolNegotiation,
    ProtocolCapabilities,
    NegotiationRequest,
    NegotiationResponse,
    ProtocolSelection,
    CompatibilityCheck,
};

// Main protocol coordinator that manages all security protocols
pub struct SecurityProtocolCoordinator {
    mutual_tls: Arc<MutualTLS>,
    api_authentication: Arc<APIAuthentication>,
    user_pairing: Arc<UserPairing>,
    device_discovery: Arc<DeviceDiscovery>,
    session_management: Arc<SessionManagement>,
    protocol_negotiation: Arc<ProtocolNegotiation>,
    configuration: ProtocolConfiguration,
    active_connections: Arc<RwLock<HashMap<String, ActiveConnection>>>,
}

impl SecurityProtocolCoordinator {
    /// Initialize the complete security protocol system
    /// This sets up all protocols needed for secure ecosystem communication
    pub async fn initialize(config: ProtocolConfiguration) -> SecurityResult<Self> {
        // Initialize mutual TLS for AI App authentication
        let mutual_tls = Arc::new(
            MutualTLS::new(&config.mutual_tls_config).await?
        );

        // Set up API authentication for programmatic access
        let api_authentication = Arc::new(
            APIAuthentication::new(&config.api_auth_config).await?
        );

        // Initialize user pairing for human user authentication
        let user_pairing = Arc::new(
            UserPairing::new(&config.user_pairing_config).await?
        );

        // Set up secure device discovery
        let device_discovery = Arc::new(
            DeviceDiscovery::new(&config.device_discovery_config).await?
        );

        // Initialize session management
        let session_management = Arc::new(
            SessionManagement::new(&config.session_config).await?
        );

        // Set up protocol negotiation
        let protocol_negotiation = Arc::new(
            ProtocolNegotiation::new(&config.negotiation_config).await?
        );

        Ok(Self {
            mutual_tls,
            api_authentication,
            user_pairing,
            device_discovery,
            session_management,
            protocol_negotiation,
            configuration: config,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Establish a secure connection between two ecosystem entities
    /// This negotiates the appropriate security protocol based on entity types
    pub async fn establish_secure_connection(
        &self,
        local_entity: &EntityInfo,
        remote_entity: &EntityInfo,
        connection_context: &ConnectionContext,
    ) -> SecurityResult<SecureConnection> {
        // Generate connection ID for tracking
        let connection_id = Uuid::new_v4().to_string();

        // Negotiate the appropriate security protocol
        let protocol_selection = self.protocol_negotiation
            .negotiate_security_protocol(local_entity, remote_entity, connection_context)
            .await?;

        // Establish the connection using the selected protocol
        let secure_connection = match protocol_selection.selected_protocol {
            SecurityProtocol::MutualTLS => {
                self.establish_mutual_tls_connection(
                    &connection_id,
                    local_entity,
                    remote_entity,
                    &protocol_selection
                ).await?
            },
            SecurityProtocol::APIAuthentication => {
                self.establish_api_authenticated_connection(
                    &connection_id,
                    local_entity,
                    remote_entity,
                    &protocol_selection
                ).await?
            },
            SecurityProtocol::UserPairing => {
                self.establish_user_paired_connection(
                    &connection_id,
                    local_entity,
                    remote_entity,
                    &protocol_selection
                ).await?
            },
        };

        // Register the active connection
        {
            let mut active_connections = self.active_connections.write().await;
            active_connections.insert(
                connection_id.clone(),
                ActiveConnection {
                    connection: secure_connection.clone(),
                    established_at: SystemTime::now(),
                    last_activity: SystemTime::now(),
                    connection_metrics: ConnectionMetrics::new(),
                }
            );
        }

        Ok(secure_connection)
    }

    /// Authenticate an AI App using mutual TLS with certificate verification
    /// This is the primary authentication method for AI App to OZONE STUDIO communication
    pub async fn authenticate_ai_app(
        &self,
        app_certificate: &Certificate,
        connection_info: &ConnectionInfo,
    ) -> SecurityResult<AIAppAuthenticationResult> {
        // Perform mutual TLS handshake and certificate verification
        let tls_verification = self.mutual_tls
            .verify_ai_app_certificate(app_certificate, connection_info)
            .await?;

        if !tls_verification.certificate_valid {
            return Err(SecurityError::AuthenticationFailed {
                reason: format!("AI App certificate verification failed: {:?}", tls_verification.validation_errors)
            });
        }

        // Extract AI App identity and capabilities from certificate
        let app_identity = self.mutual_tls
            .extract_app_identity(app_certificate)
            .await?;

        let app_capabilities = self.mutual_tls
            .extract_app_capabilities(app_certificate)
            .await?;

        // Create authentication session
        let auth_session = self.session_management
            .create_ai_app_session(&app_identity, &app_capabilities)
            .await?;

        Ok(AIAppAuthenticationResult {
            authentication_successful: true,
            app_identity,
            app_capabilities,
            auth_session,
            tls_session: tls_verification.tls_session,
            authenticated_at: SystemTime::now(),
        })
    }

    /// Authenticate a human user through device pairing
    /// This enables secure human access to the ecosystem through BRIDGE
    pub async fn authenticate_user(
        &self,
        user_credentials: &UserCredentials,
        device_info: &DeviceInfo,
        pairing_context: &PairingContext,
    ) -> SecurityResult<UserAuthenticationResult> {
        // Initiate user pairing process
        let pairing_request = self.user_pairing
            .initiate_user_pairing(user_credentials, device_info, pairing_context)
            .await?;

        // Complete the pairing challenge-response
        let pairing_result = self.user_pairing
            .complete_pairing_challenge(pairing_request)
            .await?;

        if !pairing_result.pairing_successful {
            return Err(SecurityError::AuthenticationFailed {
                reason: format!("User pairing failed: {:?}", pairing_result.failure_reason)
            });
        }

        // Create user session with device binding
        let user_session = self.session_management
            .create_user_session(
                &pairing_result.user_identity,
                &pairing_result.device_binding
            )
            .await?;

        Ok(UserAuthenticationResult {
            authentication_successful: true,
            user_identity: pairing_result.user_identity,
            device_binding: pairing_result.device_binding,
            user_session,
            authenticated_at: SystemTime::now(),
        })
    }

    /// Discover and securely register new devices in the ecosystem
    /// This enables ecosystem expansion across multiple devices
    pub async fn discover_and_register_device(
        &self,
        discovery_context: &DiscoveryContext,
    ) -> SecurityResult<DeviceRegistrationResult> {
        // Start secure device discovery
        let discovery_result = self.device_discovery
            .discover_ecosystem_devices(discovery_context)
            .await?;

        // Verify discovered devices have valid security capabilities
        let mut registered_devices = Vec::new();
        for discovered_device in discovery_result.discovered_devices {
            // Verify device security capabilities
            let security_verification = self.device_discovery
                .verify_device_security_capabilities(&discovered_device)
                .await?;

            if security_verification.security_adequate {
                // Establish trust with the device
                let trust_establishment = self.device_discovery
                    .establish_device_trust(&discovered_device)
                    .await?;

                if trust_establishment.trust_established {
                    registered_devices.push(RegisteredDevice {
                        device_info: discovered_device,
                        security_capabilities: security_verification.capabilities,
                        trust_level: trust_establishment.trust_level,
                        registered_at: SystemTime::now(),
                    });
                }
            }
        }

        Ok(DeviceRegistrationResult {
            discovery_session_id: discovery_result.session_id,
            devices_discovered: discovery_result.discovered_devices.len(),
            devices_registered: registered_devices.len(),
            registered_devices,
            discovery_completed_at: SystemTime::now(),
        })
    }

    /// Validate an existing secure session for continued use
    /// This ensures sessions remain valid and secure over time
    pub async fn validate_session(
        &self,
        session_id: &str,
        validation_context: &SessionValidationContext,
    ) -> SecurityResult<SessionValidationResult> {
        // Get the active connection for this session
        let active_connection = {
            let active_connections = self.active_connections.read().await;
            active_connections.get(session_id)
                .ok_or_else(|| SecurityError::AuthenticationFailed {
                    reason: format!("Session {} not found", session_id)
                })?
                .clone()
        };

        // Validate the session using the session management system
        let session_validation = self.session_management
            .validate_active_session(&active_connection.connection.session_info, validation_context)
            .await?;

        // Check if session needs renewal
        if session_validation.needs_renewal {
            let renewal_result = self.session_management
                .renew_session(&active_connection.connection.session_info)
                .await?;

            if renewal_result.renewal_successful {
                // Update the active connection with renewed session
                self.update_active_connection_session(session_id, renewal_result.renewed_session).await?;
            }
        }

        Ok(SessionValidationResult {
            session_valid: session_validation.session_valid,
            validation_errors: session_validation.validation_errors,
            session_expires_at: session_validation.expires_at,
            renewal_performed: session_validation.needs_renewal,
            validated_at: SystemTime::now(),
        })
    }

    /// Helper method to update active connection session after renewal
    async fn update_active_connection_session(
        &self,
        connection_id: &str,
        renewed_session: SessionInfo,
    ) -> SecurityResult<()> {
        let mut active_connections = self.active_connections.write().await;
        if let Some(active_connection) = active_connections.get_mut(connection_id) {
            active_connection.connection.session_info = renewed_session;
            active_connection.last_activity = SystemTime::now();
        }
        Ok(())
    }
    
    /// Establish a mutual TLS connection for AI App communication
    /// This is the primary secure communication method between AI Apps and OZONE STUDIO
    async fn establish_mutual_tls_connection(
        &self,
        connection_id: &str,
        local_entity: &EntityInfo,
        remote_entity: &EntityInfo,
        protocol_selection: &ProtocolSelection,
    ) -> SecurityResult<SecureConnection> {
        // Create TLS configuration for this specific connection
        let tls_config = self.mutual_tls
            .create_connection_config(local_entity, remote_entity)
            .await?;

        // Establish the TLS handshake
        let tls_handshake = self.mutual_tls
            .perform_handshake(&tls_config, &protocol_selection.connection_parameters)
            .await?;

        if !tls_handshake.handshake_successful {
            return Err(SecurityError::AuthenticationFailed {
                reason: format!("TLS handshake failed: {:?}", tls_handshake.failure_reason)
            });
        }

        // Create the secure connection wrapper
        Ok(SecureConnection {
            connection_id: connection_id.to_string(),
            connection_type: ConnectionType::MutualTLS,
            local_entity: local_entity.clone(),
            remote_entity: remote_entity.clone(),
            security_context: SecurityContext::TLS(tls_handshake.security_context),
            established_at: SystemTime::now(),
            last_activity: SystemTime::now(),
        })
    }

    /// Establish an API-authenticated connection for programmatic access
    /// This is used for external tool integration and automated systems
    async fn establish_api_authenticated_connection(
        &self,
        connection_id: &str,
        local_entity: &EntityInfo,
        remote_entity: &EntityInfo,
        protocol_selection: &ProtocolSelection,
    ) -> SecurityResult<SecureConnection> {
        // Validate API credentials
        let api_validation = self.api_authentication
            .validate_api_credentials(&protocol_selection.authentication_data)
            .await?;

        if !api_validation.credentials_valid {
            return Err(SecurityError::AuthenticationFailed {
                reason: format!("API credentials validation failed: {:?}", api_validation.validation_errors)
            });
        }

        // Create API security context
        let api_context = self.api_authentication
            .create_security_context(&api_validation)
            .await?;

        Ok(SecureConnection {
            connection_id: connection_id.to_string(),
            connection_type: ConnectionType::APIAuthentication,
            local_entity: local_entity.clone(),
            remote_entity: remote_entity.clone(),
            security_context: SecurityContext::API(api_context),
            established_at: SystemTime::now(),
            last_activity: SystemTime::now(),
        })
    }

    /// Establish a user-paired connection for human access through BRIDGE
    /// This enables secure human interaction with the AGI ecosystem
    async fn establish_user_paired_connection(
        &self,
        connection_id: &str,
        local_entity: &EntityInfo,
        remote_entity: &EntityInfo,
        protocol_selection: &ProtocolSelection,
    ) -> SecurityResult<SecureConnection> {
        // Validate user pairing credentials
        let pairing_validation = self.user_pairing
            .validate_pairing_credentials(&protocol_selection.authentication_data)
            .await?;

        if !pairing_validation.pairing_valid {
            return Err(SecurityError::AuthenticationFailed {
                reason: format!("User pairing validation failed: {:?}", pairing_validation.validation_errors)
            });
        }

        // Create user security context with device binding
        let user_context = self.user_pairing
            .create_user_security_context(&pairing_validation)
            .await?;

        Ok(SecureConnection {
            connection_id: connection_id.to_string(),
            connection_type: ConnectionType::UserPairing,
            local_entity: local_entity.clone(),
            remote_entity: remote_entity.clone(),
            security_context: SecurityContext::User(user_context),
            established_at: SystemTime::now(),
            last_activity: SystemTime::now(),
        })
    }

    /// Validate an active connection's security status
    /// This performs ongoing security validation for established connections
    pub async fn validate_connection_security(
        &self,
        connection_id: &str,
    ) -> SecurityResult<ConnectionSecurityStatus> {
        let active_connections = self.active_connections.read().await;
        
        let connection = active_connections.get(connection_id)
            .ok_or_else(|| SecurityError::AuthenticationFailed {
                reason: format!("Connection {} not found", connection_id)
            })?;

        // Perform security validation based on connection type
        let security_status = match &connection.connection.connection_type {
            ConnectionType::MutualTLS => {
                self.mutual_tls.validate_connection_security(&connection.connection).await?
            },
            ConnectionType::APIAuthentication => {
                self.api_authentication.validate_connection_security(&connection.connection).await?
            },
            ConnectionType::UserPairing => {
                self.user_pairing.validate_connection_security(&connection.connection).await?
            },
        };

        // Update connection activity timestamp
        drop(active_connections);
        let mut active_connections = self.active_connections.write().await;
        if let Some(conn) = active_connections.get_mut(connection_id) {
            conn.last_activity = SystemTime::now();
            conn.connection_metrics.validation_count += 1;
        }

        Ok(security_status)
    }

    /// Terminate a secure connection and clean up resources
    /// This ensures proper cleanup of security contexts and sessions
    pub async fn terminate_connection(
        &self,
        connection_id: &str,
        termination_reason: TerminationReason,
    ) -> SecurityResult<()> {
        let mut active_connections = self.active_connections.write().await;
        
        let connection = active_connections.remove(connection_id)
            .ok_or_else(|| SecurityError::AuthenticationFailed {
                reason: format!("Connection {} not found for termination", connection_id)
            })?;

        // Perform connection-type specific cleanup
        match &connection.connection.connection_type {
            ConnectionType::MutualTLS => {
                self.mutual_tls.terminate_connection(&connection.connection, &termination_reason).await?;
            },
            ConnectionType::APIAuthentication => {
                self.api_authentication.terminate_connection(&connection.connection, &termination_reason).await?;
            },
            ConnectionType::UserPairing => {
                self.user_pairing.terminate_connection(&connection.connection, &termination_reason).await?;
            },
        }

        // Clean up associated sessions
        self.session_management.cleanup_connection_sessions(connection_id).await?;

        Ok(())
    }

    /// Get security metrics for all active connections
    /// This provides visibility into the security posture of the ecosystem
    pub async fn get_security_metrics(&self) -> SecurityResult<SecurityMetrics> {
        let active_connections = self.active_connections.read().await;
        
        let total_connections = active_connections.len();
        let mut connection_types = HashMap::new();
        let mut total_validation_count = 0;
        let mut average_connection_age = Duration::from_secs(0);
        
        let now = SystemTime::now();
        let mut connection_ages = Vec::new();

        for (_, active_connection) in active_connections.iter() {
            // Count connection types
            let conn_type = match active_connection.connection.connection_type {
                ConnectionType::MutualTLS => "mutual_tls",
                ConnectionType::APIAuthentication => "api_auth", 
                ConnectionType::UserPairing => "user_pairing",
            };
            *connection_types.entry(conn_type.to_string()).or_insert(0) += 1;

            // Accumulate metrics
            total_validation_count += active_connection.connection_metrics.validation_count;
            
            if let Ok(age) = now.duration_since(active_connection.established_at) {
                connection_ages.push(age);
            }
        }

        // Calculate average connection age
        if !connection_ages.is_empty() {
            let total_age: Duration = connection_ages.iter().sum();
            average_connection_age = total_age / connection_ages.len() as u32;
        }

        // Get protocol-specific metrics
        let mutual_tls_metrics = self.mutual_tls.get_security_metrics().await?;
        let api_auth_metrics = self.api_authentication.get_security_metrics().await?;
        let user_pairing_metrics = self.user_pairing.get_security_metrics().await?;

        Ok(SecurityMetrics {
            total_active_connections: total_connections,
            connection_types,
            total_validations_performed: total_validation_count,
            average_connection_age,
            mutual_tls_metrics,
            api_authentication_metrics: api_auth_metrics,
            user_pairing_metrics,
            metrics_collected_at: SystemTime::now(),
        })
    }

    /// Perform security audit of all active protocols
    /// This comprehensive audit helps maintain security posture
    pub async fn perform_security_audit(&self) -> SecurityResult<SecurityAuditReport> {
        let mut audit_findings = Vec::new();
        let mut recommendations = Vec::new();
        let mut critical_issues = Vec::new();

        // Audit mutual TLS configuration and certificates
        let tls_audit = self.mutual_tls.perform_security_audit().await?;
        audit_findings.extend(tls_audit.findings);
        recommendations.extend(tls_audit.recommendations);
        critical_issues.extend(tls_audit.critical_issues);

        // Audit API authentication policies and token management
        let api_audit = self.api_authentication.perform_security_audit().await?;
        audit_findings.extend(api_audit.findings);
        recommendations.extend(api_audit.recommendations);
        critical_issues.extend(api_audit.critical_issues);

        // Audit user pairing and device management
        let pairing_audit = self.user_pairing.perform_security_audit().await?;
        audit_findings.extend(pairing_audit.findings);
        recommendations.extend(pairing_audit.recommendations);
        critical_issues.extend(pairing_audit.critical_issues);

        // Audit session management and lifecycle
        let session_audit = self.session_management.perform_security_audit().await?;
        audit_findings.extend(session_audit.findings);
        recommendations.extend(session_audit.recommendations);
        critical_issues.extend(session_audit.critical_issues);

        // Calculate overall security score
        let security_score = self.calculate_security_score(&audit_findings, &critical_issues);

        Ok(SecurityAuditReport {
            audit_id: Uuid::new_v4().to_string(),
            audit_timestamp: SystemTime::now(),
            overall_security_score: security_score,
            total_findings: audit_findings.len(),
            critical_issues_count: critical_issues.len(),
            audit_findings,
            recommendations,
            critical_issues,
            next_audit_recommended: SystemTime::now() + Duration::from_secs(86400 * 7), // 1 week
        })
    }

    /// Calculate overall security score based on audit findings
    /// This provides a quantitative measure of ecosystem security health
    fn calculate_security_score(&self, findings: &[AuditFinding], critical_issues: &[CriticalIssue]) -> f64 {
        let base_score = 100.0;
        let critical_penalty = critical_issues.len() as f64 * 20.0; // -20 points per critical issue
        let finding_penalty = findings.iter()
            .map(|f| match f.severity {
                AuditSeverity::Critical => 15.0,
                AuditSeverity::High => 10.0,
                AuditSeverity::Medium => 5.0,
                AuditSeverity::Low => 2.0,
                AuditSeverity::Info => 0.0,
            })
            .sum::<f64>();

        (base_score - critical_penalty - finding_penalty).max(0.0)
    }
}

// Core protocol configuration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfiguration {
    pub mutual_tls_config: MutualTLSConfig,
    pub api_auth_config: APIAuthenticationConfig,
    pub user_pairing_config: UserPairingConfig,
    pub device_discovery_config: DeviceDiscoveryConfig,
    pub session_config: SessionManagementConfig,
    pub negotiation_config: ProtocolNegotiationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutualTLSConfig {
    pub certificate_path: String,
    pub private_key_path: String,
    pub ca_certificate_path: String,
    pub cipher_suites: Vec<String>,
    pub protocol_versions: Vec<String>,
    pub certificate_verification_strict: bool,
    pub session_resumption_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIAuthenticationConfig {
    pub token_signing_key_path: String,
    pub token_expiration_duration: Duration,
    pub api_key_length: usize,
    pub rate_limiting_enabled: bool,
    pub max_requests_per_minute: u32,
    pub token_refresh_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPairingConfig {
    pub pairing_timeout: Duration,
    pub challenge_complexity: ChallengeComplexity,
    pub device_binding_required: bool,
    pub multi_factor_authentication: bool,
    pub biometric_authentication: bool,
    pub max_paired_devices: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeComplexity {
    Simple,
    Standard,
    Complex,
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDiscoveryConfig {
    pub discovery_timeout: Duration,
    pub broadcast_interval: Duration,
    pub security_verification_required: bool,
    pub trust_establishment_method: TrustEstablishmentMethod,
    pub discovery_encryption_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustEstablishmentMethod {
    CertificateChain,
    WebOfTrust,
    ManualVerification,
    AutomaticWithThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManagementConfig {
    pub session_timeout: Duration,
    pub session_renewal_enabled: bool,
    pub concurrent_sessions_per_user: u32,
    pub session_encryption_enabled: bool,
    pub session_integrity_checks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolNegotiationConfig {
    pub negotiation_timeout: Duration,
    pub supported_protocols: Vec<SecurityProtocol>,
    pub protocol_preferences: HashMap<String, u32>,
    pub fallback_protocol: SecurityProtocol,
    pub compatibility_mode_enabled: bool,
}

// Core types for protocol operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub entity_id: String,
    pub entity_type: EntityType,
    pub security_capabilities: Vec<SecurityCapability>,
    pub trust_level: TrustLevel,
    pub certificate: Option<Certificate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    AIApp,
    HumanUser,
    ExternalTool,
    EcosystemService,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityCapability {
    MutualTLS,
    APIAuthentication,
    UserPairing,
    EndToEndEncryption,
    DigitalSigning,
    BiometricAuth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    Untrusted,
    Basic,
    Standard,
    High,
    Maximum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionContext {
    pub connection_purpose: ConnectionPurpose,
    pub security_requirements: Vec<SecurityRequirement>,
    pub performance_requirements: PerformanceRequirements,
    pub context_metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionPurpose {
    AIAppCommunication,
    HumanInterface,
    DataTransfer,
    SystemAdministration,
    ExternalIntegration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityRequirement {
    Authentication,
    Authorization,
    Encryption,
    Integrity,
    NonRepudiation,
    PrivacyProtection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    pub max_latency: Duration,
    pub min_throughput: u64,
    pub reliability_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityProtocol {
    MutualTLS,
    APIAuthentication,
    UserPairing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureConnection {
    pub connection_id: String,
    pub connection_type: ConnectionType,
    pub local_entity: EntityInfo,
    pub remote_entity: EntityInfo,
    pub security_context: SecurityContext,
    pub established_at: SystemTime,
    pub last_activity: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    MutualTLS,
    APIAuthentication,
    UserPairing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityContext {
    TLS(TLSSecurityContext),
    API(APISecurityContext),
    User(UserSecurityContext),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TLSSecurityContext {
    pub cipher_suite: String,
    pub protocol_version: String,
    pub session_id: String,
    pub peer_certificate: Certificate,
    pub session_resumption_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSecurityContext {
    pub user_id: String,
    pub device_id: String,
    pub authentication_factors: Vec<AuthenticationFactor>,
    pub session_token: String,
    pub device_binding: DeviceBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationFactor {
    Password,
    Certificate,
    Biometric,
    Token,
    Challenge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceBinding {
    pub device_fingerprint: String,
    pub binding_certificate: Certificate,
    pub binding_established_at: SystemTime,
    pub binding_strength: BindingStrength,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BindingStrength {
    Weak,
    Standard,
    Strong,
    Cryptographic,
}

// Active connection tracking
#[derive(Debug, Clone)]
pub struct ActiveConnection {
    pub connection: SecureConnection,
    pub established_at: SystemTime,
    pub last_activity: SystemTime,
    pub connection_metrics: ConnectionMetrics,
}

#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub validation_count: u64,
    pub error_count: u64,
}

impl ConnectionMetrics {
    pub fn new() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
            validation_count: 0,
            error_count: 0,
        }
    }
}

// Authentication result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAppAuthenticationResult {
    pub authentication_successful: bool,
    pub app_identity: AIAppIdentity,
    pub app_capabilities: Vec<String>,
    pub auth_session: SessionInfo,
    pub tls_session: TLSSessionInfo,
    pub authenticated_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAppIdentity {
    pub app_id: String,
    pub app_name: String,
    pub app_type: String,
    pub version: String,
    pub developer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub expires_at: SystemTime,
    pub permissions: Vec<String>,
    pub session_type: SessionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    AIApp,
    User,
    ExternalTool,
    SystemService,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TLSSessionInfo {
    pub session_id: String,
    pub cipher_suite: String,
    pub protocol_version: String,
    pub resumption_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAuthenticationResult {
    pub authentication_successful: bool,
    pub user_identity: UserIdentity,
    pub device_binding: DeviceBinding,
    pub user_session: SessionInfo,
    pub authenticated_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    pub user_id: String,
    pub display_name: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub account_created: SystemTime,
}

// Additional credential and info types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredentials {
    pub credential_type: UserCredentialType,
    pub primary_credential: Vec<u8>,
    pub secondary_factors: Vec<SecondaryFactor>,
    pub device_context: DeviceContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserCredentialType {
    Password,
    Certificate,
    Biometric,
    Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryFactor {
    pub factor_type: AuthenticationFactor,
    pub factor_data: Vec<u8>,
    pub expires_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
    pub operating_system: String,
    pub hardware_fingerprint: String,
    pub security_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceContext {
    pub location: Option<String>,
    pub network_info: NetworkInfo,
    pub security_posture: SecurityPosture,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub ip_address: String,
    pub connection_type: String,
    pub security_level: NetworkSecurityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkSecurityLevel {
    Public,
    Protected,
    Private,
    Secure,
    HighSecurity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPosture {
    pub firewall_enabled: bool,
    pub antivirus_enabled: bool,
    pub encryption_enabled: bool,
    pub security_updates_current: bool,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingContext {
    pub pairing_method: PairingMethod,
    pub security_level: SecurityLevel,
    pub trust_requirements: TrustRequirements,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PairingMethod {
    QRCode,
    SharedSecret,
    CertificateExchange,
    BiometricChallenge,
    MultiFactorChallenge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    Basic,
    Standard,
    Enhanced,
    Maximum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRequirements {
    pub minimum_trust_level: TrustLevel,
    pub verification_required: bool,
    pub additional_factors_required: u32,
}

// Device discovery and registration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryContext {
    pub discovery_scope: DiscoveryScope,
    pub security_requirements: Vec<SecurityRequirement>,
    pub timeout: Duration,
    pub trust_establishment_method: TrustEstablishmentMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryScope {
    LocalNetwork,
    SecureNetwork,
    Internet,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistrationResult {
    pub discovery_session_id: String,
    pub devices_discovered: usize,
    pub devices_registered: usize,
    pub registered_devices: Vec<RegisteredDevice>,
    pub discovery_completed_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredDevice {
    pub device_info: DeviceInfo,
    pub security_capabilities: SecurityCapabilities,
    pub trust_level: TrustLevel,
    pub registered_at: SystemTime,
}

// Connection management types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub remote_address: SocketAddr,
    pub connection_time: SystemTime,
    pub protocol_version: String,
    pub security_context: Option<SecurityContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSecurityStatus {
    pub connection_id: String,
    pub security_valid: bool,
    pub last_validated: SystemTime,
    pub validation_errors: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminationReason {
    UserRequest,
    SecurityViolation,
    Timeout,
    SystemShutdown,
    ProtocolError,
    MaintenanceMode,
}

// Security metrics and audit types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub total_active_connections: usize,
    pub connection_types: HashMap<String, usize>,
    pub total_validations_performed: u64,
    pub average_connection_age: Duration,
    pub mutual_tls_metrics: ProtocolMetrics,
    pub api_authentication_metrics: ProtocolMetrics,
    pub user_pairing_metrics: ProtocolMetrics,
    pub metrics_collected_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMetrics {
    pub active_sessions: usize,
    pub successful_authentications: u64,
    pub failed_authentications: u64,
    pub security_violations: u64,
    pub average_session_duration: Duration,
    pub protocol_specific_metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditReport {
    pub audit_id: String,
    pub audit_timestamp: SystemTime,
    pub overall_security_score: f64,
    pub total_findings: usize,
    pub critical_issues_count: usize,
    pub audit_findings: Vec<AuditFinding>,
    pub recommendations: Vec<SecurityRecommendation>,
    pub critical_issues: Vec<CriticalIssue>,
    pub next_audit_recommended: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFinding {
    pub finding_id: String,
    pub category: AuditCategory,
    pub severity: AuditSeverity,
    pub description: String,
    pub affected_components: Vec<String>,
    pub risk_level: RiskLevel,
    pub remediation_priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditCategory {
    Authentication,
    Authorization,
    Encryption,
    CertificateManagement,
    SessionManagement,
    NetworkSecurity,
    Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Immediate,
    High,
    Medium,
    Low,
    Planned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRecommendation {
    pub recommendation_id: String,
    pub title: String,
    pub description: String,
    pub impact: SecurityImpact,
    pub implementation_effort: ImplementationEffort,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityImpact {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationEffort {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalIssue {
    pub issue_id: String,
    pub title: String,
    pub description: String,
    pub severity_score: f64,
    pub affected_systems: Vec<String>,
    pub immediate_actions_required: Vec<String>,
    pub discovered_at: SystemTime,
}

// Protocol-specific result and validation types that are commonly used
pub type ProtocolResult<T> = Result<T, SecurityError>;

/// Utility trait for protocol security validation
pub trait SecurityValidator {
    fn validate_security_context(&self, context: &SecurityContext) -> ProtocolResult<bool>;
    fn assess_risk_level(&self, entity: &EntityInfo, context: &ConnectionContext) -> ProtocolResult<RiskLevel>;
    fn recommend_security_improvements(&self) -> ProtocolResult<Vec<SecurityRecommendation>>;
}

/// Trait for managing protocol lifecycle
pub trait ProtocolLifecycle {
    fn initialize(&mut self, config: &ProtocolConfiguration) -> ProtocolResult<()>;
    fn activate(&mut self) -> ProtocolResult<()>;
    fn deactivate(&mut self) -> ProtocolResult<()>;
    fn cleanup(&mut self) -> ProtocolResult<()>;
}

// Implementation of Default for commonly used types
impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ProtocolConfiguration {
    fn default() -> Self {
        Self {
            mutual_tls_config: MutualTLSConfig {
                certificate_path: "/etc/ozone-studio/certs/server.crt".to_string(),
                private_key_path: "/etc/ozone-studio/private/server.key".to_string(),
                ca_certificate_path: "/etc/ozone-studio/ca/ca.crt".to_string(),
                cipher_suites: vec![
                    "TLS_AES_256_GCM_SHA384".to_string(),
                    "TLS_CHACHA20_POLY1305_SHA256".to_string(),
                ],
                protocol_versions: vec!["TLSv1.3".to_string()],
                certificate_verification_strict: true,
                session_resumption_enabled: true,
            },
            api_auth_config: APIAuthenticationConfig {
                token_signing_key_path: "/etc/ozone-studio/keys/api-signing.key".to_string(),
                token_expiration_duration: Duration::from_secs(3600), // 1 hour
                api_key_length: 32,
                rate_limiting_enabled: true,
                max_requests_per_minute: 1000,
                token_refresh_enabled: true,
            },
            user_pairing_config: UserPairingConfig {
                pairing_timeout: Duration::from_secs(300), // 5 minutes
                challenge_complexity: ChallengeComplexity::Standard,
                device_binding_required: true,
                multi_factor_authentication: false,
                biometric_authentication: false,
                max_paired_devices: 10,
            },
            device_discovery_config: DeviceDiscoveryConfig {
                discovery_timeout: Duration::from_secs(60),
                broadcast_interval: Duration::from_secs(10),
                security_verification_required: true,
                trust_establishment_method: TrustEstablishmentMethod::CertificateChain,
                discovery_encryption_enabled: true,
            },
            session_config: SessionManagementConfig {
                session_timeout: Duration::from_secs(1800), // 30 minutes
                session_renewal_enabled: true,
                concurrent_sessions_per_user: 5,
                session_encryption_enabled: true,
                session_integrity_checks: true,
            },
            negotiation_config: ProtocolNegotiationConfig {
                negotiation_timeout: Duration::from_secs(30),
                supported_protocols: vec![
                    SecurityProtocol::MutualTLS,
                    SecurityProtocol::APIAuthentication,
                    SecurityProtocol::UserPairing,
                ],
                protocol_preferences: {
                    let mut prefs = HashMap::new();
                    prefs.insert("MutualTLS".to_string(), 100);
                    prefs.insert("APIAuthentication".to_string(), 80);
                    prefs.insert("UserPairing".to_string(), 60);
                    prefs
                },
                fallback_protocol: SecurityProtocol::MutualTLS,
                compatibility_mode_enabled: false,
            },
        }
    }
}
