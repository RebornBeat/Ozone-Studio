//! Prompt Orchestrator - v0.4.0
//!
//! Orchestrates the full 11-stage flow from user input to response delivery.
//! This is the CENTRAL COORDINATOR that ties all pipelines together.
//!
//! STAGE 1:  Input Capture (from workspace_tab or meta_portion)
//! STAGE 2:  Text/Prompt Normalization + AMT (text modality + zero-shot)
//! STAGE 3:  Blueprint Assignment (100% match or create new)
//! STAGE 4:  Zero-Shot Simulation (with AMT traversal)
//! STAGE 5:  Consciousness Decision Gate (if enabled)
//! STAGE 6:  Context Aggregation PER STEP (context_aggregation)
//! STAGE 7:  Task Creation (task_manager)
//! STAGE 8:  Execution per blueprint step (with loops, sub-steps, dependencies)
//! STAGE 9:  Result Collection
//! STAGE 10: Post-execution consciousness (experience_memory)
//! STAGE 11: Response Delivery
//!
//! CRITICAL: This orchestrator respects I-Loop protection.
//! Tasks MUST wait for I-Loop to complete before starting.
//!
//! KEY FEATURES:
//! - Layer-by-layer AMT building from chunks (processes each chunk individually)
//! - 5 consecutive Valid validations required
//! - Blueprint step execution with loop/sub-step/dependency support
//! - Direct ZSEI access (no deprecated pipeline wrappers)
//! - Pipeline awareness for blueprint creation
//! - Coverage aspects derived from methodologies (not hardcoded)
//! - Queue-based task execution via TaskManager

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

// Import task module
use crate::task::{TaskManager, TaskPriority};

pub mod adapters;
mod amt;
mod response;
mod voice;
mod stages;
mod graphs;

// K-ALGORITHM contracts come from the unified shared module (single
// compilation, tests run once): crate::shared_contracts::{k_validation, k_loops}.
use crate::shared_contracts::k_validation;
pub use adapters::{RegistryExecutorAdapter, ZseiStoreAdapter};

// ============================================================================
// Types
// ============================================================================

/// Text-processing pathway preference (host-side; forwarded to the text
/// pipeline, which owns the final routing). Auto resolves per model context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProcessingPathPref {
    #[default]
    Auto,
    Path1,
    Path2,
    OmexNative,
}

/// Executor model class (host-side mirror of the pipeline's enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutorModelKind {
    #[default]
    Auto,
    Llm,
    Slm,
    Omex,
}

fn path_str(p: ProcessingPathPref) -> &'static str {
    match p {
        ProcessingPathPref::Path1 => "Path1",
        ProcessingPathPref::Path2 => "Path2",
        ProcessingPathPref::OmexNative => "OmexNative",
        ProcessingPathPref::Auto => "Auto",
    }
}

fn executor_model_str(e: ExecutorModelKind) -> &'static str {
    match e {
        ExecutorModelKind::Auto => "Auto",
        ExecutorModelKind::Llm => "Llm",
        ExecutorModelKind::Slm => "Slm",
        ExecutorModelKind::Omex => "Omex",
    }
}

/// Orchestration request from UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRequest {
    /// User's prompt input
    pub prompt: String,
    /// Optional project context (scoped chat)
    pub project_id: Option<u64>,
    /// Optional workspace context
    pub workspace_id: Option<u64>,
    /// User ID for consciousness tracking
    pub user_id: u64,
    /// Device ID
    pub device_id: u64,
    /// Whether consciousness is enabled
    pub consciousness_enabled: bool,
    /// Token budget for context (overrides model default if set)
    pub token_budget: Option<u32>,
    /// Model configuration override
    pub model_config: Option<ModelConfigOverride>,
    /// Files attached to this prompt (paths or inline content)
    #[serde(default)]
    pub attached_files: Vec<AttachedFileSpec>,
    /// Text-processing pathway preference (host concept, forwarded to the
    /// text modality pipeline). Auto resolves per model context.
    #[serde(default)]
    pub processing_path: ProcessingPathPref,
    /// Executor model class for structural work.
    #[serde(default)]
    pub executor_model: ExecutorModelKind,
    /// Voice-originated input for HEADLESS/gRPC callers — transcribed via
    /// the Voice pipeline (#10, local Whisper preferred per config::VoiceConfig)
    /// before orchestration. The desktop UI transcribes upstream
    /// (MetaPortion → ProcessAudio → text prompt) and does NOT attach this,
    /// so there is no double transcription in the UI flow.
    #[serde(default)]
    pub voice_input: Option<VoiceInputSpec>,
}

/// Voice input attached to a request by non-UI callers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceInputSpec {
    #[serde(default)]
    pub audio_path: Option<String>,
    #[serde(default)]
    pub audio_base64: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub use_api: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigOverride {
    pub model_type: Option<String>,
    pub model_identifier: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub context_length: Option<u32>,
}

/// Orchestration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResponse {
    pub success: bool,
    pub response: Option<String>,
    pub task_id: Option<u64>,
    pub blueprint_id: Option<u64>,
    pub stages_completed: Vec<StageResult>,
    pub consciousness_gate: Option<GateResult>,
    pub error: Option<String>,
    pub total_tokens_used: Option<u32>,
    pub execution_time_ms: u64,
    /// Methodologies used during this request
    pub methodologies_used: Vec<u64>,
    /// Categories created during this request
    pub categories_created: u32,
    /// Blueprints created during this request
    pub blueprints_created: u32,
    /// Clarification points requiring user input
    pub clarification_points: Vec<String>,
    /// Whether clarification is needed before proceeding
    pub needs_clarification: bool,
    /// AMT structure (for debugging/visualization)
    pub amt_summary: Option<AMTSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: u8,
    pub name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub output_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub decision: String,
    /// Captured when the gate pipeline reports it; None = not reported
    /// (never fabricated).
    pub confidence: Option<f32>,
    pub reasoning: String,
}

/// A file attached to the prompt for processing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttachedFileSpec {
    pub file_path: String,
    pub mime_type: Option<String>,
    pub is_inline: bool, // true if content provided inline, false if path only
    pub content_preview: Option<String>, // first 512 bytes if text-readable
}

/// Role of a file graph relative to the prompt intent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FileGraphRole {
    Primary,       // the thing being worked on
    Supplementary, // provides context/reference
    #[default]
    RawData, // data to be processed
    Unknown,
}

/// Classification result for a file's graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedFileGraph {
    pub file_path: String,
    pub graph_id: u64,
    pub modality: String,
    pub role: FileGraphRole,
    pub reasoning: String,
}

/// Evidence that a modality was detected in a specific chunk span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityEvidence {
    pub chunk_index: u32,
    pub span_start: usize,
    pub span_end: usize,
    pub intent_reference: String,
}

/// A modality with a verified pipeline handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedModality {
    pub modality: String,
    pub pipeline_id: u64,
    pub evidence: Vec<ModalityEvidence>,
}

/// A modality detected in chunks but with no registered handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnhandledModality {
    pub modality: String,
    pub evidence: Vec<ModalityEvidence>,
}

/// Aggregated root modality list produced after all chunks are processed.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RootModalityList {
    pub verified_modalities: Vec<VerifiedModality>,
    pub unhandled_modalities: Vec<UnhandledModality>,
    pub total_chunk_count: u32,
}

/// State of a modality graph through the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ModalityGraphState {
    #[default]
    Created,
    SemanticEnriched,
    CrossLinked,
    Stable,
    Updated,
    Failed,
}

/// Tracks a modality graph's state within this orchestration session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionGraphState {
    pub graph_id: u64,
    pub modality: String,
    pub pipeline_id: u64,
    pub state: ModalityGraphState,
    pub cross_modal_edge_count: usize,
    pub consecutive_stable_passes: u32,
}

/// Structured evidence assembled from all available graphs for one AMT layer pass.
/// Produced by `gather_layer_input()`, consumed by `build_amt_layer_by_layer()`.
#[derive(Debug, Clone, Default)]
pub struct LayerInput {
    /// Union of all keywords across chunk graphs
    pub keywords: Vec<String>,
    /// Union of all topics across chunk graphs
    pub topics: Vec<String>,
    /// Grammar relationships from all chunks — structured edge evidence
    pub grammar_evidence: Vec<GrammarEvidence>,
    /// Modality span evidence grouped by modality name
    pub modality_span_evidence: HashMap<String, Vec<ModalitySpanEvidence>>,
    /// File context from classified file graphs
    pub file_contexts: Vec<FileLayerContext>,
    /// Modality graph state contexts
    pub graph_contexts: Vec<GraphLayerContext>,
    /// Verified modality names from root_modality_list
    pub verified_modalities: Vec<String>,
    /// Original cleaned prompt text
    pub cleaned_prompt: String,
    /// Number of processed chunks
    pub chunk_count: u32,
    /// Ordered chunk graph IDs for reconstruction
    pub chunk_graph_ids: Vec<u64>,
}

