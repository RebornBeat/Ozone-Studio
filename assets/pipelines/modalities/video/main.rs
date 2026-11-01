//! OZONE Studio - Pipeline 104: Video Analysis
//!
//! Modality pipeline for video processing and structural graph creation.
//! Analyzes videos to detect scenes, shots, objects over time, and audio content.
//! Creates traversable graphs that can be enriched by ZSEI semantic hooks.
//!
//! # Actions
//! - `Analyze`: Extract scenes, shots, keyframes, object tracks
//! - `CreateGraph`: Build structural graph from analysis
//! - `ExtractFrame`: Extract single frame at timestamp
//! - `ExtractClip`: Extract video clip between timestamps
//! - `QueryGraph`: Query graph for specific information
//! - `TriggerSemanticHook`: Trigger ZSEI hooks for semantic enrichment
//!
//! # Graph Structure
//! - Nodes: Video, Scene, Shot, Keyframe, ObjectTrack, AudioTrack
//! - Edges: Contains, Precedes, Follows, AppearsIn, SynchronizedWith

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// ============================================================================
// PIPELINE METADATA
// ============================================================================

pub const PIPELINE_ID: u64 = 104;
pub const PIPELINE_NAME: &str = "video_analysis";
pub const PIPELINE_VERSION: &str = "0.4.0";
pub const MODALITY: &str = "video";

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoModalityInput {
    pub action: VideoAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VideoAction {
    /// Analyze video: scenes, shots, objects, timeline
    Analyze {
        video_data: VideoData,
        #[serde(default)]
        depth: AnalysisDepth,
        #[serde(default = "default_sample_rate")]
        sample_rate_fps: f32,
        #[serde(default)]
        detect_scenes: bool,
        #[serde(default)]
        detect_objects: bool,
        #[serde(default)]
        analyze_audio: bool,
        #[serde(default)]
        extract_keyframes: bool,
    },

    /// Create graph from analysis result
    CreateGraph {
        analysis: VideoAnalysisResult,
        project_id: u64,
        #[serde(default)]
        graph_name: Option<String>,
    },

    /// Update existing graph
    UpdateGraph {
        graph_id: u64,
        updates: VideoGraphUpdate,
    },

    /// Query video graph
    QueryGraph {
        graph_id: u64,
        query: VideoQuery,
    },

    /// Get graph
    GetGraph {
        graph_id: u64,
    },

    /// Extract frame at timestamp
    ExtractFrame {
        video_data: VideoData,
        timestamp: f32,
        #[serde(default)]
        format: ImageFormat,
    },

    /// Extract video clip
    ExtractClip {
        video_data: VideoData,
        start_time: f32,
        end_time: f32,
        #[serde(default)]
        include_audio: bool,
    },

    /// Detect scene boundaries
    DetectScenes {
        video_data: VideoData,
        #[serde(default)]
        threshold: f32,
        #[serde(default)]
        min_scene_length: f32,
    },

    /// Track objects through video
    TrackObjects {
        video_data: VideoData,
        #[serde(default)]
        object_classes: Option<Vec<String>>,
        #[serde(default)]
        sample_rate_fps: f32,
    },

    /// Link to another modality
    LinkToModality {
        video_graph_id: u64,
        target_graph_id: u64,
        target_modality: String,
        relationship: CrossModalityRelation,
    },

    /// Trigger ZSEI semantic hook
    TriggerSemanticHook {
        graph_id: u64,
        hook_type: ZSEIHookType,
        #[serde(default)]
        options: HookOptions,
    },
}

fn default_sample_rate() -> f32 {
    1.0
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoModalityOutput {
    pub success: bool,
    pub action: String,
    pub result: VideoResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub metadata: OutputMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VideoResult {
    Analysis(VideoAnalysisResult),
    Graph(VideoGraph),
    Query(QueryResult),
    Frame(FrameResult),
    Clip(ClipResult),
    Scenes(Vec<Scene>),
    Tracks(Vec<ObjectTrack>),
    Link(LinkResult),
    Hook(HookResult),
    Empty,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub pipeline_id: u64,
    pub pipeline_version: String,
    pub processing_time_ms: u64,
    pub timestamp: String,
}

// ============================================================================
// CORE DATA TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VideoData {
    FilePath(String),
    Url(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
pub enum AnalysisDepth {
    Surface,
    #[default]
    Standard,
    Deep,
    Comprehensive,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum ImageFormat {
    #[default]
    PNG,
    JPEG,
    WebP,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct VideoAnalysisResult {
    /// Total duration in seconds
    pub duration_seconds: f32,
    /// Video width
    pub width: u32,
    /// Video height
    pub height: u32,
    /// Frame rate
    pub fps: f32,
    /// Total frame count
    pub frame_count: u64,
    /// Video codec
    pub codec: String,
    /// Container format
    pub format: String,
    /// Bitrate in kbps
    pub bitrate_kbps: u32,
    /// Detected scenes
    pub scenes: Vec<Scene>,
    /// Detected shots
    pub shots: Vec<Shot>,
    /// Extracted keyframes
    pub keyframes: Vec<Keyframe>,
    /// Object tracks over time
    pub object_tracks: Vec<ObjectTrack>,
    /// Audio analysis (if included)
    pub audio_analysis: Option<AudioAnalysis>,
    /// Video quality metrics
    pub quality: VideoQuality,
    /// Metadata
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scene {
    /// Unique scene identifier
    pub scene_id: String,
    /// Scene number (1-indexed)
    pub scene_number: usize,
    /// Start time in seconds
    pub start_time: f32,
    /// End time in seconds
    pub end_time: f32,
    /// Start frame number
    pub start_frame: u64,
    /// End frame number
    pub end_frame: u64,
    /// Scene description
    pub description: Option<String>,
    /// Dominant colors in scene
    pub dominant_colors: Vec<Color>,
    /// Key objects appearing in scene
    pub key_objects: Vec<String>,
    /// Scene type/category
    pub scene_type: Option<SceneType>,
    /// Detection confidence
    pub confidence: f32,
    /// Thumbnail frame number
    pub thumbnail_frame: u64,
}

impl Scene {
    pub fn duration(&self) -> f32 {
        self.end_time - self.start_time
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SceneType {
    Indoor,
    Outdoor,
    Dialog,
    Action,
    Establishing,
    CloseUp,
    Montage,
    Transition,
    Credits,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shot {
    /// Unique shot identifier
    pub shot_id: String,
    /// Parent scene ID
    pub scene_id: String,
    /// Shot number within scene
    pub shot_number: usize,
    /// Start time in seconds
    pub start_time: f32,
    /// End time in seconds
    pub end_time: f32,
    /// Start frame
    pub start_frame: u64,
    /// End frame
    pub end_frame: u64,
    /// Shot type
    pub shot_type: ShotType,
    /// Camera movement
    pub camera_movement: Option<CameraMovement>,
    /// Confidence
    pub confidence: f32,
}

impl Shot {
    pub fn duration(&self) -> f32 {
        self.end_time - self.start_time
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ShotType {
    ExtremeCloseUp,
    CloseUp,
    MediumCloseUp,
    MediumShot,
    MediumLongShot,
    LongShot,
    ExtremeLongShot,
    OverTheShoulder,
    PointOfView,
    TwoShot,
    GroupShot,
    InsertShot,
    CutawayShot,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CameraMovement {
    Static,
    Pan,
    Tilt,
    Zoom,
    Dolly,
    Tracking,
    Crane,
    Handheld,
    Steadicam,
    Aerial,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Keyframe {
    /// Frame number
    pub frame_number: u64,
    /// Timestamp in seconds
    pub timestamp: f32,
    /// Why this frame is significant
    pub significance: KeyframeSignificance,
    /// Significance score
    pub score: f32,
    /// Objects detected in frame
    pub objects: Vec<DetectedObject>,
    /// Scene ID this keyframe belongs to
    pub scene_id: Option<String>,
    /// Shot ID this keyframe belongs to
    pub shot_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum KeyframeSignificance {
    SceneStart,
    SceneEnd,
    ShotBoundary,
    PeakAction,
    VisuallyDistinct,
    ObjectAppearance,
    FaceDetected,
    TextVisible,
    Periodic,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectedObject {
    pub object_id: String,
    pub label: String,
    pub confidence: f32,
    pub bounding_box: BoundingBox,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectTrack {
    /// Track identifier
    pub track_id: String,
    /// Object label/class
    pub object_label: String,
    /// Object appearances over time
    pub appearances: Vec<ObjectAppearance>,
    /// Total time object is visible
    pub total_visible_time: f32,
    /// First appearance timestamp
    pub first_seen: f32,
    /// Last appearance timestamp
    pub last_seen: f32,
    /// Average confidence
    pub avg_confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectAppearance {
    /// Start time
    pub start_time: f32,
    /// End time
    pub end_time: f32,
    /// Bounding boxes with timestamps
    pub bounding_boxes: Vec<TimestampedBox>,
    /// Confidence
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimestampedBox {
    pub timestamp: f32,
    pub frame_number: u64,
    pub bounding_box: BoundingBox,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub percentage: f32,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioAnalysis {
    /// Has audio track
    pub has_audio: bool,
    /// Audio codec
    pub codec: Option<String>,
    /// Sample rate
    pub sample_rate: Option<u32>,
    /// Channels
    pub channels: Option<u8>,
    /// Speech segments
    pub speech_segments: Vec<AudioSegment>,
    /// Music segments
    pub music_segments: Vec<AudioSegment>,
    /// Silence segments
    pub silence_segments: Vec<AudioSegment>,
    /// Loudness in LUFS
    pub loudness_lufs: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioSegment {
    pub start_time: f32,
    pub end_time: f32,
    pub segment_type: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VideoQuality {
    /// Average bitrate quality
    pub bitrate_quality: f32,
    /// Resolution quality score
    pub resolution_quality: f32,
    /// Frame rate stability
    pub framerate_stability: f32,
    /// Compression artifacts level
    pub compression_artifacts: f32,
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
pub struct VideoGraph {
    pub graph_id: u64,
    pub name: String,
    pub modality: String,
    pub project_id: u64,
    pub source: VideoSource,
    pub nodes: Vec<VideoGraphNode>,
    pub edges: Vec<VideoGraphEdge>,
    pub metadata: GraphMetadata,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoSource {
    pub path: Option<String>,
    pub url: Option<String>,
    pub hash: String,
    pub duration_seconds: f32,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoGraphNode {
    pub node_id: u64,
    pub node_type: VideoNodeType,
    pub label: String,
    pub content: String,
    pub time_range: Option<TimeRange>,
    pub frame_range: Option<FrameRange>,
    pub confidence: f32,
    pub properties: HashMap<String, Value>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrameRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum VideoNodeType {
    Video,
    Scene,
    Shot,
    Keyframe,
    ObjectTrack,
    ObjectAppearance,
    AudioTrack,
    AudioSegment,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticAnnotation {
    pub annotation_type: String,
    pub value: Value,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoGraphEdge {
    pub edge_id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: VideoEdgeType,
    pub weight: f32,
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum VideoEdgeType {
    Contains,
    ContainedBy,
    Precedes,
    Follows,
    Overlaps,
    AppearsIn,
    SynchronizedWith,
    SimilarTo,
    TransitionsTo,
    ContinuesFrom,
    // Cross-modality
    DescribedBy,
    TranscribedAs,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub scene_count: usize,
    pub shot_count: usize,
    pub keyframe_count: usize,
    pub object_track_count: usize,
    pub has_audio_analysis: bool,
    pub semantic_enriched: bool,
    pub cross_modal_links: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoGraphUpdate {
    pub add_nodes: Vec<VideoGraphNode>,
    pub update_nodes: Vec<VideoGraphNode>,
    pub remove_nodes: Vec<u64>,
    pub add_edges: Vec<VideoGraphEdge>,
    pub remove_edges: Vec<u64>,
    pub metadata_updates: Option<HashMap<String, Value>>,
}

// ============================================================================
// QUERY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoQuery {
    pub query_type: VideoQueryType,
    pub parameters: HashMap<String, Value>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub min_confidence: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VideoQueryType {
    /// Find scenes
    FindScenes { scene_types: Option<Vec<String>> },
    /// Find object by label
    FindObject { label: String },
    /// Get timeline of all events
    GetTimeline { start_time: f32, end_time: f32 },
    /// Get nodes at specific time
    GetNodesAtTime { time: f32 },
    /// Semantic search
    SemanticSearch { query: String },
    /// Get keyframes
    GetKeyframes { scene_id: Option<String> },
    /// Get object tracks
    GetObjectTracks { min_duration: Option<f32> },
    /// Get nodes by type
    GetNodesByType { node_type: VideoNodeType },
    /// Custom query
    Custom { query: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub query_type: String,
    pub nodes: Vec<VideoGraphNode>,
    pub edges: Vec<VideoGraphEdge>,
    pub total_matches: usize,
    pub metadata: HashMap<String, Value>,
}

// ============================================================================
// EXTRACTION RESULTS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrameResult {
    pub timestamp: f32,
    pub frame_number: u64,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub data: String, // Base64 encoded (empty when over the inline cap — see `path`/`note`)
    /// Temp-file path of the extracted frame (real file written by ffmpeg output).
    #[serde(default)]
    pub path: Option<String>,
    /// FNV-1a 64 content hash (hex) of the encoded frame bytes — identity, not security.
    #[serde(default)]
    pub content_hash_fnv64: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Pixel-grid geometry in frame "video-px" (docs/SPATIAL_GEOMETRY_LAYER.md).
    #[serde(default)]
    pub geom: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipResult {
    pub start_time: f32,
    pub end_time: f32,
    pub duration: f32,
    pub has_audio: bool,
    pub path: Option<String>,
    pub data: Option<String>, // Base64 if small enough
    #[serde(default)]
    pub note: Option<String>,
}

// ============================================================================
// CROSS-MODALITY & HOOKS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CrossModalityRelation {
    TranscribedAs,
    DescribedBy,
    IllustratedBy,
    SynchronizedWith,
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

// ============================================================================
// EXECUTION
// ============================================================================

pub async fn execute(input: Value) -> Result<Value, String> {
    let start_time = std::time::Instant::now();

    // Honest refusal for a text-only "mention" with no real raw video data
    // attached, BEFORE the strict typed deserialization below would reject
    // it with an accidental, low-level parse error. Only the one action that
    // requires raw data (Analyze) is checked.
    if input.get("action").and_then(|a| a.get("type")).and_then(|t| t.as_str()) == Some("Analyze")
        && input.get("action").and_then(|a| a.get("video_data")).map(|d| d.is_null()).unwrap_or(true)
    {
        return Ok(serde_json::json!({
            "success": false,
            "action": "Analyze",
            "result": null,
            "error": "No raw video data provided — this call appears to be a text-only mention with no attached data file; real video analysis requires an actual data file",
            "metadata": {
                "pipeline_id": PIPELINE_ID,
                "pipeline_version": PIPELINE_VERSION,
                "processing_time_ms": start_time.elapsed().as_millis(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        }));
    }

    let input: VideoModalityInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    let result = match input.action {
        VideoAction::Analyze {
            video_data,
            depth,
            sample_rate_fps,
            detect_scenes,
            detect_objects,
            analyze_audio,
            extract_keyframes,
        } => {
            let analysis = analyze_video(
                &video_data,
                depth,
                sample_rate_fps,
                detect_scenes,
                detect_objects,
                analyze_audio,
                extract_keyframes,
            )
            .await?;
            ("Analyze", VideoResult::Analysis(analysis))
        }

        VideoAction::CreateGraph {
            analysis,
            project_id,
            graph_name,
        } => {
            let mut graph = create_graph(analysis, project_id, graph_name).await?;
            if let Err(e) = persist_video_graph(&mut graph).await {
                eprintln!("video CreateGraph: ZSEI persistence failed: {e}");
            }
            ("CreateGraph", VideoResult::Graph(graph))
        }

        VideoAction::UpdateGraph { graph_id, updates } => {
            let graph = update_graph(graph_id, updates).await?;
            ("UpdateGraph", VideoResult::Graph(graph))
        }

        VideoAction::QueryGraph { graph_id, query } => {
            let result = query_graph(graph_id, query).await?;
            ("QueryGraph", VideoResult::Query(result))
        }

        VideoAction::GetGraph { graph_id } => {
            let graph = get_graph(graph_id).await?;
            ("GetGraph", VideoResult::Graph(graph))
        }

        VideoAction::ExtractFrame {
            video_data,
            timestamp,
            format,
        } => {
            let frame = extract_frame(&video_data, timestamp, format).await?;
            ("ExtractFrame", VideoResult::Frame(frame))
        }

        VideoAction::ExtractClip {
            video_data,
            start_time,
            end_time,
            include_audio,
        } => {
            let clip = extract_clip(&video_data, start_time, end_time, include_audio).await?;
            ("ExtractClip", VideoResult::Clip(clip))
        }

        VideoAction::DetectScenes {
            video_data,
            threshold,
            min_scene_length,
        } => {
            let scenes = detect_scenes(&video_data, threshold, min_scene_length).await?;
            ("DetectScenes", VideoResult::Scenes(scenes))
        }

        VideoAction::TrackObjects {
            video_data,
            object_classes,
            sample_rate_fps,
        } => {
            let tracks = track_objects(&video_data, object_classes, sample_rate_fps).await?;
            ("TrackObjects", VideoResult::Tracks(tracks))
        }

        VideoAction::LinkToModality {
            video_graph_id,
            target_graph_id,
            target_modality,
            relationship,
        } => {
            let link =
                link_to_modality(video_graph_id, target_graph_id, &target_modality, relationship)
                    .await?;
            ("LinkToModality", VideoResult::Link(link))
        }

        VideoAction::TriggerSemanticHook {
            graph_id,
            hook_type,
            options,
        } => {
            let result = trigger_semantic_hook(graph_id, hook_type, options).await?;
            ("TriggerSemanticHook", VideoResult::Hook(result))
        }
    };

    let output = VideoModalityOutput {
        success: true,
        action: result.0.to_string(),
        result: result.1,
        error: None,
        metadata: OutputMetadata {
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
// REAL VIDEO ANALYSIS (D16): ffprobe / ffmpeg adapters.
// Every value below is either read from the file's own container/stream
// metadata (ffprobe), or measured by an ffmpeg filter (scene cuts, silence,
// loudness), or ABSENT with a reason. Object detection / tracking / shot typing
// / speech-vs-music classification need models and are visual-mcp's surface.
// ============================================================================

const FFPROBE_MISSING: &str = "ffprobe not installed (video metadata is absent)";
const FFMPEG_MISSING: &str = "ffmpeg not installed (this operation is absent)";
const MAX_KEYFRAMES: usize = 400;
const MAX_INLINE_FRAME_BYTES: usize = 2 * 1024 * 1024;

/// A video made available as a local file. Byte inputs are written to the temp dir
/// and removed on drop.
struct LocalVideo {
    path: String,
    temp: bool,
}

impl Drop for LocalVideo {
    fn drop(&mut self) {
        if self.temp {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn fnv64_hex(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:016x}", h)
}

fn materialize_video(v: &VideoData) -> Result<LocalVideo, String> {
    match v {
        VideoData::FilePath(p) => {
            if p.contains('\0') || !std::path::Path::new(p).is_file() {
                return Err(format!("video file not found: {p}"));
            }
            Ok(LocalVideo { path: p.clone(), temp: false })
        }
        VideoData::Bytes(b) => {
            let path = std::env::temp_dir().join(format!("ozone-video-in-{}.bin", fnv64_hex(b)));
            std::fs::write(&path, b).map_err(|e| format!("cannot stage video bytes: {e}"))?;
            Ok(LocalVideo { path: path.to_string_lossy().into_owned(), temp: true })
        }
        VideoData::Url(u) => Err(format!("video URL input is absent: this pipeline does no network I/O ({u})")),
    }
}

/// Run an external tool; "not found" becomes the exact absent-reason string.
fn run_tool(bin: &str, args: &[String], missing: &str) -> Result<(Vec<u8>, String, bool), String> {
    match std::process::Command::new(bin).args(args).output() {
        Ok(o) => Ok((o.stdout, String::from_utf8_lossy(&o.stderr).into_owned(), o.status.success())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(missing.to_string()),
        Err(e) => Err(format!("{bin} failed to start: {e}")),
    }
}

fn tail(s: &str, n: usize) -> String {
    let t: Vec<&str> = s.lines().rev().take(n).collect();
    t.into_iter().rev().collect::<Vec<_>>().join(" | ")
}

fn ffprobe_json(path: &str) -> Result<Value, String> {
    let args: Vec<String> = ["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams", "-i", path].iter().map(|s| s.to_string()).collect();
    let (out, err, ok) = run_tool("ffprobe", &args, FFPROBE_MISSING)?;
    if !ok {
        return Err(format!("ffprobe could not read the file: {}", tail(&err, 2)));
    }
    serde_json::from_slice(&out).map_err(|e| format!("ffprobe returned unparseable JSON: {e}"))
}

/// "30000/1001" -> 29.97; "0/0" and malformed -> None.
fn parse_frame_rate(s: &str) -> Option<f64> {
    let (n, d) = s.split_once('/')?;
    let (n, d): (f64, f64) = (n.trim().parse().ok()?, d.trim().parse().ok()?);
    if d == 0.0 || n <= 0.0 {
        None
    } else {
        Some(n / d)
    }
}

fn jstr(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn jnum_str(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| x.as_f64()))
}

/// Container/stream facts -> VideoAnalysisResult (pure; testable on canned ffprobe JSON).
fn probe_to_analysis(j: &Value, want_audio: bool) -> VideoAnalysisResult {
    let empty = Vec::new();
    let streams = j.get("streams").and_then(|s| s.as_array()).unwrap_or(&empty);
    let fmt = j.get("format").cloned().unwrap_or(Value::Null);
    let video = streams.iter().find(|s| {
        s.get("codec_type").and_then(|c| c.as_str()) == Some("video")
            && s.get("disposition").and_then(|d| d.get("attached_pic")).and_then(|a| a.as_i64()) != Some(1)
    });
    let audio = streams.iter().find(|s| s.get("codec_type").and_then(|c| c.as_str()) == Some("audio"));
    let mut meta: HashMap<String, Value> = HashMap::new();
    meta.insert("source".to_string(), Value::String("ffprobe".to_string()));
    let mut absent: Vec<Value> = vec![
        Value::String("shots: absent — shot typing/camera movement needs a classifier".to_string()),
        Value::String("object_tracks: absent — detection/tracking needs models (visual-mcp over extracted frames)".to_string()),
        Value::String("quality.*: absent — no computed estimator (0.0 means 'not computed')".to_string()),
    ];

    let mut res = VideoAnalysisResult::default();
    if let Some(v) = video {
        res.width = v.get("width").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        res.height = v.get("height").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let fps = jstr(v, "avg_frame_rate").and_then(|s| parse_frame_rate(&s)).or_else(|| jstr(v, "r_frame_rate").and_then(|s| parse_frame_rate(&s)));
        match fps {
            Some(f) => res.fps = f as f32,
            None => absent.push(Value::String("fps: absent — stream reports no valid frame rate".to_string())),
        }
        res.codec = jstr(v, "codec_name").unwrap_or_default();
        for k in ["pix_fmt", "profile", "level", "field_order", "display_aspect_ratio", "sample_aspect_ratio", "color_space", "color_range", "bits_per_raw_sample"] {
            if let Some(x) = v.get(k) {
                meta.insert(k.to_string(), x.clone());
            }
        }
        let rot = v
            .get("tags")
            .and_then(|t| t.get("rotate"))
            .and_then(|r| r.as_str().and_then(|s| s.parse::<i64>().ok()))
            .or_else(|| {
                v.get("side_data_list")
                    .and_then(|l| l.as_array())
                    .and_then(|l| l.iter().find_map(|d| d.get("rotation").and_then(|r| r.as_i64())))
            });
        if let Some(r) = rot {
            meta.insert("rotation_degrees".to_string(), Value::from(r));
        }
        if let Some(n) = jnum_str(v, "nb_frames") {
            res.frame_count = n as u64;
        }
    } else {
        absent.push(Value::String("video stream: absent — the file has no video stream".to_string()));
    }
    let dur = jnum_str(&fmt, "duration").or_else(|| video.and_then(|v| jnum_str(v, "duration")));
    match dur {
        Some(d) => res.duration_seconds = d as f32,
        None => absent.push(Value::String("duration: absent — container reports none".to_string())),
    }
    if res.frame_count == 0 && res.fps > 0.0 && res.duration_seconds > 0.0 {
        res.frame_count = (res.duration_seconds as f64 * res.fps as f64).floor() as u64;
        meta.insert("frame_count_estimated".to_string(), Value::Bool(true));
        meta.insert("frame_count_basis".to_string(), Value::String("floor(duration x fps); the stream reports no nb_frames".to_string()));
    }
    res.format = jstr(&fmt, "format_name").unwrap_or_default();
    if let Some(n) = jstr(&fmt, "format_long_name") {
        meta.insert("format_long_name".to_string(), Value::String(n));
    }
    if let Some(b) = jnum_str(&fmt, "bit_rate") {
        res.bitrate_kbps = (b / 1000.0).round() as u32;
    }
    if let Some(tags) = fmt.get("tags") {
        for k in ["creation_time", "title", "encoder", "major_brand"] {
            if let Some(x) = tags.get(k) {
                meta.insert(k.to_string(), x.clone());
            }
        }
    }
    if want_audio {
        res.audio_analysis = Some(match audio {
            Some(a) => AudioAnalysis {
                has_audio: true,
                codec: jstr(a, "codec_name"),
                sample_rate: jnum_str(a, "sample_rate").map(|x| x as u32),
                channels: a.get("channels").and_then(|c| c.as_u64()).map(|c| c as u8),
                speech_segments: Vec::new(),
                music_segments: Vec::new(),
                silence_segments: Vec::new(),
                loudness_lufs: None,
            },
            None => AudioAnalysis { has_audio: false, codec: None, sample_rate: None, channels: None, speech_segments: Vec::new(), music_segments: Vec::new(), silence_segments: Vec::new(), loudness_lufs: None },
        });
        absent.push(Value::String("audio speech/music segments: absent — classification needs a model".to_string()));
    }
    meta.insert("absent_reasons".to_string(), Value::Array(absent));
    meta.insert(
        "geom".to_string(),
        serde_json::json!({"frame": "video-px", "kind": "grid", "x": 0, "y": 0, "w": res.width, "h": res.height, "origin": "top-left"}),
    );
    meta.insert(
        "span".to_string(),
        serde_json::json!({"frame": "time:video", "kind": "span", "start": 0.0, "end": res.duration_seconds}),
    );
    res.metadata = meta;
    res
}

/// Scene cuts from ffmpeg's `select` + `metadata=print` output (stdout+stderr text).
/// Lines carry `pts_time:<t>` then `lavfi.scene_score=<s>`; both layouts of
/// different ffmpeg versions are accepted.
fn parse_scene_cuts(text: &str) -> Vec<(f64, f64)> {
    let mut cur: Option<f64> = None;
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(i) = line.find("pts_time:") {
            let rest = &line[i + "pts_time:".len()..];
            let num: String = rest.trim_start().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
            if let Ok(t) = num.parse::<f64>() {
                cur = Some(t);
            }
        }
        if let Some(i) = line.find("lavfi.scene_score=") {
            let rest = &line[i + "lavfi.scene_score=".len()..];
            let num: String = rest.trim().chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let (Some(t), Ok(s)) = (cur, num.parse::<f64>()) {
                out.push((t, s));
                cur = None;
            }
        }
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    out.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-6);
    out
}

/// silencedetect stderr -> (start, end) pairs; an unterminated silence ends at `total`.
fn parse_silence(text: &str, total: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut open: Option<f64> = None;
    let num_after = |line: &str, key: &str| -> Option<f64> {
        let i = line.find(key)?;
        let rest = &line[i + key.len()..];
        let n: String = rest.trim_start().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
        n.parse().ok()
    };
    for line in text.lines() {
        if let Some(s) = num_after(line, "silence_start:") {
            open = Some(s.max(0.0));
        } else if let Some(e) = num_after(line, "silence_end:") {
            if let Some(s) = open.take() {
                out.push((s, e));
            }
        }
    }
    if let Some(s) = open {
        if total > s {
            out.push((s, total));
        }
    }
    out
}

/// ebur128 summary -> integrated loudness (LUFS).
fn parse_integrated_lufs(text: &str) -> Option<f32> {
    let i = text.rfind("Integrated loudness")?;
    let rest = &text[i..];
    let j = rest.find("I:")?;
    let num: String = rest[j + 2..].trim_start().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
    num.parse().ok()
}

/// ffprobe `-skip_frame nokey` frames JSON -> I-frame timestamps.
fn parse_keyframes(j: &Value) -> Vec<f64> {
    let mut v: Vec<f64> = j
        .get("frames")
        .and_then(|f| f.as_array())
        .map(|a| a.iter().filter_map(|f| jnum_str(f, "pts_time").or_else(|| jnum_str(f, "best_effort_timestamp_time"))).collect())
        .unwrap_or_default();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    v.truncate(MAX_KEYFRAMES);
    v
}

fn scene_cuts_for(path: &str, threshold: f64) -> Result<Vec<(f64, f64)>, String> {
    let thr = if threshold > 0.0 { threshold.clamp(0.05, 0.95) } else { 0.4 };
    let args: Vec<String> = vec![
        "-hide_banner".into(),
        "-nostats".into(),
        "-i".into(),
        path.into(),
        "-an".into(),
        "-vf".into(),
        format!("select='gt(scene,{thr:.3})',metadata=print"),
        "-f".into(),
        "null".into(),
        "-".into(),
    ];
    let (out, err, ok) = run_tool("ffmpeg", &args, FFMPEG_MISSING)?;
    if !ok {
        return Err(format!("ffmpeg scene detection failed: {}", tail(&err, 2)));
    }
    let text = format!("{}\n{}", String::from_utf8_lossy(&out), err);
    Ok(parse_scene_cuts(&text))
}

/// Cuts -> contiguous scenes covering [0, duration]; cuts closer than `min_len` to the
/// previous kept cut (or to either end) are dropped. First scene has no cut score: confidence 1.0
/// is NOT a claim — it starts at 0 by definition (see metadata).
fn scenes_from_cuts(cuts: &[(f64, f64)], duration: f64, fps: f64, min_len: f64) -> Vec<Scene> {
    let mut kept: Vec<(f64, f64)> = Vec::new();
    let mut last = 0.0f64;
    for (t, s) in cuts {
        if *t > last + min_len && *t < duration - min_len.min(duration) {
            kept.push((*t, *s));
            last = *t;
        }
    }
    let mut bounds: Vec<(f64, f64)> = vec![(0.0, 1.0)];
    bounds.extend(kept);
    let mut scenes = Vec::new();
    for (i, (start, score)) in bounds.iter().enumerate() {
        let end = bounds.get(i + 1).map(|b| b.0).unwrap_or(duration);
        if end <= *start {
            continue;
        }
        let sf = (*start * fps).round() as u64;
        let ef = (end * fps).round() as u64;
        scenes.push(Scene {
            scene_id: format!("scene_{}", i + 1),
            scene_number: i + 1,
            start_time: *start as f32,
            end_time: end as f32,
            start_frame: sf,
            end_frame: ef,
            description: None,
            dominant_colors: Vec::new(),
            key_objects: Vec::new(),
            scene_type: None,
            confidence: *score as f32,
            thumbnail_frame: sf + (ef.saturating_sub(sf)) / 2,
        });
    }
    scenes
}

fn analyze_video_blocking(
    path: &str,
    detect_scenes: bool,
    detect_objects: bool,
    analyze_audio: bool,
    extract_keyframes: bool,
) -> Result<VideoAnalysisResult, String> {
    let probe = ffprobe_json(path)?;
    let mut res = probe_to_analysis(&probe, analyze_audio);
    let dur = res.duration_seconds as f64;
    let fps = res.fps as f64;
    let mut notes: Vec<Value> = Vec::new();

    if detect_scenes {
        if dur > 0.0 && fps > 0.0 {
            match scene_cuts_for(path, 0.4) {
                Ok(cuts) => {
                    res.scenes = scenes_from_cuts(&cuts, dur, fps, 0.0);
                    notes.push(Value::String("scenes: ffmpeg select(scene>0.4); confidence = the cut's scene score; scene 1 starts at 0 by definition (confidence 1.0 is not a score)".to_string()));
                }
                Err(e) => notes.push(Value::String(format!("scenes: absent — {e}"))),
            }
        } else {
            notes.push(Value::String("scenes: absent — duration or frame rate unknown".to_string()));
        }
    }
    if extract_keyframes {
        let args: Vec<String> = ["-v", "quiet", "-select_streams", "v:0", "-skip_frame", "nokey", "-show_entries", "frame=pts_time,best_effort_timestamp_time", "-of", "json", "-i", path].iter().map(|s| s.to_string()).collect();
        match run_tool("ffprobe", &args, FFPROBE_MISSING) {
            Ok((out, _, true)) => {
                let times = parse_keyframes(&serde_json::from_slice::<Value>(&out).unwrap_or(Value::Null));
                for t in times {
                    let scene = res.scenes.iter().find(|s| (s.start_time as f64) <= t + 1e-3 && t < s.end_time as f64 + 1e-3);
                    let at_start = scene.map(|s| (s.start_time as f64 - t).abs() < 0.05).unwrap_or(false);
                    res.keyframes.push(Keyframe {
                        frame_number: (t * fps).round() as u64,
                        timestamp: t as f32,
                        significance: if at_start { KeyframeSignificance::SceneStart } else { KeyframeSignificance::Periodic },
                        score: 1.0,
                        objects: Vec::new(),
                        scene_id: scene.map(|s| s.scene_id.clone()),
                        shot_id: None,
                    });
                }
                notes.push(Value::String("keyframes: codec I-frames (score 1.0 = certain I-frame, NOT visual importance; capped at 400)".to_string()));
            }
            Ok((_, err, false)) => notes.push(Value::String(format!("keyframes: absent — ffprobe failed: {}", tail(&err, 2)))),
            Err(e) => notes.push(Value::String(format!("keyframes: absent — {e}"))),
        }
    }
    if analyze_audio {
        if let Some(a) = res.audio_analysis.as_mut() {
            if a.has_audio {
                let sil_args: Vec<String> = ["-hide_banner", "-nostats", "-i", path, "-vn", "-af", "silencedetect=noise=-35dB:d=0.5", "-f", "null", "-"].iter().map(|s| s.to_string()).collect();
                match run_tool("ffmpeg", &sil_args, FFMPEG_MISSING) {
                    Ok((_, err, true)) => {
                        a.silence_segments = parse_silence(&err, dur)
                            .into_iter()
                            .map(|(s, e)| AudioSegment { start_time: s as f32, end_time: e as f32, segment_type: "silence".to_string(), confidence: 1.0 })
                            .collect();
                        notes.push(Value::String("audio silence: ffmpeg silencedetect noise<=-35dB d>=0.5s (threshold decision; confidence 1.0 = rule satisfied)".to_string()));
                    }
                    Ok((_, err, false)) => notes.push(Value::String(format!("audio silence: absent — ffmpeg failed: {}", tail(&err, 2)))),
                    Err(e) => notes.push(Value::String(format!("audio silence: absent — {e}"))),
                }
                let ld_args: Vec<String> = ["-hide_banner", "-nostats", "-i", path, "-vn", "-af", "ebur128=framelog=quiet", "-f", "null", "-"].iter().map(|s| s.to_string()).collect();
                if let Ok((_, err, true)) = run_tool("ffmpeg", &ld_args, FFMPEG_MISSING) {
                    a.loudness_lufs = parse_integrated_lufs(&err);
                }
            }
        }
    }
    if detect_objects {
        notes.push(Value::String("object_tracks: absent — detection needs models (extract frames, then visual-mcp yolo_detect)".to_string()));
    }
    if !notes.is_empty() {
        res.metadata.insert("analysis_notes".to_string(), Value::Array(notes));
    }
    Ok(res)
}

async fn analyze_video(
    video_data: &VideoData,
    _depth: AnalysisDepth,
    _sample_rate_fps: f32,
    detect_scenes: bool,
    detect_objects: bool,
    analyze_audio: bool,
    extract_keyframes: bool,
) -> Result<VideoAnalysisResult, String> {
    let lv = materialize_video(video_data)?;
    tokio::task::spawn_blocking(move || {
        let path = lv.path.clone();
        let r = analyze_video_blocking(&path, detect_scenes, detect_objects, analyze_audio, extract_keyframes);
        drop(lv);
        r
    })
    .await
    .map_err(|e| format!("analysis task failed: {e}"))?
}

// ============================================================================
// ZSEI OVER HTTP + PERSISTENCE (the text/code/math/image revival pattern)
// ============================================================================

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let host = env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let client = reqwest::Client::new();
    let body = serde_json::json!({"query": query, "session_token": ""});
    let resp = client
        .post(format!("{host}/zsei/query"))
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

async fn persist_video_graph(graph: &mut VideoGraph) -> Result<(), String> {
    let keywords: Vec<String> = {
        let mut k: Vec<String> = graph.nodes.iter()
            .filter_map(|n| n.label.to_lowercase().split_whitespace().next().map(String::from))
            .collect();
        k.sort(); k.dedup();
        if k.is_empty() { k.push("video".to_string()); }
        k.truncate(12);
        k
    };
    let name = format!(
        "Video graph ({} nodes, {} edges)",
        graph.nodes.len(), graph.edges.len()
    );
    let now = chrono::Utc::now().timestamp() as u64;
    let container = serde_json::json!({
        "global_state": {"container_id": 0, "parent_id": graph.project_id, "child_ids": [], "child_count": 0, "version": 1},
        "local_state": {
            "metadata": {"container_type": "ModalityGraph", "modality": "Unknown", "created_at": now,
                "updated_at": now, "provenance": "video-pipeline", "permissions": 0, "owner_id": 0,
                "name": name, "materialized_path": null},
            "context": {"categories": [], "methodologies": [], "keywords": keywords,
                "topics": ["video"], "relationships": [], "learned_associations": [], "embedding": null},
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
    let rel = format!("graphs/video_{}.json", new_id);
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
    analysis: VideoAnalysisResult,
    project_id: u64,
    graph_name: Option<String>,
) -> Result<VideoGraph, String> {
    let graph_id = generate_graph_id();
    let now = chrono::Utc::now().to_rfc3339();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_id_counter: u64 = 1;

    // Create root video node
    let video_node_id = node_id_counter;
    nodes.push(VideoGraphNode {
        node_id: video_node_id,
        node_type: VideoNodeType::Video,
        label: "Video".to_string(),
        content: format!(
            "{}x{} {:.1}s @ {:.0}fps",
            analysis.width, analysis.height, analysis.duration_seconds, analysis.fps
        ),
        time_range: Some(TimeRange {
            start: 0.0,
            end: analysis.duration_seconds,
        }),
        frame_range: Some(FrameRange {
            start: 0,
            end: analysis.frame_count,
        }),
        confidence: 1.0,
        properties: {
            let mut props = HashMap::new();
            props.insert("width".to_string(), Value::from(analysis.width));
            props.insert("height".to_string(), Value::from(analysis.height));
            props.insert("fps".to_string(), Value::from(analysis.fps));
            props.insert("codec".to_string(), Value::from(analysis.codec.clone()));
            // Geometry (docs/SPATIAL_GEOMETRY_LAYER.md): the picture plane is a pixel
            // grid in frame "video-px"; the clip's extent in time is a Span.
            props.insert("geom".to_string(), serde_json::json!({"frame": "video-px", "kind": "grid", "x": 0, "y": 0, "w": analysis.width, "h": analysis.height, "origin": "top-left"}));
            props.insert("frame".to_string(), Value::from("video-px"));
            props.insert("span".to_string(), serde_json::json!({"frame": "time:video", "kind": "span", "start": 0.0, "end": analysis.duration_seconds}));
            props
        },
        annotations: vec![],
    });
    node_id_counter += 1;

    // Create scene nodes
    let mut scene_node_map: HashMap<String, u64> = HashMap::new();
    for scene in &analysis.scenes {
        let scene_node_id = node_id_counter;
        scene_node_map.insert(scene.scene_id.clone(), scene_node_id);

        nodes.push(VideoGraphNode {
            node_id: scene_node_id,
            node_type: VideoNodeType::Scene,
            label: format!("Scene {}", scene.scene_number),
            content: scene
                .description
                .clone()
                .unwrap_or_else(|| format!("Scene {}", scene.scene_number)),
            time_range: Some(TimeRange {
                start: scene.start_time,
                end: scene.end_time,
            }),
            frame_range: Some(FrameRange {
                start: scene.start_frame,
                end: scene.end_frame,
            }),
            confidence: scene.confidence,
            properties: {
                let mut props = HashMap::new();
                if let Some(st) = &scene.scene_type {
                    props.insert(
                        "scene_type".to_string(),
                        serde_json::to_value(st).unwrap_or(Value::Null),
                    );
                }
                props.insert("geom".to_string(), serde_json::json!({"frame": "time:video", "kind": "span", "start": scene.start_time, "end": scene.end_time}));
                props
            },
            annotations: vec![],
        });

        edges.push(VideoGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: video_node_id,
            to_node: scene_node_id,
            edge_type: VideoEdgeType::Contains,
            weight: 1.0,
            properties: HashMap::new(),
        });

        // LIVE NODE EVENT (2026-10-19, additive — same pattern as 3D's
        // first real producer). Fire-and-forget, never awaited inline.
        {
            let host = env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
            let pid = project_id;
            let gid = graph_id;
            let nid = scene_node_id;
            let scene_num = scene.scene_number;
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let _ = client.post(format!("{host}/orchestration/node-event"))
                    .json(&serde_json::json!({
                        "project_id": pid,
                        "modality": "video",
                        "graph_id": gid,
                        "node_id": nid,
                        "related_node_id": serde_json::Value::Null,
                        "node_type": format!("Scene:{}", scene_num),
                        "operation": "node_created",
                    }))
                    .timeout(std::time::Duration::from_secs(2))
                    .send().await;
            });
        }

        node_id_counter += 1;
    }

    // Add temporal edges between scenes
    for i in 0..analysis.scenes.len() {
        if i + 1 < analysis.scenes.len() {
            let current_id = scene_node_map[&analysis.scenes[i].scene_id];
            let next_id = scene_node_map[&analysis.scenes[i + 1].scene_id];

            edges.push(VideoGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: current_id,
                to_node: next_id,
                edge_type: VideoEdgeType::Precedes,
                weight: 1.0,
                properties: HashMap::new(),
            });
        }
    }

    // Create shot nodes
    for shot in &analysis.shots {
        let shot_node_id = node_id_counter;

        nodes.push(VideoGraphNode {
            node_id: shot_node_id,
            node_type: VideoNodeType::Shot,
            label: format!("Shot {}", shot.shot_number),
            content: format!("{:?}", shot.shot_type),
            time_range: Some(TimeRange {
                start: shot.start_time,
                end: shot.end_time,
            }),
            frame_range: Some(FrameRange {
                start: shot.start_frame,
                end: shot.end_frame,
            }),
            confidence: shot.confidence,
            properties: {
                let mut props = HashMap::new();
                props.insert("geom".to_string(), serde_json::json!({"frame": "time:video", "kind": "span", "start": shot.start_time, "end": shot.end_time}));
                props
            },
            annotations: vec![],
        });

        // Edge from scene to shot
        if let Some(&scene_node_id) = scene_node_map.get(&shot.scene_id) {
            edges.push(VideoGraphEdge {
                edge_id: edges.len() as u64 + 1,
                from_node: scene_node_id,
                to_node: shot_node_id,
                edge_type: VideoEdgeType::Contains,
                weight: shot.confidence,
                properties: HashMap::new(),
            });
        }

        node_id_counter += 1;
    }

    // Create keyframe nodes
    for keyframe in &analysis.keyframes {
        let kf_node_id = node_id_counter;

        nodes.push(VideoGraphNode {
            node_id: kf_node_id,
            node_type: VideoNodeType::Keyframe,
            label: format!("Frame {}", keyframe.frame_number),
            content: format!("{:?} at {:.2}s", keyframe.significance, keyframe.timestamp),
            time_range: Some(TimeRange {
                start: keyframe.timestamp,
                end: keyframe.timestamp,
            }),
            frame_range: Some(FrameRange {
                start: keyframe.frame_number,
                end: keyframe.frame_number,
            }),
            confidence: keyframe.score,
            properties: {
                let mut props = HashMap::new();
                props.insert("geom".to_string(), serde_json::json!({"frame": "time:video", "kind": "span", "start": keyframe.timestamp, "end": keyframe.timestamp}));
                props
            },
            annotations: vec![],
        });

        // Edge from scene to keyframe
        if let Some(scene_id) = &keyframe.scene_id {
            if let Some(&scene_node_id) = scene_node_map.get(scene_id) {
                edges.push(VideoGraphEdge {
                    edge_id: edges.len() as u64 + 1,
                    from_node: scene_node_id,
                    to_node: kf_node_id,
                    edge_type: VideoEdgeType::Contains,
                    weight: keyframe.score,
                    properties: HashMap::new(),
                });
            }
        }

        node_id_counter += 1;
    }

    // Create object track nodes
    for track in &analysis.object_tracks {
        let track_node_id = node_id_counter;

        nodes.push(VideoGraphNode {
            node_id: track_node_id,
            node_type: VideoNodeType::ObjectTrack,
            label: track.object_label.clone(),
            content: format!(
                "{} visible for {:.1}s",
                track.object_label, track.total_visible_time
            ),
            time_range: Some(TimeRange {
                start: track.first_seen,
                end: track.last_seen,
            }),
            frame_range: None,
            confidence: track.avg_confidence,
            properties: HashMap::new(),
            annotations: vec![],
        });

        edges.push(VideoGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: video_node_id,
            to_node: track_node_id,
            edge_type: VideoEdgeType::Contains,
            weight: track.avg_confidence,
            properties: HashMap::new(),
        });

        node_id_counter += 1;
    }

    let edge_count = edges.len();
    let node_count = nodes.len();
    Ok(VideoGraph {
        graph_id,
        name: graph_name.unwrap_or_else(|| format!("Video Graph {}", graph_id)),
        modality: MODALITY.to_string(),
        project_id,
        source: VideoSource {
            path: None,
            url: None,
            hash: format!("hash_{}", graph_id),
            duration_seconds: analysis.duration_seconds,
            width: analysis.width,
            height: analysis.height,
            fps: analysis.fps,
            format: analysis.format,
        },
        nodes,
        edges,
        metadata: GraphMetadata {
            node_count: node_count,
            edge_count: edge_count,
            scene_count: analysis.scenes.len(),
            shot_count: analysis.shots.len(),
            keyframe_count: analysis.keyframes.len(),
            object_track_count: analysis.object_tracks.len(),
            has_audio_analysis: analysis.audio_analysis.is_some(),
            semantic_enriched: false,
            cross_modal_links: 0,
        },
        created_at: now.clone(),
        updated_at: now,
    })
}

async fn update_graph(graph_id: u64, updates: VideoGraphUpdate) -> Result<VideoGraph, String> {
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

async fn query_graph(graph_id: u64, query: VideoQuery) -> Result<QueryResult, String> {
    let graph = get_graph(graph_id).await?;
    let limit = query.limit.unwrap_or(100);
    let min_confidence = query.min_confidence.unwrap_or(0.0);

    let (nodes, edges) = match query.query_type.clone() {
        VideoQueryType::FindScenes { scene_types } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.node_type == VideoNodeType::Scene && n.confidence >= min_confidence
                })
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        VideoQueryType::FindObject { label } => {
            let label_lower = label.to_lowercase();
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.node_type == VideoNodeType::ObjectTrack
                        && n.label.to_lowercase().contains(&label_lower)
                })
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        VideoQueryType::GetTimeline { start_time, end_time } => {
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

        VideoQueryType::GetNodesAtTime { time } => {
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

        VideoQueryType::GetKeyframes { scene_id } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.node_type == VideoNodeType::Keyframe)
                .take(limit)
                .cloned()
                .collect();
            (matching_nodes, vec![])
        }

        VideoQueryType::GetNodesByType { node_type } => {
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

async fn get_graph(graph_id: u64) -> Result<VideoGraph, String> {
    // Load the real persisted sidecar (written by persist_video_graph at
    // graphs/video_<id>.json) instead of fabricating empty nodes/edges.
    let dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let abs = std::path::Path::new(&dir).join(format!("graphs/video_{}.json", graph_id));
    let raw = std::fs::read_to_string(&abs)
        .map_err(|e| format!("no persisted video graph at {}: {e}", abs.display()))?;
    let sidecar: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("corrupt video graph sidecar {}: {e}", abs.display()))?;
    let nodes: Vec<VideoGraphNode> = sidecar.get("nodes")
        .and_then(|n| serde_json::from_value(n.clone()).ok())
        .unwrap_or_default();
    let edges: Vec<VideoGraphEdge> = sidecar.get("edges")
        .and_then(|e| serde_json::from_value(e.clone()).ok())
        .unwrap_or_default();
    Ok(VideoGraph {
        graph_id,
        name: format!("Video Graph {}", graph_id),
        modality: MODALITY.to_string(),
        project_id: 1,
        source: VideoSource {
            path: None,
            url: None,
            hash: format!("hash_{}", graph_id),
            duration_seconds: 60.0,
            width: 1920,
            height: 1080,
            fps: 30.0,
            format: "MP4".to_string(),
        },
        nodes,
        edges,
        metadata: GraphMetadata::default(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// width/height from PNG, JPEG (SOF) or WEBP bytes — only what the bytes decide.
fn encoded_frame_dims(b: &[u8]) -> Option<(u32, u32)> {
    if b.len() >= 24 && b.starts_with(&[0x89u8, b'P', b'N', b'G']) {
        return Some((u32::from_be_bytes([b[16], b[17], b[18], b[19]]), u32::from_be_bytes([b[20], b[21], b[22], b[23]])));
    }
    if b.len() >= 4 && b[0] == 0xFF && b[1] == 0xD8 {
        let mut i = 2usize;
        while i + 9 <= b.len() {
            if b[i] != 0xFF {
                i += 1;
                continue;
            }
            let m = b[i + 1];
            if (0xC0..=0xCF).contains(&m) && ![0xC4u8, 0xC8, 0xCC].contains(&m) {
                return Some((u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32, u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32));
            }
            i += 2 + u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        }
        return None;
    }
    if b.len() >= 30 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return match &b[12..16] {
            b"VP8X" => Some((1 + (b[24] as u32 | (b[25] as u32) << 8 | (b[26] as u32) << 16), 1 + (b[27] as u32 | (b[28] as u32) << 8 | (b[29] as u32) << 16))),
            b"VP8L" if b[20] == 0x2F => {
                let bits = u32::from_le_bytes([b[21], b[22], b[23], b[24]]);
                Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
            }
            b"VP8 " if b[23..26] == [0x9D, 0x01, 0x2A] => Some(((u16::from_le_bytes([b[26], b[27]]) & 0x3FFF) as u32, (u16::from_le_bytes([b[28], b[29]]) & 0x3FFF) as u32)),
            _ => None,
        };
    }
    None
}

fn extract_frame_blocking(path: &str, timestamp: f32, format: ImageFormat) -> Result<FrameResult, String> {
    if !timestamp.is_finite() || timestamp < 0.0 {
        return Err("timestamp must be a finite number of seconds >= 0".to_string());
    }
    let (codec, ext) = match format {
        ImageFormat::PNG => ("png", "png"),
        ImageFormat::JPEG => ("mjpeg", "jpg"),
        ImageFormat::WebP => ("libwebp", "webp"),
    };
    let args: Vec<String> = vec![
        "-hide_banner".into(),
        "-nostats".into(),
        "-v".into(),
        "error".into(),
        "-ss".into(),
        format!("{timestamp:.3}"),
        "-i".into(),
        path.into(),
        "-frames:v".into(),
        "1".into(),
        "-f".into(),
        "image2pipe".into(),
        "-c:v".into(),
        codec.into(),
        "-".into(),
    ];
    let (bytes, err, ok) = run_tool("ffmpeg", &args, FFMPEG_MISSING)?;
    if !ok || bytes.is_empty() {
        return Err(format!(
            "no frame produced at {timestamp:.3}s (past the end of the stream, or the {codec} encoder is unavailable): {}",
            tail(&err, 2)
        ));
    }
    let (width, height) = encoded_frame_dims(&bytes).unwrap_or((0, 0));
    let mut note: Vec<String> = Vec::new();
    let frame_number = match ffprobe_json(path) {
        Ok(j) => {
            let a = probe_to_analysis(&j, false);
            if a.fps > 0.0 {
                (timestamp as f64 * a.fps as f64).floor() as u64
            } else {
                note.push("frame_number is 0: the stream reports no frame rate".to_string());
                0
            }
        }
        Err(e) => {
            note.push(format!("frame_number is 0: {e}"));
            0
        }
    };
    let hash = fnv64_hex(&bytes);
    let out_path = std::env::temp_dir().join(format!("ozone-video-frame-{hash}.{ext}"));
    let path_str = match std::fs::write(&out_path, &bytes) {
        Ok(_) => Some(out_path.to_string_lossy().into_owned()),
        Err(e) => {
            note.push(format!("frame file not written: {e}"));
            None
        }
    };
    let data = if bytes.len() <= MAX_INLINE_FRAME_BYTES {
        use base64::{engine::general_purpose::STANDARD, Engine};
        STANDARD.encode(&bytes)
    } else {
        note.push(format!("frame data not inlined ({} bytes > {} byte cap): read it from `path`", bytes.len(), MAX_INLINE_FRAME_BYTES));
        String::new()
    };
    Ok(FrameResult {
        timestamp,
        frame_number,
        width,
        height,
        format,
        data,
        path: path_str,
        content_hash_fnv64: Some(hash),
        note: if note.is_empty() { None } else { Some(note.join("; ")) },
        geom: Some(serde_json::json!({"frame": "video-px", "kind": "grid", "x": 0, "y": 0, "w": width, "h": height, "origin": "top-left", "at_time": timestamp})),
    })
}

async fn extract_frame(video_data: &VideoData, timestamp: f32, format: ImageFormat) -> Result<FrameResult, String> {
    let lv = materialize_video(video_data)?;
    tokio::task::spawn_blocking(move || {
        let r = extract_frame_blocking(&lv.path, timestamp, format);
        drop(lv);
        r
    })
    .await
    .map_err(|e| format!("frame task failed: {e}"))?
}

fn extract_clip_blocking(path: &str, start_time: f32, end_time: f32, include_audio: bool) -> Result<ClipResult, String> {
    if !(start_time.is_finite() && end_time.is_finite()) || start_time < 0.0 || end_time <= start_time {
        return Err("clip range must satisfy 0 <= start < end (finite seconds)".to_string());
    }
    let hash = fnv64_hex(format!("{path}|{start_time}|{end_time}|{include_audio}").as_bytes());
    let out_path = std::env::temp_dir().join(format!("ozone-video-clip-{hash}.mp4"));
    let out_str = out_path.to_string_lossy().into_owned();
    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-nostats".into(),
        "-v".into(),
        "error".into(),
        "-y".into(),
        "-ss".into(),
        format!("{start_time:.3}"),
        "-i".into(),
        path.into(),
        "-t".into(),
        format!("{:.3}", end_time - start_time),
        "-c".into(),
        "copy".into(),
        "-avoid_negative_ts".into(),
        "make_zero".into(),
    ];
    if !include_audio {
        args.push("-an".into());
    }
    args.push(out_str.clone());
    let (_, err, ok) = run_tool("ffmpeg", &args, FFMPEG_MISSING)?;
    let size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    if !ok || size == 0 {
        return Err(format!("clip extraction failed: {}", tail(&err, 2)));
    }
    let has_audio = ffprobe_json(&out_str)
        .map(|j| j.get("streams").and_then(|s| s.as_array()).map(|a| a.iter().any(|s| s.get("codec_type").and_then(|c| c.as_str()) == Some("audio"))).unwrap_or(false))
        .unwrap_or(include_audio);
    Ok(ClipResult {
        start_time,
        end_time,
        duration: end_time - start_time,
        has_audio,
        path: Some(out_str),
        data: None,
        note: Some("stream copy (-c copy): cut points snap to the nearest preceding keyframe; the written duration may exceed the request".to_string()),
    })
}

async fn extract_clip(video_data: &VideoData, start_time: f32, end_time: f32, include_audio: bool) -> Result<ClipResult, String> {
    let lv = materialize_video(video_data)?;
    tokio::task::spawn_blocking(move || {
        let r = extract_clip_blocking(&lv.path, start_time, end_time, include_audio);
        drop(lv);
        r
    })
    .await
    .map_err(|e| format!("clip task failed: {e}"))?
}

async fn detect_scenes(video_data: &VideoData, threshold: f32, min_scene_length: f32) -> Result<Vec<Scene>, String> {
    let lv = materialize_video(video_data)?;
    tokio::task::spawn_blocking(move || {
        let r = (|| -> Result<Vec<Scene>, String> {
            let a = probe_to_analysis(&ffprobe_json(&lv.path)?, false);
            if !(a.duration_seconds > 0.0 && a.fps > 0.0) {
                return Err("scene detection absent: duration or frame rate unknown".to_string());
            }
            let cuts = scene_cuts_for(&lv.path, threshold as f64)?;
            Ok(scenes_from_cuts(&cuts, a.duration_seconds as f64, a.fps as f64, min_scene_length.max(0.0) as f64))
        })();
        drop(lv);
        r
    })
    .await
    .map_err(|e| format!("scene task failed: {e}"))?
}

async fn track_objects(_video_data: &VideoData, _object_classes: Option<Vec<String>>, _sample_rate_fps: f32) -> Result<Vec<ObjectTrack>, String> {
    // D16: the old canned 'person' track is deleted. Tracking needs per-frame
    // detections (extract frames here, run visual-mcp yolo_detect, then link boxes
    // across frames); that composition is orchestration above this pipeline.
    Err("object tracking is absent: it needs per-frame detections from visual-mcp (extract frames, yolo_detect, then link) — this pipeline does not fabricate tracks".to_string())
}

/// Append one real ZSEI Relation (RelatedTo, discovered_via Manual — this is
/// an explicit LinkToModality call, not something inferred) to a container's
/// existing `context.relationships`, preserving every other context field
/// exactly (ContainerUpdate.context is a full replace, not a merge, so the
/// current context has to be read back first). Idempotent: skips the write
/// if an identical relation to target_id already exists (unlike math's
/// equivalent, which lacks this check and produces duplicate edges on
/// repeated calls).
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
        r.get("target_id").and_then(|v| v.as_u64()) == Some(target_id)
            && r.get("relation_type").and_then(|v| v.as_str()) == Some("RelatedTo")
            && r.get("discovered_via").and_then(|v| v.as_str()) == Some("Manual")
    });
    if already_linked {
        return Ok(());
    }

    relationships.push(serde_json::json!({
        "target_id": target_id,
        "relation_type": "RelatedTo",
        "confidence": 1.0,
        "discovered_via": "Manual"
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
    video_graph_id: u64,
    target_graph_id: u64,
    _target_modality: &str,
    relationship: CrossModalityRelation,
) -> Result<LinkResult, String> {
    // Real, bidirectional ZSEI relationship — both containers must actually
    // exist; this used to unconditionally fabricate a "success" LinkResult
    // for any graph_id, real or not, with nothing behind it (2026-10-12
    // audit finding). An honest error here (e.g. a target_graph_id that was
    // never persisted) is correct behavior, not a regression.
    add_relationship(video_graph_id, target_graph_id).await?;
    add_relationship(target_graph_id, video_graph_id).await?;

    Ok(LinkResult {
        link_id: generate_graph_id(),
        source_graph_id: video_graph_id,
        target_graph_id,
        relationship: format!("{:?}", relationship),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn trigger_semantic_hook(
    _graph_id: u64,
    hook_type: ZSEIHookType,
    _options: HookOptions,
) -> Result<HookResult, String> {
    let start_time = std::time::Instant::now();

    // Honest zero — no real hook-processing logic exists yet for this
    // modality (2026-10-12 audit finding: previously returned hardcoded
    // non-zero fake metrics — 12/6/18 — regardless of input). Full semantic
    // enrichment is a separate, larger task; this fixes the immediate
    // honesty problem without pretending enrichment exists.
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
        if args[ai] == "--input" && ai + 1 < args.len() { input_json = args[ai + 1].clone(); ai += 2; } else { ai += 1; }
    }
    if input_json.is_empty() {
        use std::io::Read;
        let _ = std::io::stdin().read_to_string(&mut input_json);
    }
    if input_json.trim().is_empty() {
        eprintln!("Usage: video --input '<json>' (or JSON on stdin)");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_video_missing_file_is_an_error_not_a_canned_result() {
        // D16: the 120 s / 30 fps / 'person' track fabrication is gone; a missing file errors.
        let input = serde_json::json!({
            "action": {
                "type": "Analyze",
                "video_data": {"FilePath": "/test/video-does-not-exist.mp4"},
                "depth": "Standard",
                "sample_rate_fps": 1.0,
                "detect_scenes": true,
                "detect_objects": true,
                "analyze_audio": true,
                "extract_keyframes": true
            }
        });
        let result = execute(input).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_time_range() {
        let tr1 = TimeRange {
            start: 0.0,
            end: 30.0,
        };
        let tr2 = TimeRange {
            start: 25.0,
            end: 60.0,
        };

        assert!(tr1.overlaps(&tr2));
        assert!(tr1.contains(15.0));
        assert!(!tr1.contains(35.0));
    }

    fn canned_probe() -> Value {
        serde_json::json!({
            "streams": [
                {"codec_type": "video", "codec_name": "h264", "width": 1280, "height": 720,
                 "avg_frame_rate": "30000/1001", "r_frame_rate": "30000/1001", "pix_fmt": "yuv420p",
                 "nb_frames": "300", "duration": "10.01", "side_data_list": [{"rotation": -90}]},
                {"codec_type": "audio", "codec_name": "aac", "sample_rate": "48000", "channels": 2}
            ],
            "format": {"format_name": "mov,mp4,m4a,3gp,3g2,mj2", "format_long_name": "QuickTime / MOV",
                       "duration": "10.010000", "bit_rate": "2500000", "tags": {"creation_time": "2026-01-02T03:04:05Z"}}
        })
    }

    #[test]
    fn probe_json_becomes_real_metadata_only() {
        let a = probe_to_analysis(&canned_probe(), true);
        assert_eq!((a.width, a.height), (1280, 720));
        assert!((a.fps - 29.97).abs() < 0.01);
        assert_eq!(a.codec, "h264");
        assert_eq!(a.frame_count, 300);
        assert_eq!(a.bitrate_kbps, 2500);
        assert!((a.duration_seconds - 10.01).abs() < 1e-3);
        let au = a.audio_analysis.unwrap();
        assert!(au.has_audio && au.sample_rate == Some(48000) && au.channels == Some(2));
        assert!(au.speech_segments.is_empty() && au.music_segments.is_empty());
        assert_eq!(a.metadata["rotation_degrees"], -90);
        assert!(a.scenes.is_empty() && a.shots.is_empty() && a.object_tracks.is_empty());
        assert_eq!(a.metadata["geom"]["frame"], "video-px");
        assert_eq!(a.metadata["span"]["kind"], "span");
        assert!(a.metadata["absent_reasons"].to_string().contains("object_tracks"));
    }

    #[test]
    fn missing_stream_fields_are_absent_not_invented() {
        let a = probe_to_analysis(&serde_json::json!({"streams": [], "format": {}}), true);
        assert_eq!(a.width, 0);
        assert_eq!(a.duration_seconds, 0.0);
        assert!(!a.audio_analysis.unwrap().has_audio);
        let r = a.metadata["absent_reasons"].to_string();
        assert!(r.contains("video stream") && r.contains("duration"));
    }

    #[test]
    fn frame_count_is_marked_estimated_when_not_reported() {
        let j = serde_json::json!({"streams": [{"codec_type": "video", "width": 8, "height": 8, "avg_frame_rate": "10/1", "duration": "2.0"}], "format": {"duration": "2.0"}});
        let a = probe_to_analysis(&j, false);
        assert_eq!(a.frame_count, 20);
        assert_eq!(a.metadata["frame_count_estimated"], true);
    }

    #[test]
    fn frame_rate_parsing_rejects_zero_and_garbage() {
        assert_eq!(parse_frame_rate("25/1"), Some(25.0));
        assert!(parse_frame_rate("0/0").is_none());
        assert!(parse_frame_rate("abc").is_none());
        assert!(parse_frame_rate("30/0").is_none());
    }

    #[test]
    fn scene_cuts_parse_both_ffmpeg_layouts_and_build_contiguous_scenes() {
        let text = "[Parsed_metadata_1 @ 0x1] frame:120  pts:3003    pts_time:4.004\n[Parsed_metadata_1 @ 0x1] lavfi.scene_score=0.62\nframe:300 pts:9009 pts_time:10.01\nlavfi.scene_score=0.45\n";
        let cuts = parse_scene_cuts(text);
        assert_eq!(cuts.len(), 2);
        assert!((cuts[0].0 - 4.004).abs() < 1e-9 && (cuts[0].1 - 0.62).abs() < 1e-9);
        let scenes = scenes_from_cuts(&cuts, 20.0, 25.0, 0.0);
        assert_eq!(scenes.len(), 3);
        assert_eq!(scenes[0].start_time, 0.0);
        assert!((scenes[2].end_time - 20.0).abs() < 1e-6);
        assert!(scenes.windows(2).all(|w| (w[0].end_time - w[1].start_time).abs() < 1e-6));
        assert!(scenes.iter().all(|s| s.description.is_none() && s.key_objects.is_empty() && s.scene_type.is_none()));
        // min length merges: a cut 0.2 s after the previous one is dropped
        let close = vec![(5.0, 0.5), (5.2, 0.5), (12.0, 0.7)];
        assert_eq!(scenes_from_cuts(&close, 20.0, 25.0, 1.0).len(), 3);
        // a score line with no preceding time is not paired
        assert!(parse_scene_cuts("lavfi.scene_score=0.9\n").is_empty());
    }

    #[test]
    fn silence_and_loudness_parsers() {
        let text = "[silencedetect @ 0x1] silence_start: 1.5\n[silencedetect @ 0x1] silence_end: 3.25 | silence_duration: 1.75\n[silencedetect @ 0x1] silence_start: 8.0\n";
        let s = parse_silence(text, 10.0);
        assert_eq!(s, vec![(1.5, 3.25), (8.0, 10.0)]);
        assert!(parse_silence("nothing here", 10.0).is_empty());
        let l = "  Integrated loudness:\n    I:         -23.4 LUFS\n    Threshold: -33.4 LUFS\n";
        assert_eq!(parse_integrated_lufs(l), Some(-23.4));
        assert_eq!(parse_integrated_lufs("no summary"), None);
    }

    #[test]
    fn keyframe_json_is_sorted_deduped_and_capped() {
        let j = serde_json::json!({"frames": [{"pts_time": "4.0"}, {"pts_time": "0.0"}, {"best_effort_timestamp_time": "2.0"}, {"pts_time": "4.0"}]});
        assert_eq!(parse_keyframes(&j), vec![0.0, 2.0, 4.0]);
        assert!(parse_keyframes(&Value::Null).is_empty());
    }

    #[test]
    fn staged_bytes_are_removed_and_urls_are_refused() {
        let lv = materialize_video(&VideoData::Bytes(vec![1, 2, 3, 4])).unwrap();
        let path = lv.path.clone();
        assert!(std::path::Path::new(&path).is_file());
        drop(lv);
        assert!(!std::path::Path::new(&path).exists());
        assert!(materialize_video(&VideoData::Url("http://x/y.mp4".to_string())).unwrap_err().contains("no network"));
        assert!(materialize_video(&VideoData::FilePath("/definitely/not/here.mp4".to_string())).is_err());
    }

    #[test]
    fn track_objects_never_fabricates() {
        let r = futures_block(track_objects(&VideoData::Bytes(vec![]), None, 1.0));
        assert!(r.unwrap_err().contains("absent"));
    }

    fn futures_block<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f)
    }

    #[test]
    fn frame_dimension_sniffing() {
        let mut png = vec![0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        png.extend_from_slice(&64u32.to_be_bytes());
        png.extend_from_slice(&48u32.to_be_bytes());
        assert_eq!(encoded_frame_dims(&png), Some((64, 48)));
        assert_eq!(encoded_frame_dims(b"nope"), None);
    }

    fn tool_available(bin: &str) -> bool {
        std::process::Command::new(bin).arg("-version").output().map(|o| o.status.success()).unwrap_or(false)
    }

    /// Conditional integration test: runs only where ffmpeg+ffprobe exist (EU), builds a
    /// real 2 s 64x48 10 fps test pattern, and checks the pipeline's measurements against
    /// the values it was generated with. Skips (returns) when the tools are absent — an
    /// honest skip, not a pass claim.
    #[test]
    fn real_ffmpeg_roundtrip_when_tools_exist() {
        if !(tool_available("ffmpeg") && tool_available("ffprobe")) {
            eprintln!("SKIPPED: ffmpeg/ffprobe not installed");
            return;
        }
        let out = std::env::temp_dir().join("ozone-video-test-64x48.mp4");
        let st = std::process::Command::new("ffmpeg")
            .args(["-hide_banner", "-v", "error", "-y", "-f", "lavfi", "-i", "testsrc=duration=2:size=64x48:rate=10", "-pix_fmt", "yuv420p", out.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(st.success());
        let a = analyze_video_blocking(out.to_str().unwrap(), true, false, false, true).unwrap();
        assert_eq!((a.width, a.height), (64, 48));
        assert!((a.fps - 10.0).abs() < 0.01);
        assert!((a.duration_seconds - 2.0).abs() < 0.2);
        assert!(!a.keyframes.is_empty());
        let f = extract_frame_blocking(out.to_str().unwrap(), 0.5, ImageFormat::PNG).unwrap();
        assert_eq!((f.width, f.height), (64, 48));
        assert!(f.content_hash_fnv64.is_some() && f.path.is_some());
        let _ = std::fs::remove_file(&out);
    }
}
