//! SettingsTabPipeline - Pipeline #8
//! 
//! Manages all system configuration including:
//! - Model selection (API/GGUF/ONNX/BitNet)
//! - UI preferences
//! - Network settings
//! - Pipeline-specific settings
//! - Consciousness settings (if enabled)
//! 
//! This is how users configure model selection - NOT hardcoded in core.
//! 
//! STORAGE: Settings persist to data/config/settings.json
//! v0.4.0: Added BitNet model type support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// ============================================================================
// Storage Layer - File-based persistence
// ============================================================================

/// Get the path to the settings file
fn settings_path() -> PathBuf {
    let base = std::env::var("OZONE_DATA_PATH")
        .unwrap_or_else(|_| "./data".to_string());
    PathBuf::from(base).join("config")
}

/// Load settings from file, or return defaults if not exists
fn load_settings() -> AllSettings {
    let path = settings_path().join("settings.json");
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
    }
    default_settings()
}

/// Save settings to file
fn save_settings(settings: &AllSettings) -> Result<(), String> {
    let dir = settings_path();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    let path = dir.join("settings.json");
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write settings: {}", e))?;
    Ok(())
}

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
    pub model_type: String,  // "api", "gguf", "onnx", "bitnet"
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

/// Operator policy: paid models are never selectable. This pipeline cannot
/// read the live OpenRouter catalog, so it applies the naming rule OpenRouter
/// documents for free variants: `openrouter/free` or a `:free` suffix, on the
/// OpenRouter endpoint. Local model types are always allowed. Returns the
/// refusal text, or None when the selection is allowed.
/// The operator's per-user paid-models setting, read from the host's
/// config.toml ([models] allow_paid_models, default false). Pipeline children
/// inherit the host's working directory, so the live file is the single
/// source of truth — no input plumbing, and a /config/set change takes
/// effect on the next settings operation. Unreadable file = false (the safe
/// default), never an invented true.
fn allow_paid_models() -> bool {
    let Ok(text) = fs::read_to_string("config.toml") else { return false; };
    let mut in_models = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_models = t == "[models]";
            continue;
        }
        if in_models {
            if let Some((key, val)) = t.split_once('=') {
                if key.trim() == "allow_paid_models" {
                    return val.trim().parse().unwrap_or(false);
                }
            }
        }
    }
    false
}

fn paid_policy_refusal(model_type: &str, identifier: &str, endpoint: Option<&str>) -> Option<String> {
    const POLICY: &str = "policy: only free OpenRouter models and local BitNet may be used";
    if model_type != "api" {
        return None;
    }
    if let Some(ep) = endpoint {
        if !ep.contains("openrouter.ai") {
            return Some(format!("refused: endpoint '{ep}' is a paid direct provider; {POLICY}"));
        }
    }
    if identifier == "openrouter/free" || identifier.ends_with(":free") {
        return None;
    }
    Some(format!("refused: model '{identifier}' is not a free OpenRouter model; {POLICY}"))
}

/// Drop stored API entries that the policy forbids from the selectable list,
/// returning their identifiers so the caller can say what was removed.
fn prune_paid_available(models: &mut ModelSettings) -> Vec<String> {
    let mut removed = Vec::new();
    models.available_models.retain(|m| {
        if paid_policy_refusal(&m.model_type, &m.identifier, None).is_some() {
            removed.push(m.identifier.clone());
            false
        } else {
            true
        }
    });
    removed
}

