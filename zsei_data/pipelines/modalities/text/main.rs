//! OZONE Studio - Text Modality Pipeline (ID: 100)
//!
//! Analyzes text and creates structural graphs for:
//! - Entities (people, places, organizations) via zero-shot LLM
//! - Topics and themes via zero-shot LLM
//! - Keywords via zero-shot LLM
//! - Document structure (sections, paragraphs)
//! - Relationships between concepts
//! - Cross-references
//!
//! This pipeline uses zero-shot LLM calls for accurate extraction
//! instead of regex-based patterns.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn default_version() -> u32 {
    1
}
fn default_hotness() -> f32 {
    0.5
}

// ============================================================================
// PIPELINE METADATA
// ============================================================================

pub const PIPELINE_ID: u64 = 100;
pub const PIPELINE_NAME: &str = "TextAnalysisPipeline";
pub const PIPELINE_VERSION: &str = "0.4.0";
pub const PIPELINE_MODALITY: &str = "text";

// ============================================================================
// OMEX PIPELINE CONSTANTS
// ============================================================================

/// OMEX GrammarParser — text → full grammar graph in a single forward pass
/// (non-autoregressive). Emits sentences with spans/corrections, grammar
/// trees, section events, modality spans, cross-sentence relationships,
/// coreference chains, and knowledge references.
pub const OMEX_TEXT_PARSER_PIPELINE_ID: u64 = 900;
/// OMEX KnowledgeLinker — knowledge span + context → ranked knowledge paths.
pub const OMEX_KNOWLEDGE_LINKER_PIPELINE_ID: u64 = 901;
/// OMEX Realizer — response graph → text (reverse grammar traversal).
pub const OMEX_REALIZER_PIPELINE_ID: u64 = 902;

/// Which executor model class is serving this request. Determines routing:
/// LLM → Path 1 whole-chunk prompting; SLM → Path 2 granular loops;
/// OMEX → native single-pass parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutorModelKind {
    #[default]
    Auto,
    Llm,
    Slm,
    Omex,
}

/// How Phase 3 grammar extraction runs. GraphNative (default) extracts on
/// the full graph after all chunks are processed; PerSentence uses the
/// legacy per-chunk extractor (kept as a selectable option for review and
/// selective use — never removed, never deprecated).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GrammarExtractionMode {
    #[default]
    GraphNative,
    PerSentence,
}

/// Fallback modality registry — used only when the caller passes an empty
/// `available_modalities` list (first run, before the root modality list has
/// aggregated anything). The registry-driven list always wins when provided.
const BUILTIN_MODALITY_NAMES: &[&str] = &[
    "code","image","audio","video","math","chemistry","dna","eeg","3d","sound",
    "biology","proteomics","haptic","thermal","depth","imu","geospatial",
    "electromagnetic","bci","parametric_cad","kinematics","control_systems",
    "network_topology","radar","sonar","hyperspectral",
];

// ═══════════════════════════════════════════════════════════════════════════
// UNIVERSAL TYPES (per-pipeline copy; JSON is the contract)
// ═══════════════════════════════════════════════════════════════════════════

/// Source-object provenance for edges and nodes.
/// Answers: "what object was this derived from?"
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum EdgeProvenance {
    #[default]
    Unknown,
    DerivedFromPrompt,
    DerivedFromChunk(u32),
    DerivedFromChunkGraph(u64),
    DerivedFromModalityGraph(u64),
    DerivedFromFile(String),
    DerivedFromFileGraph(u64),
    DerivedFromAMT,
    DerivedFromAMTBranch,
    DerivedFromBlueprint(u32),
    DerivedFromBlueprintStep(u32),
    DerivedFromMethodology(u64),
    DerivedFromCrossModal,
    DerivedFromHook,
    VersionOf(u32),
    ForkedFrom(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ProvisionalStatus {
    #[default]
    Planned,
    Generating,
    Generated,
    Validated,
    Finalized,
    Failed,
    RolledBack,
}

/// Merged ChangeType — both drafts combined.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChangeType {
    #[default]
    Created,
    Updated,
    Modified,
    RelationshipAdded,
    CrossModalLinkAdded,
    EnrichedBySemantic,
    EnrichedByHook,
    ProvisionalFinalized,
    RolledBack,
    Finalized,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionNote {
    pub version: u32,
    pub note: String,
    pub step_index: Option<u32>,
    pub timestamp: String,
    pub change_type: ChangeType,
}

/// Inline cross-modal reference stored on each node for fast lookup.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrossModalRef {
    pub target_graph_id: u64,
    pub target_node_id: u64,
    pub target_modality: String,
    pub relationship: String,
    pub bidirectional: bool,
}

/// Graph lifecycle state machine.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum GraphStateType {
    #[default]
    Created,
    SemanticEnriched,
    CrossLinked,
    Stable,
    Updated,
    ReValidating,
    Failed,
    Archived,
}

/// Convenience struct for cross-pipeline graph state tracking.
/// Extracted from node fields; used by orchestrator queries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeCore {
    pub node_id: u64,
    pub node_type: String,
    pub label: String,
    pub content: String,
    pub provisional: bool,
    pub provisional_status: ProvisionalStatus,
    pub provenance: EdgeProvenance,
    pub source_chunk_id: Option<u64>,
    pub source_file_id: Option<u64>,
    pub created_by_step: Option<u32>,
    pub updated_by_step: Option<u32>,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
    pub materialized_path: Option<String>,
    pub keywords: Vec<String>,
    pub embedding_hint: Option<String>,
    pub hotness_score: f32,
    pub source_chunk_index: Option<u32>,
    pub source_start_char: Option<usize>,
    pub source_end_char: Option<usize>,
    pub cross_modal_refs: Vec<CrossModalRef>,
    pub time_range_start: Option<f32>,
    pub time_range_end: Option<f32>,
}

/// Standard fields for ALL modality graph edges.
/// These are added inline to each pipeline's edge struct.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EdgeMeta {
    pub provenance: EdgeProvenance,
    pub created_by_step: Option<u32>,
    pub version: u32,
    pub version_notes: Vec<VersionNote>,
    pub is_cross_modal: bool,
    pub cross_modal_index_id: Option<u64>,
}

// ============================================================================
// EXECUTOR TRAIT
// ============================================================================

/// Trait for executing other pipelines (injected by runtime)
#[async_trait::async_trait]
pub trait PipelineExecutor: Send + Sync {
    async fn execute(
        &self,
        pipeline_id: u64,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String>;
}

// ============================================================================
// PROCESSING PATHS & STRUCTURED NODES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingPath {
    /// Deconstructor — top-down: paragraphs 1x1 → sentences → whole-chunk clean.
    Path1,
    /// Constructor — bottom-up: sentences 1x1; structure built later from graphs.
    Path2,
    /// OMEX GrammarParser — the entire chunk graph in one forward pass.
    #[serde(alias = "omex")]
    OmexNative,
}

impl Default for ProcessingPath {
    fn default() -> Self {
        ProcessingPath::Path1
    }
}

// ============================================================================
// PHASE 1 STATE MACHINES (cross-chunk carried)
// ============================================================================

/// The section tracking state machine. Documents are NEVER tracked here —
/// document boundaries are Phase-4 graph-traversal territory (they need the
/// full relationship evidence to judge a true break without context explosion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SectionTrackingState {
    #[default]
    Idle,
    InSection,
    InSubsection,
}

/// The only section events the per-chunk tracker emits. No `document_break` —
/// that is decided later on the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionEventType {
    SectionStart,
    SubsectionStart,
    SectionCloseAndStart,
}

/// One open section in the ancestor stack (outermost → innermost).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionAncestor {
    pub section_id: u64,
    pub title: String,
    pub level: u8,
}

/// Cross-chunk section carry: persistent section IDs, formatting pattern,
/// ancestor stack for nesting. Once a section opens it stays open until a
/// same-or-higher marker or the end of all chunks — never a style shift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionCarryState {
    pub state: SectionTrackingState,
    pub ancestors: Vec<SectionAncestor>,
    pub formatting_pattern: Option<String>,
}

impl Default for SectionCarryState {
    fn default() -> Self {
        Self {
            state: SectionTrackingState::Idle,
            ancestors: Vec::new(),
            formatting_pattern: None,
        }
    }
}

impl SectionCarryState {
    /// The innermost open section, if any.
    pub fn current(&self) -> Option<&SectionAncestor> {
        self.ancestors.last()
    }

    /// The section id sentences/paragraphs tie to right now.
    pub fn current_section_id(&self) -> Option<u64> {
        self.current().map(|a| a.section_id)
    }
}

/// Cross-chunk paragraph carry (Path 1). A paragraph may begin in chunk N and
/// end in chunk N+1 — this carry makes the continuation explicit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParagraphCarryState {
    pub open: bool,
    /// Absolute offset of the open paragraph's start in the full prompt text.
    pub absolute_start: Option<usize>,
    /// Tail of the open paragraph's last known text (small, for context).
    pub tail: Option<String>,
    pub paragraphs_found_total: u64,
}

/// Cross-chunk sentence carry. The last sentence of a chunk may be cut — the
/// fragment is prepended to the next chunk, absolutely anchored.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SentenceCarryState {
    pub open_fragment: Option<String>,
    pub fragment_absolute_start: Option<usize>,
}

/// Cross-chunk modality carry: an open block whose closing delimiter did not
/// appear in this chunk continues into the next.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModalityCarryState {
    pub open: bool,
    pub open_modality: Option<String>,
}

/// A candidate sentence returned by extraction, before validation.
#[derive(Debug, Clone)]
pub struct CandidateSentence {
    pub order: u32,
    pub original: String,
    pub span_start: usize,
    pub span_end: usize,
    pub corrected: String,
}

