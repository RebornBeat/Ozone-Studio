//! Pipeline types - Section 10 of the specification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{PipelineID, TaskID, Blake3Hash, PublicKey, Value, SemVer, ContainerID};

/// Pipeline metadata (§10.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineMetadata {
    pub author: PublicKey,
    pub description: String,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub language: Language,
    pub runtime_requirements: Vec<Dependency>,
}

/// Supported languages for pipeline implementation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Custom(String),
}

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub optional: bool,
}

/// Schema for pipeline inputs/outputs (§10.1)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schema {
    pub fields: Vec<Field>,
    pub validation_rules: Vec<ValidationRule>,
}

/// Field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<Value>,
    pub description: String,
}

/// Field types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    Text,
    Number,
    Float,
    Bool,
    Array(Box<FieldType>),
    Object(Schema),
    ContainerID,
    TaskID,
    PipelineID,
    Binary,
    Custom(String),
}

/// Validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: RuleType,
    pub message: String,
}

/// Rule types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Range(f64, f64),
    Custom(String),
}

/// Pipeline input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInput {
    pub data: HashMap<String, Value>,
    pub context: ExecutionContext,
}

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
    pub data: HashMap<String, Value>,
    pub task_id: TaskID,
    pub success: bool,
    pub error: Option<String>,
}

/// Execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub user_id: u64,
    pub device_id: u64,
    pub workspace_id: Option<u64>,
    pub project_id: Option<u64>,
    pub task_context_id: Option<u64>,
    pub metadata: HashMap<String, String>,
}

/// Pipeline blueprint for library storage (§10.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineBlueprint {
    pub pipeline_id: PipelineID,
    pub name: String,
    pub version: SemVer,
    pub author: PublicKey,
    pub description: String,
    pub specification: BlueprintSpec,
    pub implementations: Vec<Implementation>,
    pub content_hash: Blake3Hash,
    pub peers: Vec<PeerNode>,
    pub consensus_status: ConsensusStatus,
    pub verified_by: u32,
}

/// Blueprint specification (language-agnostic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintSpec {
    pub input_schema: Schema,
    pub output_schema: Schema,
    pub dependencies: Vec<PipelineID>,
    pub sub_pipelines: Vec<PipelineID>,
    pub execution_flow: ExecutionFlow,
}

/// Implementation for a specific language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub language: Language,
    pub runtime_requirements: Vec<Dependency>,
    pub code_location: CodePointer,
    pub executable: bool,
}

/// Code pointer for distributed storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePointer {
    pub hash: Blake3Hash,
    pub size: u64,
    pub chunks: Vec<ChunkID>,
    pub mirrors: Vec<PeerNode>,
}

pub type ChunkID = Blake3Hash;

/// Peer node for distributed network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerNode {
    pub peer_id: PublicKey,
    pub address: String,
    pub last_seen: u64,
}

/// Consensus status for distributed verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusStatus {
    Open,
    Verifying,
    Accepted,
    Rejected,
    Expired,
}

/// Execution flow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionFlow {
    Sequential(Vec<PipelineID>),
    Parallel(Vec<PipelineID>),
    Conditional {
        condition: String,
        branches: HashMap<String, PipelineID>,
    },
    DAG(ExecutionGraph),
}

/// Execution graph for complex flows
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionGraph {
    pub nodes: Vec<ExecutionNode>,
    pub edges: Vec<ExecutionEdge>,
}

/// Execution node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionNode {
    pub pipeline_id: PipelineID,
    pub inputs_required: Vec<String>,
    pub outputs_provided: Vec<String>,
}

/// Execution edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEdge {
    pub from_node: PipelineID,
    pub to_node: PipelineID,
    pub data_mapping: Vec<(String, String)>,
}

/// Pipeline container for composition (§10.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContainer {
    pub container_id: ContainerID,
    pub contained_pipelines: Vec<PipelineID>,
    pub execution_order: ExecutionOrder,
    pub data_flow: PipelineDataFlowGraph,
}

/// Execution order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOrder {
    Sequential,
    Parallel,
    Conditional {
        condition: String,
        branches: HashMap<String, PipelineID>,
    },
    DAG(ExecutionGraph),
}

/// Pipeline data flow graph
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineDataFlowGraph {
    pub edges: Vec<PipelineDataEdge>,
}

/// Pipeline data edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDataEdge {
    pub from: (PipelineID, String),
    pub to: (PipelineID, String),
    pub transform: Option<String>,
}

/// Built-in pipeline identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u64)]
pub enum BuiltinPipeline {
    // Core System Pipelines (1-38)
    Auth = 1,
    ThemeLoader = 2,
    ZSEIQuery = 3,
    ZSEIWrite = 4,
    TaskManager = 5,
    WorkspaceTab = 6,
    LibraryTab = 7,
    SettingsTab = 8,
    Prompt = 9,
    Voice = 10,
    MethodologyFetch = 11,
    MethodologyCreate = 12,
    BlueprintSearch = 13,
    BlueprintCreate = 14,
    PipelineCreation = 15,
    ZeroShotSimulation = 16,
    TraversalML = 17,
    CodeAnalysis = 18,
    PackageContext = 19,
    TextAnalysis = 20,
    ContextAggregation = 21,
    GraphVisualization = 22,
    TaskRecommendation = 23,
    Reordering = 24,
    BrowserNavigation = 25,
    IntegrityCheck = 26,
    Consensus = 27,
    ExternalReference = 28,
    PackageRelationship = 29,
    FileLink = 30,
    URLLink = 31,
    PackageLink = 32,
    Sync = 33,
    DeviceRegister = 34,
    HomeReturn = 35,
    TaskViewer = 36,
    LogViewer = 37,
    DeviceStatus = 38,
    
