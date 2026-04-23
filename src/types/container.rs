//! Container types - Section 6 of the specification (ZSEI Core)
//!
//! Also houses all reserved container ID constants and ID-computation helpers
//! that were previously scattered across bootstrap.rs and store modules.
//! Single source of truth for the ZSEI container identity layer.

use super::{Blake3Hash, ContainerID, PublicKey, SemVer, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// RESERVED CONTAINER IDs — structural roots of the ZSEI hierarchy
//
// Ranges:
//   0–6        : system roots
//   10–18      : modality nodes (one per modality, under /Modality/)
//   20–31      : consciousness sphere roots
//   10000+     : pipeline containers        (10000 + pipeline_id)
//   30000+     : methodology containers     (30000 + methodology_id)
//   40000+     : blueprint containers       (40000 + blueprint_id)
//   100000+    : runtime containers         (experiences, graphs, etc.)
// ============================================================================

// ── System Roots (0–6) ─────────────────────────────────────────────────────
pub const ROOT_CONTAINER_ID: ContainerID = 0;
pub const MODALITY_ROOT_ID: ContainerID = 1;
pub const METHODOLOGY_ROOT_ID: ContainerID = 2;
pub const BLUEPRINT_ROOT_ID: ContainerID = 3;
pub const PIPELINE_ROOT_ID: ContainerID = 4;
pub const CONSCIOUSNESS_ROOT_ID: ContainerID = 5;
pub const EXTERNAL_ROOT_ID: ContainerID = 6;

// ── Core Modality Nodes (10–18) ────────────────────────────────────────────
// One node per modality pipeline, under /Modality/
pub const MODALITY_TEXT_ID: ContainerID = 10;
pub const MODALITY_CODE_ID: ContainerID = 11;
pub const MODALITY_IMAGE_ID: ContainerID = 12;
pub const MODALITY_AUDIO_ID: ContainerID = 13;
pub const MODALITY_VIDEO_ID: ContainerID = 14;
pub const MODALITY_MATH_ID: ContainerID = 15;
pub const MODALITY_CHEMISTRY_ID: ContainerID = 16;
pub const MODALITY_DNA_ID: ContainerID = 17;
pub const MODALITY_EEG_ID: ContainerID = 18;

// ── Extended Modality Nodes (19–35) ───────────────────────────────────────
// Engines and new modalities added in v2
pub const MODALITY_3D_ID: ContainerID = 19;
pub const MODALITY_SOUND_ID: ContainerID = 20; // sound reconstruction
pub const MODALITY_BIOLOGY_ID: ContainerID = 21;
pub const MODALITY_PROTEOMICS_ID: ContainerID = 22;
pub const MODALITY_HAPTIC_ID: ContainerID = 23;
pub const MODALITY_THERMAL_ID: ContainerID = 24;
pub const MODALITY_DEPTH_ID: ContainerID = 25;
pub const MODALITY_IMU_ID: ContainerID = 26;
pub const MODALITY_GEOSPATIAL_ID: ContainerID = 27;
pub const MODALITY_ELECTROMAGNETIC_ID: ContainerID = 28;
pub const MODALITY_BCI_ID: ContainerID = 29;
pub const MODALITY_PARAMETRIC_CAD_ID: ContainerID = 30;
pub const MODALITY_KINEMATICS_ID: ContainerID = 31;
pub const MODALITY_CONTROL_SYSTEMS_ID: ContainerID = 32;
pub const MODALITY_NETWORK_TOPOLOGY_ID: ContainerID = 33;
pub const MODALITY_RADAR_ID: ContainerID = 34;
pub const MODALITY_SONAR_ID: ContainerID = 35;
pub const MODALITY_HYPERSPECTRAL_ID: ContainerID = 36;

// ── Consciousness Sphere Roots (50–65) ────────────────────────────────────
pub const CONSCIOUSNESS_EXPERIENCE_ROOT_ID: ContainerID = 50;
pub const CONSCIOUSNESS_CORE_MEMORY_ROOT_ID: ContainerID = 51;
pub const CONSCIOUSNESS_EMOTIONAL_ROOT_ID: ContainerID = 52;
pub const CONSCIOUSNESS_IDENTITY_ROOT_ID: ContainerID = 53;
pub const CONSCIOUSNESS_METACOGNITION_ROOT_ID: ContainerID = 54;
pub const CONSCIOUSNESS_RELATIONSHIPS_ROOT_ID: ContainerID = 55;
pub const CONSCIOUSNESS_ETHICS_ROOT_ID: ContainerID = 56;
pub const CONSCIOUSNESS_NARRATIVES_ROOT_ID: ContainerID = 57;
pub const CONSCIOUSNESS_COLLECTIVE_ROOT_ID: ContainerID = 58;

// ── External Roots (70–71) ────────────────────────────────────────────────
pub const EXTERNAL_PACKAGES_ROOT_ID: ContainerID = 70;
pub const EXTERNAL_URLS_ROOT_ID: ContainerID = 71;

// ── Runtime Graph Roots (75–79) ───────────────────────────────────────────
// Stores for chunk graphs, file graphs, and cross-modal indices
pub const CHUNK_GRAPH_ROOT_ID: ContainerID = 75;
pub const FILE_GRAPH_ROOT_ID: ContainerID = 76;
pub const CROSS_MODAL_INDEX_ROOT_ID: ContainerID = 77;

// ============================================================================
// ID COMPUTATION HELPERS
//
// These map domain IDs to their reserved container ID ranges.
// Used by pipeline store, blueprint store, methodology store, etc.
// ============================================================================

/// Returns the container ID for a pipeline (range: 10000+).
#[inline]
pub const fn pipeline_container_id(pipeline_id: u64) -> ContainerID {
    10000 + pipeline_id
}

/// Returns the container ID for a methodology (range: 30000+).
#[inline]
pub const fn methodology_container_id(methodology_id: u64) -> ContainerID {
    30000 + methodology_id
}

/// Returns the container ID for a blueprint (range: 40000+).
#[inline]
pub const fn blueprint_container_id(blueprint_id: u64) -> ContainerID {
    40000 + blueprint_id
}

/// Returns the container ID for a runtime experience (range: 100000+).
#[inline]
pub const fn experience_container_id(experience_id: u64) -> ContainerID {
    100000 + experience_id
}

/// Returns the container ID for a modality graph (range: 200000+).
#[inline]
pub const fn modality_graph_container_id(graph_id: u64) -> ContainerID {
    200000 + graph_id
}

/// Returns the container ID for a chunk graph (range: 300000+).
#[inline]
pub const fn chunk_graph_container_id(chunk_graph_id: u64) -> ContainerID {
    300000 + chunk_graph_id
}

/// Returns the container ID for a file graph (range: 400000+).
#[inline]
pub const fn file_graph_container_id(file_graph_id: u64) -> ContainerID {
    400000 + file_graph_id
}

// ============================================================================
// CONTAINER STRUCT
// ============================================================================

/// Container — the fundamental unit in ZSEI (§6.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub global_state: GlobalState,
    pub local_state: LocalState,
}

