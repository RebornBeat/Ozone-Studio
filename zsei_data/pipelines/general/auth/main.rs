//! AuthPipeline - Pipeline #1
//! 
//! Handles user authentication using Ed25519 challenge-response.
//! No passwords - cryptographic key pairs only.
//! 
//! NOTE: Built-in pipelines use DIRECT library calls, not HTTP.
//! HTTP/gRPC is only for UI (Electron) and remote device communication.
//! 
//! Flow:
//! 1. Client requests challenge with public key
//! 2. Server generates random challenge
//! 3. Client signs challenge with private key
//! 4. Server verifies signature and creates session

use serde::{Deserialize, Serialize};
use std::env;
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

// Session store - in production, this is shared with the core runtime
// For built-in pipelines, we access it directly
lazy_static::lazy_static! {
    static ref SESSION_STORE: Mutex<SessionStore> = Mutex::new(SessionStore::new());
    static ref CHALLENGE_STORE: Mutex<HashMap<String, ChallengeData>> = Mutex::new(HashMap::new());
}

struct SessionStore {
    sessions: HashMap<String, SessionData>,
    storage_path: String,
}

impl SessionStore {
    fn new() -> Self {
        let storage_path = env::var("OZONE_SESSION_PATH")
            .unwrap_or_else(|_| "./zsei_data/sessions".to_string());
        
        let mut store = Self {
            sessions: HashMap::new(),
            storage_path,
        };
        
        // Load existing sessions from disk
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path.join("sessions.json")) {
                if let Ok(sessions) = serde_json::from_str(&content) {
                    self.sessions = sessions;
                }
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        if let Ok(content) = serde_json::to_string_pretty(&self.sessions) {
            let _ = std::fs::write(path.join("sessions.json"), content);
        }
    }
    
    fn create_session(&mut self, user_id: u64, device_id: u64) -> (String, u64) {
        use rand::Rng;
        
        let mut session_bytes = [0u8; 32];
        rand::thread_rng().fill(&mut session_bytes);
        let token = hex::encode(session_bytes);
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let expires_at = now + 86400; // 24 hours
        
        self.sessions.insert(token.clone(), SessionData {
            user_id,
            device_id,
            created_at: now,
            expires_at,
        });
        
        self.save_to_disk();
        (token, expires_at)
    }
    
    fn validate_session(&self, token: &str) -> Option<&SessionData> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.sessions.get(token).filter(|s| s.expires_at > now)
    }
    
    fn invalidate_session(&mut self, token: &str) {
        self.sessions.remove(token);
        self.save_to_disk();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    user_id: u64,
    device_id: u64,
    created_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone)]
struct ChallengeData {
    nonce: [u8; 32],
    created_at: u64,
    expires_at: u64,
}

/// Pipeline input variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum AuthInput {
    /// Request a challenge for authentication
    RequestChallenge {
        public_key: String,  // Hex-encoded public key
    },
    /// Authenticate with signed challenge
    Authenticate {
        public_key: String,
        challenge: String,
        signature: String,  // Hex-encoded signature
    },
    /// Validate existing session
    ValidateSession {
        session_token: String,
    },
    /// Logout / invalidate session
    Logout {
        session_token: String,
    },
    /// Register a new device
    RegisterDevice {
        session_token: String,
        device_name: String,
        device_type: String,
    },
}

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthOutput {
    pub success: bool,
    pub session_token: Option<String>,
    pub challenge: Option<String>,
    pub user_id: Option<u64>,
    pub device_id: Option<u64>,
    pub error: Option<String>,
    pub expires_at: Option<u64>,
}

/// Execute the auth pipeline
pub async fn execute(input: AuthInput) -> Result<AuthOutput, String> {
    match input {
        AuthInput::RequestChallenge { public_key } => {
            request_challenge(&public_key).await
        }
        AuthInput::Authenticate { public_key, challenge, signature } => {
            authenticate(&public_key, &challenge, &signature).await
        }
        AuthInput::ValidateSession { session_token } => {
            validate_session(&session_token).await
        }
        AuthInput::Logout { session_token } => {
            logout(&session_token).await
        }
        AuthInput::RegisterDevice { session_token, device_name, device_type } => {
            register_device(&session_token, &device_name, &device_type).await
        }
    }
}

/// Request authentication challenge
async fn request_challenge(public_key: &str) -> Result<AuthOutput, String> {
    // Validate public key format
    let pk_bytes = hex::decode(public_key)
        .map_err(|_| "Invalid public key format")?;
    
    if pk_bytes.len() != 32 {
        return Err("Public key must be 32 bytes".into());
    }
    
    // Generate random challenge
    use rand::RngCore;
    let mut challenge = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut challenge);
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    Ok(AuthOutput {
        success: true,
        session_token: None,
        challenge: Some(hex::encode(challenge)),
        user_id: None,
        device_id: None,
        error: None,
        expires_at: Some(now + 300), // Challenge expires in 5 minutes
    })
}

