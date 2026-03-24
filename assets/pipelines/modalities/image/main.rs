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
// EXECUTION
// ============================================================================

pub async fn execute(input: Value) -> Result<Value, String> {
    let start_time = std::time::Instant::now();

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
            let graph = create_graph(analysis, project_id, graph_name).await?;
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

async fn analyze_image(
    image_data: &ImageData,
    depth: AnalysisDepth,
    detect_objects: bool,
    detect_text: bool,
    detect_faces: bool,
    analyze_colors: bool,
    analyze_composition: bool,
) -> Result<ImageAnalysisResult, String> {
    // Load image data
    let _image_bytes = load_image_data(image_data).await?;

    // For now, return a structured result
    // In production, this would call actual CV models (YOLO, OCR, face detection, etc.)

    let mut objects = Vec::new();
    let mut regions = Vec::new();
    let mut text_regions = Vec::new();
    let mut faces = Vec::new();
    let mut dominant_colors = Vec::new();

    if detect_objects {
        // Object detection would go here (YOLO, Faster R-CNN, etc.)
        objects.push(DetectedObject {
            object_id: "obj_1".to_string(),
            label: "example_object".to_string(),
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
        });
    }

    if detect_text {
        // OCR would go here (Tesseract, EasyOCR, etc.)
        text_regions.push(TextRegion {
            text: "Sample text".to_string(),
            confidence: 0.90,
            bounding_box: BoundingBox {
                x: 50.0,
                y: 50.0,
                width: 100.0,
                height: 20.0,
                rotation: 0.0,
            },
            language: Some("en".to_string()),
            direction: TextDirection::LeftToRight,
            words: vec![],
            font_attributes: None,
        });
    }

    if detect_faces {
        // Face detection would go here
        faces.push(DetectedFace {
            face_id: "face_1".to_string(),
            bounding_box: BoundingBox {
                x: 200.0,
                y: 100.0,
                width: 100.0,
                height: 120.0,
                rotation: 0.0,
            },
            landmarks: vec![],
            attributes: FaceAttributes::default(),
            confidence: 0.92,
            embedding: None,
        });
    }

    if analyze_colors {
        // Color analysis
        dominant_colors.push(Color::new(65, 105, 225, 0.35)); // Royal Blue
        dominant_colors.push(Color::new(255, 255, 255, 0.25)); // White
        dominant_colors.push(Color::new(50, 50, 50, 0.20)); // Dark Gray
    }

    // Region segmentation
    regions.push(ImageRegion {
        region_id: "region_1".to_string(),
        region_type: RegionType::Background,
        bounding_box: BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
            rotation: 0.0,
        },
        description: Some("Main background".to_string()),
        confidence: 0.85,
        polygon: None,
        semantic_label: Some("background".to_string()),
    });

    let composition = if analyze_composition {
        CompositionAnalysis {
            rule_of_thirds: RuleOfThirds {
                compliant: true,
                score: 0.75,
                intersection_points: vec![(0.33, 0.33), (0.66, 0.66)],
            },
            symmetry: SymmetryAnalysis {
                horizontal: 0.6,
                vertical: 0.4,
                radial: 0.2,
                axis: None,
            },
            focal_points: vec![FocalPoint {
                x: 0.5,
                y: 0.4,
                strength: 0.8,
                object_id: Some("obj_1".to_string()),
            }],
            visual_weight: VisualWeight {
                quadrants: [0.25, 0.30, 0.20, 0.25],
                center: (0.52, 0.48),
                balance_score: 0.85,
            },
            leading_lines: vec![],
            depth_layers: vec![],
            composition_score: 0.78,
        }
    } else {
        CompositionAnalysis::default()
    };

    let confidence = match depth {
        AnalysisDepth::Surface => 0.7,
        AnalysisDepth::Standard => 0.85,
        AnalysisDepth::Deep => 0.92,
        AnalysisDepth::Comprehensive => 0.95,
    };

    Ok(ImageAnalysisResult {
        width: 800,
        height: 600,
        format: "PNG".to_string(),
        color_space: "sRGB".to_string(),
        bit_depth: 8,
        objects,
        regions,
        text_regions,
        faces,
        dominant_colors,
        composition,
        quality: ImageQuality {
            sharpness: 0.85,
            brightness: 0.65,
            contrast: 0.75,
            saturation: 0.70,
            noise_level: 0.1,
            blur_amount: 0.05,
            overall_score: 0.80,
            issues: vec![],
        },
        metadata: HashMap::new(),
        confidence,
    })
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
            edge_count: edges.len(),
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

    Ok(graph)
}