/// Global State — ALWAYS a list of IDs (mmap-friendly) (§6.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalState {
    pub container_id: ContainerID,
    pub child_count: u32,
    pub version: u32,
    pub parent_id: ContainerID,
    pub child_ids: Vec<ContainerID>,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            container_id: 0,
            child_count: 0,
            version: 1,
            parent_id: 0,
            child_ids: Vec::new(),
        }
    }
}

/// Local State — Metadata, context, pointers (§6.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalState {
    pub metadata: Metadata,
    pub context: Context,
    pub storage: StoragePointers,
    pub hints: TraversalHints,
    pub integrity: IntegrityData,

    // Type-specific context
    pub file_context: Option<FileContext>,
    pub code_context: Option<CodeContext>,
    pub text_context: Option<TextContext>,
    pub external_ref: Option<ExternalReference>,
}

impl Default for LocalState {
    fn default() -> Self {
        Self {
            metadata: Metadata::default(),
            context: Context::default(),
            storage: StoragePointers::default(),
            hints: TraversalHints::default(),
            integrity: IntegrityData::default(),
            file_context: None,
            code_context: None,
            text_context: None,
            external_ref: None,
        }
    }
}

/// Container metadata (§6.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub container_type: ContainerType,
    pub modality: Modality,
    pub created_at: u64,
    pub updated_at: u64,
    pub provenance: String,
    pub permissions: u64,
    pub owner_id: u64,
    pub name: Option<String>,
    pub materialized_path: Option<String>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            container_type: ContainerType::Root,
            modality: Modality::Unknown,
            created_at: 0,
            updated_at: 0,
            provenance: String::new(),
            permissions: 0,
            owner_id: 0,
            name: None,
            materialized_path: None,
        }
    }
}

/// Context information for traversal and retrieval (§6.2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Context {
    pub categories: Vec<ContainerID>,
    pub methodologies: Vec<ContainerID>,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub relationships: Vec<Relation>,
    pub learned_associations: Vec<Association>,
    pub embedding: Option<Vec<f32>>,
}

/// Relationship between containers (§6.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub target_id: ContainerID,
    pub relation_type: RelationType,
    pub confidence: f32,
    pub discovered_via: DiscoveryMethod,
}

/// Relationship types (§6.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum RelationType {
    // Structural
    DependsOn = 1,
    PartOf = 2,
    Contains = 3,

    // Semantic
    SimilarTo = 10,
    RelatedTo = 11,
    Contradicts = 12,
    Supersedes = 13,

    // Code-specific
    ImportsFrom = 20,
    ExportsTo = 21,
    CallsTo = 22,
    CalledBy = 23,
    Implements = 24,
    Extends = 25,

    // Temporal
    Precedes = 30,
    Follows = 31,

    // Reference
    References = 40,
    ReferencedBy = 41,

    // External
    DocumentedAt = 50,
    SourcedFrom = 51,

    Custom = 65535,
}