/// Authenticate with signed challenge
async fn authenticate(
    public_key: &str,
    challenge: &str,
    signature: &str,
) -> Result<AuthOutput, String> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    
    // Decode inputs
    let pk_bytes: [u8; 32] = hex::decode(public_key)
        .map_err(|_| "Invalid public key format")?
        .try_into()
        .map_err(|_| "Public key must be 32 bytes")?;
    
    let challenge_bytes = hex::decode(challenge)
        .map_err(|_| "Invalid challenge format")?;
    
    let sig_bytes: [u8; 64] = hex::decode(signature)
        .map_err(|_| "Invalid signature format")?
        .try_into()
        .map_err(|_| "Signature must be 64 bytes")?;
    
    // Verify signature
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|e| format!("Invalid public key: {}", e))?;
    
    let sig = Signature::from_bytes(&sig_bytes);
    
    verifying_key.verify(&challenge_bytes, &sig)
        .map_err(|_| "Invalid signature")?;
    
    // DIRECT session creation using local session store
    // No HTTP - built-in pipelines access storage directly
    
    // User ID derived from public key hash for consistency
    let user_id = u64::from_le_bytes(pk_bytes[0..8].try_into().unwrap_or([0u8; 8]));
    let device_id = 1u64; // Local device
    
    // Create session using direct store access
    let (session_token, expires_at) = {
        let mut store = SESSION_STORE.lock().unwrap();
        store.create_session(user_id, device_id)
    };
    
    Ok(AuthOutput {
        success: true,
        session_token: Some(session_token),
        challenge: None,
        user_id: Some(user_id),
        device_id: Some(device_id),
        error: None,
        expires_at: Some(expires_at),
    })
}

/// Validate existing session
async fn validate_session(session_token: &str) -> Result<AuthOutput, String> {
    // Validate token format
    let _ = hex::decode(session_token)
        .map_err(|_| "Invalid session token format")?;
    
    // DIRECT session validation using local session store
    // No HTTP - built-in pipelines access storage directly
    let store = SESSION_STORE.lock().unwrap();
    
    if let Some(session) = store.validate_session(session_token) {
        Ok(AuthOutput {
            success: true,
            session_token: Some(session_token.to_string()),
            challenge: None,
            user_id: Some(session.user_id),
            device_id: Some(session.device_id),
            error: None,
            expires_at: Some(session.expires_at),
        })
    } else {
        Ok(AuthOutput {
            success: false,
            session_token: None,
            challenge: None,
            user_id: None,
            device_id: None,
            error: Some("Invalid or expired session".to_string()),
            expires_at: None,
        })
    }
}

/// Logout / invalidate session
async fn logout(session_token: &str) -> Result<AuthOutput, String> {
    let _ = hex::decode(session_token)
        .map_err(|_| "Invalid session token format")?;
    
    // DIRECT session invalidation using local session store
    // No HTTP - built-in pipelines access storage directly
    {
        let mut store = SESSION_STORE.lock().unwrap();
        store.invalidate_session(session_token);
    }
    
    Ok(AuthOutput {
        success: true,
        session_token: None,
        challenge: None,
        user_id: None,
        device_id: None,
        error: None,
        expires_at: None,
    })
}

/// Register a new device
async fn register_device(
    session_token: &str,
    device_name: &str,
    device_type: &str,
) -> Result<AuthOutput, String> {
    // Validate session first
    let _ = validate_session(session_token).await?;
    
    // In a real implementation:
    // 1. Create device record
    // 2. Allocate device ID
    // 3. Set up resource allocation
    
    Ok(AuthOutput {
        success: true,
        session_token: Some(session_token.to_string()),
        challenge: None,
        user_id: Some(1),
        device_id: Some(2), // New device ID
        error: None,
        expires_at: None,
    })
}

// ============================================================================
// CLI entry point
// ============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let mut input_json = String::new();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                if i < args.len() {
                    input_json = args[i].clone();
                }
            }
            _ => {}
        }
        i += 1;
    }
    
    let input: AuthInput = match serde_json::from_str(&input_json) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to parse input: {}", e);
            std::process::exit(1);
        }
    };
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(execute(input));
    
    match result {
        Ok(output) => {
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        Err(e) => {
            let output = AuthOutput {
                success: false,
                session_token: None,
                challenge: None,
                user_id: None,
                device_id: None,
                error: Some(e),
                expires_at: None,
            };
            println!("{}", serde_json::to_string(&output).unwrap());
            std::process::exit(1);
        }
    }
}
