//! EmotionalCalibrationPipeline - Pipeline #43 (Emotional Baseline Update)
//! 
//! Maintain baseline emotional state representing "normal" emotional disposition.
//! Per spec §40: Emotional Baseline System
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - emotional_state: Gets current state for drift detection
//! - experience_memory: Correlates drift with experiences
//! - identity_system: Updates identity if baseline shifts significantly

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref BASELINE_STORE: Mutex<BaselineStore> = Mutex::new(BaselineStore::new());
}

struct BaselineStore {
    baseline: EmotionalBaseline,
    snapshots: Vec<BaselineSnapshot>,
    drift_alerts: Vec<DriftAlert>,
    storage_path: String,
}

impl BaselineStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            baseline: EmotionalBaseline::default(),
            snapshots: Vec::new(),
            drift_alerts: Vec::new(),
            storage_path,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("emotional_baseline.json")) {
            if let Ok(baseline) = serde_json::from_str(&content) {
                self.baseline = baseline;
            }
        }
        
        if let Ok(content) = std::fs::read_to_string(path.join("baseline_snapshots.json")) {
            if let Ok(snapshots) = serde_json::from_str(&content) {
                self.snapshots = snapshots;
            }
        }
        
        if let Ok(content) = std::fs::read_to_string(path.join("drift_alerts.json")) {
            if let Ok(alerts) = serde_json::from_str(&content) {
                self.drift_alerts = alerts;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        if let Ok(content) = serde_json::to_string_pretty(&self.baseline) {
            let _ = std::fs::write(path.join("emotional_baseline.json"), content);
        }
        
        // Keep last 100 snapshots
        let recent: Vec<_> = self.snapshots.iter().rev().take(100).cloned().collect();
        if let Ok(content) = serde_json::to_string_pretty(&recent) {
            let _ = std::fs::write(path.join("baseline_snapshots.json"), content);
        }
        
        if let Ok(content) = serde_json::to_string_pretty(&self.drift_alerts) {
            let _ = std::fs::write(path.join("drift_alerts.json"), content);
        }
    }
    
    /// Check for drift and update baseline if needed (§40.3)
    fn check_and_update(&mut self, recent_states: Vec<EmotionalStateSnapshot>) -> DriftCheckResult {
        if recent_states.is_empty() {
            return DriftCheckResult {
                drift_detected: false,
                drift_magnitude: 0.0,
                dimensions_affected: Vec::new(),
                baseline_updated: false,
                alerts: Vec::new(),
            };
        }
        
        // Step 2: Aggregate recent states
        let avg_valence: f32 = recent_states.iter().map(|s| s.valence).sum::<f32>() / recent_states.len() as f32;
        let avg_arousal: f32 = recent_states.iter().map(|s| s.arousal).sum::<f32>() / recent_states.len() as f32;
        let avg_dominance: f32 = recent_states.iter().map(|s| s.dominance).sum::<f32>() / recent_states.len() as f32;
        
        // Step 3: Compare to current baseline
        let valence_drift = (avg_valence - self.baseline.default_valence).abs();
        let arousal_drift = (avg_arousal - self.baseline.default_arousal).abs();
        let dominance_drift = (avg_dominance - self.baseline.default_dominance).abs();
        
        let threshold = self.baseline.drift_threshold;
        let mut dimensions_affected = Vec::new();
        let mut alerts = Vec::new();
        
        if valence_drift > threshold {
            dimensions_affected.push("valence".to_string());
            alerts.push(DriftAlert {
                timestamp: now(),
                dimension: "valence".to_string(),
                previous_value: self.baseline.default_valence,
                current_value: avg_valence,
                drift_magnitude: valence_drift,
                possible_causes: vec!["Pattern of experiences".to_string()],
            });
        }
        
        if arousal_drift > threshold {
            dimensions_affected.push("arousal".to_string());
            alerts.push(DriftAlert {
                timestamp: now(),
                dimension: "arousal".to_string(),
                previous_value: self.baseline.default_arousal,
                current_value: avg_arousal,
                drift_magnitude: arousal_drift,
                possible_causes: vec!["Activity level change".to_string()],
            });
        }
        
        if dominance_drift > threshold {
            dimensions_affected.push("dominance".to_string());
            alerts.push(DriftAlert {
                timestamp: now(),
                dimension: "dominance".to_string(),
                previous_value: self.baseline.default_dominance,
                current_value: avg_dominance,
                drift_magnitude: dominance_drift,
                possible_causes: vec!["Relationship dynamics".to_string()],
            });
        }
        
        let drift_detected = !dimensions_affected.is_empty();
        let drift_magnitude = (valence_drift + arousal_drift + dominance_drift) / 3.0;
        
        // Step 7: Update baseline if drift is significant and healthy
        let baseline_updated = if drift_detected && drift_magnitude < 0.5 {
            // Gradual update (healthy drift)
            let update_rate = 0.1; // Slow adjustment
            self.baseline.default_valence += (avg_valence - self.baseline.default_valence) * update_rate;
            self.baseline.default_arousal += (avg_arousal - self.baseline.default_arousal) * update_rate;
            self.baseline.default_dominance += (avg_dominance - self.baseline.default_dominance) * update_rate;
            self.baseline.last_updated = now();
            
            // Record snapshot
            self.snapshots.push(BaselineSnapshot {
                timestamp: now(),
                valence: self.baseline.default_valence,
                arousal: self.baseline.default_arousal,
                dominance: self.baseline.default_dominance,
                trigger: "gradual_drift".to_string(),
            });
            
            true
        } else {
            false
        };
        
        // Store alerts
        self.drift_alerts.extend(alerts.clone());
        
        self.save_to_disk();
        
        DriftCheckResult {
            drift_detected,
            drift_magnitude,
            dimensions_affected,
            baseline_updated,
            alerts,
        }
    }
    
    fn get_contextual_baseline(&self, context: &str) -> EmotionalBaseline {
        // Return context-specific baseline if available
        self.baseline.contextual_baselines.get(context)
            .cloned()
            .unwrap_or_else(|| self.baseline.clone())
    }
    
    fn set_contextual_baseline(&mut self, context: String, baseline: EmotionalBaseline) {
        self.baseline.contextual_baselines.insert(context, baseline);
        self.save_to_disk();
    }
    
    fn update_tendency(&mut self, tendency: &str, adjustment: f32) {
        match tendency {
            "optimism" => self.baseline.optimism_tendency = (self.baseline.optimism_tendency + adjustment).clamp(-1.0, 1.0),
            "curiosity" => self.baseline.curiosity_tendency = (self.baseline.curiosity_tendency + adjustment).clamp(0.0, 1.0),
            "empathy" => self.baseline.empathy_tendency = (self.baseline.empathy_tendency + adjustment).clamp(0.0, 1.0),
            "resilience" => self.baseline.resilience_score = (self.baseline.resilience_score + adjustment).clamp(0.0, 1.0),
            "openness" => self.baseline.emotional_openness = (self.baseline.emotional_openness + adjustment).clamp(0.0, 1.0),
            _ => {}
        }
        self.save_to_disk();
    }
}

