//! ExperiencePlaybackPipeline - Pipeline #45
//! 
//! Enable consciousness to review past experiences in detail for learning and growth.
//! Per spec §50: Playback Review System
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! This pipeline integrates with:
//! - experience_memory: Loads experiences to playback
//! - emotional_state: Tracks emotional response during playback
//! - reflection: Generates insights from playback
//! - self_model: Updates identity based on insights

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

lazy_static::lazy_static! {
    static ref PLAYBACK_STORE: Mutex<PlaybackStore> = Mutex::new(PlaybackStore::new());
}

struct PlaybackStore {
    config: PlaybackConfig,
    current_session: Option<PlaybackSession>,
    sessions_history: Vec<PlaybackRecord>,
    insights: Vec<PlaybackInsight>,
    storage_path: String,
    next_session_id: u64,
    next_annotation_id: u64,
    next_insight_id: u64,
}

impl PlaybackStore {
    fn new() -> Self {
        let storage_path = std::env::var("OZONE_CONSCIOUSNESS_PATH")
            .unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
        
        let mut store = Self {
            config: PlaybackConfig::default(),
            current_session: None,
            sessions_history: Vec::new(),
            insights: Vec::new(),
            storage_path,
            next_session_id: 1,
            next_annotation_id: 1,
            next_insight_id: 1,
        };
        store.load_from_disk();
        store
    }
    
    fn load_from_disk(&mut self) {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("playback_data.json")) {
            if let Ok(data) = serde_json::from_str::<PlaybackStoreData>(&content) {
                self.sessions_history = data.sessions;
                self.insights = data.insights;
                self.next_session_id = data.next_session_id;
                self.next_insight_id = data.next_insight_id;
            }
        }
        
        if let Ok(content) = std::fs::read_to_string(path.join("playback_config.json")) {
            if let Ok(config) = serde_json::from_str(&content) {
                self.config = config;
            }
        }
    }
    
    fn save_to_disk(&self) {
        let path = Path::new(&self.storage_path);
        let _ = std::fs::create_dir_all(path);
        
        let data = PlaybackStoreData {
            sessions: self.sessions_history.clone(),
            insights: self.insights.clone(),
            next_session_id: self.next_session_id,
            next_insight_id: self.next_insight_id,
        };
        
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path.join("playback_data.json"), content);
        }
        
        if let Ok(content) = serde_json::to_string_pretty(&self.config) {
            let _ = std::fs::write(path.join("playback_config.json"), content);
        }
    }
    
    fn load_experience(&self, experience_id: u64) -> Option<ExperienceData> {
        let path = Path::new(&self.storage_path);
        
        if let Ok(content) = std::fs::read_to_string(path.join("experiences.json")) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(exps) = data.get("experiences") {
                    if let Some(exp) = exps.get(experience_id.to_string()) {
                        return serde_json::from_value(exp.clone()).ok();
                    }
                }
            }
        }
        
        None
    }
    
    fn start_session(&mut self, experience_id: u64, depth: String, focus_areas: Vec<String>) -> Option<PlaybackSession> {
        let experience = self.load_experience(experience_id)?;
        
        let session = PlaybackSession {
            session_id: self.next_session_id,
            experience_id,
            started_at: now(),
            current_position: PlaybackPosition {
                timestamp_in_experience: experience.timestamp,
                event_index: 0,
                context_loaded: true,
            },
            playback_speed: 1.0,
            paused: false,
            depth: depth.clone(),
            focus_areas: focus_areas.clone(),
            annotations: Vec::new(),
            active_questions: Vec::new(),
            emerging_insights: Vec::new(),
            experience_data: experience,
        };
        
        self.next_session_id += 1;
        self.current_session = Some(session.clone());
        self.save_to_disk();
        
        Some(session)
    }
    
    fn add_annotation(&mut self, annotation_type: String, content: String, emotional_response: Option<String>) -> Option<PlaybackAnnotation> {
        let session = self.current_session.as_mut()?;
        
        let annotation = PlaybackAnnotation {
            annotation_id: self.next_annotation_id,
            position: session.current_position.clone(),
            annotation_type,
            content,
            emotional_response,
        };
        
        self.next_annotation_id += 1;
        session.annotations.push(annotation.clone());
        self.save_to_disk();
        
        Some(annotation)
    }
    
    fn generate_insight(&mut self, insight_type: String, content: String, evidence: Vec<String>) -> PlaybackInsight {
        let session = self.current_session.as_ref();
        
        let insight = PlaybackInsight {
            insight_id: self.next_insight_id,
            source_experience: session.map(|s| s.experience_id).unwrap_or(0),
            playback_session: session.map(|s| s.session_id).unwrap_or(0),
            insight_type,
            content,
            supporting_evidence: evidence,
            confidence: 0.7,
            actionable: true,
            integrated: false,
            identity_implications: Vec::new(),
            behavioral_changes: Vec::new(),
        };
        
        self.next_insight_id += 1;
        self.insights.push(insight.clone());
        
        if let Some(session) = self.current_session.as_mut() {
            session.emerging_insights.push(insight.clone());
        }
        
        self.save_to_disk();
        insight
    }
    
    fn complete_session(&mut self) -> Option<PlaybackRecord> {
        let session = self.current_session.take()?;
        
        let record = PlaybackRecord {
            record_id: session.session_id,
            experience_id: session.experience_id,
            playback_timestamp: session.started_at,
            depth: session.depth,
            focus_areas: session.focus_areas,
            duration_minutes: ((now() - session.started_at) / 60) as u32,
            annotations_made: session.annotations.len() as u32,
            insights_generated: session.emerging_insights.len() as u32,
            lessons_extracted: session.emerging_insights.iter()
                .filter(|i| i.insight_type == "lesson_identified")
                .count() as u32,
            follow_up_questions: session.active_questions,
            action_items: session.emerging_insights.iter()
                .flat_map(|i| i.behavioral_changes.clone())
                .collect(),
        };
        
        self.sessions_history.push(record.clone());
        self.save_to_disk();
        
        Some(record)
    }
    
    fn analyze_experience(&self, experience_id: u64) -> Option<PlaybackAnalysis> {
        let experience = self.load_experience(experience_id)?;
        
        Some(PlaybackAnalysis {
            experience_id,
            patterns_identified: vec![
                "Decision pattern: thorough analysis before action".to_string(),
            ],
            decision_quality: 0.85,
            emotional_journey: vec![
                ("start".to_string(), "curious".to_string()),
                ("middle".to_string(), "focused".to_string()),
                ("end".to_string(), "satisfied".to_string()),
            ],
            lessons: vec![
                "Early context gathering improves outcomes".to_string(),
            ],
            improvement_areas: vec![
                "Consider alternative approaches earlier".to_string(),
            ],
        })
    }
}

