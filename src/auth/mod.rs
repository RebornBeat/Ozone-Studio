//! Authentication system
//!
//! Based on Section 4 of the specification.
//!
//! Uses Ed25519 signatures for challenge-response authentication.
//! No passwords - cryptographic key pairs only.

use crate::config::AuthConfig;
use crate::types::{UserID, DeviceID, OzoneError, OzoneResult, PublicKey};
use crate::types::auth::{User, Session, DeviceRegistration, DeviceType, AuthChallenge};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Authentication system
pub struct AuthSystem {
    /// Configuration
    config: AuthConfig,
    
    /// Registered users
    users: Arc<RwLock<HashMap<UserID, User>>>,
    
    /// Active sessions
    sessions: Arc<RwLock<HashMap<Vec<u8>, Session>>>,
    
    /// Pending challenges
    challenges: Arc<RwLock<HashMap<Vec<u8>, AuthChallenge>>>,
    
    /// Next user ID
    next_user_id: Arc<RwLock<UserID>>,
    
    /// Next device ID
    next_device_id: Arc<RwLock<DeviceID>>,
}

impl AuthSystem {
    /// Create new auth system
    pub fn new(config: &AuthConfig) -> OzoneResult<Self> {
        // Ensure keystore directory exists
        std::fs::create_dir_all(&config.keystore_path)
            .map_err(|e| OzoneError::AuthError(format!("Failed to create keystore: {}", e)))?;
        
        Ok(Self {
            config: config.clone(),
            users: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            challenges: Arc::new(RwLock::new(HashMap::new())),
            next_user_id: Arc::new(RwLock::new(1)),
            next_device_id: Arc::new(RwLock::new(1)),
        })
    }
    
    /// Create authentication challenge
    pub async fn create_challenge(&self, public_key: &[u8]) -> OzoneResult<AuthChallenge> {
        // Generate random challenge
        let mut challenge_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut challenge_bytes);
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let challenge = AuthChallenge {
            challenge_id: rand::random(),
            challenge: challenge_bytes.to_vec(),
            nonce: challenge_bytes,
            public_key: public_key.to_vec(),
            created_at: now,
            expires_at: now + self.config.challenge_expiry_secs,
        };
        
        // Store challenge
        self.challenges.write().await
            .insert(public_key.to_vec(), challenge.clone());
        
