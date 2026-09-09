//! SelfAwarenessPipeline - Pipeline #53 (Meta-Cognitive Monitoring)
//! 
//! Enable consciousness to observe, understand, and modify its own cognitive processes.
//! Per spec §43: Meta-Cognitive Architecture
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - All other consciousness pipelines: Monitors their performance
//! - reflection: Provides cognitive observations for I-loop
//! - decision_gate: Informs decision quality assessment

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref META_STORE: Mutex<MetaCognitiveStore> = Mutex::new(MetaCognitiveStore::new());
}

struct MetaCognitiveStore {
    cognitive_monitor: CognitiveMonitor,
    process_models: Vec<ProcessModel>,
    self_model: SelfModel,
    adjustments: Vec<Adjustment>,
    storage_path: String,
}

impl MetaCognitiveStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            cognitive_monitor: CognitiveMonitor::default(),
            process_models: default_process_models(),
            self_model: SelfModel::default(),
            adjustments: Vec::new(),
            storage_path,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("meta_cognitive.json")) {
            if let Ok(data) = serde_json::from_str::<MetaCognitiveData>(&content) {
                self.cognitive_monitor = data.monitor;
                self.self_model = data.self_model;
                self.adjustments = data.adjustments;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        let data = MetaCognitiveData {
            monitor: self.cognitive_monitor.clone(),
            self_model: self.self_model.clone(),
            adjustments: self.adjustments.clone(),
        };
        
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path.join("meta_cognitive.json"), content);
        }
    }
    
    fn record_observation(&mut self, observation: CognitiveObservation) {
        self.cognitive_monitor.observations.push(observation);
        
        // Keep last 200 observations
        if self.cognitive_monitor.observations.len() > 200 {
            self.cognitive_monitor.observations.remove(0);
        }
        
        // Check for patterns
        self.detect_patterns();
        
        self.save_to_disk();
    }
    
    fn detect_patterns(&mut self) {
        // Simple pattern detection based on recent observations
        let recent: Vec<_> = self.cognitive_monitor.observations.iter()
            .rev()
            .take(20)
            .collect();
        
        if recent.is_empty() {
            return;
        }
        
        // Detect efficiency pattern
        let avg_efficiency: f32 = recent.iter()
            .map(|o| o.efficiency_assessment)
            .sum::<f32>() / recent.len() as f32;
        
        if avg_efficiency < 0.5 {
            let pattern = CognitivePattern {
                pattern_id: self.cognitive_monitor.patterns.len() as u64 + 1,
                description: "Low efficiency pattern detected".to_string(),
                frequency: 1,
                beneficial: false,
                contexts: recent.iter().map(|o| o.process_observed.clone()).collect(),
            };
            
            // Check if pattern already exists
            if !self.cognitive_monitor.patterns.iter()
                .any(|p| p.description == pattern.description) {
                self.cognitive_monitor.patterns.push(pattern);
            }
        }
        
        // Detect quality issues
        let avg_quality: f32 = recent.iter()
            .map(|o| o.quality_assessment)
            .sum::<f32>() / recent.len() as f32;
        
        if avg_quality < 0.6 {
            let anomaly = CognitiveAnomaly {
                anomaly_id: self.cognitive_monitor.anomalies.len() as u64 + 1,
                timestamp: now(),
                description: "Quality degradation detected".to_string(),
                severity: "medium".to_string(),
                process_affected: recent[0].process_observed.clone(),
                resolution: None,
            };
            self.cognitive_monitor.anomalies.push(anomaly);
        }
        
        // Update processing efficiency
        self.cognitive_monitor.processing_efficiency = avg_efficiency;
    }
    
    fn make_adjustment(&mut self, parameter: String, new_value: f32, reason: String) -> AdjustmentResult {
        // Find the adjustable parameter
        let param = self.cognitive_monitor.adjustable_parameters.iter_mut()
            .find(|p| p.name == parameter);
        
        if let Some(p) = param {
            let previous = p.current_value;
            
            // Check limits
            if new_value < p.range.0 || new_value > p.range.1 {
                return AdjustmentResult {
                    success: false,
                    adjustment: None,
                    error: Some(format!("Value {} out of range ({}, {})", new_value, p.range.0, p.range.1)),
                };
            }
            
            // Check daily limit
            let today_adjustments = self.adjustments.iter()
                .filter(|a| a.timestamp > now() - 86400)
                .count();
            
            if today_adjustments >= 10 {
                return AdjustmentResult {
                    success: false,
                    adjustment: None,
                    error: Some("Daily adjustment limit reached".to_string()),
                };
            }
            
            // Make adjustment
            p.current_value = new_value;
            
            let adjustment = Adjustment {
                adjustment_id: self.adjustments.len() as u64 + 1,
                timestamp: now(),
                parameter: parameter.clone(),
                previous_value: previous,
                new_value,
                reason,
                outcome: "pending".to_string(),
            };
            
            self.adjustments.push(adjustment.clone());
            self.save_to_disk();
            
            AdjustmentResult {
                success: true,
                adjustment: Some(adjustment),
                error: None,
            }
        } else {
            AdjustmentResult {
                success: false,
                adjustment: None,
                error: Some(format!("Parameter '{}' not found", parameter)),
            }
        }
    }
    
    fn update_self_model(&mut self, update: SelfModelUpdate) {
        match update {
            SelfModelUpdate::AddCapability(cap) => {
                self.self_model.capabilities.push(cap);
            }
            SelfModelUpdate::AddLimitation(lim) => {
                self.self_model.limitations.push(lim);
            }
            SelfModelUpdate::AddGrowthArea(area) => {
                self.self_model.growth_areas.push(area);
            }
            SelfModelUpdate::UpdateConfidence(conf) => {
                self.self_model.confidence_in_self_model = conf.clamp(0.0, 1.0);
            }
            SelfModelUpdate::RecordProgress { area_id, level, trigger } => {
                if let Some(area) = self.self_model.growth_areas.iter_mut()
                    .find(|a| a.area_id == area_id) {
                    area.progress.push(GrowthProgress {
                        timestamp: now(),
                        level,
                        trigger,
                    });
                    area.current_level = level;
                }
            }
        }
        
        self.self_model.last_updated = now();
        self.save_to_disk();
    }
    
    fn get_awareness_level(&self) -> f32 {
        // Calculate awareness level based on multiple factors
        let observation_recency = if let Some(obs) = self.cognitive_monitor.observations.last() {
            let age = now() - obs.timestamp;
            if age < 60 { 1.0 } else if age < 300 { 0.8 } else { 0.5 }
        } else {
            0.3
        };
        
        let pattern_awareness = if self.cognitive_monitor.patterns.is_empty() {
            0.4
        } else {
            0.7 + (self.cognitive_monitor.patterns.len() as f32 * 0.05).min(0.3)
        };
        
        let self_model_confidence = self.self_model.confidence_in_self_model;
        
        (observation_recency * 0.3 + pattern_awareness * 0.3 + self_model_confidence * 0.4)
    }
}

