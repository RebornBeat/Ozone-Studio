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
        k.truncate(12);
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


/// HONESTY FIX (D17, 2026-10-31): analyze_image no longer returns fabricated
/// values. It parses the byte headers for what is DECIDABLE (format,
/// dimensions) and reports every semantic layer — objects, text, faces,
/// colors, composition, depth — as explicitly ABSENT with the reason that
/// visual-mcp (:3220) is the detection surface (D17: objects and depth come
/// only from visual-mcp). A caller that needs detections calls visual-mcp's
/// yolo_* tools through /mcp/call.
async fn analyze_image(
    image_data: &ImageData,
    _depth: AnalysisDepth,
    _detect_objects: bool,
    _detect_text: bool,
    _detect_faces: bool,
    _analyze_colors: bool,
    _analyze_composition: bool,
) -> Result<ImageAnalysisResult, String> {
    let bytes = load_image_data(image_data).await?;

    // Header parse: format + dimensions are decidable from the bytes alone.
    let (format, width, height) = parse_image_header(&bytes)
        .ok_or_else(|| "unrecognized image format (PNG/JPEG/GIF headers only; other formats report absent)".to_string())?;

    let absent = |what: &str| -> String {
        format!("{what}: absent — detection is visual-mcp's surface (D17); this pipeline reports only what the bytes decide")
    };

    Ok(ImageAnalysisResult {
        width,
        height,
        format: format.to_string(),
        color_space: "unknown (header parse does not read the color profile)".to_string(),
        bit_depth: 0,
        objects: Vec::new(),
        regions: Vec::new(),
        text_regions: Vec::new(),
        faces: Vec::new(),
        dominant_colors: Vec::new(),
        composition: CompositionAnalysis::default(),
        quality: ImageQuality::default(),
        metadata: {
            let mut m = std::collections::HashMap::new();
            m.insert(
                "honesty".to_string(),
                Value::String(
                    "header-parse only; semantic layers are visual-mcp's surface (D17)".to_string(),
                ),
            );
            m.insert(
                "absent_reasons".to_string(),
                Value::Array(vec![
                    Value::String(absent("objects")),
                    Value::String(absent("text")),
                    Value::String(absent("faces")),
                    Value::String(absent("dominant colors")),
                    Value::String(absent("composition")),
                ]),
            );
            m
        },
        confidence: 1.0,
    })
}

/// Parse PNG (IHDR), JPEG (SOF scan) and GIF headers for format +
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
                let _ = client.post(format!("{host}/orchestration/node-event"))
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
    _image1: &ImageData,
    _image2: &ImageData,
    _comparison_type: ComparisonType,
) -> Result<ComparisonResult, String> {
    // Real image similarity comparison (perceptual hashing / structural
    // comparison) is not yet implemented. Previously this returned fixed
    // fabricated scores (0.75/0.80/0.70/0.85) regardless of the actual
    // images compared — honest failure instead.
    Err("real image similarity comparison not yet implemented".to_string())
}

async fn extract_region(
    _image_data: &ImageData,
    _bounding_box: &BoundingBox,
) -> Result<RegionResult, String> {
    // DE-FABRICATED (2026-10-13): this used to return the WHOLE image labeled
    // as the extracted region — the caller asked for a crop and silently got
    // the unclipped original with the bounding box's dimensions claimed on
    // top. Real cropping needs an image decoder this crate doesn't carry
    // (ImageData is Base64/FilePath/Url/Bytes — no pixels, no dimensions),
    // so until one is added, region extraction fails HONESTLY with the exact
    // missing capability named, proteomics-PredictStructure-style.
    Err(
        "region extraction is not implemented: cropping requires an image \
         decoder (PNG/JPEG -> raw pixels) which this pipeline does not yet \
         depend on; no cropped image was produced"
            .to_string(),
    )
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
            input_str = args[i + 1].clone();
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

    #[tokio::test]
    async fn test_analyze_image() {
        let input = serde_json::json!({
            "action": {
                "type": "Analyze",
                "image_data": {"FilePath": "/test/image.png"},
                "depth": "Standard",
                "detect_objects": true,
                "detect_text": true,
                "detect_faces": false,
                "analyze_colors": true,
                "analyze_composition": true
            }
        });

        let result = execute(input).await;
        assert!(result.is_ok());

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
}
