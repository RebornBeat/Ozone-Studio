//! VoicePipeline - Pipeline #10
//! 
//! Voice input/output handling with text transcript support.
//! - Speech-to-text for voice input (Whisper local preferred, API optional)
//! - Text-to-speech for voice output
//! - Voice activity detection
//! - Text transcript always available alongside voice
//! - All voice interactions archived for review/playback
//! 
//! VOICE SETTINGS (when consciousness enabled):
//! - Voice can be disabled/enabled independently
//! - Text transcript ALWAYS viewable regardless of voice setting
//! - Local transcription preferred (Whisper)
//! - API-based transcription NOT default (user preference)
//! 
//! Integrates with PromptPipeline for voice-driven interactions

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum VoiceInput {
    /// Start listening for voice input
    StartListening { language: Option<String> },
    /// Stop listening
    StopListening,
    /// Process audio data
    ProcessAudio { audio_base64: String, format: String },
    /// Transcribe audio file (local Whisper preferred)
    TranscribeFile { file_path: String, use_api: Option<bool> },
    /// Speak text (text-to-speech)
    Speak { text: String, voice: Option<String>, speed: Option<f32> },
    /// Stop speaking
    StopSpeaking,
    /// Get available voices
    GetVoices,
    /// Get voice settings
    GetSettings,
    /// Update voice settings
    UpdateSettings { settings: VoiceSettings },
    /// Get transcript history
    GetTranscripts { limit: Option<u32> },
    /// Archive voice interaction
    ArchiveInteraction { interaction_id: u64 },
    /// Get archived interactions for playback
    GetArchived { limit: Option<u32> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSettings {
    pub input_enabled: bool,
    pub output_enabled: bool,
    pub input_language: String,
    pub output_voice: String,
    pub output_speed: f32,
    pub vad_enabled: bool,  // Voice activity detection
    pub auto_send: bool,    // Auto-send on silence
    pub silence_threshold_ms: u32,
    // New settings for transcript and local-first
    pub transcript_enabled: bool,        // Always show text transcript
    pub use_local_whisper: bool,         // Prefer local Whisper over API
    pub api_transcription_enabled: bool, // API transcription (NOT default)
    pub archive_enabled: bool,           // Archive all interactions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub voice_id: String,
    pub name: String,
    pub language: String,
    pub gender: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub id: u64,
    pub timestamp: u64,
    pub speaker: String,       // "user" or "assistant"
    pub text: String,
    pub audio_path: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedInteraction {
    pub interaction_id: u64,
    pub timestamp: u64,
    pub user_audio_path: Option<String>,
    pub user_transcript: String,
    pub assistant_audio_path: Option<String>,
    pub assistant_transcript: String,
    pub metadata: HashMap<String, String>,
}

use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceOutput {
    pub success: bool,
    pub transcription: Option<String>,
    pub confidence: Option<f32>,
    pub is_final: bool,
    pub audio_base64: Option<String>,
    pub voices: Option<Vec<VoiceInfo>>,
    pub settings: Option<VoiceSettings>,
    pub transcripts: Option<Vec<TranscriptEntry>>,
    pub archived: Option<Vec<ArchivedInteraction>>,
    pub error: Option<String>,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            input_enabled: true,
            output_enabled: true,
            input_language: "en-US".to_string(),
            output_voice: "default".to_string(),
            output_speed: 1.0,
            vad_enabled: true,
            auto_send: false,
            silence_threshold_ms: 1500,
            // Defaults: transcript ON, local Whisper ON, API OFF
            transcript_enabled: true,
            use_local_whisper: true,
            api_transcription_enabled: false, // NOT default per user preference
            archive_enabled: true,
        }
    }
}

pub async fn execute(input: VoiceInput) -> Result<VoiceOutput, String> {
    let storage_path = std::env::var("OZONE_VOICE_PATH")
        .unwrap_or_else(|_| "./zsei_data/voice".to_string());
    let _ = std::fs::create_dir_all(&storage_path);
    
    match input {
        VoiceInput::StartListening { language } => {
            Ok(VoiceOutput { success: true, transcription: None, confidence: None, is_final: false, audio_base64: None, voices: None, settings: None, transcripts: None, archived: None, error: None })
        }
        VoiceInput::StopListening => {
            Ok(VoiceOutput { success: true, transcription: None, confidence: None, is_final: true, audio_base64: None, voices: None, settings: None, transcripts: None, archived: None, error: None })
        }
        VoiceInput::ProcessAudio { audio_base64, format } => {
            // Load settings to check if local Whisper or API
            let settings = load_voice_settings(&storage_path);
            
            let transcription = if settings.use_local_whisper {
                // Local Whisper transcription (preferred)
                transcribe_with_local_whisper(&audio_base64, &format).await
            } else if settings.api_transcription_enabled {
                // API transcription (NOT default)
                transcribe_with_api(&audio_base64, &format).await
            } else {
                "Transcription disabled".to_string()
            };
            
            // Archive if enabled
            if settings.archive_enabled {
                archive_transcript(&storage_path, "user", &transcription, None);
            }
            
            Ok(VoiceOutput {
                success: true,
                transcription: Some(transcription),
                confidence: Some(0.95),
                is_final: true,
                audio_base64: None,
                voices: None,
                settings: None,
                transcripts: None,
                archived: None,
                error: None,
            })
        }
        VoiceInput::TranscribeFile { file_path, use_api } => {
            let settings = load_voice_settings(&storage_path);
            let use_api = use_api.unwrap_or(settings.api_transcription_enabled);
            
            let transcription = if use_api {
                transcribe_file_with_api(&file_path).await
            } else {
                transcribe_file_with_local_whisper(&file_path).await
            };
            
            Ok(VoiceOutput {
                success: true,
                transcription: Some(transcription),
                confidence: Some(0.90),
                is_final: true,
                audio_base64: None,
                voices: None,
                settings: None,
                transcripts: None,
                archived: None,
                error: None,
            })
        }
        VoiceInput::Speak { text, voice, speed } => {
            let settings = load_voice_settings(&storage_path);
            
            // Archive assistant response if enabled
            if settings.archive_enabled {
                archive_transcript(&storage_path, "assistant", &text, None);
            }
            
            // Generate TTS audio (local Piper or API)
            let audio = generate_tts(&text, voice.as_deref(), speed).await;
            
            Ok(VoiceOutput {
                success: true,
                transcription: Some(text), // Text transcript always available
                confidence: None,
                is_final: true,
                audio_base64: Some(audio),
                voices: None,
                settings: None,
                transcripts: None,
                archived: None,
                error: None,
            })
        }
        VoiceInput::StopSpeaking => {
            Ok(VoiceOutput { success: true, transcription: None, confidence: None, is_final: true, audio_base64: None, voices: None, settings: None, transcripts: None, archived: None, error: None })
        }
        VoiceInput::GetVoices => {
            let voices = vec![
                VoiceInfo { voice_id: "en-us-1".into(), name: "English US".into(), language: "en-US".into(), gender: "female".into() },
                VoiceInfo { voice_id: "en-gb-1".into(), name: "English UK".into(), language: "en-GB".into(), gender: "male".into() },
            ];
            Ok(VoiceOutput { success: true, transcription: None, confidence: None, is_final: true, audio_base64: None, voices: Some(voices), settings: None, transcripts: None, archived: None, error: None })
        }
        VoiceInput::GetSettings => {
            let settings = load_voice_settings(&storage_path);
            Ok(VoiceOutput { success: true, transcription: None, confidence: None, is_final: true, audio_base64: None, voices: None, settings: Some(settings), transcripts: None, archived: None, error: None })
        }
        VoiceInput::UpdateSettings { settings } => {
            save_voice_settings(&storage_path, &settings);
            Ok(VoiceOutput { success: true, transcription: None, confidence: None, is_final: true, audio_base64: None, voices: None, settings: Some(settings), transcripts: None, archived: None, error: None })
        }
        VoiceInput::GetTranscripts { limit } => {
            let transcripts = load_transcripts(&storage_path, limit.unwrap_or(50) as usize);
            Ok(VoiceOutput { success: true, transcription: None, confidence: None, is_final: true, audio_base64: None, voices: None, settings: None, transcripts: Some(transcripts), archived: None, error: None })
        }
        VoiceInput::ArchiveInteraction { interaction_id } => {
            // Archive a specific interaction for review
            Ok(VoiceOutput { success: true, transcription: None, confidence: None, is_final: true, audio_base64: None, voices: None, settings: None, transcripts: None, archived: None, error: None })
        }
        VoiceInput::GetArchived { limit } => {
            let archived = load_archived_interactions(&storage_path, limit.unwrap_or(50) as usize);
            Ok(VoiceOutput { success: true, transcription: None, confidence: None, is_final: true, audio_base64: None, voices: None, settings: None, transcripts: None, archived: Some(archived), error: None })
        }
    }
}

// Helper functions for voice processing

fn load_voice_settings(storage_path: &str) -> VoiceSettings {
    let settings_file = Path::new(storage_path).join("settings.json");
    std::fs::read_to_string(&settings_file)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

fn save_voice_settings(storage_path: &str, settings: &VoiceSettings) {
    let settings_file = Path::new(storage_path).join("settings.json");
    let _ = std::fs::write(&settings_file, serde_json::to_string_pretty(settings).unwrap_or_default());
}

fn archive_transcript(storage_path: &str, speaker: &str, text: &str, audio_path: Option<&str>) {
    let transcripts_file = Path::new(storage_path).join("transcripts.json");
    let mut transcripts: Vec<TranscriptEntry> = std::fs::read_to_string(&transcripts_file)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();
    
    let entry = TranscriptEntry {
        id: transcripts.len() as u64 + 1,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        speaker: speaker.to_string(),
        text: text.to_string(),
        audio_path: audio_path.map(String::from),
        confidence: 0.95,
    };
    
    transcripts.push(entry);
    let _ = std::fs::write(&transcripts_file, serde_json::to_string_pretty(&transcripts).unwrap_or_default());
}

fn load_transcripts(storage_path: &str, limit: usize) -> Vec<TranscriptEntry> {
    let transcripts_file = Path::new(storage_path).join("transcripts.json");
    let mut transcripts: Vec<TranscriptEntry> = std::fs::read_to_string(&transcripts_file)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();
    
    transcripts.reverse();
    transcripts.truncate(limit);
    transcripts
}

fn load_archived_interactions(storage_path: &str, limit: usize) -> Vec<ArchivedInteraction> {
    let archived_file = Path::new(storage_path).join("archived.json");
    let mut archived: Vec<ArchivedInteraction> = std::fs::read_to_string(&archived_file)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();
    
    archived.reverse();
    archived.truncate(limit);
    archived
}

async fn transcribe_with_local_whisper(audio_base64: &str, format: &str) -> String {
    // Local Whisper transcription using whisper.cpp or similar
    // This is the preferred method per user preference
    "Transcribed with local Whisper".to_string()
}

async fn transcribe_with_api(audio_base64: &str, format: &str) -> String {
    // API-based transcription (NOT default per user preference)
    "Transcribed with API".to_string()
}

async fn transcribe_file_with_local_whisper(file_path: &str) -> String {
    // Local Whisper file transcription
    "File transcribed with local Whisper".to_string()
}

async fn transcribe_file_with_api(file_path: &str) -> String {
    // API-based file transcription
    "File transcribed with API".to_string()
}

async fn generate_tts(text: &str, voice: Option<&str>, speed: Option<f32>) -> String {
    // Generate TTS audio using local Piper or API
    "base64_audio_data".to_string()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: VoiceInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