/// Discovery method for relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    Manual,
    ZeroShot,
    Traversal,
    MLPrediction,
    CodeAnalysis,
    TextAnalysis,
    WebNavigation,
}

/// Learned association between containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Association {
    pub related_container: ContainerID,
    pub strength: f32,
    pub discovered_at: u64,
    pub source: String,
}

/// Storage pointers (§6.2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoragePointers {
    pub db_shard_id: Option<u64>,
    pub vector_index_ref: Option<String>,
    pub object_store_path: Option<String>,
    pub compression_type: CompressionType,
}

/// Compression types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum CompressionType {
    #[default]
    None = 0,
    LZ4 = 1,
    Zstd = 2,
    Custom = 255,
}

/// Traversal hints for optimization (§6.2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraversalHints {
    pub access_frequency: u32,
    pub hotness_score: f32,
    pub last_accessed: u64,
    pub centroid: Option<Vec<f32>>,
    pub ml_prediction_weight: f32,
}

/// Integrity data for verification (§6.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityData {
    pub content_hash: Blake3Hash,
    pub semantic_fingerprint: Vec<f32>,
    pub last_verified: u64,
    pub integrity_score: f32,
    pub version_history: Vec<VersionRecord>,
}

impl Default for IntegrityData {
    fn default() -> Self {
        Self {
            content_hash: [0u8; 32],
            semantic_fingerprint: Vec::new(),
            last_verified: 0,
            integrity_score: 1.0,
            version_history: Vec::new(),
        }
    }
}

/// Version record for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRecord {
    pub version: u64,
    pub timestamp: u64,
    pub content_hash: Blake3Hash,
    pub change_type: ChangeType,
    pub rollback_available: bool,
}

/// Change types for version tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Create,
    Update,
    Delete,
    Merge,
}

// ============================================================================
// CONTAINER TYPES
// ============================================================================

/// Container types (§6.3)
///
/// Variants are grouped and annotated.  The `repr(u16)` discriminant is stored
/// on disk and must remain stable across versions — only append, never renumber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(u16)]
pub enum ContainerType {
    // ── System roots ────────────────────────────────────────────────────────
    #[default]
    Root = 0,

    // ── User organisation ────────────────────────────────────────────────────
    User = 1,
    Workspace = 2,
    Project = 3,

    // ── Global / distributed ─────────────────────────────────────────────────
    Modality = 10,
    Category = 11,
    SubCategory = 12,

    // ── Knowledge ────────────────────────────────────────────────────────────
    Methodology = 20,
    Blueprint = 21,
    Pipeline = 22,

    // ── Execution ────────────────────────────────────────────────────────────
    Task = 30,
    TaskContext = 31,
    TaskExecutionState = 32,

    // ── Data ─────────────────────────────────────────────────────────────────
    Dataset = 40,
    Shard = 41,
    Document = 42,
    Chunk = 43,
    Embedding = 44,

    // ── File references (NOT copies) ─────────────────────────────────────────
    FileReference = 50,
    DirectoryReference = 51,

    // ── External references (NOT copies) ─────────────────────────────────────
    URLReference = 55,
    PackageReference = 56,

    // ── Code-specific ─────────────────────────────────────────────────────────
    CodeModule = 60,
    CodeFunction = 61,
    CodeClass = 62,
    CodeDependency = 63,

    // ── Text-specific ─────────────────────────────────────────────────────────
    TextDocument = 70,
    TextSection = 71,
    TextParagraph = 72,
    TextTheme = 73,

    // ── Computed ─────────────────────────────────────────────────────────────
    Derived = 80,
    Virtual = 99,

    // ── Consciousness (Part II) ───────────────────────────────────────────────
    ExperienceMemory = 100,
    CoreMemory = 101,
    EmotionalContext = 102,
    IdentityState = 103,
    Relationship = 104,
    EthicalPrinciple = 105,
    Narrative = 106,
    CollectiveWisdom = 107,
    Experience = 108,
    EmotionalStateRecord = 109,

    // ── Hierarchy root nodes ─────────────────────────────────────────────────
    ModalityRoot = 200,      // /Modality/ root
    MethodologyRoot = 201,   // /Methodologies/ root
    BlueprintRoot = 202,     // /Blueprints/ root
    PipelineRoot = 203,      // /Pipelines/ root
    ConsciousnessRoot = 204, // /Consciousness/ root
    ExternalRoot = 205,      // /External/ root
    PackageRoot = 206,       // /External/Packages/
    URLRoot = 207,           // /External/URLs/
    ModalityGraph = 210,     // generic modality graph container

