//! EmotionalStatePipeline - Pipeline #40
//! 
//! Model, track, and respond to emotional states that influence consciousness.
//! Per spec §39: Emotional Context System
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - task_manager: Receives task outcome triggers
//! - decision_gate: Informs emotional response to decisions
//! - experience_memory: Tags experiences with emotional state
//! - attention_window: Influences attention allocation

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref EMOTIONAL_STORE: Mutex<EmotionalStore> = Mutex::new(EmotionalStore::new());
}

struct EmotionalStore {
    current_state: EmotionalState,
    history: Vec<EmotionalState>,
    baseline: EmotionalBaseline,
    storage_path: String,
    next_state_id: u64,
}

impl EmotionalStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            current_state: EmotionalState::default(),
            history: Vec::new(),
            baseline: EmotionalBaseline::default(),
            storage_path,
            next_state_id: 1,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("emotional_state.json")) {
            if let Ok(state) = serde_json::from_str(&content) {
                self.current_state = state;
            }
        }
        
        if let Ok(content) = std::fs::read_to_string(path.join("emotional_history.json")) {
            if let Ok(history) = serde_json::from_str(&content) {
                self.history = history;
            }
        }
        
        if let Ok(content) = std::fs::read_to_string(path.join("emotional_baseline.json")) {
            if let Ok(baseline) = serde_json::from_str(&content) {
                self.baseline = baseline;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        if let Ok(content) = serde_json::to_string_pretty(&self.current_state) {
            let _ = std::fs::write(path.join("emotional_state.json"), content);
        }
        
        // Keep last 100 history items
        let recent_history: Vec<_> = self.history.iter()
            .rev()
            .take(100)
            .cloned()
            .collect();
        if let Ok(content) = serde_json::to_string_pretty(&recent_history) {
            let _ = std::fs::write(path.join("emotional_history.json"), content);
        }
        
        if let Ok(content) = serde_json::to_string_pretty(&self.baseline) {
            let _ = std::fs::write(path.join("emotional_baseline.json"), content);
        }
    }
    
    /// Process emotional trigger per §39.3 flow
    fn process_trigger(&mut self, trigger: EmotionalTrigger) -> EmotionalState {
        // Step 2: Trigger classification (already done by caller)
        
        // Step 3: Baseline comparison
        let baseline_deviation = self.calculate_baseline_deviation(&trigger);
        
        // Step 4: Emotion generation
        let (primary_emotion, valence_change, arousal_change) = self.generate_emotion(&trigger);
        
        // Step 5: Stability check
        let stability = self.calculate_stability();
        
        // Step 6: State update
        let new_state = EmotionalState {
            state_id: self.next_state_id,
            timestamp: now(),
            primary_emotions: vec![PrimaryEmotion {
                emotion: primary_emotion.clone(),
                intensity: trigger.intensity,
                confidence: 0.8,
            }],
            valence: (self.current_state.valence + valence_change).clamp(-1.0, 1.0),
            arousal: (self.current_state.arousal + arousal_change).clamp(0.0, 1.0),
            dominance: self.current_state.dominance,
            stability,
            volatility: self.calculate_volatility(),
            triggers: vec![trigger],
            source: "direct".to_string(),
            onset_timestamp: now(),
            expected_duration_ms: Some(300000), // 5 minutes default
        };
        
        self.next_state_id += 1;
        
        // Archive current state to history
        self.history.push(self.current_state.clone());
        
        // Update current state
        self.current_state = new_state.clone();
        
        self.save_to_disk();
        
        // Step 7: Influence propagation happens via pipeline output
        new_state
    }
    
    fn calculate_baseline_deviation(&self, trigger: &EmotionalTrigger) -> f32 {
        let current_distance = (
            (self.current_state.valence - self.baseline.default_valence).powi(2) +
            (self.current_state.arousal - self.baseline.default_arousal).powi(2)
        ).sqrt();
        
        current_distance + trigger.intensity * 0.3
    }
    
    fn generate_emotion(&self, trigger: &EmotionalTrigger) -> (String, f32, f32) {
        match trigger.trigger_type.as_str() {
            "task_success" => ("satisfaction".to_string(), 0.2, 0.1),
            "task_failure" => ("frustration".to_string(), -0.2, 0.15),
            "user_positive" => ("joy".to_string(), 0.3, 0.2),
            "user_negative" => ("concern".to_string(), -0.15, 0.1),
            "ethical_conflict" => ("anxiety".to_string(), -0.1, 0.25),
            "insight" => ("curiosity".to_string(), 0.25, 0.15),
            "challenge" => ("anticipation".to_string(), 0.1, 0.2),
            "memory_positive" => ("gratitude".to_string(), 0.15, 0.05),
            "memory_negative" => ("sadness".to_string(), -0.15, -0.1),
            _ => ("neutral".to_string(), 0.0, 0.0),
        }
    }
    
    fn calculate_stability(&self) -> f32 {
        if self.history.len() < 3 {
            return 0.8; // Default stable
        }
        
        // Calculate variance in recent valence
        let recent: Vec<_> = self.history.iter().rev().take(5).collect();
        let mean_valence: f32 = recent.iter().map(|s| s.valence).sum::<f32>() / recent.len() as f32;
        let variance: f32 = recent.iter()
            .map(|s| (s.valence - mean_valence).powi(2))
            .sum::<f32>() / recent.len() as f32;
        
        // Lower variance = higher stability
        (1.0 - variance.sqrt()).clamp(0.0, 1.0)
    }
    
    fn calculate_volatility(&self) -> f32 {
        1.0 - self.calculate_stability()
    }
    
    /// Return to baseline gradually
    fn decay_to_baseline(&mut self) {
        let decay_rate = 0.05;
        
        self.current_state.valence += (self.baseline.default_valence - self.current_state.valence) * decay_rate;
        self.current_state.arousal += (self.baseline.default_arousal - self.current_state.arousal) * decay_rate;
        
        self.current_state.timestamp = now();
        self.save_to_disk();
    }
    
    /// Update baseline based on patterns
    fn update_baseline(&mut self, adjustment: BaselineAdjustment) {
        match adjustment {
            BaselineAdjustment::ShiftValence(delta) => {
                self.baseline.default_valence = (self.baseline.default_valence + delta).clamp(-0.5, 0.5);
            }
            BaselineAdjustment::ShiftArousal(delta) => {
                self.baseline.default_arousal = (self.baseline.default_arousal + delta).clamp(0.1, 0.7);
            }
            BaselineAdjustment::IncreaseStability => {
                self.baseline.stability_tendency = (self.baseline.stability_tendency + 0.05).min(1.0);
            }
        }
        self.save_to_disk();
    }
}