/// Single grammar relationship extracted from a chunk.
#[derive(Debug, Clone)]
pub struct GrammarEvidence {
    pub from_text: String,
    pub to_text: String,
    pub edge_type: String,
    pub chunk_index: u32,
    pub tense: Option<String>,
    pub negated: bool,
}

/// A detected modality span within a chunk.
#[derive(Debug, Clone)]
pub struct ModalitySpanEvidence {
    pub chunk_index: u32,
    pub span_start: usize,
    pub span_end: usize,
    pub intent_reference: String,
}

/// File context contribution to a layer pass.
#[derive(Debug, Clone)]
pub struct FileLayerContext {
    pub file_path: String,
    pub modality: String,
    pub role: String, // "Primary" | "Supplementary" | "RawData"
    pub graph_id: u64,
}

/// Modality graph state at the time of a layer pass.
#[derive(Debug, Clone)]
pub struct GraphLayerContext {
    pub modality: String,
    pub graph_id: u64,
    pub state: String,
    pub cross_modal_edge_count: usize,
    pub pipeline_id: u64,
}

/// Result of ZSEI knowledge enrichment for a layer pass.
#[derive(Debug, Clone, Default)]
pub struct LayerKnowledge {
    /// Methodology IDs newly discovered for this layer
    pub new_methodology_ids: Vec<u64>,
    /// Related blueprint IDs found via keyword search
    pub related_blueprint_ids: Vec<u64>,
    /// Enrichment summaries from methodologies (for prompt injection)
    pub methodology_summaries: Vec<String>,
}

/// Multi-modal synthesis result — cross-modality patterns found in evidence.
#[derive(Debug, Clone, Default)]
pub struct ModalSynthesis {
    /// Modality pairs that share evidence (text references code, text references image, etc.)
    pub cross_modal_pairs: Vec<(String, String, String)>, // (modality_a, modality_b, relationship)
    /// A short narrative summary of what's across modalities — injected into AMT prompts
    pub cross_modal_summary: String,
    /// Modalities that have strong evidence (node_count > 0 or span_evidence.len() > 3)
    pub active_modalities: Vec<String>,
}

/// Quality score for a discovered AMT branch.
#[derive(Debug, Clone)]
pub struct BranchQuality {
    pub branch: String,
    pub evidence_score: f32,    // 0.0–1.0 based on chunk evidence
    pub methodology_score: f32, // 0.0–1.0 based on methodology support
    pub modal_coverage: f32,    // 0.0–1.0 fraction of active modalities it touches
    pub total_score: f32,       // weighted sum
    pub should_prune: bool,     // true if total_score < 0.2
}

/// Orchestrator-level step lifecycle. Distinct from types::task::StepStatus.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum OrchestratorStepStatus {
    #[default]
    Planned,
    WaitingOnDependency,
    Active,
    StageComplete(u32), // completed stage N, more stages remain
    WaitingPostReview,  // stage done, waiting for hook review to finish
    Complete,
    Failed,
    Rollback,
    Cancelled,
}

/// Per-step tracking state maintained by the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchestratorStepState {
    pub step_index: u32,
    pub pipeline_id: u64,
    pub pipeline_name: String,
    pub status: OrchestratorStepStatus,
    pub stages_completed: Vec<String>,
    pub stages_pending: Vec<String>,
    pub current_stage: Option<String>,
    pub graph_ids_read: Vec<String>,    // modality names read
    pub graph_ids_updated: Vec<String>, // modality names updated
    pub waiting_on_step_indices: Vec<u32>,
    pub version: u32,
    pub tokens_used: u32,
}

/// Result of a methodology cross-reference check for an AMT layer.
#[derive(Debug, Clone)]
pub enum MethodologyFinding {
    /// An existing methodology was added to the active set for this layer.
    Loaded(u64),
    /// A new methodology was synthesized and stored because none existed.
    Created(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMTSummary {
    pub total_nodes: usize,
    pub branch_count: usize,
    pub max_depth: usize,
    pub validation_status: String,
}

// ============================================================================
// Pipeline Registry Types
// ============================================================================

/// Pipeline info from index.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInfo {
    pub pipeline_id: u64,
    pub name: String,
    pub folder_name: String,
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub modality: Option<String>,
    #[serde(default)]
    pub has_ui: bool,
    #[serde(default)]
    pub is_tab: bool,
    #[serde(default)]
    pub deprecated: bool,
}

/// Pipeline index from ZSEI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineIndex {
    pub version: u32,
    pub pipeline_count: u32,
    pub pipelines: Vec<PipelineInfo>,
    #[serde(default)]
    pub categories: HashMap<String, Vec<u64>>,
    #[serde(default)]
    pub next_custom_id: u64,
}

// ============================================================================
// AMT Types - Abstract Meaning Tree
// ============================================================================