    // ── Consciousness sub-roots ───────────────────────────────────────────────
    CoreMemoryRoot = 211,    // /Consciousness/CoreMemory/
    EmotionalRoot = 212,     // /Consciousness/Emotional/
    IdentityRoot = 213,      // /Consciousness/Identity/
    MetaCognitionRoot = 214, // /Consciousness/MetaCognition/
    RelationshipRoot = 215,  // /Consciousness/Relationships/
    EthicsRoot = 216,        // /Consciousness/Ethics/
    NarrativeRoot = 217,     // /Consciousness/Narratives/
    CollectiveRoot = 218,    // /Consciousness/Collective/

    // ── Modality-specific roots (v2 — engine pipelines 109–126) ─────────────
    ThreeDRoot = 220,              // /Modality/3D/           (pipeline 109)
    SoundReconstructionRoot = 221, // /Modality/Sound/      (pipeline 110)
    BiologyRoot = 222,             // /Modality/Biology/      (pipeline 111)
    ProteomicsRoot = 223,          // /Modality/Proteomics/   (pipeline 112)
    HapticRoot = 224,              // /Modality/Haptic/       (pipeline 113)
    ThermalRoot = 225,             // /Modality/Thermal/      (pipeline 114)
    DepthRoot = 226,               // /Modality/Depth/        (pipeline 115)
    IMURoot = 227,                 // /Modality/IMU/          (pipeline 116)
    GeospatialRoot = 228,          // /Modality/Geospatial/   (pipeline 117)
    ElectromagneticRoot = 229,     // /Modality/Electromagnetic/ (pipeline 118)
    BCIRoot = 230,                 // /Modality/BCI/          (pipeline 119)
    ParametricCADRoot = 231,       // /Modality/ParametricCAD/ (pipeline 120)
    KinematicsRoot = 232,          // /Modality/Kinematics/   (pipeline 121)
    ControlSystemsRoot = 233,      // /Modality/ControlSystems/ (pipeline 122)
    NetworkTopologyRoot = 234,     // /Modality/NetworkTopology/ (pipeline 123)
    ActiveRadarRoot = 235,         // /Modality/Radar/        (pipeline 124)
    ActiveSonarRoot = 236,         // /Modality/Sonar/        (pipeline 125)
    HyperspectralRoot = 237,       // /Modality/Hyperspectral/ (pipeline 126)

    // ── Runtime graph stores ───────────────────────────────────────────────
    ChunkGraphRoot = 240,      // /ChunkGraphs/    — persistent chunk graphs
    FileGraphRoot = 241,       // /FileGraphs/     — per-file modality graphs
    CrossModalIndexRoot = 242, // /CrossModalIndex/ — cross-modal link registry
    ChunkGraph = 243,          // individual chunk graph container
    FileGraph = 244,           // individual file graph container
}