/// Get default settings
fn default_settings() -> AllSettings {
    AllSettings {
        models: ModelSettings {
            active_model_type: "api".to_string(),
            active_model_identifier: "openrouter/free".to_string(),
            api_endpoint: Some("https://openrouter.ai/api/v1/chat/completions".to_string()),
            api_key_env: "OPENROUTER_API_KEY".to_string(),
            local_model_path: None,
            context_length: 200000,
            gpu_layers: None,
            allow_user_selection: true,
            available_models: vec![
                // API Models: free OpenRouter only (operator policy). Paid
                // direct providers (Anthropic, OpenAI) are not listed and
                // are refused by paid_policy_refusal if requested.
                AvailableModel {
                    name: "OpenRouter (Free router)".to_string(),
                    model_type: "api".to_string(),
                    identifier: "openrouter/free".to_string(),
                    description: Some("OpenRouter free router: free models only".to_string()),
                    context_length: Some(200000),
                    is_local: false,
                },
                // BitNet Models (1-bit quantized, CPU-efficient)
                AvailableModel {
                    name: "BitNet b1.58 2B4T".to_string(),
                    model_type: "bitnet".to_string(),  // Distinct type for BitNet
                    identifier: "bitnet-b1.58-2B4T".to_string(),
                    description: Some("1-bit quantized model - CPU efficient, 2B parameters".to_string()),
                    context_length: Some(4096),
                    is_local: true,
                },
                AvailableModel {
                    name: "BitNet b1.58 7B".to_string(),
                    model_type: "bitnet".to_string(),
                    identifier: "bitnet-b1.58-7B".to_string(),
                    description: Some("1-bit quantized model - CPU efficient, 7B parameters".to_string()),
                    context_length: Some(4096),
                    is_local: true,
                },
                // GGUF Models (llama.cpp compatible)
                AvailableModel {
                    name: "Llama 3 8B (GGUF)".to_string(),
                    model_type: "gguf".to_string(),
                    identifier: "llama-3-8b.gguf".to_string(),
                    description: Some("Meta's Llama 3 8B in GGUF format".to_string()),
                    context_length: Some(8192),
                    is_local: true,
                },
                AvailableModel {
                    name: "Mistral 7B (GGUF)".to_string(),
                    model_type: "gguf".to_string(),
                    identifier: "mistral-7b.gguf".to_string(),
                    description: Some("Mistral 7B in GGUF format".to_string()),
                    context_length: Some(32768),
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
                "tasks".to_string(),
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
    match input {
        SettingsInput::GetAll => {
            let settings = load_settings();
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
            let settings = load_settings();
            let value = match category.as_str() {
                "models" => serde_json::to_value(&settings.models).ok(),
                "ui" => serde_json::to_value(&settings.ui).ok(),
                "network" => serde_json::to_value(&settings.network).ok(),
                "consciousness" => serde_json::to_value(&settings.consciousness).ok(),
                _ => settings.pipelines.get(&category).cloned(),
            };
            
            // Bind the emptiness check BEFORE `category_settings: value` moves it
            // (E0382 — latent break; the crate had not been rebuilt since the
            // shared-dir migration, so no compiler ever saw it).
            let missing = value.is_none();

            Ok(SettingsOutput {
                success: !missing,
                settings: None,
                category_settings: value,
                available_models: None,
                exported_json: None,
                error: if missing { Some(format!("Category '{}' not found", category)) } else { None },
            })
        }
        
        SettingsInput::Update { updates } => {
            let mut settings = load_settings();
            
            // Apply updates to appropriate categories
            for (key, value) in updates {
                match key.as_str() {
                    // Model settings updates
                    "active_model_type" => {
                        if let Some(s) = value.as_str() {
                            settings.models.active_model_type = s.to_string();
                        }
                    }
                    "active_model_identifier" => {
                        if let Some(s) = value.as_str() {
                            settings.models.active_model_identifier = s.to_string();
                        }
                    }
                    "api_endpoint" => {
                        settings.models.api_endpoint = value.as_str().map(|s| s.to_string());
                    }
                    "api_key_env" => {
                        if let Some(s) = value.as_str() {
                            settings.models.api_key_env = s.to_string();
                        }
                    }
                    "local_model_path" => {
                        settings.models.local_model_path = value.as_str().map(|s| s.to_string());
                    }
                    "context_length" => {
                        if let Some(n) = value.as_u64() {
                            settings.models.context_length = n as usize;
                        }
                    }
                    "gpu_layers" => {
                        settings.models.gpu_layers = value.as_u64().map(|n| n as u32);
                    }
                    // UI settings updates
                    "theme" => {
                        if let Some(s) = value.as_str() {
                            settings.ui.theme = s.to_string();
                        }
                    }
                    "meta_portion_width_percent" => {
                        if let Some(n) = value.as_u64() {
                            settings.ui.meta_portion_width_percent = n as u8;
                        }
                    }
                    "show_task_recommendations" => {
                        if let Some(b) = value.as_bool() {
                            settings.ui.show_task_recommendations = b;
                        }
                    }
                    "show_connection_bar" => {
                        if let Some(b) = value.as_bool() {
                            settings.ui.show_connection_bar = b;
                        }
                    }
                    // Network settings updates
                    "network_enabled" => {
                        if let Some(b) = value.as_bool() {
                            settings.network.enabled = b;
                        }
                    }
                    "listen_port" => {
                        if let Some(n) = value.as_u64() {
                            settings.network.listen_port = n as u16;
                        }
                    }
                    "max_peers" => {
                        if let Some(n) = value.as_u64() {
                            settings.network.max_peers = n as u32;
                        }
                    }
                    // Consciousness settings updates
                    "consciousness_enabled" => {
                        if let Some(b) = value.as_bool() {
                            let mut cs = settings.consciousness.unwrap_or(ConsciousnessSettings {
                                enabled: false,
                                emotional_system_enabled: false,
                                experience_memory_enabled: false,
                                show_emotional_state: false,
                                show_decision_reasoning: false,
                                playback_enabled: false,
                            });
                            cs.enabled = b;
                            settings.consciousness = Some(cs);
                        }
                    }
                    // Pipeline-specific settings (stored as JSON)
                    other if other.starts_with("pipeline_") => {
                        let pipeline_name = other.strip_prefix("pipeline_").unwrap();
                        settings.pipelines.insert(pipeline_name.to_string(), value);
                    }
                    _ => {
                        // Unknown key - store in pipelines as catch-all
                        settings.pipelines.insert(key, value);
                    }
                }
            }
            
            // Policy: stored paid entries leave the selectable list (named
            // on stderr), and an active paid selection refuses the save —
            // unless the operator's per-user allow_paid_models flag is set.
            if !allow_paid_models() {
            for removed in prune_paid_available(&mut settings.models) {
                eprintln!("settings: removed paid entry '{removed}' from the selectable list (policy)");
            }
            if let Some(reason) = paid_policy_refusal(
                &settings.models.active_model_type,
                &settings.models.active_model_identifier,
                settings.models.api_endpoint.as_deref(),
            ) {
                return Ok(SettingsOutput {
                    success: false,
                    settings: None,
                    category_settings: None,
                    available_models: None,
                    exported_json: None,
                    error: Some(reason),
                });
            }
            } // allow_paid_models gate

            // Save to file
            save_settings(&settings)?;

            Ok(SettingsOutput {
                success: true,
                settings: Some(settings),
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::ResetDefaults { category } => {
            let mut settings = load_settings();
            let defaults = default_settings();
            
            match category.as_deref() {
                Some("models") => settings.models = defaults.models,
                Some("ui") => settings.ui = defaults.ui,
                Some("network") => settings.network = defaults.network,
                Some("consciousness") => settings.consciousness = defaults.consciousness,
                Some(cat) => {
                    settings.pipelines.remove(cat);
                }
                None => {
                    // Reset everything
                    settings = defaults;
                }
            }
            
            save_settings(&settings)?;
            
            Ok(SettingsOutput {
                success: true,
                settings: Some(settings),
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::GetAvailableModels => {
            let settings = load_settings();
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
            let mut settings = load_settings();
            
            // Validate model exists in available models
            let model_exists = settings.models.available_models.iter()
                .any(|m| m.model_type == model.model_type && m.identifier == model.identifier);
            
            if !model_exists {
                return Ok(SettingsOutput {
                    success: false,
                    settings: None,
                    category_settings: None,
                    available_models: None,
                    exported_json: None,
                    error: Some(format!("Model not found: {} ({})", model.identifier, model.model_type)),
                });
            }
            
            if !allow_paid_models() {
            if let Some(reason) = paid_policy_refusal(
                &model.model_type,
                &model.identifier,
                settings.models.api_endpoint.as_deref(),
            ) {
                return Ok(SettingsOutput {
                    success: false,
                    settings: None,
                    category_settings: None,
                    available_models: None,
                    exported_json: None,
                    error: Some(reason),
                });
            }
            } // allow_paid_models gate

            settings.models.active_model_type = model.model_type;
            settings.models.active_model_identifier = model.identifier;
            
            // Update context_length based on selected model
            if let Some(m) = settings.models.available_models.iter()
                .find(|m| m.identifier == settings.models.active_model_identifier) {
                if let Some(ctx) = m.context_length {
                    settings.models.context_length = ctx;
                }
            }
            
            save_settings(&settings)?;
            
            Ok(SettingsOutput {
                success: true,
                settings: Some(settings),
                category_settings: None,
                available_models: None,
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::AddCustomModel { model } => {
            let mut settings = load_settings();
            
            // Check for duplicates
            if settings.models.available_models.iter()
                .any(|m| m.identifier == model.identifier && m.model_type == model.model_type) {
                return Ok(SettingsOutput {
                    success: false,
                    settings: None,
                    category_settings: None,
                    available_models: None,
                    exported_json: None,
                    error: Some(format!("Model already exists: {}", model.identifier)),
                });
            }
            
            settings.models.available_models.push(model);
            save_settings(&settings)?;
            
            Ok(SettingsOutput {
                success: true,
                settings: None,
                category_settings: None,
                available_models: Some(settings.models.available_models),
                exported_json: None,
                error: None,
            })
        }
        
        SettingsInput::RemoveCustomModel { identifier } => {
            let mut settings = load_settings();
            let initial_len = settings.models.available_models.len();
            
            // Don't allow removing the active model
            if settings.models.active_model_identifier == identifier {
                return Ok(SettingsOutput {
                    success: false,
                    settings: None,
                    category_settings: None,
                    available_models: None,
                    exported_json: None,
                    error: Some("Cannot remove the active model".to_string()),
                });
            }
            
            settings.models.available_models.retain(|m| m.identifier != identifier);
            
            if settings.models.available_models.len() < initial_len {
                save_settings(&settings)?;
                Ok(SettingsOutput {
                    success: true,
                    settings: None,
                    category_settings: None,
                    available_models: Some(settings.models.available_models),
                    exported_json: None,
                    error: None,
                })
            } else {
                Ok(SettingsOutput {
                    success: false,
                    settings: None,
                    category_settings: None,
                    available_models: None,
                    exported_json: None,
                    error: Some(format!("Model not found: {}", identifier)),
                })
            }
        }
        
        SettingsInput::Export => {
            let settings = load_settings();
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
            let imported: AllSettings = serde_json::from_str(&settings_json)
                .map_err(|e| format!("Failed to parse settings: {}", e))?;
            
            // Save imported settings
            save_settings(&imported)?;
            
            Ok(SettingsOutput {
                success: true,
                settings: Some(imported),
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