/// AMT Node with chunk reference and methodology links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMTNode {
    pub id: u64,
    pub node_type: AMTNodeType,
    pub content: String,
    pub source_chunk_indices: Vec<u32>,
    pub children: Vec<AMTNode>,
    pub relationships: Vec<AMTRelation>,
    pub methodology_ids: Vec<u64>,
    pub metadata: HashMap<String, String>,
    pub depth: u32,
    /// Verification status — "either confident or not". True ONLY when the
    /// node is backed by source provenance (chunk evidence from validated
    /// graph extraction). Never a fabricated score.
    #[serde(default)]
    pub verified: bool,
    /// Wire-compat mirror of `verified` (1.0 / 0.0) — derived, not claimed.
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AMTNodeType {
    Root,           // Primary intent/goal
    Branch,         // Major sub-component/requirement
    Leaf,           // Specific detail/constraint
    Consideration,  // Security, edge case, dependency
    CrossReference, // Link to related branch
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMTRelation {
    pub target_id: u64,
    pub relation_type: AMTRelationType,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AMTRelationType {
    DependsOn,
    Requires,
    RelatesTo,
    Contradicts,
    Elaborates,
    SharedContext,
}

impl AMTNode {
    fn new(id: u64, node_type: AMTNodeType, content: String, depth: u32) -> Self {
        Self {
            id,
            node_type,
            content,
            source_chunk_indices: Vec::new(),
            children: Vec::new(),
            relationships: Vec::new(),
            methodology_ids: Vec::new(),
            metadata: HashMap::new(),
            depth,
            verified: false,
            confidence: 0.0, // unverified until source provenance is attached
        }
    }

    fn count_nodes(&self) -> usize {
        1 + self.children.iter().map(|c| c.count_nodes()).sum::<usize>()
    }

    fn max_depth(&self) -> usize {
        if self.children.is_empty() {
            self.depth as usize
        } else {
            self.children
                .iter()
                .map(|c| c.max_depth())
                .max()
                .unwrap_or(self.depth as usize)
        }
    }

    fn branch_count(&self) -> usize {
        let own_branches = if self.node_type == AMTNodeType::Branch {
            1
        } else {
            0
        };
        own_branches
            + self
                .children
                .iter()
                .map(|c| c.branch_count())
                .sum::<usize>()
    }
}

// ============================================================================
// Chunk Types (from text modality)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawChunk {
    pub index: u32,
    pub text: String,
    pub start_char: u32,
    pub end_char: u32,
    pub token_count: u32,
    pub is_complete_paragraph: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedChunk {
    pub index: u32,
    pub original_text: String,
    pub cleaned_text: String,
    pub start_offset: u32,
    pub end_offset: u32,
    pub token_count: u32,
    pub keywords: Vec<String>,
    pub entities: Vec<ExtractedEntity>,
    pub topics: Vec<String>,
    pub overlap_from_previous: u32,
    pub overlap_to_next: u32,
    /// Validated sentence nodes (grammar relationships ride on the sentence
    /// node itself — grammar is local to each sentence).
    #[serde(default)]
    pub sentence_nodes: Vec<OrchSentenceNode>,
    /// Pairwise cross-sentence relationships (global sentence ids).
    #[serde(default)]
    pub cross_sentence_relationships: Vec<OrchCrossSentenceRelationship>,
    /// Coreference chains (stored on the chunk owning the first mention).
    #[serde(default)]
    pub coreference_chains: Vec<OrchCoreferenceChain>,
    /// Typed modality detections tied to sentence/paragraph parents.
    #[serde(default)]
    pub detected_modalities: Vec<OrchModalityDetection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub text: String,
    pub entity_type: String,
    pub confidence: f32,
}

// ============================================================================
// TEXT GRAPH MIRROR TYPES (typed mirrors of the text pipeline's emitted JSON)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchGrammarRelationship {
    #[serde(default)]
    pub from_text: String,
    #[serde(default)]
    pub to_text: String,
    #[serde(default)]
    pub edge_type: String,
    #[serde(default)]
    pub tense: Option<String>,
    #[serde(default)]
    pub negated: bool,
    #[serde(default)]
    pub verb: String,
    #[serde(default)]
    pub verb_type: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub source_sentence_start: Option<usize>,
    #[serde(default)]
    pub source_sentence_end: Option<usize>,
    #[serde(default)]
    pub chunk_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchKnowledgeRef {
    #[serde(default)]
    pub span_start: usize,
    #[serde(default)]
    pub span_end: usize,
    #[serde(default)]
    pub surface: String,
    #[serde(default)]
    pub knowledge_kind: String,
    #[serde(default)]
    pub topic_path: Vec<String>,
    #[serde(default)]
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchModalityDetection {
    #[serde(default)]
    pub modality: String,
    #[serde(default)]
    pub span_start: usize,
    #[serde(default)]
    pub span_end: usize,
    #[serde(default)]
    pub intent_reference: String,
    #[serde(default)]
    pub chunk_index: u32,
    #[serde(default)]
    pub parent_node_id: Option<u64>,
    #[serde(default)]
    pub open_at_chunk_end: bool,
    #[serde(default)]
    pub needs_reparent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchSentenceNode {
    #[serde(default)]
    pub node_id: u64,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub original_content: String,
    #[serde(default)]
    pub chunk_id: u32,
    #[serde(default)]
    pub chunk_offset: usize,
    #[serde(default)]
    pub paragraph_id: Option<u64>,
    #[serde(default)]
    pub section_id: Option<u64>,
    #[serde(default)]
    pub grammar_relationships: Vec<OrchGrammarRelationship>,
    #[serde(default)]
    pub knowledge_refs: Vec<OrchKnowledgeRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchCrossSentenceRelationship {
    #[serde(default)]
    pub from_sentence_id: u64,
    #[serde(default)]
    pub to_sentence_id: u64,
    #[serde(default)]
    pub relationship_type: String,
    #[serde(default)]
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchCoreferenceMention {
    #[serde(default)]
    pub sentence_id: u64,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub grammar_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchCoreferenceChain {
    #[serde(default)]
    pub chain_id: u64,
    #[serde(default)]
    pub canonical_form: String,
    #[serde(default)]
    pub mentions: Vec<OrchCoreferenceMention>,
}

/// Which AMT build strategy ran. GraphTraversal is the graph-native path
/// (Stage 1 pools → Stage 2 boundaries → Stage 3 promotion) used when the
/// chunks carry sentence nodes with grammar relationships (Path 2 and OMEX
/// native outputs). ChunkZeroShot is the legacy per-chunk prompting loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AmtBuildMode {
    #[default]
    ChunkZeroShot,
    GraphTraversal,
}

// ============================================================================
// Blueprint Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintStep {
    pub step_index: u32,
    pub action: String,
    pub description: String,
    pub pipeline_id: u64,
    pub context_requirements: Vec<String>,
    /// Loop configuration
    pub loop_config: Option<LoopConfig>,
    /// Sub-steps within this step
    pub sub_steps: Vec<BlueprintSubStep>,
    /// IDs of steps this depends on
    pub depends_on: Vec<u32>,
    /// Whether to wait for graph update before proceeding
    pub wait_for_graph_update: bool,
    /// Maximum retries on failure
    pub max_retries: u32,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    /// Loop type
    pub loop_type: LoopType,
    /// Maximum iterations (safety limit)
    pub max_iterations: u32,
    /// Condition for continuing (evaluated each iteration)
    pub continue_condition: String,
    /// Variable to iterate over (for ForEach)
    pub iterate_over: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoopType {
    /// Loop while condition is true
    While,
    /// Loop until condition is true
    Until,
    /// Loop for each item in a collection
    ForEach,
    /// Fixed number of iterations
    Count,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintSubStep {
    pub sub_index: u32,
    pub action: String,
    pub pipeline_id: u64,
    pub input_mapping: HashMap<String, String>,
    pub output_mapping: HashMap<String, String>,
}

// ============================================================================
// Model Context Limits
// ============================================================================

fn get_model_context_limit(model_identifier: &str) -> u32 {
    match model_identifier {
        // Claude models
        s if s.contains("claude-3-opus") => 200000,
        s if s.contains("claude-3-sonnet") => 200000,
        s if s.contains("claude-3-haiku") => 200000,
        s if s.contains("claude-sonnet-4") => 200000,
        s if s.contains("claude-haiku-4") => 200000,
        s if s.contains("claude-opus-4") => 200000,
        // GPT models
        s if s.contains("gpt-4-turbo") => 128000,
        s if s.contains("gpt-4o") => 128000,
        s if s.contains("gpt-4") => 8192,
        s if s.contains("gpt-3.5") => 16385,
        // Local models
        s if s.contains("llama-3") => 8192,
        s if s.contains("llama-2") => 4096,
        s if s.contains("mistral") => 32768,
        s if s.contains("mixtral") => 32768,
        // BitNet models (smaller context)
        s if s.contains("bitnet") => 4096,
        // Default
        _ => 100000,
    }
}

// ============================================================================
// Internal State
// ============================================================================

/// Tracks a discovered intent with provenance
#[derive(Debug, Clone)]
struct IntentCapture {
    intent: String,
    is_parallel: bool, // true if this is an unrelated parallel intent
    source_chunk_indices: Vec<u32>,
    source_sentences: Vec<String>, // exact sentences/paragraphs from chunks
    node_id: u64,                  // assigned when AMT node is created
}

/// Tracks a discovered branch with methodology provenance
#[derive(Debug, Clone)]
#[allow(dead_code)] // parent_intent captured for movement-graph/Hearth consumers
struct BranchCapture {
    branch: String,
    parent_intent: String,
    source_methodology_ids: Vec<u64>, // which methodologies suggested this branch
    source_chunk_indices: Vec<u32>,   // chunks that mention this branch
    source_sentences: Vec<String>,
    node_id: u64,
}

/// Tracks a discovered detail/sub-task
#[derive(Debug, Clone)]
struct DetailCapture {
    content: String,
    detail_type: String, // "detail", "requirement", "constraint"
    parent_branch: String,
    parent_intent: String,
    source_chunk_indices: Vec<u32>,
    source_sentences: Vec<String>,
    node_id: u64,
}

/// Tracks cross-references between branches
#[derive(Debug, Clone)]
#[allow(dead_code)] // intent fields + description ride on the wire later
struct CrossRef {
    from_branch: String,
    to_branch: String,
    from_intent: String,
    to_intent: String,
    relation_type: AMTRelationType,
    description: String,
}

#[allow(dead_code)] // several fields are captured for consumers not yet wired
pub(crate) struct OrchestrationState {
    request: OrchestrationRequest,
    start_time: std::time::Instant,
    stages: Vec<StageResult>,

    // Model context management
    model_context_limit: u32,
    tokens_used_so_far: u32,

    // Stage 2 outputs
    raw_chunks: Vec<RawChunk>,

    // PHASE 1: FILE GRAPHS (produced before modality graphs, before classification)
    pub file_graphs: HashMap<String, u64>, // file_path → graph_id
    pub classified_file_graphs: Vec<ClassifiedFileGraph>, // primary/supplementary/raw roles
    pub chunk_graph_ids: Vec<u64>,         // ordered list of chunk graph IDs

    // PHASE 2: INITIAL MODALITY GRAPHS
    pub modality_graphs: HashMap<String, u64>, // modality_name → graph_id
    pub graph_states: HashMap<u64, SessionGraphState>, // graph_id → state
    pub root_modality_list: RootModalityList,  // aggregated from chunks
    pub initial_graphs_created: bool,
    pub cross_modal_index_id: Option<u64>,

    processed_chunks: Vec<ProcessedChunk>,
    cleaned_prompt: String,
    prompt_tokens: u32,
    keywords: Vec<String>,
    entities: Vec<ExtractedEntity>,
    topics: Vec<String>,

    // Methodology/Category tracking
    methodologies: Vec<u64>,
    categories: Vec<u64>,
    categories_created: u32,

    // AMT
    amt: Option<AMTNode>,
    amt_validated: bool,
    validation_streak: u32, // Need 5 consecutive Valid for completion
    /// How the AMT was built this session (telemetry + routing record).
    amt_build_mode: AmtBuildMode,
    needs_clarification: bool,
    clarification_points: Vec<String>,
    intent_captures: Vec<IntentCapture>,
    branch_captures: Vec<BranchCapture>,
    detail_captures: Vec<DetailCapture>,
    cross_refs: Vec<CrossRef>,
    amt_pass_count: u32,
    coverage_aspects: Vec<String>,

    // Blueprint
    blueprint_id: Option<u64>,
    blueprint_steps: Vec<BlueprintStep>,
    pub orch_step_states: HashMap<u32, OrchestratorStepState>, // step_index → state
    blueprints_created: u32,

    // Execution
    task_id: Option<u64>,
    step_results: Vec<StepResult>,
    final_response: Option<String>,
    step_contexts: HashMap<u32, String>,
    step_outputs: HashMap<u32, serde_json::Value>,

    // Consciousness
    gate_result: Option<GateResult>,
    voice_identity: Option<VoiceIdentity>,

    // Pipeline registry (loaded from index.json)
    available_pipelines: Vec<PipelineInfo>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // pipeline_id/iterations/sub_step_results = captured metrics
struct StepResult {
    step_index: u32,
    pipeline_id: u64,
    output: serde_json::Value,
    tokens_used: u32,
    iterations: u32,
    sub_step_results: Vec<SubStepResult>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // captured per-sub-step telemetry (dashboard consumers)
struct SubStepResult {
    sub_index: u32,
    output: serde_json::Value,
    success: bool,
}

/// Voice identity from consciousness self_model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceIdentity {
    pub tone: String,
    pub formality: f32,
    pub warmth: f32,
    pub directness: f32,
    pub humor_level: f32,
    pub vocabulary_style: String,
}

// ============================================================================
// OMEX PIPELINE CONSTANTS (host-side; matched by the text pipeline's consts)
// ============================================================================

/// OMEX GrammarParser — text → grammar graph, single non-autoregressive pass.
pub const OMEX_TEXT_PARSER_PIPELINE_ID: u64 = 900;
/// OMEX KnowledgeLinker — knowledge span + context → ranked knowledge paths.
pub const OMEX_KNOWLEDGE_LINKER_PIPELINE_ID: u64 = 901;
/// OMEX Realizer — response graph → text (reverse grammar traversal).
pub const OMEX_REALIZER_PIPELINE_ID: u64 = 902;

// ============================================================================
// RESPONSE GRAPH — the reasoning → rendering handoff contract
// ============================================================================
// The orchestrator ASSEMBLES this; the renderer only realizes it. Content
// leaves are pre-filled verified material carrying knowledge provenance.
// The Realizer (or any ladder tier) cannot invent what is not here.

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseVoiceSpec {
    #[serde(default)]
    pub tone: String,
    #[serde(default = "default_voice_f")]
    pub formality: f32,
    #[serde(default = "default_voice_f")]
    pub warmth: f32,
    #[serde(default = "default_voice_f")]
    pub directness: f32,
    #[serde(default = "default_voice_f")]
    pub humor: f32,
}

fn default_voice_f() -> f32 {
    0.5
}

/// One sentence of the response at FRAME granularity (Subject/Verb/Object —
/// the renderer owns internal phrase structure and ordering variation).
/// TREE granularity (a fully specified grammar tree) is reserved for
/// exactness needs and carried as raw JSON in the same sentence slot.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseFrame {
    pub subject: String,
    pub verb: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub modifiers: Vec<String>,
    /// tense / polarity / number hints the renderer must respect
    #[serde(default)]
    pub properties: HashMap<String, String>,
    /// knowledge provenance for this content leaf (ZSEI paths)
    #[serde(default)]
    pub knowledge_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseSentenceSpec {
    /// "frame" | "tree"
    pub granularity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<ResponseFrame>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<serde_json::Value>,
    /// Elaborates | Contrast | Cause | Sequence | Example | Summary —
    /// relation to the PREVIOUS sentence; the renderer realizes it as a
    /// connective. The relation is content; the connective word is surface.
    #[serde(default)]
    pub discourse_relation_to_previous: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseGraphSpec {
    pub schema_version: String,
    pub voice: ResponseVoiceSpec,
    pub sentences: Vec<ResponseSentenceSpec>,
}

// ============================================================================
// Pipeline Executor Trait
// ============================================================================

/// Trait for executing pipelines (implemented by runtime)
#[async_trait::async_trait]
pub trait PipelineExecutor: Send + Sync {
    async fn execute(
        &self,
        pipeline_id: u64,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String>;
}

// ============================================================================
// ZSEI Direct Access Trait
// ============================================================================

#[async_trait::async_trait]
/// THE STORE CONTRACT — backend-agnostic by design ("the store of the
/// store"). Records are untyped JSON Values; container semantics are a ZSEI
/// implementation detail, not part of this contract. ZSEI (container store)
/// is selectable backend #1 via `adapters::ZseiStoreAdapter`; any other
/// store implements this trait and swaps in without orchestrator changes.
pub trait StoreAccess: Send + Sync {
    /// Execute a ZSEI query
    async fn query(&self, query: serde_json::Value) -> Result<serde_json::Value, String>;

    /// Perform traversal
    async fn traverse(&self, request: serde_json::Value) -> Result<serde_json::Value, String>;

    /// Create a container
    async fn create_container(
        &self,
        parent_id: u64,
        container: serde_json::Value,
    ) -> Result<u64, String>;

    /// Update a container
    async fn update_container(
        &self,
        container_id: u64,
        updates: serde_json::Value,
    ) -> Result<(), String>;

    /// Get container by ID
    async fn get_container(&self, container_id: u64) -> Result<Option<serde_json::Value>, String>;

    /// Search containers by keywords
    async fn search_by_keywords(
        &self,
        keywords: &[String],
        container_type: Option<&str>,
    ) -> Result<Vec<u64>, String>;

    /// Get all categories
    async fn get_categories(&self, modality: &str) -> Result<Vec<u64>, String>;
}

// ============================================================================
// Orchestrator Implementation
// ============================================================================

pub struct PromptOrchestrator {
    executor: Arc<dyn PipelineExecutor>,
    store: Arc<dyn StoreAccess>,
    task_manager: Arc<RwLock<TaskManager>>,
    pipeline_index: Arc<RwLock<Option<PipelineIndex>>>,
}

impl PromptOrchestrator {
    pub fn new(
        executor: Arc<dyn PipelineExecutor>,
        store: Arc<dyn StoreAccess>,
        task_manager: Arc<RwLock<TaskManager>>,
        pipeline_index: Arc<RwLock<Option<PipelineIndex>>>,
    ) -> Self {
        Self {
            executor,
            store,
            task_manager,
            pipeline_index,
        }
    }

    fn extract_json_from_response<'a>(s: &'a str, open: char, close: char) -> &'a str {
        let trimmed = s.trim();
        if let Some(start) = trimmed.find(open) {
            if let Some(end) = trimmed.rfind(close) {
                if end >= start {
                    return &trimmed[start..=end];
                }
            }
        }
        trimmed
    }

    /// Resolve the effective processing path. Auto rules:
    /// - executor_model == Omex (or model identifier contains "omex") → OmexNative
    /// - context limit <= 8192 → Path2 (constrained SLMs — granular loops)
    /// - otherwise → Path1 (capable LLMs — whole-chunk deconstruction)
    fn resolve_processing_path(
        &self,
        request: &OrchestrationRequest,
        model_identifier: &str,
        context_limit: u32,
    ) -> ProcessingPathPref {
        match (&request.executor_model, &request.processing_path) {
            (ExecutorModelKind::Omex, _) => ProcessingPathPref::OmexNative,
            (_, ProcessingPathPref::OmexNative) => ProcessingPathPref::OmexNative,
            (_, ProcessingPathPref::Path1) => ProcessingPathPref::Path1,
            (_, ProcessingPathPref::Path2) => ProcessingPathPref::Path2,
            (ExecutorModelKind::Slm, ProcessingPathPref::Auto) => ProcessingPathPref::Path2,
            (ExecutorModelKind::Llm, ProcessingPathPref::Auto) => ProcessingPathPref::Path1,
            _ => {
                let id = model_identifier.to_lowercase();
                if id.contains("omex") {
                    ProcessingPathPref::OmexNative
                } else if context_limit <= 8192 {
                    ProcessingPathPref::Path2
                } else {
                    ProcessingPathPref::Path1
                }
            }
        }
    }

    /// Load pipeline index from ZSEI
    pub async fn load_pipeline_index(&self) -> Result<(), String> {
        // Try to load from ZSEI storage
        let index_result = self
            .store
            .query(serde_json::json!({
                "type": "GetPipelineIndex"
            }))
            .await;

        if let Ok(result) = index_result {
            if let Ok(index) = serde_json::from_value::<PipelineIndex>(result) {
                *self.pipeline_index.write().await = Some(index);
            }
        }
        Ok(())
    }

    /// Get available pipelines
    async fn get_available_pipelines(&self) -> Vec<PipelineInfo> {
        if let Some(index) = self.pipeline_index.read().await.as_ref() {
            index
                .pipelines
                .iter()
                .filter(|p| !p.deprecated)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Main entry point - orchestrates the full 11-stage flow
    pub async fn orchestrate(&self, request: OrchestrationRequest) -> OrchestrationResponse {
        let model_identifier = request
            .model_config
            .as_ref()
            .and_then(|c| c.model_identifier.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("claude-sonnet-4");

        let model_context_limit = request
            .model_config
            .as_ref()
            .and_then(|c| c.context_length)
            .unwrap_or_else(|| get_model_context_limit(model_identifier));

        let prompt_tokens = Self::estimate_tokens(&request.prompt);

        // Load pipeline index if not already loaded
        let _ = self.load_pipeline_index().await;
        let available_pipelines = self.get_available_pipelines().await;

        let mut state = OrchestrationState {
            request: request.clone(),
            start_time: std::time::Instant::now(),
            stages: Vec::new(),
            model_context_limit,
            tokens_used_so_far: prompt_tokens,
            raw_chunks: Vec::new(),
            file_graphs: HashMap::new(),
            classified_file_graphs: Vec::new(),
            chunk_graph_ids: Vec::new(),
            modality_graphs: HashMap::new(),
            graph_states: HashMap::new(),
            root_modality_list: RootModalityList::default(),
            initial_graphs_created: false,
            cross_modal_index_id: None,
            processed_chunks: Vec::new(),
            cleaned_prompt: String::new(),
            prompt_tokens,
            keywords: Vec::new(),
            entities: Vec::new(),
            topics: Vec::new(),
            methodologies: Vec::new(),
            categories: Vec::new(),
            categories_created: 0,
            amt: None,
            amt_validated: false,
            validation_streak: 0,
            amt_build_mode: AmtBuildMode::ChunkZeroShot,
            needs_clarification: false,
            clarification_points: Vec::new(),
            intent_captures: Vec::new(),
            branch_captures: Vec::new(),
            detail_captures: Vec::new(),
            cross_refs: Vec::new(),
            amt_pass_count: 0,
            coverage_aspects: Vec::new(),
            blueprint_id: None,
            blueprint_steps: Vec::new(),
            orch_step_states: HashMap::new(),
            blueprints_created: 0,
            task_id: None,
            step_results: Vec::new(),
            final_response: None,
            step_contexts: HashMap::new(),
            step_outputs: HashMap::new(),
            gate_result: None,
            voice_identity: None,
            available_pipelines,
        };

        // ── Voice input (Whisper, headless path) ──
        // Non-UI callers attach audio here; the Voice pipeline (#10)
        // transcribes it (local Whisper preferred per config::VoiceConfig).
        // The desktop UI transcribes upstream and sends text — this path
        // only fires when voice_input is attached, so the flows never
        // double-transcribe. Routed through the executor trait.
        if let Some(voice_input) = state.request.voice_input.clone() {
            let has_audio =
                voice_input.audio_path.is_some() || voice_input.audio_base64.is_some();
            if has_audio {
                let action = if let Some(path) = &voice_input.audio_path {
                    serde_json::json!({
                        "action": "TranscribeFile",
                        "file_path": path,
                        "use_api": voice_input.use_api
                    })
                } else {
                    serde_json::json!({
                        "action": "ProcessAudio",
                        "audio_base64": voice_input.audio_base64.clone().unwrap_or_default(),
                        "format": voice_input.format.clone().unwrap_or_else(|| "wav".to_string())
                    })
                };
                match self.executor.execute(10, action).await {
                    Ok(result) => {
                        let transcript = result
                            .get("transcription")
                            .or_else(|| result.get("transcript"))
                            .or_else(|| result.get("text"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("");
                        if !transcript.trim().is_empty() {
                            if state.request.prompt.trim().is_empty() {
                                state.request.prompt = transcript.to_string();
                            } else {
                                state.request.prompt =
                                    format!("{}\n\n[Voice]: {}", state.request.prompt, transcript);
                            }
                        }
                    }
                    Err(e) => tracing::warn!(
                        "Voice transcription failed ({}); continuing with text prompt",
                        e
                    ),
                }
            }
        }

        // Check I-Loop before starting (if consciousness enabled)
        if request.consciousness_enabled {
            if let Err(e) = self.wait_for_i_loop().await {
                return self.build_error_response(&mut state, format!("I-Loop wait failed: {}", e));
            }
        }

        let result = self.execute_stages(&mut state).await;

        match result {
            Ok(_) => self.build_success_response(&state),
            Err(e) => self.build_error_response(&mut state, e),
        }
    }

    // ========================================================================
    // STAGE 2: Text/Prompt Normalization
    // ========================================================================

    async fn prompt_normalization(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // ── STEP 0: Process attached files → produce file graphs ──
        // Files are processed BEFORE prompt chunking so file modality data
        // can inform the AMT. Analyze only — graph creation + storage here.
        if !state.request.attached_files.is_empty() {
            let available_modalities: Vec<String> = state
                .root_modality_list
                .verified_modalities
                .iter()
                .map(|vm| vm.modality.clone())
                .collect();

            for file_spec in &state.request.attached_files.clone() {
                let modality = self.detect_file_modality(&file_spec.file_path);
                let pipeline_id = self.modality_name_to_pipeline_id(&modality);

                if pipeline_id == 0 {
                    tracing::warn!(
                        "Unknown modality for file: {} — treating as text",
                        file_spec.file_path
                    );
                }

                let effective_pipeline = if pipeline_id == 0 { 100u64 } else { pipeline_id };

                let analysis_result = self
                    .process_modality(
                        file_spec.content_preview.as_deref().unwrap_or(""),
                        effective_pipeline,
                        &available_modalities,
                        None,
                        None,
                        None,
                    )
                    .await?;

                let graph_input = serde_json::json!({
                    "action": {
                        "type": "CreateGraph",
                        "analysis_result": analysis_result.get("analysis").cloned().unwrap_or_default(),
                        "project_id": state.request.project_id.unwrap_or(0),
                        "link_to_existing": false
                    }
                });

                let graph_result = self
                    .executor
                    .execute(effective_pipeline, graph_input)
                    .await
                    .unwrap_or_default();

                let graph_id = graph_result
                    .get("graph_id")
                    .and_then(|g| g.as_u64())
                    .unwrap_or(Self::generate_id_static());

                state
                    .file_graphs
                    .insert(file_spec.file_path.clone(), graph_id);
            }
        }

        // ── STEP 1: Analyze the prompt via the text modality pipeline ──
        // Route resolution: which path (and executor class) serves this
        // request. max_chunk_tokens = 1/4 of the model context limit — the
        // shared-window rule that keeps prompt + structured response inside
        // even 4K-context SLMs.
        let model_identifier = state
            .request
            .model_config
            .as_ref()
            .and_then(|c| c.model_identifier.clone())
            .unwrap_or_else(|| "claude-sonnet-4".to_string());
        let path = self.resolve_processing_path(
            &state.request,
            &model_identifier,
            state.model_context_limit,
        );
        state.amt_build_mode = match path {
            ProcessingPathPref::Path2 | ProcessingPathPref::OmexNative => {
                AmtBuildMode::GraphTraversal
            }
            _ => AmtBuildMode::ChunkZeroShot,
        };
        let executor_model = match state.request.executor_model {
            ExecutorModelKind::Auto => match path {
                ProcessingPathPref::OmexNative => ExecutorModelKind::Omex,
                ProcessingPathPref::Path2 => ExecutorModelKind::Slm,
                _ => ExecutorModelKind::Llm,
            },
            other => other,
        };

        let available_modalities: Vec<String> = state
            .root_modality_list
            .verified_modalities
            .iter()
            .map(|vm| vm.modality.clone())
            .collect();

        let process_result = self
            .executor
            .execute(
                100,
                serde_json::json!({
                    "action": {
                        "type": "Analyze",
                        "text": state.request.prompt.clone(),
                        "max_chunk_tokens": (state.model_context_limit / 4).max(256),
                        "depth": "Standard",
                        "extract_entities": true,
                        "extract_topics": true,
                        "available_modalities": available_modalities,
                        "processing_path": path_str(path),
                        "executor_model": executor_model_str(executor_model),
                    }
                }),
            )
            .await?;

        // Fold the pipeline's LLM token usage into the orchestrator budget
        // (extraction + every 5x validation call is metered downstream).
        if let Some(used) = process_result.get("llm_tokens_used").and_then(|t| t.as_u64()) {
            state.tokens_used_so_far += used as u32;
        }

        // ── STEP 2: Collect processed chunks ──
        let mut all_keywords: HashSet<String> = HashSet::new();
        let mut all_entities: Vec<ExtractedEntity> = Vec::new();
        let mut all_topics: HashSet<String> = HashSet::new();

        if let Some(processed_arr) = process_result
            .get("updated_chunks")
            .or_else(|| process_result.get("processed_chunks"))
            .and_then(|p| p.as_array())
        {
            for processed_val in processed_arr {
                if let Ok(processed) =
                    serde_json::from_value::<ProcessedChunk>(processed_val.clone())
                {
                    for kw in &processed.keywords {
                        all_keywords.insert(kw.clone());
                    }
                    all_entities.extend(processed.entities.clone());
                    for topic in &processed.topics {
                        all_topics.insert(topic.clone());
                    }
                    state.processed_chunks.push(processed);
                }
            }
        }

        // Collect chunk graph ID for cross-model reconstruction capability
        if let Some(cg) = process_result
            .get("chunk_graph")
            .and_then(|cg| cg.get("graph_id"))
            .and_then(|id| id.as_u64())
        {
            state.chunk_graph_ids.push(cg);
        }

        // ── STEP 3: Graph-derived keywords ──
        // When chunks carry grammar relationships (Path 2 / OMEX), keywords
        // derive from the graph: subject/object noun phrases and knowledge
        // reference surfaces — NOT a separate extraction pass.
        for chunk in &state.processed_chunks {
            for sent in &chunk.sentence_nodes {
                for gr in &sent.grammar_relationships {
                    for side in [&gr.subject, &gr.from_text, &gr.to_text] {
                        let t = side.trim().to_lowercase();
                        if t.len() > 2 {
                            all_keywords.insert(t);
                        }
                    }
                }
                for kr in &sent.knowledge_refs {
                    let t = kr.surface.trim().to_lowercase();
                    if t.len() > 2 {
                        all_keywords.insert(t);
                    }
                }
            }
        }

        // ── STEP 4: Reconstruct cleaned prompt from chunks ──
        let reconstruct_input = serde_json::json!({
            "action": {
                "type": "ReconstructFromChunks",
                "chunks": state.processed_chunks.clone()
            }
        });

        let reconstruct_result = self.executor.execute(100, reconstruct_input).await?;

        state.cleaned_prompt = reconstruct_result
            .get("reconstructed_text")
            .and_then(|t| t.as_str())
            .unwrap_or(&state.request.prompt)
            .to_string();

        state.prompt_tokens = Self::estimate_tokens(&state.cleaned_prompt);
        state.tokens_used_so_far += state.prompt_tokens;
        state.keywords = all_keywords.into_iter().collect();
        state.entities = all_entities;
        state.topics = all_topics.into_iter().collect();

        let sentence_node_total: usize = state
            .processed_chunks
            .iter()
            .map(|c| c.sentence_nodes.len())
            .sum();

        self.record_stage_timed(
            state,
            2,
            "Text Normalization",
            true,
            &format!(
                "Chunks: {}, ChunkGraphs: {}, Files: {}, SentenceNodes: {}, Modalities: {}, Path: {:?}, AMT mode: {:?}",
                state.processed_chunks.len(),
                state.chunk_graph_ids.len(),
                state.file_graphs.len(),
                sentence_node_total,
                state
                    .processed_chunks
                    .iter()
                    .map(|c| c.detected_modalities.len())
                    .sum::<usize>(),
                path,
                state.amt_build_mode,
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    // ========================================================================
    // STAGE 3: Gather Methodologies
    // ========================================================================

    async fn gather_methodologies(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Search methodologies by keywords via ZSEI
        let methodology_ids = self
            .store
            .search_by_keywords(
                &state.keywords.iter().take(20).cloned().collect::<Vec<_>>(),
                Some("Methodology"),
            )
            .await
            .unwrap_or_default();

        state.methodologies = methodology_ids;

        // Get categories from methodologies and cross-reference
        let mut methodology_categories: HashSet<u64> = HashSet::new();

        for method_id in &state.methodologies {
            if let Ok(Some(container)) = self.store.get_container(*method_id).await {
                if let Some(cats) = container
                    .get("local_state")
                    .and_then(|ls| ls.get("context"))
                    .and_then(|ctx| ctx.get("categories"))
                    .and_then(|c| c.as_array())
                {
                    for cat in cats {
                        if let Some(cat_id) = cat.as_u64() {
                            methodology_categories.insert(cat_id);
                        }
                    }
                }
            }
        }

        // Get all existing categories
        let mut existing_category_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // Check methodology_categories (already loaded container IDs)
        for &cat_id in &methodology_categories {
            if let Ok(Some(container)) = self.store.get_container(cat_id).await {
                if let Some(name) = container
                    .get("local_state")
                    .and_then(|ls| ls.get("metadata"))
                    .and_then(|m| m.get("name"))
                    .and_then(|n| n.as_str())
                {
                    existing_category_names.insert(name.to_lowercase());
                }
                // Also check keywords as category names may be stored there
                if let Some(kws) = container
                    .get("local_state")
                    .and_then(|ls| ls.get("context"))
                    .and_then(|ctx| ctx.get("keywords"))
                    .and_then(|k| k.as_array())
                {
                    for kw in kws {
                        if let Some(kw_str) = kw.as_str() {
                            existing_category_names.insert(kw_str.to_lowercase());
                        }
                    }
                }
            }
        }

        // Also search ZSEI for existing categories matching each topic
        for topic in &state.topics {
            if topic.is_empty() {
                continue;
            }
            let topic_lower = topic.to_lowercase();

            // Use search_by_keywords to find if a category with this name exists
            let existing_matches = self
                .store
                .search_by_keywords(&[topic.clone()], Some("Category"))
                .await
                .unwrap_or_default();

            let needs_creation =
                !existing_category_names.contains(&topic_lower) && existing_matches.is_empty();

            if needs_creation {
                let new_category = serde_json::json!({
                    "container_type": "Category",
                    "modality": "Text",
                    "metadata": {
                        "name": topic,
                        "description": format!("Auto-created category for topic: {}", topic),
                        "created_by": "orchestrator"
                    },
                    "context": {
                        "keywords": [topic_lower],
                        "topics": []
                    }
                });

                if let Ok(new_id) = self.store.create_container(0, new_category).await {
                    state.categories.push(new_id);
                    state.categories_created += 1;
                    existing_category_names.insert(topic_lower);
                }
            } else if let Some(&first_match) = existing_matches.first() {
                // Add to categories if found but not already tracked
                if !state.categories.contains(&first_match) {
                    state.categories.push(first_match);
                }
            }
        }

        // NOTE: AMT construction and file classification moved OUT of this
        // stage — classification is Stage 4A (execute_stages, logged there
        // with its classified count) and the AMT is Stage 5 (build_amt,
        // which logs Intents/Branches/Details/Cross-refs/Passes/Validated).
        // This stage owns ONLY the methodology set + categories.
        self.record_stage_timed(
            state,
            3,
            "Gather Methodologies",
            true,
            &format!(
                "Methodologies: {}, Categories: {} ({} created)",
                state.methodologies.len(),
                state.categories.len(),
                state.categories_created,
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    // ========================================================================
    // STAGE 4: AMT Build
    // ========================================================================
    // ========================================================================
    // AMT TREE ASSEMBLY (shared by both build modes)
    // ========================================================================

    // ========================================================================
    // METERED EXECUTION (orchestrator side)
    // ========================================================================

    /// Execute an LLM call and fold its token usage into the running budget.
    async fn metered_execute(
        &self,
        state: &mut OrchestrationState,
        pipeline_id: u64,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let result = self.executor.execute(pipeline_id, input).await?;
        if let Some(tokens) = result.get("tokens_used").and_then(|t| t.as_u64()) {
            state.tokens_used_so_far += tokens as u32;
        }
        Ok(result)
    }

    /// Orchestrator-side YES/NO confirmation — delegates to the shared
    /// K-ALGORITHM contract (k_validation); the metered oracle is the only
    /// contextual part. Strength comes from the policy (default 5). Tokens
    /// used during validation are captured in the oracle closure and folded
    /// into the budget afterward — metering stays truthful without &mut
    /// state escaping into the future.
    async fn confirm_yes_no_orch(
        &self,
        state: &mut OrchestrationState,
        prompt: String,
        n: u32,
    ) -> bool {
        use std::sync::atomic::{AtomicU64, Ordering};
        let policy = k_validation::ValidationPolicy { strength: n };
        let tokens_during = Arc::new(AtomicU64::new(0));
        let t = tokens_during.clone();
        let ok = k_validation::confirm_consecutive_yes_with(
            move || {
                let input = k_validation::yes_no_input(&prompt);
                let t = t.clone();
                async move {
                    match self.executor.execute(9, input).await {
                        Ok(result) => {
                            if let Some(tok) =
                                result.get("tokens_used").and_then(|x| x.as_u64())
                            {
                                t.fetch_add(tok, Ordering::Relaxed);
                            }
                            let raw = result
                                .get("response")
                                .and_then(|r| r.as_str())
                                .unwrap_or("{}");
                            k_validation::parse_yes_no(raw)
                        }
                        Err(_) => None,
                    }
                }
            },
            &policy,
        )
        .await;
        state.tokens_used_so_far += tokens_during.load(Ordering::Relaxed) as u32;
        ok
    }

    // ========================================================================
    // E-1: LAYER INPUT GATHERING (typed — grammar rides on SentenceNode)
    // ========================================================================

    // ========================================================================
    // K: ZSEI KNOWLEDGE ENRICHMENT
    // ========================================================================

    // ========================================================================
    // M: MULTI-MODAL SYNTHESIS
    // ========================================================================

    // ========================================================================
    // Q: BRANCH QUALITY SCORING + PRUNING
    // ========================================================================

    // ========================================================================
    // GRAPH-NATIVE AMT (Stage 1 pools → Stage 2 boundaries → Stage 3 promotion)
    // ========================================================================

    // ========================================================================
    // STAGE 3: Blueprint Assignment
    // ========================================================================

    // ========================================================================
    // STAGE 4: Zero-Shot Simulation
    // ========================================================================

    // ========================================================================
    // STAGE 5: Consciousness Decision Gate
    // ========================================================================

    // ========================================================================
    // STAGES 6-8: Context Aggregation + Task Creation + Step Execution
    // ========================================================================

    /// Hook fires on step completion. Detects graph changes, reviews AMT,
    /// synthesizes new steps if AMT expanded. This IS the living system.
    async fn on_step_complete(
        &self,
        state: &mut OrchestrationState,
        step: &BlueprintStep,
        result: &StepResult,
    ) {
        // Detect graph modifications from step output
        let graphs_updated: Vec<String> = result
            .output
            .get("graphs_updated")
            .and_then(|g| g.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if graphs_updated.is_empty() {
            return;
        }

        // Update graph states for modified graphs
        for modality in &graphs_updated {
            if let Some(&gid) = state.modality_graphs.get(modality) {
                if let Some(gs) = state.graph_states.get_mut(&gid) {
                    gs.state = ModalityGraphState::Updated;
                    gs.consecutive_stable_passes = 0;
                }
            }
        }

        // Review AMT alignment — check if new intents emerged from graph updates
        let review_prompt = format!(
            r#"Step {} just completed and updated these modality graphs: {}.
    Current AMT intent: {}
    Current branches: {}

    Did this step's completion reveal any new intents or requirements not already in the AMT?

    Return ONLY valid JSON:
    {{"new_intents": [], "new_branches": [], "amt_needs_expansion": false}}"#,
            step.step_index,
            graphs_updated.join(", "),
            state.amt.as_ref().map(|a| a.content.as_str()).unwrap_or(""),
            state
                .branch_captures
                .iter()
                .map(|b| b.branch.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let input = serde_json::json!({
            "prompt": review_prompt,
            "max_tokens": 300,
            "temperature": 0.2,
            "system_context": "AMT alignment review. Return only valid JSON."
        });

        if let Ok(result) = self.metered_execute(state, 9, input).await {
            let raw = result
                .get("response")
                .and_then(|r| r.as_str())
                .unwrap_or("{}");
            let start = raw.find('{').unwrap_or(0);
            let end = raw.rfind('}').map(|i| i + 1).unwrap_or(raw.len());

            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw[start..end]) {
                let amt_expanded = v
                    .get("amt_needs_expansion")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);

                if amt_expanded {
                    // New steps will be synthesized at the next iteration of execute_all_steps
                    // because new intent/branch captures are added here
                    if let Some(new_branches) = v.get("new_branches").and_then(|nb| nb.as_array()) {
                        for branch_val in new_branches {
                            if let Some(branch_str) = branch_val.as_str() {
                                let already_exists = state.branch_captures.iter().any(|bc| {
                                    bc.branch.to_lowercase() == branch_str.to_lowercase()
                                });
                                if !already_exists {
                                    // Add to branch_captures so blueprint creation can pick it up
                                    state.branch_captures.push(BranchCapture {
                                        branch: branch_str.to_string(),
                                        parent_intent: state
                                            .intent_captures
                                            .first()
                                            .map(|ic| ic.intent.clone())
                                            .unwrap_or_default(),
                                        source_methodology_ids: vec![],
                                        source_chunk_indices: vec![],
                                        source_sentences: vec![format!(
                                            "Discovered during step {} execution",
                                            step.step_index
                                        )],
                                        node_id: Self::generate_id_static(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_sub_step_input(
        &self,
        state: &OrchestrationState,
        sub_step: &BlueprintSubStep,
        context: &str,
    ) -> Result<serde_json::Value, String> {
        let mut input = serde_json::json!({
            "action": sub_step.action,
            "context": context,
            "prompt": state.cleaned_prompt
        });

        // Apply input mappings
        for (key, value) in &sub_step.input_mapping {
            if let Some(obj) = input.as_object_mut() {
                obj.insert(key.clone(), serde_json::json!(value));
            }
        }

        Ok(input)
    }

    fn extract_output_text(&self, output: &serde_json::Value) -> String {
        output
            .get("response")
            .and_then(|r| r.as_str())
            .or_else(|| output.get("output").and_then(|o| o.as_str()))
            .or_else(|| output.get("result").and_then(|r| r.as_str()))
            .unwrap_or("")
            .to_string()
    }

    // ========================================================================
    // STAGE 9: Result Collection
    // ========================================================================

    // ========================================================================
    // STAGE 10: Post-execution Consciousness
    // ========================================================================

    // ========================================================================
    // STAGE 11: Response Delivery
    // ========================================================================

    // ========================================================================
    // OMEX KNOWLEDGE LINKER — span + context → ranked knowledge paths
    // ========================================================================

    // ========================================================================
    // RESPONSE GRAPH ASSEMBLY — content fixed here, never in the renderer
    // ========================================================================

    // ========================================================================
    // RENDERING LADDER — Tier 0 template · Tier 1 LLM stand-in · Tier 2 OMEX
    // ========================================================================
    // Content is fixed at assembly time and IDENTICAL across tiers; only
    // fluency climbs. Every tier above Tier 0 passes the same deterministic
    // parse-back coverage gate. Tier 0 is the unbreakable floor.

    // ========================================================================
    // Helpers
    // ========================================================================

    async fn wait_for_i_loop(&self) -> Result<(), String> {
        let max_wait_ms = 30000;
        let check_interval_ms = 100;
        let mut waited = 0u64;

        loop {
            let input = serde_json::json!({ "action": "GetStatus" });

            if let Ok(result) = self.executor.execute(44, input).await {
                let active = result
                    .get("active")
                    .and_then(|a| a.as_bool())
                    .unwrap_or(false);
                if !active {
                    return Ok(());
                }
            } else {
                return Ok(());
            }

            if waited >= max_wait_ms {
                return Err("Timeout waiting for I-Loop".to_string());
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
            waited += check_interval_ms;
        }
    }

    /// Also add this static helper for use in non-&self contexts:
    fn generate_id_static() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    fn record_stage(
        &self,
        state: &mut OrchestrationState,
        stage: u8,
        name: &str,
        success: bool,
        summary: &str,
    ) {
        state.stages.push(StageResult {
            stage,
            name: name.to_string(),
            success,
            duration_ms: 0,
            output_summary: Some(summary.to_string()),
        });
    }

    fn record_stage_timed(
        &self,
        state: &mut OrchestrationState,
        stage: u8,
        name: &str,
        success: bool,
        summary: &str,
        duration_ms: u64,
    ) {
        state.stages.push(StageResult {
            stage,
            name: name.to_string(),
            success,
            duration_ms,
            output_summary: Some(summary.to_string()),
        });
    }

    fn build_success_response(&self, state: &OrchestrationState) -> OrchestrationResponse {
        OrchestrationResponse {
            success: !state.needs_clarification,
            response: state.final_response.clone(),
            task_id: state.task_id,
            blueprint_id: state.blueprint_id,
            stages_completed: state.stages.clone(),
            consciousness_gate: state.gate_result.clone(),
            error: None,
            total_tokens_used: Some(state.tokens_used_so_far),
            execution_time_ms: state.start_time.elapsed().as_millis() as u64,
            methodologies_used: state.methodologies.clone(),
            categories_created: state.categories_created,
            blueprints_created: state.blueprints_created,
            clarification_points: state.clarification_points.clone(),
            needs_clarification: state.needs_clarification,
            amt_summary: state.amt.as_ref().map(|amt| AMTSummary {
                total_nodes: amt.count_nodes(),
                branch_count: amt.branch_count(),
                max_depth: amt.max_depth(),
                validation_status: if state.amt_validated {
                    "Validated".to_string()
                } else {
                    format!("Streak: {}/5", state.validation_streak)
                },
            }),
        }
    }

    fn build_error_response(
        &self,
        state: &mut OrchestrationState,
        error: String,
    ) -> OrchestrationResponse {
        OrchestrationResponse {
            success: false,
            response: None,
            task_id: state.task_id,
            blueprint_id: state.blueprint_id,
            stages_completed: state.stages.clone(),
            consciousness_gate: state.gate_result.clone(),
            error: Some(error),
            total_tokens_used: Some(state.tokens_used_so_far),
            execution_time_ms: state.start_time.elapsed().as_millis() as u64,
            methodologies_used: state.methodologies.clone(),
            categories_created: state.categories_created,
            blueprints_created: state.blueprints_created,
            clarification_points: state.clarification_points.clone(),
            needs_clarification: state.needs_clarification,
            amt_summary: None,
        }
    }

    fn estimate_tokens(text: &str) -> u32 {
        ((text.len() + 3) / 4) as u32
    }

    fn parse_json_object(s: &str) -> serde_json::Value {
        let trimmed = s.trim();
        let json_str = if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                &trimmed[start..=end]
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        serde_json::from_str(json_str).unwrap_or_else(|_| serde_json::json!({}))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{RefinementConfig, TaskQueueConfig};

    struct MockExecutor;

    #[async_trait::async_trait]
    impl PipelineExecutor for MockExecutor {
        async fn execute(
            &self,
            pipeline_id: u64,
            input: serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            match pipeline_id {
                9 => Ok(serde_json::json!({
                    "response": "Test response from LLM",
                    "tokens_used": 100
                })),
                100 => {
                    // Text modality
                    let action_type = input
                        .get("action")
                        .and_then(|a| a.get("type"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("");

                    match action_type {
                        "Analyze" => Ok(serde_json::json!({
                            "analysis": {
                                "word_count": 5,
                                "sentence_count": 1,
                                "paragraph_count": 1,
                                "character_count": 25,
                                "entities": [],
                                "topics": [],
                                "keywords": [],
                                "structure": {
                                    "sections": [],
                                    "has_title": false,
                                    "has_abstract": false,
                                    "has_toc": false,
                                    "document_type": "Unknown"
                                }
                            },
                            "processed_chunks": [{
                                "index": 0,
                                "original_text": "Test",
                                "cleaned_text": "Test cleaned",
                                "start_offset": 0,
                                "end_offset": 12,
                                "token_count": 3,
                                "keywords": ["test"],
                                "entities": [],
                                "topics": ["testing"],
                                "overlap_from_previous": 0,
                                "overlap_to_next": 0
                            }],
                            "chunk_graph": {
                                "graph_id": 42
                            }
                        })),
                        "ReconstructFromChunks" => Ok(serde_json::json!({
                            "reconstructed_text": "Test cleaned text"
                        })),
                        _ => Ok(serde_json::json!({"success": true})),
                    }
                }
                _ => Ok(serde_json::json!({"success": true})),
            }
        }
    }

    struct MockZSEI;

    #[async_trait::async_trait]
    impl StoreAccess for MockZSEI {
        async fn query(&self, _query: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({"containers": []}))
        }

        async fn traverse(&self, _request: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({"results": []}))
        }

        async fn create_container(
            &self,
            _parent_id: u64,
            _container: serde_json::Value,
        ) -> Result<u64, String> {
            Ok(1001)
        }

        async fn update_container(
            &self,
            _container_id: u64,
            _updates: serde_json::Value,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn get_container(
            &self,
            _container_id: u64,
        ) -> Result<Option<serde_json::Value>, String> {
            Ok(Some(serde_json::json!({
                "local_state": {
                    "context": {
                        "keywords": ["test"],
                        "categories": []
                    },
                    "storage": {
                        "principles": ["Consider error handling", "Ensure security"]
                    }
                }
            })))
        }

        async fn search_by_keywords(
            &self,
            _keywords: &[String],
            _container_type: Option<&str>,
        ) -> Result<Vec<u64>, String> {
            Ok(vec![])
        }

        async fn get_categories(&self, _modality: &str) -> Result<Vec<u64>, String> {
            Ok(vec![])
        }
    }

    // Implement ZSEIAccess for task module too
    #[async_trait::async_trait]
    impl crate::task::ZSEIAccess for MockZSEI {
        async fn get_container(
            &self,
            _container_id: u64,
        ) -> Result<Option<serde_json::Value>, String> {
            Ok(Some(serde_json::json!({
                "local_state": {
                    "context": { "keywords": ["test"] },
                    "storage": { "principles": ["test principle"] }
                }
            })))
        }

        async fn search_by_keywords(
            &self,
            _keywords: &[String],
            _container_type: Option<&str>,
        ) -> Result<Vec<u64>, String> {
            Ok(vec![])
        }

        async fn get_categories(&self, _modality: &str) -> Result<Vec<u64>, String> {
            Ok(vec![])
        }

        async fn create_container(
            &self,
            _parent_id: u64,
            _container: serde_json::Value,
        ) -> Result<u64, String> {
            Ok(1001)
        }

        async fn update_container(
            &self,
            _container_id: u64,
            _updates: serde_json::Value,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_basic_orchestration() {
        let executor = Arc::new(MockExecutor);
        let zsei = Arc::new(MockZSEI);

        let task_config = TaskQueueConfig {
            consciousness_enabled: false,
            storage_path: "/tmp/test_tasks".to_string(),
            ..Default::default()
        };
        let refinement_config = RefinementConfig {
            enabled: false,
            ..Default::default()
        };
        let task_manager = Arc::new(tokio::sync::RwLock::new(TaskManager::new(task_config, refinement_config).unwrap()));

        let orchestrator =
            PromptOrchestrator::new(executor, zsei, task_manager, Arc::new(RwLock::new(None)));

        let request = OrchestrationRequest {
            prompt: "Hello, how are you?".to_string(),
            project_id: None,
            workspace_id: None,
            user_id: 1,
            device_id: 1,
            consciousness_enabled: false,
            token_budget: Some(10000),
            model_config: None,
            attached_files: Vec::new(),
            processing_path: ProcessingPathPref::default(),
            executor_model: ExecutorModelKind::default(),
            voice_input: None,
        };

        let response = orchestrator.orchestrate(request).await;

        assert!(!response.stages_completed.is_empty());
        // First stage should always complete
        assert_eq!(response.stages_completed[0].stage, 1);
        assert!(response.stages_completed[0].success);
    }

    #[test]
    fn test_amt_node_counting() {
        let mut root = AMTNode::new(1, AMTNodeType::Root, "Root".to_string(), 0);
        let mut branch = AMTNode::new(2, AMTNodeType::Branch, "Branch".to_string(), 1);
        branch
            .children
            .push(AMTNode::new(3, AMTNodeType::Leaf, "Leaf".to_string(), 2));
        root.children.push(branch);

        assert_eq!(root.count_nodes(), 3);
        assert_eq!(root.branch_count(), 1);
        assert_eq!(root.max_depth(), 2);
    }

    #[test]
    fn test_parse_json_object() {
        let result = PromptOrchestrator::parse_json_object(
            r#"Some text before {"key": "value"} some text after"#,
        );
        assert_eq!(result.get("key").and_then(|v| v.as_str()), Some("value"));

        let empty_result = PromptOrchestrator::parse_json_object("no json here");
        assert!(empty_result
            .as_object()
            .map(|o| o.is_empty())
            .unwrap_or(true));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(PromptOrchestrator::estimate_tokens("test"), 1);
        assert_eq!(
            PromptOrchestrator::estimate_tokens("test test test test"),
            5
        );
        assert_eq!(PromptOrchestrator::estimate_tokens(""), 0);
    }
}