        Ok(challenge)
    }
    
    /// Authenticate with signed challenge
    pub async fn authenticate(
        &self,
        public_key: &[u8],
        signature: &[u8],
    ) -> OzoneResult<Session> {
        // Get pending challenge
        let challenge = self.challenges.write().await
            .remove(public_key)
            .ok_or_else(|| OzoneError::AuthError("No pending challenge".into()))?;
        
        // Check expiry
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now > challenge.expires_at {
            return Err(OzoneError::AuthError("Challenge expired".into()));
        }
        
        // Verify signature
        self.verify_signature(public_key, &challenge.challenge, signature)?;
        
        // Get or create user
        let user = self.get_or_create_user(public_key).await?;
        
        // Get or create device for this public key
        let device_id = self.get_or_create_device(&user, public_key).await?;
        
        // Create session
        let mut session_token = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut session_token);
        
        let session = Session {
            session_id: rand::random(),
            session_token: session_token.to_vec(),
            token: session_token.to_vec(),
            user_id: user.user_id,
            device_id,
            created_at: now,
            expires_at: now + self.config.session_duration_secs,
            last_activity: now,
            active_workspace: None,
            active_project: None,
        };
        
        // Store session
        self.sessions.write().await
            .insert(session_token.to_vec(), session.clone());
        
        tracing::info!("User {} authenticated", user.user_id);
        
        Ok(session)
    }
    
    /// Verify Ed25519 signature
    fn verify_signature(
        &self,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> OzoneResult<()> {
        // Parse public key
        let pk_bytes: [u8; 32] = public_key.try_into()
            .map_err(|_| OzoneError::AuthError("Invalid public key length".into()))?;
        
        let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
            .map_err(|e| OzoneError::AuthError(format!("Invalid public key: {}", e)))?;
        
        // Parse signature
        let sig_bytes: [u8; 64] = signature.try_into()
            .map_err(|_| OzoneError::AuthError("Invalid signature length".into()))?;
        
        let sig = Signature::from_bytes(&sig_bytes);
        
        // Verify
        verifying_key.verify(message, &sig)
            .map_err(|_| OzoneError::AuthError("Invalid signature".into()))?;
        
        Ok(())
    }
    
    /// Get or create user by public key
    async fn get_or_create_user(&self, public_key: &[u8]) -> OzoneResult<User> {
        // Check if user exists
        let users = self.users.read().await;
        for user in users.values() {
            if user.public_key == public_key {
                return Ok(user.clone());
            }
        }
        drop(users);
        
        // Create new user
        let mut next_id = self.next_user_id.write().await;
        let user_id = *next_id;
        *next_id += 1;
        drop(next_id);
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let user = User {
            user_id,
            public_key: public_key.to_vec(),
            private_key_hash: Vec::new(),
            registered_devices: Vec::new(),
            devices: Vec::new(),
            workspaces: Vec::new(),
            permissions: crate::types::auth::Permissions::default(),
            contribution_score: 0.0,
            contribution_rank: None,
            contribution_count: 0,
            reputation: 0.5,
            created_at: now,
            last_login: now,
            last_seen: now,
        };
        
        self.users.write().await.insert(user_id, user.clone());
        
        tracing::info!("Created new user {}", user_id);
        
        Ok(user)
    }
    
    /// Get or create device for a user based on public key
    async fn get_or_create_device(&self, user: &User, public_key: &[u8]) -> OzoneResult<DeviceID> {
        // Check if device with this public key exists for user
        let users = self.users.read().await;
        if let Some(u) = users.get(&user.user_id) {
            for device in &u.registered_devices {
                if device.public_key == public_key {
                    return Ok(device.device_id);
                }
            }
        }
        drop(users);
        
        // Create new device
        let mut next_id = self.next_device_id.write().await;
        let device_id = *next_id;
        *next_id += 1;
        drop(next_id);
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let device = DeviceRegistration {
            device_id,
            device_name: format!("Device-{}", device_id),
            device_type: DeviceType::Custom("unknown".to_string()),
            public_key: public_key.to_vec(),
            registered_at: now,
            last_seen: now,
            status: crate::types::auth::DeviceStatus::Online,
            resource_contribution: crate::types::auth::ResourceAllocation::default(),
            capabilities: Vec::new(),
        };
        
        // Add device to user
        let mut users = self.users.write().await;
        if let Some(u) = users.get_mut(&user.user_id) {
            u.registered_devices.push(device);
        }
        
        tracing::info!("Created new device {} for user {}", device_id, user.user_id);
        
        Ok(device_id)
    }
    
    /// Validate session token
    pub async fn validate_session(&self, token: &[u8]) -> OzoneResult<Session> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(token)
            .ok_or_else(|| OzoneError::AuthError("Invalid session".into()))?;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now > session.expires_at {
            drop(sessions);
            self.sessions.write().await.remove(token);
            return Err(OzoneError::AuthError("Session expired".into()));
        }
        
        Ok(session.clone())
    }
    
    /// Update session activity
    pub async fn touch_session(&self, token: &[u8]) -> OzoneResult<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(token)
            .ok_or_else(|| OzoneError::AuthError("Invalid session".into()))?;
        
        session.last_activity = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Ok(())
    }
    
    /// Logout (invalidate session)
    pub async fn logout(&self, token: &[u8]) -> OzoneResult<()> {
        self.sessions.write().await.remove(token);
        Ok(())
    }
    
    /// Get user by ID
    pub async fn get_user(&self, user_id: UserID) -> Option<User> {
        self.users.read().await.get(&user_id).cloned()
    }
    
    /// Register a device for a user
    pub async fn register_device(
        &self,
        user_id: UserID,
        device: DeviceRegistration,
    ) -> OzoneResult<DeviceID> {
        let mut users = self.users.write().await;
        let user = users.get_mut(&user_id)
            .ok_or_else(|| OzoneError::NotFound(format!("User {} not found", user_id)))?;
        
        let mut next_id = self.next_device_id.write().await;
        let device_id = *next_id;
        *next_id += 1;
        
        let mut device = device;
        device.device_id = device_id;
        
        user.registered_devices.push(device);
        
        Ok(device_id)
    }
}
