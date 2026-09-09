//! OZONE Modality Pipelines
//!
//! Modality pipelines are the foundation of OZONE's AGI capabilities. Each modality
//! creates a structural graph from its input data type, enabling:
//!
//! 1. **Grounded Work** - Work is based on traversable structure, not regenerated from statistics
//! 2. **Long-Form Consistency** - Can maintain coherence over 1000+ step operations
//! 3. **Cross-Modality Integration** - Different data types can link and reference each other
//! 4. **Semantic Enrichment** - ZSEI adds semantic layer on top of structural
//!
//! # Pipeline Index
//!
//! | ID  | Name | Modality | Description |
//! |-----|------|----------|-------------|
//! | 100 | TextAnalysis | text | Text → structural graph |
//! | 101 | CodeAnalysis | code | Code → structural graph (AST) |
//! | 102 | ImageAnalysis | image | Image → structural graph |
//! | 103 | AudioAnalysis | audio | Audio → structural graph |
//! | 104 | VideoAnalysis | video | Video → structural graph |
//! | 105 | MathAnalysis | math | Math → structural graph |
//! | 106 | ChemistryAnalysis | chemistry | Chemistry → structural graph |
//! | 107 | DNAAnalysis | dna | DNA → structural graph |
//! | 108 | EEGAnalysis | eeg | EEG → structural graph |
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                     INPUT DATA                               │
//! │  (text, code, images, audio, video, math, chemistry, etc.)   │
//! └──────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌──────────────────────────────────────────────────────────────┐
//! │              MODALITY PIPELINE (100-199)                     │
//! │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐  │
//! │  │    ANALYZE     │─▶│  CREATE GRAPH  │─▶│ TRIGGER HOOKS  │  │
//! │  │  (structural)  │  │   (nodes +     │  │   (semantic)   │  │
//! │  │                │  │    edges)      │  │                │  │
//! │  └────────────────┘  └────────────────┘  └────────────────┘  │
//! └──────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌──────────────────────────────────────────────────────────────┐
//! │                   ZSEI SEMANTIC HOOKS                        │
//! │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐  │
//! │  │ OnGraphCreated │  │ OnInferRelations│ │ OnCrossModality│  │
//! │  │                │  │                │  │     Link       │  │
//! │  └────────────────┘  └────────────────┘  └────────────────┘  │
//! └──────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌──────────────────────────────────────────────────────────────┐
//! │              ENRICHED GRAPH (Structural + Semantic)          │
//! │  - Traversable structure for long-form work                  │
//! │  - Indexed in ZSEI for retrieval                             │
//! │  - Linkable to other modality graphs                         │
//! └──────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::{OzoneResult, OzoneError};
use crate::zsei::{ZSEIHook, HookResult};

// =============================================================================
// Modality Types
// =============================================================================

/// Supported modality types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModalityType {
    Text,
    Code,
    Image,
    Audio,
    Video,
    Math,
    Chemistry,
    DNA,
    EEG,
}

impl ModalityType {
    /// Get the pipeline ID for this modality
    pub fn pipeline_id(&self) -> u32 {
        match self {
            ModalityType::Text => 100,
            ModalityType::Code => 101,
            ModalityType::Image => 102,
            ModalityType::Audio => 103,
            ModalityType::Video => 104,
            ModalityType::Math => 105,
            ModalityType::Chemistry => 106,
            ModalityType::DNA => 107,
            ModalityType::EEG => 108,
        }
    }
    
    /// Get modality from pipeline ID
    pub fn from_pipeline_id(id: u32) -> Option<Self> {
        match id {
            100 => Some(ModalityType::Text),
            101 => Some(ModalityType::Code),
            102 => Some(ModalityType::Image),
            103 => Some(ModalityType::Audio),
            104 => Some(ModalityType::Video),
            105 => Some(ModalityType::Math),
            106 => Some(ModalityType::Chemistry),
            107 => Some(ModalityType::DNA),
            108 => Some(ModalityType::EEG),
            _ => None,
        }
    }
    
