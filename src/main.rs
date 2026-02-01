//! Ozone Studio - Main Entry Point
//!
//! This is the bootloader that initializes the Ozone Studio runtime
//! and starts the gRPC server for UI communication.

use ozone_studio::{OzoneRuntime, OzoneConfig, init_logging, OzoneError};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), OzoneError> {
    // Initialize logging
    init_logging();
    
    tracing::info!("╔═══════════════════════════════════════════════════════════╗");
    tracing::info!("║                     OZONE STUDIO                          ║");
    tracing::info!("║        Omnidirectional Zero-Shot Neural Engine            ║");
    tracing::info!("║                     Version 0.3.0                         ║");
    tracing::info!("╚═══════════════════════════════════════════════════════════╝");
    
    // Load configuration
    let config_path = std::env::var("OZONE_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.toml"));
    
    let config = OzoneConfig::load(&config_path)?;
    
    tracing::info!("Configuration loaded from: {}", config_path.display());
    
    // Initialize runtime
    let runtime = OzoneRuntime::new(config).await?;
    
    tracing::info!("Runtime initialized successfully");
    
    // Start the runtime (this blocks until shutdown)
    runtime.start().await?;
    
    Ok(())
}
