//! SettingsTabPipeline - Pipeline #8
//! 
//! Manages all system configuration including:
//! - Model selection (API/GGUF/ONNX)
//! - UI preferences
//! - Network settings
//! - Pipeline-specific settings
//! - Consciousness settings (if enabled)
//! 
//! This is how users configure model selection - NOT hardcoded in core.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pipeline input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SettingsInput {
    /// Get all settings
    GetAll,
    /// Get settings for a specific category
    GetCategory { category: String },
    /// Update settings
    Update { updates: HashMap<String, serde_json::Value> },
    /// Reset to defaults
    ResetDefaults { category: Option<String> },
    /// Get available models
    GetAvailableModels,
    /// Set active model
    SetActiveModel { model: ModelSelection },
    /// Add custom model
    AddCustomModel { model: AvailableModel },
    /// Remove custom model
    RemoveCustomModel { identifier: String },
    /// Export settings
    Export,
    /// Import settings
    Import { settings_json: String },
}

/// Model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    pub model_type: String,  // "api", "gguf", "onnx"
    pub identifier: String,  // Model name or path
}

/// Available model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableModel {
    pub name: String,
    pub model_type: String,
    pub identifier: String,
    pub description: Option<String>,
    pub context_length: Option<usize>,
    pub is_local: bool,
}

/// All settings organized by category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllSettings {
    pub models: ModelSettings,
    pub ui: UISettings,
    pub network: NetworkSettings,
    pub pipelines: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consciousness: Option<ConsciousnessSettings>,
}

/// Model settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    pub active_model_type: String,
    pub active_model_identifier: String,
    pub api_endpoint: Option<String>,
    pub api_key_env: String,
    pub local_model_path: Option<String>,
    pub context_length: usize,
    pub gpu_layers: Option<u32>,
    pub allow_user_selection: bool,
    pub available_models: Vec<AvailableModel>,
}

/// UI settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UISettings {
    pub theme: String,
    pub meta_portion_width_percent: u8,
    pub show_task_recommendations: bool,
    pub show_connection_bar: bool,
    pub default_tabs: Vec<String>,
}

/// Network settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub enabled: bool,
    pub listen_port: u16,
    pub max_peers: u32,
    pub bootstrap_nodes: Vec<String>,
}

/// Consciousness settings (if feature enabled)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessSettings {
    pub enabled: bool,
    pub emotional_system_enabled: bool,
    pub experience_memory_enabled: bool,
    pub show_emotional_state: bool,
    pub show_decision_reasoning: bool,
    pub playback_enabled: bool,
}

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsOutput {
    pub success: bool,
    pub settings: Option<AllSettings>,
    pub category_settings: Option<serde_json::Value>,
    pub available_models: Option<Vec<AvailableModel>>,
    pub exported_json: Option<String>,
    pub error: Option<String>,
}