    /// Get file extensions associated with this modality
    pub fn file_extensions(&self) -> &[&str] {
        match self {
            ModalityType::Text => &["txt", "md", "doc", "docx", "pdf", "rtf", "csv", "json", "xml", "html"],
            ModalityType::Code => &["rs", "py", "js", "ts", "go", "java", "c", "cpp", "h", "hpp", "cs", "rb", "php", "swift", "kt"],
            ModalityType::Image => &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "tiff", "ico"],
            ModalityType::Audio => &["mp3", "wav", "flac", "ogg", "m4a", "aac", "wma"],
            ModalityType::Video => &["mp4", "mov", "avi", "mkv", "webm", "wmv", "flv"],
            ModalityType::Math => &["tex", "latex", "nb", "m", "mpl"],
            ModalityType::Chemistry => &["mol", "sdf", "pdb", "xyz", "cif", "mol2"],
            ModalityType::DNA => &["fasta", "fastq", "gbk", "gb", "gff", "gtf", "vcf", "sam", "bam"],
            ModalityType::EEG => &["edf", "bdf", "gdf", "set", "fif"],
        }
    }
}

// =============================================================================
// Common Interface
// =============================================================================

/// Input for modality analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityInput {
    /// Raw input data
    pub data: ModalityData,
    
    /// Source information (file path, URL, etc.)
    pub source: Option<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Analysis configuration
    pub config: AnalysisConfig,
}

/// Raw data types for modality input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModalityData {
    Text(String),
    Binary(Vec<u8>),
    Path(std::path::PathBuf),
    Url(String),
}

/// Configuration for analysis depth and options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Analysis depth: shallow, medium, deep
    pub depth: AnalysisDepth,
    
    /// Whether to include metrics calculation
    pub include_metrics: bool,
    
    /// Whether to suggest methodologies
    pub suggest_methodologies: bool,
    
    /// Custom options per modality
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum AnalysisDepth {
    Shallow,
    #[default]
    Medium,
    Deep,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            depth: AnalysisDepth::Medium,
            include_metrics: true,
            suggest_methodologies: true,
            options: HashMap::new(),
        }
    }
}

// =============================================================================
// Graph Structures
// =============================================================================

/// Unique identifier for a graph
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphId(pub String);

/// Unique identifier for a node
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

/// A node in the modality graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier
    pub node_id: NodeId,
    
    /// Type of node (varies by modality)
    pub node_type: String,
    
    /// Node content
    pub content: serde_json::Value,
    
    /// Position in source (if applicable)
    pub position: Option<SourcePosition>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Position in source material
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePosition {
    pub start_line: Option<u32>,
    pub start_col: Option<u32>,
    pub end_line: Option<u32>,
    pub end_col: Option<u32>,
    pub start_offset: Option<usize>,
    pub end_offset: Option<usize>,
}

/// An edge in the modality graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node
    pub from_node: NodeId,
    
    /// Target node
    pub to_node: NodeId,
    
    /// Type of edge
    pub edge_type: EdgeType,
    
    /// Edge weight (for semantic edges)
    pub weight: f32,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of edges in modality graphs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    // Structural edges (created by modality pipeline)
    Contains,
    References,
    Depends,
    Precedes,
    Follows,
    ParentOf,
    ChildOf,
    Calls,
    Imports,
    Exports,
    
    // Semantic edges (added by ZSEI hooks)
    RelatesTo,
    Contradicts,
    Supports,
    Extends,
    Specializes,
    Generalizes,
    SimilarTo,
    
    // Cross-modality edges
    Describes,
    Implements,
    Represents,
    Transcribes,
    Documents,
    Illustrates,
    
    // Custom edge type
    Custom(String),
}

impl EdgeType {
    pub fn is_structural(&self) -> bool {
        matches!(self, 
            EdgeType::Contains | EdgeType::References | EdgeType::Depends |
            EdgeType::Precedes | EdgeType::Follows | EdgeType::ParentOf |
            EdgeType::ChildOf | EdgeType::Calls | EdgeType::Imports | EdgeType::Exports
        )
    }
    
