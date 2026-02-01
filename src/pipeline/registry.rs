//! Pipeline registry utilities
//!
//! Additional utilities for pipeline registration and discovery.

use crate::types::{PipelineID, PipelineBlueprint, OzoneResult};
use std::path::Path;

/// Load pipeline blueprint from file
pub fn load_blueprint_from_file(path: &Path) -> OzoneResult<PipelineBlueprint> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::OzoneError::PipelineError(format!("Failed to read blueprint: {}", e)))?;
    
    // Try TOML first, then JSON
    if path.extension().map(|e| e == "toml").unwrap_or(false) {
        toml::from_str(&content)
            .map_err(|e| crate::OzoneError::PipelineError(format!("Failed to parse TOML blueprint: {}", e)))
    } else {
        serde_json::from_str(&content)
            .map_err(|e| crate::OzoneError::PipelineError(format!("Failed to parse JSON blueprint: {}", e)))
    }
}

/// Save pipeline blueprint to file
pub fn save_blueprint_to_file(blueprint: &PipelineBlueprint, path: &Path) -> OzoneResult<()> {
    let content = if path.extension().map(|e| e == "toml").unwrap_or(false) {
        toml::to_string_pretty(blueprint)
            .map_err(|e| crate::OzoneError::PipelineError(format!("Failed to serialize TOML: {}", e)))?
    } else {
        serde_json::to_string_pretty(blueprint)
            .map_err(|e| crate::OzoneError::PipelineError(format!("Failed to serialize JSON: {}", e)))?
    };
    
    std::fs::write(path, content)
        .map_err(|e| crate::OzoneError::PipelineError(format!("Failed to write blueprint: {}", e)))?;
    
    Ok(())
}

/// Scan directory for pipeline blueprints
pub fn scan_pipeline_directory(dir: &Path) -> OzoneResult<Vec<PipelineBlueprint>> {
    let mut blueprints = Vec::new();
    
    if !dir.exists() {
        return Ok(blueprints);
    }
    
    for entry in std::fs::read_dir(dir)
        .map_err(|e| crate::OzoneError::PipelineError(format!("Failed to read directory: {}", e)))? 
    {
        let entry = entry
            .map_err(|e| crate::OzoneError::PipelineError(format!("Failed to read entry: {}", e)))?;
        let path = entry.path();
        
        // Look for blueprint files
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str());
            if ext == Some("toml") || ext == Some("json") {
                if let Ok(blueprint) = load_blueprint_from_file(&path) {
                    blueprints.push(blueprint);
                }
            }
        }
        
        // Also check subdirectories for pipeline.toml/pipeline.json
        if path.is_dir() {
            let toml_path = path.join("pipeline.toml");
            let json_path = path.join("pipeline.json");
            
            if toml_path.exists() {
                if let Ok(blueprint) = load_blueprint_from_file(&toml_path) {
                    blueprints.push(blueprint);
                }
            } else if json_path.exists() {
                if let Ok(blueprint) = load_blueprint_from_file(&json_path) {
                    blueprints.push(blueprint);
                }
            }
        }
    }
    
    Ok(blueprints)
}

/// Validate pipeline blueprint
pub fn validate_blueprint(blueprint: &PipelineBlueprint) -> OzoneResult<Vec<String>> {
    let mut warnings = Vec::new();
    
    // Check required fields
    if blueprint.name.is_empty() {
        return Err(crate::OzoneError::ValidationError("Pipeline name is required".into()));
    }
    
    if blueprint.description.is_empty() {
        warnings.push("Pipeline description is empty".into());
    }
    
    // Check for circular dependencies
    if blueprint.specification.dependencies.contains(&blueprint.pipeline_id) {
        return Err(crate::OzoneError::ValidationError(
            "Pipeline cannot depend on itself".into()
        ));
    }
    
    // Check for duplicate sub-pipelines
    let mut seen = std::collections::HashSet::new();
    for sub in &blueprint.specification.sub_pipelines {
        if !seen.insert(sub) {
            warnings.push(format!("Duplicate sub-pipeline: {}", sub));
        }
    }
    
    Ok(warnings)
}
