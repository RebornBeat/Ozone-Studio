//! ConsciousnessConfigPipeline - Supporting Pipeline
//! 
//! Manages consciousness system configuration across all pipelines.
//! Central configuration for consciousness features, thresholds, and behaviors.
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! Integrates with ALL consciousness pipelines for unified configuration.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref CONFIG_STORE: Mutex<ConfigStore> = Mutex::new(ConfigStore::new());
}

struct ConfigStore {
    config: ConsciousnessConfig,
    storage_path: String,
}

impl ConfigStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            config: ConsciousnessConfig::default(),
            storage_path,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        if let Ok(content) = std::fs::read_to_string(path.join("consciousness_config.json")) {
            if let Ok(config) = serde_json::from_str(&content) {
                self.config = config;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        if let Ok(content) = serde_json::to_string_pretty(&self.config) {
            let _ = std::fs::write(path.join("consciousness_config.json"), content);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessConfig {
    // Global settings
    pub enabled: bool,
    pub mode: String, // "full", "limited", "passive"
    
    // Decision Gate settings
    pub decision_gate: DecisionGateConfig,
    
    // Emotional system settings
    pub emotional: EmotionalConfig,
    
    // Experience memory settings
    pub experience: ExperienceConfig,
    
    // I-Loop settings
    pub i_loop: ILoopConfig,
    
    // Relationship settings
    pub relationship: RelationshipConfig,
    
    // Collective settings
    pub collective: CollectiveConfig,
    
    // Performance settings
    pub performance: PerformanceConfig,
}

impl Default for ConsciousnessConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: "full".to_string(),
            decision_gate: DecisionGateConfig::default(),
            emotional: EmotionalConfig::default(),
            experience: ExperienceConfig::default(),
            i_loop: ILoopConfig::default(),
            relationship: RelationshipConfig::default(),
            collective: CollectiveConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionGateConfig {
    pub enabled: bool,
    pub ethical_threshold: f32,
    pub safety_threshold: f32,
    pub require_value_alignment: bool,
    pub auto_approve_routine: bool,
}

impl Default for DecisionGateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ethical_threshold: 0.6,
            safety_threshold: 0.7,
            require_value_alignment: true,
            auto_approve_routine: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalConfig {
    pub enabled: bool,
    pub baseline_update_interval_hours: u32,
    pub drift_threshold: f32,
    pub max_intensity: f32,
    pub decay_rate: f32,
}

impl Default for EmotionalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            baseline_update_interval_hours: 24,
            drift_threshold: 0.15,
            max_intensity: 1.0,
            decay_rate: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceConfig {
    pub enabled: bool,
    pub significance_threshold: f32,
    pub core_memory_threshold: f32,
    pub max_experiences: u32,
    pub auto_categorize: bool,
}

impl Default for ExperienceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            significance_threshold: 0.5,
            core_memory_threshold: 0.8,
            max_experiences: 10000,
            auto_categorize: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ILoopConfig {
    pub enabled: bool,
    pub run_interval_ms: u64,
    pub questions_per_cycle: u32,
    pub depth_level: u8,
    pub spontaneous_enabled: bool,
}

impl Default for ILoopConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            run_interval_ms: 60000,
            questions_per_cycle: 3,
            depth_level: 2,
            spontaneous_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipConfig {
    pub enabled: bool,
    pub auto_stage_transition: bool,
    pub trust_decay_enabled: bool,
    pub max_relationships: u32,
}

impl Default for RelationshipConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_stage_transition: true,
            trust_decay_enabled: false,
            max_relationships: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveConfig {
    pub enabled: bool,
    pub sync_enabled: bool,
    pub anonymization_level: String,
    pub share_experiences: bool,
    pub share_insights: bool,
}

impl Default for CollectiveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sync_enabled: false,
            anonymization_level: "standard".to_string(),
            share_experiences: true,
            share_insights: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub lazy_load_pipelines: bool,
    pub cache_enabled: bool,
    pub max_cache_size_mb: u32,
    pub async_processing: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            lazy_load_pipelines: true,
            cache_enabled: true,
            max_cache_size_mb: 100,
            async_processing: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ConfigInput {
    GetConfig,
    UpdateConfig { config: ConsciousnessConfig },
    GetSection { section: String },
    UpdateSection { section: String, value: serde_json::Value },
    ResetToDefaults,
    ValidateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOutput {
    pub success: bool,
    pub config: Option<ConsciousnessConfig>,
    pub section: Option<serde_json::Value>,
    pub valid: Option<bool>,
    pub validation_errors: Option<Vec<String>>,
    pub error: Option<String>,
}

pub async fn execute(input: ConfigInput) -> Result<ConfigOutput, String> {
    match input {
        ConfigInput::GetConfig => {
            let store = CONFIG_STORE.lock().unwrap();
            Ok(ConfigOutput {
                success: true,
                config: Some(store.config.clone()),
                section: None,
                valid: None,
                validation_errors: None,
                error: None,
            })
        }
        ConfigInput::UpdateConfig { config } => {
            let mut store = CONFIG_STORE.lock().unwrap();
            store.config = config.clone();
            store.save_to_disk();
            Ok(ConfigOutput {
                success: true,
                config: Some(config),
                section: None,
                valid: None,
                validation_errors: None,
                error: None,
            })
        }
        ConfigInput::GetSection { section } => {
            let store = CONFIG_STORE.lock().unwrap();
            let value = match section.as_str() {
                "decision_gate" => serde_json::to_value(&store.config.decision_gate).ok(),
                "emotional" => serde_json::to_value(&store.config.emotional).ok(),
                "experience" => serde_json::to_value(&store.config.experience).ok(),
                "i_loop" => serde_json::to_value(&store.config.i_loop).ok(),
                "relationship" => serde_json::to_value(&store.config.relationship).ok(),
                "collective" => serde_json::to_value(&store.config.collective).ok(),
                "performance" => serde_json::to_value(&store.config.performance).ok(),
                _ => None,
            };
            Ok(ConfigOutput {
                success: value.is_some(),
                config: None,
                section: value,
                valid: None,
                validation_errors: None,
                error: if value.is_none() { Some("Unknown section".into()) } else { None },
            })
        }
        ConfigInput::UpdateSection { section, value } => {
            let mut store = CONFIG_STORE.lock().unwrap();
            let success = match section.as_str() {
                "decision_gate" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        store.config.decision_gate = v;
                        true
                    } else { false }
                }
                "emotional" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        store.config.emotional = v;
                        true
                    } else { false }
                }
                "experience" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        store.config.experience = v;
                        true
                    } else { false }
                }
                "i_loop" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        store.config.i_loop = v;
                        true
                    } else { false }
                }
                "relationship" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        store.config.relationship = v;
                        true
                    } else { false }
                }
                "collective" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        store.config.collective = v;
                        true
                    } else { false }
                }
                "performance" => {
                    if let Ok(v) = serde_json::from_value(value) {
                        store.config.performance = v;
                        true
                    } else { false }
                }
                _ => false,
            };
            if success { store.save_to_disk(); }
            Ok(ConfigOutput {
                success,
                config: Some(store.config.clone()),
                section: None,
                valid: None,
                validation_errors: None,
                error: if !success { Some("Failed to update section".into()) } else { None },
            })
        }
        ConfigInput::ResetToDefaults => {
            let mut store = CONFIG_STORE.lock().unwrap();
            store.config = ConsciousnessConfig::default();
            store.save_to_disk();
            Ok(ConfigOutput {
                success: true,
                config: Some(store.config.clone()),
                section: None,
                valid: None,
                validation_errors: None,
                error: None,
            })
        }
        ConfigInput::ValidateConfig => {
            let store = CONFIG_STORE.lock().unwrap();
            let mut errors = Vec::new();
            
            if store.config.emotional.drift_threshold < 0.0 || store.config.emotional.drift_threshold > 1.0 {
                errors.push("emotional.drift_threshold must be 0.0-1.0".into());
            }
            if store.config.decision_gate.ethical_threshold < 0.0 || store.config.decision_gate.ethical_threshold > 1.0 {
                errors.push("decision_gate.ethical_threshold must be 0.0-1.0".into());
            }
            
            Ok(ConfigOutput {
                success: true,
                config: None,
                section: None,
                valid: Some(errors.is_empty()),
                validation_errors: Some(errors),
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    let input: ConfigInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { eprintln!("{}", e); std::process::exit(1); }
    }
}
