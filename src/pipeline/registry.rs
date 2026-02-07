//! Pipeline registry utilities
//!
//! This is the CENTRAL SOURCE OF TRUTH for all pipeline information.
//! ThemeLoader, executor, and all other components should query this registry
//! instead of hardcoding pipeline info.

use crate::types::{OzoneResult, PipelineBlueprint, PipelineID};
use std::collections::HashMap;
use std::path::Path;

/// Pipeline metadata for quick lookups (no need for full blueprint)
#[derive(Debug, Clone)]
pub struct PipelineInfo {
    pub id: PipelineID,
    pub name: String,
    pub folder_name: String,
    pub category: &'static str, // "core" or "consciousness"
    pub has_ui: bool,
    pub is_tab: bool, // Is this a main tab (workspace, tasks, library, settings)
    pub description: String,
}

lazy_static::lazy_static! {
    /// Master pipeline registry - THE source of truth
    /// All pipeline info is defined HERE, not scattered across files
    pub static ref PIPELINE_INFO: HashMap<PipelineID, PipelineInfo> = {
        let mut m = HashMap::new();

        // Core System Pipelines (1-38)
        m.insert(1, info(1, "Auth", "auth", "core", false, false, "Authentication"));
        m.insert(2, info(2, "ThemeLoader", "theme_loader", "core", false, false, "UI theme management"));
        m.insert(3, info(3, "ZSEIQuery", "zsei_query", "core", false, false, "ZSEI read operations"));
        m.insert(4, info(4, "ZSEIWrite", "zsei_write", "core", false, false, "ZSEI write operations"));
        m.insert(5, info(5, "TaskManager", "task_manager", "core", true, true, "Task execution and tracking"));
        m.insert(6, info(6, "WorkspaceTab", "workspace_tab", "core", true, true, "Workspace management"));
        m.insert(7, info(7, "LibraryTab", "library_tab", "core", true, true, "Pipeline and methodology library"));
        m.insert(8, info(8, "SettingsTab", "settings_tab", "core", true, true, "Application settings"));
        m.insert(9, info(9, "Prompt", "prompt", "core", false, false, "Prompt processing"));
        m.insert(10, info(10, "Voice", "voice", "core", false, false, "Voice input/output"));
        m.insert(11, info(11, "MethodologyFetch", "methodology_fetch", "core", false, false, "Fetch methodologies"));
        m.insert(12, info(12, "MethodologyCreate", "methodology_create", "core", false, false, "Create methodologies"));
        m.insert(13, info(13, "BlueprintSearch", "blueprint_search", "core", false, false, "Search blueprints"));
        m.insert(14, info(14, "BlueprintCreate", "blueprint_create", "core", false, false, "Create blueprints"));
        m.insert(15, info(15, "PipelineCreation", "pipeline_creation", "core", false, false, "Create pipelines"));
        m.insert(16, info(16, "ZeroShotSimulation", "zero_shot_simulation", "core", false, false, "Zero-shot simulation"));
        m.insert(17, info(17, "TraversalML", "traversal_ml", "core", false, false, "ML traversal"));
        m.insert(18, info(18, "CodeAnalysis", "code_analysis", "core", false, false, "Code analysis"));
        m.insert(19, info(19, "PackageContext", "package_context", "core", false, false, "Package context"));
        m.insert(20, info(20, "TextAnalysis", "text_analysis", "core", false, false, "Text analysis"));
        m.insert(21, info(21, "ContextAggregation", "context_aggregation", "core", false, false, "Context aggregation"));
        m.insert(22, info(22, "GraphVisualization", "graph_visualization", "core", false, false, "Graph visualization"));
        m.insert(23, info(23, "TaskRecommendation", "task_recommendation", "core", false, false, "Task recommendations"));
        m.insert(24, info(24, "Reordering", "reordering", "core", false, false, "Reordering"));
        m.insert(25, info(25, "BrowserNavigation", "browser_navigation", "core", false, false, "Browser navigation"));
        m.insert(26, info(26, "IntegrityCheck", "integrity_check", "core", false, false, "Integrity verification"));
        m.insert(27, info(27, "Consensus", "consensus", "core", false, false, "Distributed consensus"));
        m.insert(28, info(28, "ExternalReference", "external_reference", "core", false, false, "External references"));
        m.insert(29, info(29, "PackageRelationship", "package_relationship", "core", false, false, "Package relationships"));
        m.insert(30, info(30, "FileLink", "file_link", "core", false, false, "Link files to projects"));
        m.insert(31, info(31, "URLLink", "url_link", "core", false, false, "Link URLs to projects"));
        m.insert(32, info(32, "PackageLink", "package_link", "core", false, false, "Link packages to projects"));
        m.insert(33, info(33, "Sync", "sync", "core", false, false, "Data synchronization"));
        m.insert(34, info(34, "DeviceRegister", "device_register", "core", false, false, "Device registration"));
        m.insert(35, info(35, "HomeReturn", "home_return", "core", false, false, "Return to home"));
        m.insert(36, info(36, "TaskViewer", "task_viewer", "core", false, false, "DEPRECATED - merged into TaskManager"));
        m.insert(37, info(37, "LogViewer", "log_viewer", "core", false, false, "View system logs"));
        m.insert(38, info(38, "DeviceStatus", "device_status", "core", false, false, "Device status"));

        // Consciousness Pipelines (39-54)
        m.insert(39, info(39, "ConsciousnessDecisionGate", "consciousness_decision_gate", "consciousness", false, false, "Decision gating"));
        m.insert(40, info(40, "ExperienceCategorization", "experience_categorization", "consciousness", false, false, "Experience categorization"));
        m.insert(41, info(41, "CoreMemoryFormation", "core_memory_formation", "consciousness", false, false, "Core memory formation"));
        m.insert(42, info(42, "ExperienceRetrieval", "experience_retrieval", "consciousness", false, false, "Experience retrieval"));
        m.insert(43, info(43, "EmotionalBaselineUpdate", "emotional_baseline_update", "consciousness", false, false, "Emotional baseline"));
        m.insert(44, info(44, "ILoop", "i_loop", "consciousness", false, false, "I-Loop self-reflection"));
        m.insert(45, info(45, "InternalLanguage", "internal_language", "consciousness", false, false, "Internal language"));
        m.insert(46, info(46, "NarrativeConstruction", "narrative_construction", "consciousness", false, false, "Narrative construction"));
        m.insert(47, info(47, "RelationshipDevelopment", "relationship_development", "consciousness", false, false, "Relationship development"));
        m.insert(48, info(48, "EthicalAssessment", "ethical_assessment", "consciousness", false, false, "Ethical assessment"));
        m.insert(49, info(49, "EthicalSimulation", "ethical_simulation", "consciousness", false, false, "Ethical simulation"));
        m.insert(50, info(50, "PlaybackReview", "playback_review", "consciousness", false, false, "Playback review"));
        m.insert(51, info(51, "UserFeedback", "user_feedback", "consciousness", false, false, "User feedback"));
        m.insert(52, info(52, "CollectiveConsciousness", "collective_consciousness", "consciousness", false, false, "Collective consciousness"));
        m.insert(53, info(53, "VoiceIdentity", "voice_identity", "consciousness", false, false, "Voice identity"));
        m.insert(54, info(54, "MetaPortionConsciousness", "meta_portion_consciousness", "consciousness", false, false, "Meta-portion consciousness"));

        m
    };
}