    pub fn is_semantic(&self) -> bool {
        matches!(self,
            EdgeType::RelatesTo | EdgeType::Contradicts | EdgeType::Supports |
            EdgeType::Extends | EdgeType::Specializes | EdgeType::Generalizes |
            EdgeType::SimilarTo
        )
    }
    
    pub fn is_cross_modality(&self) -> bool {
        matches!(self,
            EdgeType::Describes | EdgeType::Implements | EdgeType::Represents |
            EdgeType::Transcribes | EdgeType::Documents | EdgeType::Illustrates
        )
    }
}

/// Complete modality graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityGraph {
    /// Graph identifier
    pub id: GraphId,
    
    /// Modality type
    pub modality: ModalityType,
    
    /// Project this graph belongs to
    pub project_id: String,
    
    /// All nodes in the graph
    pub nodes: HashMap<NodeId, GraphNode>,
    
    /// All edges in the graph
    pub edges: Vec<GraphEdge>,
    
    /// Graph-level metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// =============================================================================
// Query Interface
// =============================================================================

/// Query for graph retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    /// Query type
    pub query_type: QueryType,
    
    /// Query parameters
    pub params: HashMap<String, serde_json::Value>,
    
    /// Maximum results
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    /// Find nodes by type
    NodesByType(String),
    
    /// Find nodes by content match
    NodesByContent(String),
    
    /// Get neighbors of a node
    Neighbors(NodeId),
    
    /// Get path between nodes
    PathBetween(NodeId, NodeId),
    
    /// Get subgraph rooted at node
    Subgraph(NodeId, usize),
    
    /// Find disconnected components
    Components,
    
    /// Custom query
    Custom(String),
}

/// Result of a graph query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub metadata: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Modality Pipeline Trait
// =============================================================================

/// Common trait for all modality pipelines
#[async_trait]
pub trait ModalityPipeline: Send + Sync {
    /// Get the modality type this pipeline handles
    fn modality_type(&self) -> ModalityType;
    
    /// Get the pipeline ID
    fn pipeline_id(&self) -> u32 {
        self.modality_type().pipeline_id()
    }
    
    /// Analyze input data and extract structural components
    async fn analyze(&self, input: ModalityInput) -> OzoneResult<AnalysisResult>;
    
    /// Create a traversable graph from analysis
    async fn create_graph(
        &self, 
        analysis: AnalysisResult, 
        project_id: &str
    ) -> OzoneResult<ModalityGraph>;
    
    /// Update an existing graph with new data
    async fn update_graph(
        &self, 
        graph: &mut ModalityGraph, 
        delta: GraphDelta
    ) -> OzoneResult<()>;
    
    /// Query the graph for information
    async fn query_graph(
        &self, 
        graph: &ModalityGraph, 
        query: GraphQuery
    ) -> OzoneResult<QueryResult>;
    
    /// Link to another modality's graph
    async fn link_to_modality(
        &self,
        source_graph: &mut ModalityGraph,
        target_graph_id: GraphId,
        relationship: EdgeType,
    ) -> OzoneResult<Vec<GraphEdge>>;
    
    /// Get ZSEI hooks this pipeline triggers
    fn get_hooks(&self) -> Vec<ZSEIHook> {
        vec![
            ZSEIHook::OnGraphCreated,
            ZSEIHook::OnInferRelationships,
        ]
    }
}

/// Result of modality analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Extracted components (varies by modality)
    pub components: serde_json::Value,
    
    /// Metrics (if requested)
    pub metrics: Option<HashMap<String, f64>>,
    
    /// Suggested methodologies
    pub suggested_methodologies: Vec<u32>,
    
    /// Warnings or issues found
    pub warnings: Vec<String>,
    
    /// Analysis metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Changes to apply to a graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDelta {
    /// Nodes to add
    pub add_nodes: Vec<GraphNode>,
    
    /// Nodes to remove (by ID)
    pub remove_nodes: Vec<NodeId>,
    
    /// Edges to add
    pub add_edges: Vec<GraphEdge>,
    
    /// Edges to remove
    pub remove_edges: Vec<(NodeId, NodeId, EdgeType)>,
    
    /// Metadata updates
    pub metadata_updates: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Provisional Nodes (for Graph-First Code Generation)
