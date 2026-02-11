//! ThemeLoaderPipeline - Pipeline #2
//! 
//! Loads and manages UI themes. Themes define what appears in the ThemeArea.
//! The home_dashboard theme is the default boot theme.
//! 
//! NEW IN v0.4.0: Dynamic Pipeline UI Loading
//! ============================================
//! Pipeline UIs can be loaded at runtime without rebuilding the frontend!
//! 
//! How to add a new pipeline UI (NO REBUILD NEEDED):
//! 1. Create pipelines/core/{pipeline_name}/ui/schema.json
//! 2. Define the UI using JSON schema (form, list, detail, etc.)
//! 3. The frontend will automatically load and render it
//! 
//! Example schema.json:
//! {
//!   "type": "form",
//!   "pipelineId": 30,
//!   "title": "Link File",
//!   "fields": [
//!     {"name": "file_path", "type": "file", "label": "File Path", "required": true}
//!   ],
//!   "submitAction": "Link",
//!   "submitLabel": "Link File"
//! }
//! 
//! Themes can be:
//! - Built-in (shipped with Ozone Studio)
//! - Custom (created by users via PipelineCreationPipeline)
//! - Network (downloaded from the distributed network)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Pipeline input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ThemeLoaderInput {
    /// Load a theme by name
    Load { theme_name: String },
    /// List available themes
    List,
    /// Get current theme info
    Current,
    /// Unload current theme (return to home)
    Unload,
    /// Refresh theme (reload without full unload)
    Refresh,
    /// Get theme metadata
    GetMetadata { theme_name: String },
    /// Get pipeline UI schema for dynamic loading (NO REBUILD!)
    GetPipelineUI { pipeline_id: u64 },
    /// Get pipeline UI JS component (v0.4.0 - loads component.js)
    GetPipelineUIComponent { pipeline_id: u64 },
    /// Inject a tab into the current theme
    InjectTab { pipeline_id: u64, label: Option<String>, icon: Option<String>, make_active: bool },
    /// Remove an injected tab
    UninjectTab { tab_id: String },
    /// List pipelines that have UI schemas
    ListPipelineUIs,
    /// Get full pipeline registry (v0.4.0 - central source of truth)
    GetPipelineRegistry,
}

/// Theme metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub is_builtin: bool,
    pub entry_component: String,
    pub required_pipelines: Vec<u64>,
    pub tabs: Vec<ThemeTab>,
    pub interaction_handlers: Vec<String>,
}

/// Theme tab definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeTab {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub pipeline_id: Option<u64>,
}

