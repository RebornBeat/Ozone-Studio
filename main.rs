// OZONE STUDIO - Main Binary Entry Point
//
// This file serves as the main entry point for the OZONE STUDIO application,
// providing a simple wrapper that delegates to the CLI implementation.

use std::process;

fn main() {
    // Initialize logging
    if let Err(e) = ozone_studio::utils::logging::init() {
        eprintln!("Failed to initialize logging: {}", e);
    }

    // Run the CLI and handle any errors
    if let Err(e) = ozone_studio::cli::run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