/// Get default settings
fn default_settings() -> AllSettings {
    AllSettings {
        models: ModelSettings {
            active_model_type: "api".to_string(),
            active_model_identifier: "claude-sonnet-4-20250514".to_string(),
            api_endpoint: Some("https://api.anthropic.com/v1/messages".to_string()),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            local_model_path: None,
            context_length: 100000,
            gpu_layers: None,
            allow_user_selection: true,
            available_models: vec![
                AvailableModel {
                    name: "Claude Sonnet".to_string(),
                    model_type: "api".to_string(),
                    identifier: "claude-sonnet-4-20250514".to_string(),
                    description: Some("Anthropic's Claude Sonnet model".to_string()),
                    context_length: Some(200000),
                    is_local: false,
                },
                AvailableModel {
                    name: "Claude Opus".to_string(),
                    model_type: "api".to_string(),
                    identifier: "claude-opus-4-20250514".to_string(),
                    description: Some("Anthropic's most capable model".to_string()),
                    context_length: Some(200000),
                    is_local: false,
                },
                AvailableModel {
                    name: "GPT-4".to_string(),
                    model_type: "api".to_string(),
                    identifier: "gpt-4-turbo".to_string(),
                    description: Some("OpenAI's GPT-4 Turbo".to_string()),
                    context_length: Some(128000),
                    is_local: false,
                },
                AvailableModel {
                    name: "BitNet b1.58 2B4T".to_string(),
                    model_type: "gguf".to_string(),
                    identifier: "bitnet-b158-2b4t.gguf".to_string(),
                    description: Some("Efficient local model with 1-bit weights".to_string()),
                    context_length: Some(4096),
                    is_local: true,
                },
            ],
        },
        ui: UISettings {
            theme: "home_dashboard".to_string(),
            meta_portion_width_percent: 20,
            show_task_recommendations: true,
            show_connection_bar: true,
            default_tabs: vec![
                "workspace".to_string(),
                "library".to_string(),
                "settings".to_string(),
            ],
        },
        network: NetworkSettings {
            enabled: false,
            listen_port: 9000,
            max_peers: 50,
            bootstrap_nodes: vec![],
        },
        pipelines: HashMap::new(),
        consciousness: None,
    }
}

/// Execute the settings pipeline
pub async fn execute(input: SettingsInput) -> Result<SettingsOutput, String> {
    // In a real implementation, this would read/write to config file
    let settings = default_settings();
    
    match input {
        SettingsInput::GetAll => {
            Ok(SettingsOutput {
                success: true,
                settings: Some(settings),
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::GetCategory { category } => {
            let value = match category.as_str() {
                "models" => serde_json::to_value(&settings.models).ok(),
                "ui" => serde_json::to_value(&settings.ui).ok(),
                "network" => serde_json::to_value(&settings.network).ok(),
                "consciousness" => serde_json::to_value(&settings.consciousness).ok(),
                _ => settings.pipelines.get(&category).cloned(),
            };
            
            Ok(SettingsOutput {
                success: value.is_some(),
                settings: None,
                category_settings: value,
                available_models: None,
                exported_json: None,
                error: if value.is_none() { Some(format!("Category '{}' not found", category)) } else { None },
            })
        }
        
        SettingsInput::Update { updates } => {
            // In real implementation, apply updates to config file
            Ok(SettingsOutput {
                success: true,
                settings: Some(settings), // Would be updated settings
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::ResetDefaults { category } => {
            Ok(SettingsOutput {
                success: true,
                settings: Some(default_settings()),
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::GetAvailableModels => {
            Ok(SettingsOutput {
                success: true,
                settings: None,
                category_settings: None,
                available_models: Some(settings.models.available_models),
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::SetActiveModel { model } => {
            // In real implementation, update config file
            Ok(SettingsOutput {
                success: true,
                settings: None,
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::AddCustomModel { model } => {
            // In real implementation, add to available models
            Ok(SettingsOutput {
                success: true,
                settings: None,
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::RemoveCustomModel { identifier } => {
            // In real implementation, remove from available models
            Ok(SettingsOutput {
                success: true,
                settings: None,
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::Export => {
            let json = serde_json::to_string_pretty(&settings)
                .map_err(|e| format!("Failed to export: {}", e))?;
            
            Ok(SettingsOutput {
                success: true,
                settings: None,
                category_settings: None,
                available_models: None,
                exported_json: Some(json),
                error: None,
            })
        }
        
        SettingsInput::Import { settings_json } => {
            let _imported: AllSettings = serde_json::from_str(&settings_json)
                .map_err(|e| format!("Failed to parse settings: {}", e))?;
            
            // In real implementation, save imported settings
            Ok(SettingsOutput {
                success: true,
                settings: None,
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
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
    
    let input: SettingsInput = match serde_json::from_str(&input_json) {
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
            let output = SettingsOutput {
                success: false,
                settings: None,
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: Some(e),
            };
            println!("{}", serde_json::to_string(&output).unwrap());
            std::process::exit(1);
        }
    }
}