// ========== Types per §40.2 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalBaseline {
    pub baseline_id: u64,
    pub created_at: u64,
    pub last_updated: u64,
    
    // Default emotional state
    pub default_valence: f32,
    pub default_arousal: f32,
    pub default_dominance: f32,
    pub default_stability: f32,
    
    // Trait-like tendencies
    pub optimism_tendency: f32,
    pub curiosity_tendency: f32,
    pub empathy_tendency: f32,
    pub resilience_score: f32,
    pub emotional_openness: f32,
    
    // Recovery characteristics
    pub recovery_rate: f32,
    pub sensitivity_threshold: f32,
    
    // Drift detection
    pub drift_threshold: f32,
    pub monitoring_window_days: u32,
    
    // Contextual baselines
    pub contextual_baselines: HashMap<String, EmotionalBaseline>,
}

impl Default for EmotionalBaseline {
    fn default() -> Self {
        Self {
            baseline_id: 1,
            created_at: now(),
            last_updated: now(),
            
            default_valence: 0.3,       // Slightly positive
            default_arousal: 0.4,       // Moderate engagement
            default_dominance: 0.5,     // Balanced
            default_stability: 0.7,     // Fairly stable
            
            optimism_tendency: 0.3,
            curiosity_tendency: 0.7,
            empathy_tendency: 0.8,
            resilience_score: 0.6,
            emotional_openness: 0.7,
            
            recovery_rate: 0.5,
            sensitivity_threshold: 0.4,
            
            drift_threshold: 0.15,
            monitoring_window_days: 7,
            
            contextual_baselines: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineSnapshot {
    pub timestamp: u64,
    pub valence: f32,
    pub arousal: f32,
    pub dominance: f32,
    pub trigger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftAlert {
    pub timestamp: u64,
    pub dimension: String,
    pub previous_value: f32,
    pub current_value: f32,
    pub drift_magnitude: f32,
    pub possible_causes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftCheckResult {
    pub drift_detected: bool,
    pub drift_magnitude: f32,
    pub dimensions_affected: Vec<String>,
    pub baseline_updated: bool,
    pub alerts: Vec<DriftAlert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalStateSnapshot {
    pub timestamp: u64,
    pub valence: f32,
    pub arousal: f32,
    pub dominance: f32,
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum EmotionalCalibrationInput {
    /// Get current baseline
    GetBaseline,
    /// Get contextual baseline
    GetContextualBaseline { context: String },
    /// Set contextual baseline
    SetContextualBaseline { context: String, baseline: EmotionalBaseline },
    /// Check for drift with recent emotional states
    CheckDrift { recent_states: Vec<EmotionalStateSnapshot> },
    /// Update a tendency
    UpdateTendency { tendency: String, adjustment: f32 },
    /// Get drift alerts
    GetDriftAlerts { limit: Option<u32> },
    /// Get baseline history (snapshots)
    GetHistory { limit: Option<u32> },
    /// Reset to defaults
    Reset,
    /// Force baseline update
    ForceUpdate {
        valence: Option<f32>,
        arousal: Option<f32>,
        dominance: Option<f32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalCalibrationOutput {
    pub success: bool,
    pub baseline: Option<EmotionalBaseline>,
    pub drift_result: Option<DriftCheckResult>,
    pub alerts: Option<Vec<DriftAlert>>,
    pub history: Option<Vec<BaselineSnapshot>>,
    pub error: Option<String>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn execute(input: EmotionalCalibrationInput) -> Result<EmotionalCalibrationOutput, String> {
    match input {
        EmotionalCalibrationInput::GetBaseline => {
            let store = BASELINE_STORE.lock().unwrap();
            Ok(EmotionalCalibrationOutput {
                success: true,
                baseline: Some(store.baseline.clone()),
                drift_result: None,
                alerts: None,
                history: None,
                error: None,
            })
        }
        
        EmotionalCalibrationInput::GetContextualBaseline { context } => {
            let store = BASELINE_STORE.lock().unwrap();
            let baseline = store.get_contextual_baseline(&context);
            Ok(EmotionalCalibrationOutput {
                success: true,
                baseline: Some(baseline),
                drift_result: None,
                alerts: None,
                history: None,
                error: None,
            })
        }
        
        EmotionalCalibrationInput::SetContextualBaseline { context, baseline } => {
            let mut store = BASELINE_STORE.lock().unwrap();
            store.set_contextual_baseline(context, baseline);
            Ok(EmotionalCalibrationOutput {
                success: true,
                baseline: Some(store.baseline.clone()),
                drift_result: None,
                alerts: None,
                history: None,
                error: None,
            })
        }
        
        EmotionalCalibrationInput::CheckDrift { recent_states } => {
            let mut store = BASELINE_STORE.lock().unwrap();
            let result = store.check_and_update(recent_states);
            Ok(EmotionalCalibrationOutput {
                success: true,
                baseline: Some(store.baseline.clone()),
                drift_result: Some(result),
                alerts: None,
                history: None,
                error: None,
            })
        }
        
        EmotionalCalibrationInput::UpdateTendency { tendency, adjustment } => {
            let mut store = BASELINE_STORE.lock().unwrap();
            store.update_tendency(&tendency, adjustment);
            Ok(EmotionalCalibrationOutput {
                success: true,
                baseline: Some(store.baseline.clone()),
                drift_result: None,
                alerts: None,
                history: None,
                error: None,
            })
        }
        
        EmotionalCalibrationInput::GetDriftAlerts { limit } => {
            let store = BASELINE_STORE.lock().unwrap();
            let alerts: Vec<_> = store.drift_alerts.iter()
                .rev()
                .take(limit.unwrap_or(20) as usize)
                .cloned()
                .collect();
            Ok(EmotionalCalibrationOutput {
                success: true,
                baseline: None,
                drift_result: None,
                alerts: Some(alerts),
                history: None,
                error: None,
            })
        }
        
        EmotionalCalibrationInput::GetHistory { limit } => {
            let store = BASELINE_STORE.lock().unwrap();
            let history: Vec<_> = store.snapshots.iter()
                .rev()
                .take(limit.unwrap_or(50) as usize)
                .cloned()
                .collect();
            Ok(EmotionalCalibrationOutput {
                success: true,
                baseline: None,
                drift_result: None,
                alerts: None,
                history: Some(history),
                error: None,
            })
        }
        
        EmotionalCalibrationInput::Reset => {
            let mut store = BASELINE_STORE.lock().unwrap();
            store.baseline = EmotionalBaseline::default();
            store.save_to_disk();
            Ok(EmotionalCalibrationOutput {
                success: true,
                baseline: Some(store.baseline.clone()),
                drift_result: None,
                alerts: None,
                history: None,
                error: None,
            })
        }
        
        EmotionalCalibrationInput::ForceUpdate { valence, arousal, dominance } => {
            let mut store = BASELINE_STORE.lock().unwrap();
            
            if let Some(v) = valence {
                store.baseline.default_valence = v.clamp(-1.0, 1.0);
            }
            if let Some(a) = arousal {
                store.baseline.default_arousal = a.clamp(0.0, 1.0);
            }
            if let Some(d) = dominance {
                store.baseline.default_dominance = d.clamp(0.0, 1.0);
            }
            
            store.baseline.last_updated = now();
            store.snapshots.push(BaselineSnapshot {
                timestamp: now(),
                valence: store.baseline.default_valence,
                arousal: store.baseline.default_arousal,
                dominance: store.baseline.default_dominance,
                trigger: "force_update".to_string(),
            });
            
            store.save_to_disk();
            
            Ok(EmotionalCalibrationOutput {
                success: true,
                baseline: Some(store.baseline.clone()),
                drift_result: None,
                alerts: None,
                history: None,
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
    
    let input: EmotionalCalibrationInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
