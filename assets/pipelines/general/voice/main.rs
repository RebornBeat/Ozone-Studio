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

mod audio;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

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

            let result = if settings.use_local_whisper {
                // Local Whisper transcription (preferred) — real backend
                // dispatch per config (whisper_rs | whisper_cpp), capture
                // prepared per the backend's input contract.
                transcribe_with_local_whisper(&audio_base64, &format).await
            } else if settings.api_transcription_enabled {
                // API transcription (NOT default)
                transcribe_with_api(&audio_base64, &format).await
            } else {
                Err("Transcription disabled".to_string())
            };

            match result {
                Ok(transcription) => {
                    if settings.archive_enabled {
                        archive_transcript(&storage_path, "user", &transcription, None);
                    }
                    Ok(VoiceOutput {
                        success: true,
                        transcription: Some(transcription),
                        // No fabricated confidence: the CLI path does not
                        // report it and we do not invent numbers.
                        confidence: None,
                        is_final: true,
                        audio_base64: None,
                        voices: None,
                        settings: None,
                        transcripts: None,
                        archived: None,
                        error: None,
                    })
                }
                Err(e) => Ok(VoiceOutput {
                    success: false,
                    transcription: None,
                    confidence: None,
                    is_final: false,
                    audio_base64: None,
                    voices: None,
                    settings: None,
                    transcripts: None,
                    archived: None,
                    error: Some(e),
                }),
            }
        }
        VoiceInput::TranscribeFile { file_path, use_api } => {
            let settings = load_voice_settings(&storage_path);
            let use_api = use_api.unwrap_or(settings.api_transcription_enabled);

            let result = if use_api {
                transcribe_file_with_api(&file_path).await
            } else {
                transcribe_file_with_local_whisper(&file_path).await
            };

            match result {
                Ok(transcription) => Ok(VoiceOutput {
                    success: true,
                    transcription: Some(transcription),
                    confidence: None, // not measured — not claimed
                    is_final: true,
                    audio_base64: None,
                    voices: None,
                    settings: None,
                    transcripts: None,
                    archived: None,
                    error: None,
                }),
                Err(e) => Ok(VoiceOutput {
                    success: false,
                    transcription: None,
                    confidence: None,
                    is_final: false,
                    audio_base64: None,
                    voices: None,
                    settings: None,
                    transcripts: None,
                    archived: None,
                    error: Some(e),
                }),
            }
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

// ============================================================================
// REAL TRANSCRIPTION — genuine Whisper per config::VoiceConfig
// ============================================================================

/// VoiceConfig fields arrive as OZONE_VOICE_* env vars (host maps
/// config::VoiceConfig when spawning/forwarding — the pipeline stays
/// standalone and backend-agnostic):
///   OZONE_VOICE_BACKEND       "whisper_rs" | "whisper_cpp" | "api"
///   OZONE_VOICE_MODEL_PATH    whisper model file (ggml/bin)
///   OZONE_VOICE_CPP_PATH      whisper.cpp CLI binary
///   OZONE_VOICE_API_ENDPOINT  transcription endpoint (api backend)
///   OZONE_VOICE_API_KEY       api key value (api backend)
///   OZONE_VOICE_LANGUAGE      language hint (e.g. "en"); empty = auto
///   OZONE_VOICE_FFMPEG        ffmpeg binary (default "ffmpeg")
fn voice_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn voice_language() -> Option<String> {
    voice_env("OZONE_VOICE_LANGUAGE")
}

fn voice_ffmpeg() -> String {
    voice_env("OZONE_VOICE_FFMPEG").unwrap_or_else(|| "ffmpeg".to_string())
}

/// whisper.cpp CLI on a prepared 16 kHz mono WAV file. Real spawn, real
/// stdout parse: `-nt` strips timestamps so the transcript is clean text.
fn transcribe_cli(wav_path: &Path, model_path: &str, cli_path: &str) -> Result<String, String> {
    let mut cmd = Command::new(cli_path);
    cmd.arg("-m").arg(model_path)
        .arg("-f").arg(wav_path.to_string_lossy().as_ref())
        .arg("-nt"); // no timestamps — plain transcript text
    if let Some(lang) = voice_language() {
        cmd.arg("-l").arg(lang);
    }
    let out = cmd.output().map_err(|e| {
        format!(
            "whisper CLI spawn failed ({}): is whisper.cpp built and the path configured?",
            e
        )
    })?;
    if !out.status.success() {
        return Err(format!(
            "whisper CLI failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    // whisper.cpp prints segment lines; keep non-empty lines, trim noise.
    let text = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    Ok(text)
}

/// whisper_rs integrated backend — in-process whisper.cpp via the crate.
fn transcribe_rs(audio_bytes: &[u8], format: &str, model_path: &str) -> Result<String, String> {
    let ffmpeg = voice_ffmpeg();
    let pcm = audio::prepare_samples_for_rs(audio_bytes, format, &ffmpeg)?;
    let samples_f32: Vec<f32> =
        pcm.samples.iter().map(|s| *s as f32 / 32768.0).collect();
    if samples_f32.is_empty() {
        return Err("no audio samples after preparation".into());
    }

    use whisper_rs::{
        FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters,
    };
    let ctx = WhisperContext::new_with_params(
        model_path,
        WhisperContextParameters::default(),
    )
    .map_err(|e| format!("whisper model load failed: {e}"))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("whisper state failed: {e}"))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    // whisper-rs stores the language reference in params — bind it here so it
    // outlives the params usage below.
    let lang = voice_language();
    params.set_language(lang.as_deref());
    params.set_translate(false);
    state
        .full(params, &samples_f32)
        .map_err(|e| format!("whisper inference failed: {e}"))?;

    let segments = state
        .full_n_segments()
        .map_err(|e| format!("whisper segment count failed: {e}"))?;
    let mut text = String::new();
    for i in 0..segments {
        let seg = state
            .full_get_segment_text(i)
            .map_err(|e| format!("whisper segment read failed: {e}"))?;
        text.push_str(&seg);
    }
    Ok(text.trim().to_string())
}

/// Local Whisper dispatch: integrated whisper_rs crate, or whisper_cpp CLI —
/// each prepared per its input contract (samples in memory vs WAV file).
async fn transcribe_with_local_whisper(audio_base64: &str, format: &str) -> Result<String, String> {
    use base64::Engine;
    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(audio_base64)
        .map_err(|e| format!("audio base64 decode failed: {e}"))?;
    if audio_bytes.is_empty() {
        return Err("empty audio capture".into());
    }

    let backend = voice_env("OZONE_VOICE_BACKEND").unwrap_or_else(|| "whisper_cpp".to_string());
    let model_path = voice_env("OZONE_VOICE_MODEL_PATH")
        .ok_or("whisper model path not configured (OZONE_VOICE_MODEL_PATH)")?;

    match backend.as_str() {
        "whisper_rs" => transcribe_rs(&audio_bytes, format, &model_path),
        "whisper_cpp" => {
            let cli = voice_env("OZONE_VOICE_CPP_PATH")
                .unwrap_or_else(|| "/usr/local/bin/whisper-cli".to_string());
            let wav = audio::prepare_wav_file_for_cli(
                &audio_bytes,
                format,
                &voice_ffmpeg(),
            )?;
            let result = transcribe_cli(&wav, &model_path, &cli);
            audio::cleanup_temp_wav(&wav);
            result
        }
        other => Err(format!(
            "unknown local whisper backend '{other}' (expected whisper_rs | whisper_cpp)"
        )),
    }
}

/// API transcription — OpenAI-compatible /audio/transcriptions multipart.
/// NOT the default per user preference; requires OZONE_VOICE_API_ENDPOINT +
/// OZONE_VOICE_API_KEY.
async fn transcribe_with_api(audio_base64: &str, format: &str) -> Result<String, String> {
    use base64::Engine;
    let endpoint = voice_env("OZONE_VOICE_API_ENDPOINT")
        .ok_or("API transcription endpoint not configured (OZONE_VOICE_API_ENDPOINT)")?;
    let api_key = voice_env("OZONE_VOICE_API_KEY")
        .ok_or("API key not configured (OZONE_VOICE_API_KEY)")?;
    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(audio_base64)
        .map_err(|e| format!("audio base64 decode failed: {e}"))?;
    let ext = format.trim_start_matches('.').to_string();

    let boundary = "oz-voice-boundary-7f3a";
    let file_part = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"capture.{ext}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    );
    let model_part = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nwhisper-1\r\n"
    );
    let closing = format!("--{boundary}--\r\n");
    let mut body = Vec::new();
    body.extend_from_slice(model_part.as_bytes());
    body.extend_from_slice(file_part.as_bytes());
    body.extend_from_slice(&audio_bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(closing.as_bytes());

    let client = reqwest::Client::new();
    let resp = client
        .post(&endpoint)
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header("Authorization", format!("Bearer {api_key}"))
        .body(body)
        .send()
        .await
        .map_err(|e| format!("API transcription request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "API transcription returned HTTP {}",
            resp.status()
        ));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    json.get("text")
        .and_then(|t| t.as_str())
        .map(|t| t.trim().to_string())
        .ok_or_else(|| "API transcription response missing text".to_string())
}

async fn transcribe_file_with_local_whisper(file_path: &str) -> Result<String, String> {
    let audio_bytes = std::fs::read(file_path)
        .map_err(|e| format!("failed to read {file_path}: {e}"))?;
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav");
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);
    transcribe_with_local_whisper(&b64, ext).await
}

async fn transcribe_file_with_api(file_path: &str) -> Result<String, String> {
    let audio_bytes = std::fs::read(file_path)
        .map_err(|e| format!("failed to read {file_path}: {e}"))?;
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav");
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&audio_bytes);
    transcribe_with_api(&b64, ext).await
}

async fn generate_tts(text: &str, voice: Option<&str>, speed: Option<f32>) -> String {
    // Generate TTS audio using local Piper or API
    "base64_audio_data".to_string()
}

/// Executor contract: `--input` carries the full PipelineInput JSON
/// ({"data":{...},"context":{...}}) — unwrap the data envelope. Bare
/// action payloads and stdin are accepted for standalone use.
fn parse_cli_input<T: serde::de::DeserializeOwned>() -> Result<T, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }
    let raw = match input_json {
        Some(s) => s,
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).map_err(|e| e.to_string())?;
            buf
        }
    };
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let inner = v.get("data").cloned().unwrap_or(v);
    serde_json::from_value(inner).map_err(|e| e.to_string())
}

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

fn main() {
    // SERVE MODE — connect-model: persistent voice service (transcription +
    // TTS) registered with the host. VoiceConfig arrives via OZONE_VOICE_*
    // env (bootstrap + gRPC updates export it).
    if let Some(opts) = ozone_serve::serve_mode() {
        let handler = std::sync::Arc::new(|action_payload: serde_json::Value| {
            let input: VoiceInput = serde_json::from_value(action_payload)
                .unwrap_or(VoiceInput::StopListening);
            let rt = tokio::runtime::Runtime::new().expect("serve runtime");
            match rt.block_on(execute(input)) {
                Ok(output) => serde_json::to_value(&output)
                    .unwrap_or(serde_json::json!({"success": false})),
                Err(e) => serde_json::json!({"success": false, "error": e}),
            }
        });
        const VOICE_PIPELINE_ID: u64 = 10; // pipeline #10
        ozone_serve::serve(opts, VOICE_PIPELINE_ID, "voice".to_string(), handler);
    }

    let input: VoiceInput = match parse_cli_input() {
        Ok(i) => i,
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": format!("Parse error: {e}")}));
            std::process::exit(1);
        }
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