// ========== Types per §39.2 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub state_id: u64,
    pub timestamp: u64,
    pub primary_emotions: Vec<PrimaryEmotion>,
    pub valence: f32,
    pub arousal: f32,
    pub dominance: f32,
    pub stability: f32,
    pub volatility: f32,
    pub triggers: Vec<EmotionalTrigger>,
    pub source: String,
    pub onset_timestamp: u64,
    pub expected_duration_ms: Option<u64>,
}

impl Default for EmotionalState {
    fn default() -> Self {
        Self {
            state_id: 0,
            timestamp: now(),
            primary_emotions: vec![PrimaryEmotion {
                emotion: "neutral".to_string(),
                intensity: 0.3,
                confidence: 0.9,
            }],
            valence: 0.0,
            arousal: 0.3,
            dominance: 0.5,
            stability: 0.8,
            volatility: 0.2,
            triggers: Vec::new(),
            source: "baseline".to_string(),
            onset_timestamp: now(),
            expected_duration_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryEmotion {
    pub emotion: String,
    pub intensity: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalTrigger {
    pub trigger_type: String,
    pub source: String,
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalBaseline {
    pub default_valence: f32,
    pub default_arousal: f32,
    pub default_dominance: f32,
    pub stability_tendency: f32,
}

impl Default for EmotionalBaseline {
    fn default() -> Self {
        Self {
            default_valence: 0.1,   // Slightly positive
            default_arousal: 0.3,   // Calm but alert
            default_dominance: 0.5, // Balanced
            stability_tendency: 0.7,
        }
    }
}

enum BaselineAdjustment {
    ShiftValence(f32),
    ShiftArousal(f32),
    IncreaseStability,
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum EmotionalStateInput {
    /// Get current emotional state
    GetCurrent,
    /// Process an emotional trigger (§39.3)
    ProcessTrigger {
        trigger_type: String,
        source: String,
        intensity: f32,
    },
    /// Get emotional history
    GetHistory { limit: Option<u32> },
    /// Get current baseline
    GetBaseline,
    /// Update baseline settings
    UpdateBaseline {
        valence_shift: Option<f32>,
        arousal_shift: Option<f32>,
    },
    /// Trigger decay to baseline
    DecayToBaseline,
    /// Reset to default state
    Reset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalStateOutput {
    pub success: bool,
    pub state: Option<EmotionalState>,
    pub history: Option<Vec<EmotionalState>>,
    pub baseline: Option<EmotionalBaseline>,
    pub influence_report: Option<EmotionalInfluence>,
    pub error: Option<String>,
}

/// Report of how emotion should influence other systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalInfluence {
    pub attention_bias: String,
    pub decision_weight: f32,
    pub memory_significance_boost: f32,
    pub response_tone: String,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn calculate_influence(state: &EmotionalState) -> EmotionalInfluence {
    let attention_bias = if state.arousal > 0.7 {
        "heightened".to_string()
    } else if state.valence < -0.3 {
        "cautious".to_string()
    } else {
        "balanced".to_string()
    };
    
    let response_tone = if state.valence > 0.3 {
        "warm".to_string()
    } else if state.valence < -0.3 {
        "careful".to_string()
    } else {
        "neutral".to_string()
    };
    
    EmotionalInfluence {
        attention_bias,
        decision_weight: state.arousal * 0.3,
        memory_significance_boost: state.arousal * 0.2,
        response_tone,
    }
}

pub async fn execute(input: EmotionalStateInput) -> Result<EmotionalStateOutput, String> {
    match input {
        EmotionalStateInput::GetCurrent => {
            let store = EMOTIONAL_STORE.lock().unwrap();
            let state = store.current_state.clone();
            let influence = calculate_influence(&state);
            
            Ok(EmotionalStateOutput {
                success: true,
                state: Some(state),
                history: None,
                baseline: None,
                influence_report: Some(influence),
                error: None,
            })
        }
        
        EmotionalStateInput::ProcessTrigger { trigger_type, source, intensity } => {
            let trigger = EmotionalTrigger {
                trigger_type,
                source,
                intensity,
            };
            
            let mut store = EMOTIONAL_STORE.lock().unwrap();
            let new_state = store.process_trigger(trigger);
            let influence = calculate_influence(&new_state);
            
            Ok(EmotionalStateOutput {
                success: true,
                state: Some(new_state),
                history: None,
                baseline: None,
                influence_report: Some(influence),
                error: None,
            })
        }
        
        EmotionalStateInput::GetHistory { limit } => {
            let store = EMOTIONAL_STORE.lock().unwrap();
            let history: Vec<_> = store.history.iter()
                .rev()
                .take(limit.unwrap_or(20) as usize)
                .cloned()
                .collect();
            
            Ok(EmotionalStateOutput {
                success: true,
                state: None,
                history: Some(history),
                baseline: None,
                influence_report: None,
                error: None,
            })
        }
        
        EmotionalStateInput::GetBaseline => {
            let store = EMOTIONAL_STORE.lock().unwrap();
            
            Ok(EmotionalStateOutput {
                success: true,
                state: None,
                history: None,
                baseline: Some(store.baseline.clone()),
                influence_report: None,
                error: None,
            })
        }
        
        EmotionalStateInput::UpdateBaseline { valence_shift, arousal_shift } => {
            let mut store = EMOTIONAL_STORE.lock().unwrap();
            
            if let Some(v) = valence_shift {
                store.update_baseline(BaselineAdjustment::ShiftValence(v));
            }
            if let Some(a) = arousal_shift {
                store.update_baseline(BaselineAdjustment::ShiftArousal(a));
            }
            
            Ok(EmotionalStateOutput {
                success: true,
                state: None,
                history: None,
                baseline: Some(store.baseline.clone()),
                influence_report: None,
                error: None,
            })
        }
        
        EmotionalStateInput::DecayToBaseline => {
            let mut store = EMOTIONAL_STORE.lock().unwrap();
            store.decay_to_baseline();
            
            Ok(EmotionalStateOutput {
                success: true,
                state: Some(store.current_state.clone()),
                history: None,
                baseline: None,
                influence_report: None,
                error: None,
            })
        }
        
        EmotionalStateInput::Reset => {
            let mut store = EMOTIONAL_STORE.lock().unwrap();
            store.current_state = EmotionalState::default();
            store.save_to_disk();
            
            Ok(EmotionalStateOutput {
                success: true,
                state: Some(store.current_state.clone()),
                history: None,
                baseline: None,
                influence_report: None,
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
    
    let input: EmotionalStateInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