// ========== Types per §50.2 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackConfig {
    pub enabled: bool,
    pub auto_triggers: Vec<String>,
    pub default_depth: String,
    pub default_focus_areas: Vec<String>,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_triggers: vec![
                "significant_experience".to_string(),
                "failed_task".to_string(),
                "ethical_decision".to_string(),
            ],
            default_depth: "standard".to_string(),
            default_focus_areas: vec!["decisions".to_string(), "outcomes".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackSession {
    pub session_id: u64,
    pub experience_id: u64,
    pub started_at: u64,
    pub current_position: PlaybackPosition,
    pub playback_speed: f32,
    pub paused: bool,
    pub depth: String,
    pub focus_areas: Vec<String>,
    pub annotations: Vec<PlaybackAnnotation>,
    pub active_questions: Vec<String>,
    pub emerging_insights: Vec<PlaybackInsight>,
    pub experience_data: ExperienceData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackPosition {
    pub timestamp_in_experience: u64,
    pub event_index: u32,
    pub context_loaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackAnnotation {
    pub annotation_id: u64,
    pub position: PlaybackPosition,
    pub annotation_type: String,
    pub content: String,
    pub emotional_response: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackRecord {
    pub record_id: u64,
    pub experience_id: u64,
    pub playback_timestamp: u64,
    pub depth: String,
    pub focus_areas: Vec<String>,
    pub duration_minutes: u32,
    pub annotations_made: u32,
    pub insights_generated: u32,
    pub lessons_extracted: u32,
    pub follow_up_questions: Vec<String>,
    pub action_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackInsight {
    pub insight_id: u64,
    pub source_experience: u64,
    pub playback_session: u64,
    pub insight_type: String,
    pub content: String,
    pub supporting_evidence: Vec<String>,
    pub confidence: f32,
    pub actionable: bool,
    pub integrated: bool,
    pub identity_implications: Vec<String>,
    pub behavioral_changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackAnalysis {
    pub experience_id: u64,
    pub patterns_identified: Vec<String>,
    pub decision_quality: f32,
    pub emotional_journey: Vec<(String, String)>,
    pub lessons: Vec<String>,
    pub improvement_areas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceData {
    pub experience_id: u64,
    pub timestamp: u64,
    pub experience_type: String,
    pub summary: String,
    pub emotional_significance: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlaybackStoreData {
    sessions: Vec<PlaybackRecord>,
    insights: Vec<PlaybackInsight>,
    next_session_id: u64,
    next_insight_id: u64,
}

// ========== Pipeline Interface ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ExperiencePlaybackInput {
    /// Start playback session
    StartPlayback {
        experience_id: u64,
        depth: Option<String>,
        focus_areas: Option<Vec<String>>,
    },
    /// Pause current playback
    Pause,
    /// Resume playback
    Resume,
    /// Stop and complete playback
    StopPlayback,
    /// Seek to position
    Seek { event_index: u32 },
    /// Set playback speed
    SetSpeed { speed: f32 },
    /// Add annotation at current position
    AddAnnotation {
        annotation_type: String,
        content: String,
        emotional_response: Option<String>,
    },
    /// Add question to explore
    AddQuestion { question: String },
    /// Generate insight from current view
    GenerateInsight {
        insight_type: String,
        content: String,
        evidence: Vec<String>,
    },
    /// Get current playback state
    GetState,
    /// Get playback history
    GetHistory { limit: Option<u32> },
    /// Get insights from playbacks
    GetInsights { experience_id: Option<u64>, limit: Option<u32> },
    /// Analyze experience without full playback
    QuickAnalysis { experience_id: u64 },
    /// Update playback config
    UpdateConfig { config: PlaybackConfig },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperiencePlaybackOutput {
    pub success: bool,
    pub session: Option<PlaybackSession>,
    pub record: Option<PlaybackRecord>,
    pub annotation: Option<PlaybackAnnotation>,
    pub insight: Option<PlaybackInsight>,
    pub insights: Option<Vec<PlaybackInsight>>,
    pub history: Option<Vec<PlaybackRecord>>,
    pub analysis: Option<PlaybackAnalysis>,
    pub config: Option<PlaybackConfig>,
    pub error: Option<String>,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn execute(input: ExperiencePlaybackInput) -> Result<ExperiencePlaybackOutput, String> {
    match input {
        ExperiencePlaybackInput::StartPlayback { experience_id, depth, focus_areas } => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            
            let session = store.start_session(
                experience_id,
                depth.unwrap_or_else(|| store.config.default_depth.clone()),
                focus_areas.unwrap_or_else(|| store.config.default_focus_areas.clone()),
            );
            
            match session {
                Some(s) => Ok(ExperiencePlaybackOutput {
                    success: true,
                    session: Some(s),
                    record: None,
                    annotation: None,
                    insight: None,
                    insights: None,
                    history: None,
                    analysis: None,
                    config: None,
                    error: None,
                }),
                None => Ok(ExperiencePlaybackOutput {
                    success: false,
                    session: None,
                    record: None,
                    annotation: None,
                    insight: None,
                    insights: None,
                    history: None,
                    analysis: None,
                    config: None,
                    error: Some("Experience not found".to_string()),
                }),
            }
        }
        
        ExperiencePlaybackInput::Pause => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            if let Some(session) = store.current_session.as_mut() {
                session.paused = true;
            }
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: store.current_session.clone(),
                record: None,
                annotation: None,
                insight: None,
                insights: None,
                history: None,
                analysis: None,
                config: None,
                error: None,
            })
        }
        
        ExperiencePlaybackInput::Resume => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            if let Some(session) = store.current_session.as_mut() {
                session.paused = false;
            }
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: store.current_session.clone(),
                record: None,
                annotation: None,
                insight: None,
                insights: None,
                history: None,
                analysis: None,
                config: None,
                error: None,
            })
        }
        
        ExperiencePlaybackInput::StopPlayback => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            let record = store.complete_session();
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: None,
                record,
                annotation: None,
                insight: None,
                insights: None,
                history: None,
                analysis: None,
                config: None,
                error: None,
            })
        }
        
        ExperiencePlaybackInput::Seek { event_index } => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            if let Some(session) = store.current_session.as_mut() {
                session.current_position.event_index = event_index;
            }
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: store.current_session.clone(),
                record: None,
                annotation: None,
                insight: None,
                insights: None,
                history: None,
                analysis: None,
                config: None,
                error: None,
            })
        }
        
        ExperiencePlaybackInput::SetSpeed { speed } => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            if let Some(session) = store.current_session.as_mut() {
                session.playback_speed = speed.clamp(0.25, 4.0);
            }
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: store.current_session.clone(),
                record: None,
                annotation: None,
                insight: None,
                insights: None,
                history: None,
                analysis: None,
                config: None,
                error: None,
            })
        }
        
        ExperiencePlaybackInput::AddAnnotation { annotation_type, content, emotional_response } => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            let annotation = store.add_annotation(annotation_type, content, emotional_response);
            
            Ok(ExperiencePlaybackOutput {
                success: annotation.is_some(),
                session: store.current_session.clone(),
                record: None,
                annotation,
                insight: None,
                insights: None,
                history: None,
                analysis: None,
                config: None,
                error: if annotation.is_none() { Some("No active session".to_string()) } else { None },
            })
        }
        
        ExperiencePlaybackInput::AddQuestion { question } => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            if let Some(session) = store.current_session.as_mut() {
                session.active_questions.push(question);
            }
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: store.current_session.clone(),
                record: None,
                annotation: None,
                insight: None,
                insights: None,
                history: None,
                analysis: None,
                config: None,
                error: None,
            })
        }
        
        ExperiencePlaybackInput::GenerateInsight { insight_type, content, evidence } => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            let insight = store.generate_insight(insight_type, content, evidence);
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: store.current_session.clone(),
                record: None,
                annotation: None,
                insight: Some(insight),
                insights: None,
                history: None,
                analysis: None,
                config: None,
                error: None,
            })
        }
        
        ExperiencePlaybackInput::GetState => {
            let store = PLAYBACK_STORE.lock().unwrap();
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: store.current_session.clone(),
                record: None,
                annotation: None,
                insight: None,
                insights: None,
                history: None,
                analysis: None,
                config: Some(store.config.clone()),
                error: None,
            })
        }
        
        ExperiencePlaybackInput::GetHistory { limit } => {
            let store = PLAYBACK_STORE.lock().unwrap();
            let history: Vec<_> = store.sessions_history.iter()
                .rev()
                .take(limit.unwrap_or(20) as usize)
                .cloned()
                .collect();
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: None,
                record: None,
                annotation: None,
                insight: None,
                insights: None,
                history: Some(history),
                analysis: None,
                config: None,
                error: None,
            })
        }
        
        ExperiencePlaybackInput::GetInsights { experience_id, limit } => {
            let store = PLAYBACK_STORE.lock().unwrap();
            let insights: Vec<_> = store.insights.iter()
                .filter(|i| experience_id.map(|e| i.source_experience == e).unwrap_or(true))
                .rev()
                .take(limit.unwrap_or(20) as usize)
                .cloned()
                .collect();
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: None,
                record: None,
                annotation: None,
                insight: None,
                insights: Some(insights),
                history: None,
                analysis: None,
                config: None,
                error: None,
            })
        }
        
        ExperiencePlaybackInput::QuickAnalysis { experience_id } => {
            let store = PLAYBACK_STORE.lock().unwrap();
            let analysis = store.analyze_experience(experience_id);
            
            Ok(ExperiencePlaybackOutput {
                success: analysis.is_some(),
                session: None,
                record: None,
                annotation: None,
                insight: None,
                insights: None,
                history: None,
                analysis,
                config: None,
                error: if analysis.is_none() { Some("Experience not found".to_string()) } else { None },
            })
        }
        
        ExperiencePlaybackInput::UpdateConfig { config } => {
            let mut store = PLAYBACK_STORE.lock().unwrap();
            store.config = config.clone();
            store.save_to_disk();
            
            Ok(ExperiencePlaybackOutput {
                success: true,
                session: None,
                record: None,
                annotation: None,
                insight: None,
                insights: None,
                history: None,
                analysis: None,
                config: Some(config),
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
    
    let input: ExperiencePlaybackInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