impl ContainerType {
    /// Human-readable name (matches folder names used in ZSEI paths).
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Root => "Root",
            Self::User => "User",
            Self::Workspace => "Workspace",
            Self::Project => "Project",
            Self::Modality => "Modality",
            Self::Category => "Category",
            Self::SubCategory => "SubCategory",
            Self::Methodology => "Methodology",
            Self::Blueprint => "Blueprint",
            Self::Pipeline => "Pipeline",
            Self::Task => "Task",
            Self::TaskContext => "TaskContext",
            Self::TaskExecutionState => "TaskExecutionState",
            Self::Dataset => "Dataset",
            Self::Shard => "Shard",
            Self::Document => "Document",
            Self::Chunk => "Chunk",
            Self::Embedding => "Embedding",
            Self::FileReference => "FileReference",
            Self::DirectoryReference => "DirectoryReference",
            Self::URLReference => "URLReference",
            Self::PackageReference => "PackageReference",
            Self::CodeModule => "CodeModule",
            Self::CodeFunction => "CodeFunction",
            Self::CodeClass => "CodeClass",
            Self::CodeDependency => "CodeDependency",
            Self::TextDocument => "TextDocument",
            Self::TextSection => "TextSection",
            Self::TextParagraph => "TextParagraph",
            Self::TextTheme => "TextTheme",
            Self::Derived => "Derived",
            Self::Virtual => "Virtual",
            Self::ExperienceMemory => "ExperienceMemory",
            Self::CoreMemory => "CoreMemory",
            Self::EmotionalContext => "EmotionalContext",
            Self::IdentityState => "IdentityState",
            Self::Relationship => "Relationship",
            Self::EthicalPrinciple => "EthicalPrinciple",
            Self::Narrative => "Narrative",
            Self::CollectiveWisdom => "CollectiveWisdom",
            Self::Experience => "Experience",
            Self::EmotionalStateRecord => "EmotionalStateRecord",
            Self::ModalityRoot => "ModalityRoot",
            Self::MethodologyRoot => "MethodologyRoot",
            Self::BlueprintRoot => "BlueprintRoot",
            Self::PipelineRoot => "PipelineRoot",
            Self::ConsciousnessRoot => "ConsciousnessRoot",
            Self::ExternalRoot => "ExternalRoot",
            Self::PackageRoot => "PackageRoot",
            Self::URLRoot => "URLRoot",
            Self::ModalityGraph => "ModalityGraph",
            Self::CoreMemoryRoot => "CoreMemoryRoot",
            Self::EmotionalRoot => "EmotionalRoot",
            Self::IdentityRoot => "IdentityRoot",
            Self::MetaCognitionRoot => "MetaCognitionRoot",
            Self::RelationshipRoot => "RelationshipRoot",
            Self::EthicsRoot => "EthicsRoot",
            Self::NarrativeRoot => "NarrativeRoot",
            Self::CollectiveRoot => "CollectiveRoot",
            Self::ThreeDRoot => "ThreeDRoot",
            Self::SoundReconstructionRoot => "SoundReconstructionRoot",
            Self::BiologyRoot => "BiologyRoot",
            Self::ProteomicsRoot => "ProteomicsRoot",
            Self::HapticRoot => "HapticRoot",
            Self::ThermalRoot => "ThermalRoot",
            Self::DepthRoot => "DepthRoot",
            Self::IMURoot => "IMURoot",
            Self::GeospatialRoot => "GeospatialRoot",
            Self::ElectromagneticRoot => "ElectromagneticRoot",
            Self::BCIRoot => "BCIRoot",
            Self::ParametricCADRoot => "ParametricCADRoot",
            Self::KinematicsRoot => "KinematicsRoot",
            Self::ControlSystemsRoot => "ControlSystemsRoot",
            Self::NetworkTopologyRoot => "NetworkTopologyRoot",
            Self::ActiveRadarRoot => "ActiveRadarRoot",
            Self::ActiveSonarRoot => "ActiveSonarRoot",
            Self::HyperspectralRoot => "HyperspectralRoot",
            Self::ChunkGraphRoot => "ChunkGraphRoot",
            Self::FileGraphRoot => "FileGraphRoot",
            Self::CrossModalIndexRoot => "CrossModalIndexRoot",
            Self::ChunkGraph => "ChunkGraph",
            Self::FileGraph => "FileGraph",
        }
    }

    /// Whether this container type represents a hierarchy root that should be
    /// created unconditionally during bootstrap.
    pub fn is_bootstrap_root(&self) -> bool {
        matches!(
            self,
            Self::ModalityRoot
                | Self::MethodologyRoot
                | Self::BlueprintRoot
                | Self::PipelineRoot
                | Self::ConsciousnessRoot
                | Self::ExternalRoot
                | Self::PackageRoot
                | Self::URLRoot
                | Self::CoreMemoryRoot
                | Self::EmotionalRoot
                | Self::IdentityRoot
                | Self::MetaCognitionRoot
                | Self::RelationshipRoot
                | Self::EthicsRoot
                | Self::NarrativeRoot
                | Self::CollectiveRoot
                | Self::ThreeDRoot
                | Self::SoundReconstructionRoot
                | Self::BiologyRoot
                | Self::ProteomicsRoot
                | Self::HapticRoot
                | Self::ThermalRoot
                | Self::DepthRoot
                | Self::IMURoot
                | Self::GeospatialRoot
                | Self::ElectromagneticRoot
                | Self::BCIRoot
                | Self::ParametricCADRoot
                | Self::KinematicsRoot
                | Self::ControlSystemsRoot
                | Self::NetworkTopologyRoot
                | Self::ActiveRadarRoot
                | Self::ActiveSonarRoot
                | Self::HyperspectralRoot
                | Self::ChunkGraphRoot
                | Self::FileGraphRoot
                | Self::CrossModalIndexRoot
        )
    }
}

// ============================================================================
// MODALITY TYPES
// ============================================================================

/// Modality types (§6.4)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum Modality {
    #[default]
    Unknown = 0,
    Text = 1,
    Code = 2,
    Image = 3,
    Audio = 4,
    Video = 5,
    Graph = 6,
    TimeSeries = 7,
    Structured = 8,
    External = 9,
    Multimodal = 255,
}

// ============================================================================
// FILE CONTEXT (§7.2)
// ============================================================================

/// File context for linked files (§7.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub file_path: String,
    pub file_hash: Blake3Hash,
    pub file_size: u64,
    pub last_modified: u64,
    pub last_analyzed: u64,
    pub modality: Modality,
    pub analysis_version: u32,
    pub semantic_summary: String,
    pub key_concepts: Vec<String>,
    pub relationships: Vec<FileRelation>,
    pub chunks: Vec<ContextChunk>,
    pub code_context: Option<Box<CodeContext>>,
    pub text_context: Option<Box<TextContext>>,
    pub integrity: ChunkIntegrity,
}

/// File relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRelation {
    pub target_file: ContainerID,
    pub relation_type: RelationType,
    pub strength: f32,
}

/// Context chunk for semantic storage (§7.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    pub chunk_id: u64,
    pub chunk_type: ChunkType,
    pub content_summary: String,
    pub semantic_embedding: Vec<f32>,
    pub position_in_file: (u64, u64),
    pub relationships: Vec<ChunkRelation>,
    pub leading_context: String,
    pub trailing_context: String,
    pub content_hash: Blake3Hash,
}

