//! ThemeLoaderPipeline - Pipeline #2
//! 
//! Loads and manages UI themes. Themes define what appears in the ThemeArea.
//! The home_dashboard theme is the default boot theme.
//! 
//! Themes can be:
//! - Built-in (shipped with Ozone Studio)
//! - Custom (created by users via PipelineCreationPipeline)
//! - Network (downloaded from the distributed network)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pipeline input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ThemeLoaderInput {
    /// Load a theme by name
    Load {
        theme_name: String,
    },
    /// List available themes
    List,
    /// Get current theme info
    Current,
    /// Unload current theme (return to home)
    Unload,
    /// Refresh theme (reload without full unload)
    Refresh,
    /// Get theme metadata
    GetMetadata {
        theme_name: String,
    },
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

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeLoaderOutput {
    pub success: bool,
    pub theme: Option<ThemeMetadata>,
    pub available_themes: Option<Vec<String>>,
    pub error: Option<String>,
}

/// Built-in themes registry
fn get_builtin_themes() -> HashMap<String, ThemeMetadata> {
    let mut themes = HashMap::new();
    
    // Home Dashboard - the default boot theme
    themes.insert("home_dashboard".to_string(), ThemeMetadata {
        name: "home_dashboard".to_string(),
        display_name: "Home Dashboard".to_string(),
        description: "The default Ozone Studio home screen with task overview and quick actions".to_string(),
        version: "0.3.0".to_string(),
        author: "Ozone Studio".to_string(),
        is_builtin: true,
        entry_component: "HomeDashboard".to_string(),
        required_pipelines: vec![
            5,  // TaskManagerPipeline
            23, // TaskRecommendationPipeline
            35, // HomeReturnPipeline
        ],
        tabs: vec![
            ThemeTab {
                id: "workspace".to_string(),
                name: "Workspace".to_string(),
                icon: Some("folder".to_string()),
                pipeline_id: Some(6), // WorkspaceTabPipeline
            },
            ThemeTab {
                id: "library".to_string(),
                name: "Library".to_string(),
                icon: Some("library".to_string()),
                pipeline_id: Some(7), // LibraryTabPipeline
            },
            ThemeTab {
                id: "settings".to_string(),
                name: "Settings".to_string(),
                icon: Some("settings".to_string()),
                pipeline_id: Some(8), // SettingsTabPipeline
            },
        ],
        interaction_handlers: vec![
            "onTaskSelect".to_string(),
            "onNewTask".to_string(),
            "onQuickAction".to_string(),
        ],
    });
    
    // Code Editor theme
    themes.insert("code_editor".to_string(), ThemeMetadata {
        name: "code_editor".to_string(),
        display_name: "Code Editor".to_string(),
        description: "Full-featured code editing environment with ZSEI integration".to_string(),
        version: "0.3.0".to_string(),
        author: "Ozone Studio".to_string(),
        is_builtin: true,
        entry_component: "CodeEditor".to_string(),
        required_pipelines: vec![
            18, // CodeAnalysisPipeline
            19, // PackageContextPipeline
            30, // FileLinkPipeline
        ],
        tabs: vec![
            ThemeTab {
                id: "files".to_string(),
                name: "Files".to_string(),
                icon: Some("file".to_string()),
                pipeline_id: None,
            },
            ThemeTab {
                id: "analysis".to_string(),
                name: "Analysis".to_string(),
                icon: Some("search".to_string()),
                pipeline_id: Some(18),
            },
        ],
        interaction_handlers: vec![
            "onFileOpen".to_string(),
            "onFileSave".to_string(),
            "onAnalyze".to_string(),
        ],
    });
    
    // Graph Visualization theme
    themes.insert("graph_view".to_string(), ThemeMetadata {
        name: "graph_view".to_string(),
        display_name: "Graph View".to_string(),
        description: "Visualize ZSEI container relationships and knowledge graph".to_string(),
        version: "0.3.0".to_string(),
        author: "Ozone Studio".to_string(),
        is_builtin: true,
        entry_component: "GraphView".to_string(),
        required_pipelines: vec![
            3,  // ZSEIQueryPipeline
            22, // GraphVisualizationPipeline
        ],
        tabs: vec![
            ThemeTab {
                id: "graph".to_string(),
                name: "Graph".to_string(),
                icon: Some("share".to_string()),
                pipeline_id: Some(22),
            },
            ThemeTab {
                id: "details".to_string(),
                name: "Details".to_string(),
                icon: Some("info".to_string()),
                pipeline_id: None,
            },
        ],
        interaction_handlers: vec![
            "onNodeSelect".to_string(),
            "onZoom".to_string(),
            "onPan".to_string(),
        ],
    });
    
    themes
}