/// Pipeline UI schema (loaded from ui/schema.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineUISchema {
    #[serde(rename = "type")]
    pub ui_type: String,
    #[serde(rename = "pipelineId")]
    pub pipeline_id: u64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub fields: Option<Vec<serde_json::Value>>,
    #[serde(rename = "submitAction")]
    pub submit_action: Option<String>,
    #[serde(rename = "submitLabel")]
    pub submit_label: Option<String>,
    pub actions: Option<Vec<serde_json::Value>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Pipeline UI info (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineUIInfo {
    pub pipeline_id: u64,
    pub pipeline_name: String,
    pub ui_type: String,
    pub title: String,
}

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeLoaderOutput {
    pub success: bool,
    pub theme: Option<ThemeMetadata>,
    pub available_themes: Option<Vec<String>>,
    pub schema: Option<PipelineUISchema>,
    pub pipeline_uis: Option<Vec<PipelineUIInfo>>,
    pub component_js: Option<String>,  // v0.4.0: Raw JS content for component
    pub registry: Option<Vec<serde_json::Value>>,  // v0.4.0: Full pipeline registry
    pub error: Option<String>,
}

fn get_pipelines_path() -> String {
    std::env::var("OZONE_PIPELINES_PATH").unwrap_or_else(|_| "./pipelines".to_string())
}

/// Get pipeline folder name and category from ID
/// ALIGNED WITH src/pipeline/registry.rs - THE SOURCE OF TRUTH
fn get_pipeline_info(id: u64) -> (&'static str, &'static str) {
    // Returns (category, folder_name)
    match id {
        // Core System Pipelines (1-38)
        1 => ("core", "auth"),
        2 => ("core", "theme_loader"),
        3 => ("core", "zsei_query"),
        4 => ("core", "zsei_write"),
        5 => ("core", "task_manager"),
        6 => ("core", "workspace_tab"),
        7 => ("core", "library_tab"),
        8 => ("core", "settings_tab"),
        9 => ("core", "prompt"),
        10 => ("core", "voice"),
        11 => ("core", "methodology_fetch"),
        12 => ("core", "methodology_create"),
        13 => ("core", "blueprint_search"),
        14 => ("core", "blueprint_create"),
        15 => ("core", "pipeline_creation"),
        16 => ("core", "zero_shot_simulation"),
        17 => ("core", "traversal_ml"),
        18 => ("core", "code_analysis"),
        19 => ("core", "package_context"),
        20 => ("core", "text_analysis"),
        21 => ("core", "context_aggregation"),
        22 => ("core", "graph_visualization"),
        23 => ("core", "task_recommendation"),
        24 => ("core", "reordering"),
        25 => ("core", "browser_navigation"),
        26 => ("core", "integrity_check"),
        27 => ("core", "consensus"),
        28 => ("core", "external_reference"),
        29 => ("core", "package_relationship"),
        30 => ("core", "file_link"),
        31 => ("core", "url_link"),
        32 => ("core", "package_link"),
        33 => ("core", "sync"),
        34 => ("core", "device_register"),
        35 => ("core", "home_return"),
        36 => ("core", "task_viewer"),  // DEPRECATED - merged into task_manager
        37 => ("core", "log_viewer"),
        38 => ("core", "device_status"),
        // Consciousness Pipelines (39-54)
        39 => ("consciousness", "consciousness_decision_gate"),
        40 => ("consciousness", "experience_categorization"),
        41 => ("consciousness", "core_memory_formation"),
        42 => ("consciousness", "experience_retrieval"),
        43 => ("consciousness", "emotional_baseline_update"),
        44 => ("consciousness", "i_loop"),
        45 => ("consciousness", "internal_language"),
        46 => ("consciousness", "narrative_construction"),
        47 => ("consciousness", "relationship_development"),
        48 => ("consciousness", "ethical_assessment"),
        49 => ("consciousness", "ethical_simulation"),
        50 => ("consciousness", "playback_review"),
        51 => ("consciousness", "user_feedback"),
        52 => ("consciousness", "collective_consciousness"),
        53 => ("consciousness", "voice_identity"),
        54 => ("consciousness", "meta_portion_consciousness"),
        _ => ("custom", "unknown"),
    }
}

/// Legacy function for backward compatibility
fn get_pipeline_folder_name(id: u64) -> &'static str {
    get_pipeline_info(id).1
}

fn load_pipeline_ui_schema(pipeline_id: u64) -> Option<PipelineUISchema> {
    let pipelines_path = get_pipelines_path();
    let (category, folder_name) = get_pipeline_info(pipeline_id);
    
    let possible_paths = [
        format!("{}/{}/{}/ui/schema.json", pipelines_path, category, folder_name),
        format!("./pipelines/{}/{}/ui/schema.json", category, folder_name),
    ];
    
    for path in &possible_paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(schema) = serde_json::from_str::<PipelineUISchema>(&content) {
                return Some(schema);
            }
        }
    }
    None
}

/// Load pipeline UI component.js (v0.4.0 - dynamic JS loading)
fn load_pipeline_ui_component(pipeline_id: u64) -> Option<String> {
    let pipelines_path = get_pipelines_path();
    let (category, folder_name) = get_pipeline_info(pipeline_id);
    
    let possible_paths = [
        format!("{}/{}/{}/ui/component.js", pipelines_path, category, folder_name),
        format!("./pipelines/{}/{}/ui/component.js", category, folder_name),
    ];
    
    for path in &possible_paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return Some(content);
        }
    }
    None
}