/// Chunk types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkType {
    // Code
    Function,
    Class,
    Module,
    Import,

    // Text
    Paragraph,
    Section,
    Heading,

    // Generic
    Block,
}

/// Chunk relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRelation {
    pub target_chunk: u64,
    pub relation_type: RelationType,
}

/// Chunk integrity data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkIntegrity {
    pub total_chunks: u32,
    pub chunk_hashes: Vec<Blake3Hash>,
    pub cross_chunk_verification: bool,
    pub reconstruction_verified: bool,
    pub last_integrity_check: u64,
}

// ============================================================================
// CODE CONTEXT (§8.2)
// ============================================================================

/// Code context (§8.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    pub language: ProgrammingLanguage,
    pub parser_version: u32,
    pub ast_summary: ASTSummary,
    pub modules: Vec<ModuleInfo>,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<ExportInfo>,
    pub dependencies: Vec<DependencyInfo>,
    pub dependents: Vec<ContainerID>,
    pub package_contexts: Vec<PackageContextRef>,
    pub patterns_detected: Vec<PatternInfo>,
    pub architectural_layer: Option<ArchitecturalLayer>,
    pub quality_metrics: QualityMetrics,
    pub call_graph: CallGraph,
    pub data_flow: DataFlowGraph,
    pub type_dependencies: Vec<TypeDependency>,
    pub doc_integrity: Option<CodeDocIntegrity>,
}

/// Programming languages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
    Cpp,
    C,
    Ruby,
    PHP,
    Swift,
    Kotlin,
    Scala,
    Custom(String),
}

/// AST summary
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ASTSummary {
    pub node_count: u32,
    pub depth: u16,
    pub complexity_score: f32,
}

/// Module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub module_id: u64,
    pub name: String,
    pub path: String,
    pub visibility: Visibility,
    pub purpose_summary: String,
    pub imports: Vec<u64>,
    pub exports: Vec<u64>,
    pub functions: Vec<u64>,
    pub classes: Vec<u64>,
}

/// Function information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub function_id: u64,
    pub name: String,
    pub visibility: Visibility,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub purpose_summary: String,
    pub behavior_description: String,
    pub complexity: ComplexityMetrics,
    pub calls_to: Vec<u64>,
    pub called_by: Vec<u64>,
}

/// Parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
    pub purpose: String,
}

/// Class information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub class_id: u64,
    pub name: String,
    pub visibility: Visibility,
    pub parent_classes: Vec<String>,
    pub interfaces: Vec<String>,
    pub methods: Vec<u64>,
    pub properties: Vec<PropertyInfo>,
    pub purpose_summary: String,
}

/// Property information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    pub type_annotation: Option<String>,
    pub visibility: Visibility,
    pub is_static: bool,
}

/// Import information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub import_id: u64,
    pub source: String,
    pub items: Vec<String>,
    pub is_external: bool,
    pub package_version: Option<String>,
    pub package_ref: Option<ContainerID>,
}

/// Export information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportInfo {
    pub export_id: u64,
    pub name: String,
    pub export_type: ExportType,
}

/// Export types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportType {
    Function,
    Class,
    Constant,
    Type,
    Module,
    Default,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub package_name: String,
    pub version_constraint: String,
    pub resolved_version: Option<String>,
    pub is_dev_dependency: bool,
    pub is_optional: bool,
    pub package_ref: Option<ContainerID>,
}

/// Package context reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageContextRef {
    pub package_ref_id: ContainerID,
    pub used_apis: Vec<String>,
    pub relationship_type: PackageRelationType,
}

/// Package relationship types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageRelationType {
    DirectDependency,
    TransitiveDependency,
    DevDependency,
    PeerDependency,
}

/// Pattern information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternInfo {
    pub pattern_type: DesignPattern,
    pub confidence: f32,
    pub involved_elements: Vec<u64>,
}

/// Design patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesignPattern {
    Singleton,
    Factory,
    Observer,
    Strategy,
    Decorator,
    Adapter,
    Facade,
    Repository,
    ServiceLocator,
    DependencyInjection,
    MVC,
    MVVM,
    Custom(String),
}

/// Architectural layers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchitecturalLayer {
    Presentation,
    Application,
    Domain,
    Infrastructure,
    DataAccess,
    API,
    Utility,
}

/// Quality metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityMetrics {
    pub cyclomatic_complexity: f32,
    pub cognitive_complexity: f32,
    pub maintainability_index: f32,
    pub test_coverage: Option<f32>,
    pub documentation_coverage: f32,
}

/// Call graph
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CallGraph {
    pub nodes: Vec<CallGraphNode>,
    pub edges: Vec<CallGraphEdge>,
}

/// Call graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphNode {
    pub function_id: u64,
    pub call_depth: u16,
    pub fan_in: u32,
    pub fan_out: u32,
}

/// Call graph edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphEdge {
    pub caller: u64,
    pub callee: u64,
    pub call_count: u32,
    pub is_recursive: bool,
}