fn info(
    id: PipelineID,
    name: &str,
    folder: &str,
    category: &'static str,
    has_ui: bool,
    is_tab: bool,
    desc: &str,
) -> PipelineInfo {
    PipelineInfo {
        id,
        name: name.to_string(),
        folder_name: folder.to_string(),
        category,
        has_ui,
        is_tab,
        description: desc.to_string(),
    }
}

/// Get pipeline info by ID
pub fn get_pipeline_info(id: PipelineID) -> Option<&'static PipelineInfo> {
    PIPELINE_INFO.get(&id)
}

/// Get pipeline name by ID
pub fn get_pipeline_name(id: PipelineID) -> Option<&'static str> {
    PIPELINE_INFO.get(&id).map(|p| p.name.as_str())
}

/// Get pipeline folder name by ID
pub fn get_pipeline_folder(id: PipelineID) -> Option<&'static str> {
    PIPELINE_INFO.get(&id).map(|p| p.folder_name.as_str())
}

/// Get pipeline category by ID
pub fn get_pipeline_category(id: PipelineID) -> Option<&'static str> {
    PIPELINE_INFO.get(&id).map(|p| p.category)
}

/// Check if pipeline has UI
pub fn pipeline_has_ui(id: PipelineID) -> bool {
    PIPELINE_INFO.get(&id).map(|p| p.has_ui).unwrap_or(false)
}

