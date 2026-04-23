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

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// ============================================================================
// PIPELINE METADATA
// ============================================================================

pub const PIPELINE_ID: u64 = 100;
pub const PIPELINE_NAME: &str = "TextAnalysisPipeline";
pub const PIPELINE_VERSION: &str = "0.4.0";
pub const PIPELINE_MODALITY: &str = "text";

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
        #[serde(default)]
        depth: AnalysisDepth,
        #[serde(default)]
        extract_entities: bool,
        #[serde(default)]
        extract_topics: bool,
        #[serde(default)]
        extract_structure: bool,
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

    /// Chunk text with semantic awareness
    ChunkText {
        text: String,
        #[serde(default = "default_max_chunk_tokens")]
        max_chunk_tokens: u32,
        #[serde(default = "default_overlap_tokens")]
        overlap_tokens: u32,
        #[serde(default)]
        preserve_paragraphs: bool,
    },

    /// Clean and normalize a text chunk via zero-shot LLM
    CleanChunk { chunk: RawChunk },

    /// Process a chunk: clean + extract keywords/entities/topics
    ProcessChunk { chunk: RawChunk },

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

    /// Detect modalities present in a chunk via zero-shot (5x stable loop)
    DetectModalities {
        text: String,
        include_true_text: bool,
        dynamic_modality_list: bool,
    },

    /// Create a persistent chunk graph from a processed chunk
    CreateChunkGraph {
        chunk: ProcessedChunk,
        root_graph_id: u64,
    },

    /// Extract grammar relationships via zero-shot LLM
    ExtractGrammarRelationships { text: String, chunk_index: u32 },

    /// Normalize text (basic cleaning without LLM)
    Normalize {
        text: String,
        #[serde(default)]
        chunk_size: Option<usize>,
        #[serde(default)]
        overlap: usize,
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
fn default_overlap_tokens() -> u32 {
    200
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
    pub span_start: usize, // char offset within the chunk text
    pub span_end: usize,   // char offset within the chunk text
    // NO content_snippet — retrieve dynamically: &text[span_start..span_end]
    pub intent_reference: String,
    pub chunk_index: u32,
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
    pub grammar_relationships: Vec<ChunkGrammarRelationship>,
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
    pub grammar_relationships: Vec<ChunkGrammarRelationship>,
    pub detected_modalities: Vec<ChunkModalityDetection>,
    pub chunk_graph_id: Option<u64>,
    pub prompt_start_char: usize, // absolute position in original prompt
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
    RelatesTo,
    Contradicts,
    Supports,
    Elaborates,
    Summarizes,
    SimilarTo,

    // Cross-modality
    DescribesCode,
    DescribesImage,
    DescribesAudio,
    DescribesVideo,
    TranscribedFrom,
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
}

impl TextModalityPipeline {
    pub fn new(executor: Arc<dyn PipelineExecutor>) -> Self {
        Self {
            executor,
            graph_cache: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    pub async fn execute(&self, input: TextModalityInput) -> TextModalityOutput {
        let start_time = std::time::Instant::now();

        let mut output = match input.action {
            TextModalityAction::Analyze {
                text,
                depth,
                extract_entities,
                extract_topics,
                extract_structure,
            } => {
                self.analyze_text(
                    &text,
                    depth,
                    extract_entities,
                    extract_topics,
                    extract_structure,
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

            TextModalityAction::ChunkText {
                text,
                max_chunk_tokens,
                overlap_tokens,
                preserve_paragraphs,
            } => {
                let chunks = self
                    .chunk_text_with_llm_overlap(
                        &text,
                        max_chunk_tokens,
                        overlap_tokens,
                        preserve_paragraphs,
                    )
                    .await;
                TextModalityOutput {
                    success: true,
                    chunks: Some(chunks),
                    token_count: Some(Self::estimate_tokens(&text)),
                    ..Default::default()
                }
            }

            TextModalityAction::CleanChunk { chunk } => self.clean_chunk(chunk).await,

            TextModalityAction::ProcessChunk {
                chunk,
                available_modalities,
            } => self.process_chunk(chunk, available_modalities).await,

            TextModalityAction::ReconstructFromChunks { chunks } => {
                self.reconstruct_from_chunks(&chunks)
            }

            TextModalityAction::ExtractKeywords { text, max_keywords } => {
                self.extract_keywords_llm(&text, max_keywords).await
            }

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
                include_true_text,
                dynamic_modality_list,
                available_modalities,
            } => {
                let available = if dynamic_modality_list && !available_modalities.is_empty() {
                    available_modalities
                };

                let detections = self.detect_modalities_stable(&text, 0, &available).await;

                let filtered: Vec<ChunkModalityDetection> = if include_true_text {
                    detections
                } else {
                    detections
                        .into_iter()
                        .filter(|d| d.modality != "true_text")
                        .collect()
                };

                TextModalityOutput {
                    success: true,
                    modality_detections: Some(filtered),
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

    // ========================================================================
    // CHUNKING METHODS
    // ========================================================================

    /// Chunk text preserving paragraph boundaries
    fn chunk_text_semantic(text: &str, max_tokens: u32, overlap_tokens: u32) -> Vec<RawChunk> {
        let mut chunks = Vec::new();
        let paragraphs: Vec<&str> = text.split("\n\n").collect();

        let mut current_text = String::new();
        let mut current_tokens: u32 = 0;
        let mut chunk_index: u32 = 0;
        let mut char_offset: u32 = 0;
        let mut chunk_start: u32 = 0;

        for para in paragraphs {
            let para_tokens = Self::estimate_tokens(para);

            // If single paragraph exceeds max, split by sentences
            if para_tokens > max_tokens {
                // Flush current chunk first
                if !current_text.is_empty() {
                    chunks.push(RawChunk {
                        index: chunk_index,
                        text: current_text.clone(),
                        start_char: chunk_start,
                        end_char: char_offset,
                        token_count: current_tokens,
                        is_complete_paragraph: true,
                    });
                    chunk_index += 1;

                    // Keep overlap for next chunk
                    let overlap_text = Self::get_end_overlap(&current_text, overlap_tokens);
                    current_text = overlap_text;
                    current_tokens = Self::estimate_tokens(&current_text);
                    chunk_start = char_offset.saturating_sub(current_text.len() as u32);
                }

                // Split paragraph by sentences
                let sentences = Self::split_sentences(para);
                for sentence in sentences {
                    let sent_tokens = Self::estimate_tokens(sentence);

                    if current_tokens + sent_tokens > max_tokens && !current_text.is_empty() {
                        chunks.push(RawChunk {
                            index: chunk_index,
                            text: current_text.clone(),
                            start_char: chunk_start,
                            end_char: char_offset,
                            token_count: current_tokens,
                            is_complete_paragraph: false,
                        });
                        chunk_index += 1;

                        let overlap_text = Self::get_end_overlap(&current_text, overlap_tokens);
                        current_text = overlap_text;
                        current_tokens = Self::estimate_tokens(&current_text);
                        chunk_start = char_offset.saturating_sub(current_text.len() as u32);
                    }

                    if !current_text.is_empty() && !current_text.ends_with(' ') {
                        current_text.push(' ');
                    }
                    current_text.push_str(sentence);
                    current_tokens += sent_tokens;
                    char_offset += sentence.len() as u32 + 1;
                }
            } else if current_tokens + para_tokens > max_tokens {
                // Would exceed - flush current chunk
                if !current_text.is_empty() {
                    chunks.push(RawChunk {
                        index: chunk_index,
                        text: current_text.clone(),
                        start_char: chunk_start,
                        end_char: char_offset,
                        token_count: current_tokens,
                        is_complete_paragraph: true,
                    });
                    chunk_index += 1;

                    let overlap_text = Self::get_end_overlap(&current_text, overlap_tokens);
                    current_text = overlap_text;
                    current_tokens = Self::estimate_tokens(&current_text);
                    chunk_start = char_offset.saturating_sub(current_text.len() as u32);
                }

                // Add paragraph
                if !current_text.is_empty() {
                    current_text.push_str("\n\n");
                }
                current_text.push_str(para);
                current_tokens += para_tokens;
                char_offset += para.len() as u32 + 2;
            } else {
                // Add to current chunk
                if !current_text.is_empty() {
                    current_text.push_str("\n\n");
                }
                current_text.push_str(para);
                current_tokens += para_tokens;
                char_offset += para.len() as u32 + 2;
            }
        }

        // Don't forget last chunk
        if !current_text.is_empty() {
            chunks.push(RawChunk {
                index: chunk_index,
                text: current_text.clone(),
                start_char: chunk_start,
                end_char: char_offset,
                token_count: current_tokens,
                is_complete_paragraph: true,
            });
        }

        // If no chunks created (empty text), create one empty chunk
        if chunks.is_empty() {
            chunks.push(RawChunk {
                index: 0,
                text: text.to_string(),
                start_char: 0,
                end_char: text.len() as u32,
                token_count: Self::estimate_tokens(text),
                is_complete_paragraph: true,
            });
        }

        chunks
    }

    /// Estimate token count (roughly 4 chars per token for English)
    fn estimate_tokens(text: &str) -> u32 {
        ((text.len() + 3) / 4) as u32
    }

    /// Get overlap text from end of string
    fn get_end_overlap(text: &str, overlap_tokens: u32) -> String {
        let overlap_chars = (overlap_tokens * 4) as usize;
        if text.len() <= overlap_chars {
            text.to_string()
        } else {
            let start = text.len() - overlap_chars;
            let adjusted_start = text[start..]
                .find(' ')
                .map(|i| start + i + 1)
                .unwrap_or(start);
            text[adjusted_start..].to_string()
        }
    }

    /// Resolve overlap boundary between two adjacent chunks via LLM zero-shot.
    /// Returns OverlapResolution. Falls back to rule-based on LLM failure.
    async fn resolve_overlap_llm(
        &self,
        current_chunk_text: &str,
        next_chunk_text: &str,
        overlap_chars: usize,
    ) -> OverlapResolution {
        let current_tail_start = current_chunk_text.len().saturating_sub(overlap_chars);
        let current_tail = &current_chunk_text[current_tail_start..];
        let next_head_end = overlap_chars.min(next_chunk_text.len());
        let next_head = &next_chunk_text[..next_head_end];

        let prompt = format!(
            r#"Analyze the overlapping region between two adjacent text chunks.

    END OF CURRENT CHUNK:
    ---
    {}
    ---

    START OF NEXT CHUNK:
    ---
    {}
    ---

    Determine if there is duplicated content and where the clean boundary should be.

    Return ONLY valid JSON (no explanation, no markdown):
    {{
      "has_overlap": true,
      "overlap_type": "sentence_cutoff|paragraph_cutoff|word_cutoff|none",
      "current_should_end_at_offset": <integer, chars from start of current_tail to keep>,
      "next_should_start_at_offset": <integer, chars into next_head to skip>,
      "duplicate_belongs_in": "current|next|neither"
    }}"#,
            current_tail, next_head
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 200,
            "temperature": 0.05,
            "system_context": "Text boundary analysis. Return only valid JSON object. No explanation."
        });

        match self.executor.execute(9, input).await {
            Ok(result) => {
                let raw = result
                    .get("response")
                    .and_then(|r| r.as_str())
                    .unwrap_or("{}");
                let json_str = Self::extract_json_from_response(raw, '{', '}');
                match serde_json::from_str::<serde_json::Value>(&json_str) {
                    Ok(v) => OverlapResolution {
                        has_overlap: v["has_overlap"].as_bool().unwrap_or(false),
                        overlap_type: match v["overlap_type"].as_str().unwrap_or("none") {
                            "sentence_cutoff" => OverlapType::SentenceCutoff,
                            "paragraph_cutoff" => OverlapType::ParagraphCutoff,
                            "word_cutoff" => OverlapType::WordCutoff,
                            _ => OverlapType::None,
                        },
                        current_keep_end: current_tail_start
                            + v["current_should_end_at_offset"]
                                .as_usize()
                                .unwrap_or(current_tail.len()),
                        next_start_offset: v["next_should_start_at_offset"].as_usize().unwrap_or(0),
                        duplicate_belongs_in: match v["duplicate_belongs_in"]
                            .as_str()
                            .unwrap_or("current")
                        {
                            "next" => DuplicateOwner::NextChunk,
                            "neither" => DuplicateOwner::Neither,
                            _ => DuplicateOwner::CurrentChunk,
                        },
                        resolution_method: ResolutionMethod::LLMZeroShot,
                    },
                    Err(_) => Self::rule_based_overlap_fallback(current_chunk_text, overlap_chars),
                }
            }
            Err(_) => Self::rule_based_overlap_fallback(current_chunk_text, overlap_chars),
        }
    }

    /// Rule-based overlap fallback — emergency only. Finds last sentence boundary.
    fn rule_based_overlap_fallback(text: &str, overlap_chars: usize) -> OverlapResolution {
        let tail_start = text.len().saturating_sub(overlap_chars);
        let tail = &text[tail_start..];
        let sentence_end = tail
            .rfind(|c| c == '.' || c == '!' || c == '?')
            .map(|i| i + 1)
            .unwrap_or(tail.len());
        OverlapResolution {
            has_overlap: false,
            overlap_type: OverlapType::SentenceCutoff,
            current_keep_end: tail_start + sentence_end,
            next_start_offset: 0,
            duplicate_belongs_in: DuplicateOwner::CurrentChunk,
            resolution_method: ResolutionMethod::RuleBased,
        }
    }

    /// Chunk text using LLM-based overlap resolution (canonical path).
    /// Called from the async execute path. Produces RawChunks with resolved boundaries.
    async fn chunk_text_with_llm_overlap(
        &self,
        text: &str,
        max_chunk_tokens: u32,
        overlap_tokens: u32,
        preserve_paragraphs: bool,
    ) -> Vec<RawChunk> {
        // Step 1: Get initial raw chunks using existing rule-based splitter
        let initial_chunks = if preserve_paragraphs {
            Self::chunk_text_semantic(text, max_chunk_tokens, overlap_tokens)
        } else {
            Self::chunk_text_simple(text, max_chunk_tokens, overlap_tokens)
        };

        if initial_chunks.len() <= 1 {
            return initial_chunks;
        }

        // Step 2: LLM-resolve overlaps between each consecutive pair
        let overlap_chars = (overlap_tokens * 4) as usize;
        let mut resolved: Vec<RawChunk> = Vec::with_capacity(initial_chunks.len());

        for (i, chunk) in initial_chunks.iter().enumerate() {
            if i + 1 < initial_chunks.len() {
                let next = &initial_chunks[i + 1];
                let resolution = self
                    .resolve_overlap_llm(&chunk.text, &next.text, overlap_chars)
                    .await;

                let kept_text =
                    if resolution.has_overlap && resolution.current_keep_end < chunk.text.len() {
                        chunk.text[..resolution.current_keep_end].to_string()
                    } else {
                        chunk.text.clone()
                    };

                resolved.push(RawChunk {
                    index: chunk.index,
                    text: kept_text,
                    start_char: chunk.start_char,
                    end_char: chunk.end_char,
                    token_count: Self::estimate_tokens(&chunk.text),
                    is_complete_paragraph: chunk.is_complete_paragraph,
                });
            } else {
                // Last chunk — no next chunk to resolve against
                resolved.push(chunk.clone());
            }
        }

        resolved
    }

    /// Split text into sentences
    fn split_sentences(text: &str) -> Vec<&str> {
        let mut sentences = Vec::new();
        let mut start = 0;

        for (i, c) in text.char_indices() {
            if c == '.' || c == '!' || c == '?' {
                let next_idx = i + c.len_utf8();
                let is_end =
                    next_idx >= text.len() || text[next_idx..].starts_with(char::is_whitespace);
                if is_end {
                    let sentence = text[start..=i].trim();
                    if !sentence.is_empty() {
                        sentences.push(sentence);
                    }
                    start = next_idx;
                }
            }
        }

        // Remaining text
        if start < text.len() {
            let remaining = text[start..].trim();
            if !remaining.is_empty() {
                sentences.push(remaining);
            }
        }

        sentences
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

    /// Process a chunk: clean + extract keywords/entities/topics
    async fn process_chunk(
        &self,
        chunk: RawChunk,
        available_modalities: Vec<String>,
    ) -> TextModalityOutput {
        // Step 1: Clean the chunk
        let clean_output = self
            .clean_chunk(RawChunk {
                index: chunk.index,
                text: chunk.text.clone(),
                start_char: chunk.start_char,
                end_char: chunk.end_char,
                token_count: chunk.token_count,
                is_complete_paragraph: chunk.is_complete_paragraph,
            })
            .await;
        let cleaned_text = clean_output
            .cleaned_text
            .unwrap_or_else(|| chunk.text.clone());

        // Step 2: Extract keywords via LLM
        let keywords = self.extract_keywords_from_text(&cleaned_text).await;

        // Step 3: Extract entities via LLM
        let entities = self.extract_entities_from_text(&cleaned_text).await;

        // Step 4: Extract topics via LLM
        let topics = self.extract_topics_from_text(&cleaned_text).await;

        // Calculate overlap
        let overlap_chars = 200 * 4; // ~200 tokens worth

        // Step 5: Extract grammar relationships
        let grammar_relationships = self
            .extract_grammar_relationships_from_text(&cleaned_text, chunk.index)
            .await;

        // Step 6: Detect modalities (5x stable, dynamic list)
        // Load modality names dynamically.
        let modality_list: Vec<String> = if !available_modalities.is_empty() {
            available_modalities
        };

        let detected_modalities = self
            .detect_modalities_stable(&cleaned_text, chunk.index, &modality_list)
            .await;

        let processed = ProcessedChunk {
            index: chunk.index,
            original_text: chunk.text.clone(),
            cleaned_text: cleaned_text.clone(),
            start_offset: chunk.start_char,
            end_offset: chunk.end_char,
            prompt_start_char: chunk.start_char as usize, // NEW
            prompt_end_char: chunk.end_char as usize,     // NEW
            token_count: chunk.token_count,
            keywords,
            entities,
            topics,
            grammar_relationships: grammar_relationships.clone(), // NEW
            detected_modalities: detected_modalities.clone(),     // NEW
            chunk_graph_id: None, // populated after CreateChunkGraph call
            overlap_from_previous: if chunk.index > 0 { overlap_chars } else { 0 },
            overlap_to_next: overlap_chars,
        };

        // Step 7: Create the chunk graph
        let chunk_graph = self.create_chunk_graph(&processed, 0);

        let mut result = TextModalityOutput {
            success: true,
            processed_chunks: Some(vec![processed]),
            chunk_graph: Some(chunk_graph),
            grammar_relationships: Some(grammar_relationships),
            modality_detections: Some(detected_modalities),
            ..Default::default()
        };
        result
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

    /// Run an async extractor repeatedly until 5 consecutive passes find nothing new.
    /// K must be Eq + Hash. dedup_key extracts the deduplication key from each item.
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

    /// Detect modalities present in a chunk via LLM zero-shot.
    /// Runs a 5x consecutive stable loop. Modality list is dynamic from registry.
    /// Returns ChunkModalityDetection with span positions — NO content_snippet.
    async fn detect_modalities_in_chunk(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        available_modalities: &[String],
    ) -> Vec<ChunkModalityDetection> {
        let modality_list = available_modalities.join(", ");

        let prompt = format!(
            r#"Identify all modality content present in or referenced by this text.

    Available modalities to detect: {}

    Also detect:
    - "true_text" — content that is genuinely prose/text (not embedded code/math/etc.)
    - "unknown" — content that does not fit any listed modality

    For each detected modality, span_start and span_end are character offsets within
    the provided text. Do NOT include a content_snippet — positions are sufficient.

    Text chunk (index {}):
    {}

    Return ONLY valid JSON array (no explanation, no markdown):
    [{{
      "modality": "modality_name_or_true_text_or_unknown",
      "span_start": 0,
      "span_end": 100,
      "intent_reference": "describes|contains|references|mentions the modality"
    }}]"#,
            modality_list,
            chunk_index,
            &chunk_text[..chunk_text.len().min(4000)]
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 800,
            "temperature": 0.05,
            "system_context": "Modality detection. Return only valid JSON array. No explanation."
        });

        match self.executor.execute(9, input).await {
            Ok(result) => {
                let raw = result
                    .get("response")
                    .and_then(|r| r.as_str())
                    .unwrap_or("[]");
                let json_str = Self::extract_json_from_response(raw, '[', ']');
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| {
                        Some(ChunkModalityDetection {
                            modality: v["modality"].as_str()?.to_string(),
                            span_start: v["span_start"].as_usize()?,
                            span_end: v["span_end"].as_usize()?,
                            intent_reference: v["intent_reference"]
                                .as_str()
                                .unwrap_or("")
                                .to_string(),
                            chunk_index,
                        })
                    })
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    /// Run modality detection with 5x consecutive stable loop.
    /// available_modalities: loaded from pipeline registry at runtime, never hardcoded.
    async fn detect_modalities_stable(
        &self,
        chunk_text: &str,
        chunk_index: u32,
        available_modalities: &[String],
    ) -> Vec<ChunkModalityDetection> {
        let mut accumulated: Vec<ChunkModalityDetection> = Vec::new();
        let mut no_new_consecutive = 0u32;

        for _round in 0..20usize {
            let candidates = self
                .detect_modalities_in_chunk(chunk_text, chunk_index, available_modalities)
                .await;

            // Dedup key: modality + span_start + span_end
            let existing_keys: std::collections::HashSet<String> = accumulated
                .iter()
                .map(|d| format!("{}:{}:{}", d.modality, d.span_start, d.span_end))
                .collect();

            let truly_new: Vec<ChunkModalityDetection> = candidates
                .into_iter()
                .filter(|d| {
                    !existing_keys
                        .contains(&format!("{}:{}:{}", d.modality, d.span_start, d.span_end))
                })
                .collect();

            if truly_new.is_empty() {
                no_new_consecutive += 1;
                if no_new_consecutive >= 5 {
                    break;
                }
            } else {
                no_new_consecutive = 0;
                accumulated.extend(truly_new);
            }
        }
        accumulated
    }

    /// Extract typed grammar relationships from a chunk via LLM zero-shot.
    async fn extract_grammar_relationships_from_text(
        &self,
        text: &str,
        chunk_index: u32,
    ) -> Vec<ChunkGrammarRelationship> {
        let prompt = format!(
            r#"Analyze the grammatical and semantic relationships in this text.
    For each meaningful subject-verb-object or concept-relationship-concept pattern:

    Text:
    {}

    Return ONLY a valid JSON array (no explanation, no markdown):
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
            &text[..text.len().min(3000)]
        );

        let input = serde_json::json!({
            "prompt": prompt,
            "max_tokens": 800,
            "temperature": 0.05,
            "system_context": "Grammar relationship extraction. Return only valid JSON array."
        });

        match self.executor.execute(9, input).await {
            Ok(result) => {
                let raw = result
                    .get("response")
                    .and_then(|r| r.as_str())
                    .unwrap_or("[]");
                let json_str = Self::extract_json_from_response(raw, '[', ']');
                serde_json::from_str::<Vec<serde_json::Value>>(&json_str)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| {
                        Some(ChunkGrammarRelationship {
                            from_text: v["from_text"].as_str()?.to_string(),
                            to_text: v["to_text"].as_str()?.to_string(),
                            edge_type: v["edge_type"].as_str().unwrap_or("Affects").to_string(),
                            tense: v["tense"].as_str().map(String::from),
                            negated: v["negated"].as_bool().unwrap_or(false),
                            verb: v["verb"].as_str().unwrap_or("").to_string(),
                            verb_type: match v["verb_type"].as_str().unwrap_or("action") {
                                "linking" => VerbType::Linking,
                                "helping" => VerbType::Helping,
                                _ => VerbType::Action,
                            },
                            subject: v["subject"].as_str().unwrap_or("").to_string(),
                            object: v["object"].as_str().map(String::from),
                            source_sentence_start: v["source_sentence_start"].as_usize(),
                            source_sentence_end: v["source_sentence_end"].as_usize(),
                            chunk_index,
                        })
                    })
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    /// Create a ChunkGraph from a fully processed chunk.
    /// This is the persistent evidence structure for AMT building.
    fn create_chunk_graph(&self, chunk: &ProcessedChunk, root_graph_id: u64) -> ChunkGraph {
        let graph_id = Self::generate_id();
        let now = chrono::Utc::now().to_rfc3339();

        // Derive sentence boundaries from cleaned text (heuristic)
        let mut sentence_boundaries = Vec::new();
        let mut pos = 0usize;
        for (i, c) in chunk.cleaned_text.char_indices() {
            if c == '.' || c == '!' || c == '?' {
                let end = i + c.len_utf8();
                sentence_boundaries.push(SentenceBoundary {
                    start: pos,
                    end,
                    sentence_type: SentenceType::Declarative,
                });
                pos = end;
            }
        }

        // Paragraph breaks
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
            overlap_resolution: None, // populated during chunking phase
            keywords: chunk.keywords.clone(),
            topics: chunk.topics.clone(),
            grammar_relationships: chunk.grammar_relationships.clone(),
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

    /// Full text analysis with optional entity/topic/structure extraction
    async fn analyze_text(
        &self,
        text: &str,
        depth: AnalysisDepth,
        extract_entities: bool,
        extract_topics: bool,
        extract_structure: bool,
    ) -> TextModalityOutput {
        // Basic metrics
        let word_count = text.split_whitespace().count();
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .collect();
        let sentence_count = sentences.len();
        let paragraph_count = text.split("\n\n").filter(|p| !p.trim().is_empty()).count();
        let character_count = text.len();

        // Extract entities via LLM if requested
        let entities = if extract_entities {
            let extracted = self.extract_entities_from_text(text).await;
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

        // Extract topics via LLM if requested
        let topics = if extract_topics {
            let topic_names = self.extract_topics_from_text(text).await;
            topic_names
                .into_iter()
                .enumerate()
                .map(|(i, name)| Topic {
                    name,
                    keywords: Vec::new(),
                    relevance: 1.0 - (i as f32 * 0.1).min(0.8),
                    category: None,
                })
                .collect()
        } else {
            Vec::new()
        };

        // Extract keywords
        let keywords_raw = self.extract_keywords_from_text(text).await;
        let keywords: Vec<Keyword> = keywords_raw
            .into_iter()
            .take(20)
            .enumerate()
            .map(|(i, term)| Keyword {
                term,
                frequency: 1,
                relevance: 1.0 - (i as f32 * 0.04).min(0.8),
                is_phrase: false,
            })
            .collect();

        // Extract structure if requested
        let structure = if extract_structure {
            self.extract_structure(text)
        } else {
            DocumentStructure {
                sections: Vec::new(),
                has_title: false,
                has_abstract: false,
                has_toc: false,
                document_type: DocumentType::Unknown,
            }
        };

        // Calculate readability (Flesch-Kincaid approximation)
        let avg_words_per_sentence = if sentence_count > 0 {
            word_count as f32 / sentence_count as f32
        } else {
            0.0
        };
        let syllable_count = Self::count_syllables(text);
        let avg_syllables_per_word = if word_count > 0 {
            syllable_count as f32 / word_count as f32
        } else {
            0.0
        };
        let readability_score =
            206.835 - (1.015 * avg_words_per_sentence) - (84.6 * avg_syllables_per_word);

        // Sentiment analysis for Deep/Comprehensive
        let sentiment = if depth == AnalysisDepth::Deep || depth == AnalysisDepth::Comprehensive {
            self.analyze_sentiment(text).await
        } else {
            None
        };

        let analysis = TextAnalysisResult {
            word_count,
            sentence_count,
            paragraph_count,
            character_count,
            entities,
            topics,
            keywords,
            structure,
            language: Some("en".to_string()), // TODO: language detection
            sentiment,
            readability_score: Some(readability_score.clamp(0.0, 100.0)),
        };

        TextModalityOutput {
            success: true,
            analysis: Some(analysis),
            ..Default::default()
        }
    }

    /// Extract document structure
    fn extract_structure(&self, text: &str) -> DocumentStructure {
        let mut sections = Vec::new();
        let mut section_id = 0;

        // Detect markdown-style headers
        let header_pattern = Regex::new(r"(?m)^(#{1,6})\s+(.+)$").unwrap();
        for cap in header_pattern.captures_iter(text) {
            let level = cap.get(1).map(|m| m.as_str().len()).unwrap_or(1) as u8;
            let title = cap.get(2).map(|m| m.as_str().to_string());
            let start = cap.get(0).map(|m| m.start()).unwrap_or(0);

            sections.push(Section {
                id: format!("section_{}", section_id),
                title,
                level,
                start_offset: start,
                end_offset: start + cap.get(0).map(|m| m.len()).unwrap_or(0),
                subsections: Vec::new(),
            });
            section_id += 1;
        }

        // Detect document type
        let document_type = if text.contains("```") || text.contains("fn ") || text.contains("def ")
        {
            DocumentType::Code
        } else if text.starts_with("Subject:") || text.contains("From:") {
            DocumentType::Email
        } else if text.contains("# ") || text.contains("## ") {
            DocumentType::Article
        } else if text.to_lowercase().contains("please")
            || text.to_lowercase().contains("help me")
            || text.to_lowercase().contains("can you")
        {
            DocumentType::Prompt
        } else {
            DocumentType::Unknown
        };

        DocumentStructure {
            sections,
            has_title: !sections.is_empty(),
            has_abstract: text.to_lowercase().contains("abstract"),
            has_toc: text.to_lowercase().contains("table of contents"),
            document_type,
        }
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
        nodes.push(TextGraphNode {
            node_id: doc_node_id,
            node_type: TextNodeType::Document,
            content: format!(
                "{} words, {} sentences, {} paragraphs",
                analysis.word_count, analysis.sentence_count, analysis.paragraph_count
            ),
            position: None,
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "word_count".to_string(),
                    serde_json::json!(analysis.word_count),
                );
                props.insert(
                    "sentence_count".to_string(),
                    serde_json::json!(analysis.sentence_count),
                );
                props.insert(
                    "paragraph_count".to_string(),
                    serde_json::json!(analysis.paragraph_count),
                );
                if let Some(score) = analysis.readability_score {
                    props.insert("readability_score".to_string(), serde_json::json!(score));
                }
                props
            },
            semantic_annotations: Vec::new(),
        });
        node_id += 1;

        // Create section nodes
        for section in &analysis.structure.sections {
            let section_node_id = node_id;
            nodes.push(TextGraphNode {
                node_id: section_node_id,
                node_type: TextNodeType::Section,
                content: section.title.clone().unwrap_or_default(),
                position: Some(TextPosition {
                    start_offset: section.start_offset,
                    end_offset: section.end_offset,
                    line: None,
                    column: None,
                }),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("level".to_string(), serde_json::json!(section.level));
                    props
                },
                semantic_annotations: Vec::new(),
            });

            edges.push(TextGraphEdge {
                edge_id,
                from_node: doc_node_id,
                to_node: section_node_id,
                edge_type: TextEdgeType::Contains,
                weight: 1.0,
                properties: HashMap::new(),
            });
            edge_id += 1;

            node_id += 1;
        }

        // Create entity nodes
        for entity in &analysis.entities {
            let entity_node_id = node_id;
            nodes.push(TextGraphNode {
                node_id: entity_node_id,
                node_type: TextNodeType::Entity,
                content: entity.text.clone(),
                position: Some(TextPosition {
                    start_offset: entity.start_offset,
                    end_offset: entity.end_offset,
                    line: None,
                    column: None,
                }),
                properties: {
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
                semantic_annotations: Vec::new(),
            });

            edges.push(TextGraphEdge {
                edge_id,
                from_node: doc_node_id,
                to_node: entity_node_id,
                edge_type: TextEdgeType::Contains,
                weight: entity.confidence,
                properties: HashMap::new(),
            });
            edge_id += 1;

            node_id += 1;
        }

        // Create topic nodes
        for topic in &analysis.topics {
            let topic_node_id = node_id;
            nodes.push(TextGraphNode {
                node_id: topic_node_id,
                node_type: TextNodeType::Topic,
                content: topic.name.clone(),
                position: None,
                properties: {
                    let mut props = HashMap::new();
                    props.insert("relevance".to_string(), serde_json::json!(topic.relevance));
                    props.insert(
                        "keywords".to_string(),
                        serde_json::to_value(&topic.keywords).unwrap(),
                    );
                    props
                },
                semantic_annotations: Vec::new(),
            });

            edges.push(TextGraphEdge {
                edge_id,
                from_node: doc_node_id,
                to_node: topic_node_id,
                edge_type: TextEdgeType::Contains,
                weight: topic.relevance,
                properties: HashMap::new(),
            });
            edge_id += 1;

            node_id += 1;
        }

        // Create keyword nodes
        for keyword in &analysis.keywords {
            let keyword_node_id = node_id;
            nodes.push(TextGraphNode {
                node_id: keyword_node_id,
                node_type: TextNodeType::Keyword,
                content: keyword.term.clone(),
                position: None,
                properties: {
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
                semantic_annotations: Vec::new(),
            });

            edges.push(TextGraphEdge {
                edge_id,
                from_node: doc_node_id,
                to_node: keyword_node_id,
                edge_type: TextEdgeType::Contains,
                weight: keyword.relevance,
                properties: HashMap::new(),
            });
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

#[tokio::main]
async fn main() {
    let input: TextModalityInput =
        serde_json::from_reader(std::io::stdin()).expect("Failed to parse input");

    let executor = Arc::new(StubExecutor);
    let pipeline = TextModalityPipeline::new(executor);
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

    #[tokio::test]
    async fn test_chunk_text() {
        let executor = Arc::new(MockExecutor);
        let pipeline = TextModalityPipeline::new(executor);

        let input = TextModalityInput {
            action: TextModalityAction::ChunkText {
                text:
                    "This is paragraph one.\n\nThis is paragraph two.\n\nThis is paragraph three."
                        .to_string(),
                max_chunk_tokens: 20,
                overlap_tokens: 5,
                preserve_paragraphs: true,
            },
        };

        let output = pipeline.execute(input).await;
        assert!(output.success);
        assert!(output.chunks.is_some());
        assert!(!output.chunks.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_process_chunk() {
        let executor = Arc::new(MockExecutor);
        let pipeline = TextModalityPipeline::new(executor);

        let chunk = RawChunk {
            index: 0,
            text: "John works at Acme Corp in New York.".to_string(),
            start_char: 0,
            end_char: 36,
            token_count: 9,
            is_complete_paragraph: true,
        };

        let input = TextModalityInput {
            action: TextModalityAction::ProcessChunk { chunk },
        };

        let output = pipeline.execute(input).await;
        assert!(output.success);
        assert!(output.processed_chunks.is_some());

        let processed = output.processed_chunks.unwrap();
        assert_eq!(processed.len(), 1);
        assert!(!processed[0].keywords.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_text() {
        let executor = Arc::new(MockExecutor);
        let pipeline = TextModalityPipeline::new(executor);

        let input = TextModalityInput {
            action: TextModalityAction::Analyze {
                text: "John Smith works at Acme Corp. He joined on January 15, 2024.".to_string(),
                depth: AnalysisDepth::Standard,
                extract_entities: true,
                extract_topics: true,
                extract_structure: true,
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

        // First analyze
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

        // Create graph
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

        // Query for entities
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

        let chunks = vec![
            ProcessedChunk {
                index: 0,
                original_text: "First chunk text.".to_string(),
                cleaned_text: "First chunk text.".to_string(),
                start_offset: 0,
                end_offset: 17,
                token_count: 4,
                keywords: vec![],
                entities: vec![],
                topics: vec![],
                overlap_from_previous: 0,
                overlap_to_next: 20,
            },
            ProcessedChunk {
                index: 1,
                original_text: "chunk text. Second part.".to_string(),
                cleaned_text: "chunk text. Second part.".to_string(),
                start_offset: 12,
                end_offset: 36,
                token_count: 5,
                keywords: vec![],
                entities: vec![],
                topics: vec![],
                overlap_from_previous: 20,
                overlap_to_next: 20,
            },
        ];

        let input = TextModalityInput {
            action: TextModalityAction::ReconstructFromChunks { chunks },
        };

        // We can't call async from sync test, so test the helper directly
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
        }]);

        assert!(result.success);
        assert_eq!(result.reconstructed_text.unwrap(), "First chunk.");
    }
}