/// Data flow graph
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataFlowGraph {
    pub nodes: Vec<DataFlowNode>,
    pub edges: Vec<DataFlowEdge>,
}

/// Data flow node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowNode {
    pub node_id: u64,
    pub node_type: DataFlowNodeType,
    pub name: String,
}

/// Data flow node types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataFlowNodeType {
    Variable,
    Parameter,
    Return,
    Property,
    External,
}

/// Data flow edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowEdge {
    pub source: u64,
    pub target: u64,
    pub flow_type: DataFlowType,
}

/// Data flow types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataFlowType {
    Assignment,
    Transformation,
    PassThrough,
    Conditional,
}

/// Type dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDependency {
    pub source_type: String,
    pub target_type: String,
    pub dependency_type: TypeDependencyType,
}

/// Type dependency types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeDependencyType {
    Inheritance,
    Implementation,
    Usage,
    Generic,
}

/// Visibility levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Module,
}

/// Complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplexityMetrics {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub lines_of_code: u32,
    pub parameter_count: u8,
}

/// Code-doc integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDocIntegrity {
    pub code_entity: u64,
    pub doc_entity: Option<u64>,
    pub forward_verified: bool,
    pub backward_verified: bool,
    pub last_check: u64,
    pub discrepancies: Vec<Discrepancy>,
}

/// Discrepancy information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discrepancy {
    pub discrepancy_type: DiscrepancyType,
    pub location: String,
    pub description: String,
    pub severity: super::Severity,
}

/// Discrepancy types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscrepancyType {
    MissingDocumentation,
    OutdatedDocumentation,
    Contradiction,
    UndocumentedBehavior,
}

// ============================================================================
// TEXT CONTEXT (§9.2)
// ============================================================================

/// Text context (§9.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContext {
    pub document_type: DocumentType,
    pub language: NaturalLanguage,
    pub structure: DocumentStructure,
    pub themes: Vec<ThemeInfo>,
    pub concepts: Vec<ConceptInfo>,
    pub arguments: Vec<ArgumentInfo>,
    pub purpose: DocumentPurpose,
    pub audience: Option<AudienceProfile>,
    pub tone: ToneProfile,
    pub effectiveness_metrics: EffectivenessMetrics,
    pub thematic_relationships: Vec<ThematicRelation>,
    pub conceptual_connections: Vec<ConceptualConnection>,
    pub structure_integrity: StructureIntegrity,
}

/// Document types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentType {
    Article,
    Report,
    Essay,
    Documentation,
    Tutorial,
    Email,
    Letter,
    Contract,
    Specification,
    Narrative,
    Research,
    Custom(String),
}

/// Natural languages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NaturalLanguage {
    English,
    Spanish,
    French,
    German,
    Chinese,
    Japanese,
    Korean,
    Portuguese,
    Russian,
    Arabic,
    Custom(String),
}

/// Document structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentStructure {
    pub total_sections: u32,
    pub total_paragraphs: u32,
    pub total_sentences: u32,
    pub total_words: u32,
    pub hierarchy: Vec<StructureNode>,
}

/// Structure node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureNode {
    pub node_id: u64,
    pub node_type: StructureNodeType,
    pub level: u8,
    pub title: Option<String>,
    pub summary: String,
    pub position: (u64, u64),
    pub children: Vec<u64>,
}

/// Structure node types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureNodeType {
    Document,
    Section,
    Subsection,
    Paragraph,
    Sentence,
    List,
    ListItem,
    Quote,
    CodeBlock,
}

/// Theme information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeInfo {
    pub theme_id: u64,
    pub name: String,
    pub description: String,
    pub occurrences: Vec<ThemeOccurrence>,
    pub evolution: Vec<ThemeEvolution>,
    pub strength: f32,
}

/// Theme occurrence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeOccurrence {
    pub location: u64,
    pub context: String,
    pub relevance: f32,
}

/// Theme evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEvolution {
    pub position_percent: f32,
    pub intensity: f32,
}

/// Concept information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptInfo {
    pub concept_id: u64,
    pub name: String,
    pub definition: String,
    pub related_concepts: Vec<u64>,
    pub occurrences: Vec<u64>,
}

/// Argument information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentInfo {
    pub argument_id: u64,
    pub claim: String,
    pub supporting_evidence: Vec<EvidenceInfo>,
    pub counter_arguments: Vec<u64>,
    pub strength: f32,
    pub location: u64,
}

/// Evidence information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceInfo {
    pub evidence_type: EvidenceType,
    pub content: String,
    pub source: Option<String>,
    pub location: u64,
}

/// Evidence types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    Fact,
    Statistic,
    Quote,
    Example,
    Analogy,
    Expert,
    Research,
}

/// Document purpose
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPurpose {
    pub primary_purpose: PurposeType,
    pub secondary_purposes: Vec<PurposeType>,
    pub target_outcome: String,
}