/// Outcome of one extraction attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionOutcome {
    /// Extraction returned found:false (nothing after the last sentence).
    Exhausted,
    /// A candidate was returned (validation still required).
    Candidate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum GrammarNodeType {
    Sentence,
    MainClause,
    SubordinateClause,
    RelativeClause,
    ComplementClause,
    AdverbialClause,
    ConditionalClause,
    ComparativeClause,
    CoordinateClause,
    ParentheticalClause,
    EllipticalClause,
    QuotedClause,
    Phrase,
    NounPhrase,
    VerbPhrase,
    AdjectivePhrase,
    AdverbPhrase,
    PrepositionalPhrase,
    InfinitivePhrase,
    ParticipialPhrase,
    GerundPhrase,
    AbsolutePhrase,
    AppositivePhrase,
    CoordinatePhrase,
    ParentheticalPhrase,
    Predicate,
    Verb,
    MainVerb,
    AuxiliaryVerb,
    ModalVerb,
    LinkingVerb,
    HelpingVerb,
    PhrasalVerb,
    Copula,
    PredicateComplement,
    PredicateNominative,
    PredicateAdjective,
    Subject,
    ImpliedSubject,
    ExpletiveSubject,
    DirectObject,
    IndirectObject,
    ObjectComplement,
    SubjectComplement,
    Complement,
    Noun,
    CommonNoun,
    ProperNoun,
    CollectiveNoun,
    MassNoun,
    CountNoun,
    ConcreteNoun,
    AbstractNoun,
    CompoundNoun,
    Pronoun,
    PersonalPronoun,
    ReflexivePronoun,
    ReciprocalPronoun,
    RelativePronoun,
    DemonstrativePronoun,
    InterrogativePronoun,
    IndefinitePronoun,
    PossessivePronoun,
    Determiner,
    Article,
    Demonstrative,
    PossessiveDeterminer,
    Quantifier,
    Numeral,
    DistributiveDeterminer,
    Modifier,
    Adjective,
    AttributiveAdjective,
    PredicativeAdjective,
    ComparativeAdjective,
    SuperlativeAdjective,
    Adverb,
    AdverbOfTime,
    AdverbOfPlace,
    AdverbOfManner,
    AdverbOfDegree,
    AdverbOfFrequency,
    SentenceAdverb,
    Preposition,
    SimplePreposition,
    CompoundPreposition,
    PhrasalPreposition,
    Conjunction,
    CoordinatingConjunction,
    SubordinatingConjunction,
    CorrelativeConjunction,
    RelativePronounClause,
    RelativeModifier,
    AdjectivalModifier,
    AdverbialModifier,
    NominalModifier,
    DeterminerModifier,
    PossessiveModifier,
    NumericModifier,
    QuantifierModifier,
    Apposition,
    Vocative,
    Parenthetical,
    PrepositionalObject,
    ObjectOfPreposition,
    ComplementOfPreposition,
    InfinitiveMarker,
    Infinitive,
    Gerund,
    PresentParticiple,
    PastParticiple,
    ClauseSubject,
    ClausePredicate,
    ClauseObject,
    ClauseComplement,
    ClauseModifier,
    Negation,
    NegativeMarker,
    NegativeDeterminer,
    NegativePronoun,
    QuestionMarker,
    InterrogativeWord,
    TagQuestion,
    WhPhrase,
    Comparison,
    ComparativeMarker,
    SuperlativeMarker,
    EqualityMarker,
    Coordination,
    Coordinator,
    CoordinatedElement,
    AgreementMarker,
    CaseMarker,
    NumberMarker,
    GenderMarker,
    PersonMarker,
    Punctuation,
    Comma,
    Period,
    Colon,
    Semicolon,
    Dash,
    Hyphen,
    Parenthesis,
    Quotation,
    Apostrophe,
    Ellipsis,
    Exclamation,
    QuestionMark,
    //====================================================
    // STRUCTURAL (document-layer nodes — text modality graph)
    //====================================================

    Paragraph,
    Section,
    Document,

    //====================================================
    // TERMINALS
    //====================================================

    Token,
    Word,
    Symbol,
    Number,
    Letter,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GrammarProperties {
    pub tense: Option<String>,
    pub aspect: Option<String>,
    pub mood: Option<String>,
    pub voice: Option<String>,
    pub person: Option<String>,
    pub number: Option<String>,
    pub gender: Option<String>,
    pub case: Option<String>,
    pub definiteness: Option<String>,
    pub comparison: Option<String>,
    pub polarity: Option<String>,
    pub subtype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarNode {
    pub node_id: u64,
    pub node_type: GrammarNodeType,
    pub text: String,
    pub position: TextPosition,
    pub children: Vec<GrammarNode>,
    pub properties: GrammarProperties,
}

/// A knowledge reference emitted by the OMEX GrammarParser (or the factory
/// pipeline). Detection rides on the sentence; resolution is external.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRef {
    pub span_start: usize,
    pub span_end: usize,
    pub surface: String,
    /// concept | principle | procedure | entity | process | tool | material
    pub knowledge_kind: String,
    pub topic_path: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceNode {
    pub node_id: u64,
    pub node_type: GrammarNodeType,
    pub content: String,
    pub original_content: String,
    pub position: TextPosition,
    pub chunk_id: u32,
    pub chunk_offset: usize,
    pub paragraph_id: Option<u64>,
    pub section_id: Option<u64>,
    pub properties: GrammarProperties,
    pub grammar_nodes: Vec<GrammarNode>,
    /// Grammar relationships extracted for THIS sentence (Phase 3,
    /// graph-native). Tied to the sentence node itself — grammar is local to
    /// each sentence, not a chunk-level aggregate.
    #[serde(default)]
    pub grammar_relationships: Vec<ChunkGrammarRelationship>,
    /// Knowledge-bearing spans (OMEX native). Detection only — the system
    /// never resolves or loads knowledge here.
    #[serde(default)]
    pub knowledge_refs: Vec<KnowledgeRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphNode {
    pub node_id: u64,
    pub node_type: GrammarNodeType,
    pub sentence_count: u32,
    pub parent_section: Option<u64>,
    pub position: TextPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionNode {
    pub node_id: u64,
    pub node_type: GrammarNodeType,
    pub content: String,
    pub level: u8,
    pub formatting_pattern: Option<String>,
    pub parent_section: Option<u64>,
    pub position: TextPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentNode {
    pub node_id: u64,
    pub node_type: GrammarNodeType,
    pub content: String,
    pub document_type: String,
    pub title: Option<String>,
    pub position: TextPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSentenceRelationship {
    pub from_sentence_id: u64,
    pub to_sentence_id: u64,
    pub relationship_type: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreferenceMention {
    pub sentence_id: u64,
    pub text: String,
    pub grammar_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreferenceChain {
    pub chain_id: u64,
    pub canonical_form: String,
    pub mentions: Vec<CoreferenceMention>,
}

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct TextModalityInput {
    pub action: TextModalityAction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TextModalityAction {
    /// Analyze text and create structural representation
    Analyze {
        text: String,
        #[serde(default = "default_max_chunk_tokens")]
        max_chunk_tokens: u32,
        #[serde(default)]
        depth: AnalysisDepth,
        #[serde(default)]
        extract_entities: bool,
        #[serde(default)]
        extract_topics: bool,
        #[serde(default)]
        available_modalities: Vec<String>,
        #[serde(default)]
        processing_path: ProcessingPath,
        /// Executor model class — selects the route and can override to
        /// OMEX-native single-pass parsing.
        #[serde(default)]
        executor_model: ExecutorModelKind,
        /// Phase 3 grammar extraction mode. GraphNative (default) extracts
        /// on the full graph after all chunks are processed; PerSentence
        /// uses the legacy per-chunk extractor (kept for review/debug and
        /// selective use via ExtractGrammarRelationships).
        #[serde(default)]
        grammar_mode: GrammarExtractionMode,
    },

    /// Create a graph from analysis results
    CreateGraph {
        analysis_result: TextAnalysisResult,
        project_id: u64,
        #[serde(default)]
        link_to_existing: bool,
    },

    /// Update existing graph with new text
    UpdateGraph { graph_id: u64, delta: TextDelta },

    /// Query the text graph
    QueryGraph {
        graph_id: u64,
        query: TextGraphQuery,
    },

    /// Get existing graph
    GetGraph { graph_id: u64 },

    /// Clean and normalize a text chunk via zero-shot LLM
    CleanChunk { chunk: RawChunk },

    /// Reconstruct cleaned prompt from processed chunks
    ReconstructFromChunks { chunks: Vec<ProcessedChunk> },

    /// Extract keywords via zero-shot LLM
    ExtractKeywords {
        text: String,
        #[serde(default = "default_max_keywords")]
        max_keywords: usize,
    },

    /// Extract entities via zero-shot LLM
    ExtractEntities { text: String },

    /// Extract topics via zero-shot LLM
    ExtractTopics { text: String },

    /// Detect modalities present in a chunk — 1x1 in order, dual 5x
    /// validation. No true_text, no unknown: prose is covered by sentence
    /// extraction; only non-prose spans are reported.
    DetectModalities {
        text: String,
        #[serde(default)]
        chunk_index: u32,
        #[serde(default)]
        available_modalities: Vec<String>,
    },

    /// Create a persistent chunk graph from a processed chunk
    CreateChunkGraph {
        chunk: ProcessedChunk,
        root_graph_id: u64,
    },

    /// Phase 3 — grammar extraction on the FULL graph (all chunks processed):
    /// per-sentence grammar into SentenceNode.grammar_relationships, pairwise
    /// cross-sentence relationships, coreference chains.
    ExtractGrammarFromGraphs {
        chunks: Vec<ProcessedChunk>,
    },

    /// Legacy per-chunk grammar extraction — KEPT as a selectable option
    /// (default path is ExtractGrammarFromGraphs). Also useful for review
    /// and debugging of individual text spans.
    ExtractGrammarRelationships {
        text: String,
        chunk_index: u32,
    },

    /// Trigger ZSEI semantic analysis hook
    TriggerSemanticHook {
        graph_id: u64,
        hook_type: ZSEIHookType,
    },

    /// Link to another modality graph
    LinkToModality {
        source_graph_id: u64,
        target_graph_id: u64,
        target_modality: String,
        relationship: CrossModalityRelation,
    },
}

fn default_max_keywords() -> usize {
    20
}
fn default_max_chunk_tokens() -> u32 {
    2000
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
pub enum AnalysisDepth {
    Surface,
    #[default]
    Standard,
    Deep,
    Comprehensive,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextModalityOutput {
    pub success: bool,
    pub error: Option<String>,

    // Analysis results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<TextAnalysisResult>,

    // Graph results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_id: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<TextGraph>,

    // Chunking results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunks: Option<Vec<RawChunk>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_chunks: Option<Vec<ProcessedChunk>>,

    // Normalization results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleaned_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconstructed_text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u32>,

    // Extraction results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<Keyword>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<Entity>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<String>>,

    // Hook results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_result: Option<HookResult>,

    // Cross-modality link result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_result: Option<LinkResult>,

    // Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_time_ms: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar_relationships: Option<Vec<ChunkGrammarRelationship>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality_detections: Option<Vec<ChunkModalityDetection>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_graph: Option<ChunkGraph>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentence_analyses: Option<Vec<SentenceAnalysis>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub true_text_spans: Option<Vec<TextSpan>>,

    /// Total LLM tokens consumed by this analysis (extraction + validation
    /// + grammar calls). The orchestrator folds this into its budget.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_tokens_used: Option<u64>,

    /// Chunks after Phase 3/4 mutation (grammar relationships on sentence
    /// nodes, cross-sentence relationships, coreference chains, and — Path 2
    /// — constructed paragraphs). Prefer these over processed_chunks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_chunks: Option<Vec<ProcessedChunk>>,
}

impl Default for TextModalityOutput {
    fn default() -> Self {
        Self {
            success: false,
            error: None,
            analysis: None,
            graph_id: None,
            graph: None,
            chunks: None,
            processed_chunks: None,
            normalized_text: None,
            cleaned_text: None,
            reconstructed_text: None,
            token_count: None,
            keywords: None,
            entities: None,
            topics: None,
            hook_result: None,
            link_result: None,
            processing_time_ms: None,
            grammar_relationships: None,
            modality_detections: None,
            chunk_graph: None,
            sentence_analyses: None,
            true_text_spans: None,
            llm_tokens_used: None,
            updated_chunks: None,
        }
    }
}

// ── OVERLAP RESOLUTION TYPES ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OverlapResolution {
    pub has_overlap: bool,
    pub overlap_type: OverlapType,
    pub current_keep_end: usize, // char index in original current chunk text
    pub next_start_offset: usize, // char offset to skip in next chunk
    pub duplicate_belongs_in: DuplicateOwner,
    pub resolution_method: ResolutionMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OverlapType {
    #[default]
    None,
    SentenceCutoff,
    ParagraphCutoff,
    WordCutoff,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DuplicateOwner {
    #[default]
    CurrentChunk,
    NextChunk,
    Neither,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ResolutionMethod {
    #[default]
    LLMZeroShot,
    RuleBased, // emergency fallback only — not default path
}

// ── GRAMMAR TYPES ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum VerbType {
    #[default]
    Action, // run, create, modify
    Linking, // is, seems, becomes
    Helping, // has, will, can
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkGrammarRelationship {
    pub from_text: String,
    pub to_text: String,
    pub edge_type: String,
    pub tense: Option<String>,
    pub negated: bool,
    pub verb: String,
    pub verb_type: VerbType,
    pub subject: String,
    pub object: Option<String>,
    pub source_sentence_start: Option<usize>,
    pub source_sentence_end: Option<usize>,
    pub chunk_index: u32,
}

// ── MODALITY DETECTION TYPES ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkModalityDetection {
    pub modality: String,
    pub span_start: usize,
    pub span_end: usize,
    pub intent_reference: String,
    pub chunk_index: u32,
    /// Tied to a SentenceNode (span inside a sentence) or ParagraphNode
    /// (span outside all sentences). NEVER to the chunk — chunks are
    /// position anchors only.
    pub parent_node_id: Option<u64>,
    /// True when a block-style span had no closing delimiter in this chunk —
    /// carried into the next chunk for end-confirmation.
    #[serde(default)]
    pub open_at_chunk_end: bool,
    /// True when tied interim to the nearest preceding SentenceNode and must
    /// be re-tied to a ParagraphNode once paragraphs are constructed (Path 2,
    /// Phase 4).
    #[serde(default)]
    pub needs_reparent: bool,
}

// ── CHUNK GRAPH TYPES ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceBoundary {
    pub start: usize, // char offset relative to chunk start
    pub end: usize,
    pub sentence_type: SentenceType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SentenceType {
    #[default]
    Declarative,
    Interrogative,
    Imperative,
    Fragment,
    CodeBlock,
    MathExpression,
}

/// Chunk graph — one per processed chunk. Persistent historical evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkGraph {
    pub graph_id: u64,
    pub chunk_index: u32,
    pub prompt_start_char: usize,
    pub prompt_end_char: usize,
    pub sentence_boundaries: Vec<SentenceBoundary>,
    pub paragraph_breaks: Vec<usize>,
    pub cleaned_text: String,
    pub overlap_resolution: Option<OverlapResolution>,
    pub keywords: Vec<String>,
    pub topics: Vec<String>,
    pub sentence_nodes: Vec<SentenceNode>,
    pub paragraph_nodes: Vec<ParagraphNode>,
    pub section_nodes: Vec<SectionNode>,
    pub document_nodes: Vec<DocumentNode>,
    pub cross_sentence_relationships: Vec<CrossSentenceRelationship>,
    pub coreference_chains: Vec<CoreferenceChain>,
    pub modality_detections: Vec<ChunkModalityDetection>,
    pub root_modality_list_contribution: Vec<String>,
    pub created_at: String,
}

/// Result of smart context reconstruction from chunk graphs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructedContext {
    pub text: String,
    pub included_chunk_indices: Vec<u32>,
    pub total_chars: usize,
    pub estimated_tokens: usize,
}

// ============================================================================
// CHUNK TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawChunk {
    pub index: u32,
    pub text: String,
    pub start_char: u32,
    pub end_char: u32,
    pub token_count: u32,
    #[serde(default)]
    pub is_complete_paragraph: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    #[serde(default)]
    pub overlap_from_previous: u32,
    #[serde(default)]
    pub overlap_to_next: u32,
    pub sentence_nodes: Vec<SentenceNode>,
    pub paragraph_nodes: Vec<ParagraphNode>,
    pub section_nodes: Vec<SectionNode>,
    pub document_nodes: Vec<DocumentNode>,
    pub cross_sentence_relationships: Vec<CrossSentenceRelationship>,
    pub coreference_chains: Vec<CoreferenceChain>,
    pub detected_modalities: Vec<ChunkModalityDetection>,
    pub chunk_graph_id: Option<u64>,
    pub prompt_start_char: usize,
    pub prompt_end_char: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtractedEntity {
    pub text: String,
    pub entity_type: String,
    pub confidence: f32,
    pub start_offset: Option<usize>,
    pub end_offset: Option<usize>,
}

// ============================================================================
// ANALYSIS TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextAnalysisResult {
    pub word_count: usize,
    pub sentence_count: usize,
    pub paragraph_count: usize,
    pub character_count: usize,
    pub entities: Vec<Entity>,
    pub topics: Vec<Topic>,
    pub keywords: Vec<Keyword>,
    pub structure: DocumentStructure,
    pub language: Option<String>,
    pub sentiment: Option<Sentiment>,
    pub readability_score: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    pub text: String,
    pub entity_type: EntityType,
    pub start_offset: usize,
    pub end_offset: usize,
    pub confidence: f32,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Date,
    Time,
    Money,
    Percentage,
    Product,
    Event,
    Technology,
    Concept,
    Custom(String),
}

impl EntityType {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "PERSON" | "PER" => EntityType::Person,
            "ORGANIZATION" | "ORG" => EntityType::Organization,
            "LOCATION" | "LOC" | "GPE" => EntityType::Location,
            "DATE" => EntityType::Date,
            "TIME" => EntityType::Time,
            "MONEY" | "CURRENCY" => EntityType::Money,
            "PERCENTAGE" | "PERCENT" => EntityType::Percentage,
            "PRODUCT" => EntityType::Product,
            "EVENT" => EntityType::Event,
            "TECHNOLOGY" | "TECH" => EntityType::Technology,
            "CONCEPT" => EntityType::Concept,
            other => EntityType::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Topic {
    pub name: String,
    pub keywords: Vec<String>,
    pub relevance: f32,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Keyword {
    pub term: String,
    pub frequency: usize,
    pub relevance: f32,
    pub is_phrase: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentStructure {
    pub sections: Vec<Section>,
    pub has_title: bool,
    pub has_abstract: bool,
    pub has_toc: bool,
    pub document_type: DocumentType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Section {
    pub id: String,
    pub title: Option<String>,
    pub level: u8,
    pub start_offset: usize,
    pub end_offset: usize,
    pub subsections: Vec<Section>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DocumentType {
    Article,
    Report,
    Email,
    Code,
    Chat,
    Documentation,
    Prompt,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sentiment {
    pub overall: f32,
    pub positive: f32,
    pub negative: f32,
    pub neutral: f32,
}

// ============================================================================
// GRAPH TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGraph {
    pub graph_id: u64,
    pub modality: String,
    pub version: String,
    pub nodes: Vec<TextGraphNode>,
    pub edges: Vec<TextGraphEdge>,
    pub metadata: HashMap<String, Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGraphNode {
    pub node_id: u64,
    pub node_type: TextNodeType,
    pub content: String,
    pub position: Option<TextPosition>,
    pub properties: HashMap<String, Value>,
    pub semantic_annotations: Vec<SemanticAnnotation>,
    // UNIVERSAL NODE FIELDS
    #[serde(default)]
    pub provisional: bool,
    #[serde(default)]
    pub provisional_status: ProvisionalStatus,
    #[serde(default)]
    pub provenance: EdgeProvenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_chunk_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_step: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by_step: Option<u32>,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub version_notes: Vec<VersionNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materialized_path: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_hint: Option<String>,
    #[serde(default = "default_hotness")]
    pub hotness_score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_chunk_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_start_char: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_end_char: Option<usize>,
    #[serde(default)]
    pub cross_modal_refs: Vec<CrossModalRef>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TextNodeType {
    Document,
    Section,
    Paragraph,
    Sentence,
    Entity,
    Topic,
    Keyword,
    Reference,
    Chunk,
    ModalityReference,
    TrueTextSpan,
    FileReference,
    ChunkReference,
    SupplementarySection,
    GrammarSubject,
    GrammarObject,
    InferredConcept,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextPosition {
    pub start_offset: usize,
    pub end_offset: usize,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SemanticAnnotation {
    pub annotation_type: String,
    pub value: String,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGraphEdge {
    pub edge_id: u64,
    pub from_node: u64,
    pub to_node: u64,
    pub edge_type: TextEdgeType,
    pub weight: f32,
    pub properties: HashMap<String, Value>,
    // UNIVERSAL EDGE FIELDS
    #[serde(default)]
    pub provenance: EdgeProvenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_step: Option<u32>,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub version_notes: Vec<VersionNote>,
    #[serde(default)]
    pub is_cross_modal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_modal_index_id: Option<u64>,
    // Grammar info for grammar-derived edges
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar_info: Option<GrammarEdgeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarEdgeInfo {
    pub verb: String,
    pub verb_type: VerbType,
    pub tense: String,
    pub negated: bool,
    pub source_sentence: String,
}

/// Sentence-level analysis used in SentenceAnalysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentInfo {
    pub label: String, // positive / negative / neutral
    pub score: f32,
}

/// A span that is genuinely text (not embedded code/math/etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSpan {
    pub start_char: usize,
    pub end_char: usize,
    pub chunk_index: u32,
    pub is_true_text: bool,
}

/// Full sentence analysis produced per-chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceAnalysis {
    pub text: String,
    pub start_char: usize,
    pub end_char: usize,
    pub subject: String,
    pub verb: String,
    pub verb_type: VerbType,
    pub object: Option<String>,
    pub tense: String,
    pub relationships: Vec<ChunkGrammarRelationship>,
    pub sentiment: Option<SentimentInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TextEdgeType {
    // Structural
    Contains,
    ContainedBy,
    Follows,
    Precedes,

    // Semantic (added by ZSEI)
    References,
    Contradicts,
    Supports,
    Elaborates,
    Summarizes,

    // Cross-modality
    DescribesCode,
    DescribesImage,
    DescribesAudio,
    DescribesVideo,
    TranscribedFrom,

    // Universal Semantic
    Performs,
    Affects,
    Implies,
    TemporalPrecedes,
    TemporalFollows,
    CausedBy,
    Enables,
    Prevents,
    PartOf,
    HasPart,
    FunctionalRole,
    InstanceOf,
    HasInstance,
    SimilarTo,
    RelatesTo,

    // Versioning / Derivation
    DerivedFrom,
    VersionOf,
    RefinesTo,
    ForkedFrom,

    // Cross-Modality
    ReferencesModality,
    ReferencedBy,
    DescribedBy,
    Describes,
    ImplementedIn,
    Implements,
    VisualizedAs,
    Visualizes,
    SyncedTo,
    SyncedBy,
    AnnotatedBy,

    // Supplementary
    SupplementsPrompt,
    ContextProvides,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextDelta {
    pub operation: DeltaOperation,
    pub position: Option<TextPosition>,
    pub content: Option<String>,
    pub affected_nodes: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DeltaOperation {
    Insert,
    Delete,
    Replace,
    Reorder,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGraphQuery {
    pub query_type: TextQueryType,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TextQueryType {
    FindEntities,
    FindTopics,
    FindKeywords,
    FindReferences,
    GetStructure,
    SemanticSearch,
    PathBetween,
    GetNodesByType,
    Custom,
}

// ============================================================================
// CROSS-MODALITY TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum CrossModalityRelation {
    DescribesCode,
    DescribesImage,
    DescribesAudio,
    DescribesVideo,
    TranscribedFrom,
    IllustratedBy,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkResult {
    pub link_id: u64,
    pub source_graph_id: u64,
    pub target_graph_id: u64,
    pub relationship: CrossModalityRelation,
    pub created_at: String,
}

// ============================================================================
// ZSEI HOOK TYPES
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ZSEIHookType {
    OnGraphCreated,
    OnEdgeCompletion,
    OnInferRelationships,
    OnCrossModalityLink,
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
// PIPELINE IMPLEMENTATION
// ============================================================================

pub struct TextModalityPipeline {
    executor: Arc<dyn PipelineExecutor>,
    graph_cache: tokio::sync::RwLock<HashMap<u64, TextGraph>>,
    /// Cumulative LLM tokens consumed across this pipeline instance.
    llm_tokens_used: AtomicU64,
    /// Cumulative executor call count (observability).
    llm_calls: AtomicU64,
}

impl TextModalityPipeline {
    pub fn new(executor: Arc<dyn PipelineExecutor>) -> Self {
        Self {
            executor,
            graph_cache: tokio::sync::RwLock::new(HashMap::new()),
            llm_tokens_used: AtomicU64::new(0),
            llm_calls: AtomicU64::new(0),
        }
    }

    /// Read-and-reset the meters (explicit external reads only).
    pub fn take_llm_metrics(&self) -> (u64, u64) {
        (
            self.llm_tokens_used.swap(0, Ordering::Relaxed),
            self.llm_calls.swap(0, Ordering::Relaxed),
        )
    }

    /// Snapshot the meters WITHOUT resetting (used internally per phase).
    fn llm_metrics_snapshot(&self) -> (u64, u64) {
        (
            self.llm_tokens_used.load(Ordering::Relaxed),
            self.llm_calls.load(Ordering::Relaxed),
        )
    }

    pub async fn execute(&self, input: TextModalityInput) -> TextModalityOutput {
        let start_time = std::time::Instant::now();

        let mut output = match input.action {
            TextModalityAction::Analyze {
                text,
                max_chunk_tokens,
                depth,
                extract_entities,
                extract_topics,
                available_modalities,
                processing_path,
                executor_model,
                grammar_mode,
            } => {
                self.analyze_text(
                    &text,
                    max_chunk_tokens,
                    depth,
                    extract_entities,
                    extract_topics,
                    &available_modalities,
                    processing_path,
                    executor_model,
                    grammar_mode,
                )
                .await
            }

            TextModalityAction::CreateGraph {
                analysis_result,
                project_id,
                link_to_existing,
            } => {
                self.create_graph(analysis_result, project_id, link_to_existing)
                    .await
            }

            TextModalityAction::UpdateGraph { graph_id, delta } => {
                self.update_graph(graph_id, delta).await
            }

            TextModalityAction::QueryGraph { graph_id, query } => {
                self.query_graph(graph_id, query).await
            }

            TextModalityAction::GetGraph { graph_id } => self.get_graph(graph_id).await,

            TextModalityAction::CreateChunkGraph {
                chunk,
                root_graph_id,
            } => {
                let graph = self.create_chunk_graph(&chunk, root_graph_id);
                TextModalityOutput {
                    success: true,
                    chunk_graph: Some(graph),
                    ..Default::default()
                }
            }

            TextModalityAction::CleanChunk { chunk } => self.clean_chunk(chunk).await,

            TextModalityAction::ReconstructFromChunks { chunks } => {
                self.reconstruct_from_chunks(&chunks)
            }

            TextModalityAction::ExtractKeywords { text, max_keywords } => {
                self.extract_keywords_llm(&text, max_keywords).await
            }

            TextModalityAction::ExtractGrammarFromGraphs { chunks } => {
                let (updated, tokens) = self.extract_grammar_from_graphs(chunks).await;
                TextModalityOutput {
                    success: true,
                    updated_chunks: Some(updated),
                    llm_tokens_used: Some(tokens),
                    ..Default::default()
                }
            }

            // Legacy per-chunk extractor — kept as a selectable option
            // (default path is ExtractGrammarFromGraphs); useful for review.
            TextModalityAction::ExtractGrammarRelationships { text, chunk_index } => {
                let rels = self
                    .extract_grammar_relationships_from_text(&text, chunk_index)
                    .await;
                TextModalityOutput {
                    success: true,
                    grammar_relationships: Some(rels),
                    ..Default::default()
                }
            }

            TextModalityAction::ExtractEntities { text } => self.extract_entities_llm(&text).await,

            TextModalityAction::ExtractTopics { text } => self.extract_topics_llm(&text).await,

            TextModalityAction::TriggerSemanticHook {
                graph_id,
                hook_type,
            } => self.trigger_semantic_hook(graph_id, hook_type).await,

            TextModalityAction::DetectModalities {
                text,
                chunk_index,
                available_modalities,
            } => {
                // 1x1 in order, dual 5x validation. true_text is never
                // emitted — it is derivable by subtraction (validated
                // sentences cover prose; modality spans mark the rest).
                let mut carry = ModalityCarryState::default();
                let detections = self
                    .detect_modalities_ordered(&text, chunk_index, &available_modalities, &mut carry)
                    .await;
                TextModalityOutput {
                    success: true,
                    modality_detections: Some(detections),
                    ..Default::default()
                }
            }

            TextModalityAction::LinkToModality {
                source_graph_id,
                target_graph_id,
                target_modality,
                relationship,
            } => {
                self.link_to_modality(
                    source_graph_id,
                    target_graph_id,
                    &target_modality,
                    relationship,
                )
                .await
            }
        };

        output.processing_time_ms = Some(start_time.elapsed().as_millis() as u64);
        output
    }

    /// Estimate token count (roughly 4 chars per token for English)
    fn estimate_tokens(text: &str) -> u32 {
        ((text.len() + 3) / 4) as u32
    }

    // ========================================================================
    // CHUNKING (no overlap — carry machines own boundaries)
    // ========================================================================

    /// Floor a byte index to the nearest UTF-8 char boundary at or before it.
    fn floor_to_char_boundary(text: &str, mut idx: usize) -> usize {
        if idx >= text.len() {
            return text.len();
        }
        while idx > 0 && !text.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }

    /// Ceiling a byte index to the nearest UTF-8 char boundary at or after it.
    fn ceil_to_char_boundary(text: &str, mut idx: usize) -> usize {
        if idx >= text.len() {
            return text.len();
        }
        while idx < text.len() && !text.is_char_boundary(idx) {
            idx += 1;
        }
        idx
    }

    /// Chunking happens ONCE on intake. NO overlap: boundaries between chunks
    /// are owned by the carry state machines (sentence fragments, open
    /// paragraphs, open sections, open modality blocks) — overlap duplicated
    /// content and forced LLM boundary resolution for no benefit.
    /// `max_chunk_chars` is sized by the caller (≈ 4 chars per token from the
    /// executor's 1/4-context budget) so prompts + structured responses
    /// always fit, even on 4K-context SLMs like BitNet.
    fn chunk_text(text: &str, max_chunk_chars: usize) -> Vec<RawChunk> {
        let mut chunks = Vec::new();
        if text.is_empty() {
            return chunks;
        }
        let max_chars = max_chunk_chars.max(256);
        let mut start = 0usize;
        let mut index = 0u32;

        while start < text.len() {
            let mut end = Self::floor_to_char_boundary(text, start.saturating_add(max_chars));
            if end <= start {
                end = text.len();
            }
            if end < text.len() {
                // Prefer breaking at a paragraph break, then a line break,
                // then a space — never mid-word.
                let window_start = Self::floor_to_char_boundary(text, end.saturating_sub(400));
                let window = &text[window_start..end];
                let para_break = window.rfind("\n\n").map(|i| window_start + i + 2);
                let line_break = window.rfind('\n').map(|i| window_start + i + 1);
                let space = window.rfind(' ').map(|i| window_start + i + 1);
                let candidate = para_break.or(line_break).or(space).filter(|&p| p > start);
                if let Some(b) = candidate {
                    end = b;
                } else {
                    end = Self::ceil_to_char_boundary(text, end);
                }
            }

            let slice = &text[start..end];
            chunks.push(RawChunk {
                index,
                text: slice.to_string(),
                token_count: Self::estimate_tokens(slice),
                start_char: start as u32,
                end_char: end as u32,
                is_complete_paragraph: slice.ends_with("\n\n"),
            });

            index += 1;
            start = end;
        }

        chunks
    }

    // ========================================================================
    // METERED EXECUTION + 5x YES/NO VALIDATION PRIMITIVES
    // ========================================================================

    /// Execute a pipeline call, metering tokens and call count. Every LLM
    /// interaction in this pipeline goes through here so the orchestrator's
    /// token budget stays truthful.
    async fn llm_execute(
        &self,
        pipeline_id: u64,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.llm_calls.fetch_add(1, Ordering::Relaxed);
        let result = self.executor.execute(pipeline_id, input).await?;
        if let Some(tokens) = result.get("tokens_used").and_then(|t| t.as_u64()) {
            self.llm_tokens_used.fetch_add(tokens, Ordering::Relaxed);
        }
        Ok(result)
    }

    /// Ask a strict one-word YES/NO question. Returns (answer, tokens_used).
    /// Standardized question/response forms come from the shared K-ALGORITHM
    /// contract (k_validation) — single source for host and pipelines.
    async fn ask_yes_no(&self, prompt: String) -> (Option<bool>, u64) {
        let input = k_validation::yes_no_input(&prompt);
        match self.llm_execute(9, input).await {
            Ok(result) => {
                let tokens = result.get("tokens_used").and_then(|t| t.as_u64()).unwrap_or(0);
                let raw = result.get("response").and_then(|r| r.as_str()).unwrap_or("{}");
                (k_validation::parse_yes_no(raw), tokens)
            }
            Err(_) => (None, 0),
        }
    }

    /// Require `n` CONSECUTIVE YES answers. Aborts on the first non-YES.
    async fn confirm_times_yes(&self, prompt: String, n: u32) -> bool {
        for _ in 0..n {
            let (answer, _) = self.ask_yes_no(prompt.clone()).await;
            match answer {
                Some(true) => continue,
                _ => return false,
            }
        }
        true
    }

    /// Require `n` CONSECUTIVE NO answers. Aborts on the first non-NO.
    async fn confirm_times_no(&self, prompt: String, n: u32) -> bool {
        for _ in 0..n {
            let (answer, _) = self.ask_yes_no(prompt.clone()).await;
            match answer {
                Some(false) => continue,
                _ => return false,
            }
        }
        true
    }

    // ========================================================================
    // SENTENCE MACHINERY (Path 2 core — 1x1 in order, dual 5x validation)
    // ========================================================================

    /// Extract the FIRST sentence (empty seen) or the sentence FOLLOWING the
    /// last captured one. O(1) prompt state: count + last sentence + its end
    /// offset — never the full list. This is what makes the loop BitNet-safe
    /// at 1/4-context chunk sizing: the prompt does not grow as the chunk is
    /// processed, so tokens are not eaten up or stacked.
    async fn extract_next_sentence(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        count_so_far: usize,
        last_original: Option<&str>,
        last_span_end: Option<usize>,
    ) -> (ExtractionOutcome, Option<CandidateSentence>) {
        let state_block = match (count_so_far, last_original, last_span_end) {
            (0, _, _) => "CURRENT STATE:\nNo sentences have been captured from this chunk yet.\n\nYOUR TASK:\nIdentify the FIRST sentence in the chunk below. A sentence may be a complete grammatical sentence or a fragment functioning as one.".to_string(),
            (n, Some(last), Some(end_off)) => format!(
                "CURRENT STATE:\n{n} sentences have already been captured from this chunk, in order.\nThe most recently captured sentence (order {n}) is:\n\"{last}\"\nIt ends at character offset {end_off}.\n\nYOUR TASK:\nIdentify the sentence that IMMEDIATELY FOLLOWS the sentence above in the chunk. Do not repeat it. Do not skip ahead."
            ),
            _ => "CURRENT STATE:\nNo sentences have been captured from this chunk yet.\n\nYOUR TASK:\nIdentify the FIRST sentence in the chunk below.".to_string(),
        };

        let next_order = (count_so_far + 1) as u32;
        let task_word = if count_so_far == 0 { "no" } else { "no further" };
        let prompt = format!(
            r#"You are performing sentence identification and grammar correction on a text chunk, one sentence at a time, in reading order.

YOUR ROLE:
This chunk is being decomposed into an ordered list of sentences. Each sentence you identify becomes a permanent node in a text graph, so the exactness of the text span and the order are critical. Sentences in the chunk may or may not be grammatically correct. You must return the sentence EXACTLY as it appears in the chunk (the original, character for character, even if ungrammatical), and separately return a corrected version. If the sentence is already grammatically correct, the corrected version is identical to the original. Do not merge two sentences. Do not split one sentence. Do not summarize. Do not skip punctuation.

{state_block}

CHUNK TEXT (chunk {chunk_index}):
{chunk_text}

Return ONLY valid JSON, no explanation, no markdown:
{{"found": true,
 "order": {next_order},
 "original_sentence": "<exact text, character for character>",
 "text_span_start": <char offset where it starts in the chunk>,
 "text_span_end": <char offset where it ends, exclusive>,
 "corrected_sentence": "<corrected version, or identical to the original if already correct>"}}
If {task_word} sentence exists: {{"found": false}}"#,
            state_block = state_block,
            chunk_index = chunk_index,
            chunk_text = &chunk_text[..Self::floor_to_char_boundary(chunk_text, chunk_text.len().min(8000))],
            next_order = next_order,
            task_word = task_word,
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 600,
            "temperature": 0.1,
            "system_context": "Sentence identification with grammar correction. Return only valid JSON. No explanation."
        });

        match self.llm_execute(9, input).await {
            Ok(result) => {
                let raw = result.get("response").and_then(|r| r.as_str()).unwrap_or("{}");
                let json_str = Self::extract_json_from_response(raw, '{', '}');
                let parsed: serde_json::Value =
                    serde_json::from_str(&json_str).unwrap_or(serde_json::json!({"found": false}));
                if !parsed.get("found").and_then(|f| f.as_bool()).unwrap_or(false) {
                    return (ExtractionOutcome::Exhausted, None);
                }
                let original = parsed
                    .get("original_sentence")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                if original.is_empty() {
                    return (ExtractionOutcome::Exhausted, None);
                }
                let span_start = Self::floor_to_char_boundary(
                    chunk_text,
                    parsed.get("text_span_start").and_then(|s| s.as_u64()).unwrap_or(0) as usize,
                );
                let span_end = Self::ceil_to_char_boundary(
                    chunk_text,
                    parsed.get("text_span_end").and_then(|s| s.as_u64()).unwrap_or(original.len() as u64) as usize,
                )
                .min(chunk_text.len());
                let corrected = parsed
                    .get("corrected_sentence")
                    .and_then(|s| s.as_str())
                    .unwrap_or(&original)
                    .to_string();
                (
                    ExtractionOutcome::Candidate,
                    Some(CandidateSentence {
                        order: next_order,
                        original,
                        span_start,
                        span_end,
                        corrected,
                    }),
                )
            }
            Err(_) => (ExtractionOutcome::Exhausted, None),
        }
    }

    /// Order validation: is the candidate EXACTLY the next sentence?
    /// 5 consecutive YES = validated. Any NO rejects the candidate (a
    /// candidate failing twice flags the chunk for escalation).
    async fn sentence_order_validation(
        &self,
        chunk_text: &str,
        last_original: Option<&str>,
        candidate: &CandidateSentence,
    ) -> bool {
        let (prev_block, question) = match last_original {
            Some(last) => (
                format!("PREVIOUSLY CAPTURED SENTENCE:\n\"{last}\"\n"),
                "is the candidate sentence the sentence that IMMEDIATELY FOLLOWS the previously captured sentence, with no other sentence between them and no part of either one missing or duplicated?",
            ),
            None => (
                String::new(),
                "is the candidate the FIRST sentence in the chunk, starting at its very beginning (ignoring leading whitespace)?",
            ),
        };
        let prompt = format!(
            r#"You are validating sentence extraction order.

CHUNK TEXT:
{chunk}

{prev_block}
CANDIDATE SENTENCE:
"{candidate}"

QUESTION: In the chunk above, {question}

Return ONLY valid JSON with a single one-word answer:
{{"answer": "YES"}} or {{"answer": "NO"}}"#,
            chunk = &chunk_text[..Self::floor_to_char_boundary(chunk_text, chunk_text.len().min(6000))],
            prev_block = prev_block,
            candidate = candidate.original,
            question = question,
        );
        self.confirm_times_yes(prompt, 5).await
    }

    /// Correction validation: does the corrected version preserve the
    /// original's meaning exactly while being grammatically correct?
    /// (Trivially YES when corrected == original — no call is made.)
    async fn sentence_correction_validation(&self, original: &str, corrected: &str) -> bool {
        if original == corrected {
            return true;
        }
        let prompt = format!(
            r#"You are validating a grammar correction.

ORIGINAL SENTENCE:
"{original}"

CORRECTED SENTENCE:
"{corrected}"

QUESTION: Is the corrected sentence a grammatically correct version of the original that preserves its meaning EXACTLY — no facts added, removed, or altered?

Return ONLY valid JSON with a single one-word answer:
{{"answer": "YES"}} or {{"answer": "NO"}}"#
        );
        self.confirm_times_yes(prompt, 5).await
    }

    /// Duplicate backstop — invoked ONLY when the deterministic span guard is
    /// ambiguous (e.g., after fragment carryover). We need 5 consecutive
    /// "NO, not a duplicate".
    async fn sentence_duplicate_validation(
        &self,
        seen: &[(u32, String)],
        candidate: &CandidateSentence,
    ) -> bool {
        let seen_list = seen
            .iter()
            .map(|(order, s)| format!("{}. {}", order, s))
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!(
            r#"You are validating that a newly extracted sentence is not a duplicate.

ALREADY CAPTURED SENTENCES (in order):
{seen_list}

CANDIDATE SENTENCE:
"{candidate}"

QUESTION: Does the candidate sentence ALREADY appear in the captured list above (same content, possibly with different punctuation or casing)?

Return ONLY valid JSON with a single one-word answer:
{{"answer": "YES"}} if it is a duplicate, {{"answer": "NO"}} if it is new."#,
            seen_list = if seen_list.is_empty() {
                "(none yet)".to_string()
            } else {
                seen_list
            },
            candidate = candidate.original,
        );
        for _ in 0..5 {
            let (answer, _) = self.ask_yes_no(prompt.clone()).await;
            match answer {
                Some(false) => continue, // not a duplicate — keep confirming
                _ => return false,
            }
        }
        true
    }

    /// The Path 2 sentence loop: extract → deterministic span guard →
    /// 5x order validation → 5x correction validation → OFFLOAD to
    /// SentenceNode → keep only {order, original} in the seen list → repeat
    /// until extraction returns exhausted (confirmed by 5x NO). Offloading
    /// keeps prompt state O(1) and prevents token stacking across the chunk.
    /// Returns (nodes, seen, fragment_carry(relative to the given text),
    /// completed_in_order).
    #[allow(clippy::too_many_arguments)]
    async fn extract_sentences_ordered(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        section_id: Option<u64>,
        paragraph_id: Option<u64>,
    ) -> (
        Vec<SentenceNode>,
        Vec<(u32, String)>,
        Option<(String, usize)>,
        bool,
    ) {
        let mut nodes: Vec<SentenceNode> = Vec::new();
        let mut seen: Vec<(u32, String)> = Vec::new(); // (order, original) — O(1) prompt state
        let mut last_original: Option<String> = None;
        let mut last_span_end: Option<usize> = None;
        let mut completed_in_order = false;
        let mut consecutive_exhausted = 0u32;
        let mut consecutive_rejections = 0u32;

        loop {
            let (outcome, candidate) = self
                .extract_next_sentence(
                    chunk_text,
                    chunk_index,
                    seen.len(),
                    last_original.as_deref(),
                    last_span_end,
                )
                .await;

            let cand = match (outcome, candidate) {
                (ExtractionOutcome::Exhausted, _) | (_, None) => {
                    consecutive_exhausted += 1;
                    if consecutive_exhausted < 2 {
                        continue; // one more extraction attempt before confirming
                    }
                    // Extraction says exhausted twice — confirm with 5x NO.
                    let last_block = match (&last_original, last_span_end) {
                        (Some(l), Some(e)) => format!(
                            "{} sentences have been captured from this chunk, in order. The last one (order {}) is: \"{}\", ending at offset {}.",
                            seen.len(),
                            seen.len(),
                            l,
                            e
                        ),
                        _ => "No sentences have been captured from this chunk yet.".to_string(),
                    };
                    let prompt = format!(
                        r#"You are confirming sentence extraction is complete.

{last_block}

QUESTION: Is there any sentence in this chunk after the last captured one that has not been captured? Look carefully for fragments, list items, or short statements.

Return ONLY valid JSON with a single one-word answer:
{{"answer": "YES"}} if an uncaptured sentence remains, {{"answer": "NO"}} if the chunk is fully captured."#,
                    );
                    if seen.is_empty() || self.confirm_times_no(prompt, 5).await {
                        completed_in_order = true;
                        break;
                    }
                    // The model insists something remains — keep extracting.
                    consecutive_exhausted = 0;
                    continue;
                }
                (_, Some(c)) => c,
            };

            // Deterministic guard 1: span monotonicity (free — zero LLM calls).
            // Any candidate starting before the last span's end is a
            // duplicate or regression.
            if let Some(prev_end) = last_span_end {
                if cand.span_start < prev_end {
                    continue; // discard silently, re-extract
                }
            }

            // Deterministic guard 2: exact original repeat.
            if seen.iter().any(|(_, s)| *s == cand.original) {
                continue;
            }

            // 5x order validation (the position axis).
            let order_ok = self
                .sentence_order_validation(chunk_text, last_original.as_deref(), &cand)
                .await;
            if !order_ok {
                consecutive_rejections += 1;
                if consecutive_rejections >= 2 {
                    tracing::warn!(
                        "chunk {}: sentence candidate rejected twice — flagging for escalation",
                        chunk_index
                    );
                    completed_in_order = !seen.is_empty();
                    break;
                }
                continue; // re-extract
            }
            consecutive_rejections = 0;

            // Duplicate backstop only when spans were ambiguous (fragment
            // carryover) — otherwise the span guard already decided.
            if last_span_end.is_none() && !seen.is_empty() {
                if !self.sentence_duplicate_validation(&seen, &cand).await {
                    continue;
                }
            }

            // 5x correction validation (the meaning axis). A failed
            // correction keeps the ORIGINAL — never an unvalidated rewrite.
            let correction_ok = self
                .sentence_correction_validation(&cand.original, &cand.corrected)
                .await;
            let corrected = if correction_ok {
                cand.corrected.clone()
            } else {
                cand.original.clone()
            };

            // OFFLOAD: everything except {order, original} goes to the node NOW.
            let node = SentenceNode {
                node_id: Self::generate_id(),
                node_type: GrammarNodeType::Sentence,
                content: corrected,
                original_content: cand.original.clone(),
                position: TextPosition {
                    start_offset: cand.span_start,
                    end_offset: cand.span_end,
                    line: None,
                    column: None,
                },
                chunk_id: chunk_index,
                chunk_offset: cand.span_start,
                paragraph_id,
                section_id,
                properties: GrammarProperties::default(),
                grammar_nodes: Vec::new(),
                grammar_relationships: Vec::new(),
                knowledge_refs: Vec::new(),
            };
            nodes.push(node);

            // Seen list keeps ONLY order + original.
            seen.push((cand.order, cand.original.clone()));
            last_original = Some(cand.original);
            last_span_end = Some(cand.span_end);
        }

        // Fragment carry: any non-whitespace tail after the last span is an
        // open sentence fragment — prepended to the next chunk (the caller
        // maps the relative offset to absolute).
        let fragment = last_span_end.and_then(|end| {
            if end < chunk_text.len() {
                let rest = &chunk_text[end..];
                let trimmed = rest.trim();
                if !trimmed.is_empty() {
                    let rel = rest.find(|c: char| !c.is_whitespace()).unwrap_or(0);
                    return Some((trimmed.to_string(), end + rel));
                }
            }
            None
        });

        (nodes, seen, fragment, completed_in_order)
    }

    // ========================================================================
    // SECTION MACHINERY (both paths — runs before path dispatch)
    // ========================================================================

    /// Extract ONE section event (1x1, in order) from `scan_offset` onward.
    /// The prompt EXPLAINS the state machine so the model knows how to diverge
    /// based on the state it is in. No document boundaries — ever.
    async fn extract_next_section_event(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        carry: &SectionCarryState,
        scan_offset: usize,
    ) -> Option<serde_json::Value> {
        let (state_name, open_desc, ancestors_desc) = match carry.state {
            SectionTrackingState::Idle => ("IDLE", "none".to_string(), "none".to_string()),
            SectionTrackingState::InSection | SectionTrackingState::InSubsection => {
                let cur = carry.current();
                (
                    if carry.state == SectionTrackingState::InSection {
                        "IN_SECTION"
                    } else {
                        "IN_SUBSECTION"
                    },
                    match cur {
                        Some(a) => format!("\"{}\" at level {}", a.title, a.level),
                        None => "none".to_string(),
                    },
                    carry
                        .ancestors
                        .iter()
                        .map(|a| format!("\"{}\"(L{})", a.title, a.level))
                        .collect::<Vec<_>>()
                        .join(" → "),
                )
            }
        };

        let prompt = format!(
            r#"You are tracking document SECTION structure across sequential text chunks, ONE structural event at a time, in reading order.

WHAT A SECTION IS:
A section is a coherent block of content that begins with a recognizable heading or structural marker, in ANY format. The formatting style is UNKNOWN — do not assume markdown. Markers include (non-exhaustive): markdown headers (#, ##), numbered headings (1., 2.1, IV.), ALL-CAPS heading lines, underlined or decorated headings (===, ---), bold lead lines, indented outline labels, or any consistent structural pattern present in the text itself.

HOW TRACKING WORKS (the state machine you are operating):
- IDLE: no section is open. You are looking for the FIRST marker in this chunk. Finding one opens a section.
- IN_SECTION (a section is open at level L): you are looking for the NEXT marker after the given position.
    · A marker at the SAME or HIGHER level than L closes the current section and opens a new one.
    · A marker at a LOWER level than L opens a SUBSECTION nested under the current section.
- IN_SUBSECTION: the same rules apply; a marker at the parent's level or higher closes the subsection (and any deeper nesting) and opens at that level.
- Sections NEVER close because of blank lines, style changes, or voice shifts. You NEVER report document boundaries — that is handled elsewhere on the graph.

CURRENT STATE:
- State: {state_name}
- Open section: {open_desc}
- Open ancestors (outermost → innermost): {ancestors_desc}
- Formatting pattern tracked so far: {pattern}
- Scan from character offset: {scan_offset}   (0 when starting this chunk)

YOUR TASK:
Scan the chunk below FROM that offset and report ONLY THE FIRST structural marker you find, or that no further marker exists in this chunk.

CHUNK TEXT (chunk {chunk_index}):
{chunk_text}

Return ONLY valid JSON:
{{"found": true,
 "event": {{
   "type": "section_start" | "subsection_start" | "section_close_and_start",
   "position": <char offset of the marker>,
   "title": "<heading text>",
   "level": <1-6>,
   "formatting_pattern": "<description of the marker pattern>"}}}}
or {{"found": false}}"#,
            state_name = state_name,
            open_desc = open_desc,
            ancestors_desc = ancestors_desc,
            pattern = carry.formatting_pattern.as_deref().unwrap_or("not yet detected"),
            scan_offset = scan_offset,
            chunk_index = chunk_index,
            chunk_text = &chunk_text[..Self::floor_to_char_boundary(chunk_text, chunk_text.len().min(8000))],
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 400,
            "temperature": 0.1,
            "system_context": "Section structure tracking. Return only valid JSON. No explanation."
        });

        match self.llm_execute(9, input).await {
            Ok(result) => {
                let raw = result.get("response").and_then(|r| r.as_str()).unwrap_or("{}");
                let json_str = Self::extract_json_from_response(raw, '{', '}');
                let parsed: serde_json::Value =
                    serde_json::from_str(&json_str).unwrap_or(serde_json::json!({"found": false}));
                if parsed.get("found").and_then(|f| f.as_bool()).unwrap_or(false) {
                    parsed.get("event").cloned()
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Validate a section event: 5x order ("is this the first marker at or
    /// after offset X?") then 5x classification ("is level L and type T
    /// correct for this marker, given the tracked pattern?").
    async fn validate_section_event(
        &self,
        chunk_text: &str,
        event: &serde_json::Value,
        scan_offset: usize,
        carry: &SectionCarryState,
    ) -> bool {
        let pos = event.get("position").and_then(|p| p.as_u64()).unwrap_or(0) as usize;
        let title = event.get("title").and_then(|t| t.as_str()).unwrap_or("");
        let level = event.get("level").and_then(|l| l.as_u64()).unwrap_or(1) as u8;
        let etype = event.get("type").and_then(|t| t.as_str()).unwrap_or("section_start");

        let order_prompt = format!(
            r#"You are validating section marker detection order.

CHUNK TEXT:
{chunk}

PROPOSED MARKER: "{title}" at character offset {pos} (level {level}).

QUESTION: Is this the FIRST structural marker in the chunk at or after offset {scan_offset}?

Return ONLY valid JSON with a single one-word answer:
{{"answer": "YES"}} or {{"answer": "NO"}}"#,
            chunk = &chunk_text[..Self::floor_to_char_boundary(chunk_text, chunk_text.len().min(6000))],
            title = title,
            pos = pos,
            level = level,
            scan_offset = scan_offset,
        );
        if !self.confirm_times_yes(order_prompt, 5).await {
            return false;
        }

        let class_prompt = format!(
            r#"You are validating a section marker's classification.

TRACKED STATE: open ancestors: {ancestors}, formatting pattern tracked so far: {pattern}
MARKER TEXT: "{title}" (proposed type {etype}, proposed level {level})

QUESTION: Given the tracked pattern and the structural conventions visible in this chunk, is the proposed type and level correct for this marker?

Return ONLY valid JSON with a single one-word answer:
{{"answer": "YES"}} or {{"answer": "NO"}}"#,
            ancestors = carry
                .ancestors
                .iter()
                .map(|a| format!("\"{}\"(L{})", a.title, a.level))
                .collect::<Vec<_>>()
                .join(" → "),
            pattern = carry.formatting_pattern.as_deref().unwrap_or("not yet detected"),
            title = title,
            etype = etype,
            level = level,
        );
        self.confirm_times_yes(class_prompt, 5).await
    }

    /// Apply a validated section event to the carry state and emit a
    /// SectionNode (parent populated from the ancestor stack so nesting lands
    /// directly in the graph).
    fn apply_section_event(
        &self,
        carry: &mut SectionCarryState,
        event: &serde_json::Value,
        _chunk_index: u32,
    ) -> Option<SectionNode> {
        let etype = match event.get("type").and_then(|t| t.as_str()) {
            Some("subsection_start") => SectionEventType::SubsectionStart,
            Some("section_close_and_start") => SectionEventType::SectionCloseAndStart,
            _ => SectionEventType::SectionStart,
        };
        let title = event
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        let level = event.get("level").and_then(|l| l.as_u64()).unwrap_or(1) as u8;
        let pos = event.get("position").and_then(|p| p.as_u64()).unwrap_or(0) as usize;
        let pattern = event
            .get("formatting_pattern")
            .and_then(|f| f.as_str())
            .map(String::from);

        let section_id = Self::generate_id();
        let parent_section = match etype {
            SectionEventType::SectionStart | SectionEventType::SectionCloseAndStart => {
                // Close/nest-exit: pop the stack until the new section sits at
                // the right level, then the new section's parent is whatever
                // remains on top.
                while carry
                    .ancestors
                    .last()
                    .map(|a| a.level >= level)
                    .unwrap_or(false)
                {
                    carry.ancestors.pop();
                }
                carry.ancestors.last().map(|a| a.section_id)
            }
            SectionEventType::SubsectionStart => carry.current().map(|a| a.section_id),
        };

        // Push the new section onto the stack.
        carry.ancestors.push(SectionAncestor {
            section_id,
            title: title.clone(),
            level,
        });
        carry.state = SectionTrackingState::InSection;
        if pattern.is_some() {
            carry.formatting_pattern = pattern.clone();
        }

        Some(SectionNode {
            node_id: section_id,
            node_type: GrammarNodeType::Section,
            content: title.clone(),
            level,
            formatting_pattern: pattern,
            parent_section,
            position: TextPosition {
                start_offset: pos,
                end_offset: pos + title.len(),
                line: None,
                column: None,
            },
        })
    }

    /// Per-chunk section tracking loop (both paths). Runs BEFORE path
    /// dispatch so the open-section id is available to sentence/paragraph
    /// nodes. 1x1 in order; each event dual-5x-validated; sections persist
    /// across chunks via the carry.
    async fn track_section_events(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        carry: &mut SectionCarryState,
    ) -> Vec<SectionNode> {
        let mut nodes = Vec::new();
        let mut scan_offset = 0usize;
        // Exhaustion discipline from the shared K-ALGORITHM contract.
        let mut exhausted =
            k_loops::ExhaustionTracker::new(k_loops::Ordered1x1Policy::default());

        loop {
            let event = self
                .extract_next_section_event(chunk_text, chunk_index, carry, scan_offset)
                .await;

            let Some(event) = event else {
                if exhausted.miss() {
                    break; // no further markers in this chunk
                }
                continue;
            };
            exhausted.hit();

            let pos = event.get("position").and_then(|p| p.as_u64()).unwrap_or(0) as usize;
            if pos < scan_offset {
                continue; // regression — discard
            }

            if self
                .validate_section_event(chunk_text, &event, scan_offset, carry)
                .await
            {
                if let Some(node) = self.apply_section_event(carry, &event, chunk_index) {
                    nodes.push(node);
                }
                scan_offset = Self::ceil_to_char_boundary(chunk_text, pos + 1);
            } else {
                // Rejected — advance past the marker to avoid a loop.
                scan_offset = Self::ceil_to_char_boundary(chunk_text, pos + 1);
            }
        }

        nodes
    }

    // ========================================================================
    // PARAGRAPH MACHINERY (Path 1 — prompt-based, 1x1 in order, carry-aware)
    // ========================================================================

    /// Extract ONE paragraph event (1x1, in order), carry-aware.
    async fn extract_next_paragraph(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        carry: &ParagraphCarryState,
        last_end: Option<usize>,
    ) -> Option<serde_json::Value> {
        let state_block = if carry.open {
            format!(
                "CURRENT STATE:\n- Open paragraph carried from previous chunk: YES\n  It began at absolute offset {}; its last known text ends with: \"...{}\"\n- Paragraphs already captured in this chunk: {}\n\nYOUR TASK:\nDetermine whether the start of this chunk CONTINUES the open paragraph. If it continues, report where it ENDS in this chunk (or that it remains open past this chunk's end). If it does not continue, report that, then find the next paragraph normally.",
                carry
                    .absolute_start
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                carry.tail.as_deref().unwrap_or(""),
                carry.paragraphs_found_total
            )
        } else {
            format!(
                "CURRENT STATE:\n- Open paragraph carried from previous chunk: NO\n- Paragraphs already captured in this chunk: {}\n- Most recent paragraph ended at offset: {}\n\nYOUR TASK:\nFind the NEXT paragraph starting after the given offset. Report its start, and its end if within this chunk — or that it remains open at the chunk's end.",
                carry.paragraphs_found_total,
                last_end
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "none".to_string())
            )
        };

        let prompt = format!(
            r#"You are detecting PARAGRAPH boundaries within a text chunk, one paragraph at a time, in reading order.

WHAT A PARAGRAPH IS:
A distinct block of prose separated from other blocks by blank lines, indentation conventions, or a clear break in the local point being developed. A paragraph groups consecutive sentences developing one local point.

CROSS-CHUNK BEHAVIOR:
Chunks are arbitrary windows over a larger text. A paragraph may begin in one chunk and end in the next. Most paragraphs do not span two chunks — but the LAST paragraph of a chunk is very often cut. The carry state below exists to capture that correctly.

{state_block}

CHUNK TEXT (chunk {chunk_index}):
{chunk_text}

Return ONLY valid JSON:
{{"found": true,
 "order": <1-based order within this chunk>,
 "continues_previous": true|false,
 "start": <char offset, or null if continuing the carried paragraph>,
 "end": <char offset, exclusive, or null if open at chunk end>,
 "open_at_chunk_end": true|false}}
or {{"found": false}} if no paragraph remains after the given position."#,
            state_block = state_block,
            chunk_index = chunk_index,
            chunk_text = &chunk_text[..Self::floor_to_char_boundary(chunk_text, chunk_text.len().min(8000))],
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 300,
            "temperature": 0.1,
            "system_context": "Paragraph boundary detection. Return only valid JSON. No explanation."
        });

        match self.llm_execute(9, input).await {
            Ok(result) => {
                let raw = result.get("response").and_then(|r| r.as_str()).unwrap_or("{}");
                let json_str = Self::extract_json_from_response(raw, '{', '}');
                let parsed: serde_json::Value =
                    serde_json::from_str(&json_str).unwrap_or(serde_json::json!({"found": false}));
                if parsed.get("found").and_then(|f| f.as_bool()).unwrap_or(false) {
                    Some(parsed)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Validate a paragraph event: 5x order, then 5x boundary (with the
    /// actual slice shown).
    async fn validate_paragraph(
        &self,
        chunk_text: &str,
        event: &serde_json::Value,
        scan_offset: usize,
    ) -> bool {
        let start = event.get("start").and_then(|s| s.as_u64()).map(|v| v as usize);
        let end = event.get("end").and_then(|s| s.as_u64()).map(|v| v as usize);

        let order_prompt = format!(
            r#"You are validating paragraph detection order.

CHUNK TEXT:
{chunk}

PROPOSED: paragraph {start_desc}{end_desc}

QUESTION: Is this the next paragraph boundary event in the chunk after offset {scan_offset}?

Return ONLY valid JSON: {{"answer": "YES"}} or {{"answer": "NO"}}"#,
            chunk = &chunk_text[..Self::floor_to_char_boundary(chunk_text, chunk_text.len().min(6000))],
            start_desc = start
                .map(|s| format!("starting at offset {}", s))
                .unwrap_or_else(|| "continuing the previously open paragraph".to_string()),
            end_desc = end
                .map(|e| format!(" ending at offset {}", e))
                .unwrap_or_else(|| " (open at chunk end)".to_string()),
            scan_offset = scan_offset,
        );
        if !self.confirm_times_yes(order_prompt, 5).await {
            return false;
        }

        if let (Some(s), Some(e)) = (start, end) {
            if e > s && e <= chunk_text.len() {
                let slice = &chunk_text[Self::floor_to_char_boundary(chunk_text, s)
                    ..Self::floor_to_char_boundary(chunk_text, e)];
                let boundary_prompt = format!(
                    r#"You are validating a paragraph boundary.

PROPOSED PARAGRAPH TEXT:
{slice}

QUESTION: Does the text above form ONE coherent paragraph — a single block developing one local point — with nothing extraneous included and nothing missing?

Return ONLY valid JSON: {{"answer": "YES"}} or {{"answer": "NO"}}"#
                );
                return self.confirm_times_yes(boundary_prompt, 5).await;
            }
        }
        true
    }

    /// Path 1 paragraph loop: 1x1 in order, carry-aware. Updates the carry
    /// and returns ParagraphNodes for this chunk (absolute positions).
    async fn detect_paragraphs_ordered(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        chunk_start_char: usize,
        carry: &mut ParagraphCarryState,
        section_id: Option<u64>,
    ) -> Vec<ParagraphNode> {
        let mut nodes = Vec::new();
        let mut last_end: Option<usize> = None;
        let mut exhausted =
            k_loops::ExhaustionTracker::new(k_loops::Ordered1x1Policy::default());

        loop {
            let event = self
                .extract_next_paragraph(chunk_text, chunk_index, carry, last_end)
                .await;

            let Some(event) = event else {
                if exhausted.miss() {
                    break;
                }
                continue;
            };
            exhausted.hit();

            if !self
                .validate_paragraph(chunk_text, &event, last_end.unwrap_or(0))
                .await
            {
                // Rejected — advance past the proposed start to avoid a loop.
                if let Some(s) = event.get("start").and_then(|s| s.as_u64()) {
                    last_end = Some(
                        Self::ceil_to_char_boundary(chunk_text, s as usize + 1)
                            .min(chunk_text.len()),
                    );
                }
                continue;
            }

            let continues = event
                .get("continues_previous")
                .and_then(|c| c.as_bool())
                .unwrap_or(false);
            let start = event.get("start").and_then(|s| s.as_u64()).map(|v| v as usize);
            let end = event.get("end").and_then(|s| s.as_u64()).map(|v| v as usize);
            let open = event
                .get("open_at_chunk_end")
                .and_then(|o| o.as_bool())
                .unwrap_or(false);

            carry.paragraphs_found_total += 1;

            let abs_start = if continues {
                carry.absolute_start.unwrap_or(chunk_start_char)
            } else {
                chunk_start_char + start.unwrap_or(0)
            };
            let abs_end = end.map(|e| chunk_start_char + e);

            nodes.push(ParagraphNode {
                node_id: Self::generate_id(),
                node_type: GrammarNodeType::Paragraph,
                sentence_count: 0, // populated when sentences are tied
                parent_section: section_id,
                position: TextPosition {
                    start_offset: abs_start,
                    end_offset: abs_end.unwrap_or(usize::MAX),
                    line: None,
                    column: None,
                },
            });

            // Update carry for the next chunk.
            carry.open = open;
            carry.absolute_start = if open { Some(abs_start) } else { None };
            carry.tail = if open {
                let ts = Self::floor_to_char_boundary(chunk_text, end.unwrap_or(0));
                let tail: String = chunk_text[ts..]
                    .chars()
                    .rev()
                    .take(60)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                Some(tail)
            } else {
                None
            };

            if open {
                break; // paragraph continues into the next chunk
            }
            last_end = end;
        }

        nodes
    }

    // ========================================================================
    // MODALITY MACHINERY (both paths — 1x1 in order, dual 5x validation)
    // ========================================================================

    /// Extract ONE modality occurrence (1x1, in order), carry-aware. Plain
    /// prose is NEVER reported — sentence extraction covers it exhaustively;
    /// true_text is derivable by subtraction, never detected.
    async fn extract_next_modality(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        available: &[String],
        carry: &ModalityCarryState,
        last_end: Option<usize>,
        order: u32,
    ) -> Option<serde_json::Value> {
        // Registry-driven list wins; empty list → builtin fallback so the
        // first analysis pass still detects non-prose content.
        let available: Vec<String> = if available.is_empty() {
            BUILTIN_MODALITY_NAMES.iter().map(|s| s.to_string()).collect()
        } else {
            available.to_vec()
        };

        let state_block = if carry.open {
            format!(
                "CURRENT STATE:\nAn OPEN non-prose block was carried from the previous chunk (modality '{}', no closing delimiter found there). Your FIRST task is to confirm where it ENDS in this chunk, or that it remains open.",
                carry.open_modality.as_deref().unwrap_or("?")
            )
        } else {
            format!(
                "CURRENT STATE:\n{} occurrence(s) captured in this chunk so far. Most recent: {}.",
                order.saturating_sub(1),
                match last_end {
                    Some(e) => format!("ended at character offset {}", e),
                    None => "none yet".to_string(),
                }
            )
        };

        let prompt = format!(
            r#"You are detecting embedded non-prose modality content within a text chunk, ONE occurrence at a time, in reading order.

WHAT YOU ARE LOOKING FOR:
Spans that are NOT plain prose — embedded code (any language), mathematical formulas or expressions (inline within a sentence or as blocks), chemical formulas, data tables, sequences, or any other listed modality. An occurrence may be as small as a short inline formula inside a sentence or as large as a multi-line block between sentences. Plain prose is NEVER reported.

AVAILABLE MODALITIES (report only these): {available}

{state_block}

YOUR TASK:
Find the FIRST modality occurrence in the chunk after character offset {scan}. Report exactly ONE occurrence. If none exists after that position, report not found.

CHUNK TEXT (chunk {chunk_index}):
{chunk_text}

Return ONLY valid JSON:
{{"found": true,
 "order": {order},
 "modality": "<one of the available modalities>",
 "span_start": <char offset>, "span_end": <char offset, exclusive>,
 "intent_reference": "contains" | "describes" | "references" | "mentions",
 "open_at_chunk_end": true|false}}
or {{"found": false}}"#,
            available = available.join(", "),
            state_block = state_block,
            scan = last_end.map(|e| e.to_string()).unwrap_or_else(|| "0".to_string()),
            chunk_index = chunk_index,
            chunk_text = &chunk_text[..Self::floor_to_char_boundary(chunk_text, chunk_text.len().min(8000))],
            order = order,
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 250,
            "temperature": 0.05,
            "system_context": "Modality occurrence detection. Return only valid JSON. No explanation."
        });

        match self.llm_execute(9, input).await {
            Ok(result) => {
                let raw = result.get("response").and_then(|r| r.as_str()).unwrap_or("{}");
                let json_str = Self::extract_json_from_response(raw, '{', '}');
                let parsed: serde_json::Value =
                    serde_json::from_str(&json_str).unwrap_or(serde_json::json!({"found": false}));
                if parsed.get("found").and_then(|f| f.as_bool()).unwrap_or(false) {
                    Some(parsed)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Validate a modality occurrence: 5x order, then 5x classification with
    /// the actual span content shown.
    async fn validate_modality(
        &self,
        chunk_text: &str,
        event: &serde_json::Value,
        scan_offset: usize,
    ) -> bool {
        let s = event.get("span_start").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let e = event.get("span_end").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let m = event.get("modality").and_then(|v| v.as_str()).unwrap_or("");

        let order_prompt = format!(
            r#"You are validating modality detection order.

CHUNK TEXT:
{chunk}

PROPOSED OCCURRENCE: modality '{m}' spanning characters {s}..{e}.

QUESTION: Is the span at {s}..{e} the FIRST modality occurrence after offset {scan} in this chunk?

Return ONLY valid JSON: {{"answer": "YES"}} or {{"answer": "NO"}}"#,
            chunk = &chunk_text[..Self::floor_to_char_boundary(chunk_text, chunk_text.len().min(6000))],
            m = m,
            s = s,
            e = e,
            scan = scan_offset,
        );
        if !self.confirm_times_yes(order_prompt, 5).await {
            return false;
        }

        let slice_start = Self::floor_to_char_boundary(chunk_text, s);
        let slice_end = Self::floor_to_char_boundary(chunk_text, e.min(chunk_text.len()));
        let slice = if slice_end > slice_start {
            &chunk_text[slice_start..slice_end]
        } else {
            ""
        };
        let class_prompt = format!(
            r#"You are validating a modality classification.

SPAN CONTENT:
{slice}

PROPOSED CLASSIFICATION: '{m}'

QUESTION: Is the content above correctly classified as modality '{m}' (and not plain prose, and not a different listed modality)?

Return ONLY valid JSON: {{"answer": "YES"}} or {{"answer": "NO"}}"#
        );
        self.confirm_times_yes(class_prompt, 5).await
    }

    /// Full per-chunk modality loop (both paths). Updates the carry; returns
    /// ordered detections with open_at_chunk_end set. Parent tying happens
    /// afterwards in `tie_modalities_to_parents`.
    async fn detect_modalities_ordered(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        available: &[String],
        carry: &mut ModalityCarryState,
    ) -> Vec<ChunkModalityDetection> {
        let mut detections = Vec::new();
        let mut last_end: Option<usize> = None;
        let mut order = 1u32;
        let mut exhausted =
            k_loops::ExhaustionTracker::new(k_loops::Ordered1x1Policy::default());

        loop {
            let event = self
                .extract_next_modality(chunk_text, chunk_index, available, carry, last_end, order)
                .await;

            let Some(event) = event else {
                if exhausted.miss() {
                    break;
                }
                continue;
            };
            exhausted.hit();

            let s = event.get("span_start").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let e = event.get("span_end").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if e <= s || e > chunk_text.len() {
                continue; // malformed — discard
            }

            if !self
                .validate_modality(chunk_text, &event, last_end.unwrap_or(0))
                .await
            {
                last_end = Some(
                    Self::ceil_to_char_boundary(chunk_text, s + 1).min(chunk_text.len()),
                );
                continue;
            }

            let open = event
                .get("open_at_chunk_end")
                .and_then(|o| o.as_bool())
                .unwrap_or(false);
            detections.push(ChunkModalityDetection {
                modality: event
                    .get("modality")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                span_start: Self::floor_to_char_boundary(chunk_text, s),
                span_end: Self::floor_to_char_boundary(chunk_text, e),
                intent_reference: event
                    .get("intent_reference")
                    .and_then(|v| v.as_str())
                    .unwrap_or("contains")
                    .to_string(),
                chunk_index,
                parent_node_id: None,
                open_at_chunk_end: open,
                needs_reparent: false,
            });

            if open {
                carry.open = true;
                carry.open_modality = detections.last().map(|d| d.modality.clone());
                break; // block continues into the next chunk
            }
            last_end = Some(e);
            order += 1;
        }

        detections
    }

    /// Tie modality detections to graph parents: inside a validated
    /// SentenceNode span → that sentence; else ParagraphNode when available;
    /// else the nearest preceding SentenceNode with needs_reparent=true
    /// (re-tied to its ParagraphNode in Phase 4). NEVER to the chunk.
    /// NOTE: `sentences`/`paragraphs` and detection spans must be in the SAME
    /// coordinate space when called.
    fn tie_modalities_to_parents(
        detections: &mut [ChunkModalityDetection],
        sentences: &[SentenceNode],
        paragraphs: &[ParagraphNode],
    ) {
        for d in detections.iter_mut() {
            if let Some(sent) = sentences.iter().find(|s| {
                d.span_start >= s.position.start_offset && d.span_end <= s.position.end_offset
            }) {
                d.parent_node_id = Some(sent.node_id);
                d.needs_reparent = false;
                continue;
            }
            if let Some(para) = paragraphs.iter().find(|p| {
                d.span_start >= p.position.start_offset
                    && (p.position.end_offset == usize::MAX || d.span_end <= p.position.end_offset)
            }) {
                d.parent_node_id = Some(para.node_id);
                d.needs_reparent = false;
                continue;
            }
            // Out-of-sentence with no paragraph yet (Path 2): interim tie to
            // the nearest preceding sentence with a re-parent flag.
            if let Some(sent) = sentences
                .iter()
                .filter(|s| s.position.start_offset <= d.span_start)
                .last()
            {
                d.parent_node_id = Some(sent.node_id);
                d.needs_reparent = true;
            } else {
                d.needs_reparent = true;
            }
        }
    }

    // ========================================================================
    // LEGACY GRAMMAR EXTRACTOR (kept as a selectable option — default path
    // is extract_grammar_from_graphs; never removed, never deprecated)
    // ========================================================================

    /// Legacy per-chunk grammar extraction. Analyzes one text span and
    /// returns a flat relationship list. Powers the ExtractGrammarRelationships
    /// action and PerSentence grammar mode.
    async fn extract_grammar_relationships_from_text(
        &self,
        text: &str,
        chunk_index: u32,
    ) -> Vec<ChunkGrammarRelationship> {
        let prompt = format!(
            r#"Analyze the grammatical and semantic relationships in this text.

Text:
{}

Return ONLY a valid JSON array:
[{{
  "from_text": "subject or source concept",
  "to_text": "object or target concept",
  "edge_type": "Performs|Affects|Implies|Contradicts|Elaborates|Summarizes|Supports|TemporalPrecedes|TemporalFollows|CausedBy|Enables|Prevents|PartOf|HasPart|FunctionalRole|InstanceOf|HasInstance|SimilarTo|DerivedFrom|VersionOf",
  "tense": "past|present|future|unknown",
  "negated": false,
  "verb": "the main verb",
  "verb_type": "action|linking|helping",
  "subject": "grammatical subject",
  "object": "grammatical object or null",
  "source_sentence_start": 0,
  "source_sentence_end": 100
}}]"#,
            &text[..Self::floor_to_char_boundary(text, text.len().min(3000))]
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 800,
            "temperature": 0.05,
            "system_context": "Grammar relationship extraction. Return only valid JSON array."
        });

        match self.llm_execute(9, input).await {
            Ok(result) => {
                let raw = result.get("response").and_then(|r| r.as_str()).unwrap_or("[]");
                let json_str = Self::extract_json_from_response(raw, '[', ']');
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| {
                        Some(ChunkGrammarRelationship {
                            from_text: v.get("from_text")?.as_str()?.to_string(),
                            to_text: v.get("to_text")?.as_str()?.to_string(),
                            edge_type: v
                                .get("edge_type")
                                .and_then(|e| e.as_str())
                                .unwrap_or("Affects")
                                .to_string(),
                            tense: v.get("tense").and_then(|t| t.as_str()).map(String::from),
                            negated: v.get("negated").and_then(|b| b.as_bool()).unwrap_or(false),
                            verb: v.get("verb").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                            verb_type: match v
                                .get("verb_type")
                                .and_then(|s| s.as_str())
                                .unwrap_or("action")
                            {
                                "linking" => VerbType::Linking,
                                "helping" => VerbType::Helping,
                                _ => VerbType::Action,
                            },
                            subject: v
                                .get("subject")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string(),
                            object: v.get("object").and_then(|s| s.as_str()).map(String::from),
                            source_sentence_start: v
                                .get("source_sentence_start")
                                .and_then(|n| n.as_u64())
                                .map(|n| n as usize),
                            source_sentence_end: v
                                .get("source_sentence_end")
                                .and_then(|n| n.as_u64())
                                .map(|n| n as usize),
                            chunk_index,
                        })
                    })
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    // ========================================================================
    // ZERO-SHOT LLM EXTRACTION METHODS
    // ========================================================================

    /// Clean a chunk via zero-shot LLM
    async fn clean_chunk(&self, chunk: RawChunk) -> TextModalityOutput {
        let clean_prompt = format!(
            r#"Clean and normalize the following text chunk. Fix spelling errors, grammar issues, formatting inconsistencies, and broken words from chunking. Preserve the original meaning exactly.

        TEXT TO CLEAN:
        {}

        Return ONLY valid JSON with no explanation, preamble, or markdown:
        {{"cleaned_text": "the fully cleaned text here"}}"#,
            chunk.text
        );

        let clean_input = serde_json::json!({
            "prompt": clean_prompt,
            "max_tokens": (chunk.token_count + 150) as u32,
            "temperature": 0.1,
            "system_context": "You are a text cleaning assistant. Return only valid JSON: {\"cleaned_text\": \"...\"}. No explanation. No markdown. No preamble."
        });

        match self.executor.execute(9, clean_input).await {
            Ok(result) => {
                let cleaned_text = result
                    .get("response")
                    .and_then(|r| r.as_str())
                    .map(|s| {
                        let json_str = Self::extract_json_from_response(s, '{', '}');
                        serde_json::from_str::<serde_json::Value>(&json_str)
                            .ok()
                            .and_then(|v| v.get("cleaned_text")?.as_str().map(|t| t.to_string()))
                            .unwrap_or_else(|| chunk.text.clone())
                    })
                    .unwrap_or_else(|| chunk.text.clone());

                TextModalityOutput {
                    success: true,
                    cleaned_text: Some(cleaned_text),
                    ..Default::default()
                }
            }
            Err(e) => {
                // Fallback to original text if LLM fails
                TextModalityOutput {
                    success: true,
                    cleaned_text: Some(chunk.text),
                    error: Some(format!("LLM cleaning failed, using original: {}", e)),
                    ..Default::default()
                }
            }
        }
    }

    /// Extract keywords from text via LLM (internal helper)
    async fn extract_keywords_from_text(&self, text: &str) -> Vec<String> {
        let prompt = format!(
            r#"Extract all important keywords and key phrases from this text.
Return as a JSON array of strings. Focus on: topics, concepts, named entities, technical terms.

TEXT:
{}

RESPOND ONLY WITH JSON ARRAY: ["keyword1", "keyword2", ...]"#,
            text
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 300,
            "temperature": 0.2,
            "system_context": "Extract keywords. Respond only with valid JSON array."
        });

        match self.executor.execute(9, input).await {
            Ok(result) => result
                .get("response")
                .and_then(|r| r.as_str())
                .and_then(|s| Self::parse_json_array(s))
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// Extract entities from text via LLM (internal helper)
    async fn extract_entities_from_text(&self, text: &str) -> Vec<ExtractedEntity> {
        let prompt = format!(
            r#"Extract named entities from this text.
Return as JSON array with objects: {{"text": "entity", "type": "PERSON|ORG|LOCATION|DATE|PRODUCT|EVENT|TECHNOLOGY|OTHER", "confidence": 0.0-1.0}}

TEXT:
{}

RESPOND ONLY WITH JSON ARRAY."#,
            text
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 500,
            "temperature": 0.2,
            "system_context": "Output only a valid JSON array. No explanation. No markdown code blocks. No preamble. Start directly with [."
        });

        match self.executor.execute(9, input).await {
            Ok(result) => result
                .get("response")
                .and_then(|r| r.as_str())
                .and_then(|s| {
                    let json_str = Self::extract_json_from_response(s, '[', ']');
                    serde_json::from_str::<Vec<serde_json::Value>>(&json_str).ok()
                })
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            Some(ExtractedEntity {
                                text: v.get("text")?.as_str()?.to_string(),
                                entity_type: v.get("type")?.as_str()?.to_string(),
                                confidence: v.get("confidence")?.as_f64()? as f32,
                                start_offset: None,
                                end_offset: None,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// Extract topics from text via LLM (internal helper)
    async fn extract_topics_from_text(&self, text: &str) -> Vec<String> {
        let prompt = format!(
            r#"What are the main topics/themes in this text?
Return as JSON array of topic strings.

TEXT:
{}

RESPOND ONLY WITH JSON ARRAY: ["topic1", "topic2", ...]"#,
            text
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 200,
            "temperature": 0.2,
            "system_context": "Output only a valid JSON array of strings. No explanation. No markdown. Start directly with [."
        });

        match self.executor.execute(9, input).await {
            Ok(result) => result
                .get("response")
                .and_then(|r| r.as_str())
                .and_then(|s| Self::parse_json_array(s))
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// Run an async extractor repeatedly until 5 consecutive passes find
    /// nothing new. K must be Eq + Hash. KEPT as a generic reusable 5x-stable
    /// utility (currently unused by the main paths — available for review
    /// and selective use; never removed, never deprecated).
    async fn extract_strings_until_stable<F, Fut>(
        &self,
        input: &str,
        extractor: F,
        max_rounds: usize,
    ) -> Vec<String>
    where
        F: Fn(String, Vec<String>) -> Fut,
        Fut: std::future::Future<Output = Vec<String>>,
    {
        let mut accumulated: Vec<String> = Vec::new();
        let mut no_new_consecutive = 0u32;
        let stable_threshold = 5u32;

        for _round in 0..max_rounds {
            let candidates = extractor(input.to_string(), accumulated.clone()).await;
            let existing: std::collections::HashSet<String> = accumulated.iter().cloned().collect();
            let truly_new: Vec<String> = candidates
                .into_iter()
                .filter(|s| !existing.contains(s))
                .collect();

            if truly_new.is_empty() {
                no_new_consecutive += 1;
                if no_new_consecutive >= stable_threshold {
                    break;
                }
            } else {
                no_new_consecutive = 0;
                accumulated.extend(truly_new);
            }
        }
        accumulated
    }

    /// Extract keywords via zero-shot LLM (public action)
    async fn extract_keywords_llm(&self, text: &str, max_keywords: usize) -> TextModalityOutput {
        let keywords_raw = self.extract_keywords_from_text(text).await;

        let keywords: Vec<Keyword> = keywords_raw
            .into_iter()
            .take(max_keywords)
            .enumerate()
            .map(|(i, term)| Keyword {
                term,
                frequency: 1,
                relevance: 1.0 - (i as f32 * 0.05).min(0.9),
                is_phrase: false,
            })
            .collect();

        TextModalityOutput {
            success: true,
            keywords: Some(keywords),
            ..Default::default()
        }
    }

    /// Create a ChunkGraph from a fully processed chunk.
    /// This is the persistent evidence structure for AMT building.
    fn create_chunk_graph(&self, chunk: &ProcessedChunk, root_graph_id: u64) -> ChunkGraph {
        let graph_id = Self::generate_id();
        let now = chrono::Utc::now().to_rfc3339();

        // Sentence boundaries come ONLY from validated SentenceNodes. There
        // is deliberately NO punctuation-scan fallback: an unvalidated
        // boundary claim is worse than an empty one — stale non-zero-shot
        // boundary detection is removed in full. A chunk whose extraction
        // produced no sentences simply reports no boundaries.
        let sentence_boundaries: Vec<SentenceBoundary> = chunk
            .sentence_nodes
            .iter()
            .map(|s| SentenceBoundary {
                start: s.position.start_offset,
                end: s.position.end_offset,
                sentence_type: SentenceType::Declarative,
            })
            .collect();

        let mut paragraph_breaks = Vec::new();
        let mut search_pos = 0;
        while let Some(found) = chunk.cleaned_text[search_pos..].find("\n\n") {
            let abs = search_pos + found;
            paragraph_breaks.push(abs);
            search_pos = abs + 2;
        }

        ChunkGraph {
            graph_id,
            chunk_index: chunk.index,
            prompt_start_char: chunk.prompt_start_char,
            prompt_end_char: chunk.prompt_end_char,
            sentence_boundaries,
            paragraph_breaks,
            cleaned_text: chunk.cleaned_text.clone(),
            overlap_resolution: None,
            keywords: chunk.keywords.clone(),
            topics: chunk.topics.clone(),
            sentence_nodes: chunk.sentence_nodes.clone(),
            paragraph_nodes: chunk.paragraph_nodes.clone(),
            section_nodes: chunk.section_nodes.clone(),
            document_nodes: chunk.document_nodes.clone(),
            cross_sentence_relationships: chunk.cross_sentence_relationships.clone(),
            coreference_chains: chunk.coreference_chains.clone(),
            modality_detections: chunk.detected_modalities.clone(),
            root_modality_list_contribution: chunk
                .detected_modalities
                .iter()
                .filter(|d| d.modality != "true_text" && d.modality != "unknown")
                .map(|d| d.modality.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect(),
            created_at: now,
        }
    }

    /// Extract entities via zero-shot LLM (public action)
    async fn extract_entities_llm(&self, text: &str) -> TextModalityOutput {
        let extracted = self.extract_entities_from_text(text).await;

        let entities: Vec<Entity> = extracted
            .into_iter()
            .map(|e| Entity {
                text: e.text,
                entity_type: EntityType::from_str(&e.entity_type),
                start_offset: e.start_offset.unwrap_or(0),
                end_offset: e.end_offset.unwrap_or(0),
                confidence: e.confidence,
                metadata: HashMap::new(),
            })
            .collect();

        TextModalityOutput {
            success: true,
            entities: Some(entities),
            ..Default::default()
        }
    }

    /// Extract topics via zero-shot LLM (public action)
    async fn extract_topics_llm(&self, text: &str) -> TextModalityOutput {
        let topics = self.extract_topics_from_text(text).await;

        TextModalityOutput {
            success: true,
            topics: Some(topics),
            ..Default::default()
        }
    }

    // ========================================================================
    // RECONSTRUCTION
    // ========================================================================

    /// Reconstruct full cleaned prompt from processed chunks, handling overlaps
    fn reconstruct_from_chunks(&self, chunks: &[ProcessedChunk]) -> TextModalityOutput {
        if chunks.is_empty() {
            return TextModalityOutput {
                success: true,
                reconstructed_text: Some(String::new()),
                ..Default::default()
            };
        }

        if chunks.len() == 1 {
            return TextModalityOutput {
                success: true,
                reconstructed_text: Some(chunks[0].cleaned_text.clone()),
                ..Default::default()
            };
        }

        let mut result = String::new();

        for (i, chunk) in chunks.iter().enumerate() {
            if i == 0 {
                result.push_str(&chunk.cleaned_text);
            } else {
                // Skip the overlapping portion
                let skip_chars = chunk.overlap_from_previous as usize;
                if chunk.cleaned_text.len() > skip_chars {
                    // Find a good boundary (space or newline) near the skip point
                    let text_bytes = chunk.cleaned_text.as_bytes();
                    let mut actual_skip = skip_chars;

                    // Look for space/newline within 50 chars of skip point
                    for j in skip_chars..=(skip_chars + 50).min(chunk.cleaned_text.len()) {
                        if j < text_bytes.len() && (text_bytes[j] == b' ' || text_bytes[j] == b'\n')
                        {
                            actual_skip = j + 1;
                            break;
                        }
                    }

                    if actual_skip < chunk.cleaned_text.len() {
                        // Add space if needed
                        if !result.ends_with(' ') && !result.ends_with('\n') {
                            result.push(' ');
                        }
                        result.push_str(&chunk.cleaned_text[actual_skip..]);
                    }
                }
            }
        }

        TextModalityOutput {
            success: true,
            reconstructed_text: Some(result),
            ..Default::default()
        }
    }

    // ========================================================================
    // ANALYSIS
    // ========================================================================

    /// Full analysis. Chunking happens ONCE (no overlap — carry machines own
    /// boundaries). Section tracking runs for BOTH prompt paths before
    /// dispatch (OMEX handles sections natively inside its single pass). The
    /// path decides thoroughness; the executor model can override to
    /// OMEX-native (single pass per chunk, deterministic verification,
    /// fallback to the LLM path on any verification failure).
    ///
    /// Phase 3 (grammar extraction) runs graph-native over ALL chunks after
    /// per-chunk processing — Phase 4 (Path 2 paragraph construction) depends
    /// on its edges, so Phase 3 always precedes Phase 4. The selectable
    /// `grammar_mode` routes Phase 3: GraphNative (default) extracts on the
    /// full graph; PerSentence uses the legacy per-chunk extractor on each
    /// sentence (no cross-sentence pass, no coreference — legacy behavior,
    /// kept for review and selective use). OMEX chunks skip per-sentence
    /// extraction (grammar arrives natively) but still participate in the
    /// pairwise cross-sentence pass where they lack edges.
    ///
    /// Metrics come FROM THE GRAPH: sentence_count = SentenceNode count,
    /// paragraph_count = ParagraphNode count, word_count from corrected
    /// sentences. No text splits — they would disagree with the validated
    /// nodes and create two competing sources of truth.
    #[allow(clippy::too_many_arguments)]
    async fn analyze_text(
        &self,
        text: &str,
        max_chunk_tokens: u32,
        depth: AnalysisDepth,
        extract_entities: bool,
        extract_topics: bool,
        available_modalities: &[String],
        processing_path: ProcessingPath,
        executor_model: ExecutorModelKind,
        grammar_mode: GrammarExtractionMode,
    ) -> TextModalityOutput {
        // ── Chunking: ≈4 chars/token from the caller's 1/4-context budget ──
        let max_chunk_chars = (max_chunk_tokens as usize).saturating_mul(4);
        let chunks = Self::chunk_text(text, max_chunk_chars);

        let mut all_processed_chunks: Vec<ProcessedChunk> = Vec::new();
        let mut all_modality_detections: Vec<ChunkModalityDetection> = Vec::new();
        let mut first_chunk_graph: Option<ChunkGraph> = None;

        // Cross-chunk carried state machines.
        let mut section_carry = SectionCarryState::default();
        let mut paragraph_carry = ParagraphCarryState::default();
        let mut sentence_carry = SentenceCarryState::default();
        let mut modality_carry = ModalityCarryState::default();

        // Effective route.
        let use_omex = executor_model == ExecutorModelKind::Omex
            || processing_path == ProcessingPath::OmexNative;
        let use_path2 = !use_omex && processing_path == ProcessingPath::Path2;

        for chunk in &chunks {
            let mut chunk_section_nodes: Vec<SectionNode> = Vec::new();

            let (mut processed, chunk_graph, modalities) = if use_omex {
                match self
                    .process_chunk_omex(chunk, available_modalities, &mut section_carry)
                    .await
                {
                    Ok((p, g, m, secs)) => {
                        chunk_section_nodes = secs;
                        (p, g, m)
                    }
                    Err(e) => {
                        tracing::warn!(
                            "OMEX path failed for chunk {} ({}); falling back to LLM path",
                            chunk.index,
                            e
                        );
                        let secs = self
                            .track_section_events(&chunk.text, chunk.index, &mut section_carry)
                            .await;
                        chunk_section_nodes = secs;
                        let sid = section_carry.current_section_id();
                        if use_path2 {
                            self.process_chunk_path2(
                                chunk,
                                available_modalities,
                                sid,
                                &mut sentence_carry,
                                &mut modality_carry,
                            )
                            .await
                        } else {
                            self.process_chunk_path1(
                                chunk,
                                available_modalities,
                                sid,
                                &mut paragraph_carry,
                                &mut modality_carry,
                            )
                            .await
                        }
                    }
                }
            } else {
                // Section tracking (BOTH prompt paths, before dispatch).
                let secs = self
                    .track_section_events(&chunk.text, chunk.index, &mut section_carry)
                    .await;
                chunk_section_nodes = secs;
                let sid = section_carry.current_section_id();
                if use_path2 {
                    self.process_chunk_path2(
                        chunk,
                        available_modalities,
                        sid,
                        &mut sentence_carry,
                        &mut modality_carry,
                    )
                    .await
                } else {
                    self.process_chunk_path1(
                        chunk,
                        available_modalities,
                        sid,
                        &mut paragraph_carry,
                        &mut modality_carry,
                    )
                    .await
                }
            };

            processed.section_nodes = chunk_section_nodes;
            all_modality_detections.extend(modalities);
            if first_chunk_graph.is_none() {
                first_chunk_graph = Some(chunk_graph);
            }
            all_processed_chunks.push(processed);
        }

        // ── Final-fragment healing (Path 2): a sentence fragment carried out
        //       of the LAST chunk has no next chunk to heal it — extract it
        //       now so no text is silently dropped. ──
        if !use_omex && use_path2 {
            if let (Some(frag), Some(abs_start)) = (
                sentence_carry.open_fragment.clone(),
                sentence_carry.fragment_absolute_start,
            ) {
                if !frag.trim().is_empty() {
                    if let Some(last_chunk) = all_processed_chunks.last_mut() {
                        let (mut nodes, _seen, _frag2, _ok) = self
                            .extract_sentences_ordered(&frag, last_chunk.index, None, None)
                            .await;
                        for s in nodes.iter_mut() {
                            s.position.start_offset = abs_start + s.position.start_offset;
                            s.position.end_offset = abs_start + s.position.end_offset;
                            s.chunk_offset = s
                                .position
                                .start_offset
                                .saturating_sub(last_chunk.start_offset as usize);
                            s.section_id =
                                last_chunk.section_nodes.last().map(|x| x.node_id);
                        }
                        last_chunk.sentence_nodes.extend(nodes);
                    }
                }
            }
        }

        // ── Final-paragraph close (Path 1): a paragraph left open at the end
        //       of input closes at the end of the text. ──
        if paragraph_carry.open {
            if let Some(last_chunk) = all_processed_chunks.last_mut() {
                if let Some(p) = last_chunk.paragraph_nodes.last_mut() {
                    if p.position.end_offset == usize::MAX {
                        p.position.end_offset = text.len();
                    }
                }
            }
        }

        // ── PHASE 3: Grammar extraction ──
        let all_processed_chunks = if grammar_mode == GrammarExtractionMode::PerSentence {
            // Legacy per-chunk extractor (selectable option): per-sentence
            // grammar into SentenceNode.grammar_relationships — no
            // cross-sentence pass, no coreference (legacy behavior).
            let mut chunks_out = all_processed_chunks;
            for chunk in chunks_out.iter_mut() {
                for sent in chunk.sentence_nodes.iter_mut() {
                    sent.grammar_relationships = self
                        .extract_grammar_relationships_from_text(&sent.content, chunk.index)
                        .await;
                }
            }
            chunks_out
        } else {
            let (chunks_out, _grammar_tokens) =
                self.extract_grammar_from_graphs(all_processed_chunks).await;
            chunks_out
        };

        // ── PHASE 4: Path 2 exclusive — construct paragraphs from the
        //       sentence graph via Stage 1 / Stage 2 ──
        let all_processed_chunks = if use_path2 {
            self.construct_structure_from_graphs(all_processed_chunks)
                .await
        } else {
            all_processed_chunks
        };

        // ── Metrics FROM THE GRAPH ──
        let total_sentence_nodes: usize = all_processed_chunks
            .iter()
            .map(|c| c.sentence_nodes.len())
            .sum();
        let total_paragraph_nodes: usize = all_processed_chunks
            .iter()
            .map(|c| c.paragraph_nodes.len())
            .sum();
        let mut word_count: usize = all_processed_chunks
            .iter()
            .flat_map(|c| {
                c.sentence_nodes
                    .iter()
                    .map(|s| s.content.split_whitespace().count())
            })
            .sum();
        if word_count == 0 {
            // Fallback when no sentences were extracted (executor
            // degradation): estimate from the raw text so downstream stages
            // never see a degenerate zero.
            word_count = text.split_whitespace().count();
        }
        let sentence_count = if total_sentence_nodes > 0 {
            total_sentence_nodes
        } else {
            text.split(|c| c == '.' || c == '!' || c == '?')
                .filter(|s| !s.trim().is_empty())
                .count()
        };
        let character_count: usize = all_processed_chunks
            .iter()
            .map(|c| c.cleaned_text.len())
            .sum();

        let cleaned_text = all_processed_chunks
            .iter()
            .map(|c| c.cleaned_text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Entities/topics via zero-shot LLM over the cleaned text.
        let entities = if extract_entities {
            let extracted = self.extract_entities_from_text(&cleaned_text).await;
            extracted
                .into_iter()
                .map(|e| Entity {
                    text: e.text,
                    entity_type: EntityType::from_str(&e.entity_type),
                    start_offset: e.start_offset.unwrap_or(0),
                    end_offset: e.end_offset.unwrap_or(0),
                    confidence: e.confidence,
                    metadata: HashMap::new(),
                })
                .collect()
        } else {
            Vec::new()
        };

        let topics_raw = if extract_topics {
            self.extract_topics_from_text(&cleaned_text).await
        } else {
            Vec::new()
        };
        let topics: Vec<Topic> = topics_raw
            .into_iter()
            .enumerate()
            .map(|(i, name)| Topic {
                name,
                keywords: Vec::new(),
                relevance: 1.0 - (i as f32 * 0.1).min(0.8),
                category: None,
            })
            .collect();

        // Keywords DERIVED FROM THE GRAPH: subject/object noun phrases and
        // knowledge-reference surfaces — not a separate extraction pass.
        let mut keyword_freq: HashMap<String, usize> = HashMap::new();
        for chunk in &all_processed_chunks {
            for sent in &chunk.sentence_nodes {
                for gr in &sent.grammar_relationships {
                    for side in [&gr.subject, &gr.from_text, &gr.to_text] {
                        let t = side.trim().to_lowercase();
                        if t.len() > 2 {
                            *keyword_freq.entry(t).or_insert(0) += 1;
                        }
                    }
                }
                for kr in &sent.knowledge_refs {
                    let t = kr.surface.trim().to_lowercase();
                    if t.len() > 2 {
                        *keyword_freq.entry(t).or_insert(0) += 1;
                    }
                }
            }
        }
        let mut keywords: Vec<Keyword> = keyword_freq
            .into_iter()
            .map(|(term, frequency)| Keyword {
                is_phrase: term.contains(' '),
                term,
                frequency,
                relevance: 1.0,
            })
            .collect();
        keywords.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        keywords.truncate(20);

        // Structure extraction is graph-native now; the rule-based form is
        // gone in full. Sections live on the chunk nodes; documents are
        // Phase-4-graph-traversal territory (not tracked here).
        let structure = DocumentStructure {
            sections: Vec::new(),
            has_title: false,
            has_abstract: false,
            has_toc: false,
            document_type: DocumentType::Unknown,
        };

        let analysis = TextAnalysisResult {
            word_count,
            sentence_count,
            paragraph_count: total_paragraph_nodes,
            character_count,
            entities,
            topics,
            keywords,
            structure,
            language: Some("en".to_string()),
            sentiment: if depth == AnalysisDepth::Deep || depth == AnalysisDepth::Comprehensive {
                self.analyze_sentiment(&cleaned_text).await
            } else {
                None
            },
            readability_score: None,
        };

        let (total_tokens, _) = self.llm_metrics_snapshot();

        TextModalityOutput {
            success: true,
            analysis: Some(analysis),
            processed_chunks: Some(all_processed_chunks.clone()),
            updated_chunks: Some(all_processed_chunks),
            modality_detections: Some(all_modality_detections),
            chunk_graph: first_chunk_graph,
            llm_tokens_used: Some(total_tokens),
            ..Default::default()
        }
    }

    // ========================================================================
    // PHASE 4 (Path 2) — paragraph construction FROM THE SENTENCE GRAPH
    // ========================================================================

    /// Construct ParagraphNodes from the assembled sentence graph. The
    /// Stage 1 / Stage 2 methodology:
    ///
    /// STAGE 1 — Candidate structural unit formation: accumulate pools over
    ///   evidence edges between consecutive sentences, in order. Evidence is
    ///   graph-native: shared coreference chains, explicit cross-sentence
    ///   relationship edges, and shared grammar subjects (normalized).
    ///
    /// STAGE 2 — Structural boundary evaluation: a boundary between adjacent
    ///   pools is confirmed when NO evidence of continuity exists. Weak or
    ///   ambiguous cases (shared subject but nothing else) are confirmed by a
    ///   5x zero-shot YES/NO check with the pools' structured evidence shown.
    ///   Stage 2 does NOT identify section elements — the per-chunk tracker
    ///   already found those markers; Stage 2 validates whether content pools
    ///   sit within the bounds of what was detected, versus a broken run-off,
    ///   and promotes boundaries for structures that have no reliable surface
    ///   markers (paragraphs now; documents later, once the full relationship
    ///   graph exists).
    ///
    /// Sentences are then tied to their ParagraphNodes, paragraphs to the
    /// open SectionNodes, and interim-tied modality references are re-parented.
    async fn construct_structure_from_graphs(
        &self,
        mut chunks: Vec<ProcessedChunk>,
    ) -> Vec<ProcessedChunk> {
        // Global sentence index: (global order, chunk idx, sentence idx).
        let mut global: Vec<(usize, usize, usize)> = Vec::new();
        for (ci, chunk) in chunks.iter().enumerate() {
            for (si, _) in chunk.sentence_nodes.iter().enumerate() {
                global.push((global.len(), ci, si));
            }
        }
        if global.len() < 2 {
            return chunks;
        }

        let subject_at = |ci: usize, si: usize| -> String {
            chunks[ci].sentence_nodes[si]
                .grammar_relationships
                .first()
                .map(|g| g.subject.trim().to_lowercase())
                .unwrap_or_default()
        };

        // ── STAGE 1: pools by evidence, in order ──
        // Union-find over sentence global indices.
        let n = global.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut Vec<usize>, mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        fn union(p: &mut Vec<usize>, a: usize, b: usize) {
            let ra = find(p, a);
            let rb = find(p, b);
            if ra != rb {
                p[rb] = ra;
            }
        }

        // Evidence lookup helpers (pre-extracted to avoid closure borrows).
        let chunk_of: Vec<usize> = global.iter().map(|(_, ci, _)| *ci).collect();
        let sent_of: Vec<usize> = global.iter().map(|(_, _, si)| *si).collect();
        let node_ids: Vec<u64> = (0..n)
            .map(|i| chunks[chunk_of[i]].sentence_nodes[sent_of[i]].node_id)
            .collect();

        let has_cross_rel = |a: usize, b: usize| -> bool {
            let (na, nb) = (node_ids[a], node_ids[b]);
            chunks[chunk_of[a]]
                .cross_sentence_relationships
                .iter()
                .any(|r| {
                    (r.from_sentence_id == na && r.to_sentence_id == nb)
                        || (r.from_sentence_id == nb && r.to_sentence_id == na)
                })
        };
        let shares_coref = |a: usize, b: usize| -> bool {
            let (na, nb) = (node_ids[a], node_ids[b]);
            (0..chunks.len()).any(|ci| {
                chunks[ci].coreference_chains.iter().any(|c| {
                    let in_a = c.mentions.iter().any(|m| m.sentence_id == na);
                    let in_b = c.mentions.iter().any(|m| m.sentence_id == nb);
                    in_a && in_b
                })
            })
        };

        for i in 0..n.saturating_sub(1) {
            let j = i + 1;
            let subj_a = subject_at(chunk_of[i], sent_of[i]);
            let subj_b = subject_at(chunk_of[j], sent_of[j]);
            let strong = has_cross_rel(i, j) || shares_coref(i, j);
            let weak_shared_subject = !subj_a.is_empty() && subj_a == subj_b;
            if strong || weak_shared_subject {
                union(&mut parent, i, j);
            }
        }

        let mut pools: Vec<Vec<usize>> = Vec::new();
        {
            let mut map: HashMap<usize, Vec<usize>> = HashMap::new();
            for i in 0..n {
                let r = find(&mut parent, i);
                map.entry(r).or_default().push(i);
            }
            let mut keys: Vec<usize> = map.keys().cloned().collect();
            keys.sort_by_key(|k| map[k].first().cloned().unwrap_or(usize::MAX));
            for k in keys {
                pools.push(map[&k].clone());
            }
        }

        // ── STAGE 2: boundary evaluation between adjacent pools ──
        // A shared subject with NO other evidence is ambiguous — confirm via
        // 5x zero-shot; otherwise the boundary stands.
        let mut i = 0usize;
        while i + 1 < pools.len() {
            let a = &pools[i];
            let b = &pools[i + 1];
            let any_strong = a.iter().any(|&x| {
                b.iter().any(|&y| has_cross_rel(x, y) || shares_coref(x, y))
            });
            if any_strong {
                // Strong continuity — merge without a call.
                let b_take = pools.remove(i + 1);
                pools[i].extend(b_take);
                continue;
            }
            let subj_a = subject_at(chunk_of[a[0]], sent_of[a[0]]);
            let subj_b = subject_at(chunk_of[b[0]], sent_of[b[0]]);
            if !subj_a.is_empty() && subj_a == subj_b {
                // Ambiguous — show the structured evidence and confirm 5x.
                let a_summary = a
                    .iter()
                    .map(|&x| {
                        format!("  - {}", chunks[chunk_of[x]].sentence_nodes[sent_of[x]].content)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let b_summary = b
                    .iter()
                    .map(|&x| {
                        format!("  - {}", chunks[chunk_of[x]].sentence_nodes[sent_of[x]].content)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let prompt = format!(
                    r#"You are evaluating whether two sentence groups belong to the SAME paragraph — one locally developed point.

GROUP A (in reading order):
{a_summary}

GROUP B (in reading order):
{b_summary}

Both groups share the grammatical subject "{subj}" but no explicit relationship edges connect them.

QUESTION: Does the evidence indicate these two groups form ONE paragraph (a single locally developed point), or is there a paragraph boundary between them?

Return ONLY valid JSON with a single one-word answer:
{{"answer": "YES"}} if ONE paragraph, {{"answer": "NO"}} if a boundary exists."#,
                    a_summary = a_summary,
                    b_summary = b_summary,
                    subj = subj_a,
                );
                if self.confirm_times_yes(prompt, 5).await {
                    let b_take = pools.remove(i + 1);
                    pools[i].extend(b_take);
                    continue;
                }
            }
            i += 1;
        }

        // ── Promotion: pools → ParagraphNodes; tie sentences + sections ──
        // Walk pools in order; each pool's absolute span is derived from its
        // member sentences' positions; paragraph.parent_section is the
        // section whose span most recently covers the pool start (sections
        // were detected per-chunk with persistent ids).
        let mut all_section_nodes: Vec<(u64, usize, usize)> = Vec::new(); // (id, abs_start, level)
        for (ci, chunk) in chunks.iter().enumerate() {
            for s in &chunk.section_nodes {
                // Section positions are chunk-local start + title len; the
                // authoritative span is refined on the graph later. Store the
                // chunk-relative start mapped through the chunk's offset.
                let abs_start = chunk.start_offset as usize + s.position.start_offset;
                all_section_nodes.push((s.node_id, abs_start, s.level as usize));
            }
        }
        all_section_nodes.sort_by_key(|(_, start, _)| *start);

        let mut new_paragraphs_by_chunk: HashMap<usize, Vec<ParagraphNode>> = HashMap::new();
        for pool in &pools {
            if pool.is_empty() {
                continue;
            }
            let first = &global[pool[0]];
            let last = &global[*pool.last().unwrap()];
            let abs_start = chunks[first.1].sentence_nodes[first.2].position.start_offset;
            let abs_end = chunks[last.1].sentence_nodes[last.2].position.end_offset;

            // Interim parent section: the last section starting at or before
            // the pool start (refined later on the graph).
            let parent_section = all_section_nodes
                .iter()
                .filter(|(_, start, _)| *start <= abs_start)
                .last()
                .map(|(id, _, _)| *id);

            let para = ParagraphNode {
                node_id: Self::generate_id(),
                node_type: GrammarNodeType::Paragraph,
                sentence_count: pool.len() as u32,
                parent_section,
                position: TextPosition {
                    start_offset: abs_start,
                    end_offset: abs_end,
                    line: None,
                    column: None,
                },
            };

            // Tie member sentences to this paragraph + count.
            let ci = first.1;
            for &gi in pool {
                let (_, c, s) = global[gi];
                chunks[c].sentence_nodes[s].paragraph_id = Some(para.node_id);
            }
            new_paragraphs_by_chunk.entry(ci).or_default().push(para);
        }

        // Insert constructed paragraphs into their owning chunks (Path 2
        // chunks had none) and re-parent interim-tied modalities.
        for (ci, paras) in new_paragraphs_by_chunk {
            let chunk = &mut chunks[ci];
            chunk.paragraph_nodes = paras;
            // Re-tie modalities in ABSOLUTE space (sentence positions in the
            // orchestrator-facing structures are absolute per this pipeline's
            // analyze path; here within the pipeline they are chunk-local, so
            // convert paragraph spans to chunk-local for tying).
            let chunk_start = chunk.start_offset as usize;
            let paras_local: Vec<ParagraphNode> = chunk
                .paragraph_nodes
                .iter()
                .map(|p| {
                    let mut q = p.clone();
                    q.position.start_offset = q.position.start_offset.saturating_sub(chunk_start);
                    q.position.end_offset = if q.position.end_offset == usize::MAX {
                        usize::MAX
                    } else {
                        q.position.end_offset.saturating_sub(chunk_start)
                    };
                    q
                })
                .collect();
            let sents_snapshot: Vec<SentenceNode> = chunk.sentence_nodes.clone();
            Self::tie_modalities_to_parents(
                &mut chunk.detected_modalities,
                &sents_snapshot,
                &paras_local,
            );
            // Clear re-parent flags now that paragraphs exist.
            for d in chunk.detected_modalities.iter_mut() {
                if d.parent_node_id.is_some() {
                    d.needs_reparent = false;
                }
            }
        }

        chunks
    }

    /// Path 1: Deconstructor — top-down within the chunk.
    /// (Section tracking was already performed by the caller.)
    /// 1. Paragraphs 1x1 (prompt-based, carry-aware)
    /// 2. Whole-chunk clean (zero-shot, full chunk in / full chunk out)
    /// 3. Sentence identification within paragraphs — ONE listing call per
    ///    paragraph (a capable model handles the whole span at once), spans
    ///    anchored to the ORIGINAL chunk text; deterministic monotonic guard
    /// 4. Modality occurrences 1x1 in order (dual 5x validation)
    /// 5. Nodes assembled: ParagraphNodes + SentenceNodes tied by span
    async fn process_chunk_path1(
        &self,
        chunk: &RawChunk,
        available_modalities: &[String],
        section_id: Option<u64>,
        paragraph_carry: &mut ParagraphCarryState,
        modality_carry: &mut ModalityCarryState,
    ) -> (ProcessedChunk, ChunkGraph, Vec<ChunkModalityDetection>) {
        let chunk_start = chunk.start_char as usize;

        // ── 1. Paragraphs ──
        let paragraphs = self
            .detect_paragraphs_ordered(
                &chunk.text,
                chunk.index,
                chunk_start,
                paragraph_carry,
                section_id,
            )
            .await;

        // ── 2. Whole-chunk clean ──
        let clean_output = self.clean_chunk(chunk.clone()).await;
        let cleaned_text = clean_output
            .cleaned_text
            .unwrap_or_else(|| chunk.text.clone());

        // ── 3. Sentences: one listing call per paragraph (or whole chunk
        //       when none were detected) ──
        let mut sentence_nodes: Vec<SentenceNode> = Vec::new();

        let listing_targets: Vec<(Option<u64>, usize, usize)> = if paragraphs.is_empty() {
            vec![(None, 0, chunk.text.len())]
        } else {
            paragraphs
                .iter()
                .map(|p| {
                    let s = if p.position.start_offset >= chunk_start {
                        p.position.start_offset - chunk_start
                    } else {
                        0
                    };
                    let e = if p.position.end_offset == usize::MAX {
                        chunk.text.len()
                    } else if p.position.end_offset >= chunk_start {
                        (p.position.end_offset - chunk_start).min(chunk.text.len())
                    } else {
                        chunk.text.len()
                    };
                    (Some(p.node_id), s.min(chunk.text.len()), e.max(s))
                })
                .collect()
        };

        for (para_id, s, e) in listing_targets {
            let span_text = &chunk.text[s..e];
            let listed = self
                .list_sentences_for_paragraph(span_text, chunk.index)
                .await;
            for (orig, corrected, rel_s, rel_e) in listed {
                sentence_nodes.push(SentenceNode {
                    node_id: Self::generate_id(),
                    node_type: GrammarNodeType::Sentence,
                    content: corrected,
                    original_content: orig,
                    position: TextPosition {
                        start_offset: s + rel_s,
                        end_offset: s + rel_e.min(e - s),
                        line: None,
                        column: None,
                    },
                    chunk_id: chunk.index,
                    chunk_offset: s + rel_s,
                    paragraph_id: para_id,
                    section_id,
                    properties: GrammarProperties::default(),
                    grammar_nodes: Vec::new(),
                    grammar_relationships: Vec::new(),
                    knowledge_refs: Vec::new(),
                });
            }
        }

        // ── 4. Modalities (1x1 in order, dual 5x) ──
        let mut modalities = self
            .detect_modalities_ordered(&chunk.text, chunk.index, available_modalities, modality_carry)
            .await;
        // Tie in CHUNK-LOCAL coordinates (paragraphs are absolute — convert).
        let paragraphs_local: Vec<ParagraphNode> = paragraphs
            .iter()
            .map(|p| {
                let mut q = p.clone();
                q.position.start_offset = q.position.start_offset.saturating_sub(chunk_start);
                q.position.end_offset = if q.position.end_offset == usize::MAX {
                    usize::MAX
                } else {
                    q.position.end_offset.saturating_sub(chunk_start)
                };
                q
            })
            .collect();
        Self::tie_modalities_to_parents(&mut modalities, &sentence_nodes, &paragraphs_local);

        // ── 5. Assemble ──
        let processed = ProcessedChunk {
            index: chunk.index,
            original_text: chunk.text.clone(),
            cleaned_text,
            start_offset: chunk.start_char,
            end_offset: chunk.end_char,
            prompt_start_char: chunk_start,
            prompt_end_char: chunk.end_char as usize,
            token_count: chunk.token_count,
            keywords: Vec::new(),
            entities: Vec::new(),
            topics: Vec::new(),
            overlap_from_previous: 0,
            overlap_to_next: 0,
            sentence_nodes: sentence_nodes.clone(),
            paragraph_nodes: paragraphs,
            section_nodes: Vec::new(), // attached by the caller (analyze_text)
            document_nodes: Vec::new(),
            cross_sentence_relationships: Vec::new(),
            coreference_chains: Vec::new(),
            detected_modalities: modalities.clone(),
            chunk_graph_id: None,
        };

        let chunk_graph = self.create_chunk_graph(&processed, 0);
        (processed, chunk_graph, modalities)
    }

    /// One listing call per paragraph: identify ALL sentences in the span,
    /// in order, with spans and corrections. Capable models (Path 1) handle
    /// a full paragraph reliably; the deterministic monotonic-span guard
    /// still applies, and any mis-ordered span is dropped rather than kept.
    async fn list_sentences_for_paragraph(
        &self,
        span_text: &str,
        chunk_index: u32,
    ) -> Vec<(String, String, usize, usize)> {
        let prompt = format!(
            r#"You are identifying and grammar-correcting ALL sentences in a text span, in reading order.

RULES:
- Return the sentences in order, each EXACTLY as it appears in the span (original, character for character, even if ungrammatical).
- Provide a corrected version for each. If already correct, corrected == original.
- Spans are character offsets WITHIN the span provided, half-open [start, end).
- Do not merge or split sentences. Do not skip punctuation.

SPAN TEXT (chunk {chunk_index}):
{span_text}

Return ONLY valid JSON:
{{"sentences": [
  {{"order": 1, "original_sentence": "...", "span_start": 0, "span_end": 40, "corrected_sentence": "..."}}
]}}"#,
            chunk_index = chunk_index,
            span_text = &span_text[..Self::floor_to_char_boundary(span_text, span_text.len().min(8000))],
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 2000,
            "temperature": 0.1,
            "system_context": "Sentence listing with grammar correction. Return only valid JSON. No explanation."
        });

        match self.llm_execute(9, input).await {
            Ok(result) => {
                let raw = result.get("response").and_then(|r| r.as_str()).unwrap_or("[]");
                let json_str = Self::extract_json_from_response(raw, '{', '}');
                let parsed: serde_json::Value = serde_json::from_str(&json_str)
                    .unwrap_or(serde_json::json!({"sentences": []}));
                let mut out = Vec::new();
                let mut prev_end = 0usize;
                if let Some(arr) = parsed.get("sentences").and_then(|s| s.as_array()) {
                    for sv in arr {
                        let orig = sv
                            .get("original_sentence")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        if orig.is_empty() {
                            continue;
                        }
                        let s = Self::floor_to_char_boundary(
                            span_text,
                            sv.get("span_start").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                        );
                        let e = Self::ceil_to_char_boundary(
                            span_text,
                            sv.get("span_end")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(orig.len() as u64) as usize,
                        )
                        .min(span_text.len());
                        // Deterministic monotonic guard (free): drop regressions.
                        if s < prev_end {
                            continue;
                        }
                        prev_end = e;
                        let corrected = sv
                            .get("corrected_sentence")
                            .and_then(|s| s.as_str())
                            .unwrap_or(&orig)
                            .to_string();
                        out.push((orig, corrected, s, e));
                    }
                }
                out
            }
            Err(_) => Vec::new(),
        }
    }

    /// Path 2: Constructor — bottom-up within the chunk.
    /// (Section tracking was already performed by the caller.)
    /// 1. Sentences 1x1 in order (dual 5x validation, O(1) prompt state,
    ///    immediate SentenceNode offload, fragment carry)
    /// 2. Modalities 1x1 in order (dual 5x validation, carry)
    /// 3. NO per-chunk paragraphs — Phase 4 constructs them from the
    ///    sentence graph after all chunks are processed (Stage 1 pools →
    ///    Stage 2 boundaries), which is exactly what a constrained SLM
    ///    cannot be trusted to judge per-chunk but the graph can support.
    async fn process_chunk_path2(
        &self,
        chunk: &RawChunk,
        available_modalities: &[String],
        section_id: Option<u64>,
        sentence_carry: &mut SentenceCarryState,
        modality_carry: &mut ModalityCarryState,
    ) -> (ProcessedChunk, ChunkGraph, Vec<ChunkModalityDetection>) {
        let chunk_start = chunk.start_char as usize;

        // Effective text = carried fragment (if any) + chunk text.
        let (effective_text, fragment_abs, fragment_len) = match &sentence_carry.open_fragment {
            Some(frag) => {
                let mut t = frag.clone();
                t.push_str(&chunk.text);
                let flen = frag.len();
                (t, sentence_carry.fragment_absolute_start, flen)
            }
            None => (chunk.text.clone(), None, 0),
        };
        // Map effective-text offsets → absolute prompt offsets.
        let map_abs = |off: usize| -> usize {
            if off < fragment_len {
                fragment_abs.unwrap_or(chunk_start) + off
            } else {
                chunk_start + (off - fragment_len)
            }
        };

        // ── 1. Sentences 1x1 (positions returned in effective-text coords) ──
        let (sentence_nodes_raw, _seen, fragment_out, _completed) = self
            .extract_sentences_ordered(&effective_text, chunk.index, section_id, None)
            .await;

        let sentence_nodes: Vec<SentenceNode> = sentence_nodes_raw
            .into_iter()
            .map(|mut s| {
                s.position.start_offset = map_abs(s.position.start_offset);
                s.position.end_offset = map_abs(s.position.end_offset);
                s.chunk_offset = s.position.start_offset.saturating_sub(chunk_start);
                s
            })
            .collect();

        // Update the sentence carry for the next chunk (absolute anchoring).
        *sentence_carry = match fragment_out {
            Some((frag, rel_start)) => SentenceCarryState {
                open_fragment: Some(frag),
                fragment_absolute_start: Some(map_abs(rel_start)),
            },
            None => SentenceCarryState::default(),
        };

        // ── 2. Modalities 1x1 (spans in effective-text coords → absolute) ──
        let mut modalities = self
            .detect_modalities_ordered(&effective_text, chunk.index, available_modalities, modality_carry)
            .await;
        for d in modalities.iter_mut() {
            d.span_start = map_abs(d.span_start);
            d.span_end = map_abs(d.span_end);
        }
        Self::tie_modalities_to_parents(&mut modalities, &sentence_nodes, &[]);

        // ── 3. Cleaned text (sentence nodes carry the authoritative text) ──
        let cleaned_text = sentence_nodes
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let processed = ProcessedChunk {
            index: chunk.index,
            original_text: chunk.text.clone(),
            cleaned_text,
            start_offset: chunk.start_char,
            end_offset: chunk.end_char,
            prompt_start_char: chunk_start,
            prompt_end_char: chunk.end_char as usize,
            token_count: chunk.token_count,
            keywords: Vec::new(),
            entities: Vec::new(),
            topics: Vec::new(),
            overlap_from_previous: 0,
            overlap_to_next: 0,
            sentence_nodes: sentence_nodes.clone(),
            paragraph_nodes: Vec::new(),
            section_nodes: Vec::new(), // attached by the caller (analyze_text)
            document_nodes: Vec::new(),
            cross_sentence_relationships: Vec::new(),
            coreference_chains: Vec::new(),
            detected_modalities: modalities.clone(),
            chunk_graph_id: None,
        };

        let chunk_graph = self.create_chunk_graph(&processed, 0);
        (processed, chunk_graph, modalities)
    }

    /// PHASE 3 — Grammar extraction on the FULL graph (all chunks processed,
    /// complete sentence/paragraph/section hierarchy assembled). Ensures
    /// cross-sentence relationships can form across any sentence in the
    /// corpus, not just within a single chunk.
    ///
    ///  1. Assign globally unique sentence ids (chunk-local ids collide).
    ///  2. Per-sentence grammar extraction → SentenceNode.grammar_nodes AND
    ///     SentenceNode.grammar_relationships (tied to the sentence node
    ///     itself — grammar is local to each sentence). Skipped for sentences
    ///     that already carry grammar (OMEX native).
    ///  3. Pairwise cross-sentence comparison (bounded forward window) →
    ///     CrossSentenceRelationship stored on the chunk owning the
    ///     from-sentence, referenced by GLOBAL sentence id.
    ///  4. Coreference chains stored on the chunk owning the first mention.
    ///
    /// Returns the updated chunks and the tokens consumed by this phase.
    /// (The legacy per-chunk extractor remains available as a selectable
    /// option: extract_grammar_relationships_from_text / GrammarExtractionMode::PerSentence.)
    async fn extract_grammar_from_graphs(
        &self,
        mut chunks: Vec<ProcessedChunk>,
    ) -> (Vec<ProcessedChunk>, u64) {
        let (t0, _) = self.llm_metrics_snapshot();

        // ── 1. Globally unique sentence ids ──
        for chunk in chunks.iter_mut() {
            for sent in chunk.sentence_nodes.iter_mut() {
                sent.node_id = Self::generate_id();
            }
        }
        let all_sentences: Vec<(u64, String)> = chunks
            .iter()
            .flat_map(|c| {
                c.sentence_nodes
                    .iter()
                    .map(|s| (s.node_id, s.content.clone()))
            })
            .collect();

        // ── 2. Per-sentence grammar extraction ──
        for chunk in chunks.iter_mut() {
            for sent in chunk.sentence_nodes.iter_mut() {
                if !sent.grammar_relationships.is_empty() || !sent.grammar_nodes.is_empty() {
                    continue; // OMEX native — already extracted
                }
                let prompt = format!(
                    r#"You are extracting grammatical structure from a single sentence.

SENTENCE: "{}"

TASK: Identify the grammatical components and their relationships.

Return ONLY valid JSON:
{{
  "subject": {{
    "text": "the subject noun phrase",
    "position_start": 0,
    "position_end": 10,
    "entity_type_hint": "Person|Organization|Location|Document|Time|Concept|Unknown"
  }},
  "verb": {{
    "text": "the main verb",
    "position_start": 11,
    "position_end": 20,
    "verb_type": "action|linking|helping",
    "tense": "past|present|future|unknown",
    "negated": false
  }},
  "object": {{
    "text": "the object noun phrase or null",
    "position_start": 21,
    "position_end": 35,
    "entity_type_hint": "..."
  }},
  "modifiers": [
    {{
      "text": "modifier text",
      "position_start": 36,
      "position_end": 45,
      "modifier_type": "temporal|adjectival|adverbial|prepositional",
      "modifies": "subject|verb|object"
    }}
  ],
  "sentence_type": "declarative|interrogative|imperative|fragment",
  "edge_type": "Performs|Affects|Implies|Contradicts|Elaborates|Summarizes|Supports|TemporalPrecedes|TemporalFollows|CausedBy|Enables|Prevents|PartOf|HasPart|FunctionalRole|InstanceOf|HasInstance|SimilarTo|DerivedFrom|VersionOf"
}}"#,
                    sent.content
                );

                let input = serde_json::json!({
                    "prompt": prompt,
                    "max_tokens": 800,
                    "temperature": 0.05,
                    "system_context": "Grammar extraction. Return only valid JSON. No explanation."
                });

                if let Ok(response) = self.llm_execute(9, input).await {
                    let raw = response
                        .get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("{}");
                    let json_str = Self::extract_json_from_response(raw, '{', '}');
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        let mut grammar_nodes = Vec::new();
                        let mut relationships: Vec<ChunkGrammarRelationship> = Vec::new();

                        let subject_text = parsed
                            .pointer("/subject/text")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();
                        let verb_text = parsed
                            .pointer("/verb/text")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();
                        let object_text = parsed
                            .pointer("/object/text")
                            .and_then(|t| t.as_str())
                            .map(String::from);
                        let tense = parsed
                            .pointer("/verb/tense")
                            .and_then(|t| t.as_str())
                            .map(String::from);
                        let negated = parsed
                            .pointer("/verb/negated")
                            .and_then(|n| n.as_bool())
                            .unwrap_or(false);
                        let verb_type_str = parsed
                            .pointer("/verb/verb_type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("action")
                            .to_string();
                        let edge_type = parsed
                            .get("edge_type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("Performs")
                            .to_string();

                        if !subject_text.is_empty() {
                            grammar_nodes.push(GrammarNode {
                                node_id: 0,
                                node_type: GrammarNodeType::Subject,
                                text: subject_text.clone(),
                                position: TextPosition {
                                    start_offset: parsed
                                        .pointer("/subject/position_start")
                                        .and_then(|p| p.as_u64())
                                        .unwrap_or(0) as usize,
                                    end_offset: parsed
                                        .pointer("/subject/position_end")
                                        .and_then(|p| p.as_u64())
                                        .unwrap_or(0) as usize,
                                    line: None,
                                    column: None,
                                },
                                children: Vec::new(),
                                properties: GrammarProperties::default(),
                            });
                        }
                        if !verb_text.is_empty() {
                            let v_type = match verb_type_str.as_str() {
                                "linking" => GrammarNodeType::LinkingVerb,
                                "helping" => GrammarNodeType::HelpingVerb,
                                _ => GrammarNodeType::MainVerb,
                            };
                            grammar_nodes.push(GrammarNode {
                                node_id: 0,
                                node_type: v_type,
                                text: verb_text.clone(),
                                position: TextPosition {
                                    start_offset: parsed
                                        .pointer("/verb/position_start")
                                        .and_then(|p| p.as_u64())
                                        .unwrap_or(0) as usize,
                                    end_offset: parsed
                                        .pointer("/verb/position_end")
                                        .and_then(|p| p.as_u64())
                                        .unwrap_or(0) as usize,
                                    line: None,
                                    column: None,
                                },
                                children: Vec::new(),
                                properties: GrammarProperties {
                                    tense: tense.clone(),
                                    polarity: Some(negated.to_string()),
                                    ..Default::default()
                                },
                            });
                        }
                        if let Some(obj) = &object_text {
                            if !obj.is_empty() {
                                grammar_nodes.push(GrammarNode {
                                    node_id: 0,
                                    node_type: GrammarNodeType::DirectObject,
                                    text: obj.clone(),
                                    position: TextPosition {
                                        start_offset: parsed
                                            .pointer("/object/position_start")
                                            .and_then(|p| p.as_u64())
                                            .unwrap_or(0) as usize,
                                        end_offset: parsed
                                            .pointer("/object/position_end")
                                            .and_then(|p| p.as_u64())
                                            .unwrap_or(0) as usize,
                                        line: None,
                                        column: None,
                                    },
                                    children: Vec::new(),
                                    properties: GrammarProperties::default(),
                                });
                            }
                        }
                        if let Some(mods) = parsed.get("modifiers").and_then(|m| m.as_array()) {
                            for m_val in mods {
                                let m_type = match m_val
                                    .get("modifier_type")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("adverbial")
                                {
                                    "temporal" => GrammarNodeType::AdverbOfTime,
                                    "adjectival" => GrammarNodeType::Adjective,
                                    "prepositional" => GrammarNodeType::PrepositionalPhrase,
                                    _ => GrammarNodeType::Adverb,
                                };
                                grammar_nodes.push(GrammarNode {
                                    node_id: 0,
                                    node_type: m_type,
                                    text: m_val
                                        .get("text")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    position: TextPosition {
                                        start_offset: m_val
                                            .get("position_start")
                                            .and_then(|p| p.as_u64())
                                            .unwrap_or(0) as usize,
                                        end_offset: m_val
                                            .get("position_end")
                                            .and_then(|p| p.as_u64())
                                            .unwrap_or(0) as usize,
                                        line: None,
                                        column: None,
                                    },
                                    children: Vec::new(),
                                    properties: GrammarProperties::default(),
                                });
                            }
                        }

                        // The relationship IS the sentence's grammar — tied
                        // to the SentenceNode, not a chunk-level bag.
                        if !subject_text.is_empty() && !verb_text.is_empty() {
                            relationships.push(ChunkGrammarRelationship {
                                from_text: subject_text.clone(),
                                to_text: object_text.clone().unwrap_or_default(),
                                edge_type: edge_type.clone(),
                                tense: tense.clone(),
                                negated,
                                verb: verb_text.clone(),
                                verb_type: match verb_type_str.as_str() {
                                    "linking" => VerbType::Linking,
                                    "helping" => VerbType::Helping,
                                    _ => VerbType::Action,
                                },
                                subject: subject_text,
                                object: object_text,
                                source_sentence_start: Some(sent.position.start_offset),
                                source_sentence_end: Some(sent.position.end_offset),
                                chunk_index: sent.chunk_id,
                            });
                        }

                        sent.grammar_nodes = grammar_nodes;
                        sent.grammar_relationships = relationships;
                    }
                }
            }
        }

        // ── 3. Pairwise cross-sentence relationships (by GLOBAL id, bounded
        //       forward window — Stage 1 context pools in the orchestrator
        //       expand outward from these seeds) ──
        if all_sentences.len() > 1 {
            // Forward-window reach from the shared pairwise contract.
            let window = k_loops::PairwisePolicy::default().forward_window;
            let mut rel_results: Vec<CrossSentenceRelationship> = Vec::new();
            let mut chain_results: Vec<CoreferenceChain> = Vec::new();

            for (i, (from_id, from_text)) in all_sentences.iter().enumerate() {
                let to_slice: Vec<(u64, String)> = all_sentences
                    .iter()
                    .skip(i + 1)
                    .take(window)
                    .cloned()
                    .collect();
                if to_slice.is_empty() {
                    continue;
                }
                let sentence_list = to_slice
                    .iter()
                    .map(|(id, t)| format!("[id {}] {}", id, t))
                    .collect::<Vec<_>>()
                    .join("\n");

                let prompt = format!(
                    r#"You are analyzing relationships between a sentence and its following sentences in a text corpus.

SOURCE SENTENCE (id {from_id}):
"{from_text}"

FOLLOWING SENTENCES:
{sentence_list}

TASK: For each following sentence, identify a relationship to the source if one exists.

Relationship types:
- Elaborates, Causes, Enables, Prevents, Contradicts, Exemplifies, Summarizes, TemporalPrecedes, Coreference, PartOf, SimilarTo

Return ONLY valid JSON:
{{
  "relationships": [
    {{"to_id": <id>, "relationship_type": "...", "evidence": "brief quote showing the relationship"}}
  ],
  "coreference_mentions": [
    {{"to_id": <id>, "text": "the referring expression", "grammar_role": "subject|object|modifier|other"}}
  ]
}}
If nothing relates, return empty arrays."#,
                    from_id = from_id,
                    from_text = &from_text[..Self::floor_to_char_boundary(from_text, from_text.len().min(1200))],
                    sentence_list = sentence_list,
                );

                let input = serde_json::json!({
                    "prompt": prompt,
                    "max_tokens": 900,
                    "temperature": 0.1,
                    "system_context": "Cross-sentence relationship analysis. Return only valid JSON."
                });

                if let Ok(response) = self.llm_execute(9, input).await {
                    let raw = response
                        .get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("{}");
                    let json_str = Self::extract_json_from_response(raw, '{', '}');
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        if let Some(rels) = parsed.get("relationships").and_then(|r| r.as_array()) {
                            for rel in rels {
                                let to_id = rel.get("to_id").and_then(|v| v.as_u64()).unwrap_or(0);
                                if to_id == 0 {
                                    continue;
                                }
                                rel_results.push(CrossSentenceRelationship {
                                    from_sentence_id: *from_id,
                                    to_sentence_id: to_id,
                                    relationship_type: rel
                                        .get("relationship_type")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("Elaborates")
                                        .to_string(),
                                    evidence: rel
                                        .get("evidence")
                                        .and_then(|e| e.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                });
                            }
                        }
                        if let Some(mentions) =
                            parsed.get("coreference_mentions").and_then(|m| m.as_array())
                        {
                            let coref_mentions: Vec<CoreferenceMention> =
                                std::iter::once(CoreferenceMention {
                                    sentence_id: *from_id,
                                    text: from_text.clone(),
                                    grammar_role: "subject".to_string(),
                                })
                                .chain(mentions.iter().filter_map(|mv| {
                                    Some(CoreferenceMention {
                                        sentence_id: mv.get("to_id").and_then(|v| v.as_u64())?,
                                        text: mv.get("text").and_then(|v| v.as_str())?.to_string(),
                                        grammar_role: mv
                                            .get("grammar_role")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("subject")
                                            .to_string(),
                                    })
                                }))
                                .collect();
                            if coref_mentions.len() > 1 {
                                chain_results.push(CoreferenceChain {
                                    chain_id: Self::generate_id(),
                                    canonical_form: from_text.clone(),
                                    mentions: coref_mentions,
                                });
                            }
                        }
                    }
                }
            }

            // Distribute: relationship on the chunk owning the from-sentence
            // (deduped — OMEX chunks may already carry native edges);
            // coreference chain on the chunk owning the first mention.
            for rel in rel_results {
                if let Some(chunk) = chunks
                    .iter_mut()
                    .find(|c| c.sentence_nodes.iter().any(|s| s.node_id == rel.from_sentence_id))
                {
                    let dup = chunk.cross_sentence_relationships.iter().any(|r| {
                        r.from_sentence_id == rel.from_sentence_id
                            && r.to_sentence_id == rel.to_sentence_id
                            && r.relationship_type == rel.relationship_type
                    });
                    if !dup {
                        chunk.cross_sentence_relationships.push(rel);
                    }
                }
            }
            for chain in chain_results {
                if let Some(first_id) = chain.mentions.first().map(|m| m.sentence_id) {
                    if let Some(chunk) = chunks
                        .iter_mut()
                        .find(|c| c.sentence_nodes.iter().any(|s| s.node_id == first_id))
                    {
                        chunk.coreference_chains.push(chain);
                    }
                }
            }
        }

        let (t1, _) = self.llm_metrics_snapshot();
        (chunks, t1.saturating_sub(t0))
    }

    // ========================================================================
    // OMEX NATIVE PATH — one forward pass per chunk
    // ========================================================================

    /// OMEX-native processing: the GrammarParser emits the ENTIRE chunk graph
    /// in a single non-autoregressive pass — sentences with spans and
    /// corrections, grammar trees, section events with levels, embedded
    /// modality spans, cross-sentence relationships, coreference chains, and
    /// knowledge references. We run the SAME deterministic verification the
    /// granular paths enforce (monotonic spans, tree containment) and map
    /// into our node structures. On any verification failure the caller
    /// falls back to the LLM path. Phase 3 is skipped for OMEX chunks — the
    /// parser emits grammar natively.
    async fn process_chunk_omex(
        &self,
        chunk: &RawChunk,
        available_modalities: &[String],
        section_carry: &mut SectionCarryState,
    ) -> Result<
        (
            ProcessedChunk,
            ChunkGraph,
            Vec<ChunkModalityDetection>,
            Vec<SectionNode>,
        ),
        String,
    > {
        let input = serde_json::json!({
            "text": chunk.text,
            "chunk_index": chunk.index,
            "language": "en",
            "available_modalities": available_modalities,
            "carry": {
                "section_state": section_carry.state,
                "open_ancestors": section_carry.ancestors,
                "formatting_pattern": section_carry.formatting_pattern,
            },
            "options": {
                "include_grammar_trees": true,
                "include_corrections": true,
                "include_knowledge_refs": true,
                "include_section_events": true,
                "include_modality_spans": true,
                "include_cross_sentence": true
            }
        });

        let parsed = self
            .llm_execute(OMEX_TEXT_PARSER_PIPELINE_ID, input)
            .await
            .map_err(|e| format!("OMEX parser failed: {}", e))?;

        // ── Sentences + deterministic verification ──
        let mut sentence_nodes: Vec<SentenceNode> = Vec::new();
        let mut prev_end = 0usize;
        if let Some(arr) = parsed.get("sentences").and_then(|s| s.as_array()) {
            for sv in arr {
                let orig = sv
                    .get("original_sentence")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                if orig.is_empty() {
                    continue;
                }
                let s = Self::floor_to_char_boundary(
                    &chunk.text,
                    sv.get("span_start").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                );
                let e = Self::ceil_to_char_boundary(
                    &chunk.text,
                    sv.get("span_end")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(orig.len() as u64) as usize,
                )
                .min(chunk.text.len());
                if s < prev_end {
                    return Err("OMEX output: sentence spans not monotonic".to_string());
                }
                prev_end = e;

                let corrected = sv
                    .get("corrected_sentence")
                    .and_then(|s| s.as_str())
                    .unwrap_or(&orig)
                    .to_string();

                // Grammar tree → GrammarNode tree (containment verified).
                let grammar_nodes = sv
                    .get("grammar_tree")
                    .and_then(|t| Self::json_to_grammar_node(t, &corrected))
                    .map(|n| vec![n])
                    .unwrap_or_default();

                let knowledge_refs = sv
                    .get("knowledge_refs")
                    .and_then(|k| k.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|kv| {
                                Some(KnowledgeRef {
                                    span_start: kv.get("span_start").and_then(|v| v.as_u64())? as usize,
                                    span_end: kv.get("span_end").and_then(|v| v.as_u64())? as usize,
                                    surface: kv.get("surface").and_then(|v| v.as_str())?.to_string(),
                                    knowledge_kind: kv
                                        .get("knowledge_kind")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("concept")
                                        .to_string(),
                                    topic_path: kv
                                        .get("topic_path")
                                        .and_then(|p| p.as_array())
                                        .map(|a| {
                                            a.iter()
                                                .filter_map(|x| x.as_str().map(String::from))
                                                .collect()
                                        })
                                        .unwrap_or_default(),
                                    confidence: kv
                                        .get("confidence")
                                        .and_then(|v| v.as_f64())
                                        // Absence of claim → 0.0 (no evidence
                                        // captured), never a fabricated mid-score.
                                        .unwrap_or(0.0) as f32,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                sentence_nodes.push(SentenceNode {
                    node_id: Self::generate_id(),
                    node_type: GrammarNodeType::Sentence,
                    content: corrected,
                    original_content: orig,
                    position: TextPosition {
                        start_offset: s,
                        end_offset: e,
                        line: None,
                        column: None,
                    },
                    chunk_id: chunk.index,
                    chunk_offset: s,
                    paragraph_id: None,
                    section_id: section_carry.current_section_id(),
                    properties: GrammarProperties::default(),
                    grammar_nodes,
                    grammar_relationships: Vec::new(),
                    knowledge_refs,
                });
            }
        }
        if sentence_nodes.is_empty() {
            return Err("OMEX output: no sentences parsed".to_string());
        }

        // Section events → the SAME state machine the prompt path uses.
        let mut section_nodes = Vec::new();
        if let Some(events) = parsed.get("section_events").and_then(|e| e.as_array()) {
            for ev in events {
                if let Some(node) = self.apply_section_event(section_carry, ev, chunk.index) {
                    section_nodes.push(node);
                }
            }
        }
        // Re-tie sentence section ids to the final stack state.
        let sid = section_carry.current_section_id();
        for s in sentence_nodes.iter_mut() {
            if s.section_id.is_none() {
                s.section_id = sid;
            }
        }

        // Modality spans → detections.
        let mut modalities: Vec<ChunkModalityDetection> = parsed
            .get("modality_spans")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|mv| {
                        Some(ChunkModalityDetection {
                            modality: mv.get("modality").and_then(|v| v.as_str())?.to_string(),
                            span_start: Self::floor_to_char_boundary(
                                &chunk.text,
                                mv.get("span_start").and_then(|v| v.as_u64())? as usize,
                            ),
                            span_end: Self::floor_to_char_boundary(
                                &chunk.text,
                                mv.get("span_end").and_then(|v| v.as_u64())? as usize,
                            ),
                            intent_reference: mv
                                .get("intent_reference")
                                .and_then(|v| v.as_str())
                                .unwrap_or("contains")
                                .to_string(),
                            chunk_index: chunk.index,
                            parent_node_id: None,
                            open_at_chunk_end: mv
                                .get("open_at_chunk_end")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            needs_reparent: false,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self::tie_modalities_to_parents(&mut modalities, &sentence_nodes, &[]);

        // Cross-sentence relationships + coreference (OMEX emits natively).
        let cross_rels: Vec<CrossSentenceRelationship> = parsed
            .get("cross_sentence_relationships")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|rv| {
                        Some(CrossSentenceRelationship {
                            from_sentence_id: rv.get("from_sentence_id").and_then(|v| v.as_u64())?,
                            to_sentence_id: rv.get("to_sentence_id").and_then(|v| v.as_u64())?,
                            relationship_type: rv
                                .get("relationship_type")
                                .and_then(|v| v.as_str())?
                                .to_string(),
                            evidence: rv
                                .get("evidence")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let coref_chains: Vec<CoreferenceChain> = parsed
            .get("coreference_chains")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|cv| {
                        Some(CoreferenceChain {
                            chain_id: cv.get("chain_id").and_then(|v| v.as_u64())?,
                            canonical_form: cv
                                .get("canonical_form")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            mentions: cv
                                .get("mentions")
                                .and_then(|m| m.as_array())
                                .map(|ms| {
                                    ms.iter()
                                        .filter_map(|mv| {
                                            Some(CoreferenceMention {
                                                sentence_id: mv
                                                    .get("sentence_id")
                                                    .and_then(|v| v.as_u64())?,
                                                text: mv.get("text").and_then(|v| v.as_str())?.to_string(),
                                                grammar_role: mv
                                                    .get("grammar_role")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("")
                                                    .to_string(),
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let cleaned_text = sentence_nodes
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let processed = ProcessedChunk {
            index: chunk.index,
            original_text: chunk.text.clone(),
            cleaned_text,
            start_offset: chunk.start_char,
            end_offset: chunk.end_char,
            prompt_start_char: chunk.start_char as usize,
            prompt_end_char: chunk.end_char as usize,
            token_count: chunk.token_count,
            keywords: Vec::new(),
            entities: Vec::new(),
            topics: Vec::new(),
            overlap_from_previous: 0,
            overlap_to_next: 0,
            sentence_nodes: sentence_nodes.clone(),
            paragraph_nodes: Vec::new(),
            section_nodes: section_nodes.clone(),
            document_nodes: Vec::new(),
            cross_sentence_relationships: cross_rels,
            coreference_chains: coref_chains,
            detected_modalities: modalities.clone(),
            chunk_graph_id: None,
        };

        let chunk_graph = self.create_chunk_graph(&processed, 0);
        Ok((processed, chunk_graph, modalities, section_nodes))
    }

    /// Convert an OMEX grammar_tree JSON node into a GrammarNode, verifying
    /// span containment (child within parent) along the way.
    fn json_to_grammar_node(v: &serde_json::Value, text: &str) -> Option<GrammarNode> {
        let node_type_str = v.get("node_type").and_then(|t| t.as_str())?;
        let node_type: GrammarNodeType =
            serde_json::from_value(serde_json::json!(node_type_str)).ok()?;
        let s = Self::floor_to_char_boundary(
            text,
            v.get("position_start").and_then(|p| p.as_u64()).unwrap_or(0) as usize,
        );
        let e = Self::ceil_to_char_boundary(
            text,
            v.get("position_end")
                .and_then(|p| p.as_u64())
                .unwrap_or(text.len() as u64) as usize,
        )
        .min(text.len());
        if e < s {
            return None;
        }
        let children: Vec<GrammarNode> = v
            .get("children")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|cv| {
                        let c = Self::json_to_grammar_node(cv, text)?;
                        // Containment check: child within parent span.
                        if c.position.start_offset < s || c.position.end_offset > e {
                            return None;
                        }
                        Some(c)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let properties = v
            .get("properties")
            .cloned()
            .and_then(|p| serde_json::from_value::<GrammarProperties>(p).ok())
            .unwrap_or_default();
        Some(GrammarNode {
            node_id: 0,
            node_type,
            text: text[s..e].to_string(),
            position: TextPosition {
                start_offset: s,
                end_offset: e,
                line: None,
                column: None,
            },
            children,
            properties,
        })
    }
    /// Analyze sentiment via LLM
    async fn analyze_sentiment(&self, text: &str) -> Option<Sentiment> {
        let prompt = format!(
            r#"Analyze the sentiment of this text.
Return JSON: {{"overall": -1.0 to 1.0, "positive": 0-1, "negative": 0-1, "neutral": 0-1}}

TEXT:
{}

RESPOND ONLY WITH JSON."#,
            &text[..text.len().min(2000)]
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 100,
            "temperature": 0.2,
            "system_context": "Analyze sentiment. Respond with JSON only."
        });

        match self.executor.execute(9, input).await {
            Ok(result) => result
                .get("response")
                .and_then(|r| r.as_str())
                .and_then(|s| {
                    let json_str = Self::extract_json_from_response(s, '{', '}');
                    serde_json::from_str::<serde_json::Value>(&json_str).ok()
                })
                .map(|v| Sentiment {
                    overall: v.get("overall").and_then(|o| o.as_f64()).unwrap_or(0.0) as f32,
                    positive: v.get("positive").and_then(|p| p.as_f64()).unwrap_or(0.33) as f32,
                    negative: v.get("negative").and_then(|n| n.as_f64()).unwrap_or(0.33) as f32,
                    neutral: v.get("neutral").and_then(|n| n.as_f64()).unwrap_or(0.34) as f32,
                }),
            Err(_) => None,
        }
    }

    /// Count syllables (heuristic)
    fn count_syllables(text: &str) -> usize {
        let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
        let mut count = 0;
        let mut prev_was_vowel = false;

        for c in text.to_lowercase().chars() {
            let is_vowel = vowels.contains(&c);
            if is_vowel && !prev_was_vowel {
                count += 1;
            }
            prev_was_vowel = is_vowel;
        }

        count.max(1)
    }

    // ========================================================================
    // GRAPH OPERATIONS
    // ========================================================================

    /// Create graph from analysis results
    async fn create_graph(
        &self,
        analysis: TextAnalysisResult,
        project_id: u64,
        _link_to_existing: bool,
    ) -> TextModalityOutput {
        let graph_id = Self::generate_id();
        let now = chrono::Utc::now().to_rfc3339();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_id = 1u64;
        let mut edge_id = 1u64;

        // Create document root node
        let doc_node_id = node_id;
        nodes.push(Self::text_new_node(
            doc_node_id,
            TextNodeType::Document,
            format!(
                "{} words, {} sentences, {} paragraphs",
                analysis.word_count, analysis.sentence_count, analysis.paragraph_count
            ),
            None,
            {
                let mut props = HashMap::new();
                props.insert("word_count".to_string(), serde_json::json!(analysis.word_count));
                props.insert("sentence_count".to_string(), serde_json::json!(analysis.sentence_count));
                props.insert("paragraph_count".to_string(), serde_json::json!(analysis.paragraph_count));
                if let Some(score) = analysis.readability_score {
                    props.insert("readability_score".to_string(), serde_json::json!(score));
                }
                props
            },
        ));
        node_id += 1;

        // Create section nodes
        for section in &analysis.structure.sections {
            let section_node_id = node_id;
            nodes.push(Self::text_new_node(
                section_node_id,
                TextNodeType::Section,
                section.title.clone().unwrap_or_default(),
                Some(TextPosition {
                    start_offset: section.start_offset,
                    end_offset: section.end_offset,
                    line: None,
                    column: None,
                }),
                {
                    let mut props = HashMap::new();
                    props.insert("level".to_string(), serde_json::json!(section.level));
                    props
                },
            ));

            edges.push(Self::text_new_edge(
                edge_id,
                doc_node_id,
                section_node_id,
                TextEdgeType::Contains,
                1.0,
            ));
            edge_id += 1;

            node_id += 1;
        }

        // Create entity nodes
        for entity in &analysis.entities {
            let entity_node_id = node_id;
            nodes.push(Self::text_new_node(
                entity_node_id,
                TextNodeType::Entity,
                entity.text.clone(),
                Some(TextPosition {
                    start_offset: entity.start_offset,
                    end_offset: entity.end_offset,
                    line: None,
                    column: None,
                }),
                {
                    let mut props = HashMap::new();
                    props.insert(
                        "entity_type".to_string(),
                        serde_json::to_value(&entity.entity_type).unwrap(),
                    );
                    props.insert(
                        "confidence".to_string(),
                        serde_json::json!(entity.confidence),
                    );
                    props
                },
            ));

            edges.push(Self::text_new_edge(
                edge_id,
                doc_node_id,
                entity_node_id,
                TextEdgeType::Contains,
                entity.confidence,
            ));
            edge_id += 1;

            node_id += 1;
        }

        // Create topic nodes
        for topic in &analysis.topics {
            let topic_node_id = node_id;
            nodes.push(Self::text_new_node(
                topic_node_id,
                TextNodeType::Topic,
                topic.name.clone(),
                None,
                {
                    let mut props = HashMap::new();
                    props.insert("relevance".to_string(), serde_json::json!(topic.relevance));
                    props.insert(
                        "keywords".to_string(),
                        serde_json::to_value(&topic.keywords).unwrap(),
                    );
                    props
                },
            ));

            edges.push(Self::text_new_edge(
                edge_id,
                doc_node_id,
                topic_node_id,
                TextEdgeType::Contains,
                topic.relevance,
            ));
            edge_id += 1;

            node_id += 1;
        }

        // Create keyword nodes
        for keyword in &analysis.keywords {
            let keyword_node_id = node_id;
            nodes.push(Self::text_new_node(
                keyword_node_id,
                TextNodeType::Keyword,
                keyword.term.clone(),
                None,
                {
                    let mut props = HashMap::new();
                    props.insert(
                        "frequency".to_string(),
                        serde_json::json!(keyword.frequency),
                    );
                    props.insert(
                        "relevance".to_string(),
                        serde_json::json!(keyword.relevance),
                    );
                    props.insert(
                        "is_phrase".to_string(),
                        serde_json::json!(keyword.is_phrase),
                    );
                    props
                },
            ));

            edges.push(Self::text_new_edge(
                edge_id,
                doc_node_id,
                keyword_node_id,
                TextEdgeType::Contains,
                keyword.relevance,
            ));
            edge_id += 1;

            node_id += 1;
        }

        let graph = TextGraph {
            graph_id,
            modality: PIPELINE_MODALITY.to_string(),
            version: PIPELINE_VERSION.to_string(),
            nodes,
            edges,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("project_id".to_string(), serde_json::json!(project_id));
                meta.insert("created_at".to_string(), serde_json::json!(&now));
                meta.insert(
                    "document_type".to_string(),
                    serde_json::to_value(&analysis.structure.document_type).unwrap(),
                );
                meta
            },
            created_at: now.clone(),
            updated_at: now,
        };

        // Cache the graph
        {
            let mut cache = self.graph_cache.write().await;
            cache.insert(graph_id, graph.clone());
        }

        TextModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            graph: Some(graph),
            ..Default::default()
        }
    }

    /// Update existing graph
    async fn update_graph(&self, graph_id: u64, delta: TextDelta) -> TextModalityOutput {
        let mut cache = self.graph_cache.write().await;

        if let Some(graph) = cache.get_mut(&graph_id) {
            let now = chrono::Utc::now().to_rfc3339();
            graph.updated_at = now;

            match delta.operation {
                DeltaOperation::Insert => {
                    // Add new nodes/edges based on delta content
                    // Implementation depends on what's being inserted
                }
                DeltaOperation::Delete => {
                    // Remove nodes in affected_nodes
                    graph
                        .nodes
                        .retain(|n| !delta.affected_nodes.contains(&n.node_id));
                    graph.edges.retain(|e| {
                        !delta.affected_nodes.contains(&e.from_node)
                            && !delta.affected_nodes.contains(&e.to_node)
                    });
                }
                DeltaOperation::Replace => {
                    // Replace content in affected nodes
                    if let Some(content) = &delta.content {
                        for node in &mut graph.nodes {
                            if delta.affected_nodes.contains(&node.node_id) {
                                node.content = content.clone();
                            }
                        }
                    }
                }
                DeltaOperation::Reorder => {
                    // Reorder nodes (update edge weights or positions)
                }
            }

            TextModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                graph: Some(graph.clone()),
                ..Default::default()
            }
        } else {
            TextModalityOutput {
                success: false,
                error: Some(format!("Graph {} not found", graph_id)),
                ..Default::default()
            }
        }
    }

    /// Query graph
    async fn query_graph(&self, graph_id: u64, query: TextGraphQuery) -> TextModalityOutput {
        let cache = self.graph_cache.read().await;

        if let Some(graph) = cache.get(&graph_id) {
            let result_nodes: Vec<TextGraphNode> = match query.query_type {
                TextQueryType::FindEntities => graph
                    .nodes
                    .iter()
                    .filter(|n| n.node_type == TextNodeType::Entity)
                    .cloned()
                    .collect(),

                TextQueryType::FindTopics => graph
                    .nodes
                    .iter()
                    .filter(|n| n.node_type == TextNodeType::Topic)
                    .cloned()
                    .collect(),

                TextQueryType::FindKeywords => graph
                    .nodes
                    .iter()
                    .filter(|n| n.node_type == TextNodeType::Keyword)
                    .cloned()
                    .collect(),

                TextQueryType::FindReferences => graph
                    .nodes
                    .iter()
                    .filter(|n| n.node_type == TextNodeType::Reference)
                    .cloned()
                    .collect(),

                TextQueryType::GetStructure => graph
                    .nodes
                    .iter()
                    .filter(|n| {
                        n.node_type == TextNodeType::Document
                            || n.node_type == TextNodeType::Section
                            || n.node_type == TextNodeType::Paragraph
                    })
                    .cloned()
                    .collect(),

                TextQueryType::GetNodesByType => {
                    if let Some(node_type) = query.parameters.get("node_type") {
                        let type_str = node_type.as_str().unwrap_or("");
                        graph
                            .nodes
                            .iter()
                            .filter(|n| {
                                format!("{:?}", n.node_type).to_lowercase()
                                    == type_str.to_lowercase()
                            })
                            .cloned()
                            .collect()
                    } else {
                        Vec::new()
                    }
                }

                _ => graph.nodes.clone(),
            };

            // Create result graph with filtered nodes and their edges
            let result_node_ids: HashSet<u64> = result_nodes.iter().map(|n| n.node_id).collect();
            let result_edges: Vec<TextGraphEdge> = graph
                .edges
                .iter()
                .filter(|e| {
                    result_node_ids.contains(&e.from_node) || result_node_ids.contains(&e.to_node)
                })
                .cloned()
                .collect();

            let result_graph = TextGraph {
                graph_id: graph.graph_id,
                modality: graph.modality.clone(),
                version: graph.version.clone(),
                nodes: result_nodes,
                edges: result_edges,
                metadata: graph.metadata.clone(),
                created_at: graph.created_at.clone(),
                updated_at: graph.updated_at.clone(),
            };

            TextModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                graph: Some(result_graph),
                ..Default::default()
            }
        } else {
            TextModalityOutput {
                success: false,
                error: Some(format!("Graph {} not found", graph_id)),
                ..Default::default()
            }
        }
    }

    /// Get graph by ID
    async fn get_graph(&self, graph_id: u64) -> TextModalityOutput {
        let cache = self.graph_cache.read().await;

        if let Some(graph) = cache.get(&graph_id) {
            TextModalityOutput {
                success: true,
                graph_id: Some(graph_id),
                graph: Some(graph.clone()),
                ..Default::default()
            }
        } else {
            TextModalityOutput {
                success: false,
                error: Some(format!("Graph {} not found", graph_id)),
                ..Default::default()
            }
        }
    }

    // ========================================================================
    // ZSEI HOOKS
    // ========================================================================

    /// Trigger ZSEI semantic hook
    async fn trigger_semantic_hook(
        &self,
        graph_id: u64,
        hook_type: ZSEIHookType,
    ) -> TextModalityOutput {
        let start = std::time::Instant::now();

        // In production, this would call ZSEI to perform semantic enrichment
        // For now, return a stub result
        let hook_result = HookResult {
            hook_type: hook_type.clone(),
            success: true,
            nodes_processed: 0,
            edges_added: 0,
            annotations_added: 0,
            processing_time_ms: start.elapsed().as_millis() as u64,
            errors: Vec::new(),
        };

        TextModalityOutput {
            success: true,
            graph_id: Some(graph_id),
            hook_result: Some(hook_result),
            ..Default::default()
        }
    }

    // ========================================================================
    // CROSS-MODALITY
    // ========================================================================

    /// Link to another modality graph
    async fn link_to_modality(
        &self,
        source_graph_id: u64,
        target_graph_id: u64,
        _target_modality: &str,
        relationship: CrossModalityRelation,
    ) -> TextModalityOutput {
        let link_id = Self::generate_id();
        let now = chrono::Utc::now().to_rfc3339();

        let link_result = LinkResult {
            link_id,
            source_graph_id,
            target_graph_id,
            relationship,
            created_at: now,
        };

        TextModalityOutput {
            success: true,
            link_result: Some(link_result),
            ..Default::default()
        }
    }

    // ========================================================================
    // HELPERS
    // ========================================================================

    fn generate_id() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// Construct a TextGraphNode with all universal fields defaulted.
    fn text_new_node(
        node_id: u64,
        node_type: TextNodeType,
        content: String,
        position: Option<TextPosition>,
        properties: HashMap<String, Value>,
    ) -> TextGraphNode {
        TextGraphNode {
            node_id,
            node_type,
            content,
            position,
            properties,
            semantic_annotations: Vec::new(),
            provisional: false,
            provisional_status: ProvisionalStatus::Generated,
            provenance: EdgeProvenance::DerivedFromPrompt,
            source_chunk_id: None,
            source_file_id: None,
            created_by_step: None,
            updated_by_step: None,
            version: default_version(),
            version_notes: Vec::new(),
            materialized_path: None,
            keywords: Vec::new(),
            embedding_hint: None,
            hotness_score: default_hotness(),
            source_chunk_index: None,
            source_start_char: None,
            source_end_char: None,
            cross_modal_refs: Vec::new(),
        }
    }

    /// Construct a TextGraphEdge with all universal fields defaulted.
    fn text_new_edge(
        edge_id: u64,
        from_node: u64,
        to_node: u64,
        edge_type: TextEdgeType,
        weight: f32,
    ) -> TextGraphEdge {
        TextGraphEdge {
            edge_id,
            from_node,
            to_node,
            edge_type,
            weight,
            properties: HashMap::new(),
            provenance: EdgeProvenance::DerivedFromPrompt,
            created_by_step: None,
            version: default_version(),
            version_notes: Vec::new(),
            is_cross_modal: false,
            cross_modal_index_id: None,
            grammar_info: None,
        }
    }

    fn node_to_core(node: &TextGraphNode) -> NodeCore {
        NodeCore {
            node_id: node.node_id,
            node_type: format!("{:?}", node.node_type),
            label: node.content.chars().take(60).collect(),
            content: node.content.clone(),
            provisional: node.provisional,
            provisional_status: node.provisional_status.clone(),
            provenance: node.provenance.clone(),
            source_chunk_id: node.source_chunk_id,
            source_file_id: node.source_file_id,
            created_by_step: node.created_by_step,
            updated_by_step: node.updated_by_step,
            version: node.version,
            version_notes: node.version_notes.clone(),
            materialized_path: node.materialized_path.clone(),
            keywords: node.keywords.clone(),
            embedding_hint: node.embedding_hint.clone(),
            hotness_score: node.hotness_score,
            source_chunk_index: node.source_chunk_index,
            source_start_char: node.source_start_char,
            source_end_char: node.source_end_char,
            cross_modal_refs: node.cross_modal_refs.clone(),
            time_range_start: None,
            time_range_end: None,
        }
    }

    fn parse_json_array(s: &str) -> Option<Vec<String>> {
        let json_str = Self::extract_json_from_response(s, '[', ']');
        serde_json::from_str::<Vec<String>>(json_str.trim()).ok()
    }

    fn extract_json_from_response(s: &str, start_char: char, end_char: char) -> String {
        let trimmed = s.trim();
        if let Some(start) = trimmed.find(start_char) {
            if let Some(end) = trimmed.rfind(end_char) {
                return trimmed[start..=end].trim().to_string();
            }
        }
        trimmed.to_string()
    }
}

/// Reconstruct clean text from ChunkGraphs at any token limit.
/// Enables cross-model capability: same chunk graphs work for any LLM context window.
pub fn reconstruct_context_at_token_limit(
    chunk_graphs: &[ChunkGraph],
    target_tokens: usize,
    chars_per_token_estimate: usize,
) -> ReconstructedContext {
    let target_chars = target_tokens * chars_per_token_estimate;
    let mut included_chunks = Vec::new();
    let mut reconstruction_parts: Vec<String> = Vec::new();
    let mut total_chars = 0usize;

    for chunk in chunk_graphs {
        let chunk_chars = chunk.cleaned_text.len();
        if total_chars + chunk_chars <= target_chars {
            reconstruction_parts.push(chunk.cleaned_text.clone());
            included_chunks.push(chunk.chunk_index);
            total_chars += chunk_chars;
        } else {
            // Partial chunk — trim to clean sentence boundary
            let remaining = target_chars - total_chars;
            if remaining > 0 {
                let partial = trim_to_sentence_boundary(
                    &chunk.cleaned_text,
                    remaining,
                    &chunk.sentence_boundaries,
                );
                if !partial.is_empty() {
                    total_chars += partial.len();
                    reconstruction_parts.push(partial);
                    included_chunks.push(chunk.chunk_index);
                }
            }
            break;
        }
    }

    ReconstructedContext {
        text: reconstruction_parts.join("\n\n"),
        included_chunk_indices: included_chunks,
        total_chars,
        estimated_tokens: total_chars / chars_per_token_estimate.max(1),
    }
}

/// Trim text to the nearest sentence boundary at or before `max_chars`.
fn trim_to_sentence_boundary(
    text: &str,
    max_chars: usize,
    boundaries: &[SentenceBoundary],
) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }

    // Find the last sentence boundary whose end falls within max_chars
    let best_end = boundaries
        .iter()
        .filter(|b| b.end <= max_chars)
        .map(|b| b.end)
        .max();

    match best_end {
        Some(end) if end > 0 => text[..end].to_string(),
        _ => {
            // No sentence boundary found — fall back to last space before max_chars
            let slice = &text[..max_chars];
            match slice.rfind(' ') {
                Some(space_idx) => text[..space_idx].to_string(),
                None => slice.to_string(),
            }
        }
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

/// Stub executor for standalone testing
struct StubExecutor;

#[async_trait::async_trait]
impl PipelineExecutor for StubExecutor {
    async fn execute(
        &self,
        _pipeline_id: u64,
        _input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({"response": "[]"}))
    }
}

/// Executor contract: `--input` carries the full PipelineInput JSON
/// ({"data":{...},"context":{...}}) — unwrap the data envelope. A bare
/// action payload is accepted too; stdin is the legacy standalone path.
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
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| e.to_string())?;
            buf
        }
    };
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let inner = v.get("data").cloned().unwrap_or(v);
    serde_json::from_value(inner).map_err(|e| e.to_string())
}

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

/// K-ALGORITHM contracts (shared with the host crate — single source).
#[path = "../../../../shared/contracts/k_validation.rs"]
mod k_validation;

/// K-ALGORITHM loop-discipline contracts (shared — single source).
#[path = "../../../../shared/contracts/k_loops.rs"]
mod k_loops;

#[tokio::main]
async fn main() {
    let executor = Arc::new(StubExecutor);
    let pipeline = Arc::new(TextModalityPipeline::new(executor));

    // SERVE MODE — connect-model: `--serve` boots a long-running service that
    // registers with the host and answers POST /execute. No --serve = the
    // classic one-shot CLI path.
    if let Some(opts) = ozone_serve::serve_mode() {
        let pipeline_for_serve = pipeline.clone();
        let handler = Arc::new(move |action_payload: serde_json::Value| {
            let input: TextModalityInput = serde_json::from_value(
                action_payload.get("action").cloned().unwrap_or(action_payload),
            )
            .unwrap_or_else(|_| TextModalityInput {
                action: TextModalityAction::GetGraph { graph_id: 0 },
            });
            let pipeline = pipeline_for_serve.clone();
            let rt = tokio::runtime::Runtime::new().expect("serve runtime");
            let output = rt.block_on(pipeline.execute(input));
            serde_json::to_value(&output).unwrap_or(serde_json::json!({
                "success": false, "error": "serialization failed"
            }))
        });
        ozone_serve::serve(opts, PIPELINE_ID, PIPELINE_MODALITY.to_string(), handler);
    }

    let input: TextModalityInput =
        parse_cli_input().expect("Failed to parse input");

    let output = pipeline.execute(input).await;

    serde_json::to_writer(std::io::stdout(), &output).expect("Failed to write output");
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockExecutor;

    #[async_trait::async_trait]
    impl PipelineExecutor for MockExecutor {
        async fn execute(
            &self,
            _pipeline_id: u64,
            input: serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            // Return mock LLM responses
            let prompt = input.get("prompt").and_then(|p| p.as_str()).unwrap_or("");

            if prompt.contains("keywords") {
                Ok(serde_json::json!({"response": r#"["test", "keyword", "extraction"]"#}))
            } else if prompt.contains("entities") {
                Ok(
                    serde_json::json!({"response": r#"[{"text": "John", "type": "PERSON", "confidence": 0.9}]"#}),
                )
            } else if prompt.contains("topics") {
                Ok(serde_json::json!({"response": r#"["testing", "software"]"#}))
            } else if prompt.contains("Clean") {
                Ok(serde_json::json!({"response": "Cleaned text here."}))
            } else {
                Ok(serde_json::json!({"response": "test response"}))
            }
        }
    }

    #[test]
    fn test_chunk_text() {
        let text = "Para one.\n\nPara two.\n\nPara three. Final sentence!";
        let chunks = TextModalityPipeline::chunk_text(text, 20);
        assert!(!chunks.is_empty());
        // No-overlap chunking: chunks tile the text exactly.
        let total: usize = chunks.iter().map(|c| c.text.len()).sum();
        assert_eq!(total, text.len());
        assert_eq!(chunks.first().unwrap().index, 0);
    }

    #[tokio::test]
    async fn test_analyze_text() {
        let executor = Arc::new(MockExecutor);
        let pipeline = TextModalityPipeline::new(executor);

        let input = TextModalityInput {
            action: TextModalityAction::Analyze {
                text: "John Smith works at Acme Corp. He joined on January 15, 2024.".to_string(),
                max_chunk_tokens: 2000,
                depth: AnalysisDepth::Standard,
                extract_entities: true,
                extract_topics: true,
                available_modalities: vec![],
                processing_path: ProcessingPath::Path1,
                executor_model: ExecutorModelKind::default(),
                grammar_mode: GrammarExtractionMode::default(),
            },
        };

        let output = pipeline.execute(input).await;
        assert!(output.success);
        assert!(output.analysis.is_some());

        let analysis = output.analysis.unwrap();
        assert!(analysis.word_count > 0);
    }

    #[tokio::test]
    async fn test_create_and_query_graph() {
        let executor = Arc::new(MockExecutor);
        let pipeline = TextModalityPipeline::new(executor);

        let analysis = TextAnalysisResult {
            word_count: 10,
            sentence_count: 2,
            paragraph_count: 1,
            character_count: 50,
            entities: vec![Entity {
                text: "John".to_string(),
                entity_type: EntityType::Person,
                start_offset: 0,
                end_offset: 4,
                confidence: 0.9,
                metadata: HashMap::new(),
            }],
            topics: vec![Topic {
                name: "employment".to_string(),
                keywords: vec!["work".to_string()],
                relevance: 0.8,
                category: None,
            }],
            keywords: vec![],
            structure: DocumentStructure {
                sections: vec![],
                has_title: false,
                has_abstract: false,
                has_toc: false,
                document_type: DocumentType::Unknown,
            },
            language: Some("en".to_string()),
            sentiment: None,
            readability_score: Some(70.0),
        };

        let create_input = TextModalityInput {
            action: TextModalityAction::CreateGraph {
                analysis_result: analysis,
                project_id: 1,
                link_to_existing: false,
            },
        };

        let create_output = pipeline.execute(create_input).await;
        assert!(create_output.success);
        assert!(create_output.graph_id.is_some());

        let graph_id = create_output.graph_id.unwrap();

        let query_input = TextModalityInput {
            action: TextModalityAction::QueryGraph {
                graph_id,
                query: TextGraphQuery {
                    query_type: TextQueryType::FindEntities,
                    parameters: HashMap::new(),
                },
            },
        };

        let query_output = pipeline.execute(query_input).await;
        assert!(query_output.success);
        assert!(query_output.graph.is_some());

        let result_graph = query_output.graph.unwrap();
        assert!(!result_graph.nodes.is_empty());
    }

    #[test]
    fn test_reconstruct_from_chunks() {
        let executor = Arc::new(StubExecutor);
        let pipeline = TextModalityPipeline::new(executor);

        let result = pipeline.reconstruct_from_chunks(&[ProcessedChunk {
            index: 0,
            original_text: "First chunk.".to_string(),
            cleaned_text: "First chunk.".to_string(),
            start_offset: 0,
            end_offset: 12,
            token_count: 3,
            keywords: vec![],
            entities: vec![],
            topics: vec![],
            overlap_from_previous: 0,
            overlap_to_next: 0,
            sentence_nodes: vec![],
            paragraph_nodes: vec![],
            section_nodes: vec![],
            document_nodes: vec![],
            cross_sentence_relationships: vec![],
            coreference_chains: vec![],
            detected_modalities: vec![],
            chunk_graph_id: None,
            prompt_start_char: 0,
            prompt_end_char: 12,
        }]);

        assert!(result.success);
        assert_eq!(result.reconstructed_text.unwrap(), "First chunk.");
    }
}