    // Consciousness Pipelines (39-54) - only with feature
    #[cfg(feature = "consciousness")]
    ConsciousnessDecisionGate = 39,
    #[cfg(feature = "consciousness")]
    ExperienceCategorization = 40,
    #[cfg(feature = "consciousness")]
    CoreMemoryFormation = 41,
    #[cfg(feature = "consciousness")]
    ExperienceRetrieval = 42,
    #[cfg(feature = "consciousness")]
    EmotionalBaselineUpdate = 43,
    #[cfg(feature = "consciousness")]
    ILoop = 44,
    #[cfg(feature = "consciousness")]
    InternalLanguage = 45,
    #[cfg(feature = "consciousness")]
    NarrativeConstruction = 46,
    #[cfg(feature = "consciousness")]
    RelationshipDevelopment = 47,
    #[cfg(feature = "consciousness")]
    EthicalAssessment = 48,
    #[cfg(feature = "consciousness")]
    EthicalSimulation = 49,
    #[cfg(feature = "consciousness")]
    PlaybackReview = 50,
    #[cfg(feature = "consciousness")]
    UserFeedback = 51,
    #[cfg(feature = "consciousness")]
    CollectiveConsciousness = 52,
    #[cfg(feature = "consciousness")]
    VoiceIdentity = 53,
    #[cfg(feature = "consciousness")]
    MetaPortionConsciousness = 54,
}

impl BuiltinPipeline {
    pub fn id(&self) -> PipelineID {
        *self as PipelineID
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::Auth => "AuthPipeline",
            Self::ThemeLoader => "ThemeLoaderPipeline",
            Self::ZSEIQuery => "ZSEIQueryPipeline",
            Self::ZSEIWrite => "ZSEIWritePipeline",
            Self::TaskManager => "TaskManagerPipeline",
            Self::WorkspaceTab => "WorkspaceTabPipeline",
            Self::LibraryTab => "LibraryTabPipeline",
            Self::SettingsTab => "SettingsTabPipeline",
            Self::Prompt => "PromptPipeline",
            Self::Voice => "VoicePipeline",
            Self::MethodologyFetch => "MethodologyFetchPipeline",
            Self::MethodologyCreate => "MethodologyCreatePipeline",
            Self::BlueprintSearch => "BlueprintSearchPipeline",
            Self::BlueprintCreate => "BlueprintCreatePipeline",
            Self::PipelineCreation => "PipelineCreationPipeline",
            Self::ZeroShotSimulation => "ZeroShotSimulationPipeline",
            Self::TraversalML => "TraversalMLPipeline",
            Self::CodeAnalysis => "CodeAnalysisPipeline",
            Self::PackageContext => "PackageContextPipeline",
            Self::TextAnalysis => "TextAnalysisPipeline",
            Self::ContextAggregation => "ContextAggregationPipeline",
            Self::GraphVisualization => "GraphVisualizationPipeline",
            Self::TaskRecommendation => "TaskRecommendationPipeline",
            Self::Reordering => "ReorderingPipeline",
            Self::BrowserNavigation => "BrowserNavigationPipeline",
            Self::IntegrityCheck => "IntegrityCheckPipeline",
            Self::Consensus => "ConsensusPipeline",
            Self::ExternalReference => "ExternalReferencePipeline",
            Self::PackageRelationship => "PackageRelationshipPipeline",
            Self::FileLink => "FileLinkPipeline",
            Self::URLLink => "URLLinkPipeline",
            Self::PackageLink => "PackageLinkPipeline",
            Self::Sync => "SyncPipeline",
            Self::DeviceRegister => "DeviceRegisterPipeline",
            Self::HomeReturn => "HomeReturnPipeline",
            Self::TaskViewer => "TaskViewerPipeline",
            Self::LogViewer => "LogViewerPipeline",
            Self::DeviceStatus => "DeviceStatusPipeline",
            #[cfg(feature = "consciousness")]
            Self::ConsciousnessDecisionGate => "ConsciousnessDecisionGatePipeline",
            #[cfg(feature = "consciousness")]
            Self::ExperienceCategorization => "ExperienceCategorizationPipeline",
            #[cfg(feature = "consciousness")]
            Self::CoreMemoryFormation => "CoreMemoryFormationPipeline",
            #[cfg(feature = "consciousness")]
            Self::ExperienceRetrieval => "ExperienceRetrievalPipeline",
            #[cfg(feature = "consciousness")]
            Self::EmotionalBaselineUpdate => "EmotionalBaselineUpdatePipeline",
            #[cfg(feature = "consciousness")]
            Self::ILoop => "ILoopPipeline",
            #[cfg(feature = "consciousness")]
            Self::InternalLanguage => "InternalLanguagePipeline",
            #[cfg(feature = "consciousness")]
            Self::NarrativeConstruction => "NarrativeConstructionPipeline",
            #[cfg(feature = "consciousness")]
            Self::RelationshipDevelopment => "RelationshipDevelopmentPipeline",
            #[cfg(feature = "consciousness")]
            Self::EthicalAssessment => "EthicalAssessmentPipeline",
            #[cfg(feature = "consciousness")]
            Self::EthicalSimulation => "EthicalSimulationPipeline",
            #[cfg(feature = "consciousness")]
            Self::PlaybackReview => "PlaybackReviewPipeline",
            #[cfg(feature = "consciousness")]
            Self::UserFeedback => "UserFeedbackPipeline",
            #[cfg(feature = "consciousness")]
            Self::CollectiveConsciousness => "CollectiveConsciousnessPipeline",
            #[cfg(feature = "consciousness")]
            Self::VoiceIdentity => "VoiceIdentityPipeline",
            #[cfg(feature = "consciousness")]
            Self::MetaPortionConsciousness => "MetaPortionConsciousnessPipeline",
        }
    }
}
