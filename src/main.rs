//! Ozone Studio - Main Entry Point
//!
//! This is the bootloader that initializes the Ozone Studio runtime
//! and starts the gRPC server for UI communication.

use ozone_studio::{OzoneRuntime, OzoneConfig, init_logging_with_level, OzoneError};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), OzoneError> {
    // Load configuration first to get log level
    let config_path = std::env::var("OZONE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.toml"));
    
    // Pre-load config to get log level, or use default
    let log_level = if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => {
                // Try to extract log_level from existing config
                toml::from_str::<OzoneConfig>(&content)
                    .map(|c| c.general.log_level)
                    .unwrap_or_else(|_| "info".to_string())
            }
            Err(_) => "info".to_string(),
        }
    } else {
        "info".to_string()
    };
    
    // Initialize logging with configured level
    init_logging_with_level(&log_level);
    
    tracing::info!("╔═══════════════════════════════════════════════════════════════════╗");
    tracing::info!("║                        OZONE STUDIO                               ║");
    tracing::info!("║           Omnidirectional Zero-Shot Neural Engine                 ║");
    tracing::info!("║      A Collective AGI Framework with Optional Consciousness       ║");
    tracing::info!("║                        Version 0.4.0                              ║");
    tracing::info!("╚═══════════════════════════════════════════════════════════════════╝");
    
    // Load configuration (creates default if not exists)
    let config = OzoneConfig::load(&config_path)?;
    
    if config_path.exists() {
        tracing::info!("Configuration loaded from: {}", config_path.display());
    } else {
        tracing::info!("Created new configuration: {}", config_path.display());
    }
    
    // Log key settings
    tracing::info!("Log level: {}", config.general.log_level);
    tracing::info!("Data directory: {}", config.general.data_dir);
    tracing::info!("gRPC server: {}:{}", config.grpc.address, config.grpc.port);
    tracing::info!("P2P enabled: {}", config.network.enable_p2p);
    if config.network.enable_p2p {
        tracing::info!("P2P port: {}", config.network.p2p_port);
        tracing::info!("mDNS discovery: {}", config.network.enable_mdns);
    }
    tracing::info!("Model type: {}", config.models.model_type);
    
    // Log consciousness status (always compiled, enabled/disabled via config)
    tracing::info!("Consciousness enabled: {}", config.consciousness.enabled);
    
    // Initialize runtime
    tracing::info!("Initializing runtime...");
    let runtime = OzoneRuntime::new(config).await?;
    
    tracing::info!("Runtime initialized successfully");
    tracing::info!("────────────────────────────────────────────────────────────────────");
    tracing::info!("UI can connect to: http://{}:{}", 
        runtime.config.grpc.address, 
        runtime.config.grpc.port
    );
    tracing::info!("Press Ctrl+C to shutdown");
    tracing::info!("────────────────────────────────────────────────────────────────────");
    
    // Start the runtime (this blocks until shutdown)
    runtime.start().await?;
    
    tracing::info!("Ozone Studio shutdown complete");
    Ok(())
}