/// Check if pipeline has a UI (either schema.json or component.js)
fn pipeline_has_ui(pipeline_id: u64) -> bool {
    load_pipeline_ui_schema(pipeline_id).is_some() || load_pipeline_ui_component(pipeline_id).is_some()
}

fn list_pipeline_uis() -> Vec<PipelineUIInfo> {
    let mut results = Vec::new();
    for id in 1..=54 {
        let (_, folder_name) = get_pipeline_info(id);
        
        // Check for schema.json first
        if let Some(s) = load_pipeline_ui_schema(id) {
            results.push(PipelineUIInfo {
                pipeline_id: id,
                pipeline_name: folder_name.to_string(),
                ui_type: s.ui_type,
                title: s.title.unwrap_or_else(|| folder_name.to_string()),
            });
        } 
        // Then check for component.js
        else if load_pipeline_ui_component(id).is_some() {
            results.push(PipelineUIInfo {
                pipeline_id: id,
                pipeline_name: folder_name.to_string(),
                ui_type: "component".to_string(),
                title: folder_name.to_string(),
            });
        }
    }
    results
}

/// Build full pipeline registry for frontend (v0.4.0)
fn build_pipeline_registry() -> Vec<serde_json::Value> {
    let mut registry = Vec::new();
    
    // Core tabs that should always be injected on startup
    let core_tabs: Vec<u64> = vec![6, 5, 7, 8]; // workspace, task_manager, library, settings
    
    for id in 1..=54 {
        let (category, folder_name) = get_pipeline_info(id);
        let has_ui = pipeline_has_ui(id);
        let is_tab = core_tabs.contains(&id);
        
        registry.push(serde_json::json!({
            "id": id,
            "name": folder_name,
            "folder_name": folder_name,
            "category": category,
            "has_ui": has_ui,
            "is_tab": is_tab,
        }));
    }
    
    registry
}

fn get_builtin_themes() -> HashMap<String, ThemeMetadata> {
    let mut themes = HashMap::new();
    
    // home_dashboard uses TaskManager (#5), not TaskViewer (#36) which is deprecated
    themes.insert("home_dashboard".to_string(), ThemeMetadata {
        name: "home_dashboard".to_string(),
        display_name: "Home Dashboard".to_string(),
        description: "The default Ozone Studio home screen".to_string(),
        version: "0.4.0".to_string(),
        author: "Ozone Studio".to_string(),
        is_builtin: true,
        entry_component: "HomeDashboard".to_string(),
        required_pipelines: vec![5, 6, 7, 8],  // task_manager, workspace, library, settings
        tabs: vec![
            ThemeTab { id: "workspace".into(), name: "Workspace".into(), icon: Some("📁".into()), pipeline_id: Some(6) },
            ThemeTab { id: "tasks".into(), name: "Tasks".into(), icon: Some("📋".into()), pipeline_id: Some(5) },  // TaskManager
            ThemeTab { id: "library".into(), name: "Library".into(), icon: Some("📚".into()), pipeline_id: Some(7) },
            ThemeTab { id: "settings".into(), name: "Settings".into(), icon: Some("⚙️".into()), pipeline_id: Some(8) },
        ],
        interaction_handlers: vec!["onTaskSelect".into(), "onNewTask".into(), "onTabInject".into()],
    });
    
    themes.insert("code_editor".to_string(), ThemeMetadata {
        name: "code_editor".to_string(),
        display_name: "Code Editor".to_string(),
        description: "Code editing with ZSEI integration".to_string(),
        version: "0.4.0".to_string(),
        author: "Ozone Studio".to_string(),
        is_builtin: true,
        entry_component: "CodeEditor".to_string(),
        required_pipelines: vec![18, 19, 30],
        tabs: vec![
            ThemeTab { id: "files".into(), name: "Files".into(), icon: Some("📄".into()), pipeline_id: None },
            ThemeTab { id: "analysis".into(), name: "Analysis".into(), icon: Some("🔍".into()), pipeline_id: Some(18) },
        ],
        interaction_handlers: vec!["onFileOpen".into(), "onFileSave".into()],
    });
    
    themes
}