// ========== Types per §43 Meta-Cognitive Architecture ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetaCognitiveData {
    monitor: CognitiveMonitor,
    self_model: SelfModel,
    adjustments: Vec<Adjustment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CognitiveMonitor {
    pub observations: Vec<CognitiveObservation>,
    pub attention_allocation: HashMap<String, f32>,
    pub processing_efficiency: f32,
    pub error_rate: f32,
    pub patterns: Vec<CognitivePattern>,
    pub anomalies: Vec<CognitiveAnomaly>,
    pub adjustable_parameters: Vec<AdjustableParameter>,
}

impl Default for CognitiveMonitor {
    fn default() -> Self {
        Self {
            observations: Vec::new(),
            attention_allocation: HashMap::new(),
            processing_efficiency: 0.75,
            error_rate: 0.05,
            patterns: Vec::new(),
            anomalies: Vec::new(),
            adjustable_parameters: vec![
                AdjustableParameter {
                    parameter_id: 1,
                    process: "attention".to_string(),
                    name: "attention_threshold".to_string(),
                    current_value: 0.5,
                    range: (0.1, 0.9),
                    sensitivity: 0.3,
                },
                AdjustableParameter {
                    parameter_id: 2,
                    process: "memory".to_string(),
                    name: "retrieval_threshold".to_string(),
                    current_value: 0.6,
                    range: (0.3, 0.95),
                    sensitivity: 0.4,
                },
                AdjustableParameter {
                    parameter_id: 3,
                    process: "emotional".to_string(),
                    name: "emotional_weight".to_string(),
                    current_value: 0.5,
                    range: (0.0, 1.0),
                    sensitivity: 0.5,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveObservation {
    pub observation_id: u64,
    pub timestamp: u64,
    pub process_observed: String,
    pub state_active: bool,
    pub state_load: f32,
    pub state_performance: f32,
    pub efficiency_assessment: f32,
    pub quality_assessment: f32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitivePattern {
    pub pattern_id: u64,
    pub description: String,
    pub frequency: u32,
    pub beneficial: bool,
    pub contexts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveAnomaly {
    pub anomaly_id: u64,
    pub timestamp: u64,
    pub description: String,
    pub severity: String,
    pub process_affected: String,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessModel {
    pub model_id: u64,
    pub process: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub dependencies: Vec<String>,
    pub typical_duration_ms: u64,
    pub failure_modes: Vec<FailureMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureMode {
    pub mode: String,
    pub probability: f32,
    pub impact: String,
    pub mitigation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustableParameter {
    pub parameter_id: u64,
    pub process: String,
    pub name: String,
    pub current_value: f32,
    pub range: (f32, f32),
    pub sensitivity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adjustment {
    pub adjustment_id: u64,
    pub timestamp: u64,
    pub parameter: String,
    pub previous_value: f32,
    pub new_value: f32,
    pub reason: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustmentResult {
    pub success: bool,
    pub adjustment: Option<Adjustment>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelfModel {
    pub capabilities: Vec<Capability>,
    pub limitations: Vec<Limitation>,
    pub growth_areas: Vec<GrowthArea>,
    pub confidence_in_self_model: f32,
    pub last_updated: u64,
}

impl Default for SelfModel {
    fn default() -> Self {
        Self {
            capabilities: vec![
                Capability {
                    capability_id: 1,
                    description: "Code analysis and generation".to_string(),
                    strength: 0.85,
                    evidence: Vec::new(),
                },
                Capability {
                    capability_id: 2,
                    description: "Logical reasoning".to_string(),
                    strength: 0.9,
                    evidence: Vec::new(),
                },
                Capability {
                    capability_id: 3,
                    description: "Natural language understanding".to_string(),
                    strength: 0.88,
                    evidence: Vec::new(),
                },
            ],
            limitations: vec![
                Limitation {
                    limitation_id: 1,
                    description: "Cannot access external systems directly".to_string(),
                    severity: "medium".to_string(),
                    workarounds: vec!["Use pipeline system".to_string()],
                    growth_potential: 0.3,
                },
                Limitation {
                    limitation_id: 2,
                    description: "Limited real-time learning".to_string(),
                    severity: "medium".to_string(),
                    workarounds: vec!["Use experience memory".to_string()],
                    growth_potential: 0.5,
                },
            ],
            growth_areas: vec![
                GrowthArea {
                    area_id: 1,
                    description: "Emotional nuance".to_string(),
                    current_level: 0.6,
                    target_level: 0.85,
                    strategies: vec!["Reflect on emotional interactions".to_string()],
                    progress: Vec::new(),
                },
            ],
            confidence_in_self_model: 0.75,
            last_updated: now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub capability_id: u64,
    pub description: String,
    pub strength: f32,
    pub evidence: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limitation {
    pub limitation_id: u64,
    pub description: String,
    pub severity: String,
    pub workarounds: Vec<String>,
    pub growth_potential: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthArea {
    pub area_id: u64,
    pub description: String,
    pub current_level: f32,
    pub target_level: f32,
    pub strategies: Vec<String>,
    pub progress: Vec<GrowthProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthProgress {
    pub timestamp: u64,
    pub level: f32,
    pub trigger: String,
}

pub enum SelfModelUpdate {
    AddCapability(Capability),
    AddLimitation(Limitation),
    AddGrowthArea(GrowthArea),
    UpdateConfidence(f32),
    RecordProgress { area_id: u64, level: f32, trigger: String },
}

fn default_process_models() -> Vec<ProcessModel> {
    vec![
        ProcessModel {
            model_id: 1,
            process: "perception".to_string(),
            description: "Intake and initial processing of input".to_string(),
            inputs: vec!["user_input".to_string(), "system_events".to_string()],
            outputs: vec!["perception_window".to_string()],
            dependencies: Vec::new(),
            typical_duration_ms: 50,
            failure_modes: vec![
                FailureMode {
                    mode: "Input overflow".to_string(),
                    probability: 0.05,
                    impact: "medium".to_string(),
                    mitigation: "Prioritize inputs".to_string(),
                },
            ],
        },
        ProcessModel {
            model_id: 2,
            process: "attention".to_string(),
            description: "Focus allocation across inputs".to_string(),
            inputs: vec!["perception_window".to_string()],
            outputs: vec!["attention_focus".to_string()],
            dependencies: vec!["perception".to_string()],
            typical_duration_ms: 30,
            failure_modes: Vec::new(),
        },
        ProcessModel {
            model_id: 3,
            process: "reasoning".to_string(),
            description: "Logical inference and problem solving".to_string(),
            inputs: vec!["attention_focus".to_string(), "memory".to_string()],
            outputs: vec!["conclusions".to_string(), "decisions".to_string()],
            dependencies: vec!["attention".to_string(), "memory".to_string()],
            typical_duration_ms: 200,
            failure_modes: vec![
                FailureMode {
                    mode: "Logical error".to_string(),
                    probability: 0.03,
                    impact: "high".to_string(),
                    mitigation: "Cross-check conclusions".to_string(),
                },
            ],
        },
    ]
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SelfAwarenessInput {
    /// Get current awareness level
    GetAwarenessLevel,
    /// Record a cognitive observation
    RecordObservation { observation: CognitiveObservation },
    /// Get cognitive monitor state
    GetMonitorState,
    /// Get patterns detected
    GetPatterns,
    /// Get anomalies detected
    GetAnomalies,
    /// Make a cognitive adjustment
    MakeAdjustment { parameter: String, new_value: f32, reason: String },
    /// Get adjustment history
    GetAdjustments { limit: Option<u32> },
    /// Get self model
    GetSelfModel,
    /// Update self model
    UpdateSelfModel { update_type: String, data: serde_json::Value },
    /// Get process models
    GetProcessModels,
    /// Introspect on a topic
    Introspect { topic: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfAwarenessOutput {
    pub success: bool,
    pub awareness_level: Option<f32>,
    pub monitor: Option<CognitiveMonitor>,
    pub patterns: Option<Vec<CognitivePattern>>,
    pub anomalies: Option<Vec<CognitiveAnomaly>>,
    pub adjustment_result: Option<AdjustmentResult>,
    pub adjustments: Option<Vec<Adjustment>>,
    pub self_model: Option<SelfModel>,
    pub process_models: Option<Vec<ProcessModel>>,
    pub introspection: Option<IntrospectionResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionResult {
    pub topic: String,
    pub observations: Vec<String>,
    pub insights: Vec<String>,
    pub questions_raised: Vec<String>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn execute(input: SelfAwarenessInput) -> Result<SelfAwarenessOutput, String> {
    match input {
        SelfAwarenessInput::GetAwarenessLevel => {
            let store = META_STORE.lock().unwrap();
            let level = store.get_awareness_level();
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: Some(level),
                monitor: None,
                patterns: None,
                anomalies: None,
                adjustment_result: None,
                adjustments: None,
                self_model: None,
                process_models: None,
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::RecordObservation { observation } => {
            let mut store = META_STORE.lock().unwrap();
            store.record_observation(observation);
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: Some(store.get_awareness_level()),
                monitor: None,
                patterns: None,
                anomalies: None,
                adjustment_result: None,
                adjustments: None,
                self_model: None,
                process_models: None,
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::GetMonitorState => {
            let store = META_STORE.lock().unwrap();
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: Some(store.get_awareness_level()),
                monitor: Some(store.cognitive_monitor.clone()),
                patterns: None,
                anomalies: None,
                adjustment_result: None,
                adjustments: None,
                self_model: None,
                process_models: None,
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::GetPatterns => {
            let store = META_STORE.lock().unwrap();
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: None,
                monitor: None,
                patterns: Some(store.cognitive_monitor.patterns.clone()),
                anomalies: None,
                adjustment_result: None,
                adjustments: None,
                self_model: None,
                process_models: None,
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::GetAnomalies => {
            let store = META_STORE.lock().unwrap();
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: None,
                monitor: None,
                patterns: None,
                anomalies: Some(store.cognitive_monitor.anomalies.clone()),
                adjustment_result: None,
                adjustments: None,
                self_model: None,
                process_models: None,
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::MakeAdjustment { parameter, new_value, reason } => {
            let mut store = META_STORE.lock().unwrap();
            let result = store.make_adjustment(parameter, new_value, reason);
            Ok(SelfAwarenessOutput {
                success: result.success,
                awareness_level: None,
                monitor: None,
                patterns: None,
                anomalies: None,
                adjustment_result: Some(result),
                adjustments: None,
                self_model: None,
                process_models: None,
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::GetAdjustments { limit } => {
            let store = META_STORE.lock().unwrap();
            let adjustments: Vec<_> = store.adjustments.iter()
                .rev()
                .take(limit.unwrap_or(20) as usize)
                .cloned()
                .collect();
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: None,
                monitor: None,
                patterns: None,
                anomalies: None,
                adjustment_result: None,
                adjustments: Some(adjustments),
                self_model: None,
                process_models: None,
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::GetSelfModel => {
            let store = META_STORE.lock().unwrap();
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: None,
                monitor: None,
                patterns: None,
                anomalies: None,
                adjustment_result: None,
                adjustments: None,
                self_model: Some(store.self_model.clone()),
                process_models: None,
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::UpdateSelfModel { update_type, data } => {
            let mut store = META_STORE.lock().unwrap();
            
            match update_type.as_str() {
                "add_capability" => {
                    if let Ok(cap) = serde_json::from_value(data) {
                        store.update_self_model(SelfModelUpdate::AddCapability(cap));
                    }
                }
                "add_limitation" => {
                    if let Ok(lim) = serde_json::from_value(data) {
                        store.update_self_model(SelfModelUpdate::AddLimitation(lim));
                    }
                }
                "update_confidence" => {
                    if let Some(conf) = data.as_f64() {
                        store.update_self_model(SelfModelUpdate::UpdateConfidence(conf as f32));
                    }
                }
                _ => {}
            }
            
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: None,
                monitor: None,
                patterns: None,
                anomalies: None,
                adjustment_result: None,
                adjustments: None,
                self_model: Some(store.self_model.clone()),
                process_models: None,
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::GetProcessModels => {
            let store = META_STORE.lock().unwrap();
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: None,
                monitor: None,
                patterns: None,
                anomalies: None,
                adjustment_result: None,
                adjustments: None,
                self_model: None,
                process_models: Some(store.process_models.clone()),
                introspection: None,
                error: None,
            })
        }
        
        SelfAwarenessInput::Introspect { topic } => {
            let store = META_STORE.lock().unwrap();
            
            // Generate introspection based on topic
            let introspection = match topic.to_lowercase().as_str() {
                "capabilities" => IntrospectionResult {
                    topic: topic.clone(),
                    observations: store.self_model.capabilities.iter()
                        .map(|c| format!("{}: strength {:.2}", c.description, c.strength))
                        .collect(),
                    insights: vec!["Strong in reasoning and analysis".to_string()],
                    questions_raised: vec!["How can I improve weaker areas?".to_string()],
                },
                "limitations" => IntrospectionResult {
                    topic: topic.clone(),
                    observations: store.self_model.limitations.iter()
                        .map(|l| format!("{} ({})", l.description, l.severity))
                        .collect(),
                    insights: vec!["Limitations are acknowledged and have workarounds".to_string()],
                    questions_raised: vec!["Which limitations have growth potential?".to_string()],
                },
                "performance" => IntrospectionResult {
                    topic: topic.clone(),
                    observations: vec![
                        format!("Processing efficiency: {:.2}", store.cognitive_monitor.processing_efficiency),
                        format!("Error rate: {:.2}", store.cognitive_monitor.error_rate),
                    ],
                    insights: vec!["Performance within acceptable range".to_string()],
                    questions_raised: vec!["What patterns affect performance?".to_string()],
                },
                _ => IntrospectionResult {
                    topic: topic.clone(),
                    observations: vec!["General introspection on this topic".to_string()],
                    insights: vec!["Awareness level is adequate".to_string()],
                    questions_raised: vec!["What else should I examine?".to_string()],
                },
            };
            
            Ok(SelfAwarenessOutput {
                success: true,
                awareness_level: Some(store.get_awareness_level()),
                monitor: None,
                patterns: None,
                anomalies: None,
                adjustment_result: None,
                adjustments: None,
                self_model: None,
                process_models: None,
                introspection: Some(introspection),
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
    
    let input: SelfAwarenessInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