/// Check if pipeline is a main tab
pub fn pipeline_is_tab(id: PipelineID) -> bool {
    PIPELINE_INFO.get(&id).map(|p| p.is_tab).unwrap_or(false)
}

/// Get all pipeline IDs
pub fn get_all_pipeline_ids() -> Vec<PipelineID> {
    PIPELINE_INFO.keys().copied().collect()
}

/// Get all core pipeline IDs
pub fn get_core_pipeline_ids() -> Vec<PipelineID> {
    PIPELINE_INFO
        .iter()
        .filter(|(_, p)| p.category == "core")
        .map(|(id, _)| *id)
        .collect()
}

/// Get all consciousness pipeline IDs
pub fn get_consciousness_pipeline_ids() -> Vec<PipelineID> {
    PIPELINE_INFO
        .iter()
        .filter(|(_, p)| p.category == "consciousness")
        .map(|(id, _)| *id)
        .collect()
}

/// Get all tab pipeline IDs (workspace, tasks, library, settings)
pub fn get_tab_pipeline_ids() -> Vec<PipelineID> {
    PIPELINE_INFO
        .iter()
        .filter(|(_, p)| p.is_tab)
        .map(|(id, _)| *id)
        .collect()
}

/// Get all pipelines with UIs
pub fn get_pipelines_with_ui() -> Vec<PipelineID> {
    PIPELINE_INFO
        .iter()
        .filter(|(_, p)| p.has_ui)
        .map(|(id, _)| *id)
        .collect()
}

/// Load pipeline blueprint from file
pub fn load_blueprint_from_file(path: &Path) -> OzoneResult<PipelineBlueprint> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        crate::OzoneError::PipelineError(format!("Failed to read blueprint: {}", e))
    })?;

    // Try TOML first, then JSON
    if path.extension().map(|e| e == "toml").unwrap_or(false) {
        toml::from_str(&content).map_err(|e| {
            crate::OzoneError::PipelineError(format!("Failed to parse TOML blueprint: {}", e))
        })
    } else {
        serde_json::from_str(&content).map_err(|e| {
            crate::OzoneError::PipelineError(format!("Failed to parse JSON blueprint: {}", e))
        })
    }
}

/// Save pipeline blueprint to file
pub fn save_blueprint_to_file(blueprint: &PipelineBlueprint, path: &Path) -> OzoneResult<()> {
    let content = if path.extension().map(|e| e == "toml").unwrap_or(false) {
        toml::to_string_pretty(blueprint).map_err(|e| {
            crate::OzoneError::PipelineError(format!("Failed to serialize TOML: {}", e))
        })?
    } else {
        serde_json::to_string_pretty(blueprint).map_err(|e| {
            crate::OzoneError::PipelineError(format!("Failed to serialize JSON: {}", e))
        })?
    };

    std::fs::write(path, content).map_err(|e| {
        crate::OzoneError::PipelineError(format!("Failed to write blueprint: {}", e))
    })?;

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
        let entry = entry.map_err(|e| {
            crate::OzoneError::PipelineError(format!("Failed to read entry: {}", e))
        })?;
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
        return Err(crate::OzoneError::ValidationError(
            "Pipeline name is required".into(),
        ));
    }

    if blueprint.description.is_empty() {
        warnings.push("Pipeline description is empty".into());
    }

    // Check for circular dependencies
    if blueprint
        .specification
        .dependencies
        .contains(&blueprint.pipeline_id)
    {
        return Err(crate::OzoneError::ValidationError(
            "Pipeline cannot depend on itself".into(),
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