// =============================================================================

/// A provisional node created before actual generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionalNode {
    /// Provisional identifier
    pub provisional_id: String,
    
    /// Node type
    pub node_type: String,
    
    /// Planned file path
    pub planned_path: Option<std::path::PathBuf>,
    
    /// Planned name
    pub planned_name: String,
    
    /// Planned imports/dependencies
    pub planned_dependencies: Vec<String>,
    
    /// Planned exports
    pub planned_exports: Vec<String>,
    
    /// Generation order
    pub generation_order: u32,
    
    /// Current status
    pub status: ProvisionalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProvisionalStatus {
    Planned,
    Generating,
    Generated,
    Validated,
    Finalized,
    Failed,
    RolledBack,
}

// =============================================================================
// Submodules
// =============================================================================

pub mod text;
pub mod code;
pub mod image;
pub mod audio;
pub mod video;
pub mod math;
pub mod chemistry;
pub mod dna;
pub mod eeg;

// Re-exports
pub use text::TextAnalysisPipeline;
pub use code::CodeAnalysisPipeline;
// pub use image::ImageAnalysisPipeline;
// pub use audio::AudioAnalysisPipeline;
// pub use video::VideoAnalysisPipeline;
// pub use math::MathAnalysisPipeline;
// pub use chemistry::ChemistryAnalysisPipeline;
// pub use dna::DNAAnalysisPipeline;
// pub use eeg::EEGAnalysisPipeline;

// =============================================================================
// Factory
// =============================================================================

/// Factory for creating modality processors
pub struct ModalityFactory;

impl ModalityFactory {
    /// Create a processor for the given modality
    pub fn create(modality: ModalityType) -> OzoneResult<Arc<dyn ModalityPipeline>> {
        match modality {
            ModalityType::Text => {
                Ok(Arc::new(text::TextAnalysisPipeline::new()))
            }
            ModalityType::Code => {
                Ok(Arc::new(code::CodeAnalysisPipeline::new()))
            }
            _ => {
                // Other modalities are stubbed for now
                Err(OzoneError::NotImplemented(
                    format!("Modality {:?} not yet implemented", modality)
                ))
            }
        }
    }
    
    /// Create processors for all enabled modalities
    pub fn create_all(enabled: &[ModalityType]) -> HashMap<ModalityType, Arc<dyn ModalityPipeline>> {
        let mut processors = HashMap::new();
        
        for modality in enabled {
            if let Ok(processor) = Self::create(*modality) {
                processors.insert(*modality, processor);
            }
        }
        
        processors
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_modality_pipeline_ids() {
        assert_eq!(ModalityType::Text.pipeline_id(), 100);
        assert_eq!(ModalityType::Code.pipeline_id(), 101);
        assert_eq!(ModalityType::Image.pipeline_id(), 102);
        assert_eq!(ModalityType::EEG.pipeline_id(), 108);
    }
    
    #[test]
    fn test_modality_from_pipeline_id() {
        assert_eq!(ModalityType::from_pipeline_id(100), Some(ModalityType::Text));
        assert_eq!(ModalityType::from_pipeline_id(101), Some(ModalityType::Code));
        assert_eq!(ModalityType::from_pipeline_id(999), None);
    }
    
    #[test]
    fn test_edge_type_classification() {
        assert!(EdgeType::Contains.is_structural());
        assert!(EdgeType::RelatesTo.is_semantic());
        assert!(EdgeType::Describes.is_cross_modality());
        
        assert!(!EdgeType::Contains.is_semantic());
        assert!(!EdgeType::RelatesTo.is_structural());
    }
    
    #[test]
    fn test_file_extensions() {
        let code_exts = ModalityType::Code.file_extensions();
        assert!(code_exts.contains(&"rs"));
        assert!(code_exts.contains(&"py"));
        
        let dna_exts = ModalityType::DNA.file_extensions();
        assert!(dna_exts.contains(&"fasta"));
    }
}
