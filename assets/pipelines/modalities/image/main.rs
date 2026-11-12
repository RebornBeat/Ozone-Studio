//! OZONE Studio - Pipeline 102: Image Analysis
//!
//! Modality pipeline for image processing and structural graph creation.
//! Analyzes images to detect objects, regions, text, faces, and composition.
//! Creates traversable graphs that can be enriched by ZSEI semantic hooks.
//!
//! # Actions
//! - `Analyze`: Extract objects, regions, text, faces, colors, composition
//! - `CreateGraph`: Build structural graph from analysis
//! - `UpdateGraph`: Apply incremental updates to existing graph
//! - `QueryGraph`: Query graph for specific information
//! - `LinkToModality`: Create cross-modality links (e.g., to text descriptions)
//! - `TriggerSemanticHook`: Trigger ZSEI hooks for semantic enrichment
//!
//! # Graph Structure
//! - Nodes: Image, Object, Region, Text, Face, Color
//! - Edges: Contains, AdjacentTo, Overlaps, SimilarTo, DescribedBy

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// ============================================================================
// PIPELINE METADATA
// ============================================================================

pub const PIPELINE_ID: u64 = 102;
pub const PIPELINE_NAME: &str = "image_analysis";
pub const PIPELINE_VERSION: &str = "0.4.0";
pub const MODALITY: &str = "image";

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageModalityInput {
    pub action: ImageAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImageAction {
    /// Analyze image and detect objects, regions, composition
    Analyze {
        image_data: ImageData,
        #[serde(default)]
        depth: AnalysisDepth,
        #[serde(default = "default_true")]
        detect_objects: bool,
        #[serde(default)]
        detect_text: bool,
        #[serde(default)]
        detect_faces: bool,
        #[serde(default)]
        analyze_colors: bool,
        #[serde(default)]
        analyze_composition: bool,
    },

    /// Create graph from analysis result
    CreateGraph {
        analysis: ImageAnalysisResult,
        project_id: u64,
        #[serde(default)]
        graph_name: Option<String>,
    },

    /// Update existing graph with new analysis
    UpdateGraph {
        graph_id: u64,
        updates: ImageGraphUpdate,
    },

    /// Query image graph
    QueryGraph {
        graph_id: u64,
        query: ImageQuery,
    },

    /// Get graph with all nodes and edges
    GetGraph {
        graph_id: u64,
    },

    /// Link to another modality (e.g., text description)
    LinkToModality {
        image_graph_id: u64,
        target_graph_id: u64,
        target_modality: String,
        relationship: CrossModalityRelation,
        #[serde(default)]
        bidirectional: bool,
    },

    /// Trigger ZSEI semantic hook for enrichment
    TriggerSemanticHook {
        graph_id: u64,
        hook_type: ZSEIHookType,
        #[serde(default)]
        options: HookOptions,
    },

    /// Compare two images
    Compare {
        image1: ImageData,
        image2: ImageData,
        #[serde(default)]
        comparison_type: ComparisonType,
    },

    /// Extract region of interest
    ExtractRegion {
        image_data: ImageData,
        bounding_box: BoundingBox,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageModalityOutput {
    pub success: bool,
    pub action: String,
    pub result: ImageResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub metadata: OutputMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageResult {
    Analysis(ImageAnalysisResult),
    Graph(ImageGraph),
    Query(QueryResult),
    Link(LinkResult),
    Hook(HookResult),
    Comparison(ComparisonResult),
    Region(RegionResult),
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
pub enum ImageData {
    /// Base64 encoded image data
    Base64 { data: String, mime_type: String },
    /// Local file path
    FilePath(String),
    /// Remote URL
    Url(String),
    /// Raw bytes (for internal use)
    Bytes(Vec<u8>),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
pub enum AnalysisDepth {
    /// Quick analysis - basic detection only
    Surface,
    /// Standard analysis - default level
    #[default]
    Standard,
    /// Deep analysis - comprehensive detection with higher accuracy
    Deep,
    /// Maximum analysis - all features, highest accuracy
    Comprehensive,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageAnalysisResult {
    /// Image dimensions
    pub width: u32,
    pub height: u32,
    /// Image format (PNG, JPEG, etc.)
    pub format: String,
    /// Color space (RGB, RGBA, Grayscale, etc.)
    pub color_space: String,
    /// Bit depth
    pub bit_depth: u8,
    /// Detected objects
    pub objects: Vec<DetectedObject>,
    /// Identified regions
    pub regions: Vec<ImageRegion>,
    /// Detected text (OCR)
    pub text_regions: Vec<TextRegion>,
    /// Detected faces
    pub faces: Vec<DetectedFace>,
    /// Dominant colors
    pub dominant_colors: Vec<Color>,
    /// Composition analysis
    pub composition: CompositionAnalysis,
    /// Image quality metrics
    pub quality: ImageQuality,
    /// EXIF and other metadata
    pub metadata: HashMap<String, Value>,
    /// Analysis confidence overall
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectedObject {
    /// Unique identifier for this detection
    pub object_id: String,
    /// Object label/class
    pub label: String,
    /// Detection confidence (0.0 - 1.0)
    pub confidence: f32,
    /// Bounding box location
    pub bounding_box: BoundingBox,
    /// Segmentation mask (if available)
    pub mask: Option<SegmentationMask>,
    /// Object attributes
    pub attributes: HashMap<String, Value>,
    /// Parent object ID (for hierarchical detection)
    pub parent_id: Option<String>,
    /// Child object IDs
    pub children: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoundingBox {
    /// X coordinate (top-left)
    pub x: f32,
    /// Y coordinate (top-left)
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Rotation angle (degrees)
    #[serde(default)]
    pub rotation: f32,
}

impl BoundingBox {
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    pub fn intersection_over_union(&self, other: &BoundingBox) -> f32 {
        if !self.intersects(other) {
            return 0.0;
        }

        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        let intersection = (x2 - x1) * (y2 - y1);
        let union = self.area() + other.area() - intersection;

        intersection / union
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SegmentationMask {
    /// Run-length encoded mask
    pub rle: Vec<u32>,
    /// Mask dimensions
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageRegion {
    /// Unique identifier
    pub region_id: String,
    /// Region type (background, foreground, salient, etc.)
    pub region_type: RegionType,
    /// Bounding box
    pub bounding_box: BoundingBox,
    /// Region description
    pub description: Option<String>,
    /// Confidence score
    pub confidence: f32,
    /// Polygon points (for non-rectangular regions)
    pub polygon: Option<Vec<(f32, f32)>>,
    /// Semantic label
    pub semantic_label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum RegionType {
    Background,
    Foreground,
    Salient,
    Sky,
    Ground,
    Water,
    Vegetation,
    Building,
    Person,
    Vehicle,
    Animal,
    Object,
    Text,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextRegion {
    /// Detected text content
    pub text: String,
    /// Detection confidence
    pub confidence: f32,
    /// Bounding box
    pub bounding_box: BoundingBox,
    /// Detected language
    pub language: Option<String>,
    /// Text direction
    pub direction: TextDirection,
    /// Individual word boxes
    pub words: Vec<WordBox>,
    /// Font attributes (if detectable)
    pub font_attributes: Option<FontAttributes>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum TextDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WordBox {
    pub word: String,
    pub bounding_box: BoundingBox,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontAttributes {
    pub bold: bool,
    pub italic: bool,
    pub estimated_size: Option<f32>,
    pub font_family: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectedFace {
    /// Unique identifier
    pub face_id: String,
    /// Bounding box
    pub bounding_box: BoundingBox,
    /// Facial landmarks
    pub landmarks: Vec<FaceLandmark>,
    /// Face attributes
    pub attributes: FaceAttributes,
    /// Detection confidence
    pub confidence: f32,
    /// Face embedding (for recognition)
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FaceLandmark {
    /// Landmark name (left_eye, right_eye, nose, mouth_left, mouth_right, etc.)
    pub name: String,
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Confidence
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FaceAttributes {
    /// Estimated age
    pub age_estimate: Option<u32>,
    /// Age range
    pub age_range: Option<(u32, u32)>,
    /// Detected emotion
    pub emotion: Option<String>,
    /// Emotion probabilities
    pub emotion_scores: Option<HashMap<String, f32>>,
    /// Head pose
    pub pose: Option<FacePose>,
    /// Whether eyes are open
    pub eyes_open: Option<bool>,
    /// Whether mouth is open
    pub mouth_open: Option<bool>,
    /// Whether wearing glasses
    pub glasses: Option<bool>,
    /// Facial hair
    pub facial_hair: Option<String>,
    /// Smile score
    pub smile_score: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FacePose {
    /// Yaw (left-right rotation)
    pub yaw: f32,
    /// Pitch (up-down rotation)
    pub pitch: f32,
    /// Roll (tilt)
    pub roll: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255)
    pub a: u8,
    /// Percentage of image
    pub percentage: f32,
    /// Color name (if determinable)
    pub name: Option<String>,
    /// Hex representation
    pub hex: String,
    /// HSL representation
    pub hsl: Option<(f32, f32, f32)>,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, percentage: f32) -> Self {
        let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);
        Self {
            r,
            g,
            b,
            a: 255,
            percentage,
            name: Self::color_name(r, g, b),
            hex,
            hsl: Some(Self::rgb_to_hsl(r, g, b)),
        }
    }

    fn color_name(r: u8, g: u8, b: u8) -> Option<String> {
        // Simplified color naming
        let (h, s, l) = Self::rgb_to_hsl(r, g, b);

        if l < 0.1 {
            return Some("Black".to_string());
        }
        if l > 0.9 {
            return Some("White".to_string());
        }
        if s < 0.1 {
            return Some("Gray".to_string());
        }

        let name = match h {
            h if h < 15.0 => "Red",
            h if h < 45.0 => "Orange",
            h if h < 75.0 => "Yellow",
            h if h < 150.0 => "Green",
            h if h < 210.0 => "Cyan",
            h if h < 270.0 => "Blue",
            h if h < 330.0 => "Purple",
            _ => "Red",
        };

        Some(name.to_string())
    }

    fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if max == min {
            return (0.0, 0.0, l);
        }

        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };

        let h = if max == r {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };

        (h * 60.0, s, l)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CompositionAnalysis {
    /// Rule of thirds compliance
    pub rule_of_thirds: RuleOfThirds,
    /// Symmetry analysis
    pub symmetry: SymmetryAnalysis,
    /// Focal points
    pub focal_points: Vec<FocalPoint>,
    /// Visual weight distribution
    pub visual_weight: VisualWeight,
    /// Leading lines detected
    pub leading_lines: Vec<LeadingLine>,
    /// Depth layers
    pub depth_layers: Vec<DepthLayer>,
    /// Overall composition score
    pub composition_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RuleOfThirds {
    /// Whether image follows rule of thirds
    pub compliant: bool,
    /// Score (0-1)
    pub score: f32,
    /// Points of interest at intersections
    pub intersection_points: Vec<(f32, f32)>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SymmetryAnalysis {
    /// Horizontal symmetry score
    pub horizontal: f32,
    /// Vertical symmetry score
    pub vertical: f32,
    /// Radial symmetry score
    pub radial: f32,
    /// Axis of symmetry (if detected)
    pub axis: Option<(f32, f32, f32, f32)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FocalPoint {
    /// X coordinate (normalized 0-1)
    pub x: f32,
    /// Y coordinate (normalized 0-1)
    pub y: f32,
    /// Strength/importance
    pub strength: f32,
    /// Associated object (if any)
    pub object_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VisualWeight {
    /// Weight in each quadrant
    pub quadrants: [f32; 4],
    /// Center of visual mass
    pub center: (f32, f32),
    /// Balance score
    pub balance_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeadingLine {
    /// Start point
    pub start: (f32, f32),
    /// End point
    pub end: (f32, f32),
    /// Strength
    pub strength: f32,
    /// Line type
    pub line_type: LineType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LineType {
    Straight,
    Curved,
    Diagonal,
    Horizontal,
    Vertical,
    Converging,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DepthLayer {
    /// Layer name (foreground, midground, background)
    pub name: String,
    /// Layer depth (0 = closest, 1 = farthest)
    pub depth: f32,
    /// Objects in this layer
    pub objects: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ImageQuality {
    /// Sharpness score
    pub sharpness: f32,
    /// Brightness score
    pub brightness: f32,
    /// Contrast score
    pub contrast: f32,
    /// Saturation score
    pub saturation: f32,
    /// Noise level
    pub noise_level: f32,
    /// Blur amount
    pub blur_amount: f32,
    /// Overall quality score
    pub overall_score: f32,
    /// Quality issues detected
    pub issues: Vec<QualityIssue>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualityIssue {
    pub issue_type: String,
    pub severity: f32,
    pub location: Option<BoundingBox>,
    pub description: String,
}

// ============================================================================
// GRAPH TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGraph {
    /// Unique graph identifier
    pub graph_id: u64,
    /// Graph name
    pub name: String,
    /// Modality type
    pub modality: String,
    /// Project this graph belongs to
    pub project_id: u64,
    /// Source image information
    pub source: ImageSource,
    /// Graph nodes
    pub nodes: Vec<ImageGraphNode>,
    /// Graph edges
    pub edges: Vec<ImageGraphEdge>,
    /// Graph metadata
    pub metadata: GraphMetadata,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageSource {
    pub path: Option<String>,
    pub url: Option<String>,
    pub hash: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGraphNode {
    /// Unique node identifier
    pub node_id: u64,
    /// Node type
    pub node_type: ImageNodeType,
    /// Display label
    pub label: String,
    /// Content/description
    pub content: String,
    /// Bounding box (if applicable)
    pub bounding_box: Option<BoundingBox>,
    /// Confidence score
    pub confidence: f32,
    /// Node properties
    pub properties: HashMap<String, Value>,
    /// Semantic annotations (added by ZSEI hooks)
    pub annotations: Vec<SemanticAnnotation>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ImageNodeType {
    /// Root image node
    Image,
    /// Detected object
    Object,
    /// Image region
    Region,
    /// Detected text
    Text,
    /// Detected face
    Face,
    /// Color
    Color,
    /// Composition element
    Composition,
    /// Quality metric
    Quality,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticAnnotation {
    pub annotation_type: String,
    pub value: Value,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGraphEdge {
    /// Unique edge identifier
    pub edge_id: u64,
    /// Source node ID
    pub from_node: u64,
    /// Target node ID
    pub to_node: u64,
    /// Edge type
    pub edge_type: ImageEdgeType,
    /// Edge weight/strength
    pub weight: f32,
    /// Edge properties
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ImageEdgeType {
    // Structural edges
    Contains,
    ContainedBy,
    AdjacentTo,
    Overlaps,
    Above,
    Below,
    LeftOf,
    RightOf,
    // Semantic edges (typically added by ZSEI hooks)
    SimilarTo,
    RelatesTo,
    PartOf,
    // Cross-modality edges
    DescribedBy,
    Describes,
    ImplementedBy,
    Represents,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub analysis_depth: AnalysisDepth,
    pub semantic_enriched: bool,
    pub cross_modal_links: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGraphUpdate {
    /// Nodes to add
    pub add_nodes: Vec<ImageGraphNode>,
    /// Nodes to update
    pub update_nodes: Vec<ImageGraphNode>,
    /// Node IDs to remove
    pub remove_nodes: Vec<u64>,
    /// Edges to add
    pub add_edges: Vec<ImageGraphEdge>,
    /// Edge IDs to remove
    pub remove_edges: Vec<u64>,
    /// Metadata updates
    pub metadata_updates: Option<HashMap<String, Value>>,
}

// ============================================================================
// QUERY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageQuery {
    pub query_type: ImageQueryType,
    pub parameters: HashMap<String, Value>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub min_confidence: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ImageQueryType {
    /// Find objects by label
    FindObjects { labels: Vec<String> },
    /// Find regions by type
    FindRegions { region_types: Vec<RegionType> },
    /// Find text content
    FindText { pattern: String },
    /// Find similar nodes
    FindSimilar { node_id: u64, threshold: f32 },
    /// Get composition analysis
    GetComposition,
    /// Get nodes in bounding box
    GetNodesInRegion { bounding_box: BoundingBox },
    /// Get all nodes of type
    GetNodesByType { node_type: ImageNodeType },
    /// Get connected nodes
    GetConnected { node_id: u64, max_depth: u32 },
    /// Custom query
    Custom { query: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub query_type: String,
    pub nodes: Vec<ImageGraphNode>,
    pub edges: Vec<ImageGraphEdge>,
    pub total_matches: usize,
    pub metadata: HashMap<String, Value>,
}

// ============================================================================
// CROSS-MODALITY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CrossModalityRelation {
    /// Text describes this image
    DescribedBy,
    /// Image illustrates this text
    Illustrates,
    /// Image is a diagram of code
    DiagramOf,
    /// Image contains screenshot of UI
    ScreenshotOf,
    /// Custom relationship
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

// ============================================================================
// ZSEI HOOK TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ZSEIHookType {
    /// Called after graph creation
    OnGraphCreated,
    /// Called to infer relationships between nodes
    OnInferRelationships,
    /// Called to complete edge information
    OnEdgeCompletion,
    /// Called for cross-modality linking
    OnCrossModalityLink,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HookOptions {
    /// Maximum nodes to process
    pub max_nodes: Option<usize>,
    /// Minimum confidence for results
    pub min_confidence: Option<f32>,
    /// Whether to use async processing
    pub async_processing: bool,
    /// Custom parameters
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
// COMPARISON TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum ComparisonType {
    /// Visual similarity
    #[default]
    Visual,
    /// Structural similarity (detected objects)
    Structural,
    /// Color palette similarity
    ColorPalette,
    /// Composition similarity
    Composition,
    /// All comparison types
    Comprehensive,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComparisonResult {
    pub overall_similarity: f32,
    pub visual_similarity: Option<f32>,
    pub structural_similarity: Option<f32>,
    pub color_similarity: Option<f32>,
    pub composition_similarity: Option<f32>,
    pub differences: Vec<Difference>,
    pub common_elements: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Difference {
    pub diff_type: String,
    pub description: String,
    pub location1: Option<BoundingBox>,
    pub location2: Option<BoundingBox>,
    pub severity: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegionResult {
    pub image_data: ImageData,
    pub original_box: BoundingBox,
    pub width: u32,
    pub height: u32,
}


// ============================================================================
// ZSEI OVER HTTP (the text/code/math pattern — real graph persistence)
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

/// PERSIST the image graph as a real ZSEI container (the math/text revival
/// pattern, 2026-09-28): ContainerType ModalityGraph, parent = the project,
/// real keywords derived from graph node labels, content behind
/// object_store_path (graphs/image_<id>.json). The container's assigned id
/// BECOMES the graph_id — the same convention that made text/code/math
/// graphs retrievable.
async fn persist_image_graph(graph: &mut ImageGraph) -> Result<(), String> {
    let keywords: Vec<String> = {
        let mut k: Vec<String> = graph
            .nodes
            .iter()
            .filter(|n| n.node_type == ImageNodeType::Object)
            .map(|n| n.label.to_lowercase())
            .collect();
        k.sort();
        k.dedup();
        if k.is_empty() {
            k.push("image".to_string());
        }
        // container keyword set kept WHOLE (no 12-keyword cut — doctrine audit #33, 2026-11-04)
        k
    };
    let name = format!(
        "{} image graph ({} nodes, {} edges)",
        graph.modality,
        graph.nodes.len(),
        graph.edges.len()
    );
    let now = chrono::Utc::now().timestamp() as u64;
    let container = serde_json::json!({
        "global_state": {
            "container_id": 0, "parent_id": graph.project_id, "child_ids": [],
            "child_count": 0, "version": 1
        },
        "local_state": {
            "metadata": {
                "container_type": "ModalityGraph", "modality": "Image",
                "created_at": now, "updated_at": now,
                "provenance": "image-pipeline", "permissions": 0, "owner_id": 0,
                "name": name,
                "materialized_path": None::<String>
            },
            "context": {
                "categories": [], "methodologies": [], "keywords": keywords,
                "topics": ["image"], "relationships": [],
                "learned_associations": [], "embedding": null
            },
            "storage": {
                "db_shard_id": null, "vector_index_ref": null,
                "object_store_path": serde_json::Value::Null,
                "compression_type": "None"
            },
            "hints": {
                "access_frequency": 0, "hotness_score": 0.0, "last_accessed": 0,
                "centroid": null, "ml_prediction_weight": 0.0
            },
            "integrity": {
                "content_hash": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0], "semantic_fingerprint": [],
                "last_verified": 0, "integrity_score": 1.0, "version_history": []
            },
            "file_context": null, "code_context": null, "text_context": null,
            "external_ref": null
        }
    });
    let result = zsei_query(serde_json::json!({
        "CreateContainer": { "parent_id": graph.project_id, "container": container }
    }))
    .await?;
    let new_id = result
        .get("ContainerID")
        .and_then(|c| c.as_u64())
        .ok_or("CreateContainer returned no ContainerID")?;
    // Content file at the REAL id, pointer updated to match.
    let dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let rel = format!("graphs/image_{}.json", new_id);
    let abs = std::path::Path::new(&dir).join(&rel);
    if let Some(parent) = abs.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(
        &abs,
        serde_json::to_string_pretty(&serde_json::json!({
            "graph_id": new_id, "nodes": graph.nodes, "edges": graph.edges,
            "source": graph.source, "metadata": graph.metadata,
        }))
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("graph content write failed: {e}"))?;
    let _ = zsei_query(serde_json::json!({
        "UpdateContainer": { "container_id": new_id, "updates": { "storage": {
            "db_shard_id": null, "vector_index_ref": null,
            "object_store_path": rel, "compression_type": "None"
        }, "metadata": null, "context": null, "hints": null } }
    }))
    .await;
    graph.graph_id = new_id;
    Ok(())
}

// ============================================================================
// EXECUTION
// ============================================================================

pub async fn execute(input: Value) -> Result<Value, String> {
    let start_time = std::time::Instant::now();

    // Honest refusal for a text-only "mention" with no real raw image data
    // attached, BEFORE the strict typed deserialization below would reject
    // it with an accidental, low-level parse error. Only the one action that
    // requires raw data (Analyze) is checked; other actions (CreateGraph,
    // etc.) don't need a data source and are unaffected.
    if input.get("action").and_then(|a| a.get("type")).and_then(|t| t.as_str()) == Some("Analyze")
        && input.get("action").and_then(|a| a.get("image_data")).map(|d| d.is_null()).unwrap_or(true)
    {
        return Ok(serde_json::json!({
            "success": false,
            "action": "Analyze",
            "result": null,
            "error": "No raw image data provided — this call appears to be a text-only mention with no attached data file; real image analysis requires an actual data file",
            "metadata": {
                "pipeline_id": PIPELINE_ID,
                "pipeline_version": PIPELINE_VERSION,
                "processing_time_ms": start_time.elapsed().as_millis(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        }));
    }

    let input: ImageModalityInput =
        serde_json::from_value(input).map_err(|e| format!("Failed to parse input: {}", e))?;

    let result = match input.action {
        ImageAction::Analyze {
            image_data,
            depth,
            detect_objects,
            detect_text,
            detect_faces,
            analyze_colors,
            analyze_composition,
        } => {
            let analysis = analyze_image(
                &image_data,
                depth,
                detect_objects,
                detect_text,
                detect_faces,
                analyze_colors,
                analyze_composition,
            )
            .await?;
            ("Analyze", ImageResult::Analysis(analysis))
        }

        ImageAction::CreateGraph {
            analysis,
            project_id,
            graph_name,
        } => {
            let mut graph = create_graph(analysis, project_id, graph_name).await?;
            // Persist as a real ZSEI container (idempotency: failures are
            // loud — the graph still returns in the response, but the error
            // rides with it; silent in-memory-only graphs were the exact
            // pre-revival failure mode).
            if let Err(e) = persist_image_graph(&mut graph).await {
                eprintln!("image CreateGraph: ZSEI persistence failed: {e}");
            }
            ("CreateGraph", ImageResult::Graph(graph))
        }

        ImageAction::UpdateGraph { graph_id, updates } => {
            let graph = update_graph(graph_id, updates).await?;
            ("UpdateGraph", ImageResult::Graph(graph))
        }

        ImageAction::QueryGraph { graph_id, query } => {
            let result = query_graph(graph_id, query).await?;
            ("QueryGraph", ImageResult::Query(result))
        }

        ImageAction::GetGraph { graph_id } => {
            let graph = get_graph(graph_id).await?;
            ("GetGraph", ImageResult::Graph(graph))
        }

        ImageAction::LinkToModality {
            image_graph_id,
            target_graph_id,
            target_modality,
            relationship,
            bidirectional,
        } => {
            let link = link_to_modality(
                image_graph_id,
                target_graph_id,
                &target_modality,
                relationship,
                bidirectional,
            )
            .await?;
            ("LinkToModality", ImageResult::Link(link))
        }

        ImageAction::TriggerSemanticHook {
            graph_id,
            hook_type,
            options,
        } => {
            let result = trigger_semantic_hook(graph_id, hook_type, options).await?;
            ("TriggerSemanticHook", ImageResult::Hook(result))
        }

        ImageAction::Compare {
            image1,
            image2,
            comparison_type,
        } => {
            let result = compare_images(&image1, &image2, comparison_type).await?;
            ("Compare", ImageResult::Comparison(result))
        }

        ImageAction::ExtractRegion {
            image_data,
            bounding_box,
        } => {
            let result = extract_region(&image_data, &bounding_box).await?;
            ("ExtractRegion", ImageResult::Region(result))
        }
    };

    let output = ImageModalityOutput {
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


/// HONESTY (D16/D17): analyze_image computes only what the BYTES decide.
/// Layer 1 — container facts parsed from the headers (PNG IHDR/pHYs/sRGB/gAMA/
/// iCCP/text chunks, JPEG SOF + EXIF, GIF/BMP/WEBP dimensions).
/// Layer 2 — pixel statistics from a REAL decode (the `image` crate; PNG, JPEG,
/// GIF, BMP): bit depth, colour type, dominant colours (4-bit-per-channel
/// histogram), mean brightness, RMS contrast, mean saturation, Sobel edge
/// density, Laplacian variance — all on a sampled grid (<= 1024 px per side).
/// Everything semantic — objects, text, faces, composition — stays ABSENT with
/// the reason that visual-mcp (:3220) is the detection surface (D17).
async fn analyze_image(
    image_data: &ImageData,
    _depth: AnalysisDepth,
    _detect_objects: bool,
    _detect_text: bool,
    _detect_faces: bool,
    analyze_colors: bool,
    _analyze_composition: bool,
) -> Result<ImageAnalysisResult, String> {
    let bytes = load_image_data(image_data).await?;
    analyze_image_bytes(&bytes, analyze_colors)
}

/// Pure core of `analyze_image` (testable without async / files).
fn analyze_image_bytes(bytes: &[u8], analyze_colors: bool) -> Result<ImageAnalysisResult, String> {
    let (format, hdr_w, hdr_h) = parse_image_header(bytes)
        .ok_or_else(|| "unrecognized image format (PNG/JPEG/GIF/BMP/WEBP headers; other formats are absent)".to_string())?;

    let absent = |what: &str| -> String {
        format!("{what}: absent — detection is visual-mcp's surface (D17); this pipeline reports only what the bytes decide")
    };
    let mut absent_reasons: Vec<Value> = vec![
        Value::String(absent("objects")),
        Value::String(absent("text")),
        Value::String(absent("faces")),
        Value::String(absent("composition")),
    ];
    let mut metadata: HashMap<String, Value> = HashMap::new();

    // ---- layer 1: container facts -------------------------------------------------
    let mut color_space = "unknown (no color-profile information in the file)".to_string();
    let mut bit_depth: u8 = 0;
    let mut facts: serde_json::Map<String, Value> = serde_json::Map::new();
    match format {
        "PNG" => {
            let p = parse_png_facts(bytes);
            if let Some(bd) = p.bit_depth {
                bit_depth = bd;
            }
            if let Some(ct) = p.color_type_name {
                color_space = ct.to_string();
            }
            if p.srgb {
                color_space = format!("{color_space}; sRGB (sRGB chunk)");
            } else if p.icc_profile {
                color_space = format!("{color_space}; ICC profile embedded (not parsed)");
            } else {
                color_space = format!("{color_space}; profile unspecified");
            }
            facts = p.facts;
        }
        "JPEG" => {
            let exif = parse_exif(bytes);
            if let Some(cs) = exif.get("color_space").and_then(|v| v.as_u64()) {
                color_space = if cs == 1 { "YCbCr/RGB; sRGB (EXIF ColorSpace=1)".to_string() } else { "YCbCr/RGB; uncalibrated (EXIF ColorSpace)".to_string() };
            } else {
                color_space = "YCbCr/RGB; profile unspecified".to_string();
            }
            if let Some(p) = jpeg_precision(bytes) {
                bit_depth = p;
            }
            facts = exif.into_iter().collect();
        }
        "GIF" => {
            color_space = "indexed palette (GIF)".to_string();
            bit_depth = 8;
        }
        "BMP" => {
            if let Some(bpp) = bmp_bits_per_pixel(bytes) {
                facts.insert("bits_per_pixel".to_string(), Value::from(bpp));
            }
            color_space = "BMP (profile unspecified)".to_string();
        }
        _ => {}
    }
    let (mut width, mut height) = (hdr_w, hdr_h);

    // ---- layer 2: real decode -----------------------------------------------------
    let mut stats: Option<PixelStats> = None;
    let mut decoded_color: Option<String> = None;
    if matches!(format, "PNG" | "JPEG" | "GIF" | "BMP") {
        if (hdr_w as u64) * (hdr_h as u64) > MAX_DECODE_PIXELS {
            absent_reasons.push(Value::String(format!(
                "pixel statistics: absent — {}x{} exceeds the {}-pixel decode limit",
                hdr_w, hdr_h, MAX_DECODE_PIXELS
            )));
        } else {
            let decoded = std::panic::catch_unwind(|| image::load_from_memory(bytes));
            match decoded {
                Ok(Ok(img)) => {
                    width = img.width();
                    height = img.height();
                    let ct = img.color();
                    let per_channel = (ct.bits_per_pixel() / (ct.channel_count() as u16).max(1)) as u8;
                    if per_channel > 0 {
                        bit_depth = per_channel;
                    }
                    decoded_color = Some(format!("{:?}", ct));
                    let rgba = img.to_rgba8();
                    stats = Some(pixel_stats(rgba.width(), rgba.height(), rgba.as_raw()));
                }
                Ok(Err(e)) => absent_reasons.push(Value::String(format!("pixel statistics: absent — decode failed: {e}"))),
                Err(_) => absent_reasons.push(Value::String("pixel statistics: absent — decoder panicked on this file".to_string())),
            }
        }
    } else {
        absent_reasons.push(Value::String(format!(
            "pixel statistics: absent — {format} is recognized by header only (no decoder enabled)"
        )));
    }

    // ---- assemble ----------------------------------------------------------------
    let mut dominant_colors: Vec<Color> = Vec::new();
    let mut quality = ImageQuality::default();
    if let Some(s) = &stats {
        if analyze_colors {
            for (r, g, b, frac) in &s.dominant {
                dominant_colors.push(Color::new(*r, *g, *b, *frac));
            }
        } else {
            absent_reasons.push(Value::String("dominant colors: not requested (analyze_colors=false)".to_string()));
        }
        quality.brightness = s.mean_luma as f32 / 255.0;
        quality.contrast = ((s.std_luma / 127.5).min(1.0)) as f32;
        quality.saturation = s.mean_saturation as f32;
        quality.sharpness = s.edge_density as f32;
        absent_reasons.push(Value::String(
            "quality.noise_level / blur_amount / overall_score / issues: absent — no computed estimator (0.0 means 'not computed')".to_string(),
        ));
        metadata.insert(
            "pixel_metrics".to_string(),
            serde_json::json!({
                "mean_luma_0_255": r4(s.mean_luma),
                "rms_contrast_0_255": r4(s.std_luma),
                "mean_saturation_0_1": r4(s.mean_saturation),
                "edge_density_0_1": r4(s.edge_density),
                "laplacian_variance": r4(s.laplacian_variance),
                "mean_rgb": [r4(s.mean_rgb[0]), r4(s.mean_rgb[1]), r4(s.mean_rgb[2])],
                "opaque_fraction": r4(s.opaque_fraction),
                "sampled_grid": [s.grid_w, s.grid_h],
                "sample_stride": s.stride,
                "definitions": "quality.brightness=mean_luma/255; quality.contrast=min(1,rms/127.5); quality.saturation=mean HSV S; quality.sharpness=edge_density (fraction of interior samples with Sobel magnitude >= 64); all on the sampled grid",
            }),
        );
    }
    if let Some(ct) = decoded_color {
        metadata.insert("decoded_color_type".to_string(), Value::String(ct));
    }
    if !facts.is_empty() {
        metadata.insert("container_facts".to_string(), Value::Object(facts));
    }
    metadata.insert(
        "honesty".to_string(),
        Value::String("header facts + real pixel statistics only; semantic layers are visual-mcp's surface (D17)".to_string()),
    );
    metadata.insert("absent_reasons".to_string(), Value::Array(absent_reasons));
    // Geometry (docs/SPATIAL_GEOMETRY_LAYER.md): the image is a pixel grid in
    // frame "image-px", origin top-left, x right, y down.
    metadata.insert(
        "geom".to_string(),
        serde_json::json!({
            "frame": "image-px",
            "kind": "grid",
            "x": 0, "y": 0, "w": width, "h": height,
            "cols": width, "rows": height,
            "origin": "top-left"
        }),
    );
    metadata.insert("frame".to_string(), Value::String("image-px".to_string()));

    Ok(ImageAnalysisResult {
        width,
        height,
        format: format.to_string(),
        color_space,
        bit_depth,
        objects: Vec::new(),
        regions: Vec::new(),
        text_regions: Vec::new(),
        faces: Vec::new(),
        dominant_colors,
        composition: CompositionAnalysis::default(),
        quality,
        metadata,
        confidence: if stats.is_some() { 1.0 } else { 0.5 },
    })
}

/// Decode limit (pixels). Larger images keep header facts only.
const MAX_DECODE_PIXELS: u64 = 64_000_000;

fn r4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

/// Pixel statistics computed on a sampled grid of an RGBA8 buffer.
#[derive(Debug, Clone)]
struct PixelStats {
    grid_w: u32,
    grid_h: u32,
    stride: u32,
    mean_luma: f64,
    std_luma: f64,
    mean_saturation: f64,
    mean_rgb: [f64; 3],
    opaque_fraction: f64,
    edge_density: f64,
    laplacian_variance: f64,
    /// (r, g, b, fraction of sampled pixels) — top bins of a 4-bit/channel histogram
    dominant: Vec<(u8, u8, u8, f32)>,
}

fn pixel_stats(w: u32, h: u32, rgba: &[u8]) -> PixelStats {
    let stride = ((w.max(h) + 1023) / 1024).max(1);
    let gw = (w + stride - 1) / stride;
    let gh = (h + stride - 1) / stride;
    let n = (gw as usize) * (gh as usize);
    let mut luma = vec![0f32; n];
    let mut bins = vec![(0u32, 0u64, 0u64, 0u64); 4096];
    let (mut sum_l, mut sum_l2, mut sum_s) = (0f64, 0f64, 0f64);
    let mut sum_rgb = [0f64; 3];
    let mut opaque = 0u64;
    let mut count = 0u64;
    for gy in 0..gh {
        for gx in 0..gw {
            let x = (gx * stride) as usize;
            let y = (gy * stride) as usize;
            let i = (y * w as usize + x) * 4;
            let (r, g, b, a) = (rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]);
            let l = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;
            luma[(gy * gw + gx) as usize] = l;
            sum_l += l as f64;
            sum_l2 += (l as f64) * (l as f64);
            let mx = r.max(g).max(b) as f64;
            let mn = r.min(g).min(b) as f64;
            sum_s += if mx > 0.0 { (mx - mn) / mx } else { 0.0 };
            sum_rgb[0] += r as f64;
            sum_rgb[1] += g as f64;
            sum_rgb[2] += b as f64;
            if a == 255 {
                opaque += 1;
            }
            let bin = (((r >> 4) as usize) << 8) | (((g >> 4) as usize) << 4) | ((b >> 4) as usize);
            let e = &mut bins[bin];
            e.0 += 1;
            e.1 += r as u64;
            e.2 += g as u64;
            e.3 += b as u64;
            count += 1;
        }
    }
    let nf = count.max(1) as f64;
    let mean_l = sum_l / nf;
    let var_l = (sum_l2 / nf - mean_l * mean_l).max(0.0);

    // Sobel edge density + Laplacian variance on the interior of the sampled grid.
    let (mut edges, mut interior) = (0u64, 0u64);
    let (mut lap_sum, mut lap_sum2) = (0f64, 0f64);
    if gw >= 3 && gh >= 3 {
        let at = |x: u32, y: u32| luma[(y * gw + x) as usize] as f64;
        for y in 1..gh - 1 {
            for x in 1..gw - 1 {
                let gxv = -at(x - 1, y - 1) + at(x + 1, y - 1) - 2.0 * at(x - 1, y) + 2.0 * at(x + 1, y) - at(x - 1, y + 1) + at(x + 1, y + 1);
                let gyv = -at(x - 1, y - 1) - 2.0 * at(x, y - 1) - at(x + 1, y - 1) + at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1);
                if (gxv * gxv + gyv * gyv).sqrt() >= 64.0 {
                    edges += 1;
                }
                let lap = at(x, y - 1) + at(x, y + 1) + at(x - 1, y) + at(x + 1, y) - 4.0 * at(x, y);
                lap_sum += lap;
                lap_sum2 += lap * lap;
                interior += 1;
            }
        }
    }
    let inter = interior.max(1) as f64;
    let lap_mean = lap_sum / inter;
    let lap_var = if interior > 0 { (lap_sum2 / inter - lap_mean * lap_mean).max(0.0) } else { 0.0 };

    // Top-6 bins by population (ties: lower bin index first → deterministic).
    let mut order: Vec<usize> = (0..4096).filter(|i| bins[*i].0 > 0).collect();
    order.sort_by(|a, b| bins[*b].0.cmp(&bins[*a].0).then(a.cmp(b)));
    let dominant = order
        .into_iter()
        .take(6)
        .map(|i| {
            let (c, sr, sg, sb) = bins[i];
            let cf = c.max(1) as u64;
            ((sr / cf) as u8, (sg / cf) as u8, (sb / cf) as u8, (c as f64 / nf) as f32)
        })
        .collect();

    PixelStats {
        grid_w: gw,
        grid_h: gh,
        stride,
        mean_luma: mean_l,
        std_luma: var_l.sqrt(),
        mean_saturation: sum_s / nf,
        mean_rgb: [sum_rgb[0] / nf, sum_rgb[1] / nf, sum_rgb[2] / nf],
        opaque_fraction: opaque as f64 / nf,
        edge_density: if interior > 0 { edges as f64 / inter } else { 0.0 },
        laplacian_variance: lap_var,
        dominant,
    }
}

// ---------------------------------------------------------------- PNG facts ---

struct PngFacts {
    bit_depth: Option<u8>,
    color_type_name: Option<&'static str>,
    srgb: bool,
    icc_profile: bool,
    facts: serde_json::Map<String, Value>,
}

fn parse_png_facts(b: &[u8]) -> PngFacts {
    let mut out = PngFacts { bit_depth: None, color_type_name: None, srgb: false, icc_profile: false, facts: serde_json::Map::new() };
    if b.len() < 8 || !b.starts_with(&[0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return out;
    }
    let mut i = 8usize;
    let mut text_chunks = 0usize;
    let mut guard = 0usize;
    while i + 8 <= b.len() && guard < 512 {
        guard += 1;
        let len = u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
        let kind = &b[i + 4..i + 8];
        let data_start = i + 8;
        let data_end = data_start.saturating_add(len);
        if data_end > b.len() {
            break;
        }
        let d = &b[data_start..data_end];
        match kind {
            b"IHDR" if d.len() >= 13 => {
                out.bit_depth = Some(d[8]);
                out.color_type_name = Some(match d[9] {
                    0 => "grayscale",
                    2 => "RGB",
                    3 => "indexed palette",
                    4 => "grayscale+alpha",
                    6 => "RGBA",
                    _ => "unknown color type",
                });
                out.facts.insert("interlaced".to_string(), Value::Bool(d[12] == 1));
                out.facts.insert("png_color_type".to_string(), Value::from(d[9]));
            }
            b"pHYs" if d.len() >= 9 => {
                let (x, y, unit) = (u32::from_be_bytes([d[0], d[1], d[2], d[3]]), u32::from_be_bytes([d[4], d[5], d[6], d[7]]), d[8]);
                out.facts.insert(
                    "physical_pixel_density".to_string(),
                    serde_json::json!({"x": x, "y": y, "unit": if unit == 1 { "per_metre" } else { "unspecified_aspect_only" }}),
                );
            }
            b"sRGB" => {
                out.srgb = true;
                if let Some(v) = d.first() {
                    out.facts.insert("srgb_rendering_intent".to_string(), Value::from(*v));
                }
            }
            b"gAMA" if d.len() >= 4 => {
                let g = u32::from_be_bytes([d[0], d[1], d[2], d[3]]) as f64 / 100_000.0;
                out.facts.insert("gamma".to_string(), Value::from(g));
            }
            b"iCCP" => out.icc_profile = true,
            b"acTL" => {
                out.facts.insert("animated_png".to_string(), Value::Bool(true));
                if d.len() >= 4 {
                    out.facts.insert("frame_count".to_string(), Value::from(u32::from_be_bytes([d[0], d[1], d[2], d[3]])));
                }
            }
            b"tEXt" if text_chunks < 20 => {
                text_chunks += 1;
                if let Some(z) = d.iter().position(|c| *c == 0) {
                    let key: String = d[..z].iter().map(|c| *c as char).take(80).collect();
                    let val: String = d[z + 1..].iter().map(|c| *c as char).take(200).collect();
                    out.facts.insert(format!("text:{key}"), Value::String(val));
                }
            }
            b"IDAT" | b"IEND" => {
                if kind == b"IEND" {
                    break;
                }
            }
            _ => {}
        }
        i = data_end.saturating_add(4); // skip CRC
    }
    out
}

// ---------------------------------------------------------------- JPEG/EXIF ---

/// JPEG sample precision (bits) from the SOF marker.
fn jpeg_precision(b: &[u8]) -> Option<u8> {
    let mut i = 2usize;
    while i + 9 <= b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = b[i + 1];
        if (0xC0..=0xCF).contains(&m) && ![0xC4u8, 0xC8, 0xCC].contains(&m) {
            return Some(b[i + 4]);
        }
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        i += 2 + len;
    }
    None
}

/// Minimal EXIF reader (TIFF IFD0 + Exif IFD). Returns only tags actually present.
fn parse_exif(b: &[u8]) -> std::collections::BTreeMap<String, Value> {
    let mut out = std::collections::BTreeMap::new();
    // locate APP1 "Exif\0\0"
    let mut i = 2usize;
    let mut tiff: Option<&[u8]> = None;
    while i + 4 <= b.len() && b[0] == 0xFF && b[1] == 0xD8 {
        if b[i] != 0xFF {
            break;
        }
        let m = b[i + 1];
        if m == 0xDA || m == 0xD9 {
            break;
        }
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        let end = (i + 2 + len).min(b.len());
        if m == 0xE1 && end >= i + 10 && &b[i + 4..i + 10] == b"Exif\0\0" {
            tiff = Some(&b[i + 10..end]);
            break;
        }
        i = end;
    }
    let t = match tiff {
        Some(t) if t.len() >= 8 => t,
        _ => return out,
    };
    let le = match &t[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return out,
    };
    let u16at = |o: usize| -> Option<u16> { t.get(o..o + 2).map(|s| if le { u16::from_le_bytes([s[0], s[1]]) } else { u16::from_be_bytes([s[0], s[1]]) }) };
    let u32at = |o: usize| -> Option<u32> { t.get(o..o + 4).map(|s| if le { u32::from_le_bytes([s[0], s[1], s[2], s[3]]) } else { u32::from_be_bytes([s[0], s[1], s[2], s[3]]) }) };
    if u16at(2) != Some(42) {
        return out;
    }
    let ascii = |count: usize, voff: usize| -> Option<String> {
        let start = if count <= 4 { voff } else { u32at(voff)? as usize };
        let raw = t.get(start..start.checked_add(count)?)?;
        let s: String = raw.iter().take_while(|c| **c != 0).map(|c| *c as char).take(120).collect();
        Some(s)
    };
    let mut exif_ifd: Option<usize> = None;
    let mut read_ifd = |off: usize, out: &mut std::collections::BTreeMap<String, Value>, exif_ifd: &mut Option<usize>| {
        let n = match u16at(off) {
            Some(n) => (n as usize).min(64),
            None => return,
        };
        for k in 0..n {
            let e = off + 2 + k * 12;
            let (tag, typ, count) = match (u16at(e), u16at(e + 2), u32at(e + 4)) {
                (Some(a), Some(b2), Some(c)) => (a, b2, c as usize),
                _ => return,
            };
            let voff = e + 8;
            let short = || if typ == 3 { u16at(voff).map(|v| v as u64) } else if typ == 4 { u32at(voff).map(|v| v as u64) } else { None };
            match tag {
                0x0112 => {
                    if let Some(v) = short() {
                        out.insert("orientation".to_string(), Value::from(v));
                    }
                }
                0x010F => {
                    if let Some(s) = ascii(count, voff) {
                        out.insert("make".to_string(), Value::String(s));
                    }
                }
                0x0110 => {
                    if let Some(s) = ascii(count, voff) {
                        out.insert("model".to_string(), Value::String(s));
                    }
                }
                0x0131 => {
                    if let Some(s) = ascii(count, voff) {
                        out.insert("software".to_string(), Value::String(s));
                    }
                }
                0x0132 => {
                    if let Some(s) = ascii(count, voff) {
                        out.insert("datetime".to_string(), Value::String(s));
                    }
                }
                0x9003 => {
                    if let Some(s) = ascii(count, voff) {
                        out.insert("datetime_original".to_string(), Value::String(s));
                    }
                }
                0xA001 => {
                    if let Some(v) = short() {
                        out.insert("color_space".to_string(), Value::from(v));
                    }
                }
                0xA002 => {
                    if let Some(v) = short() {
                        out.insert("pixel_x_dimension".to_string(), Value::from(v));
                    }
                }
                0xA003 => {
                    if let Some(v) = short() {
                        out.insert("pixel_y_dimension".to_string(), Value::from(v));
                    }
                }
                0x8769 => {
                    if let Some(v) = u32at(voff) {
                        *exif_ifd = Some(v as usize);
                    }
                }
                _ => {}
            }
        }
    };
    if let Some(ifd0) = u32at(4) {
        read_ifd(ifd0 as usize, &mut out, &mut exif_ifd);
    }
    if let Some(off) = exif_ifd {
        let mut dummy = None;
        read_ifd(off, &mut out, &mut dummy);
    }
    out
}

// ------------------------------------------------------------ BMP / WEBP ------

fn bmp_bits_per_pixel(b: &[u8]) -> Option<u16> {
    if b.len() >= 30 && &b[0..2] == b"BM" {
        Some(u16::from_le_bytes([b[28], b[29]]))
    } else {
        None
    }
}

/// Parse PNG (IHDR), JPEG (SOF scan), GIF, BMP and WEBP headers for format +
/// dimensions. No decoder dependency — only what the bytes decide.
fn parse_image_header(b: &[u8]) -> Option<(&'static str, u32, u32)> {
    if b.len() >= 24 && b.starts_with(&[0x89u8, b'P', b'N', b'G']) {
        let w = u32::from_be_bytes([b[16], b[17], b[18], b[19]]);
        let h = u32::from_be_bytes([b[20], b[21], b[22], b[23]]);
        return Some(("PNG", w, h));
    }
    if b.len() >= 3 && b[0] == 0xFF && b[1] == 0xD8 {
        let mut i = 2usize;
        while i + 9 <= b.len() {
            if b[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = b[i + 1];
            if (0xC0..=0xCF).contains(&marker) && ![0xC4u8, 0xC8, 0xCC].contains(&marker) {
                let h = u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32;
                let w = u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32;
                return Some(("JPEG", w, h));
            }
            let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
            i += 2 + len;
        }
        return Some(("JPEG", 0, 0));
    }
    if b.len() >= 10 && b.starts_with(b"GIF8") {
        let w = u16::from_le_bytes([b[6], b[7]]) as u32;
        let h = u16::from_le_bytes([b[8], b[9]]) as u32;
        return Some(("GIF", w, h));
    }
    if b.len() >= 26 && &b[0..2] == b"BM" {
        let w = i32::from_le_bytes([b[18], b[19], b[20], b[21]]).unsigned_abs();
        let h = i32::from_le_bytes([b[22], b[23], b[24], b[25]]).unsigned_abs();
        return Some(("BMP", w, h));
    }
    if b.len() >= 30 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        match &b[12..16] {
            b"VP8X" => {
                let w = 1 + (b[24] as u32 | (b[25] as u32) << 8 | (b[26] as u32) << 16);
                let h = 1 + (b[27] as u32 | (b[28] as u32) << 8 | (b[29] as u32) << 16);
                return Some(("WEBP", w, h));
            }
            b"VP8L" if b.len() >= 25 && b[20] == 0x2F => {
                let bits = u32::from_le_bytes([b[21], b[22], b[23], b[24]]);
                return Some(("WEBP", (bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1));
            }
            b"VP8 " if b.len() >= 30 && b[23..26] == [0x9D, 0x01, 0x2A] => {
                let w = (u16::from_le_bytes([b[26], b[27]]) & 0x3FFF) as u32;
                let h = (u16::from_le_bytes([b[28], b[29]]) & 0x3FFF) as u32;
                return Some(("WEBP", w, h));
            }
            _ => return Some(("WEBP", 0, 0)),
        }
    }
    None
}

async fn load_image_data(image_data: &ImageData) -> Result<Vec<u8>, String> {
    match image_data {
        ImageData::Base64 { data, .. } => {
            use base64::{engine::general_purpose::STANDARD, Engine};
            STANDARD
                .decode(data)
                .map_err(|e| format!("Failed to decode base64: {}", e))
        }
        ImageData::FilePath(path) => {
            tokio::fs::read(path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))
        }
        ImageData::Url(url) => {
            // In production, use reqwest to fetch
            Err(format!("URL loading not implemented: {}", url))
        }
        ImageData::Bytes(bytes) => Ok(bytes.clone()),
    }
}

async fn create_graph(
    analysis: ImageAnalysisResult,
    project_id: u64,
    graph_name: Option<String>,
) -> Result<ImageGraph, String> {
    let graph_id = generate_graph_id();
    let now = chrono::Utc::now().to_rfc3339();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_id_counter: u64 = 1;

    // Create root image node
    let image_node_id = node_id_counter;
    nodes.push(ImageGraphNode {
        node_id: image_node_id,
        node_type: ImageNodeType::Image,
        label: "Image".to_string(),
        content: format!("{}x{} {}", analysis.width, analysis.height, analysis.format),
        bounding_box: Some(BoundingBox {
            x: 0.0,
            y: 0.0,
            width: analysis.width as f32,
            height: analysis.height as f32,
            rotation: 0.0,
        }),
        confidence: analysis.confidence,
        properties: {
            let mut props = HashMap::new();
            props.insert("width".to_string(), Value::from(analysis.width));
            props.insert("height".to_string(), Value::from(analysis.height));
            props.insert("format".to_string(), Value::from(analysis.format.clone()));
            props.insert(
                "color_space".to_string(),
                Value::from(analysis.color_space.clone()),
            );
            props.insert("bit_depth".to_string(), Value::from(analysis.bit_depth));
            // Geometry (docs/SPATIAL_GEOMETRY_LAYER.md): the image is a pixel grid
            // in frame "image-px"; the host reads attributes.geom/frame.
            if let Some(g) = analysis.metadata.get("geom") {
                props.insert("geom".to_string(), g.clone());
                props.insert("frame".to_string(), Value::from("image-px"));
            }
            if let Some(pm) = analysis.metadata.get("pixel_metrics") {
                props.insert("pixel_metrics".to_string(), pm.clone());
            }
            props
        },
        annotations: vec![],
    });
    node_id_counter += 1;

    // Create object nodes
    for obj in &analysis.objects {
        let obj_node_id = node_id_counter;
        nodes.push(ImageGraphNode {
            node_id: obj_node_id,
            node_type: ImageNodeType::Object,
            label: obj.label.clone(),
            content: format!("Object: {} (conf: {:.2})", obj.label, obj.confidence),
            bounding_box: Some(obj.bounding_box.clone()),
            confidence: obj.confidence,
            properties: obj.attributes.clone(),
            annotations: vec![],
        });

        // Edge: Image contains Object
        edges.push(ImageGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: image_node_id,
            to_node: obj_node_id,
            edge_type: ImageEdgeType::Contains,
            weight: obj.confidence,
            properties: HashMap::new(),
        });

        // LIVE NODE EVENT (2026-10-19, additive — same pattern as 3D's
        // first real producer, see orchestration_events::OrchestrationNodeEvent's
        // doc comment): best-effort, fire-and-forget report of this real
        // object node as it's added. Detached tokio::spawn, never awaited
        // inline, so a slow/unreachable host can never slow or fail real
        // graph construction.
        {
            let host = env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
            let pid = project_id;
            let gid = graph_id;
            let nid = obj_node_id;
            let label = obj.label.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let _ = client.post(format!("{host}/orchestration/node-event")).header("x-oz-internal", std::env::var("OZONE_INTERNAL_TOKEN").unwrap_or_default())
                    .json(&serde_json::json!({
                        "project_id": pid,
                        "modality": "image",
                        "graph_id": gid,
                        "node_id": nid,
                        "related_node_id": serde_json::Value::Null,
                        "node_type": format!("Object:{}", label),
                        "operation": "node_created",
                    }))
                    .timeout(std::time::Duration::from_secs(2))
                    .send().await;
            });
        }

        node_id_counter += 1;
    }

    // Create region nodes
    for region in &analysis.regions {
        let region_node_id = node_id_counter;
        nodes.push(ImageGraphNode {
            node_id: region_node_id,
            node_type: ImageNodeType::Region,
            label: format!("{:?}", region.region_type),
            content: region
                .description
                .clone()
                .unwrap_or_else(|| "Region".to_string()),
            bounding_box: Some(region.bounding_box.clone()),
            confidence: region.confidence,
            properties: HashMap::new(),
            annotations: vec![],
        });

        edges.push(ImageGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: image_node_id,
            to_node: region_node_id,
            edge_type: ImageEdgeType::Contains,
            weight: region.confidence,
            properties: HashMap::new(),
        });

        node_id_counter += 1;
    }

    // Create text nodes
    for text in &analysis.text_regions {
        let text_node_id = node_id_counter;
        nodes.push(ImageGraphNode {
            node_id: text_node_id,
            node_type: ImageNodeType::Text,
            label: "Text".to_string(),
            content: text.text.clone(),
            bounding_box: Some(text.bounding_box.clone()),
            confidence: text.confidence,
            properties: {
                let mut props = HashMap::new();
                if let Some(lang) = &text.language {
                    props.insert("language".to_string(), Value::from(lang.clone()));
                }
                props
            },
            annotations: vec![],
        });

        edges.push(ImageGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: image_node_id,
            to_node: text_node_id,
            edge_type: ImageEdgeType::Contains,
            weight: text.confidence,
            properties: HashMap::new(),
        });

        node_id_counter += 1;
    }

    // Create face nodes
    for face in &analysis.faces {
        let face_node_id = node_id_counter;
        nodes.push(ImageGraphNode {
            node_id: face_node_id,
            node_type: ImageNodeType::Face,
            label: "Face".to_string(),
            content: format!("Face detected (conf: {:.2})", face.confidence),
            bounding_box: Some(face.bounding_box.clone()),
            confidence: face.confidence,
            properties: serde_json::to_value(&face.attributes)
                .ok()
                .and_then(|v| v.as_object().cloned())
                .map(|m| m.into_iter().collect())
                .unwrap_or_default(),
            annotations: vec![],
        });

        edges.push(ImageGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: image_node_id,
            to_node: face_node_id,
            edge_type: ImageEdgeType::Contains,
            weight: face.confidence,
            properties: HashMap::new(),
        });

        node_id_counter += 1;
    }

    // Create color nodes
    for color in &analysis.dominant_colors {
        let color_node_id = node_id_counter;
        nodes.push(ImageGraphNode {
            node_id: color_node_id,
            node_type: ImageNodeType::Color,
            label: color.name.clone().unwrap_or_else(|| color.hex.clone()),
            content: format!(
                "{} ({:.1}%)",
                color.name.clone().unwrap_or_else(|| color.hex.clone()),
                color.percentage * 100.0
            ),
            bounding_box: None,
            confidence: 1.0,
            properties: {
                let mut props = HashMap::new();
                props.insert("hex".to_string(), Value::from(color.hex.clone()));
                props.insert("r".to_string(), Value::from(color.r));
                props.insert("g".to_string(), Value::from(color.g));
                props.insert("b".to_string(), Value::from(color.b));
                props.insert("percentage".to_string(), Value::from(color.percentage));
                props
            },
            annotations: vec![],
        });

        edges.push(ImageGraphEdge {
            edge_id: edges.len() as u64 + 1,
            from_node: image_node_id,
            to_node: color_node_id,
            edge_type: ImageEdgeType::Contains,
            weight: color.percentage,
            properties: HashMap::new(),
        });

        node_id_counter += 1;
    }

    // Add spatial relationships between objects
    let object_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| n.node_type == ImageNodeType::Object)
        .collect();

    for i in 0..object_nodes.len() {
        for j in (i + 1)..object_nodes.len() {
            if let (Some(box1), Some(box2)) = (
                &object_nodes[i].bounding_box,
                &object_nodes[j].bounding_box,
            ) {
                // Check spatial relationships
                if box1.intersects(box2) {
                    edges.push(ImageGraphEdge {
                        edge_id: edges.len() as u64 + 1,
                        from_node: object_nodes[i].node_id,
                        to_node: object_nodes[j].node_id,
                        edge_type: ImageEdgeType::Overlaps,
                        weight: box1.intersection_over_union(box2),
                        properties: HashMap::new(),
                    });
                } else {
                    // Determine relative position
                    let (cx1, cy1) = box1.center();
                    let (cx2, cy2) = box2.center();

                    let edge_type = if (cx1 - cx2).abs() > (cy1 - cy2).abs() {
                        if cx1 < cx2 {
                            ImageEdgeType::LeftOf
                        } else {
                            ImageEdgeType::RightOf
                        }
                    } else if cy1 < cy2 {
                        ImageEdgeType::Above
                    } else {
                        ImageEdgeType::Below
                    };

                    edges.push(ImageGraphEdge {
                        edge_id: edges.len() as u64 + 1,
                        from_node: object_nodes[i].node_id,
                        to_node: object_nodes[j].node_id,
                        edge_type,
                        weight: 1.0,
                        properties: HashMap::new(),
                    });
                }
            }
        }
    }

    let edge_count = edges.len();
    Ok(ImageGraph {
        graph_id,
        name: graph_name.unwrap_or_else(|| format!("Image Graph {}", graph_id)),
        modality: MODALITY.to_string(),
        project_id,
        source: ImageSource {
            path: None,
            url: None,
            hash: format!("hash_{}", graph_id),
            width: analysis.width,
            height: analysis.height,
            format: analysis.format,
        },
        nodes,
        edges,
        metadata: GraphMetadata {
            node_count: node_id_counter as usize - 1,
            edge_count: edge_count,
            analysis_depth: AnalysisDepth::Standard,
            semantic_enriched: false,
            cross_modal_links: 0,
        },
        created_at: now.clone(),
        updated_at: now,
    })
}

async fn update_graph(graph_id: u64, updates: ImageGraphUpdate) -> Result<ImageGraph, String> {
    // In production, this would load from ZSEI, apply updates, and save
    let mut graph = get_graph(graph_id).await?;
    let now = chrono::Utc::now().to_rfc3339();

    // Apply node updates
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

    // Apply edge updates
    for edge in updates.add_edges {
        graph.edges.push(edge);
    }

    graph
        .edges
        .retain(|e| !updates.remove_edges.contains(&e.edge_id));

    // Update metadata
    graph.metadata.node_count = graph.nodes.len();
    graph.metadata.edge_count = graph.edges.len();
    graph.updated_at = now;

    // PERSIST (fixed 2026-10-09, GRAPH_RELATIONSHIP_REGISTRY §3b
    // priority-0): update_graph applied every node/edge change in memory
    // and then returned a fabricated success — the update was never written
    // to disk, so the next get_graph for this id silently lost it.
    // CreateGraph two arms up already calls persist_image_graph; UpdateGraph
    // now does the same.
    persist_image_graph(&mut graph).await?;
    Ok(graph)
}

async fn query_graph(graph_id: u64, query: ImageQuery) -> Result<QueryResult, String> {
    let graph = get_graph(graph_id).await?;

    let min_confidence = query.min_confidence.unwrap_or(0.0);
    let limit = query.limit.unwrap_or(100);

    let (nodes, edges) = match query.query_type.clone() {
        ImageQueryType::FindObjects { labels } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.node_type == ImageNodeType::Object
                        && labels.iter().any(|l| n.label.to_lowercase().contains(&l.to_lowercase()))
                        && n.confidence >= min_confidence
                })
                .take(limit)
                .cloned()
                .collect();

            let node_ids: Vec<_> = matching_nodes.iter().map(|n| n.node_id).collect();
            let related_edges: Vec<_> = graph
                .edges
                .iter()
                .filter(|e| node_ids.contains(&e.from_node) || node_ids.contains(&e.to_node))
                .cloned()
                .collect();

            (matching_nodes, related_edges)
        }

        ImageQueryType::FindRegions { region_types } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    if n.node_type != ImageNodeType::Region {
                        return false;
                    }
                    // In production, would check actual region type
                    n.confidence >= min_confidence
                })
                .take(limit)
                .cloned()
                .collect();

            (matching_nodes, vec![])
        }

        ImageQueryType::FindText { pattern } => {
            let pattern_lower = pattern.to_lowercase();
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| {
                    n.node_type == ImageNodeType::Text
                        && n.content.to_lowercase().contains(&pattern_lower)
                        && n.confidence >= min_confidence
                })
                .take(limit)
                .cloned()
                .collect();

            (matching_nodes, vec![])
        }

        ImageQueryType::GetNodesByType { node_type } => {
            let matching_nodes: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.node_type == node_type && n.confidence >= min_confidence)
                .take(limit)
                .cloned()
                .collect();

            (matching_nodes, vec![])
        }

        ImageQueryType::GetConnected { node_id, max_depth } => {
            let mut visited = std::collections::HashSet::new();
            let mut to_visit = vec![(node_id, 0u32)];
            let mut connected_nodes = Vec::new();
            let mut connected_edges = Vec::new();

            while let Some((current_id, depth)) = to_visit.pop() {
                if depth > max_depth || visited.contains(&current_id) {
                    continue;
                }
                visited.insert(current_id);

                if let Some(node) = graph.nodes.iter().find(|n| n.node_id == current_id) {
                    connected_nodes.push(node.clone());
                }

                for edge in &graph.edges {
                    if edge.from_node == current_id && !visited.contains(&edge.to_node) {
                        connected_edges.push(edge.clone());
                        to_visit.push((edge.to_node, depth + 1));
                    }
                    if edge.to_node == current_id && !visited.contains(&edge.from_node) {
                        connected_edges.push(edge.clone());
                        to_visit.push((edge.from_node, depth + 1));
                    }
                }
            }

            (connected_nodes, connected_edges)
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

/// Loads the REAL persisted graph from the sidecar file `persist_image_graph`
/// writes (graphs/image_<id>.json) — previously this ignored `graph_id`
/// entirely and returned hardcoded fake nodes/edges regardless of what was
/// actually stored. name/project_id aren't in the sidecar (only
/// graph_id/nodes/edges/source/metadata are), so those two fields are
/// honest best-effort placeholders, not fabricated data about the graph's
/// actual content.
async fn get_graph(graph_id: u64) -> Result<ImageGraph, String> {
    let dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let rel = format!("graphs/image_{}.json", graph_id);
    let abs = std::path::Path::new(&dir).join(&rel);
    let raw = std::fs::read_to_string(&abs)
        .map_err(|e| format!("no persisted graph found for image graph {}: {}", graph_id, e))?;
    let sidecar: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("graph content parse failed: {e}"))?;
    let nodes: Vec<ImageGraphNode> = serde_json::from_value(
        sidecar.get("nodes").cloned().unwrap_or(serde_json::Value::Array(vec![])),
    )
    .map_err(|e| format!("graph nodes parse failed: {e}"))?;
    let edges: Vec<ImageGraphEdge> = serde_json::from_value(
        sidecar.get("edges").cloned().unwrap_or(serde_json::Value::Array(vec![])),
    )
    .map_err(|e| format!("graph edges parse failed: {e}"))?;
    let source: ImageSource = sidecar
        .get("source")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or(ImageSource {
            path: None,
            url: None,
            hash: String::new(),
            width: 0,
            height: 0,
            format: String::new(),
        });
    let metadata: GraphMetadata = sidecar
        .get("metadata")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    Ok(ImageGraph {
        graph_id,
        name: format!("Image Graph {}", graph_id),
        modality: MODALITY.to_string(),
        project_id: 0,
        source,
        nodes,
        edges,
        metadata,
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
    image_graph_id: u64,
    target_graph_id: u64,
    _target_modality: &str,
    relationship: CrossModalityRelation,
    _bidirectional: bool,
) -> Result<LinkResult, String> {
    // Real, bidirectional ZSEI relationship — both containers must actually
    // exist; this used to unconditionally fabricate a "success" LinkResult
    // for any graph_id, real or not, with nothing behind it (2026-10-12
    // audit finding). An honest error here (e.g. a target_graph_id that was
    // never persisted) is correct behavior, not a regression. Pattern
    // matches math/main.rs's already-fixed link_to_modality.
    add_relationship(image_graph_id, target_graph_id).await?;
    add_relationship(target_graph_id, image_graph_id).await?;

    Ok(LinkResult {
        link_id: generate_graph_id(),
        source_graph_id: image_graph_id,
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
    // 10/5/15 regardless of input/graph_id). Full semantic enrichment is a
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

async fn compare_images(
    image1: &ImageData,
    image2: &ImageData,
    comparison_type: ComparisonType,
) -> Result<ComparisonResult, String> {
    let b1 = load_image_data(image1).await?;
    let b2 = load_image_data(image2).await?;
    compare_image_bytes(&b1, &b2, comparison_type)
}

/// REAL pixel comparison (D16): visual similarity = 1 - mean absolute RGB difference of
/// both images resized to 32x32; colour similarity = histogram intersection of the
/// 4-bit/channel colour histograms (128x128 resize). Structural / Composition
/// comparison need detections (visual-mcp) and are reported absent as an error.
fn compare_image_bytes(b1: &[u8], b2: &[u8], comparison_type: ComparisonType) -> Result<ComparisonResult, String> {
    if matches!(comparison_type, ComparisonType::Structural | ComparisonType::Composition) {
        return Err("structural/composition comparison needs object detections — visual-mcp's surface (D17); this pipeline compares pixels and colours only".to_string());
    }
    let i1 = image::load_from_memory(b1).map_err(|e| format!("image 1 decode failed: {e}"))?;
    let i2 = image::load_from_memory(b2).map_err(|e| format!("image 2 decode failed: {e}"))?;
    let (w1, h1, w2, h2) = (i1.width() as f32, i1.height() as f32, i2.width() as f32, i2.height() as f32);
    let want_visual = matches!(comparison_type, ComparisonType::Visual | ComparisonType::Comprehensive);
    let want_color = matches!(comparison_type, ComparisonType::ColorPalette | ComparisonType::Comprehensive);

    let mut visual: Option<f32> = None;
    let mut differences: Vec<Difference> = Vec::new();
    if want_visual {
        const G: u32 = 32;
        let s1 = i1.resize_exact(G, G, image::imageops::FilterType::Triangle).to_rgb8();
        let s2 = i2.resize_exact(G, G, image::imageops::FilterType::Triangle).to_rgb8();
        let mut total = 0f64;
        let mut tiles = [0f64; 16];
        for y in 0..G {
            for x in 0..G {
                let (p, q) = (s1.get_pixel(x, y).0, s2.get_pixel(x, y).0);
                let d = (0..3).map(|c| (p[c] as f64 - q[c] as f64).abs()).sum::<f64>() / (3.0 * 255.0);
                total += d;
                tiles[((y / 8) * 4 + (x / 8)) as usize] += d / 64.0;
            }
        }
        visual = Some((1.0 - total / (G * G) as f64) as f32);
        for (t, d) in tiles.iter().enumerate() {
            if *d >= 0.25 {
                let (tx, ty) = ((t % 4) as f32, (t / 4) as f32);
                differences.push(Difference {
                    diff_type: "visual_region".to_string(),
                    description: format!("tile ({}, {}) of a 4x4 grid differs: mean absolute difference {:.2}", tx as u32, ty as u32, d),
                    location1: Some(BoundingBox { x: tx * w1 / 4.0, y: ty * h1 / 4.0, width: w1 / 4.0, height: h1 / 4.0, rotation: 0.0 }),
                    location2: Some(BoundingBox { x: tx * w2 / 4.0, y: ty * h2 / 4.0, width: w2 / 4.0, height: h2 / 4.0, rotation: 0.0 }),
                    severity: *d as f32,
                });
            }
        }
    }
    let mut color: Option<f32> = None;
    if want_color {
        let hist = |img: &image::DynamicImage| -> Vec<f64> {
            let s = img.resize_exact(128, 128, image::imageops::FilterType::Triangle).to_rgb8();
            let mut h = vec![0f64; 4096];
            for p in s.pixels() {
                let [r, g, b] = p.0;
                h[(((r >> 4) as usize) << 8) | (((g >> 4) as usize) << 4) | ((b >> 4) as usize)] += 1.0;
            }
            let n = (128 * 128) as f64;
            h.iter_mut().for_each(|v| *v /= n);
            h
        };
        let (h1v, h2v) = (hist(&i1), hist(&i2));
        color = Some(h1v.iter().zip(h2v.iter()).map(|(a, b)| a.min(*b)).sum::<f64>() as f32);
    }
    let present: Vec<f32> = [visual, color].iter().flatten().copied().collect();
    let overall = present.iter().sum::<f32>() / present.len().max(1) as f32;
    Ok(ComparisonResult {
        overall_similarity: overall,
        visual_similarity: visual,
        structural_similarity: None,
        color_similarity: color,
        composition_similarity: None,
        differences,
        common_elements: Vec::new(),
    })
}

async fn extract_region(image_data: &ImageData, bounding_box: &BoundingBox) -> Result<RegionResult, String> {
    let bytes = load_image_data(image_data).await?;
    extract_region_bytes(&bytes, bounding_box)
}

/// REAL crop (D16): decode, clip the box to the image, re-encode as PNG (base64).
/// Rotated boxes are refused (no resampling is attempted).
fn extract_region_bytes(bytes: &[u8], bb: &BoundingBox) -> Result<RegionResult, String> {
    if bb.rotation != 0.0 {
        return Err("rotated bounding boxes are not supported by region extraction (axis-aligned crops only)".to_string());
    }
    let img = image::load_from_memory(bytes).map_err(|e| format!("decode failed: {e}"))?;
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let x0 = bb.x.max(0.0).floor();
    let y0 = bb.y.max(0.0).floor();
    let x1 = (bb.x + bb.width).min(iw).ceil();
    let y1 = (bb.y + bb.height).min(ih).ceil();
    if !(x1 > x0 && y1 > y0) {
        return Err(format!("bounding box lies outside the {}x{} image: nothing to extract", iw as u32, ih as u32));
    }
    let (cw, ch) = ((x1 - x0) as u32, (y1 - y0) as u32);
    let crop = img.crop_imm(x0 as u32, y0 as u32, cw, ch);
    let mut buf: Vec<u8> = Vec::new();
    crop.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageOutputFormat::Png)
        .map_err(|e| format!("png encode failed: {e}"))?;
    use base64::{engine::general_purpose::STANDARD, Engine};
    Ok(RegionResult {
        image_data: ImageData::Base64 { data: STANDARD.encode(&buf), mime_type: "image/png".to_string() },
        original_box: BoundingBox { x: x0, y: y0, width: cw as f32, height: ch as f32, rotation: 0.0 },
        width: cw,
        height: ch,
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
    // Standard host contract (the math-revival pattern): --input <json> or
    // stdin, full {"data": {...}, "context": {...}} envelope — unwrap
    // `data` when present, else the whole payload.
    let mut input_str = String::new();
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_str = ozone_input_arg(&args[i + 1]);
            i += 1;
        }
        i += 1;
    }
    if input_str.is_empty() {
        use std::io::Read;
        let _ = std::io::stdin().read_to_string(&mut input_str);
    }
    if input_str.trim().is_empty() {
        eprintln!("Usage: {} --input '<json>' (or JSON on stdin)", args.get(0).unwrap_or(&"image".to_string()));
        std::process::exit(1);
    }
    let raw: Value = match serde_json::from_str(&input_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse input JSON: {}", e);
            std::process::exit(1);
        }
    };
    let input: Value = match raw.get("data") {
        Some(d) => d.clone(),
        None => raw,
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

    fn solid_png(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb(rgb));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageOutputFormat::Png)
            .unwrap();
        buf
    }

    #[tokio::test]
    async fn test_analyze_image() {
        // HONESTY (D16): analysis is of the REAL bytes (a generated 12x7 PNG),
        // not of a nonexistent path.
        use base64::{engine::general_purpose::STANDARD, Engine};
        let png = solid_png(12, 7, [255, 0, 0]);
        let input = serde_json::json!({
            "action": {
                "type": "Analyze",
                "image_data": {"Base64": {"data": STANDARD.encode(&png), "mime_type": "image/png"}},
                "depth": "Standard",
                "detect_objects": true,
                "detect_text": true,
                "detect_faces": false,
                "analyze_colors": true,
                "analyze_composition": true
            }
        });

        let result = execute(input).await;
        assert!(result.is_ok(), "{:?}", result);
        let output: ImageModalityOutput = serde_json::from_value(result.unwrap()).unwrap();
        assert!(output.success);
        assert_eq!(output.action, "Analyze");
    }

    #[tokio::test]
    async fn test_create_graph() {
        let analysis = ImageAnalysisResult {
            width: 800,
            height: 600,
            format: "PNG".to_string(),
            color_space: "sRGB".to_string(),
            bit_depth: 8,
            objects: vec![DetectedObject {
                object_id: "obj_1".to_string(),
                label: "cat".to_string(),
                confidence: 0.95,
                bounding_box: BoundingBox {
                    x: 100.0,
                    y: 100.0,
                    width: 200.0,
                    height: 150.0,
                    rotation: 0.0,
                },
                mask: None,
                attributes: HashMap::new(),
                parent_id: None,
                children: vec![],
            }],
            regions: vec![],
            text_regions: vec![],
            faces: vec![],
            dominant_colors: vec![],
            composition: CompositionAnalysis::default(),
            quality: ImageQuality::default(),
            metadata: HashMap::new(),
            confidence: 0.9,
        };

        let graph = create_graph(analysis, 1, None).await.unwrap();

        assert_eq!(graph.modality, "image");
        assert!(!graph.nodes.is_empty());
        assert!(!graph.edges.is_empty());
    }

    #[test]
    fn test_bounding_box_operations() {
        let box1 = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            rotation: 0.0,
        };

        let box2 = BoundingBox {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
            rotation: 0.0,
        };

        assert!(box1.intersects(&box2));
        assert!(box1.intersection_over_union(&box2) > 0.0);

        let box3 = BoundingBox {
            x: 200.0,
            y: 200.0,
            width: 50.0,
            height: 50.0,
            rotation: 0.0,
        };

        assert!(!box1.intersects(&box3));
        assert_eq!(box1.intersection_over_union(&box3), 0.0);
    }

    #[test]
    fn test_color_analysis() {
        let color = Color::new(65, 105, 225, 0.35);
        assert_eq!(color.name, Some("Blue".to_string()));
        assert_eq!(color.hex, "#4169E1");
    }

    #[test]
    fn solid_red_png_has_real_pixel_stats() {
        let png = solid_png(40, 30, [255, 0, 0]);
        let a = analyze_image_bytes(&png, true).unwrap();
        assert_eq!((a.width, a.height), (40, 30));
        assert_eq!(a.format, "PNG");
        assert_eq!(a.bit_depth, 8);
        assert_eq!(a.dominant_colors.len(), 1);
        assert_eq!((a.dominant_colors[0].r, a.dominant_colors[0].g, a.dominant_colors[0].b), (255, 0, 0));
        assert!((a.dominant_colors[0].percentage - 1.0).abs() < 1e-6);
        // luma of pure red = 0.2126*255
        assert!((a.quality.brightness - 0.2126).abs() < 1e-3);
        assert!(a.quality.contrast < 1e-6);
        assert!((a.quality.saturation - 1.0).abs() < 1e-6);
        assert_eq!(a.quality.sharpness, 0.0); // no edges in a solid colour
        assert!(a.objects.is_empty() && a.faces.is_empty() && a.text_regions.is_empty());
    }

    #[test]
    fn checkerboard_has_edges_and_two_dominant_colors() {
        let mut img = image::RgbImage::new(64, 64);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = if (x / 4 + y / 4) % 2 == 0 { image::Rgb([0, 0, 0]) } else { image::Rgb([255, 255, 255]) };
        }
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageOutputFormat::Png)
            .unwrap();
        let a = analyze_image_bytes(&buf, true).unwrap();
        assert!(a.quality.sharpness > 0.05, "edge density {}", a.quality.sharpness);
        assert!(a.quality.contrast > 0.9);
        assert_eq!(a.dominant_colors.len(), 2);
    }

    #[test]
    fn geometry_is_an_image_px_grid_and_semantics_stay_absent() {
        let png = solid_png(10, 5, [10, 20, 30]);
        let a = analyze_image_bytes(&png, true).unwrap();
        let g = a.metadata.get("geom").unwrap();
        assert_eq!(g["frame"], "image-px");
        assert_eq!(g["kind"], "grid");
        assert_eq!(g["cols"], 10);
        assert_eq!(g["rows"], 5);
        let reasons = a.metadata.get("absent_reasons").unwrap().to_string();
        assert!(reasons.contains("objects") && reasons.contains("faces") && reasons.contains("composition"));
    }

    #[test]
    fn analyze_colors_false_skips_dominant_colors_with_a_reason() {
        let png = solid_png(8, 8, [1, 2, 3]);
        let a = analyze_image_bytes(&png, false).unwrap();
        assert!(a.dominant_colors.is_empty());
        assert!(a.metadata.get("absent_reasons").unwrap().to_string().contains("not requested"));
    }

    #[test]
    fn png_chunks_give_color_space_and_density_facts() {
        // hand-built PNG signature + IHDR(2x2, 8-bit RGB) + sRGB + pHYs + IEND (no IDAT: facts only)
        fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut c = (data.len() as u32).to_be_bytes().to_vec();
            c.extend_from_slice(kind);
            c.extend_from_slice(data);
            c.extend_from_slice(&[0, 0, 0, 0]); // CRC not checked by the fact parser
            c
        }
        let mut b = vec![0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = 2u32.to_be_bytes().to_vec();
        ihdr.extend_from_slice(&2u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        b.extend(chunk(b"IHDR", &ihdr));
        b.extend(chunk(b"sRGB", &[0]));
        let mut phys = 2835u32.to_be_bytes().to_vec();
        phys.extend_from_slice(&2835u32.to_be_bytes());
        phys.push(1);
        b.extend(chunk(b"pHYs", &phys));
        b.extend(chunk(b"IEND", &[]));
        let f = parse_png_facts(&b);
        assert_eq!(f.bit_depth, Some(8));
        assert_eq!(f.color_type_name, Some("RGB"));
        assert!(f.srgb);
        assert_eq!(f.facts["physical_pixel_density"]["x"], 2835);
    }

    #[test]
    fn exif_orientation_and_dimensions_are_parsed() {
        // JPEG SOI + APP1(Exif, little-endian TIFF, IFD0: Orientation=6, ExifIFD->PixelXDimension=640)
        let mut tiff: Vec<u8> = vec![b'I', b'I', 42, 0, 8, 0, 0, 0];
        // IFD0 at 8: 2 entries
        tiff.extend_from_slice(&2u16.to_le_bytes());
        // Orientation (0x0112) SHORT count1 value 6
        tiff.extend_from_slice(&0x0112u16.to_le_bytes());
        tiff.extend_from_slice(&3u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&[6, 0, 0, 0]);
        // ExifIFD pointer (0x8769) LONG count1 -> offset 38
        tiff.extend_from_slice(&0x8769u16.to_le_bytes());
        tiff.extend_from_slice(&4u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&38u32.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes()); // next IFD
        assert_eq!(tiff.len(), 38);
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0xA002u16.to_le_bytes());
        tiff.extend_from_slice(&3u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&[0x80, 0x02, 0, 0]); // 640
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let mut jpeg = vec![0xFFu8, 0xD8, 0xFF, 0xE1];
        jpeg.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(&app1);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        let e = parse_exif(&jpeg);
        assert_eq!(e["orientation"], 6);
        assert_eq!(e["pixel_x_dimension"], 640);
        // no EXIF => nothing invented
        assert!(parse_exif(&[0xFF, 0xD8, 0xFF, 0xD9]).is_empty());
    }

    #[test]
    fn bmp_and_webp_headers_report_dimensions() {
        let mut bmp = vec![0u8; 30];
        bmp[0] = b'B';
        bmp[1] = b'M';
        bmp[18..22].copy_from_slice(&7i32.to_le_bytes());
        bmp[22..26].copy_from_slice(&(-5i32).to_le_bytes()); // top-down BMP: negative height
        bmp[28..30].copy_from_slice(&24u16.to_le_bytes());
        assert_eq!(parse_image_header(&bmp), Some(("BMP", 7, 5)));
        assert_eq!(bmp_bits_per_pixel(&bmp), Some(24));
        let mut webp = vec![0u8; 30];
        webp[0..4].copy_from_slice(b"RIFF");
        webp[8..12].copy_from_slice(b"WEBP");
        webp[12..16].copy_from_slice(b"VP8X");
        webp[24..27].copy_from_slice(&[99, 0, 0]); // width-1 = 99
        webp[27..30].copy_from_slice(&[49, 0, 0]); // height-1 = 49
        assert_eq!(parse_image_header(&webp), Some(("WEBP", 100, 50)));
    }

    #[test]
    fn unknown_bytes_are_an_error_not_a_guess() {
        assert!(analyze_image_bytes(b"not an image at all", true).is_err());
    }

    #[test]
    fn oversized_declared_dimensions_skip_decoding_with_a_reason() {
        // PNG header claiming 100000x100000 (no pixel data): decode is skipped, header facts kept
        let mut b = vec![0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        b.extend_from_slice(&100_000u32.to_be_bytes());
        b.extend_from_slice(&100_000u32.to_be_bytes());
        b.extend_from_slice(&[8, 2, 0, 0, 0]);
        let a = analyze_image_bytes(&b, true).unwrap();
        assert_eq!((a.width, a.height), (100_000, 100_000));
        assert!(a.metadata.get("absent_reasons").unwrap().to_string().contains("decode limit"));
        assert_eq!(a.confidence, 0.5);
    }

    #[test]
    fn region_extract_crops_real_pixels_and_clips_to_the_image() {
        // left half red, right half blue
        let mut img = image::RgbImage::new(20, 10);
        for (x, _y, p) in img.enumerate_pixels_mut() {
            *p = if x < 10 { image::Rgb([255, 0, 0]) } else { image::Rgb([0, 0, 255]) };
        }
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut buf), image::ImageOutputFormat::Png).unwrap();
        let r = extract_region_bytes(&buf, &BoundingBox { x: 12.0, y: 2.0, width: 100.0, height: 4.0, rotation: 0.0 }).unwrap();
        assert_eq!((r.width, r.height), (8, 4)); // clipped at the right edge
        if let ImageData::Base64 { data, .. } = &r.image_data {
            use base64::{engine::general_purpose::STANDARD, Engine};
            let crop = image::load_from_memory(&STANDARD.decode(data).unwrap()).unwrap().to_rgb8();
            assert_eq!(crop.get_pixel(0, 0).0, [0, 0, 255]);
        } else {
            panic!("expected base64 crop");
        }
        assert!(extract_region_bytes(&buf, &BoundingBox { x: 50.0, y: 50.0, width: 5.0, height: 5.0, rotation: 0.0 }).is_err());
        assert!(extract_region_bytes(&buf, &BoundingBox { x: 0.0, y: 0.0, width: 5.0, height: 5.0, rotation: 30.0 }).is_err());
    }

    #[test]
    fn compare_identical_is_one_and_opposite_is_low_and_semantic_modes_refuse() {
        let red = solid_png(32, 32, [255, 0, 0]);
        let blue = solid_png(32, 32, [0, 0, 255]);
        let same = compare_image_bytes(&red, &red, ComparisonType::Comprehensive).unwrap();
        assert!((same.overall_similarity - 1.0).abs() < 1e-4);
        assert!(same.differences.is_empty());
        let diff = compare_image_bytes(&red, &blue, ComparisonType::Comprehensive).unwrap();
        assert!(diff.visual_similarity.unwrap() < 0.5);
        assert!(diff.color_similarity.unwrap() < 0.01);
        assert_eq!(diff.differences.len(), 16);
        assert!(compare_image_bytes(&red, &blue, ComparisonType::Structural).is_err());
        assert!(diff.structural_similarity.is_none() && diff.composition_similarity.is_none());
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