/// Execute the theme loader pipeline
pub async fn execute(input: ThemeLoaderInput) -> Result<ThemeLoaderOutput, String> {
    let themes = get_builtin_themes();
    
    match input {
        ThemeLoaderInput::Load { theme_name } => {
            if let Some(theme) = themes.get(&theme_name) {
                Ok(ThemeLoaderOutput {
                    success: true,
                    theme: Some(theme.clone()),
                    available_themes: None,
                    error: None,
                })
            } else {
                // Check custom themes directory
                let custom_themes_dir = std::env::var("OZONE_CUSTOM_THEMES_PATH")
                    .unwrap_or_else(|_| "./themes/custom".to_string());
                
                let theme_path = std::path::Path::new(&custom_themes_dir)
                    .join(format!("{}.json", theme_name));
                
                if theme_path.exists() {
                    match std::fs::read_to_string(&theme_path) {
                        Ok(content) => {
                            match serde_json::from_str::<Theme>(&content) {
                                Ok(theme) => {
                                    Ok(ThemeLoaderOutput {
                                        success: true,
                                        theme: Some(theme),
                                        available_themes: None,
                                        error: None,
                                    })
                                }
                                Err(e) => Err(format!("Invalid theme file '{}': {}", theme_name, e))
                            }
                        }
                        Err(e) => Err(format!("Failed to read theme '{}': {}", theme_name, e))
                    }
                } else {
                    Err(format!("Theme '{}' not found in built-in or custom themes", theme_name))
                }
            }
        }
        
        ThemeLoaderInput::List => {
            let mut available: Vec<String> = themes.keys().cloned().collect();
            
            // Also list custom themes
            let custom_themes_dir = std::env::var("OZONE_CUSTOM_THEMES_PATH")
                .unwrap_or_else(|_| "./themes/custom".to_string());
            
            if let Ok(entries) = std::fs::read_dir(&custom_themes_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.path().file_stem() {
                        if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                            let theme_name = name.to_string_lossy().to_string();
                            if !available.contains(&theme_name) {
                                available.push(theme_name);
                            }
                        }
                    }
                }
            }
            
            available.sort();
            
            Ok(ThemeLoaderOutput {
                success: true,
                theme: None,
                available_themes: Some(available),
                error: None,
            })
        }
        
        ThemeLoaderInput::Current => {
            // Return the default theme for now
            // In real implementation, this would check current state
            if let Some(theme) = themes.get("home_dashboard") {
                Ok(ThemeLoaderOutput {
                    success: true,
                    theme: Some(theme.clone()),
                    available_themes: None,
                    error: None,
                })
            } else {
                Err("No current theme".into())
            }
        }
        
        ThemeLoaderInput::Unload => {
            // Unload returns to home_dashboard
            if let Some(theme) = themes.get("home_dashboard") {
                Ok(ThemeLoaderOutput {
                    success: true,
                    theme: Some(theme.clone()),
                    available_themes: None,
                    error: None,
                })
            } else {
                Err("Cannot unload - home_dashboard not found".into())
            }
        }
        
        ThemeLoaderInput::Refresh => {
            // Just return current theme
            // In real implementation, this would trigger re-render
            if let Some(theme) = themes.get("home_dashboard") {
                Ok(ThemeLoaderOutput {
                    success: true,
                    theme: Some(theme.clone()),
                    available_themes: None,
                    error: None,
                })
            } else {
                Err("No theme to refresh".into())
            }
        }
        
        ThemeLoaderInput::GetMetadata { theme_name } => {
            if let Some(theme) = themes.get(&theme_name) {
                Ok(ThemeLoaderOutput {
                    success: true,
                    theme: Some(theme.clone()),
                    available_themes: None,
                    error: None,
                })
            } else {
                Err(format!("Theme '{}' not found", theme_name))
            }
        }
    }
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
    
    let input: ThemeLoaderInput = match serde_json::from_str(&input_json) {
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
            let output = ThemeLoaderOutput {
                success: false,
                theme: None,
                available_themes: None,
                error: Some(e),
            };
            println!("{}", serde_json::to_string(&output).unwrap());
            std::process::exit(1);
        }
    }
}
