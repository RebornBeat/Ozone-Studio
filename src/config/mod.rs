//! Configuration module for Ozone Studio

use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::OzoneError;

/// Main configuration for Ozone Studio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzoneConfig {
    /// General settings
    pub general: GeneralConfig,
    
    /// ZSEI configuration
    pub zsei: ZSEIConfig,
    
    /// Pipeline configuration
    pub pipelines: PipelineConfig,
    
    /// Task configuration
    pub tasks: TaskConfig,
    
    /// Authentication configuration
    pub auth: AuthConfig,
    
    /// Integrity configuration
    pub integrity: IntegrityConfig,
    
    /// Network configuration
    pub network: NetworkConfig,
    
    /// gRPC server configuration
    pub grpc: GrpcConfig,
    
    /// UI configuration
    pub ui: UIConfig,
    
    /// Consciousness configuration (if enabled)
    #[cfg(feature = "consciousness")]
    #[serde(default)]
    pub consciousness: ConsciousnessConfig,
    
    /// Model configuration (for prompt pipeline)
    pub models: ModelConfig,
}

impl Default for OzoneConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            zsei: ZSEIConfig::default(),
            pipelines: PipelineConfig::default(),
            tasks: TaskConfig::default(),
            auth: AuthConfig::default(),
            integrity: IntegrityConfig::default(),
            network: NetworkConfig::default(),
            grpc: GrpcConfig::default(),
            ui: UIConfig::default(),
            #[cfg(feature = "consciousness")]
            consciousness: ConsciousnessConfig::default(),
            models: ModelConfig::default(),
        }
    }
}

impl OzoneConfig {
    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self, OzoneError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| OzoneError::ConfigError(format!("Failed to read config: {}", e)))?;
            toml::from_str(&content)
                .map_err(|e| OzoneError::ConfigError(format!("Failed to parse config: {}", e)))
        } else {
            // Create default config
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }
    
    /// Save configuration to a TOML file
    pub fn save(&self, path: &Path) -> Result<(), OzoneError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| OzoneError::ConfigError(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(path, content)
            .map_err(|e| OzoneError::ConfigError(format!("Failed to write config: {}", e)))?;
        Ok(())
    }
}

/// General configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub data_dir: String,
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            data_dir: "zsei_data".into(),
            log_level: "info".into(),
        }
    }
}

/// ZSEI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZSEIConfig {
    pub global_path: String,
    pub local_path: String,
    pub cache_path: String,
    pub ml_path: String,
    pub max_containers_in_memory: usize,
    pub mmap_enabled: bool,
    pub embedding_dimension: usize,
}

impl Default for ZSEIConfig {
    fn default() -> Self {
        Self {
            global_path: "zsei_data/global".into(),
            local_path: "zsei_data/local".into(),
            cache_path: "zsei_data/cache".into(),
            ml_path: "zsei_data/ml".into(),
            max_containers_in_memory: 10000,
            mmap_enabled: true,
            embedding_dimension: 384,
        }
    }
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub builtin_path: String,
    pub custom_path: String,
    pub max_concurrent_pipelines: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            builtin_path: "pipelines".into(),
            custom_path: "pipelines/custom".into(),
            max_concurrent_pipelines: 10,
        }
    }
}

/// Task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub max_queued_tasks: usize,
    pub task_timeout_secs: u64,
    pub preserve_completed_tasks: bool,
    pub max_task_history: usize,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            max_queued_tasks: 100,
            task_timeout_secs: 3600, // 1 hour
            preserve_completed_tasks: true,
            max_task_history: 1000,
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub keystore_path: String,
    pub session_duration_secs: u64,
    pub challenge_expiry_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            keystore_path: "zsei_data/keystore".into(),
            session_duration_secs: 86400, // 24 hours
            challenge_expiry_secs: 300,    // 5 minutes
        }
    }
}

/// Integrity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityConfig {
    pub enabled: bool,
    pub check_interval_secs: u64,
    pub rollback_path: String,
    pub max_versions: u32,
}

impl Default for IntegrityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_secs: 3600, // 1 hour
            rollback_path: "zsei_data/integrity/rollback".into(),
            max_versions: 100,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable P2P networking
    pub enable_p2p: bool,
    /// Enable cloud sync (separate from P2P)
    pub enable_cloud_sync: bool,
    /// Bootstrap nodes for P2P discovery
    pub bootstrap_nodes: Vec<String>,
    /// P2P listen port
    pub p2p_port: u16,
    /// Maximum connected peers
    pub max_peers: u32,
    /// Enable mDNS local discovery
    pub enable_mdns: bool,
    /// Sync interval in seconds for batch sync
    pub batch_sync_interval_secs: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enable_p2p: true,  // Enabled by default for P2P
            enable_cloud_sync: false,  // Disabled - local first
            bootstrap_nodes: Vec::new(),
            p2p_port: 9090,
            max_peers: 50,
            enable_mdns: true,
            batch_sync_interval_secs: 60,
        }
    }
}

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    pub address: String,
    pub port: u16,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1".into(),
            port: 50051,
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub theme: String,
    pub default_tabs: Vec<String>,
    pub meta_portion_width_percent: u8,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            theme: "home_dashboard".into(),
            default_tabs: vec!["workspace".into(), "library".into(), "settings".into()],
            meta_portion_width_percent: 20,
        }
    }
}

/// Consciousness configuration (if enabled)
#[cfg(feature = "consciousness")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessConfig {
    pub enabled: bool,
    pub emotional_system_enabled: bool,
    pub experience_memory_enabled: bool,
    pub identity_system_enabled: bool,
    pub relationship_system_enabled: bool,
    pub ethical_system_enabled: bool,
    pub collective_enabled: bool,
    pub show_emotional_state: bool,
    pub show_decision_reasoning: bool,
    pub i_loop_interval_ms: u64,
    pub playback_enabled: bool,
}

#[cfg(feature = "consciousness")]
impl Default for ConsciousnessConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            emotional_system_enabled: true,
            experience_memory_enabled: true,
            identity_system_enabled: true,
            relationship_system_enabled: true,
            ethical_system_enabled: true,
            collective_enabled: false,
            show_emotional_state: true,
            show_decision_reasoning: true,
            i_loop_interval_ms: 60000, // 1 minute
            playback_enabled: true,
        }
    }
}

/// Model configuration for prompt pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model type: "api", "gguf", "onnx"
    pub model_type: String,
    
    /// For API models
    pub api_endpoint: Option<String>,
    pub api_key_env: Option<String>,
    pub api_model: Option<String>,
    
    /// For local models (GGUF/ONNX)
    pub local_model_path: Option<String>,
    pub context_length: usize,
    pub gpu_layers: Option<u32>,
    
    /// Model selection UI setting
    pub allow_user_selection: bool,
    pub available_models: Vec<AvailableModel>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_type: "api".into(),
            api_endpoint: Some("https://api.anthropic.com/v1/messages".into()),
            api_key_env: Some("ANTHROPIC_API_KEY".into()),
            api_model: Some("claude-sonnet-4-20250514".into()),
            local_model_path: None,
            context_length: 100000,
            gpu_layers: None,
            allow_user_selection: true,
            available_models: vec![
                AvailableModel {
                    name: "Claude Sonnet".into(),
                    model_type: "api".into(),
                    identifier: "claude-sonnet-4-20250514".into(),
                },
            ],
        }
    }
}

/// Available model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableModel {
    pub name: String,
    pub model_type: String,
    pub identifier: String,
}