async fn query_graph(graph_id: u64, query: ImageQuery) -> Result<QueryResult, String> {
    let graph = get_graph(graph_id).await?;

    let min_confidence = query.min_confidence.unwrap_or(0.0);
    let limit = query.limit.unwrap_or(100);

    let (nodes, edges) = match query.query_type {
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

async fn get_graph(graph_id: u64) -> Result<ImageGraph, String> {
    // In production, this would load from ZSEI
    Ok(ImageGraph {
        graph_id,
        name: format!("Image Graph {}", graph_id),
        modality: MODALITY.to_string(),
        project_id: 1,
        source: ImageSource {
            path: None,
            url: None,
            hash: format!("hash_{}", graph_id),
            width: 800,
            height: 600,
            format: "PNG".to_string(),
        },
        nodes: vec![],
        edges: vec![],
        metadata: GraphMetadata::default(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn link_to_modality(
    image_graph_id: u64,
    target_graph_id: u64,
    _target_modality: &str,
    relationship: CrossModalityRelation,
    _bidirectional: bool,
) -> Result<LinkResult, String> {
    let link_id = generate_graph_id();

    Ok(LinkResult {
        link_id,
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

    // In production, this would call the ZSEI hook processor
    // which would use LLM to add semantic annotations

    Ok(HookResult {
        hook_type,
        success: true,
        nodes_processed: 10,
        edges_added: 5,
        annotations_added: 15,
        processing_time_ms: start_time.elapsed().as_millis() as u64,
        errors: vec![],
    })
}

async fn compare_images(
    _image1: &ImageData,
    _image2: &ImageData,
    comparison_type: ComparisonType,
) -> Result<ComparisonResult, String> {
    // In production, this would actually compare the images

    Ok(ComparisonResult {
        overall_similarity: 0.75,
        visual_similarity: match comparison_type {
            ComparisonType::Visual | ComparisonType::Comprehensive => Some(0.80),
            _ => None,
        },
        structural_similarity: match comparison_type {
            ComparisonType::Structural | ComparisonType::Comprehensive => Some(0.70),
            _ => None,
        },
        color_similarity: match comparison_type {
            ComparisonType::ColorPalette | ComparisonType::Comprehensive => Some(0.85),
            _ => None,
        },
        composition_similarity: match comparison_type {
            ComparisonType::Composition | ComparisonType::Comprehensive => Some(0.65),
            _ => None,
        },
        differences: vec![],
        common_elements: vec!["background".to_string(), "foreground_object".to_string()],
    })
}

async fn extract_region(
    image_data: &ImageData,
    bounding_box: &BoundingBox,
) -> Result<RegionResult, String> {
    // In production, this would actually extract the region

    Ok(RegionResult {
        image_data: image_data.clone(),
        original_box: bounding_box.clone(),
        width: bounding_box.width as u32,
        height: bounding_box.height as u32,
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

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <json_input>",
            args.get(0).unwrap_or(&"image_analysis".to_string())
        );
        eprintln!("Pipeline: {} v{}", PIPELINE_NAME, PIPELINE_VERSION);
        std::process::exit(1);
    }

    let input_str = &args[1];
    let input: Value = match serde_json::from_str(input_str) {
        Ok(v) => v,
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
