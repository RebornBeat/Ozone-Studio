//! OZONE Studio - Pipeline 103: Audio Analysis
//!
//! Modality pipeline for audio processing and structural graph creation.
//! Analyzes audio to detect segments, speakers, transcriptions, and music features.
//! Creates traversable graphs that can be enriched by ZSEI semantic hooks.
//!
//! # Actions
//! - `Analyze`: Extract segments, speakers, music features
//! - `Transcribe`: Speech-to-text conversion
//! - `Diarize`: Speaker identification and segmentation
//! - `CreateGraph`: Build structural graph from analysis
//! - `QueryGraph`: Query graph for specific information
//! - `LinkToModality`: Create cross-modality links (e.g., to text transcription)
//! - `TriggerSemanticHook`: Trigger ZSEI hooks for semantic enrichment
//!
//! # Graph Structure
//! - Nodes: Audio, Segment, Speaker, Word, MusicSection, Event
//! - Edges: Contains, SpokenBy, Precedes, Follows, TranscribedAs

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// ============================================================================
// PIPELINE METADATA
// ============================================================================

pub const PIPELINE_ID: u64 = 103;
pub const PIPELINE_NAME: &str = "audio_analysis";
pub const PIPELINE_VERSION: &str = "0.4.0";
pub const MODALITY: &str = "audio";

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioModalityInput {
    pub action: AudioAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AudioAction {
    /// Analyze audio: segments, speakers, content
    Analyze {
        audio_data: AudioData,
        #[serde(default)]
        depth: AnalysisDepth,
        #[serde(default)]
        transcribe: bool,
        #[serde(default)]
        diarize: bool,
        #[serde(default)]
        detect_music: bool,
        #[serde(default)]
        detect_events: bool,
    },

    /// Transcribe audio to text
    Transcribe {
        audio_data: AudioData,
        #[serde(default)]
        language: Option<String>,
        #[serde(default)]
        word_timestamps: bool,
        #[serde(default)]
        speaker_labels: bool,
    },

    /// Perform speaker diarization
    Diarize {
        audio_data: AudioData,
        #[serde(default)]
        num_speakers: Option<u32>,
        #[serde(default)]
        min_speakers: Option<u32>,
        #[serde(default)]
        max_speakers: Option<u32>,
    },

    /// Analyze music features
    AnalyzeMusic {
        audio_data: AudioData,
        #[serde(default)]
        extract_beats: bool,
        #[serde(default)]
        extract_chords: bool,
        #[serde(default)]
        extract_melody: bool,
    },

    /// Create graph from analysis result
    CreateGraph {
        analysis: AudioAnalysisResult,
        project_id: u64,
        #[serde(default)]
        graph_name: Option<String>,
    },

    /// Update existing graph
    UpdateGraph {
        graph_id: u64,
        updates: AudioGraphUpdate,
    },

    /// Query audio graph
    QueryGraph {
        graph_id: u64,
        query: AudioQuery,
    },

    /// Get graph with all nodes and edges
    GetGraph {
        graph_id: u64,
    },

    /// Link to another modality (e.g., text transcription)
    LinkToModality {
        audio_graph_id: u64,
        target_graph_id: u64,
        target_modality: String,
        relationship: CrossModalityRelation,
    },

    /// Trigger ZSEI semantic hook for enrichment
    TriggerSemanticHook {
        graph_id: u64,
        hook_type: ZSEIHookType,
        #[serde(default)]
        options: HookOptions,
    },

    /// Extract audio segment
    ExtractSegment {
        audio_data: AudioData,
        start_time: f32,
        end_time: f32,
    },

    /// Detect specific audio events
    DetectEvents {
        audio_data: AudioData,
        event_types: Vec<AudioEventType>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioModalityOutput {
    pub success: bool,
    pub action: String,
    pub result: AudioResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub metadata: OutputMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AudioResult {
    Analysis(AudioAnalysisResult),
    Transcription(Transcription),
    Diarization(DiarizationResult),
    MusicAnalysis(MusicAnalysis),
    Graph(AudioGraph),
    Query(QueryResult),
    Link(LinkResult),
    Hook(HookResult),
    Segment(SegmentResult),
    Events(Vec<AudioEvent>),
    Empty,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputMetadata {
    /// Honest absences / unsupported requests for this call (additive field).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    pub pipeline_id: u64,
    pub pipeline_version: String,
    pub processing_time_ms: u64,
    pub timestamp: String,
}

// ============================================================================
// CORE DATA TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AudioData {
    /// Base64 encoded audio data
    Base64 { data: String, mime_type: String },
    /// Local file path
    FilePath(String),
    /// Remote URL
    Url(String),
    /// Raw samples
    Samples {
        data: Vec<f32>,
        sample_rate: u32,
        channels: u8,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
pub enum AnalysisDepth {
    Surface,
    #[default]
    Standard,
    Deep,
    Comprehensive,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct AudioAnalysisResult {
    /// Total duration in seconds
    pub duration_seconds: f32,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u8,
    /// Audio format
    pub format: String,
    /// Bit depth
    pub bit_depth: u16,
    /// Audio segments
    pub segments: Vec<AudioSegment>,
    /// Detected speakers
    pub speakers: Vec<Speaker>,
    /// Transcription (if requested)
    pub transcription: Option<Transcription>,
    /// Music analysis (if applicable)
    pub music_analysis: Option<MusicAnalysis>,
    /// Detected audio events
    pub audio_events: Vec<AudioEvent>,
    /// Audio quality metrics
    pub quality: AudioQuality,
    /// Overall loudness in LUFS
    pub loudness_lufs: f32,
    /// Dynamic range
    pub dynamic_range_db: f32,
    /// Metadata
    pub metadata: HashMap<String, Value>,
    /// Time-frequency grid (band energies, quantized 0..255 over db_floor..0 dBFS).
    pub spectrogram: Option<SpectrogramGrid>,
    /// Fields that were NOT computed, with the reason (honesty rule: absent, never canned).
    pub absent: HashMap<String, String>,
}

/// Spectrogram as a geometric grid (geom kind "grid", frame "time:<graph_id>" at graph time).
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SpectrogramGrid {
    pub frame: String,
    pub rows: usize,
    pub cols: usize,
    pub hop_s: f32,
    pub win_s: f32,
    pub fmax_hz: f32,
    pub db_floor: f32,
    pub band_edges_hz: Vec<f32>,
    /// row-major rows x cols, row 0 = lowest band
    pub values: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioSegment {
    /// Unique identifier
    pub segment_id: String,
    /// Start time in seconds
    pub start_time: f32,
    /// End time in seconds
    pub end_time: f32,
    /// Segment type
    pub segment_type: SegmentType,
    /// Speaker ID (if applicable)
    pub speaker_id: Option<String>,
    /// Confidence score
    pub confidence: f32,
    /// Energy level (RMS)
    pub energy: f32,
    /// Segment label/description
    pub label: Option<String>,
}

impl AudioSegment {
    pub fn duration(&self) -> f32 {
        self.end_time - self.start_time
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SegmentType {
    Speech,
    Music,
    Silence,
    Noise,
    Effect,
    Singing,
    Applause,
    Laughter,
    Mixed,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Speaker {
    /// Unique speaker identifier
    pub speaker_id: String,
    /// Speaker label/name (if identified)
    pub label: Option<String>,
    /// Total speaking time in seconds
    pub total_speaking_time: f32,
    /// Number of speaking turns
    pub turn_count: u32,
    /// Average speaking rate (words per minute)
    pub avg_speaking_rate: Option<f32>,
    /// Segment IDs where this speaker appears
    pub segments: Vec<String>,
    /// Speaker embedding (for identification)
    pub embedding: Option<Vec<f32>>,
    /// Speaker characteristics
    pub characteristics: SpeakerCharacteristics,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SpeakerCharacteristics {
    /// Estimated gender
    pub gender: Option<String>,
    /// Estimated age range
    pub age_range: Option<String>,
    /// Average pitch in Hz
    pub avg_pitch_hz: Option<f32>,
    /// Pitch range
    pub pitch_range: Option<(f32, f32)>,
    /// Speaking style
    pub speaking_style: Option<String>,
    /// Detected accent/language
    pub accent: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transcription {
    /// Full transcribed text
    pub text: String,
    /// Detected language
    pub language: String,
    /// Language confidence
    pub language_confidence: f32,
    /// Word-level details
    pub words: Vec<TranscribedWord>,
    /// Sentence-level details
    pub sentences: Vec<TranscribedSentence>,
    /// Overall transcription confidence
    pub confidence: f32,
    /// Alternative transcriptions
    pub alternatives: Vec<AlternativeTranscription>,
    /// Set when no transcription was produced (honesty rule): the reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absent_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscribedWord {
    /// The word
    pub word: String,
    /// Start time in seconds
    pub start_time: f32,
    /// End time in seconds
    pub end_time: f32,
    /// Confidence score
    pub confidence: f32,
    /// Speaker ID
    pub speaker_id: Option<String>,
    /// Punctuation after word
    pub punctuation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscribedSentence {
    /// Sentence text
    pub text: String,
    /// Start time
    pub start_time: f32,
    /// End time
    pub end_time: f32,
    /// Speaker ID
    pub speaker_id: Option<String>,
    /// Confidence
    pub confidence: f32,
    /// Detected sentiment
    pub sentiment: Option<Sentiment>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sentiment {
    pub label: String,
    pub score: f32,
    pub emotions: HashMap<String, f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlternativeTranscription {
    pub text: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiarizationResult {
    /// Detected speakers
    pub speakers: Vec<Speaker>,
    /// Speaker turns
    pub turns: Vec<SpeakerTurn>,
    /// Total speakers detected
    pub num_speakers: u32,
    /// Diarization confidence
    pub confidence: f32,
    /// Set when no diarization was produced (honesty rule): the reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absent_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpeakerTurn {
    pub speaker_id: String,
    pub start_time: f32,
    pub end_time: f32,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MusicAnalysis {
    /// Tempo in BPM
    pub tempo_bpm: f32,
    /// Tempo confidence
    pub tempo_confidence: f32,
    /// Musical key
    pub key: String,
    /// Key confidence
    pub key_confidence: f32,
    /// Mode (major/minor)
    pub mode: String,
    /// Time signature
    pub time_signature: String,
    /// Beat positions in seconds
    pub beats: Vec<f32>,
    /// Downbeat positions
    pub downbeats: Vec<f32>,
    /// Music sections
    pub sections: Vec<MusicSection>,
    /// Chord progression
    pub chords: Vec<ChordEvent>,
    /// Melody contour
    pub melody: Option<MelodyAnalysis>,
    /// Genre classification
    pub genre: Option<GenreClassification>,
    /// Instruments detected
    pub instruments: Vec<InstrumentDetection>,
    /// Energy/arousal over time
    pub energy_curve: Vec<(f32, f32)>,
    /// Valence over time
    pub valence_curve: Vec<(f32, f32)>,
    /// Fields not computed (empty/zero above), with the reason.
    #[serde(default)]
    pub absent: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MusicSection {
    /// Section label (intro, verse, chorus, etc.)
    pub label: String,
    /// Start time
    pub start_time: f32,
    /// End time
    pub end_time: f32,
    /// Confidence
    pub confidence: f32,
    /// Repetition index (1st verse, 2nd verse, etc.)
    pub repetition: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChordEvent {
    /// Chord name (e.g., "Am", "G7")
    pub chord: String,
    /// Start time
    pub start_time: f32,
    /// End time
    pub end_time: f32,
    /// Confidence
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MelodyAnalysis {
    /// Note events
    pub notes: Vec<NoteEvent>,
    /// Melodic contour description
    pub contour: String,
    /// Pitch range
    pub pitch_range: (f32, f32),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoteEvent {
    /// MIDI note number
    pub midi_note: u8,
    /// Note name (e.g., "C4")
    pub note_name: String,
    /// Start time
    pub start_time: f32,
    /// Duration
    pub duration: f32,
    /// Velocity/loudness
    pub velocity: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenreClassification {
    /// Primary genre
    pub primary: String,
    /// Genre confidence scores
    pub scores: HashMap<String, f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstrumentDetection {
    /// Instrument name
    pub instrument: String,
    /// Confidence
    pub confidence: f32,
    /// Time ranges where detected
    pub time_ranges: Vec<(f32, f32)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioEvent {
    /// Event type
    pub event_type: AudioEventType,
    /// Start time
    pub start_time: f32,
    /// End time
    pub end_time: f32,
    /// Confidence
    pub confidence: f32,
    /// Event description
    pub description: Option<String>,
    /// Additional properties
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AudioEventType {
    // Speech events
    SpeechStart,
    SpeechEnd,
    SpeakerChange,
    Interruption,
    Overlap,
    // Music events
    BeatDrop,
    KeyChange,
    TempoChange,
    // Sound events
    DoorSlam,
    Footsteps,
    Clapping,
    Laughter,
    Coughing,
    GlassBreak,
    Gunshot,
    Alarm,
    Siren,
    DogBark,
    BabyyCry,
    Telephone,
    // Quality events
    Clipping,
    Dropout,
    NoiseSpike,
    // Custom
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AudioQuality {
    /// Signal-to-noise ratio in dB
    pub snr_db: f32,
    /// Clipping percentage
    pub clipping_percent: f32,
    /// Background noise level
    pub noise_floor_db: f32,
    /// Speech clarity score (for speech content)
    pub speech_clarity: Option<f32>,
    /// Music quality score (for music content)
    pub music_quality: Option<f32>,
    /// Overall quality score
    pub overall_score: f32,
    /// Quality issues
    pub issues: Vec<QualityIssue>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualityIssue {
    pub issue_type: String,
    pub severity: f32,
    pub start_time: Option<f32>,
    pub end_time: Option<f32>,
    pub description: String,
}

// ============================================================================
// GRAPH TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioGraph {
    /// Unique graph identifier
    pub graph_id: u64,
    /// Graph name
    pub name: String,
    /// Modality type
    pub modality: String,
    /// Project this graph belongs to
    pub project_id: u64,
    /// Source audio information
    pub source: AudioSource,
    /// Graph nodes
    pub nodes: Vec<AudioGraphNode>,
    /// Graph edges
    pub edges: Vec<AudioGraphEdge>,
    /// Graph metadata
    pub metadata: GraphMetadata,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioSource {
    pub path: Option<String>,
    pub url: Option<String>,
    pub hash: String,
    pub duration_seconds: f32,
    pub sample_rate: u32,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioGraphNode {
    /// Unique node identifier
    pub node_id: u64,
    /// Node type
    pub node_type: AudioNodeType,
    /// Display label
    pub label: String,
    /// Content/description
    pub content: String,
    /// Time range (if applicable)
    pub time_range: Option<TimeRange>,
    /// Confidence score
    pub confidence: f32,
    /// Node properties
    pub properties: HashMap<String, Value>,
    /// Semantic annotations
    pub annotations: Vec<SemanticAnnotation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeRange {
    pub start: f32,
    pub end: f32,
}

impl TimeRange {
    pub fn duration(&self) -> f32 {
        self.end - self.start
    }

    pub fn overlaps(&self, other: &TimeRange) -> bool {
        self.start < other.end && self.end > other.start
    }

    pub fn contains(&self, time: f32) -> bool {
        time >= self.start && time <= self.end
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AudioNodeType {
    /// Root audio node
    Audio,
    /// Audio segment
    Segment,
    /// Speaker
    Speaker,
    /// Transcribed word
    Word,
    /// Transcribed sentence
    Sentence,
    /// Music section
    MusicSection,
    /// Chord
    Chord,
    /// Beat
    Beat,
    /// Audio event
    Event,
    /// Note
    Note,
    /// Spectrogram grid (geometry layer: kind "grid")
    Spectrogram,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticAnnotation {
    pub annotation_type: String,
    pub value: Value,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioGraphEdge {
    /// Unique edge identifier
    pub edge_id: u64,
    /// Source node ID
    pub from_node: u64,
    /// Target node ID
    pub to_node: u64,
    /// Edge type
    pub edge_type: AudioEdgeType,
    /// Edge weight/strength
    pub weight: f32,
    /// Edge properties
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AudioEdgeType {
    // Structural edges
    Contains,
    ContainedBy,
    Precedes,
    Follows,
    Overlaps,
    // Semantic edges
    SpokenBy,
    TranscribedAs,
    BelongsToSection,
    SimilarTo,
    RelatesTo,
    // Cross-modality edges
    TranscribedTo,
    DescribedBy,
    SynchronizedWith,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub has_transcription: bool,
    pub has_diarization: bool,
    pub has_music_analysis: bool,
    pub semantic_enriched: bool,
    pub cross_modal_links: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioGraphUpdate {
    pub add_nodes: Vec<AudioGraphNode>,
    pub update_nodes: Vec<AudioGraphNode>,
    pub remove_nodes: Vec<u64>,
    pub add_edges: Vec<AudioGraphEdge>,
    pub remove_edges: Vec<u64>,
    pub metadata_updates: Option<HashMap<String, Value>>,
}

// ============================================================================
// QUERY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioQuery {
    pub query_type: AudioQueryType,
    pub parameters: HashMap<String, Value>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub min_confidence: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AudioQueryType {
    /// Find segments by speaker
    FindSpeaker { speaker_id: String },
    /// Find segments by type
    FindSegments { segment_types: Vec<SegmentType> },
    /// Search transcript text
    SearchTranscript { pattern: String },
    /// Get timeline of events
    GetTimeline { start_time: f32, end_time: f32 },
    /// Get nodes at specific time
    GetNodesAtTime { time: f32 },
    /// Get nodes by type
    GetNodesByType { node_type: AudioNodeType },
    /// Get connected nodes
    GetConnected { node_id: u64, max_depth: u32 },
    /// Find music sections
    FindMusicSections { section_types: Vec<String> },
    /// Custom query
    Custom { query: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub query_type: String,
    pub nodes: Vec<AudioGraphNode>,
    pub edges: Vec<AudioGraphEdge>,
    pub total_matches: usize,
    pub metadata: HashMap<String, Value>,
}

// ============================================================================
// CROSS-MODALITY & HOOK TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CrossModalityRelation {
    TranscribedTo,
    SynchronizedWith,
    DescribedBy,
    IllustratedBy,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkResult {
    pub link_id: u64,
    pub source_graph_id: u64,
    pub target_graph_id: u64,
    pub relationship: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ZSEIHookType {
    OnGraphCreated,
    OnInferRelationships,
    OnEdgeCompletion,
    OnCrossModalityLink,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HookOptions {
    pub max_nodes: Option<usize>,
    pub min_confidence: Option<f32>,
    pub async_processing: bool,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HookResult {
    pub hook_type: ZSEIHookType,
    pub success: bool,
    pub nodes_processed: usize,
    pub edges_added: usize,
    pub annotations_added: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SegmentResult {
    pub audio_data: AudioData,
    pub original_range: TimeRange,
    pub duration_seconds: f32,
}

// ============================================================================
// EXECUTION
// ============================================================================

pub async fn execute(input: Value) -> Result<Value, String> {
    let start_time = std::time::Instant::now();

    // Honest refusal for a text-only "mention" with no real raw audio data
    // attached, BEFORE the strict typed deserialization below would reject
    // it with an accidental, low-level parse error. Only the one action that
    // requires raw data (Analyze) is checked.
    if input.get("action").and_then(|a| a.get("type")).and_then(|t| t.as_str()) == Some("Analyze")
        && input.get("action").and_then(|a| a.get("audio_data")).map(|d| d.is_null()).unwrap_or(true)
    {
        return Ok(serde_json::json!({
            "success": false,
            "action": "Analyze",
            "result": null,
            "error": "No raw audio data provided — this call appears to be a text-only mention with no attached data file; real audio analysis requires an actual data file",
            "metadata": {
                "pipeline_id": PIPELINE_ID,
                "pipeline_version": PIPELINE_VERSION,
                "processing_time_ms": start_time.elapsed().as_millis(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        }));
    }

    let input: AudioModalityInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    let mut notes: Vec<String> = Vec::new();
    let result = match input.action {
        AudioAction::Analyze {
            audio_data,
            depth,
            transcribe,
            diarize,
            detect_music,
            detect_events,
        } => {
            let analysis = analyze_audio(
                &audio_data,
                depth,
                transcribe,
                diarize,
                detect_music,
                detect_events,
            )
            .await?;
            for (k, v) in &analysis.absent {
                notes.push(format!("absent {}: {}", k, v));
            }
            ("Analyze", AudioResult::Analysis(analysis))
        }

        AudioAction::Transcribe {
            audio_data,
            language,
            word_timestamps,
            speaker_labels,
        } => {
            let transcription =
                transcribe_audio(&audio_data, language, word_timestamps, speaker_labels).await?;
            if let Some(r) = &transcription.absent_reason {
                notes.push(format!("absent transcription: {}", r));
            }
            ("Transcribe", AudioResult::Transcription(transcription))
        }

        AudioAction::Diarize {
            audio_data,
            num_speakers,
            min_speakers,
            max_speakers,
        } => {
            let result =
                diarize_audio(&audio_data, num_speakers, min_speakers, max_speakers).await?;
            if let Some(r) = &result.absent_reason {
                notes.push(format!("absent diarization: {}", r));
            }
            ("Diarize", AudioResult::Diarization(result))
        }

        AudioAction::AnalyzeMusic {
            audio_data,
            extract_beats,
            extract_chords,
            extract_melody,
        } => {
            let analysis =
                analyze_music(&audio_data, extract_beats, extract_chords, extract_melody).await?;
            for (k, v) in &analysis.absent {
                notes.push(format!("absent {}: {}", k, v));
            }
            ("AnalyzeMusic", AudioResult::MusicAnalysis(analysis))
        }

        AudioAction::CreateGraph {
            analysis,
            project_id,
            graph_name,
        } => {
            let mut graph = create_graph(analysis, project_id, graph_name).await?;
            if let Err(e) = persist_audio_graph(&mut graph).await {
                eprintln!("audio CreateGraph: ZSEI persistence failed: {e}");
            }
            ("CreateGraph", AudioResult::Graph(graph))
        }

        AudioAction::UpdateGraph { graph_id, updates } => {
            let graph = update_graph(graph_id, updates).await?;
            ("UpdateGraph", AudioResult::Graph(graph))
        }

        AudioAction::QueryGraph { graph_id, query } => {
            let result = query_graph(graph_id, query).await?;
            ("QueryGraph", AudioResult::Query(result))
        }

        AudioAction::GetGraph { graph_id } => {
            let graph = get_graph(graph_id).await?;
            ("GetGraph", AudioResult::Graph(graph))
        }

        AudioAction::LinkToModality {
            audio_graph_id,
            target_graph_id,
            target_modality,
            relationship,
        } => {
            let link =
                link_to_modality(audio_graph_id, target_graph_id, &target_modality, relationship)
                    .await?;
            ("LinkToModality", AudioResult::Link(link))
        }

        AudioAction::TriggerSemanticHook {
            graph_id,
            hook_type,
            options,
        } => {
            let result = trigger_semantic_hook(graph_id, hook_type, options).await?;
            ("TriggerSemanticHook", AudioResult::Hook(result))
        }

        AudioAction::ExtractSegment {
            audio_data,
            start_time,
            end_time,
        } => {
            let result = extract_segment(&audio_data, start_time, end_time).await?;
            ("ExtractSegment", AudioResult::Segment(result))
        }

        AudioAction::DetectEvents {
            audio_data,
            event_types,
        } => {
            let (events, unsupported) = detect_events(&audio_data, &event_types).await?;
            for u in unsupported {
                notes.push(format!("unsupported event type: {}", u));
            }
            ("DetectEvents", AudioResult::Events(events))
        }
    };

    let output = AudioModalityOutput {
        success: true,
        action: result.0.to_string(),
        result: result.1,
        error: None,
        metadata: OutputMetadata {
            notes,
            pipeline_id: PIPELINE_ID,
            pipeline_version: PIPELINE_VERSION.to_string(),
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    };

    serde_json::to_value(output).map_err(|e| format!("Failed to serialize output: {}", e))
}

// ============================================================================
// ACTION IMPLEMENTATIONS
// ============================================================================

// ============================================================================
// REAL SIGNAL PROCESSING (honesty rule D16: compute from the input or report
// ABSENT with a reason). WAV decode, FFT, K-weighted loudness, activity
// segmentation, onset/tempo/key estimation, spectrogram grid. No model, no
// network, no external DSP crate. UNCOMPILED pending the EU build gate.
// ============================================================================

const SPEC_ROWS: usize = 48;
const SPEC_COLS: usize = 128;
const MAX_ANALYSIS_FRAMES: usize = 40_000;
const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
// Krumhansl-Kessler key profiles (tonic first).
const KK_MAJOR: [f64; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const KK_MINOR: [f64; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

const ABSENT_ASR: &str = "speech recognition is a model capability and is not run by this pipeline: call a transcription tool (e.g. whisper) through /mcp/call and attach its output";
const ABSENT_DIARIZATION: &str = "speaker diarization needs a speaker-embedding model, which this pipeline does not run: call a diarization tool through /mcp/call";

/// Decoded audio: one Vec<f32> (range -1..1) per channel.
#[derive(Debug, Clone)]
pub struct Pcm {
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub format: String,
    pub channels: Vec<Vec<f32>>,
}

impl Pcm {
    pub fn frames(&self) -> usize {
        self.channels.first().map(|c| c.len()).unwrap_or(0)
    }
    pub fn duration(&self) -> f32 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.frames() as f32 / self.sample_rate as f32
        }
    }
    pub fn mono(&self) -> Vec<f32> {
        let n = self.frames();
        let c = self.channels.len().max(1) as f32;
        (0..n).map(|i| self.channels.iter().map(|ch| ch[i]).sum::<f32>() / c).collect()
    }
}

fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for c in s.bytes() {
        let v: u8 = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => return Err(format!("invalid base64 byte 0x{:02x}", c)),
        };
        buf = (buf << 6) | (v as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

/// Parse a RIFF/WAVE file: PCM 8/16/24/32-bit, IEEE float 32/64, WAVE_FORMAT_EXTENSIBLE.
pub fn parse_wav(b: &[u8]) -> Result<Pcm, String> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file: only WAV is decoded by this pipeline; decode other formats with an ffmpeg tool call and pass the WAV".to_string());
    }
    let mut pos = 12usize;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= b.len() {
        let id = &b[pos..pos + 4];
        let sz = u32::from_le_bytes([b[pos + 4], b[pos + 5], b[pos + 6], b[pos + 7]]) as usize;
        let body_start = pos + 8;
        let body_end = body_start.saturating_add(sz).min(b.len());
        let body = &b[body_start..body_end];
        if id == b"fmt " && body.len() >= 16 {
            let mut tag = u16::from_le_bytes([body[0], body[1]]);
            let ch = u16::from_le_bytes([body[2], body[3]]);
            let rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
            let bits = u16::from_le_bytes([body[14], body[15]]);
            if tag == 0xFFFE && body.len() >= 26 {
                tag = u16::from_le_bytes([body[24], body[25]]);
            }
            fmt = Some((tag, ch, rate, bits));
        } else if id == b"data" {
            data = Some(body);
            break;
        }
        pos = body_start.saturating_add(sz).saturating_add(sz & 1);
    }
    let (tag, ch, rate, bits) = fmt.ok_or_else(|| "WAV has no fmt chunk".to_string())?;
    let data = data.ok_or_else(|| "WAV has no data chunk".to_string())?;
    if ch == 0 || rate == 0 {
        return Err("WAV fmt chunk has zero channels or zero sample rate".to_string());
    }
    let bytes_per = (bits / 8) as usize;
    if bytes_per == 0 {
        return Err("WAV bit depth is zero".to_string());
    }
    let chn = ch as usize;
    let frame_bytes = chn * bytes_per;
    let frames = data.len() / frame_bytes;
    let float = tag == 3;
    if !(tag == 1 || tag == 3) || (tag == 1 && !matches!(bits, 8 | 16 | 24 | 32)) || (float && !matches!(bits, 32 | 64)) {
        return Err(format!("unsupported WAV encoding (format tag {}, {} bits)", tag, bits));
    }
    let mut channels: Vec<Vec<f32>> = vec![Vec::with_capacity(frames); chn];
    for f in 0..frames {
        for c in 0..chn {
            let o = f * frame_bytes + c * bytes_per;
            let s = &data[o..o + bytes_per];
            let v: f32 = if float {
                if bits == 32 {
                    f32::from_le_bytes([s[0], s[1], s[2], s[3]])
                } else {
                    f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]) as f32
                }
            } else {
                match bits {
                    8 => (s[0] as f32 - 128.0) / 128.0,
                    16 => i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0,
                    24 => {
                        let mut x = (s[0] as i32) | ((s[1] as i32) << 8) | ((s[2] as i32) << 16);
                        if x & 0x0080_0000 != 0 {
                            x -= 1 << 24;
                        }
                        x as f32 / 8_388_608.0
                    }
                    _ => i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32 / 2_147_483_648.0,
                }
            };
            channels[c].push(if v.is_finite() { v } else { 0.0 });
        }
    }
    Ok(Pcm {
        sample_rate: rate,
        bit_depth: bits,
        format: if float { "WAV/float".to_string() } else { "WAV/PCM".to_string() },
        channels,
    })
}

async fn load_pcm(a: &AudioData) -> Result<Pcm, String> {
    match a {
        AudioData::FilePath(p) => {
            let bytes = std::fs::read(p).map_err(|e| format!("cannot read {}: {}", p, e))?;
            parse_wav(&bytes)
        }
        AudioData::Base64 { data, .. } => parse_wav(&b64_decode(data)?),
        AudioData::Url(_) => Err("URL audio is not fetched by this pipeline (no network by design): download it with a tool and pass FilePath".to_string()),
        AudioData::Samples { data, sample_rate, channels } => {
            let ch = (*channels as usize).max(1);
            if *sample_rate == 0 {
                return Err("Samples input has sample_rate 0".to_string());
            }
            let frames = data.len() / ch;
            let mut out: Vec<Vec<f32>> = vec![Vec::with_capacity(frames); ch];
            for f in 0..frames {
                for c in 0..ch {
                    let v = data[f * ch + c];
                    out[c].push(if v.is_finite() { v } else { 0.0 });
                }
            }
            Ok(Pcm { sample_rate: *sample_rate, bit_depth: 32, format: "samples/f32".to_string(), channels: out })
        }
    }
}

/// In-place radix-2 FFT (length must be a power of two).
fn fft_inplace(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    if n < 2 {
        return;
    }
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        let ang = -2.0 * std::f64::consts::PI / (len as f64);
        let half = len / 2;
        let mut i = 0usize;
        while i < n {
            for k in 0..half {
                let a = ang * (k as f64);
                let (wr, wi) = (a.cos(), a.sin());
                let (ur, ui) = (re[i + k], im[i + k]);
                let (xr, xi) = (re[i + k + half], im[i + k + half]);
                let (vr, vi) = (xr * wr - xi * wi, xr * wi + xi * wr);
                re[i + k] = ur + vr;
                im[i + k] = ui + vi;
                re[i + k + half] = ur - vr;
                im[i + k + half] = ui - vi;
            }
            i += len;
        }
        len <<= 1;
    }
}

fn hann(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * (i as f64) / (n as f64)).cos()).collect()
}

/// Magnitude spectrum (first nfft/2+1 bins) of one windowed frame, zero-padded.
fn frame_magnitudes(x: &[f32], start: usize, win: &[f64], nfft: usize) -> Vec<f64> {
    let mut re = vec![0.0f64; nfft];
    let mut im = vec![0.0f64; nfft];
    for (k, w) in win.iter().enumerate() {
        let idx = start + k;
        if idx < x.len() && k < nfft {
            re[k] = x[idx] as f64 * w;
        }
    }
    fft_inplace(&mut re, &mut im);
    (0..=nfft / 2).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt()).collect()
}

fn percentile(v: &[f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s: Vec<f64> = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((s.len() - 1) as f64 * p.clamp(0.0, 1.0)).round() as usize;
    s[idx]
}

fn to_db(x: f64) -> f64 {
    20.0 * (x + 1e-9).log10()
}

/// Frame-level features of the mono mix.
struct Features {
    win: usize,
    hop: usize,
    sr: u32,
    rms: Vec<f64>,
    centroid_hz: Vec<f64>,
    flux: Vec<f64>,
    chroma: [f64; 12],
    /// frames x SPEC_ROWS band energies in dB (flattened row-major by frame)
    band_db: Vec<f32>,
    band_edges_hz: Vec<f32>,
}

fn analyze_signal(pcm: &Pcm) -> Features {
    let mono = pcm.mono();
    let sr = pcm.sample_rate.max(1);
    let mut win = ((sr as f64 * 0.032) as usize).next_power_of_two().clamp(256, 4096);
    if mono.len() < win {
        win = mono.len().next_power_of_two().max(16);
    }
    let mut hop = win / 2;
    if hop == 0 {
        hop = 1;
    }
    let frames_needed = if mono.len() > win { (mono.len() - win) / hop + 1 } else { 1 };
    if frames_needed > MAX_ANALYSIS_FRAMES {
        hop = (mono.len() / MAX_ANALYSIS_FRAMES).max(1);
    }
    let nfft = win;
    let w = hann(win);
    let nyq = sr as f64 / 2.0;
    let f_lo = 50.0f64.min(nyq * 0.5);
    let f_hi = nyq.min(16_000.0).max(f_lo * 2.0);
    let band_edges: Vec<f64> = (0..=SPEC_ROWS)
        .map(|i| f_lo * (f_hi / f_lo).powf(i as f64 / SPEC_ROWS as f64))
        .collect();
    let bin_hz = sr as f64 / nfft as f64;
    let mut rms = Vec::new();
    let mut centroid = Vec::new();
    let mut flux = Vec::new();
    let mut chroma = [0.0f64; 12];
    let mut band_db: Vec<f32> = Vec::new();
    let mut prev: Option<Vec<f64>> = None;
    let mut start = 0usize;
    loop {
        let end = (start + win).min(mono.len());
        let seg = &mono[start.min(mono.len())..end];
        let ms = if seg.is_empty() { 0.0 } else { seg.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>() / seg.len() as f64 };
        rms.push(ms.sqrt());
        let mags = frame_magnitudes(&mono, start, &w, nfft);
        let total: f64 = mags.iter().sum();
        let cen = if total > 1e-12 { mags.iter().enumerate().map(|(k, m)| k as f64 * bin_hz * m).sum::<f64>() / total } else { 0.0 };
        centroid.push(cen);
        let fl = match &prev {
            Some(p) => mags.iter().zip(p.iter()).map(|(a, b)| ((1.0 + a).ln() - (1.0 + b).ln()).max(0.0)).sum::<f64>(),
            None => 0.0,
        };
        flux.push(fl);
        for (k, m) in mags.iter().enumerate().skip(1) {
            let f = k as f64 * bin_hz;
            if f >= 60.0 && f <= 5000.0 {
                let midi = 69.0 + 12.0 * (f / 440.0).log2();
                let pc = (midi.round() as i64).rem_euclid(12) as usize;
                chroma[pc] += *m;
            }
        }
        for r in 0..SPEC_ROWS {
            let (lo, hi) = (band_edges[r], band_edges[r + 1]);
            let k0 = (lo / bin_hz).floor() as usize;
            let k1 = ((hi / bin_hz).ceil() as usize).max(k0 + 1).min(mags.len());
            let k0 = k0.min(mags.len().saturating_sub(1));
            let e: f64 = mags[k0..k1.max(k0 + 1).min(mags.len())].iter().map(|m| m * m).sum::<f64>();
            band_db.push((10.0 * (e / (nfft as f64) + 1e-12).log10()) as f32);
        }
        prev = Some(mags);
        if end >= mono.len() {
            break;
        }
        start += hop;
    }
    Features {
        win,
        hop,
        sr,
        rms,
        centroid_hz: centroid,
        flux,
        chroma,
        band_db,
        band_edges_hz: band_edges.iter().map(|&e| e as f32).collect(),
    }
}

fn spectrogram_grid(f: &Features) -> SpectrogramGrid {
    let frames = f.rms.len();
    let cols = frames.min(SPEC_COLS).max(1);
    let mut values = vec![0u8; SPEC_ROWS * cols];
    let db_floor = -100.0f32;
    for c in 0..cols {
        let a = c * frames / cols;
        let b = ((c + 1) * frames / cols).max(a + 1).min(frames);
        for r in 0..SPEC_ROWS {
            let mut s = 0.0f32;
            for fr in a..b {
                s += f.band_db[fr * SPEC_ROWS + r];
            }
            let db = (s / (b - a) as f32).clamp(db_floor, 0.0);
            values[r * cols + c] = (((db - db_floor) / -db_floor) * 255.0).round() as u8;
        }
    }
    SpectrogramGrid {
        frame: String::new(),
        rows: SPEC_ROWS,
        cols,
        hop_s: f.hop as f32 / f.sr as f32 * (frames as f32 / cols as f32),
        win_s: f.win as f32 / f.sr as f32,
        fmax_hz: *f.band_edges_hz.last().unwrap_or(&0.0),
        db_floor,
        band_edges_hz: f.band_edges_hz.clone(),
        values,
    }
}

fn biquad(x: &[f32], b: [f64; 3], a: [f64; 2]) -> Vec<f64> {
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut out = Vec::with_capacity(x.len());
    for &s in x {
        let x0 = s as f64;
        let y0 = b[0] * x0 + b[1] * x1 + b[2] * x2 - a[0] * y1 - a[1] * y2;
        x2 = x1;
        x1 = x0;
        y2 = y1;
        y1 = y0;
        out.push(y0);
    }
    out
}

/// ITU-R BS.1770 K-weighting (coefficients derived for any sample rate, libebur128 form).
fn k_weight(x: &[f32], fs: f64) -> Vec<f64> {
    let (f0, g, q) = (1681.974_450_955_531_9f64, 3.999_843_853_973_347f64, 0.707_175_236_955_419_6f64);
    let k = (std::f64::consts::PI * f0 / fs).tan();
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.499_666_774_154_541_6);
    let a0 = 1.0 + k / q + k * k;
    let b = [(vh + vb * k / q + k * k) / a0, 2.0 * (k * k - vh) / a0, (vh - vb * k / q + k * k) / a0];
    let a = [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0];
    let stage1 = biquad(x, b, a);
    let (f0, q) = (38.135_470_876_139_82f64, 0.500_327_037_325_395_3f64);
    let k = (std::f64::consts::PI * f0 / fs).tan();
    let a0 = 1.0 + k / q + k * k;
    let a = [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0];
    let stage1_f32: Vec<f32> = stage1.iter().map(|&v| v as f32).collect();
    // RLB high-pass: numerator 1,-2,1 (unnormalised, per libebur128), denominator normalised.
    biquad(&stage1_f32, [1.0, -2.0, 1.0], a)
}

/// Integrated loudness (LUFS) per BS.1770-4 with absolute (-70) and relative (-10 LU) gating.
/// Channel weights are 1.0 for all channels (surround weighting is not applied). None when
/// the signal is shorter than one 400 ms block or entirely gated out.
fn integrated_lufs(pcm: &Pcm) -> Option<f64> {
    let fs = pcm.sample_rate as f64;
    let block = (0.4 * fs) as usize;
    let step = ((0.1 * fs) as usize).max(1);
    if block == 0 || pcm.frames() < block {
        return None;
    }
    let kw: Vec<Vec<f64>> = pcm.channels.iter().map(|c| k_weight(c, fs)).collect();
    let mut powers: Vec<f64> = Vec::new();
    let mut s = 0usize;
    while s + block <= pcm.frames() {
        let mut sum = 0.0;
        for ch in &kw {
            sum += ch[s..s + block].iter().map(|v| v * v).sum::<f64>() / block as f64;
        }
        powers.push(sum);
        s += step;
    }
    let loud = |p: f64| -0.691 + 10.0 * p.max(1e-12).log10();
    let abs_gated: Vec<f64> = powers.iter().copied().filter(|&p| loud(p) > -70.0).collect();
    if abs_gated.is_empty() {
        return None;
    }
    let mean_abs = abs_gated.iter().sum::<f64>() / abs_gated.len() as f64;
    let rel_thr = loud(mean_abs) - 10.0;
    let rel_gated: Vec<f64> = abs_gated.iter().copied().filter(|&p| loud(p) > rel_thr).collect();
    if rel_gated.is_empty() {
        return None;
    }
    Some(loud(rel_gated.iter().sum::<f64>() / rel_gated.len() as f64))
}

fn zero_crossing_rate(x: &[f32]) -> f64 {
    if x.len() < 2 {
        return 0.0;
    }
    let z = x.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count();
    z as f64 / (x.len() - 1) as f64
}

fn rms_of(x: &[f32]) -> f64 {
    if x.is_empty() {
        0.0
    } else {
        (x.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>() / x.len() as f64).sqrt()
    }
}

/// Activity segmentation from frame RMS. Returns (segments, threshold_db, noise_floor_db, peak_db).
fn activity_segments(f: &Features, duration: f32) -> (Vec<AudioSegment>, f64, f64, f64) {
    let dbs: Vec<f64> = f.rms.iter().map(|&r| to_db(r)).collect();
    let floor = percentile(&dbs, 0.10);
    let peak = percentile(&dbs, 0.95);
    let threshold = if peak - floor < 10.0 { (peak - 3.0).max(-60.0).min(peak + 1.0) } else { (floor + 10.0).max(-60.0) };
    let active: Vec<bool> = dbs.iter().map(|&d| d > threshold && d > -60.0).collect();
    let hop_s = f.hop as f64 / f.sr as f64;
    let min_gap = ((0.15 / hop_s).ceil() as usize).max(1);
    let min_run = ((0.1 / hop_s).ceil() as usize).max(1);
    // merge short inactive gaps between active frames, drop very short active runs
    let mut state = active.clone();
    let mut i = 0usize;
    while i < state.len() {
        if !state[i] {
            let mut j = i;
            while j < state.len() && !state[j] {
                j += 1;
            }
            if i > 0 && j < state.len() && (j - i) < min_gap {
                for s in state.iter_mut().take(j).skip(i) {
                    *s = true;
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    let mut runs: Vec<(bool, usize, usize)> = Vec::new();
    for (idx, &s) in state.iter().enumerate() {
        match runs.last_mut() {
            Some(last) if last.0 == s => last.2 = idx,
            _ => runs.push((s, idx, idx)),
        }
    }
    for r in runs.iter_mut() {
        if r.0 && (r.2 - r.1 + 1) < min_run {
            r.0 = false;
        }
    }
    let mut merged: Vec<(bool, usize, usize)> = Vec::new();
    for r in runs {
        match merged.last_mut() {
            Some(last) if last.0 == r.0 => last.2 = r.2,
            _ => merged.push(r),
        }
    }
    let n = merged.len();
    let mut segs = Vec::new();
    for (k, (act, a, b)) in merged.iter().enumerate() {
        let start = if k == 0 { 0.0 } else { (*a as f64 * hop_s) as f32 };
        let end = if k + 1 == n { duration } else { (((*b + 1) as f64) * hop_s).min(duration as f64) as f32 };
        let mean_rms = f.rms[*a..=*b].iter().sum::<f64>() / (*b - *a + 1) as f64;
        let seg_db = to_db(mean_rms);
        let conf = ((seg_db - threshold).abs() / 20.0).clamp(0.0, 1.0) as f32;
        segs.push(AudioSegment {
            segment_id: format!("seg_{}", k + 1),
            start_time: start,
            end_time: end.max(start),
            segment_type: if *act { SegmentType::Unknown } else { SegmentType::Silence },
            speaker_id: None,
            confidence: conf,
            energy: mean_rms as f32,
            label: Some(if *act {
                "active sound (speech/music classification not run)".to_string()
            } else {
                "below activity threshold".to_string()
            }),
        });
    }
    (segs, threshold, floor, peak)
}

/// Tempo (autocorrelation of onset strength), beats and key (chroma vs Krumhansl-Kessler).
fn music_from_features(f: &Features, duration: f32, want_beats: bool) -> MusicAnalysis {
    let mut absent: HashMap<String, String> = HashMap::new();
    let hop_s = f.hop as f64 / f.sr as f64;
    let mean = if f.flux.is_empty() { 0.0 } else { f.flux.iter().sum::<f64>() / f.flux.len() as f64 };
    let x: Vec<f64> = f.flux.iter().map(|v| v - mean).collect();
    let r0: f64 = x.iter().map(|v| v * v).sum();
    let lag_min = ((60.0 / 200.0 / hop_s).floor() as usize).max(1);
    let lag_max = (60.0 / 60.0 / hop_s).ceil() as usize;
    let (mut tempo, mut tempo_conf) = (0.0f64, 0.0f64);
    let mut beats: Vec<f32> = Vec::new();
    if duration >= 4.0 && r0 > 1e-9 && lag_max + 2 < x.len() {
        let r = |l: usize| -> f64 { (0..x.len() - l).map(|i| x[i] * x[i + l]).sum::<f64>() };
        let mut best = lag_min;
        let mut best_v = f64::MIN;
        for l in lag_min..=lag_max {
            let v = r(l);
            if v > best_v {
                best_v = v;
                best = l;
            }
        }
        let (rm, r0v, rp) = (r(best - 1), r(best), r(best + 1));
        let denom = rm - 2.0 * r0v + rp;
        let delta = if denom.abs() > 1e-12 { (0.5 * (rm - rp) / denom).clamp(-0.5, 0.5) } else { 0.0 };
        let period = best as f64 + delta;
        tempo = 60.0 / (period * hop_s);
        tempo_conf = (best_v / r0).clamp(0.0, 1.0);
        if want_beats {
            let mut best_phase = 0.0f64;
            let mut best_score = f64::MIN;
            let mut ph = 0.0f64;
            while ph < period {
                let mut sc = 0.0;
                let mut t = ph;
                while (t.round() as usize) < f.flux.len() {
                    sc += f.flux[t.round() as usize];
                    t += period;
                }
                if sc > best_score {
                    best_score = sc;
                    best_phase = ph;
                }
                ph += 0.5;
            }
            let mut t = best_phase;
            while (t.round() as usize) < f.flux.len() && beats.len() < 5000 {
                beats.push((t * hop_s) as f32);
                t += period;
            }
        }
    } else {
        absent.insert("tempo".to_string(), "needs >= 4 s of audio with onset energy".to_string());
        if want_beats {
            absent.insert("beats".to_string(), "no tempo estimate, so no beat grid".to_string());
        }
    }
    let (key, mode, key_conf) = estimate_key(&f.chroma);
    if key.is_empty() {
        absent.insert("key".to_string(), "no tonal energy between 60 Hz and 5 kHz".to_string());
    }
    absent.insert("time_signature".to_string(), "meter detection is not implemented".to_string());
    absent.insert("downbeats".to_string(), "needs meter detection, which is not implemented".to_string());
    absent.insert("sections".to_string(), "structure segmentation is not implemented".to_string());
    absent.insert("chords".to_string(), "no chord-recognition model in this pipeline".to_string());
    absent.insert("melody".to_string(), "no pitch-tracking model in this pipeline".to_string());
    absent.insert("genre".to_string(), "no genre classifier in this pipeline".to_string());
    absent.insert("instruments".to_string(), "no instrument classifier in this pipeline".to_string());
    absent.insert("valence_curve".to_string(), "no affect model in this pipeline".to_string());
    // energy curve: mean RMS in ~1 s bins, at most 600 points
    let bin_s = (duration as f64 / 600.0).max(1.0);
    let mut energy: Vec<(f32, f32)> = Vec::new();
    let mut t = 0.0f64;
    while t < duration as f64 {
        let a = (t / hop_s) as usize;
        let b = (((t + bin_s) / hop_s) as usize).max(a + 1).min(f.rms.len());
        if a < f.rms.len() {
            let m = f.rms[a..b].iter().sum::<f64>() / (b - a) as f64;
            energy.push((t as f32, m as f32));
        }
        t += bin_s;
    }
    MusicAnalysis {
        tempo_bpm: tempo as f32,
        tempo_confidence: tempo_conf as f32,
        key,
        key_confidence: key_conf as f32,
        mode,
        time_signature: String::new(),
        beats,
        downbeats: vec![],
        sections: vec![],
        chords: vec![],
        melody: None,
        genre: None,
        instruments: vec![],
        energy_curve: energy,
        valence_curve: vec![],
        absent,
    }
}

fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut num, mut da, mut db) = (0.0, 0.0, 0.0);
    for i in 0..a.len() {
        num += (a[i] - ma) * (b[i] - mb);
        da += (a[i] - ma).powi(2);
        db += (b[i] - mb).powi(2);
    }
    if da <= 1e-12 || db <= 1e-12 {
        0.0
    } else {
        num / (da.sqrt() * db.sqrt())
    }
}

/// (key name, mode, correlation). Empty key when the chroma vector carries no energy.
fn estimate_key(chroma: &[f64; 12]) -> (String, String, f64) {
    if chroma.iter().sum::<f64>() <= 1e-9 {
        return (String::new(), String::new(), 0.0);
    }
    let mut best = (String::new(), String::new(), f64::MIN);
    for tonic in 0..12 {
        for (mode, prof) in [("major", &KK_MAJOR), ("minor", &KK_MINOR)] {
            let rotated: Vec<f64> = (0..12).map(|i| prof[(i + 12 - tonic) % 12]).collect();
            let c = pearson(chroma, &rotated);
            if c > best.2 {
                best = (NOTE_NAMES[tonic].to_string(), mode.to_string(), c);
            }
        }
    }
    (best.0, best.1, best.2.clamp(0.0, 1.0))
}

fn detect_signal_events(pcm: &Pcm, f: &Features, types: &[AudioEventType]) -> (Vec<AudioEvent>, Vec<String>) {
    let sr = pcm.sample_rate as f64;
    let mut events = Vec::new();
    let mut unsupported = Vec::new();
    let n = pcm.frames();
    for t in types {
        match t {
            AudioEventType::Clipping => {
                let mut i = 0usize;
                while i < n {
                    if pcm.channels.iter().any(|c| c[i].abs() >= 0.9995) {
                        let a = i;
                        while i < n && pcm.channels.iter().any(|c| c[i].abs() >= 0.9995) {
                            i += 1;
                        }
                        if i - a >= 3 {
                            let mut props = HashMap::new();
                            props.insert("samples".to_string(), Value::from((i - a) as u64));
                            events.push(AudioEvent {
                                event_type: AudioEventType::Clipping,
                                start_time: (a as f64 / sr) as f32,
                                end_time: (i as f64 / sr) as f32,
                                confidence: 1.0,
                                description: Some("run of full-scale samples (measured)".to_string()),
                                properties: props,
                            });
                        }
                    } else {
                        i += 1;
                    }
                }
            }
            AudioEventType::Dropout => {
                let min_len = ((0.02 * sr) as usize).max(2);
                let first_nz = (0..n).find(|&i| pcm.channels.iter().any(|c| c[i] != 0.0));
                let last_nz = (0..n).rev().find(|&i| pcm.channels.iter().any(|c| c[i] != 0.0));
                if let (Some(lo), Some(hi)) = (first_nz, last_nz) {
                    let mut i = lo;
                    while i < hi {
                        if pcm.channels.iter().all(|c| c[i] == 0.0) {
                            let a = i;
                            while i < hi && pcm.channels.iter().all(|c| c[i] == 0.0) {
                                i += 1;
                            }
                            if i - a >= min_len {
                                events.push(AudioEvent {
                                    event_type: AudioEventType::Dropout,
                                    start_time: (a as f64 / sr) as f32,
                                    end_time: (i as f64 / sr) as f32,
                                    confidence: 1.0,
                                    description: Some("run of exact digital zeros inside the signal (measured)".to_string()),
                                    properties: HashMap::new(),
                                });
                            }
                        } else {
                            i += 1;
                        }
                    }
                }
            }
            AudioEventType::NoiseSpike => {
                let hop_s = f.hop as f64 / sr;
                let back = ((1.0 / hop_s).ceil() as usize).max(4);
                let dbs: Vec<f64> = f.rms.iter().map(|&r| to_db(r)).collect();
                let mut i = back;
                while i < dbs.len() {
                    let med = percentile(&dbs[i - back..i], 0.5);
                    let excess = dbs[i] - med;
                    if excess > 15.0 && dbs[i] > -50.0 {
                        let a = i;
                        while i < dbs.len() && dbs[i] - med > 15.0 {
                            i += 1;
                        }
                        let mut props = HashMap::new();
                        props.insert("excess_db".to_string(), Value::from(excess));
                        events.push(AudioEvent {
                            event_type: AudioEventType::NoiseSpike,
                            start_time: (a as f64 * hop_s) as f32,
                            end_time: ((i as f64) * hop_s).min(pcm.duration() as f64) as f32,
                            confidence: (excess / 30.0).clamp(0.0, 1.0) as f32,
                            description: Some("frame level > 15 dB above the previous 1 s median (measured)".to_string()),
                            properties: props,
                        });
                    } else {
                        i += 1;
                    }
                }
            }
            other => unsupported.push(format!("{:?}: classification needs a model; this pipeline only measures Clipping, Dropout and NoiseSpike", other)),
        }
    }
    (events, unsupported)
}

async fn analyze_audio(
    audio_data: &AudioData,
    _depth: AnalysisDepth,
    transcribe: bool,
    diarize: bool,
    detect_music: bool,
    detect_events: bool,
) -> Result<AudioAnalysisResult, String> {
    let pcm = load_pcm(audio_data).await?;
    if pcm.frames() == 0 {
        return Err("audio has no samples".to_string());
    }
    let feats = analyze_signal(&pcm);
    let duration = pcm.duration();
    let (segments, threshold_db, noise_floor_db, peak_db) = activity_segments(&feats, duration);
    let mut absent: HashMap<String, String> = HashMap::new();
    let mut metadata: HashMap<String, Value> = HashMap::new();

    // per-channel measurements
    let mut per_channel = Vec::new();
    let mut clipped_total: u64 = 0;
    for (ci, ch) in pcm.channels.iter().enumerate() {
        let peak = ch.iter().fold(0.0f32, |m, &s| m.max(s.abs())) as f64;
        let clipped = ch.iter().filter(|&&s| s.abs() >= 0.9995).count() as u64;
        clipped_total += clipped;
        per_channel.push(serde_json::json!({
            "channel": ci,
            "rms_dbfs": to_db(rms_of(ch)),
            "peak_dbfs": to_db(peak),
            "zero_crossing_rate": zero_crossing_rate(ch),
            "clipped_samples": clipped,
        }));
    }
    metadata.insert("per_channel".to_string(), Value::from(per_channel));
    let total_samples = (pcm.frames() * pcm.channels.len()).max(1) as f64;
    let clipping_percent = 100.0 * clipped_total as f64 / total_samples;

    // frame-level statistics
    let dbs: Vec<f64> = feats.rms.iter().map(|&r| to_db(r)).collect();
    let active_dbs: Vec<f64> = dbs.iter().copied().filter(|&d| d > -60.0).collect();
    let dynamic_range = if active_dbs.len() >= 3 {
        percentile(&active_dbs, 0.95) - percentile(&active_dbs, 0.10)
    } else {
        absent.insert("dynamic_range_db".to_string(), "fewer than 3 frames above -60 dBFS".to_string());
        0.0
    };
    metadata.insert(
        "dynamic_range_method".to_string(),
        Value::from("p95 - p10 of frame RMS (dBFS) over frames above -60 dBFS"),
    );
    let active_cent: Vec<f64> = feats
        .centroid_hz
        .iter()
        .zip(dbs.iter())
        .filter(|p| *p.1 > threshold_db)
        .map(|p| *p.0)
        .collect();
    if active_cent.is_empty() {
        absent.insert("spectral_centroid_hz_mean".to_string(), "no active frames".to_string());
    } else {
        metadata.insert(
            "spectral_centroid_hz_mean".to_string(),
            Value::from(active_cent.iter().sum::<f64>() / active_cent.len() as f64),
        );
    }
    metadata.insert(
        "analysis".to_string(),
        serde_json::json!({
            "win_samples": feats.win, "hop_samples": feats.hop, "frames": feats.rms.len(),
            "activity_threshold_dbfs": threshold_db, "noise_floor_dbfs": noise_floor_db, "p95_dbfs": peak_db,
        }),
    );

    // loudness
    let loudness = match integrated_lufs(&pcm) {
        Some(l) => l,
        None => {
            absent.insert(
                "loudness_lufs".to_string(),
                "needs at least one 400 ms block above the BS.1770 absolute gate (-70 LUFS)".to_string(),
            );
            0.0
        }
    };
    metadata.insert(
        "loudness_method".to_string(),
        Value::from("ITU-R BS.1770-4 K-weighted, gated (-70 LUFS absolute, -10 LU relative), all channel weights 1.0"),
    );

    // stereo level difference: a weak spatial cue, NOT a source localization
    if pcm.channels.len() == 2 {
        let (rl, rr) = (rms_of(&pcm.channels[0]), rms_of(&pcm.channels[1]));
        metadata.insert(
            "stereo".to_string(),
            serde_json::json!({
                "ild_db": 20.0 * ((rl + 1e-12) / (rr + 1e-12)).log10(),
                "pan": (rr - rl) / (rr + rl + 1e-12),
                "method": "full-band RMS level difference between left and right",
                "caveat": "weak spatial cue only; real source localization needs a microphone array (see the sound modality)",
            }),
        );
    }

    // quality
    let snr = if peak_db - noise_floor_db >= 10.0 {
        let act: Vec<f64> = feats.rms.iter().zip(dbs.iter()).filter(|p| *p.1 > threshold_db).map(|p| p.0 * p.0).collect();
        let quiet: Vec<f64> = feats.rms.iter().zip(dbs.iter()).filter(|p| *p.1 <= noise_floor_db + 3.0).map(|p| p.0 * p.0).collect();
        if act.is_empty() || quiet.is_empty() {
            absent.insert("snr_db".to_string(), "no active/quiet reference frames".to_string());
            0.0
        } else {
            10.0 * ((act.iter().sum::<f64>() / act.len() as f64) / (quiet.iter().sum::<f64>() / quiet.len() as f64 + 1e-18)).log10()
        }
    } else {
        absent.insert("snr_db".to_string(), "stationary signal: no quiet reference frames to estimate noise".to_string());
        0.0
    };
    let mut issues = Vec::new();
    if clipping_percent > 0.01 {
        issues.push(QualityIssue {
            issue_type: "clipping".to_string(),
            severity: (clipping_percent / 1.0).clamp(0.0, 1.0) as f32,
            start_time: None,
            end_time: None,
            description: format!("{:.3}% of samples are at full scale", clipping_percent),
        });
    }
    absent.insert("quality.overall_score".to_string(), "no calibrated quality model; use the individual measurements".to_string());
    absent.insert("quality.speech_clarity".to_string(), "no speech-intelligibility model".to_string());
    absent.insert("quality.music_quality".to_string(), "no music-quality model".to_string());

    // requested capabilities this pipeline does not run (model work belongs to tools)
    if transcribe {
        absent.insert("transcription".to_string(), ABSENT_ASR.to_string());
    }
    if diarize {
        absent.insert("diarization".to_string(), ABSENT_DIARIZATION.to_string());
    }
    absent.insert("segment_type".to_string(), "segments are signal-activity only (silence vs active); speech/music/noise classification needs a model".to_string());

    let music_analysis = if detect_music { Some(music_from_features(&feats, duration, true)) } else { None };
    let audio_events = if detect_events {
        let (ev, unsup) = detect_signal_events(&pcm, &feats, &[AudioEventType::Clipping, AudioEventType::Dropout, AudioEventType::NoiseSpike]);
        for u in unsup {
            absent.insert("events".to_string(), u);
        }
        ev
    } else {
        vec![]
    };

    Ok(AudioAnalysisResult {
        duration_seconds: duration,
        sample_rate: pcm.sample_rate,
        channels: pcm.channels.len().min(255) as u8,
        format: pcm.format.clone(),
        bit_depth: pcm.bit_depth,
        segments,
        speakers: vec![],
        transcription: None,
        music_analysis,
        audio_events,
        quality: AudioQuality {
            snr_db: snr as f32,
            clipping_percent: clipping_percent as f32,
            noise_floor_db: noise_floor_db as f32,
            speech_clarity: None,
            music_quality: None,
            overall_score: 0.0,
            issues,
        },
        loudness_lufs: loudness as f32,
        dynamic_range_db: dynamic_range as f32,
        metadata,
        spectrogram: Some(spectrogram_grid(&feats)),
        absent,
    })
}

async fn transcribe_audio(
    _audio_data: &AudioData,
    language: Option<String>,
    _word_timestamps: bool,
    _speaker_labels: bool,
) -> Result<Transcription, String> {
    // Honesty rule: ASR is model work done by a tool, not canned text.
    Ok(Transcription {
        text: String::new(),
        language: language.unwrap_or_default(),
        language_confidence: 0.0,
        words: vec![],
        sentences: vec![],
        confidence: 0.0,
        alternatives: vec![],
        absent_reason: Some(ABSENT_ASR.to_string()),
    })
}

async fn diarize_audio(
    _audio_data: &AudioData,
    _num_speakers: Option<u32>,
    _min_speakers: Option<u32>,
    _max_speakers: Option<u32>,
) -> Result<DiarizationResult, String> {
    Ok(DiarizationResult {
        speakers: vec![],
        turns: vec![],
        num_speakers: 0,
        confidence: 0.0,
        absent_reason: Some(ABSENT_DIARIZATION.to_string()),
    })
}

async fn analyze_music(
    audio_data: &AudioData,
    extract_beats: bool,
    _extract_chords: bool,
    _extract_melody: bool,
) -> Result<MusicAnalysis, String> {
    let pcm = load_pcm(audio_data).await?;
    if pcm.frames() == 0 {
        return Err("audio has no samples".to_string());
    }
    let feats = analyze_signal(&pcm);
    Ok(music_from_features(&feats, pcm.duration(), extract_beats))
}

// ============================================================================
// ZSEI OVER HTTP + PERSISTENCE (the text/code/math/image revival pattern)
// ============================================================================

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let host = env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let client = reqwest::Client::new();
    let body = serde_json::json!({"query": query, "session_token": ""});
    let resp = client
        .post(format!("{host}/zsei/query")).header("x-ozone-trace", std::env::var("OZONE_TRACE_ID").unwrap_or_default()).header("x-ozone-origin", std::env::var("OZONE_ORIGIN").unwrap_or_default())
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("ZSEI unreachable: {e}"))?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
        Ok(v.get("result").cloned().unwrap_or(serde_json::Value::Null))
    } else {
        Err(v.get("error").and_then(|e| e.as_str()).unwrap_or("zsei query failed").to_string())
    }
}

async fn persist_audio_graph(graph: &mut AudioGraph) -> Result<(), String> {
    let keywords: Vec<String> = {
        let mut k: Vec<String> = graph.nodes.iter()
            .filter_map(|n| n.label.to_lowercase().split_whitespace().next().map(String::from))
            .collect();
        k.sort(); k.dedup();
        if k.is_empty() { k.push("audio".to_string()); }
        // container keyword set kept WHOLE (no 12-keyword cut — doctrine audit #33, 2026-11-04)
        k
    };
    let name = format!(
        "Audio graph ({} nodes, {} edges)",
        graph.nodes.len(), graph.edges.len()
    );
    let now = chrono::Utc::now().timestamp() as u64;
    let container = serde_json::json!({
        "global_state": {"container_id": 0, "parent_id": graph.project_id, "child_ids": [], "child_count": 0, "version": 1},
        "local_state": {
            "metadata": {"container_type": "ModalityGraph", "modality": "Unknown", "created_at": now,
                "updated_at": now, "provenance": "audio-pipeline", "permissions": 0, "owner_id": 0,
                "name": name, "materialized_path": null},
            "context": {"categories": [], "methodologies": [], "keywords": keywords,
                "topics": ["audio"], "relationships": [], "learned_associations": [], "embedding": null},
            "storage": {"db_shard_id": null, "vector_index_ref": null,
                "object_store_path": null, "compression_type": "None"},
            "hints": {"access_frequency": 0, "hotness_score": 0.0, "last_accessed": 0, "centroid": null, "ml_prediction_weight": 0.0},
            "integrity": {"content_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                "semantic_fingerprint": [], "last_verified": 0, "integrity_score": 1.0, "version_history": []},
            "file_context": null, "code_context": null, "text_context": null, "external_ref": null
        }
    });
    let result = zsei_query(serde_json::json!({
        "CreateContainer": { "parent_id": graph.project_id, "container": container }
    })).await?;
    let new_id = result.get("ContainerID").and_then(|c| c.as_u64())
        .ok_or("CreateContainer returned no ContainerID")?;
    let dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let rel = format!("graphs/audio_{}.json", new_id);
    let abs = std::path::Path::new(&dir).join(&rel);
    if let Some(parent) = abs.parent() { let _ = std::fs::create_dir_all(parent); }
    std::fs::write(&abs, serde_json::to_string_pretty(&serde_json::json!({
        "graph_id": new_id, "nodes": graph.nodes, "edges": graph.edges
    })).map_err(|e| e.to_string())?).map_err(|e| format!("graph content write failed: {e}"))?;
    let _ = zsei_query(serde_json::json!({
        "UpdateContainer": { "container_id": new_id, "updates": { "storage": {
            "db_shard_id": null, "vector_index_ref": null, "object_store_path": rel,
            "compression_type": "None" }, "metadata": null, "context": null, "hints": null } }
    })).await;
    graph.graph_id = new_id;
    Ok(())
}

async fn create_graph(
    analysis: AudioAnalysisResult,
    project_id: u64,
    graph_name: Option<String>,
) -> Result<AudioGraph, String> {
    let graph_id = generate_graph_id();
    let now = chrono::Utc::now().to_rfc3339();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_id_counter: u64 = 1;

    // Create root audio node
    let audio_node_id = node_id_counter;
    nodes.push(AudioGraphNode {
        node_id: audio_node_id,
        node_type: AudioNodeType::Audio,
        label: "Audio".to_string(),
        content: format!(
            "{:.1}s {} {}Hz",
            analysis.duration_seconds, analysis.format, analysis.sample_rate
        ),
        time_range: Some(TimeRange {
            start: 0.0,
            end: analysis.duration_seconds,
        }),
        confidence: 1.0,
        properties: {
            let mut props = HashMap::new();
            props.insert(
                "duration".to_string(),
                Value::from(analysis.duration_seconds),
            );
            props.insert("sample_rate".to_string(), Value::from(analysis.sample_rate));
            props.insert("channels".to_string(), Value::from(analysis.channels));
            props.insert("format".to_string(), Value::from(analysis.format.clone()));
            // GEOMETRY LAYER: audio is a 1-D Span in frame time:<graph_id> (seconds).
            props.insert(
                "geom".to_string(),
                serde_json::json!({"frame": format!("time:{}", graph_id), "kind": "span", "start": 0.0, "end": analysis.duration_seconds}),
            );
            props
        },
        annotations: vec![],
    });
    node_id_counter += 1;

    // Spectrogram grid node (time-frequency geometry), bounded by SPEC_ROWS x SPEC_COLS.
    if let Some(grid) = &analysis.spectrogram {
        let spec_node_id = node_id_counter;
        let mut props = HashMap::new();
        props.insert(
            "geom".to_string(),
            serde_json::json!({
                "frame": format!("time:{}", graph_id), "kind": "grid",
                "rows": grid.rows, "cols": grid.cols, "hop_s": grid.hop_s, "win_s": grid.win_s,
                "fmax_hz": grid.fmax_hz, "db_floor": grid.db_floor, "row_axis": "frequency (log-spaced bands, row 0 lowest)",
            }),
        );
        props.insert("band_edges_hz".to_string(), serde_json::json!(grid.band_edges_hz));
        props.insert("values_u8_row_major".to_string(), serde_json::json!(grid.values));
        nodes.push(AudioGraphNode {
            node_id: spec_node_id,
            node_type: AudioNodeType::Spectrogram,
            label: "Spectrogram".to_string(),
            content: format!("{}x{} band-energy grid", grid.rows, grid.cols),
            time_range: Some(TimeRange { start: 0.0, end: analysis.duration_seconds }),
            confidence: 1.0,
            properties: props,
            annotations: vec![],
        });
        edges.push(AudioGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: audio_node_id,
            to_node: spec_node_id,
            edge_type: AudioEdgeType::Contains,
            weight: 1.0,
            properties: HashMap::new(),
        });
        node_id_counter += 1;
    }

    // Create segment nodes
    for segment in &analysis.segments {
        let seg_node_id = node_id_counter;
        nodes.push(AudioGraphNode {
            node_id: seg_node_id,
            node_type: AudioNodeType::Segment,
            label: format!("{:?}", segment.segment_type),
            content: segment
                .label
                .clone()
                .unwrap_or_else(|| format!("{:?} segment", segment.segment_type)),
            time_range: Some(TimeRange {
                start: segment.start_time,
                end: segment.end_time,
            }),
            confidence: segment.confidence,
            properties: {
                let mut props = HashMap::new();
                props.insert("energy".to_string(), Value::from(segment.energy));
                props.insert(
                    "geom".to_string(),
                    serde_json::json!({"frame": format!("time:{}", graph_id), "kind": "span", "start": segment.start_time, "end": segment.end_time}),
                );
                if let Some(speaker) = &segment.speaker_id {
                    props.insert("speaker_id".to_string(), Value::from(speaker.clone()));
                }
                props
            },
            annotations: vec![],
        });

        edges.push(AudioGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: audio_node_id,
            to_node: seg_node_id,
            edge_type: AudioEdgeType::Contains,
            weight: segment.confidence,
            properties: HashMap::new(),
        });

        // LIVE NODE EVENT (2026-10-19, additive — same pattern as 3D's
        // first real producer). Fire-and-forget, never awaited inline.
        {
            let host = env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
            let pid = project_id;
            let gid = graph_id;
            let nid = seg_node_id;
            let type_name = format!("{:?}", segment.segment_type);
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let _ = client.post(format!("{host}/orchestration/node-event"))
                    .json(&serde_json::json!({
                        "project_id": pid,
                        "modality": "audio",
                        "graph_id": gid,
                        "node_id": nid,
                        "related_node_id": serde_json::Value::Null,
                        "node_type": format!("Segment:{}", type_name),
                        "operation": "node_created",
                    }))
                    .timeout(std::time::Duration::from_secs(2))
                    .send().await;
            });
        }

        node_id_counter += 1;
    }

    // Create speaker nodes
    for speaker in &analysis.speakers {
        let speaker_node_id = node_id_counter;
        nodes.push(AudioGraphNode {
            node_id: speaker_node_id,
            node_type: AudioNodeType::Speaker,
            label: speaker
                .label
                .clone()
                .unwrap_or_else(|| speaker.speaker_id.clone()),
            content: format!(
                "Speaking time: {:.1}s, Turns: {}",
                speaker.total_speaking_time, speaker.turn_count
            ),
            time_range: None,
            confidence: 1.0,
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "total_speaking_time".to_string(),
                    Value::from(speaker.total_speaking_time),
                );
                props.insert("turn_count".to_string(), Value::from(speaker.turn_count));
                props
            },
            annotations: vec![],
        });

        // Link speaker to their segments
        for seg_id in &speaker.segments {
            if let Some(seg_node) = nodes.iter().find(|n| {
                n.node_type == AudioNodeType::Segment
                    && n.properties
                        .get("speaker_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s == speaker.speaker_id.as_str())
                        .unwrap_or(false)
            }) {
                edges.push(AudioGraphEdge {
                    edge_id: edges.len() as u64 + 1,
                    from_node: seg_node.node_id,
                    to_node: speaker_node_id,
                    edge_type: AudioEdgeType::SpokenBy,
                    weight: 1.0,
                    properties: HashMap::new(),
                });
            }
        }

        node_id_counter += 1;
    }

    // Create transcription nodes if available
    if let Some(transcription) = &analysis.transcription {
        for sentence in &transcription.sentences {
            let sent_node_id = node_id_counter;
            nodes.push(AudioGraphNode {
                node_id: sent_node_id,
                node_type: AudioNodeType::Sentence,
                label: "Sentence".to_string(),
                content: sentence.text.clone(),
                time_range: Some(TimeRange {
                    start: sentence.start_time,
                    end: sentence.end_time,
                }),
                confidence: sentence.confidence,
                properties: HashMap::new(),
                annotations: vec![],
            });

            edges.push(AudioGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: audio_node_id,
                to_node: sent_node_id,
                edge_type: AudioEdgeType::TranscribedAs,
                weight: sentence.confidence,
                properties: HashMap::new(),
            });

            node_id_counter += 1;
        }
    }

    // Create music section nodes if available
    if let Some(music) = &analysis.music_analysis {
        for section in &music.sections {
            let section_node_id = node_id_counter;
            nodes.push(AudioGraphNode {
                node_id: section_node_id,
                node_type: AudioNodeType::MusicSection,
                label: section.label.clone(),
                content: format!(
                    "{} ({:.1}s - {:.1}s)",
                    section.label, section.start_time, section.end_time
                ),
                time_range: Some(TimeRange {
                    start: section.start_time,
                    end: section.end_time,
                }),
                confidence: section.confidence,
                properties: HashMap::new(),
                annotations: vec![],
            });

            edges.push(AudioGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: audio_node_id,
                to_node: section_node_id,
                edge_type: AudioEdgeType::Contains,
                weight: section.confidence,
                properties: HashMap::new(),
            });

            node_id_counter += 1;
        }
    }

    // Add temporal edges between segments
    let segment_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| n.node_type == AudioNodeType::Segment)
        .collect();

    for i in 0..segment_nodes.len() {
        if i + 1 < segment_nodes.len() {
            edges.push(AudioGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: segment_nodes[i].node_id,
                to_node: segment_nodes[i + 1].node_id,
                edge_type: AudioEdgeType::Precedes,
                weight: 1.0,
                properties: HashMap::new(),
            });
        }
    }

    let edge_count = edges.len();
    Ok(AudioGraph {
        graph_id,
        name: graph_name.unwrap_or_else(|| format!("Audio Graph {}", graph_id)),
        modality: MODALITY.to_string(),
        project_id,
        source: AudioSource {
            path: None,
            url: None,
            hash: format!("hash_{}", graph_id),
            duration_seconds: analysis.duration_seconds,
            sample_rate: analysis.sample_rate,
            format: analysis.format,
        },
        nodes,
        edges,
        metadata: GraphMetadata {
            node_count: node_id_counter as usize - 1,
            edge_count: edge_count,
            has_transcription: analysis.transcription.is_some(),
            has_diarization: !analysis.speakers.is_empty(),
            has_music_analysis: analysis.music_analysis.is_some(),
            semantic_enriched: false,
            cross_modal_links: 0,
        },
        created_at: now.clone(),
        updated_at: now,
    })
}

async fn update_graph(graph_id: u64, updates: AudioGraphUpdate) -> Result<AudioGraph, String> {
    let mut graph = get_graph(graph_id).await?;
    let now = chrono::Utc::now().to_rfc3339();

    for node in updates.add_nodes {
        graph.nodes.push(node);
    }

    for update_node in updates.update_nodes {
        if let Some(existing) = graph
            .nodes
            .iter_mut()
            .find(|n| n.node_id == update_node.node_id)
        {
            *existing = update_node;
        }
    }

    graph
        .nodes
        .retain(|n| !updates.remove_nodes.contains(&n.node_id));

    for edge in updates.add_edges {
        graph.edges.push(edge);
    }

    graph
        .edges
        .retain(|e| !updates.remove_edges.contains(&e.edge_id));

    graph.metadata.node_count = graph.nodes.len();
    graph.metadata.edge_count = graph.edges.len();
    graph.updated_at = now;

    Ok(graph)
}

async fn query_graph(graph_id: u64, query: AudioQuery) -> Result<QueryResult, String> {
    let graph = get_graph(graph_id).await?;
    let min_confidence = query.min_confidence.unwrap_or(0.0);
    let limit = query.limit.unwrap_or(100);

    let (nodes, edges) = match query.query_type.clone() {
        AudioQueryType::FindSpeaker { speaker_id } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.node_type == AudioNodeType::Speaker && n.label.contains(&speaker_id)
                        || n.properties
                            .get("speaker_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s == speaker_id)
                            .unwrap_or(false)
                })
                .take(limit)
                .cloned()
                .collect();

            (matching_nodes, vec![])
        }

        AudioQueryType::FindSegments { segment_types } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.node_type == AudioNodeType::Segment && n.confidence >= min_confidence
                })
                .take(limit)
                .cloned()
                .collect();

            (matching_nodes, vec![])
        }

        AudioQueryType::SearchTranscript { pattern } => {
            let pattern_lower = pattern.to_lowercase();
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    (n.node_type == AudioNodeType::Sentence || n.node_type == AudioNodeType::Word)
                        && n.content.to_lowercase().contains(&pattern_lower)
                })
                .take(limit)
                .cloned()
                .collect();

            (matching_nodes, vec![])
        }

        AudioQueryType::GetTimeline { start_time, end_time } => {
            let time_range = TimeRange {
                start: start_time,
                end: end_time,
            };
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.time_range
                        .as_ref()
                        .map(|tr| tr.overlaps(&time_range))
                        .unwrap_or(false)
                })
                .take(limit)
                .cloned()
                .collect();

            (matching_nodes, vec![])
        }

        AudioQueryType::GetNodesAtTime { time } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.time_range
                        .as_ref()
                        .map(|tr| tr.contains(time))
                        .unwrap_or(false)
                })
                .take(limit)
                .cloned()
                .collect();

            (matching_nodes, vec![])
        }

        AudioQueryType::GetNodesByType { node_type } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.node_type == node_type && n.confidence >= min_confidence)
                .take(limit)
                .cloned()
                .collect();

            (matching_nodes, vec![])
        }

        _ => (vec![], vec![]),
    };

    Ok(QueryResult {
        query_type: format!("{:?}", query.query_type),
        total_matches: nodes.len(),
        nodes,
        edges,
        metadata: HashMap::new(),
    })
}

async fn get_graph(graph_id: u64) -> Result<AudioGraph, String> {
    // Load the real persisted sidecar (written by persist_audio_graph at
    // graphs/audio_<id>.json) instead of fabricating empty nodes/edges.
    let dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let abs = std::path::Path::new(&dir).join(format!("graphs/audio_{}.json", graph_id));
    let raw = std::fs::read_to_string(&abs)
        .map_err(|e| format!("no persisted audio graph at {}: {e}", abs.display()))?;
    let sidecar: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("corrupt audio graph sidecar {}: {e}", abs.display()))?;
    let nodes: Vec<AudioGraphNode> = sidecar.get("nodes")
        .and_then(|n| serde_json::from_value(n.clone()).ok())
        .unwrap_or_default();
    let edges: Vec<AudioGraphEdge> = sidecar.get("edges")
        .and_then(|e| serde_json::from_value(e.clone()).ok())
        .unwrap_or_default();
    Ok(AudioGraph {
        graph_id,
        name: format!("Audio Graph {}", graph_id),
        modality: MODALITY.to_string(),
        project_id: 1,
        source: AudioSource {
            path: None,
            url: None,
            hash: format!("hash_{}", graph_id),
            duration_seconds: 10.0,
            sample_rate: 44100,
            format: "WAV".to_string(),
        },
        nodes,
        edges,
        metadata: GraphMetadata::default(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Real ZSEI relationship write, idempotent (checks for an existing edge to
/// the same target before pushing). Mirrors math/main.rs's add_relationship,
/// with the existing-edge check math's own version still lacks.
async fn add_relationship(container_id: u64, target_id: u64) -> Result<(), String> {
    let existing = zsei_query(serde_json::json!({
        "GetContainer": { "container_id": container_id }
    }))
    .await?;
    let mut context = existing
        .get("Container")
        .and_then(|c| c.get("local_state"))
        .and_then(|l| l.get("context"))
        .cloned()
        .ok_or_else(|| format!("Container {} has no local_state.context", container_id))?;

    let relationships = context
        .get_mut("relationships")
        .and_then(|r| r.as_array_mut())
        .ok_or_else(|| format!("Container {} context.relationships is not an array", container_id))?;

    let already_linked = relationships.iter().any(|r| {
        r.get("target_id").and_then(|t| t.as_u64()) == Some(target_id)
            && r.get("relation_type").and_then(|t| t.as_str()) == Some("RelatedTo")
    });
    if already_linked {
        return Ok(());
    }

    relationships.push(serde_json::json!({
        "target_id": target_id,
        "relation_type": "RelatedTo",
        "confidence": 1.0,
        "discovered_via": "CrossModalLink"
    }));

    zsei_query(serde_json::json!({
        "UpdateContainer": {
            "container_id": container_id,
            "updates": { "context": context }
        }
    }))
    .await?;
    Ok(())
}

async fn link_to_modality(
    audio_graph_id: u64,
    target_graph_id: u64,
    _target_modality: &str,
    relationship: CrossModalityRelation,
) -> Result<LinkResult, String> {
    // Real, bidirectional ZSEI relationship — both containers must actually
    // exist; this used to unconditionally fabricate a "success" LinkResult
    // for any graph_id, real or not, with nothing behind it (2026-10-12
    // audit finding). An honest error here (e.g. a target_graph_id that was
    // never persisted) is correct behavior, not a regression. Pattern
    // matches math/main.rs's already-fixed link_to_modality.
    add_relationship(audio_graph_id, target_graph_id).await?;
    add_relationship(target_graph_id, audio_graph_id).await?;

    Ok(LinkResult {
        link_id: generate_graph_id(),
        source_graph_id: audio_graph_id,
        target_graph_id,
        relationship: format!("{:?}", relationship),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn trigger_semantic_hook(
    graph_id: u64,
    hook_type: ZSEIHookType,
    _options: HookOptions,
) -> Result<HookResult, String> {
    let start_time = std::time::Instant::now();

    // Honest zero — no real hook-processing logic exists yet for this
    // modality (2026-10-12 audit finding: previously hardcoded non-zero
    // 8/4/12 regardless of input/graph_id). Full semantic enrichment is a
    // separate, larger task; this fixes the fabricated-metrics problem.

    Ok(HookResult {
        hook_type,
        success: true,
        nodes_processed: 0,
        edges_added: 0,
        annotations_added: 0,
        processing_time_ms: start_time.elapsed().as_millis() as u64,
        errors: vec![],
    })
}

async fn extract_segment(
    audio_data: &AudioData,
    start_time: f32,
    end_time: f32,
) -> Result<SegmentResult, String> {
    let pcm = load_pcm(audio_data).await?;
    if !(end_time > start_time) || start_time < 0.0 {
        return Err("invalid range: need 0 <= start < end".to_string());
    }
    let sr = pcm.sample_rate as f64;
    let a = ((start_time as f64 * sr).round() as usize).min(pcm.frames());
    let b = ((end_time as f64 * sr).round() as usize).min(pcm.frames());
    if a >= b {
        return Err(format!("range {:.3}-{:.3} s is outside the audio ({:.3} s)", start_time, end_time, pcm.duration()));
    }
    let chn = pcm.channels.len();
    let mut data = Vec::with_capacity((b - a) * chn);
    for i in a..b {
        for c in 0..chn {
            data.push(pcm.channels[c][i]);
        }
    }
    Ok(SegmentResult {
        audio_data: AudioData::Samples { data, sample_rate: pcm.sample_rate, channels: chn.min(255) as u8 },
        original_range: TimeRange { start: a as f32 / pcm.sample_rate as f32, end: b as f32 / pcm.sample_rate as f32 },
        duration_seconds: (b - a) as f32 / pcm.sample_rate as f32,
    })
}

/// Returns (events found, requested types that cannot be detected here, with reasons).
async fn detect_events(
    audio_data: &AudioData,
    event_types: &[AudioEventType],
) -> Result<(Vec<AudioEvent>, Vec<String>), String> {
    let pcm = load_pcm(audio_data).await?;
    if pcm.frames() == 0 {
        return Err("audio has no samples".to_string());
    }
    let feats = analyze_signal(&pcm);
    let default = [AudioEventType::Clipping, AudioEventType::Dropout, AudioEventType::NoiseSpike];
    let types: &[AudioEventType] = if event_types.is_empty() { &default } else { event_types };
    Ok(detect_signal_events(&pcm, &feats, types))
}

fn generate_graph_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    duration.as_nanos() as u64 % 1_000_000_000
}

// ============================================================================
// CLI ENTRY POINT
// ============================================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let args: Vec<String> = env::args().collect();
    let mut input_json = String::new();
    let mut ai = 1usize;
    while ai < args.len() {
        if args[ai] == "--input" && ai + 1 < args.len() { input_json = ozone_input_arg(&args[ai + 1]); ai += 2; } else { ai += 1; }
    }
    if input_json.is_empty() {
        use std::io::Read;
        let _ = std::io::stdin().read_to_string(&mut input_json);
    }
    if input_json.trim().is_empty() {
        eprintln!("Usage: audio --input '<json>' (or JSON on stdin)");
        std::process::exit(1);
    }
    let input: Value = match serde_json::from_str::<Value>(&input_json) {
        Ok(v) => match v.get("data") { Some(d) => d.clone(), None => v },
        Err(e) => {
            eprintln!("Failed to parse input JSON: {}", e);
            std::process::exit(1);
        }
    };

    match execute(input).await {
        Ok(output) => {
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        Err(e) => {
            let error_output = serde_json::json!({
                "success": false,
                "action": "unknown",
                "result": null,
                "error": e,
                "metadata": {
                    "pipeline_id": PIPELINE_ID,
                    "pipeline_version": PIPELINE_VERSION,
                    "processing_time_ms": 0,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }
            });
            println!("{}", serde_json::to_string_pretty(&error_output).unwrap());
            std::process::exit(1);
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_wav_i16(rate: u32, chans: u16, samples: &[i16]) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&chans.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * chans as u32 * 2).to_le_bytes());
        v.extend_from_slice(&(chans * 2).to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    fn sine(rate: u32, hz: f64, secs: f64, amp: f64) -> Vec<f32> {
        (0..(rate as f64 * secs) as usize)
            .map(|i| (amp * (2.0 * std::f64::consts::PI * hz * i as f64 / rate as f64).sin()) as f32)
            .collect()
    }

    fn mono_pcm(rate: u32, x: Vec<f32>) -> Pcm {
        Pcm { sample_rate: rate, bit_depth: 32, format: "test".to_string(), channels: vec![x] }
    }

    #[test]
    fn parse_wav_16bit_stereo() {
        let wav = make_wav_i16(8000, 2, &[0, 16384, -16384, 32767, 100, -100]);
        let p = parse_wav(&wav).unwrap();
        assert_eq!((p.sample_rate, p.channels.len(), p.frames(), p.bit_depth), (8000, 2, 3, 16));
        assert!((p.channels[0][1] + 0.5).abs() < 1e-3); // frame 1 left = -16384
        assert!((p.channels[1][0] - 0.5).abs() < 1e-3); // frame 0 right = 16384
    }

    #[test]
    fn parse_wav_8bit_24bit_and_float32() {
        // 8-bit unsigned: 128 -> 0.0, 255 -> ~1.0
        let mut w8 = make_wav_i16(8000, 1, &[0, 0]);
        w8[34] = 8; // bits per sample
        w8[32] = 1; // block align
        let data_at = 44;
        w8.truncate(data_at);
        w8.extend_from_slice(&[128, 255]);
        let n = (w8.len() - 44) as u32;
        w8[40..44].copy_from_slice(&n.to_le_bytes());
        let p = parse_wav(&w8).unwrap();
        assert!(p.channels[0][0].abs() < 1e-3 && p.channels[0][1] > 0.99);
        // 24-bit negative full scale: 0x800000
        let mut w24 = make_wav_i16(8000, 1, &[0]);
        w24[34] = 24;
        w24[32] = 3;
        w24.truncate(44);
        w24.extend_from_slice(&[0x00, 0x00, 0x80]);
        w24[40..44].copy_from_slice(&3u32.to_le_bytes());
        let p = parse_wav(&w24).unwrap();
        assert!((p.channels[0][0] + 1.0).abs() < 1e-6);
        // float32 (format tag 3)
        let mut wf = make_wav_i16(8000, 1, &[0, 0]);
        wf[20] = 3;
        wf[34] = 32;
        wf[32] = 4;
        wf.truncate(44);
        wf.extend_from_slice(&0.25f32.to_le_bytes());
        wf[40..44].copy_from_slice(&4u32.to_le_bytes());
        let p = parse_wav(&wf).unwrap();
        assert!((p.channels[0][0] - 0.25).abs() < 1e-7);
        assert_eq!(p.format, "WAV/float");
    }

    #[test]
    fn non_wav_is_rejected_with_ffmpeg_hint() {
        let e = parse_wav(b"ID3\x03\x00\x00\x00 not a wav at all").unwrap_err();
        assert!(e.contains("ffmpeg"));
    }

    #[test]
    fn base64_decodes() {
        assert_eq!(b64_decode("TWFu").unwrap(), b"Man");
        assert_eq!(b64_decode("TWE=").unwrap(), b"Ma");
        assert!(b64_decode("T$Fu").is_err());
    }

    #[test]
    fn fft_sine_peaks_at_expected_bin() {
        let x = sine(8000, 1000.0, 0.128, 1.0); // 1024 samples, bin width 7.8125 Hz -> bin 128
        let mags = frame_magnitudes(&x, 0, &vec![1.0; 1024], 1024);
        let (argmax, _) = mags.iter().enumerate().fold((0, 0.0), |a, (i, &m)| if m > a.1 { (i, m) } else { a });
        assert_eq!(argmax, 128);
    }

    #[test]
    fn loudness_of_a_full_scale_997hz_sine_is_about_minus_3_lufs() {
        let l = integrated_lufs(&mono_pcm(48000, sine(48000, 997.0, 5.0, 0.99999))).unwrap();
        assert!((l + 3.01).abs() < 0.3, "got {}", l);
    }

    #[test]
    fn loudness_is_absent_for_audio_shorter_than_a_block() {
        assert!(integrated_lufs(&mono_pcm(48000, sine(48000, 440.0, 0.2, 0.5))).is_none());
    }

    #[test]
    fn activity_segments_split_silence_and_tone() {
        let mut x = vec![0.0f32; 16000];
        x.extend(sine(16000, 440.0, 1.0, 0.5));
        x.extend(vec![0.0f32; 16000]);
        let pcm = mono_pcm(16000, x);
        let f = analyze_signal(&pcm);
        let (segs, _, _, _) = activity_segments(&f, pcm.duration());
        let types: Vec<SegmentType> = segs.iter().map(|s| s.segment_type.clone()).collect();
        assert_eq!(types, vec![SegmentType::Silence, SegmentType::Unknown, SegmentType::Silence]);
        assert!((segs[1].start_time - 1.0).abs() < 0.1 && (segs[1].end_time - 2.0).abs() < 0.1);
        assert_eq!(segs[0].start_time, 0.0);
        assert!((segs[2].end_time - 3.0).abs() < 1e-3);
    }

    #[test]
    fn spectrogram_is_bounded_and_peaks_in_the_1khz_band() {
        let pcm = mono_pcm(16000, sine(16000, 1000.0, 2.0, 0.5));
        let f = analyze_signal(&pcm);
        let g = spectrogram_grid(&f);
        assert!(g.rows <= SPEC_ROWS && g.cols <= SPEC_COLS && g.values.len() == g.rows * g.cols);
        let means: Vec<f64> = (0..g.rows).map(|r| (0..g.cols).map(|c| g.values[r * g.cols + c] as f64).sum::<f64>() / g.cols as f64).collect();
        let argmax = means.iter().enumerate().fold((0usize, 0.0f64), |a, (i, &m)| if m > a.1 { (i, m) } else { a }).0;
        let (lo, hi) = (g.band_edges_hz[argmax], g.band_edges_hz[argmax + 1]);
        assert!(lo <= 1000.0 && 1000.0 <= hi * 1.05, "band {}..{}", lo, hi);
    }

    #[test]
    fn tempo_from_a_click_train() {
        // 8192-sample period at 16 kHz = 0.512 s -> 117.19 BPM
        let mut x = vec![0.0f32; 16000 * 12];
        let mut i = 0usize;
        while i + 8 < x.len() {
            for k in 0..8 {
                x[i + k] = 0.9;
            }
            i += 8192;
        }
        let pcm = mono_pcm(16000, x);
        let f = analyze_signal(&pcm);
        let m = music_from_features(&f, pcm.duration(), true);
        assert!((m.tempo_bpm - 117.19).abs() < 2.5, "got {}", m.tempo_bpm);
        assert!(m.tempo_confidence > 0.2);
        assert!(m.beats.len() >= 20);
        assert!(m.absent.contains_key("chords") && m.absent.contains_key("time_signature"));
    }

    #[test]
    fn key_estimation_recovers_a_rotated_profile() {
        let mut chroma = [0.0f64; 12];
        for i in 0..12 {
            chroma[i] = KK_MAJOR[(i + 12 - 7) % 12];
        }
        let (k, mode, conf) = estimate_key(&chroma);
        assert_eq!((k.as_str(), mode.as_str()), ("G", "major"));
        assert!(conf > 0.99);
        assert_eq!(estimate_key(&[0.0; 12]).0, "");
    }

    #[test]
    fn clipping_and_dropout_events_are_measured() {
        let mut x = sine(16000, 440.0, 0.5, 0.4);
        for k in 2000..2200 {
            x[k] = 1.0;
        }
        x.extend(vec![0.0f32; 1600]); // 0.1 s of digital zeros inside the signal
        x.extend(sine(16000, 440.0, 0.5, 0.4));
        let pcm = mono_pcm(16000, x);
        let f = analyze_signal(&pcm);
        let (ev, unsup) = detect_signal_events(&pcm, &f, &[AudioEventType::Clipping, AudioEventType::Dropout, AudioEventType::DoorSlam]);
        assert!(ev.iter().any(|e| e.event_type == AudioEventType::Clipping));
        assert!(ev.iter().any(|e| e.event_type == AudioEventType::Dropout && (e.end_time - e.start_time - 0.1).abs() < 0.01));
        assert_eq!(unsup.len(), 1);
    }

    #[tokio::test]
    async fn analyze_a_real_wav_file_reports_measured_values_and_absences() {
        let mut samples = Vec::new();
        for i in 0..16000usize {
            let t = i as f64 / 16000.0;
            let s = (2.0 * std::f64::consts::PI * 440.0 * t).sin();
            samples.push((s * 0.5 * 32767.0) as i16); // left 0.5
            samples.push((s * 0.25 * 32767.0) as i16); // right 0.25 (-6.02 dB)
        }
        let path = std::env::temp_dir().join(format!("ozone_audio_test_{}.wav", std::process::id()));
        std::fs::write(&path, make_wav_i16(16000, 2, &samples)).unwrap();
        let r = analyze_audio(&AudioData::FilePath(path.to_string_lossy().to_string()), AnalysisDepth::Standard, true, true, false, true)
            .await
            .unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!((r.sample_rate, r.channels), (16000, 2));
        assert!((r.duration_seconds - 1.0).abs() < 1e-3);
        let ild = r.metadata["stereo"]["ild_db"].as_f64().unwrap();
        assert!((ild - 6.02).abs() < 0.15, "ild {}", ild);
        assert!(r.metadata["stereo"]["caveat"].as_str().unwrap().contains("weak"));
        assert!(r.transcription.is_none() && r.speakers.is_empty());
        assert!(r.absent.contains_key("transcription") && r.absent.contains_key("diarization"));
        assert!(r.spectrogram.is_some());
    }

    #[tokio::test]
    async fn analyze_refuses_a_missing_or_non_wav_file() {
        let e = analyze_audio(&AudioData::FilePath("/nonexistent/audio.wav".to_string()), AnalysisDepth::Standard, false, false, false, false)
            .await
            .unwrap_err();
        assert!(e.contains("cannot read"));
        let e = load_pcm(&AudioData::Url("http://x/y.mp3".to_string())).await.unwrap_err();
        assert!(e.contains("no network"));
    }

    #[tokio::test]
    async fn transcribe_and_diarize_are_absent_not_canned() {
        let t = transcribe_audio(&AudioData::FilePath("x".to_string()), Some("en".to_string()), true, true).await.unwrap();
        assert!(t.text.is_empty() && t.words.is_empty() && t.absent_reason.is_some());
        let d = diarize_audio(&AudioData::FilePath("x".to_string()), None, None, None).await.unwrap();
        assert!(d.speakers.is_empty() && d.num_speakers == 0 && d.absent_reason.is_some());
    }

    #[tokio::test]
    async fn extract_segment_returns_real_samples() {
        let data = sine(8000, 440.0, 2.0, 0.5);
        let r = extract_segment(&AudioData::Samples { data, sample_rate: 8000, channels: 1 }, 0.5, 1.0).await.unwrap();
        match r.audio_data {
            AudioData::Samples { data, .. } => assert_eq!(data.len(), 4000),
            _ => panic!("expected samples"),
        }
        assert!((r.duration_seconds - 0.5).abs() < 1e-3);
    }

    #[test]
    fn test_time_range() {
        let tr1 = TimeRange { start: 0.0, end: 5.0 };
        let tr2 = TimeRange { start: 3.0, end: 8.0 };
        let tr3 = TimeRange { start: 6.0, end: 10.0 };
        assert!(tr1.overlaps(&tr2));
        assert!(!tr1.overlaps(&tr3));
        assert!(tr1.contains(2.5));
        assert!(!tr1.contains(6.0));
    }
}

/// `--input @<path>`: the host stages oversize inputs (> 100 KB) in a file because a
/// single argv string is capped at 128 KiB (E2BIG). Plain values pass through unchanged.
#[allow(dead_code)]
fn ozone_input_arg(raw: &str) -> String {
    match raw.strip_prefix('@') {
        Some(path) if std::path::Path::new(path).is_file() => std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Failed to read staged input {}: {}", path, e);
            std::process::exit(1);
        }),
        _ => raw.to_string(),
    }
}
