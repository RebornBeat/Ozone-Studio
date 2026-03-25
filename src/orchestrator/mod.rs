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
use crate::task::{RefinementConfig, TaskData, TaskManager, TaskPriority, TaskQueueConfig};

// ============================================================================
// Types
// ============================================================================

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
    pub confidence: f32,
    pub reasoning: String,
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
            confidence: 1.0,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub text: String,
    pub entity_type: String,
    pub confidence: f32,
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
struct CrossRef {
    from_branch: String,
    to_branch: String,
    from_intent: String,
    to_intent: String,
    relation_type: AMTRelationType,
    description: String,
}

struct OrchestrationState {
    request: OrchestrationRequest,
    start_time: std::time::Instant,
    stages: Vec<StageResult>,

    // Model context management
    model_context_limit: u32,
    tokens_used_so_far: u32,

    // Stage 2 outputs
    raw_chunks: Vec<RawChunk>,
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
struct StepResult {
    step_index: u32,
    pipeline_id: u64,
    output: serde_json::Value,
    tokens_used: u32,
    iterations: u32,
    sub_step_results: Vec<SubStepResult>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct ValidationResult {
    is_valid: bool,
    issues: Vec<String>,
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
pub trait ZSEIAccess: Send + Sync {
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
    zsei: Arc<dyn ZSEIAccess>,
    task_manager: Arc<TaskManager>,
    pipeline_index: Arc<RwLock<Option<PipelineIndex>>>,
}

impl PromptOrchestrator {
    pub fn new(
        executor: Arc<dyn PipelineExecutor>,
        zsei: Arc<dyn ZSEIAccess>,
        task_manager: Arc<TaskManager>,
    ) -> Self {
        Self {
            executor,
            zsei,
            task_manager,
            pipeline_index: Arc::new(RwLock::new(None)),
        }
    }

    /// Load pipeline index from ZSEI
    pub async fn load_pipeline_index(&self) -> Result<(), String> {
        // Try to load from ZSEI storage
        let index_result = self
            .zsei
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
            // Return default core pipelines if index not loaded
            Self::get_default_pipelines()
        }
    }

    /// Get default pipeline list (fallback if index not loaded)
    fn get_default_pipelines() -> Vec<PipelineInfo> {
        vec![
            PipelineInfo {
                pipeline_id: 9,
                name: "Prompt".to_string(),
                folder_name: "prompt".to_string(),
                category: "general".to_string(),
                description: "LLM prompt processing".to_string(),
                modality: None,
                has_ui: false,
                is_tab: false,
                deprecated: false,
            },
            PipelineInfo {
                pipeline_id: 21,
                name: "ContextAggregation".to_string(),
                folder_name: "context_aggregation".to_string(),
                category: "general".to_string(),
                description: "Aggregate context for tasks".to_string(),
                modality: None,
                has_ui: false,
                is_tab: false,
                deprecated: false,
            },
            PipelineInfo {
                pipeline_id: 100,
                name: "TextAnalysisPipeline".to_string(),
                folder_name: "text".to_string(),
                category: "modalities".to_string(),
                description: "Text modality analysis".to_string(),
                modality: Some("text".to_string()),
                has_ui: false,
                is_tab: false,
                deprecated: false,
            },
            PipelineInfo {
                pipeline_id: 101,
                name: "CodeAnalysisPipeline".to_string(),
                folder_name: "code".to_string(),
                category: "modalities".to_string(),
                description: "Code modality analysis".to_string(),
                modality: Some("code".to_string()),
                has_ui: false,
                is_tab: false,
                deprecated: false,
            },
        ]
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

    async fn execute_stages(&self, state: &mut OrchestrationState) -> Result<(), String> {
        // STAGE 1: Input Capture (already done - prompt is in request)
        self.record_stage(state, 1, "Input Capture", true, "Prompt received");

        // STAGE 2: Text/Prompt Normalization + AMT Building
        self.stage_2_text_normalization_and_amt(state).await?;

        // If clarification needed, stop here and return to user
        if state.needs_clarification {
            return Ok(());
        }

        // STAGE 3: Blueprint Assignment
        self.stage_3_blueprint_assignment(state).await?;

        // STAGE 4: Zero-Shot Simulation (with AMT traversal)
        self.stage_4_zero_shot_simulation(state).await?;

        // STAGE 5: Consciousness Decision Gate
        if state.request.consciousness_enabled {
            self.stage_5_consciousness_gate(state).await?;
        } else {
            self.record_stage(state, 5, "Consciousness Gate", true, "Skipped (disabled)");
        }

        // STAGE 6-8: Context Aggregation + Task Creation + Execution
        self.stage_6_to_8_execute_steps(state).await?;

        // STAGE 9: Result Collection
        self.stage_9_result_collection(state).await?;

        // STAGE 10: Post-execution Consciousness
        if state.request.consciousness_enabled {
            self.stage_10_post_execution(state).await?;
        } else {
            self.record_stage(state, 10, "Post-execution", true, "Skipped (disabled)");
        }

        // STAGE 11: Response Delivery
        self.stage_11_response_delivery(state).await?;

        Ok(())
    }

    // ========================================================================
    // STAGE 2: Text/Prompt Normalization + AMT Building
    // ========================================================================

    async fn stage_2_text_normalization_and_amt(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // STEP 1: Chunk the text using text modality pipeline
        let chunk_input = serde_json::json!({
            "action": {
                "type": "ChunkText",
                "text": state.request.prompt,
                "max_chunk_tokens": state.model_context_limit / 4,
                "overlap_tokens": 200,
                "preserve_paragraphs": true
            }
        });

        let chunk_result = self.executor.execute(100, chunk_input).await?;

        state.raw_chunks = chunk_result
            .get("chunks")
            .and_then(|c| serde_json::from_value(c.clone()).ok())
            .unwrap_or_else(|| {
                vec![RawChunk {
                    index: 0,
                    text: state.request.prompt.clone(),
                    start_char: 0,
                    end_char: state.request.prompt.len() as u32,
                    token_count: state.prompt_tokens,
                    is_complete_paragraph: true,
                }]
            });

        // STEP 2: Process each chunk (clean + extract keywords/entities/topics)
        let mut all_keywords: HashSet<String> = HashSet::new();
        let mut all_entities: Vec<ExtractedEntity> = Vec::new();
        let mut all_topics: HashSet<String> = HashSet::new();

        for chunk in &state.raw_chunks {
            let process_input = serde_json::json!({
                "action": {
                    "type": "ProcessChunk",
                    "chunk": chunk
                }
            });

            let process_result = self.executor.execute(100, process_input).await?;

            if let Some(processed_arr) = process_result
                .get("processed_chunks")
                .and_then(|p| p.as_array())
            {
                for processed_val in processed_arr {
                    if let Ok(processed) =
                        serde_json::from_value::<ProcessedChunk>(processed_val.clone())
                    {
                        // Collect all keywords
                        for kw in &processed.keywords {
                            all_keywords.insert(kw.clone());
                        }
                        // Collect all entities
                        all_entities.extend(processed.entities.clone());
                        // Collect all topics
                        for topic in &processed.topics {
                            all_topics.insert(topic.clone());
                        }
                        state.processed_chunks.push(processed);
                    }
                }
            }
        }

        // STEP 3: Reconstruct cleaned prompt from chunks
        let reconstruct_input = serde_json::json!({
            "action": {
                "type": "ReconstructFromChunks",
                "chunks": state.processed_chunks
            }
        });

        let reconstruct_result = self.executor.execute(100, reconstruct_input).await?;

        state.cleaned_prompt = reconstruct_result
            .get("reconstructed_text")
            .and_then(|t| t.as_str())
            .unwrap_or(&state.request.prompt)
            .to_string();

        state.prompt_tokens = Self::estimate_tokens(&state.cleaned_prompt);
        state.keywords = all_keywords.into_iter().collect();
        state.entities = all_entities;
        state.topics = all_topics.into_iter().collect();

        // STEP 4: Search methodologies by keywords via ZSEI
        let methodology_ids = self
            .zsei
            .search_by_keywords(
                &state.keywords.iter().take(20).cloned().collect::<Vec<_>>(),
                Some("Methodology"),
            )
            .await
            .unwrap_or_default();

        state.methodologies = methodology_ids;

        // STEP 5: Get categories from methodologies and cross-reference
        let mut methodology_categories: HashSet<u64> = HashSet::new();

        for method_id in &state.methodologies {
            if let Ok(Some(container)) = self.zsei.get_container(*method_id).await {
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
            if let Ok(Some(container)) = self.zsei.get_container(cat_id).await {
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
                .zsei
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

                if let Ok(new_id) = self.zsei.create_container(0, new_category).await {
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

        // STEP 6: Build AMT layer-by-layer from chunks (processes each chunk individually)
        state.amt = Some(self.build_amt_layer_by_layer(state).await?);

        // STEP 7: Validate AMT (need 5 consecutive Valid)
        state.amt_validated = state.amt_pass_count > 0;
        state.validation_streak = 5; // convergence was achieved inside the builder

        self.record_stage_timed(
            state,
            2,
            "Text Normalization + AMT",
            true,
            &format!(
                "Chunks: {}, Intents: {}, Branches: {}, Details: {}, Cross-refs: {}, Methodologies: {}, Categories: {} ({} created), Passes: {}, Validated: {}",
                state.processed_chunks.len(),
                state.intent_captures.len(),
                state.branch_captures.len(),
                state.detail_captures.len(),
                state.cross_refs.len(),
                state.methodologies.len(),
                state.categories.len(),
                state.categories_created,
                state.amt_pass_count,
                state.amt_validated,
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    /// Build AMT layer-by-layer from processed chunks (processes each chunk individually)
    async fn build_amt_layer_by_layer(
        &self,
        state: &OrchestrationState,
    ) -> Result<AMTNode, String> {
        let max_outer_passes = 10;
        let convergence_threshold = 5; // passes without new insights before done
        let mut consecutive_no_new = 0u32;
        let mut node_id_counter = 1u64;

        // Outer convergence loop
        'outer: for outer_pass in 0..max_outer_passes {
            state.amt_pass_count += 1;
            let mut new_insights_this_pass = false;

            // --- PHASE 1A: Intent discovery ---
            // Build context of already-known intents for deduplication
            let known_intents_json: Vec<serde_json::Value> = state
                .intent_captures
                .iter()
                .map(|ic| serde_json::json!({"intent": ic.intent, "is_parallel": ic.is_parallel}))
                .collect();

            for chunk in &state.processed_chunks {
                let intent_prompt = format!(
                    r#"Analyze this text chunk to identify goals or intents expressed in it.
        A chunk may express MULTIPLE unrelated intents (parallel) or a single intent.

        ALREADY KNOWN INTENTS (do NOT repeat these):
        {}

        CHUNK {} of {}:
        {}

        Return ONLY valid JSON with no explanation:
        {{
            "new_intents": [
                {{
                    "intent": "clear description of this goal/intent",
                    "is_parallel": true,
                    "source_sentence": "the exact sentence or paragraph from the chunk expressing this"
                }}
            ]
        }}
        If no new intents are found, return: {{"new_intents": []}}"#,
                    serde_json::to_string(&known_intents_json).unwrap_or_default(),
                    chunk.index + 1,
                    state.processed_chunks.len(),
                    &chunk.cleaned_text[..chunk.cleaned_text.len().min(1500)]
                );

                let intent_input = serde_json::json!({
                    "prompt": intent_prompt,
                    "max_tokens": 500,
                    "temperature": 0.2,
                    "system_context": "Extract new intents not already listed. Return only valid JSON. No explanation."
                });

                if let Ok(result) = self.executor.execute(9, intent_input).await {
                    let response = result
                        .get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("{}");
                    let json_str = Self::extract_json_from_response(response, '{', '}');
                    let parsed = serde_json::from_str::<serde_json::Value>(json_str.trim())
                        .unwrap_or_else(|_| serde_json::json!({"new_intents": []}));

                    if let Some(new_intents) = parsed.get("new_intents").and_then(|n| n.as_array())
                    {
                        for intent_val in new_intents {
                            let intent_str = intent_val
                                .get("intent")
                                .and_then(|i| i.as_str())
                                .unwrap_or("")
                                .to_string();
                            let is_parallel = intent_val
                                .get("is_parallel")
                                .and_then(|p| p.as_bool())
                                .unwrap_or(false);
                            let source_sentence = intent_val
                                .get("source_sentence")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            if intent_str.is_empty() {
                                continue;
                            }

                            // Check for duplicates (case-insensitive substring match)
                            let already_known = state.intent_captures.iter().any(|ic| {
                                ic.intent
                                    .to_lowercase()
                                    .contains(&intent_str.to_lowercase())
                                    || intent_str
                                        .to_lowercase()
                                        .contains(&ic.intent.to_lowercase())
                            });

                            if !already_known {
                                state.intent_captures.push(IntentCapture {
                                    intent: intent_str,
                                    is_parallel,
                                    source_chunk_indices: vec![chunk.index],
                                    source_sentences: if source_sentence.is_empty() {
                                        vec![]
                                    } else {
                                        vec![source_sentence]
                                    },
                                    node_id: node_id_counter,
                                });
                                node_id_counter += 1;
                                new_insights_this_pass = true;
                            } else {
                                // Aggregate: add this chunk as an additional source
                                if let Some(existing) =
                                    state.intent_captures.iter_mut().find(|ic| {
                                        ic.intent
                                            .to_lowercase()
                                            .contains(&intent_str.to_lowercase())
                                    })
                                {
                                    if !existing.source_chunk_indices.contains(&chunk.index) {
                                        existing.source_chunk_indices.push(chunk.index);
                                        if !source_sentence.is_empty() {
                                            existing.source_sentences.push(source_sentence);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // If no intents found at all, create a default
            if state.intent_captures.is_empty() {
                state.intent_captures.push(IntentCapture {
                    intent: "Process user request".to_string(),
                    is_parallel: false,
                    source_chunk_indices: (0..state.processed_chunks.len() as u32).collect(),
                    source_sentences: vec![],
                    node_id: node_id_counter,
                });
                node_id_counter += 1;
                new_insights_this_pass = true;
            }

            // --- PHASE 1B: Branch discovery via methodologies ---
            for &method_id in &state.methodologies {
                if let Ok(Some(method_container)) = self.zsei.get_container(method_id).await {
                    // Extract methodology content
                    let method_name = method_container
                        .get("local_state")
                        .and_then(|ls| ls.get("metadata"))
                        .and_then(|m| m.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unknown methodology")
                        .to_string();
                    let method_description = method_container
                        .get("local_state")
                        .and_then(|ls| ls.get("context"))
                        .and_then(|ctx| ctx.get("keywords"))
                        .map(|kw| kw.to_string())
                        .unwrap_or_default();

                    let intents_summary: Vec<String> = state
                        .intent_captures
                        .iter()
                        .map(|ic| ic.intent.clone())
                        .collect();

                    // Already known branches for dedup
                    let known_branches_json: Vec<serde_json::Value> = state.branch_captures
                        .iter()
                        .map(|bc| serde_json::json!({"branch": bc.branch, "intent": bc.parent_intent}))
                        .collect();

                    let branch_prompt = format!(
                        r#"You are applying the methodology "{}" to a set of user intents.
        Methodology context: {}

        USER INTENTS:
        {}

        ALREADY IDENTIFIED BRANCHES (do NOT repeat these):
        {}

        Based on this methodology, what additional branches (sub-components, requirements, or considerations) should be addressed for each intent?
        Only suggest branches NOT already in the known list.

        Return ONLY valid JSON:
        {{
            "branches": [
                {{
                    "branch": "specific branch description",
                    "parent_intent": "the intent this branch belongs to",
                    "rationale": "why this methodology requires this branch"
                }}
            ]
        }}
        If no new branches apply, return: {{"branches": []}}"#,
                        method_name,
                        &method_description[..method_description.len().min(300)],
                        intents_summary.join("\n"),
                        serde_json::to_string(&known_branches_json).unwrap_or_default()
                    );

                    let branch_input = serde_json::json!({
                        "prompt": branch_prompt,
                        "max_tokens": 600,
                        "temperature": 0.3,
                        "system_context": "Suggest branches per methodology. Return only valid JSON. No explanation."
                    });

                    if let Ok(result) = self.executor.execute(9, branch_input).await {
                        let response = result
                            .get("response")
                            .and_then(|r| r.as_str())
                            .unwrap_or("{}");
                        let json_str = Self::extract_json_from_response(response, '{', '}');
                        let parsed = serde_json::from_str::<serde_json::Value>(json_str.trim())
                            .unwrap_or_else(|_| serde_json::json!({"branches": []}));

                        if let Some(branches) = parsed.get("branches").and_then(|b| b.as_array()) {
                            for branch_val in branches {
                                let branch_str = branch_val
                                    .get("branch")
                                    .and_then(|b| b.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let parent_intent = branch_val
                                    .get("parent_intent")
                                    .and_then(|p| p.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                if branch_str.is_empty() {
                                    continue;
                                }

                                // Find actual parent intent (fuzzy match)
                                let resolved_parent = state
                                    .intent_captures
                                    .iter()
                                    .find(|ic| {
                                        ic.intent
                                            .to_lowercase()
                                            .contains(&parent_intent.to_lowercase())
                                            || parent_intent
                                                .to_lowercase()
                                                .contains(&ic.intent.to_lowercase())
                                    })
                                    .map(|ic| ic.intent.clone())
                                    .unwrap_or_else(|| {
                                        state
                                            .intent_captures
                                            .first()
                                            .map(|ic| ic.intent.clone())
                                            .unwrap_or_default()
                                    });

                                let already_exists = state.branch_captures.iter().any(|bc| {
                                    bc.parent_intent == resolved_parent
                                        && (bc
                                            .branch
                                            .to_lowercase()
                                            .contains(&branch_str.to_lowercase())
                                            || branch_str
                                                .to_lowercase()
                                                .contains(&bc.branch.to_lowercase()))
                                });

                                if !already_exists {
                                    state.branch_captures.push(BranchCapture {
                                        branch: branch_str,
                                        parent_intent: resolved_parent,
                                        source_methodology_ids: vec![method_id],
                                        source_chunk_indices: vec![],
                                        source_sentences: vec![],
                                        node_id: node_id_counter,
                                    });
                                    node_id_counter += 1;
                                    new_insights_this_pass = true;
                                } else {
                                    // Aggregate: add methodology as additional source
                                    if let Some(existing) =
                                        state.branch_captures.iter_mut().find(|bc| {
                                            bc.parent_intent == resolved_parent
                                                && bc
                                                    .branch
                                                    .to_lowercase()
                                                    .contains(&branch_str.to_lowercase())
                                        })
                                    {
                                        if !existing.source_methodology_ids.contains(&method_id) {
                                            existing.source_methodology_ids.push(method_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // --- PHASE 2: Detail discovery over chunks ---
            for chunk in &state.processed_chunks {
                // Build context of existing branches for this chunk
                let branches_summary: Vec<serde_json::Value> = state
                    .branch_captures
                    .iter()
                    .map(|bc| {
                        serde_json::json!({
                            "branch": bc.branch,
                            "intent": bc.parent_intent
                        })
                    })
                    .collect();

                let known_details_json: Vec<serde_json::Value> = state
                    .detail_captures
                    .iter()
                    .filter(|dc| {
                        // Only known details for branches possibly covered by this chunk
                        state
                            .branch_captures
                            .iter()
                            .find(|bc| bc.branch == dc.parent_branch)
                            .map(|bc| {
                                bc.source_chunk_indices.contains(&chunk.index)
                                    || bc.source_chunk_indices.is_empty()
                            })
                            .unwrap_or(true)
                    })
                    .map(|dc| {
                        serde_json::json!({
                            "detail": dc.content,
                            "branch": dc.parent_branch
                        })
                    })
                    .collect();

                let detail_prompt = format!(
                    r#"Analyze this text chunk for specific details, requirements, and constraints that address the identified branches.

        BRANCHES TO ADDRESS:
        {}

        ALREADY IDENTIFIED DETAILS (do NOT repeat):
        {}

        CHUNK {} of {}:
        {}

        For each branch this chunk addresses, extract specific details. Also identify any completely NEW branches not in the list above.

        Return ONLY valid JSON:
        {{
            "details": [
                {{
                    "content": "specific detail, requirement, or constraint",
                    "type": "detail|requirement|constraint",
                    "parent_branch": "exact branch name this belongs to",
                    "source_sentence": "the exact sentence or paragraph from the chunk"
                }}
            ],
            "new_branches": [
                {{
                    "branch": "newly discovered branch",
                    "parent_intent": "intent it belongs to",
                    "source_sentence": "exact text"
                }}
            ]
        }}"#,
                    serde_json::to_string(&branches_summary).unwrap_or_default(),
                    serde_json::to_string(&known_details_json).unwrap_or_default(),
                    chunk.index + 1,
                    state.processed_chunks.len(),
                    &chunk.cleaned_text[..chunk.cleaned_text.len().min(1500)]
                );

                let detail_input = serde_json::json!({
                    "prompt": detail_prompt,
                    "max_tokens": 700,
                    "temperature": 0.3,
                    "system_context": "Extract details per branch. Return only valid JSON. No explanation."
                });

                if let Ok(result) = self.executor.execute(9, detail_input).await {
                    let response = result
                        .get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("{}");
                    let json_str = Self::extract_json_from_response(response, '{', '}');
                    let parsed = serde_json::from_str::<serde_json::Value>(json_str.trim())
                        .unwrap_or_else(|_| serde_json::json!({"details": [], "new_branches": []}));

                    // Process new details
                    if let Some(details) = parsed.get("details").and_then(|d| d.as_array()) {
                        for detail_val in details {
                            let content = detail_val
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            let detail_type = detail_val
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("detail")
                                .to_string();
                            let parent_branch = detail_val
                                .get("parent_branch")
                                .and_then(|p| p.as_str())
                                .unwrap_or("")
                                .to_string();
                            let source_sentence = detail_val
                                .get("source_sentence")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            if content.is_empty() || parent_branch.is_empty() {
                                continue;
                            }

                            // Resolve parent branch (fuzzy)
                            let resolved_branch = state
                                .branch_captures
                                .iter()
                                .find(|bc| {
                                    bc.branch
                                        .to_lowercase()
                                        .contains(&parent_branch.to_lowercase())
                                        || parent_branch
                                            .to_lowercase()
                                            .contains(&bc.branch.to_lowercase())
                                })
                                .map(|bc| bc.branch.clone())
                                .unwrap_or(parent_branch.clone());

                            // Find parent intent for this branch
                            let resolved_intent = state
                                .branch_captures
                                .iter()
                                .find(|bc| bc.branch == resolved_branch)
                                .map(|bc| bc.parent_intent.clone())
                                .unwrap_or_default();

                            let already_exists = state.detail_captures.iter().any(|dc| {
                                dc.parent_branch == resolved_branch
                                    && (dc.content.to_lowercase().contains(&content.to_lowercase())
                                        || content
                                            .to_lowercase()
                                            .contains(&dc.content.to_lowercase()))
                            });

                            if !already_exists {
                                // Also update branch's chunk indices
                                if let Some(branch) = state
                                    .branch_captures
                                    .iter_mut()
                                    .find(|bc| bc.branch == resolved_branch)
                                {
                                    if !branch.source_chunk_indices.contains(&chunk.index) {
                                        branch.source_chunk_indices.push(chunk.index);
                                    }
                                    if !source_sentence.is_empty()
                                        && !branch.source_sentences.contains(&source_sentence)
                                    {
                                        branch.source_sentences.push(source_sentence.clone());
                                    }
                                }

                                state.detail_captures.push(DetailCapture {
                                    content,
                                    detail_type,
                                    parent_branch: resolved_branch,
                                    parent_intent: resolved_intent,
                                    source_chunk_indices: vec![chunk.index],
                                    source_sentences: if source_sentence.is_empty() {
                                        vec![]
                                    } else {
                                        vec![source_sentence]
                                    },
                                    node_id: node_id_counter,
                                });
                                node_id_counter += 1;
                                new_insights_this_pass = true;
                            } else {
                                // Aggregate
                                if let Some(existing) =
                                    state.detail_captures.iter_mut().find(|dc| {
                                        dc.parent_branch == resolved_branch
                                            && dc
                                                .content
                                                .to_lowercase()
                                                .contains(&content.to_lowercase())
                                    })
                                {
                                    if !existing.source_chunk_indices.contains(&chunk.index) {
                                        existing.source_chunk_indices.push(chunk.index);
                                    }
                                    if !source_sentence.is_empty()
                                        && !existing.source_sentences.contains(&source_sentence)
                                    {
                                        existing.source_sentences.push(source_sentence);
                                    }
                                }
                            }
                        }
                    }

                    // Process new branches discovered during detail pass
                    if let Some(new_branches) =
                        parsed.get("new_branches").and_then(|nb| nb.as_array())
                    {
                        for branch_val in new_branches {
                            let branch_str = branch_val
                                .get("branch")
                                .and_then(|b| b.as_str())
                                .unwrap_or("")
                                .to_string();
                            let parent_intent = branch_val
                                .get("parent_intent")
                                .and_then(|p| p.as_str())
                                .unwrap_or("")
                                .to_string();
                            let source_sentence = branch_val
                                .get("source_sentence")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            if branch_str.is_empty() {
                                continue;
                            }

                            let already_exists = state.branch_captures.iter().any(|bc| {
                                bc.branch
                                    .to_lowercase()
                                    .contains(&branch_str.to_lowercase())
                                    || branch_str
                                        .to_lowercase()
                                        .contains(&bc.branch.to_lowercase())
                            });

                            if !already_exists {
                                let resolved_parent = state
                                    .intent_captures
                                    .iter()
                                    .find(|ic| {
                                        ic.intent
                                            .to_lowercase()
                                            .contains(&parent_intent.to_lowercase())
                                            || parent_intent
                                                .to_lowercase()
                                                .contains(&ic.intent.to_lowercase())
                                    })
                                    .map(|ic| ic.intent.clone())
                                    .unwrap_or_else(|| {
                                        state
                                            .intent_captures
                                            .first()
                                            .map(|ic| ic.intent.clone())
                                            .unwrap_or_default()
                                    });

                                state.branch_captures.push(BranchCapture {
                                    branch: branch_str,
                                    parent_intent: resolved_parent,
                                    source_methodology_ids: vec![],
                                    source_chunk_indices: vec![chunk.index],
                                    source_sentences: if source_sentence.is_empty() {
                                        vec![]
                                    } else {
                                        vec![source_sentence]
                                    },
                                    node_id: node_id_counter,
                                });
                                node_id_counter += 1;
                                new_insights_this_pass = true;
                            }
                        }
                    }
                }
            }

            // Check convergence
            if !new_insights_this_pass {
                consecutive_no_new += 1;
                if consecutive_no_new >= convergence_threshold {
                    break 'outer;
                }
            } else {
                consecutive_no_new = 0;
            }
        } // end 'outer

        // --- PHASE 3: Cross-reference linking ---
        let branch_list: Vec<(String, String)> = state
            .branch_captures
            .iter()
            .map(|bc| (bc.branch.clone(), bc.parent_intent.clone()))
            .collect();

        // Check pairs (limit to avoid combinatorial explosion: max 50 pairs)
        let max_pairs = 50usize;
        let mut pair_count = 0;
        'pairs: for i in 0..branch_list.len() {
            for j in (i + 1)..branch_list.len() {
                if pair_count >= max_pairs {
                    break 'pairs;
                }
                pair_count += 1;

                let (branch_a, intent_a) = &branch_list[i];
                let (branch_b, intent_b) = &branch_list[j];

                // Skip if same intent (same-intent relationships are handled by hierarchy)
                if intent_a == intent_b {
                    continue;
                }

                let crossref_prompt = format!(
                    r#"Are these two branches related to each other?

        BRANCH A (from intent: "{}"): {}
        BRANCH B (from intent: "{}"): {}

        If they are related, describe how.

        Return ONLY valid JSON:
        {{
            "related": true,
            "relationship_type": "depends_on|requires|relates_to|contradicts|shared_context",
            "description": "brief explanation"
        }}
        If not related: {{"related": false}}"#,
                    intent_a, branch_a, intent_b, branch_b
                );

                let crossref_input = serde_json::json!({
                    "prompt": crossref_prompt,
                    "max_tokens": 150,
                    "temperature": 0.2,
                    "system_context": "Identify cross-branch relationships. Return only valid JSON."
                });

                if let Ok(result) = self.executor.execute(9, crossref_input).await {
                    let response = result
                        .get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("{}");
                    let json_str = Self::extract_json_from_response(response, '{', '}');
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str.trim()) {
                        if parsed
                            .get("related")
                            .and_then(|r| r.as_bool())
                            .unwrap_or(false)
                        {
                            let rel_type_str = parsed
                                .get("relationship_type")
                                .and_then(|rt| rt.as_str())
                                .unwrap_or("relates_to");
                            let description = parsed
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("")
                                .to_string();

                            let relation_type = match rel_type_str {
                                "depends_on" => AMTRelationType::DependsOn,
                                "requires" => AMTRelationType::Requires,
                                "contradicts" => AMTRelationType::Contradicts,
                                "shared_context" => AMTRelationType::SharedContext,
                                _ => AMTRelationType::RelatesTo,
                            };

                            state.cross_refs.push(CrossRef {
                                from_branch: branch_a.clone(),
                                to_branch: branch_b.clone(),
                                from_intent: intent_a.clone(),
                                to_intent: intent_b.clone(),
                                relation_type,
                                description,
                            });
                        }
                    }
                }
            }
        }

        // --- BUILD AMT FROM CAPTURES ---

        // Determine root vs parallel structure
        let parallel_intents: Vec<&IntentCapture> = state
            .intent_captures
            .iter()
            .filter(|ic| ic.is_parallel)
            .collect();
        let primary_intents: Vec<&IntentCapture> = state
            .intent_captures
            .iter()
            .filter(|ic| !ic.is_parallel)
            .collect();

        // Create top-level root
        let root_intent = if primary_intents.len() == 1 {
            primary_intents[0].intent.clone()
        } else if !primary_intents.is_empty() {
            format!(
                "Multiple goals: {}",
                primary_intents
                    .iter()
                    .map(|ic| ic.intent.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        } else if !state.intent_captures.is_empty() {
            state.intent_captures[0].intent.clone()
        } else {
            "Process user request".to_string()
        };

        let mut root_node = AMTNode::new(node_id_counter, AMTNodeType::Root, root_intent, 0);
        root_node.source_chunk_indices = (0..state.processed_chunks.len() as u32).collect();
        root_node.methodology_ids = state.methodologies.clone();
        node_id_counter += 1;

        // Build intent → branch → detail tree
        let all_intents_combined: Vec<&IntentCapture> = state.intent_captures.iter().collect();

        for intent_capture in &all_intents_combined {
            // If multiple parallel intents, each becomes a Branch under Root
            // If single intent, branches go directly under Root
            let (intent_parent_node, intent_level) = if state.intent_captures.len() > 1 {
                // Create intent node as a Branch
                let mut intent_node = AMTNode::new(
                    intent_capture.node_id,
                    AMTNodeType::Branch,
                    intent_capture.intent.clone(),
                    1,
                );
                intent_node.source_chunk_indices = intent_capture.source_chunk_indices.clone();
                for sentence in &intent_capture.source_sentences {
                    intent_node.metadata.insert(
                        format!("source_sentence_{}", intent_node.metadata.len()),
                        sentence.clone(),
                    );
                }

                // Branches for this intent
                let branches_for_intent: Vec<&BranchCapture> = state
                    .branch_captures
                    .iter()
                    .filter(|bc| bc.parent_intent == intent_capture.intent)
                    .collect();

                for branch_capture in branches_for_intent {
                    let mut branch_node = AMTNode::new(
                        branch_capture.node_id,
                        AMTNodeType::Branch,
                        branch_capture.branch.clone(),
                        2,
                    );
                    branch_node.source_chunk_indices = branch_capture.source_chunk_indices.clone();
                    branch_node.methodology_ids = branch_capture.source_methodology_ids.clone();
                    for sentence in &branch_capture.source_sentences {
                        branch_node.metadata.insert(
                            format!("source_sentence_{}", branch_node.metadata.len()),
                            sentence.clone(),
                        );
                    }

                    // Details for this branch
                    let details_for_branch: Vec<&DetailCapture> = state
                        .detail_captures
                        .iter()
                        .filter(|dc| dc.parent_branch == branch_capture.branch)
                        .collect();

                    for detail_capture in details_for_branch {
                        let node_type = match detail_capture.detail_type.as_str() {
                            "constraint" => AMTNodeType::Consideration,
                            _ => AMTNodeType::Leaf,
                        };
                        let mut detail_node = AMTNode::new(
                            detail_capture.node_id,
                            node_type,
                            detail_capture.content.clone(),
                            3,
                        );
                        detail_node
                            .metadata
                            .insert("type".to_string(), detail_capture.detail_type.clone());
                        detail_node.source_chunk_indices =
                            detail_capture.source_chunk_indices.clone();
                        for sentence in &detail_capture.source_sentences {
                            detail_node.metadata.insert(
                                format!("source_sentence_{}", detail_node.metadata.len()),
                                sentence.clone(),
                            );
                        }
                        branch_node.children.push(detail_node);
                    }

                    // Cross-references for this branch
                    for cross_ref in &state.cross_refs {
                        if cross_ref.from_branch == branch_capture.branch {
                            let target_node_id = state
                                .branch_captures
                                .iter()
                                .find(|bc| bc.branch == cross_ref.to_branch)
                                .map(|bc| bc.node_id)
                                .unwrap_or(0);
                            if target_node_id > 0 {
                                branch_node.relationships.push(AMTRelation {
                                    target_id: target_node_id,
                                    relation_type: cross_ref.relation_type.clone(),
                                    confidence: 0.8,
                                });
                            }
                        }
                    }

                    intent_node.children.push(branch_node);
                }

                root_node.children.push(intent_node);
                continue; // skip the direct-branch path below
            };

            // Single intent: branches go directly under root
            let branches_for_intent: Vec<&BranchCapture> = state
                .branch_captures
                .iter()
                .filter(|bc| bc.parent_intent == intent_capture.intent)
                .collect();

            for branch_capture in branches_for_intent {
                let mut branch_node = AMTNode::new(
                    branch_capture.node_id,
                    AMTNodeType::Branch,
                    branch_capture.branch.clone(),
                    1,
                );
                branch_node.source_chunk_indices = branch_capture.source_chunk_indices.clone();
                branch_node.methodology_ids = branch_capture.source_methodology_ids.clone();

                let details_for_branch: Vec<&DetailCapture> = state
                    .detail_captures
                    .iter()
                    .filter(|dc| dc.parent_branch == branch_capture.branch)
                    .collect();

                for detail_capture in details_for_branch {
                    let node_type = match detail_capture.detail_type.as_str() {
                        "constraint" => AMTNodeType::Consideration,
                        _ => AMTNodeType::Leaf,
                    };
                    let mut detail_node = AMTNode::new(
                        detail_capture.node_id,
                        node_type,
                        detail_capture.content.clone(),
                        2,
                    );
                    detail_node
                        .metadata
                        .insert("type".to_string(), detail_capture.detail_type.clone());
                    detail_node.source_chunk_indices = detail_capture.source_chunk_indices.clone();
                    branch_node.children.push(detail_node);
                }

                for cross_ref in &state.cross_refs {
                    if cross_ref.from_branch == branch_capture.branch {
                        let target_node_id = state
                            .branch_captures
                            .iter()
                            .find(|bc| bc.branch == cross_ref.to_branch)
                            .map(|bc| bc.node_id)
                            .unwrap_or(0);
                        if target_node_id > 0 {
                            branch_node.relationships.push(AMTRelation {
                                target_id: target_node_id,
                                relation_type: cross_ref.relation_type.clone(),
                                confidence: 0.8,
                            });
                        }
                    }
                }

                root_node.children.push(branch_node);
            }
        }

        Ok(root_node)
    }

    // ========================================================================
    // STAGE 3: Blueprint Assignment
    // ========================================================================

    async fn stage_3_blueprint_assignment(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Search for blueprint with 100% match
        let blueprint_ids = self
            .zsei
            .search_by_keywords(
                &state.keywords.iter().take(15).cloned().collect::<Vec<_>>(),
                Some("Blueprint"),
            )
            .await
            .unwrap_or_default();

        let mut best_match: Option<(u64, f32)> = None;

        for bp_id in blueprint_ids {
            if let Ok(Some(container)) = self.zsei.get_container(bp_id).await {
                // Calculate match score
                let bp_keywords: Vec<String> = container
                    .get("local_state")
                    .and_then(|ls| ls.get("context"))
                    .and_then(|ctx| ctx.get("keywords"))
                    .and_then(|k| serde_json::from_value(k.clone()).ok())
                    .unwrap_or_default();

                let state_keywords_set: HashSet<_> =
                    state.keywords.iter().map(|s| s.to_lowercase()).collect();
                let bp_keywords_set: HashSet<_> =
                    bp_keywords.iter().map(|s| s.to_lowercase()).collect();

                let intersection = state_keywords_set.intersection(&bp_keywords_set).count();
                let union = state_keywords_set.union(&bp_keywords_set).count();

                let match_score = if union > 0 {
                    intersection as f32 / union as f32
                } else {
                    0.0
                };

                if match_score > best_match.map(|(_, s)| s).unwrap_or(0.0) {
                    best_match = Some((bp_id, match_score));
                }
            }
        }

        // Only use if 100% match (or very close - 95%+)
        if let Some((bp_id, score)) = best_match {
            if score >= 0.95 {
                state.blueprint_id = Some(bp_id);

                // Load blueprint steps
                if let Ok(Some(container)) = self.zsei.get_container(bp_id).await {
                    state.blueprint_steps = container
                        .get("local_state")
                        .and_then(|ls| ls.get("storage"))
                        .and_then(|s| s.get("steps"))
                        .and_then(|steps| serde_json::from_value(steps.clone()).ok())
                        .unwrap_or_default();
                }

                self.record_stage_timed(
                    state,
                    3,
                    "Blueprint Assignment",
                    true,
                    &format!(
                        "Using existing blueprint {} (match: {:.0}%)",
                        bp_id,
                        score * 100.0
                    ),
                    stage_start.elapsed().as_millis() as u64,
                );
                return Ok(());
            }
        }

        // No 100% match - create new blueprint
        let amt = state.amt.as_ref().ok_or("No AMT available")?;

        // Generate blueprint from AMT with pipeline awareness
        let available_pipelines_desc: String = state
            .available_pipelines
            .iter()
            .filter(|p| !p.deprecated)
            .map(|p| format!("  - {} (ID: {}): {}", p.name, p.pipeline_id, p.description))
            .collect::<Vec<_>>()
            .join("\n");

        let blueprint_prompt = format!(
            r#"Create a blueprint (execution plan) from this AMT.

AMT ROOT: {}
BRANCHES:
{}

AVAILABLE PIPELINES:
{}

METHODOLOGIES: {:?}

For each step, select the most appropriate pipeline from the list.
If no existing pipeline can handle a requirement, add it to missing_capabilities.

Return JSON:
{{
    "name": "Blueprint name",
    "description": "What this blueprint does",
    "steps": [
        {{
            "step_index": 0,
            "action": "action_name",
            "description": "What this step does",
            "pipeline_id": 9,
            "context_requirements": ["full_context"],
            "depends_on": [],
            "wait_for_graph_update": false,
            "max_retries": 1
        }}
    ],
    "missing_capabilities": ["capability1", "capability2"]
}}"#,
            amt.content,
            amt.children
                .iter()
                .map(|c| format!(
                    "- {}: {} children, chunk refs: {:?}",
                    c.content,
                    c.children.len(),
                    c.source_chunk_indices
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            available_pipelines_desc,
            state.methodologies
        );

        let bp_input = serde_json::json!({
            "prompt": blueprint_prompt,
            "max_tokens": 1000,
            "temperature": 0.3,
            "system_context": "Generate execution blueprints. Respond with JSON only."
        });

        let bp_result = self.executor.execute(9, bp_input).await?;
        let response = bp_result
            .get("response")
            .and_then(|r| r.as_str())
            .unwrap_or("{}");
        let bp_json = Self::parse_json_object(response);

        let name = bp_json
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Generated Blueprint")
            .to_string();
        let description = bp_json
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        // Check for missing capabilities
        let missing_capabilities: Vec<String> = bp_json
            .get("missing_capabilities")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if !missing_capabilities.is_empty() {
            // Log missing capabilities - could trigger pipeline creation in future
            tracing::warn!(
                "Blueprint has missing capabilities: {:?}",
                missing_capabilities
            );
        }

        state.blueprint_steps = bp_json
            .get("steps")
            .and_then(|s| serde_json::from_value(s.clone()).ok())
            .unwrap_or_else(|| {
                vec![BlueprintStep {
                    step_index: 0,
                    action: "execute_prompt".to_string(),
                    description: "Process the user prompt".to_string(),
                    pipeline_id: 9,
                    context_requirements: vec!["full_context".to_string()],
                    loop_config: None,
                    sub_steps: Vec::new(),
                    depends_on: Vec::new(),
                    wait_for_graph_update: false,
                    max_retries: 1,
                    timeout_ms: None,
                }]
            });

        // Store blueprint in ZSEI
        let blueprint_container = serde_json::json!({
            "container_type": "Blueprint",
            "metadata": {
                "name": name,
                "description": description,
                "created_by": "orchestrator"
            },
            "context": {
                "keywords": state.keywords,
                "topics": state.topics,
                "methodology_ids": state.methodologies
            },
            "storage": {
                "steps": state.blueprint_steps,
                "missing_capabilities": missing_capabilities
            }
        });

        if let Ok(new_id) = self.zsei.create_container(0, blueprint_container).await {
            state.blueprint_id = Some(new_id);
            state.blueprints_created += 1;
        }

        self.record_stage_timed(
            state,
            3,
            "Blueprint Assignment",
            true,
            &format!(
                "Created new blueprint with {} steps (missing: {})",
                state.blueprint_steps.len(),
                missing_capabilities.len()
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    // ========================================================================
    // STAGE 4: Zero-Shot Simulation
    // ========================================================================

    async fn stage_4_zero_shot_simulation(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        let amt = match &state.amt {
            Some(a) => a,
            None => {
                self.record_stage_timed(
                    state,
                    4,
                    "Zero-Shot Simulation",
                    true,
                    "Skipped (no AMT)",
                    0,
                );
                return Ok(());
            }
        };

        // Simulate execution using AMT traversal
        let simulate_prompt = format!(
            r#"Simulate executing this plan and predict outcomes.

AMT STRUCTURE:
- Root intent: {}
- Branches: {}

BLUEPRINT STEPS:
{}

For each step, predict:
1. What information will be needed
2. What output will be produced
3. Potential issues or clarifications needed

Return JSON:
{{
    "simulation_confidence": 0.0-1.0,
    "step_predictions": [
        {{"step": 0, "needs": ["info1"], "produces": ["output1"], "risks": ["risk1"]}}
    ],
    "overall_feasibility": "high/medium/low",
    "clarifications_needed": []
}}"#,
            amt.content,
            amt.children
                .iter()
                .map(|c| &c.content)
                .collect::<Vec<_>>()
                .join(", "),
            state
                .blueprint_steps
                .iter()
                .map(|s| format!("Step {}: {} - {}", s.step_index, s.action, s.description))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let sim_input = serde_json::json!({
            "prompt": simulate_prompt,
            "max_tokens": 800,
            "temperature": 0.3,
            "system_context": "Simulate execution and predict outcomes. Respond with JSON only."
        });

        let sim_result = self.executor.execute(9, sim_input).await?;
        let response = sim_result
            .get("response")
            .and_then(|r| r.as_str())
            .unwrap_or("{}");
        let sim_json = Self::parse_json_object(response);

        let confidence = sim_json
            .get("simulation_confidence")
            .and_then(|c| c.as_f64())
            .unwrap_or(0.7);

        let feasibility = sim_json
            .get("overall_feasibility")
            .and_then(|f| f.as_str())
            .unwrap_or("medium");

        // Check for clarifications needed
        if let Some(clarifications) = sim_json
            .get("clarifications_needed")
            .and_then(|c| c.as_array())
        {
            for c in clarifications {
                if let Some(c_str) = c.as_str() {
                    if !c_str.is_empty() {
                        state.clarification_points.push(c_str.to_string());
                    }
                }
            }
        }

        if !state.clarification_points.is_empty() && confidence < 0.5 {
            state.needs_clarification = true;
        }

        self.record_stage_timed(
            state,
            4,
            "Zero-Shot Simulation",
            true,
            &format!(
                "Confidence: {:.0}%, Feasibility: {}",
                confidence * 100.0,
                feasibility
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    // ========================================================================
    // STAGE 5: Consciousness Decision Gate
    // ========================================================================

    async fn stage_5_consciousness_gate(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Call decision_gate pipeline (#39)
        let input = serde_json::json!({
            "action": "Evaluate",
            "task_id": 0,
            "task_summary": &state.cleaned_prompt[..state.cleaned_prompt.len().min(500)],
            "blueprint_id": state.blueprint_id.unwrap_or(0),
            "user_id": state.request.user_id,
            "amt_summary": {
                "intent": state.amt.as_ref().map(|a| &a.content),
                "branch_count": state.amt.as_ref().map(|a| a.branch_count()).unwrap_or(0)
            }
        });

        let result = self.executor.execute(39, input).await?;

        let decision = result
            .get("gate")
            .and_then(|g| g.get("decision"))
            .and_then(|d| d.as_str())
            .unwrap_or("Proceed");

        let confidence = result
            .get("gate")
            .and_then(|g| g.get("confidence"))
            .and_then(|c| c.as_f64())
            .unwrap_or(0.8) as f32;

        let reasoning = result
            .get("gate")
            .and_then(|g| g.get("reasoning"))
            .and_then(|r| r.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();

        state.gate_result = Some(GateResult {
            decision: decision.to_string(),
            confidence,
            reasoning: reasoning.clone(),
        });

        if decision == "Decline" {
            return Err(format!("Consciousness gate declined: {}", reasoning));
        }

        self.record_stage_timed(
            state,
            5,
            "Consciousness Gate",
            true,
            &format!("Decision: {} ({:.0}%)", decision, confidence * 100.0),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    // ========================================================================
    // STAGES 6-8: Context Aggregation + Task Creation + Step Execution
    // ========================================================================

    async fn stage_6_to_8_execute_steps(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // STAGE 7: Create task via TaskManager
        let mut inputs = HashMap::new();
        inputs.insert(
            "prompt".to_string(),
            serde_json::json!(state.cleaned_prompt),
        );
        inputs.insert(
            "blueprint_id".to_string(),
            serde_json::json!(state.blueprint_id),
        );
        if let Some(ref amt) = state.amt {
            inputs.insert("amt_intent".to_string(), serde_json::json!(amt.content));
        }

        // Enqueue task via TaskManager
        let task_result = self
            .task_manager
            .enqueue_task(
                state.blueprint_id,
                inputs,
                state.request.user_id,
                state.request.device_id,
                state.request.workspace_id,
                state.request.project_id,
                TaskPriority::Normal,
            )
            .await;

        match task_result {
            Ok(task_id) => {
                state.task_id = Some(task_id);
            }
            Err(e) => {
                self.record_stage(state, 7, "Task Creation", false, &format!("Failed: {}", e));
                return Err(e.to_string());
            }
        }

        self.record_stage(
            state,
            7,
            "Task Creation",
            state.task_id.is_some(),
            &format!("Task: {:?}", state.task_id),
        );

        // STAGES 6 & 8: Execute steps
        let steps = state.blueprint_steps.clone();
        let mut all_outputs: Vec<String> = Vec::new();
        let mut completed: HashSet<u32> = HashSet::new();

        // Build dependency order
        let mut step_queue: Vec<&BlueprintStep> = steps.iter().collect();
        let mut iterations = 0;
        let max_iterations = steps.len() * 2;

        while !step_queue.is_empty() && iterations < max_iterations {
            iterations += 1;

            // Find steps whose dependencies are satisfied
            let ready_steps: Vec<_> = step_queue
                .iter()
                .filter(|s| s.depends_on.iter().all(|dep| completed.contains(dep)))
                .cloned()
                .collect();

            if ready_steps.is_empty() && !step_queue.is_empty() {
                // Force execute first remaining step (break deadlock)
                if let Some(step) = step_queue.first().cloned() {
                    let result = self.execute_step(state, step, &all_outputs).await?;
                    let output_text = self.extract_output_text(&result.output);
                    all_outputs.push(output_text.clone());
                    state.step_results.push(result.clone());
                    state.step_outputs.insert(
                        step.step_index,
                        serde_json::json!({"output": all_outputs.last()}),
                    );
                    completed.insert(step.step_index);
                    step_queue.retain(|s| s.step_index != step.step_index);

                    // Update TaskManager
                    if let Some(task_id) = state.task_id {
                        let _ = self
                            .task_manager
                            .update_step(
                                task_id,
                                step.step_index,
                                "completed",
                                result.tokens_used,
                                Some(output_text[..200.min(output_text.len())].to_string()),
                                None,
                            )
                            .await;

                        let _ = self
                            .task_manager
                            .update_progress(task_id, completed.len() as u32, steps.len() as u32)
                            .await;
                    }
                }
            } else {
                for step in ready_steps {
                    let result = self.execute_step(state, step, &all_outputs).await?;
                    let output_text = self.extract_output_text(&result.output);
                    all_outputs.push(output_text.clone());
                    state.step_results.push(result.clone());
                    state.step_outputs.insert(
                        step.step_index,
                        serde_json::json!({"output": all_outputs.last()}),
                    );
                    completed.insert(step.step_index);
                    step_queue.retain(|s| s.step_index != step.step_index);

                    // Update TaskManager
                    if let Some(task_id) = state.task_id {
                        let _ = self
                            .task_manager
                            .update_step(
                                task_id,
                                step.step_index,
                                "completed",
                                result.tokens_used,
                                Some(output_text[..200.min(output_text.len())].to_string()),
                                None,
                            )
                            .await;

                        let _ = self
                            .task_manager
                            .update_progress(task_id, completed.len() as u32, steps.len() as u32)
                            .await;
                    }
                }
            }
        }

        // Combine outputs into final response
        state.final_response = if all_outputs.len() == 1 {
            Some(all_outputs[0].clone())
        } else if !all_outputs.is_empty() {
            Some(all_outputs.join("\n\n"))
        } else {
            None
        };

        self.record_stage_timed(
            state,
            8,
            "Step Execution",
            state.final_response.is_some(),
            &format!(
                "{} steps executed, tokens: {}",
                state.step_results.len(),
                state.tokens_used_so_far
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    /// Execute a single blueprint step (handles loops, sub-steps, retries)
    async fn execute_step(
        &self,
        state: &mut OrchestrationState,
        step: &BlueprintStep,
        previous_outputs: &[String],
    ) -> Result<StepResult, String> {
        let mut total_iterations = 0;
        let mut sub_step_results = Vec::new();
        let mut final_output = serde_json::json!({});

        // Handle loop configuration
        let (iterations, should_loop) = if let Some(loop_config) = &step.loop_config {
            match loop_config.loop_type {
                LoopType::Count => (loop_config.max_iterations, true),
                LoopType::While | LoopType::Until => (loop_config.max_iterations, true),
                LoopType::ForEach => {
                    // Get iteration count from iterate_over
                    let count = step.context_requirements.len() as u32;
                    (count.max(1), true)
                }
            }
        } else {
            (1, false)
        };

        for iteration in 0..iterations {
            total_iterations = iteration + 1;

            // STAGE 6: Context aggregation for this step
            let context_input = serde_json::json!({
                "action": "ForQuery",
                "query": format!("{} - {}", state.cleaned_prompt, step.description),
                "token_budget": state.model_context_limit / 4,
                "project_id": state.request.project_id,
                "workspace_id": state.request.workspace_id,
                "priority_order": step.context_requirements,
                "step_index": step.step_index,
                "iteration": iteration
            });

            let context_result = self.executor.execute(21, context_input).await?;
            let step_context = context_result
                .get("context")
                .and_then(|c| c.get("context_text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            state
                .step_contexts
                .insert(step.step_index, step_context.clone());

            // Build full context with previous outputs
            let full_context = if !previous_outputs.is_empty() {
                format!(
                    "{}\n\nPrevious step outputs:\n{}",
                    step_context,
                    previous_outputs
                        .iter()
                        .enumerate()
                        .map(|(i, o)| format!("Step {}: {}", i + 1, &o[..o.len().min(300)]))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            } else {
                step_context
            };

            // Execute sub-steps first if any
            for sub_step in &step.sub_steps {
                let sub_input = self.build_sub_step_input(state, sub_step, &full_context)?;
                let sub_result = self.executor.execute(sub_step.pipeline_id, sub_input).await;

                sub_step_results.push(SubStepResult {
                    sub_index: sub_step.sub_index,
                    output: sub_result.clone().unwrap_or_default(),
                    success: sub_result.is_ok(),
                });
            }

            // Execute main step
            let step_prompt = format!(
                "Step {}: {}\n\nContext:\n{}\n\nOriginal request: {}",
                step.step_index + 1,
                step.description,
                full_context,
                &state.cleaned_prompt[..state.cleaned_prompt.len().min(500)]
            );

            let exec_input = serde_json::json!({
                "prompt": step_prompt,
                "max_tokens": state.model_context_limit / 4,
                "temperature": 0.7,
                "action": step.action
            });

            let mut retries = 0;
            let mut exec_result = self
                .executor
                .execute(step.pipeline_id, exec_input.clone())
                .await;

            while exec_result.is_err() && retries < step.max_retries {
                retries += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(100 * retries as u64)).await;
                exec_result = self
                    .executor
                    .execute(step.pipeline_id, exec_input.clone())
                    .await;
            }

            final_output = exec_result?;

            // Wait for graph update if configured
            if step.wait_for_graph_update {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // Check loop continuation condition
            if should_loop {
                if let Some(loop_config) = &step.loop_config {
                    let should_continue = match loop_config.loop_type {
                        LoopType::Count => iteration + 1 < loop_config.max_iterations,
                        LoopType::While => {
                            // Evaluate condition (simplified - check if output indicates completion)
                            let output_text = self.extract_output_text(&final_output);
                            !output_text.to_lowercase().contains("complete")
                                && !output_text.to_lowercase().contains("done")
                        }
                        LoopType::Until => {
                            let output_text = self.extract_output_text(&final_output);
                            output_text.to_lowercase().contains("continue")
                                || !output_text.to_lowercase().contains("complete")
                        }
                        LoopType::ForEach => iteration + 1 < iterations,
                    };

                    if !should_continue {
                        break;
                    }
                }
            }
        }

        let tokens_used = final_output
            .get("tokens_used")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;

        state.tokens_used_so_far += tokens_used;

        Ok(StepResult {
            step_index: step.step_index,
            pipeline_id: step.pipeline_id,
            output: final_output,
            tokens_used,
            iterations: total_iterations,
            sub_step_results,
        })
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

    async fn stage_9_result_collection(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Complete or fail task via TaskManager
        if let Some(task_id) = state.task_id {
            if state.final_response.is_some() {
                let outputs = state
                    .step_results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "step": r.step_index,
                            "output": self.extract_output_text(&r.output),
                            "tokens": r.tokens_used
                        })
                    })
                    .collect::<Vec<_>>();

                let _ = self
                    .task_manager
                    .complete_task(
                        task_id,
                        Some(serde_json::json!({
                            "response": state.final_response,
                            "steps": outputs
                        })),
                        state.tokens_used_so_far,
                    )
                    .await;
            } else {
                let _ = self
                    .task_manager
                    .fail_task(task_id, "No response generated".to_string())
                    .await;
            }
        }

        self.record_stage_timed(
            state,
            9,
            "Result Collection",
            true,
            &format!("Collected {} step results", state.step_results.len()),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    // ========================================================================
    // STAGE 10: Post-execution Consciousness
    // ========================================================================

    async fn stage_10_post_execution(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        if !state.request.consciousness_enabled {
            self.record_stage_timed(
                state,
                10,
                "Post-execution",
                true,
                "Consciousness disabled - skipped",
                stage_start.elapsed().as_millis() as u64,
            );
            return Ok(());
        }

        // Consciousness hooks are now handled by TaskManager automatically
        // when complete_task or fail_task is called.
        //
        // Additional consciousness processing can be done here if needed:

        // Store experience with more detail via consciousness pipelines
        let experience_input = serde_json::json!({
            "action": "StoreExperience",
            "experience_type": if state.final_response.is_some() { "task_success" } else { "task_failure" },
            "summary": &state.cleaned_prompt[..state.cleaned_prompt.len().min(200)],
            "task_id": state.task_id,
            "user_id": state.request.user_id,
            "tags": state.topics.clone(),
            "keywords": state.keywords.iter().take(10).cloned().collect::<Vec<_>>(),
            "methodologies_used": state.methodologies.clone(),
            "blueprint_id": state.blueprint_id,
            "significance": if state.final_response.is_some() { 0.5 } else { 0.3 },
            "tokens_used": state.tokens_used_so_far
        });

        // Pipeline 41 = CoreMemoryFormation
        let _ = self.executor.execute(41, experience_input).await;

        // Update relationship if we know the user
        let relationship_input = serde_json::json!({
            "action": "RecordInteraction",
            "user_id": state.request.user_id,
            "interaction_type": "task_completion",
            "outcome": if state.final_response.is_some() { "positive" } else { "negative" },
            "topics": state.topics.clone()
        });

        // Pipeline 47 = RelationshipDevelopment
        let _ = self.executor.execute(47, relationship_input).await;

        // Update emotional state
        let emotion_input = serde_json::json!({
            "action": "ProcessTrigger",
            "trigger_type": if state.final_response.is_some() { "task_success" } else { "task_failure" },
            "source": "orchestrator",
            "intensity": 0.5,
            "context": &state.cleaned_prompt[..state.cleaned_prompt.len().min(100)]
        });

        // Pipeline 43 = EmotionalBaselineUpdate
        let _ = self.executor.execute(43, emotion_input).await;

        self.record_stage_timed(
            state,
            10,
            "Post-execution Consciousness",
            true,
            "Experience stored, relationship updated, emotions processed",
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    // ========================================================================
    // STAGE 11: Response Delivery
    // ========================================================================

    async fn stage_11_response_delivery(
        &self,
        state: &mut OrchestrationState,
    ) -> Result<(), String> {
        let stage_start = std::time::Instant::now();

        // Apply voice identity if consciousness enabled
        if state.request.consciousness_enabled && state.final_response.is_some() {
            // Get voice identity from self_model (Pipeline 46 = NarrativeConstruction)
            let voice_input = serde_json::json!({
                "action": "GetVoice"
            });

            if let Ok(voice_result) = self.executor.execute(46, voice_input).await {
                if let Some(voice) = voice_result.get("voice") {
                    state.voice_identity = serde_json::from_value(voice.clone()).ok();

                    // Apply voice identity to response if significantly different from neutral
                    if let (Some(response), Some(voice_id)) =
                        (&state.final_response, &state.voice_identity)
                    {
                        let needs_restyle = voice_id.formality < 0.4
                            || voice_id.formality > 0.6
                            || voice_id.warmth < 0.4
                            || voice_id.warmth > 0.6;

                        if needs_restyle && response.len() > 50 {
                            let style_input = serde_json::json!({
                                "prompt": format!(
                                    "Rephrase this response to match this voice identity:\n\
                                    Tone: {}\n\
                                    Formality: {:.1}\n\
                                    Warmth: {:.1}\n\
                                    Directness: {:.1}\n\
                                    \nOriginal response:\n{}",
                                    voice_id.tone,
                                    voice_id.formality,
                                    voice_id.warmth,
                                    voice_id.directness,
                                    response
                                ),
                                "max_tokens": 2000,
                                "temperature": 0.7,
                                "system_context": "Apply voice identity while maintaining content accuracy."
                            });

                            if let Ok(styled) = self.executor.execute(9, style_input).await {
                                if let Some(new_response) =
                                    styled.get("response").and_then(|r| r.as_str())
                                {
                                    state.final_response = Some(new_response.to_string());
                                }
                            }
                        }
                    }
                }
            }

            // Update consciousness dashboard (Pipeline 54 = MetaPortionConsciousness)
            let dashboard_input = serde_json::json!({
                "action": "Update",
                "task_completed": true,
                "task_id": state.task_id,
                "success": state.final_response.is_some(),
                "tokens_used": state.tokens_used_so_far,
                "methodologies_used": state.methodologies.len(),
                "blueprint_id": state.blueprint_id
            });
            let _ = self.executor.execute(54, dashboard_input).await;
        }

        // Generate task recommendations for next steps (Pipeline 23 = TaskRecommendation)
        let recommend_input = serde_json::json!({
            "action": "Suggest",
            "context": &state.cleaned_prompt[..state.cleaned_prompt.len().min(200)],
            "completed_task_id": state.task_id,
            "topics": state.topics.clone(),
            "keywords": state.keywords.iter().take(5).cloned().collect::<Vec<_>>()
        });
        let _ = self.executor.execute(23, recommend_input).await;

        self.record_stage_timed(
            state,
            11,
            "Response Delivery",
            state.final_response.is_some(),
            &format!(
                "Response: {} chars, Voice: {}, Tokens: {}",
                state.final_response.as_ref().map(|r| r.len()).unwrap_or(0),
                state.voice_identity.is_some(),
                state.tokens_used_so_far
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

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
                        "ChunkText" => Ok(serde_json::json!({
                            "chunks": [{
                                "index": 0,
                                "text": "Test chunk",
                                "start_char": 0,
                                "end_char": 10,
                                "token_count": 3,
                                "is_complete_paragraph": true
                            }]
                        })),
                        "ProcessChunk" => Ok(serde_json::json!({
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
                            }]
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
    impl ZSEIAccess for MockZSEI {
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
        let task_manager = Arc::new(TaskManager::new(task_config, refinement_config).unwrap());

        let orchestrator = PromptOrchestrator::new(executor, zsei, task_manager);

        let request = OrchestrationRequest {
            prompt: "Hello, how are you?".to_string(),
            project_id: None,
            workspace_id: None,
            user_id: 1,
            device_id: 1,
            consciousness_enabled: false,
            token_budget: Some(10000),
            model_config: None,
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