/// Purpose types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurposeType {
    Inform,
    Persuade,
    Entertain,
    Instruct,
    Document,
    Analyze,
    Propose,
    Report,
}

/// Audience profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceProfile {
    pub expertise_level: ExpertiseLevel,
    pub assumed_knowledge: Vec<String>,
    pub communication_preferences: Vec<String>,
}

/// Expertise levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpertiseLevel {
    Novice,
    Intermediate,
    Advanced,
    Expert,
    Mixed,
}

/// Tone profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToneProfile {
    pub formality: f32,
    pub objectivity: f32,
    pub confidence: f32,
    pub emotions_detected: Vec<EmotionDetection>,
}

/// Emotion detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionDetection {
    pub emotion: String,
    pub intensity: f32,
    pub locations: Vec<u64>,
}

/// Effectiveness metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EffectivenessMetrics {
    pub clarity_score: f32,
    pub coherence_score: f32,
    pub completeness_score: f32,
    pub engagement_score: f32,
    pub improvement_suggestions: Vec<ImprovementSuggestion>,
}

/// Improvement suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementSuggestion {
    pub location: u64,
    pub suggestion_type: SuggestionType,
    pub description: String,
    pub priority: super::Priority,
}

/// Suggestion types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionType {
    Clarity,
    Structure,
    Evidence,
    Flow,
    Tone,
    Grammar,
}

/// Thematic relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThematicRelation {
    pub source_theme: u64,
    pub target_theme: u64,
    pub relation_type: ThematicRelationType,
    pub strength: f32,
}

/// Thematic relation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThematicRelationType {
    Supports,
    Contrasts,
    Extends,
    Prerequisites,
    Concludes,
}

/// Conceptual connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptualConnection {
    pub source_concept: u64,
    pub target_concept: u64,
    pub connection_type: ConnectionType,
    pub explicit: bool,
}

/// Connection types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    Definition,
    Example,
    Contrast,
    Cause,
    Effect,
    Similarity,
}

/// Structure integrity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructureIntegrity {
    pub paragraph_boundaries_preserved: bool,
    pub sentence_boundaries_preserved: bool,
    pub thematic_coherence_score: f32,
    pub reconstruction_verified: bool,
}

// ============================================================================
// EXTERNAL REFERENCES (§23.2)
// ============================================================================

/// External reference (§23.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExternalReference {
    URL(URLReference),
    Package(PackageReference),
}

/// URL reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct URLReference {
    pub url_ref_id: ContainerID,
    pub url: String,
    pub captured_at: u64,
    pub last_verified: u64,
    pub semantic_snapshot: SemanticSnapshot,
    pub availability_status: AvailabilityStatus,
    pub content_changed: bool,
    pub change_severity: Option<ChangeSeverity>,
    pub linked_to: Vec<ContainerID>,
    pub related_packages: Vec<ContainerID>,
}

/// Package reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageReference {
    pub package_ref_id: ContainerID,
    pub registry: PackageRegistry,
    pub name: String,
    pub version_spec: String,
    pub resolved_version: Option<String>,
    pub captured_at: u64,
    pub last_verified: u64,
    pub api_snapshot: APISnapshot,
    pub documentation_snapshot: Option<SemanticSnapshot>,
    pub is_outdated: bool,
    pub latest_version: Option<String>,
    pub breaking_changes: Vec<BreakingChangeInfo>,
    pub deprecations: Vec<String>,
    pub source_url: Option<String>,
    pub documentation_url: Option<String>,
    pub repository_url: Option<String>,
    pub used_by: Vec<ContainerID>,
    pub depends_on: Vec<ContainerID>,
}

/// Package registries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageRegistry {
    Npm,
    Crates,
    PyPI,
    Maven,
    NuGet,
    Go,
    Custom(String),
}

/// Semantic snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SemanticSnapshot {
    pub summary: String,
    pub key_concepts: Vec<String>,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub embedding: Vec<f32>,
    pub structure_outline: Option<String>,
    pub captured_at: u64,
}

/// Availability status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvailabilityStatus {
    Available,
    Unavailable,
    Redirected { new_url: String },
    RequiresAuth,
    RateLimited,
    Unknown,
}

/// Change severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeSeverity {
    None,
    Minor,
    Moderate,
    Major,
    CompletelyDifferent,
}

/// API snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct APISnapshot {
    pub exports: Vec<ExportedAPI>,
    pub types: Vec<TypeDefinition>,
    pub version: String,
    pub captured_at: u64,
}

/// Exported API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedAPI {
    pub name: String,
    pub api_type: APIType,
    pub signature: String,
    pub description: String,
}

/// API types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum APIType {
    Function,
    Class,
    Type,
    Constant,
    Module,
}

/// Type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub definition: String,
}

/// Breaking change info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChangeInfo {
    pub from_version: String,
    pub to_version: String,
    pub description: String,
    pub affected_apis: Vec<String>,
    pub migration_guide: Option<String>,
}