pub async fn execute(input: ThemeLoaderInput) -> Result<ThemeLoaderOutput, String> {
    let themes = get_builtin_themes();
    
    match input {
        ThemeLoaderInput::Load { theme_name } => {
            if let Some(theme) = themes.get(&theme_name) {
                Ok(ThemeLoaderOutput { success: true, theme: Some(theme.clone()), available_themes: None, schema: None, pipeline_uis: None, component_js: None, registry: None, error: None })
            } else {
                Err(format!("Theme '{}' not found", theme_name))
            }
        }
        ThemeLoaderInput::List => {
            let available: Vec<String> = themes.keys().cloned().collect();
            Ok(ThemeLoaderOutput { success: true, theme: None, available_themes: Some(available), schema: None, pipeline_uis: None, component_js: None, registry: None, error: None })
        }
        ThemeLoaderInput::Current | ThemeLoaderInput::Unload | ThemeLoaderInput::Refresh => {
            if let Some(theme) = themes.get("home_dashboard") {
                Ok(ThemeLoaderOutput { success: true, theme: Some(theme.clone()), available_themes: None, schema: None, pipeline_uis: None, component_js: None, registry: None, error: None })
            } else {
                Err("home_dashboard not found".into())
            }
        }
        ThemeLoaderInput::GetMetadata { theme_name } => {
            if let Some(theme) = themes.get(&theme_name) {
                Ok(ThemeLoaderOutput { success: true, theme: Some(theme.clone()), available_themes: None, schema: None, pipeline_uis: None, component_js: None, registry: None, error: None })
            } else {
                Err(format!("Theme '{}' not found", theme_name))
            }
        }
        ThemeLoaderInput::GetPipelineUI { pipeline_id } => {
            match load_pipeline_ui_schema(pipeline_id) {
                Some(schema) => Ok(ThemeLoaderOutput { success: true, theme: None, available_themes: None, schema: Some(schema), pipeline_uis: None, component_js: None, registry: None, error: None }),
                None => Ok(ThemeLoaderOutput { success: false, theme: None, available_themes: None, schema: None, pipeline_uis: None, component_js: None, registry: None, error: Some(format!("No UI schema for pipeline {}", pipeline_id)) }),
            }
        }
        ThemeLoaderInput::GetPipelineUIComponent { pipeline_id } => {
            // v0.4.0: Load component.js for dynamic UI
            match load_pipeline_ui_component(pipeline_id) {
                Some(js_content) => Ok(ThemeLoaderOutput { success: true, theme: None, available_themes: None, schema: None, pipeline_uis: None, component_js: Some(js_content), registry: None, error: None }),
                None => Ok(ThemeLoaderOutput { success: false, theme: None, available_themes: None, schema: None, pipeline_uis: None, component_js: None, registry: None, error: Some(format!("No UI component for pipeline {}", pipeline_id)) }),
            }
        }
        ThemeLoaderInput::ListPipelineUIs => {
            let uis = list_pipeline_uis();
            Ok(ThemeLoaderOutput { success: true, theme: None, available_themes: None, schema: None, pipeline_uis: Some(uis), component_js: None, registry: None, error: None })
        }
        ThemeLoaderInput::GetPipelineRegistry => {
            // v0.4.0: Return full pipeline registry for frontend
            let registry = build_pipeline_registry();
            Ok(ThemeLoaderOutput { success: true, theme: None, available_themes: None, schema: None, pipeline_uis: None, component_js: None, registry: Some(registry), error: None })
        }
        ThemeLoaderInput::InjectTab { .. } | ThemeLoaderInput::UninjectTab { .. } => {
            // Tab injection is handled by frontend via IPC
            // This just acknowledges the request
            Ok(ThemeLoaderOutput { success: true, theme: None, available_themes: None, schema: None, pipeline_uis: None, component_js: None, registry: None, error: None })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: ThemeLoaderInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
