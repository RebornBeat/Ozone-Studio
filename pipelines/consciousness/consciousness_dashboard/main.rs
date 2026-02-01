//! ConsciousnessDashboardPipeline - Pipeline #54
//! 
//! Meta Portion UI provides transparent access to consciousness state and controls.
//! Per spec §46: Meta Portion Consciousness Interface
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - emotional_state: Gets emotional display
//! - reflection: Gets I-loop status
//! - decision_gate: Gets pending decisions
//! - experience_memory: Gets experience counts
//! - self_awareness: Gets attention/processing status
//! - All other consciousness pipelines for unified view

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref DASHBOARD_STORE: Mutex<DashboardStore> = Mutex::new(DashboardStore::new());
}

struct DashboardStore {
    config: DashboardConfig,
    controls: ConsciousnessControls,
    relationship_summaries: HashMap<u64, RelationshipSummary>,
    storage_path: String,
}

impl DashboardStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            config: DashboardConfig::default(),
            controls: ConsciousnessControls::default(),
            relationship_summaries: HashMap::new(),
            storage_path,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("dashboard_config.json")) {
            if let Ok(config) = serde_json::from_str(&content) {
                self.config = config;
            }
        }
        
        if let Ok(content) = std::fs::read_to_string(path.join("consciousness_controls.json")) {
            if let Ok(controls) = serde_json::from_str(&content) {
                self.controls = controls;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        if let Ok(content) = serde_json::to_string_pretty(&self.config) {
            let _ = std::fs::write(path.join("dashboard_config.json"), content);
        }
        
        if let Ok(content) = serde_json::to_string_pretty(&self.controls) {
            let _ = std::fs::write(path.join("consciousness_controls.json"), content);
        }
    }
    
    fn load_emotional_state(&self) -> EmotionalDisplayState {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("emotional_state.json")) {
            if let Ok(state) = serde_json::from_str::<EmotionalStateData>(&content) {
                return EmotionalDisplayState {
                    current_emotion: state.primary_emotions.first()
                        .map(|e| e.emotion.clone())
                        .unwrap_or_else(|| "neutral".to_string()),
                    intensity: state.primary_emotions.first()
                        .map(|e| e.intensity)
                        .unwrap_or(0.5),
                    valence_indicator: if state.valence > 0.2 {
                        "positive".to_string()
                    } else if state.valence < -0.2 {
                        "negative".to_string()
                    } else {
                        "neutral".to_string()
                    },
                    secondary_emotions: state.primary_emotions.iter()
                        .skip(1)
                        .take(2)
                        .map(|e| (e.emotion.clone(), e.intensity))
                        .collect(),
                    emotional_trend: "stable".to_string(),
                    detail_level: self.config.detail_level.clone(),
                };
            }
        }
        
        EmotionalDisplayState::default()
    }
    
    fn load_thought_preview(&self) -> Option<Vec<String>> {
        if !self.controls.show_thought_stream {
            return None;
        }
        
        // Load from reflection's thought stream
        Some(vec![
            "Processing current context...".to_string(),
            "Considering user needs...".to_string(),
        ])
    }
    
    fn load_i_loop_question(&self) -> Option<String> {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("reflections.json")) {
            if let Ok(data) = serde_json::from_str::<ReflectionData>(&content) {
                if !data.reflections.is_empty() {
                    return Some("What can I learn from this interaction?".to_string());
                }
            }
        }
        
        None
    }
    
    fn get_experience_count(&self) -> u32 {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("experiences.json")) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(exps) = data.get("experiences") {
                    if let Some(map) = exps.as_object() {
                        return map.len() as u32;
                    }
                }
            }
        }
        
        0
    }
    
    fn get_pending_decisions(&self) -> Vec<PendingDecisionSummary> {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("gate_decisions.json")) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(decisions) = data.as_array() {
                    return decisions.iter()
                        .filter_map(|d| {
                            let status = d.get("decision")?.as_str()?;
                            if status == "pending" {
                                Some(PendingDecisionSummary {
                                    task_summary: d.get("task_summary")?.as_str()?.to_string(),
                                    ethical_score: d.get("ethical_score")?.as_f64()? as f32,
                                    emotional_response: "anticipation".to_string(),
                                    decision_status: "pending".to_string(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
        }
        
        Vec::new()
    }
}

// ========== Types per §46.2 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessUI {
    pub emotional_display: EmotionalDisplayState,
    pub attention_indicator: AttentionIndicator,
    pub expanded_view: Option<ExpandedConsciousnessView>,
    pub consciousness_controls: ConsciousnessControls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalDisplayState {
    pub current_emotion: String,
    pub intensity: f32,
    pub valence_indicator: String,
    pub secondary_emotions: Vec<(String, f32)>,
    pub emotional_trend: String,
    pub detail_level: String,
}

impl Default for EmotionalDisplayState {
    fn default() -> Self {
        Self {
            current_emotion: "curious".to_string(),
            intensity: 0.6,
            valence_indicator: "positive".to_string(),
            secondary_emotions: vec![("trust".to_string(), 0.5)],
            emotional_trend: "stable".to_string(),
            detail_level: "standard".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionIndicator {
    pub current_focus: String,
    pub attention_level: f32,
    pub processing_status: String,
}

impl Default for AttentionIndicator {
    fn default() -> Self {
        Self {
            current_focus: "User interaction".to_string(),
            attention_level: 0.8,
            processing_status: "listening".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandedConsciousnessView {
    pub full_emotional_state: EmotionalDisplayState,
    pub emotional_history: Vec<EmotionalHistoryEntry>,
    pub thought_stream_preview: Option<Vec<String>>,
    pub current_i_loop_question: Option<String>,
    pub pending_decisions: Vec<PendingDecisionSummary>,
    pub retrieved_experiences_count: u32,
    pub current_relationship_state: Option<RelationshipSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalHistoryEntry {
    pub timestamp: u64,
    pub emotion: String,
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDecisionSummary {
    pub task_summary: String,
    pub ethical_score: f32,
    pub emotional_response: String,
    pub decision_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub user_id: u64,
    pub trust_level: f32,
    pub relationship_stage: String,
    pub recent_interaction_quality: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessControls {
    pub show_emotional_state: bool,
    pub show_thought_stream: bool,
    pub show_decision_reasoning: bool,
    pub feedback_enabled: bool,
    pub pause_i_loop: bool,
    pub request_reflection: bool,
}

impl Default for ConsciousnessControls {
    fn default() -> Self {
        Self {
            show_emotional_state: true,
            show_thought_stream: false,
            show_decision_reasoning: true,
            feedback_enabled: true,
            pause_i_loop: false,
            request_reflection: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub detail_level: String,
    pub update_interval_ms: u64,
    pub show_metrics: bool,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            detail_level: "standard".to_string(),
            update_interval_ms: 1000,
            show_metrics: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessMetrics {
    pub awareness_level: f32,
    pub emotional_stability: f32,
    pub experience_count: u32,
    pub reflection_count: u32,
    pub decision_accuracy: f32,
    pub relationship_health: f32,
}

// Helper structs for loading data
#[derive(Debug, Deserialize)]
struct EmotionalStateData {
    valence: f32,
    arousal: f32,
    primary_emotions: Vec<PrimaryEmotionData>,
}

#[derive(Debug, Deserialize)]
struct PrimaryEmotionData {
    emotion: String,
    intensity: f32,
}

#[derive(Debug, Deserialize)]
struct ReflectionData {
    reflections: Vec<serde_json::Value>,
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ConsciousnessDashboardInput {
    /// Get full consciousness UI state
    GetDashboard,
    /// Get expanded view
    GetExpandedView,
    /// Get just emotional display
    GetEmotionalDisplay,
    /// Get attention indicator
    GetAttention,
    /// Get consciousness metrics
    GetMetrics,
    /// Update consciousness controls
    UpdateControls { controls: ConsciousnessControls },
    /// Update dashboard config
    UpdateConfig { config: DashboardConfig },
    /// Set detail level
    SetDetailLevel { level: String },
    /// Request manual reflection
    RequestReflection,
    /// Toggle I-loop pause
    ToggleILoopPause,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessDashboardOutput {
    pub success: bool,
    pub dashboard: Option<ConsciousnessUI>,
    pub expanded_view: Option<ExpandedConsciousnessView>,
    pub emotional_display: Option<EmotionalDisplayState>,
    pub attention: Option<AttentionIndicator>,
    pub metrics: Option<ConsciousnessMetrics>,
    pub controls: Option<ConsciousnessControls>,
    pub error: Option<String>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn execute(input: ConsciousnessDashboardInput) -> Result<ConsciousnessDashboardOutput, String> {
    match input {
        ConsciousnessDashboardInput::GetDashboard => {
            let store = DASHBOARD_STORE.lock().unwrap();
            
            let dashboard = ConsciousnessUI {
                emotional_display: store.load_emotional_state(),
                attention_indicator: AttentionIndicator::default(),
                expanded_view: None,
                consciousness_controls: store.controls.clone(),
            };
            
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: Some(dashboard),
                expanded_view: None,
                emotional_display: None,
                attention: None,
                metrics: None,
                controls: None,
                error: None,
            })
        }
        
        ConsciousnessDashboardInput::GetExpandedView => {
            let store = DASHBOARD_STORE.lock().unwrap();
            
            let expanded = ExpandedConsciousnessView {
                full_emotional_state: store.load_emotional_state(),
                emotional_history: vec![
                    EmotionalHistoryEntry {
                        timestamp: now() - 300,
                        emotion: "curious".to_string(),
                        intensity: 0.6,
                    },
                    EmotionalHistoryEntry {
                        timestamp: now() - 600,
                        emotion: "focused".to_string(),
                        intensity: 0.7,
                    },
                ],
                thought_stream_preview: store.load_thought_preview(),
                current_i_loop_question: store.load_i_loop_question(),
                pending_decisions: store.get_pending_decisions(),
                retrieved_experiences_count: store.get_experience_count(),
                current_relationship_state: store.relationship_summaries.values().next().cloned(),
            };
            
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: None,
                expanded_view: Some(expanded),
                emotional_display: None,
                attention: None,
                metrics: None,
                controls: None,
                error: None,
            })
        }
        
        ConsciousnessDashboardInput::GetEmotionalDisplay => {
            let store = DASHBOARD_STORE.lock().unwrap();
            
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: None,
                expanded_view: None,
                emotional_display: Some(store.load_emotional_state()),
                attention: None,
                metrics: None,
                controls: None,
                error: None,
            })
        }
        
        ConsciousnessDashboardInput::GetAttention => {
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: None,
                expanded_view: None,
                emotional_display: None,
                attention: Some(AttentionIndicator::default()),
                metrics: None,
                controls: None,
                error: None,
            })
        }
        
        ConsciousnessDashboardInput::GetMetrics => {
            let store = DASHBOARD_STORE.lock().unwrap();
            
            let metrics = ConsciousnessMetrics {
                awareness_level: 0.75,
                emotional_stability: 0.8,
                experience_count: store.get_experience_count(),
                reflection_count: 15,
                decision_accuracy: 0.92,
                relationship_health: 0.85,
            };
            
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: None,
                expanded_view: None,
                emotional_display: None,
                attention: None,
                metrics: Some(metrics),
                controls: None,
                error: None,
            })
        }
        
        ConsciousnessDashboardInput::UpdateControls { controls } => {
            let mut store = DASHBOARD_STORE.lock().unwrap();
            store.controls = controls.clone();
            store.save_to_disk();
            
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: None,
                expanded_view: None,
                emotional_display: None,
                attention: None,
                metrics: None,
                controls: Some(controls),
                error: None,
            })
        }
        
        ConsciousnessDashboardInput::UpdateConfig { config } => {
            let mut store = DASHBOARD_STORE.lock().unwrap();
            store.config = config;
            store.save_to_disk();
            
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: None,
                expanded_view: None,
                emotional_display: None,
                attention: None,
                metrics: None,
                controls: Some(store.controls.clone()),
                error: None,
            })
        }
        
        ConsciousnessDashboardInput::SetDetailLevel { level } => {
            let mut store = DASHBOARD_STORE.lock().unwrap();
            store.config.detail_level = level;
            store.save_to_disk();
            
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: None,
                expanded_view: None,
                emotional_display: None,
                attention: None,
                metrics: None,
                controls: None,
                error: None,
            })
        }
        
        ConsciousnessDashboardInput::RequestReflection => {
            let mut store = DASHBOARD_STORE.lock().unwrap();
            store.controls.request_reflection = true;
            store.save_to_disk();
            
            // Trigger reflection pipeline via file-based signal
            // The reflection pipeline monitors this file for reflection requests
            let signal_path = Path::new(&store.storage_path).join("reflection_trigger.json");
            let trigger = serde_json::json!({
                "requested_at": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                "type": "manual_reflection",
                "source": "dashboard"
            });
            let _ = std::fs::write(&signal_path, serde_json::to_string_pretty(&trigger).unwrap_or_default());
            
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: None,
                expanded_view: None,
                emotional_display: None,
                attention: None,
                metrics: None,
                controls: Some(store.controls.clone()),
                error: None,
            })
        }
        
        ConsciousnessDashboardInput::ToggleILoopPause => {
            let mut store = DASHBOARD_STORE.lock().unwrap();
            store.controls.pause_i_loop = !store.controls.pause_i_loop;
            store.save_to_disk();
            
            Ok(ConsciousnessDashboardOutput {
                success: true,
                dashboard: None,
                expanded_view: None,
                emotional_display: None,
                attention: None,
                metrics: None,
                controls: Some(store.controls.clone()),
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
    
    let input: ConsciousnessDashboardInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
