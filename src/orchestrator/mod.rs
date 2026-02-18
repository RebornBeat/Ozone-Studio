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
//! - Layer-by-layer AMT building from chunks
//! - 5 consecutive Valid validations required
//! - Blueprint step execution with loop/sub-step/dependency support
//! - Direct ZSEI access (no deprecated pipeline wrappers)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

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
}

impl PromptOrchestrator {
    pub fn new(executor: Arc<dyn PipelineExecutor>, zsei: Arc<dyn ZSEIAccess>) -> Self {
        Self { executor, zsei }
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
        let existing_categories = self.zsei.get_categories("Text").await.unwrap_or_default();
        let existing_set: HashSet<u64> = existing_categories.into_iter().collect();

        // Create categories for new topics
        for topic in &state.topics {
            // Check if topic matches any existing category (simplified check)
            let needs_creation = !topic.is_empty() && methodology_categories.is_empty();

            if needs_creation {
                let new_category = serde_json::json!({
                    "container_type": "Category",
                    "modality": "Text",
                    "metadata": {
                        "name": topic,
                        "description": format!("Auto-created category for topic: {}", topic),
                        "created_by": "orchestrator"
                    }
                });

                if let Ok(new_id) = self.zsei.create_container(0, new_category).await {
                    state.categories.push(new_id);
                    state.categories_created += 1;
                }
            }
        }

        state.categories.extend(methodology_categories);

        // STEP 6: Build AMT layer-by-layer from chunks
        state.amt = Some(self.build_amt_layer_by_layer(state).await?);

        // STEP 7: Validate AMT (need 5 consecutive Valid)
        let mut validation_attempts = 0;
        let max_validation_attempts = 10;

        while state.validation_streak < 5 && validation_attempts < max_validation_attempts {
            validation_attempts += 1;

            let validation_result = self.validate_amt(state).await?;

            if validation_result.is_valid {
                state.validation_streak += 1;
            } else {
                state.validation_streak = 0;

                // If invalid, refine AMT based on issues
                if validation_attempts < max_validation_attempts {
                    self.refine_amt_from_issues(state, &validation_result.issues)
                        .await?;
                } else {
                    // Max attempts reached, collect clarification points
                    state.clarification_points.extend(validation_result.issues);
                    state.needs_clarification = true;
                }
            }
        }

        state.amt_validated = state.validation_streak >= 5;

        self.record_stage_timed(
            state, 2, "Text Normalization + AMT", true,
            &format!(
                "Chunks: {}, Keywords: {}, Methodologies: {}, Categories: {} ({} created), Validated: {} (streak: {})",
                state.processed_chunks.len(),
                state.keywords.len(),
                state.methodologies.len(),
                state.categories.len(),
                state.categories_created,
                state.amt_validated,
                state.validation_streak
            ),
            stage_start.elapsed().as_millis() as u64,
        );

        Ok(())
    }

    /// Build AMT layer-by-layer from processed chunks
    async fn build_amt_layer_by_layer(
        &self,
        state: &OrchestrationState,
    ) -> Result<AMTNode, String> {
        let mut node_id_counter = 1u64;

        // LAYER 1: Identify root intent across ALL chunks
        let root_prompt = format!(
            r#"Analyze these text chunks and identify the PRIMARY INTENT/GOAL of the entire request.

CHUNKS:
{}

Return JSON:
{{
    "primary_intent": "the main goal/intent",
    "main_branches": ["branch1", "branch2", "branch3"],
    "confidence": 0.0-1.0
}}"#,
            state
                .processed_chunks
                .iter()
                .map(|c| format!(
                    "Chunk {}: {}",
                    c.index,
                    &c.cleaned_text[..c.cleaned_text.len().min(500)]
                ))
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        let root_input = serde_json::json!({
            "prompt": root_prompt,
            "max_tokens": 500,
            "temperature": 0.3,
            "system_context": "You analyze text to identify intent and structure. Respond with JSON only."
        });

        let root_result = self.executor.execute(9, root_input).await?;
        let root_response = root_result
            .get("response")
            .and_then(|r| r.as_str())
            .unwrap_or("{}");
        let root_json = Self::parse_json_object(root_response);

        let primary_intent = root_json
            .get("primary_intent")
            .and_then(|p| p.as_str())
            .unwrap_or("Process user request")
            .to_string();

        let main_branches: Vec<String> = root_json
            .get("main_branches")
            .and_then(|b| b.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Create root node
        let mut root = AMTNode::new(node_id_counter, AMTNodeType::Root, primary_intent, 0);
        root.source_chunk_indices = (0..state.processed_chunks.len() as u32).collect();
        root.methodology_ids = state.methodologies.clone();
        node_id_counter += 1;

        // LAYER 2: Create main branch nodes
        for branch_name in main_branches {
            let mut branch_node =
                AMTNode::new(node_id_counter, AMTNodeType::Branch, branch_name, 1);
            node_id_counter += 1;

            // Find which chunks relate to this branch
            for chunk in &state.processed_chunks {
                let branch_lower = branch_node.content.to_lowercase();
                let has_overlap = chunk.keywords.iter().any(|kw| {
                    branch_lower.contains(&kw.to_lowercase())
                        || kw.to_lowercase().contains(&branch_lower)
                }) || chunk.topics.iter().any(|t| {
                    branch_lower.contains(&t.to_lowercase())
                        || t.to_lowercase().contains(&branch_lower)
                });

                if has_overlap {
                    branch_node.source_chunk_indices.push(chunk.index);
                }
            }

            root.children.push(branch_node);
        }

        // LAYER 3: Add detail nodes under each branch
        for branch in &mut root.children {
            let detail_prompt = format!(
                r#"For this branch of the request, identify specific details, requirements, and constraints.

BRANCH: {}
RELATED CHUNKS:
{}

Return JSON:
{{
    "details": ["detail1", "detail2"],
    "requirements": ["req1", "req2"],
    "constraints": ["constraint1", "constraint2"]
}}"#,
                branch.content,
                branch
                    .source_chunk_indices
                    .iter()
                    .filter_map(|&idx| state.processed_chunks.get(idx as usize))
                    .map(|c| format!("- {}", &c.cleaned_text[..c.cleaned_text.len().min(300)]))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            let detail_input = serde_json::json!({
                "prompt": detail_prompt,
                "max_tokens": 400,
                "temperature": 0.3,
                "system_context": "Extract details from text. Respond with JSON only."
            });

            if let Ok(detail_result) = self.executor.execute(9, detail_input).await {
                let detail_response = detail_result
                    .get("response")
                    .and_then(|r| r.as_str())
                    .unwrap_or("{}");
                let detail_json = Self::parse_json_object(detail_response);

                // Add detail nodes
                if let Some(details) = detail_json.get("details").and_then(|d| d.as_array()) {
                    for detail in details {
                        if let Some(detail_str) = detail.as_str() {
                            let detail_node = AMTNode::new(
                                node_id_counter,
                                AMTNodeType::Leaf,
                                detail_str.to_string(),
                                2,
                            );
                            node_id_counter += 1;
                            branch.children.push(detail_node);
                        }
                    }
                }

                // Add requirement nodes
                if let Some(reqs) = detail_json.get("requirements").and_then(|r| r.as_array()) {
                    for req in reqs {
                        if let Some(req_str) = req.as_str() {
                            let mut req_node = AMTNode::new(
                                node_id_counter,
                                AMTNodeType::Leaf,
                                req_str.to_string(),
                                2,
                            );
                            req_node
                                .metadata
                                .insert("type".to_string(), "requirement".to_string());
                            node_id_counter += 1;
                            branch.children.push(req_node);
                        }
                    }
                }

                // Add constraint nodes
                if let Some(constraints) = detail_json.get("constraints").and_then(|c| c.as_array())
                {
                    for constraint in constraints {
                        if let Some(c_str) = constraint.as_str() {
                            let mut c_node = AMTNode::new(
                                node_id_counter,
                                AMTNodeType::Consideration,
                                c_str.to_string(),
                                2,
                            );
                            c_node
                                .metadata
                                .insert("type".to_string(), "constraint".to_string());
                            node_id_counter += 1;
                            branch.children.push(c_node);
                        }
                    }
                }
            }
        }

        // LAYER 4: Add cross-references between branches with shared context
        let branch_count = root.children.len();
        for i in 0..branch_count {
            for j in (i + 1)..branch_count {
                // Check for shared chunk indices
                let shared_chunks: Vec<u32> = root.children[i]
                    .source_chunk_indices
                    .iter()
                    .filter(|idx| root.children[j].source_chunk_indices.contains(idx))
                    .cloned()
                    .collect();

                if !shared_chunks.is_empty() {
                    let target_id = root.children[j].id;
                    root.children[i].relationships.push(AMTRelation {
                        target_id,
                        relation_type: AMTRelationType::SharedContext,
                        confidence: shared_chunks.len() as f32
                            / root.children[i].source_chunk_indices.len().max(1) as f32,
                    });
                }
            }
        }

        // LAYER 5: Add consideration nodes for coverage aspects
        let coverage_aspects = [
            "security",
            "error_handling",
            "edge_cases",
            "performance",
            "testing",
        ];

        for aspect in coverage_aspects {
            // Check if aspect is mentioned in any chunk
            let is_mentioned = state.processed_chunks.iter().any(|c| {
                c.cleaned_text.to_lowercase().contains(aspect)
                    || c.keywords
                        .iter()
                        .any(|kw| kw.to_lowercase().contains(aspect))
            });

            if !is_mentioned {
                let mut consideration = AMTNode::new(
                    node_id_counter,
                    AMTNodeType::Consideration,
                    format!("Consider: {}", aspect),
                    1,
                );
                consideration
                    .metadata
                    .insert("auto_added".to_string(), "true".to_string());
                consideration.confidence = 0.5; // Lower confidence since auto-added
                node_id_counter += 1;
                root.children.push(consideration);
            }
        }

        Ok(root)
    }

    /// Validate AMT and return result
    async fn validate_amt(&self, state: &OrchestrationState) -> Result<ValidationResult, String> {
        let amt = match &state.amt {
            Some(a) => a,
            None => {
                return Ok(ValidationResult {
                    is_valid: false,
                    issues: vec!["No AMT built".to_string()],
                })
            }
        };

        let validate_prompt = format!(
            r#"Validate this Abstract Meaning Tree for completeness.

ORIGINAL KEYWORDS: {:?}
ORIGINAL TOPICS: {:?}

AMT STRUCTURE:
- Root: {}
- Branches: {}
- Total nodes: {}

BRANCH DETAILS:
{}

Check:
1. Are all key aspects of the request represented?
2. Are there any ambiguous or unclear parts?
3. Is the structure logically coherent?
4. Are security, error handling, and edge cases considered?

Respond with JSON:
{{
    "valid": true/false,
    "issues": ["issue1", "issue2"],
    "missing_aspects": ["aspect1", "aspect2"]
}}"#,
            &state.keywords[..state.keywords.len().min(15)],
            &state.topics,
            amt.content,
            amt.branch_count(),
            amt.count_nodes(),
            amt.children
                .iter()
                .map(|c| format!("- {}: {} children", c.content, c.children.len()))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let validate_input = serde_json::json!({
            "prompt": validate_prompt,
            "max_tokens": 500,
            "temperature": 0.2,
            "system_context": "You validate Abstract Meaning Trees. Be strict about completeness. Respond with JSON only."
        });

        let result = self.executor.execute(9, validate_input).await?;
        let response = result
            .get("response")
            .and_then(|r| r.as_str())
            .unwrap_or("{}");
        let validation_json = Self::parse_json_object(response);

        let is_valid = validation_json
            .get("valid")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut issues: Vec<String> = validation_json
            .get("issues")
            .and_then(|i| i.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(missing) = validation_json
            .get("missing_aspects")
            .and_then(|m| m.as_array())
        {
            for aspect in missing {
                if let Some(a_str) = aspect.as_str() {
                    issues.push(format!("Missing aspect: {}", a_str));
                }
            }
        }

        Ok(ValidationResult { is_valid, issues })
    }

    /// Refine AMT based on validation issues (surgical edits, not overwrite)
    async fn refine_amt_from_issues(
        &self,
        state: &mut OrchestrationState,
        issues: &[String],
    ) -> Result<(), String> {
        let amt = match &mut state.amt {
            Some(a) => a,
            None => return Ok(()),
        };

        let refine_prompt = format!(
            r#"Suggest specific additions to fix these AMT issues.

CURRENT AMT ROOT: {}
CURRENT BRANCHES: {:?}

ISSUES TO FIX:
{}

Return JSON with SPECIFIC nodes to add:
{{
    "add_branches": ["new_branch1", "new_branch2"],
    "add_to_existing": [
        {{"branch": "existing_branch_name", "add_children": ["child1", "child2"]}}
    ],
    "add_relationships": [
        {{"from_branch": "branch1", "to_branch": "branch2", "type": "depends_on"}}
    ]
}}"#,
            amt.content,
            amt.children.iter().map(|c| &c.content).collect::<Vec<_>>(),
            issues
                .iter()
                .map(|i| format!("- {}", i))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let refine_input = serde_json::json!({
            "prompt": refine_prompt,
            "max_tokens": 600,
            "temperature": 0.3,
            "system_context": "Suggest AMT refinements. Respond with JSON only."
        });

        if let Ok(refine_result) = self.executor.execute(9, refine_input).await {
            let response = refine_result
                .get("response")
                .and_then(|r| r.as_str())
                .unwrap_or("{}");
            let refine_json = Self::parse_json_object(response);

            let mut next_id = amt.count_nodes() as u64 + 100;

            // Add new branches
            if let Some(new_branches) = refine_json.get("add_branches").and_then(|b| b.as_array()) {
                for branch in new_branches {
                    if let Some(branch_str) = branch.as_str() {
                        let new_branch =
                            AMTNode::new(next_id, AMTNodeType::Branch, branch_str.to_string(), 1);
                        next_id += 1;
                        amt.children.push(new_branch);
                    }
                }
            }

            // Add children to existing branches
            if let Some(additions) = refine_json
                .get("add_to_existing")
                .and_then(|a| a.as_array())
            {
                for addition in additions {
                    if let (Some(branch_name), Some(children)) = (
                        addition.get("branch").and_then(|b| b.as_str()),
                        addition.get("add_children").and_then(|c| c.as_array()),
                    ) {
                        // Find the branch
                        if let Some(branch) = amt.children.iter_mut().find(|c| {
                            c.content
                                .to_lowercase()
                                .contains(&branch_name.to_lowercase())
                        }) {
                            for child in children {
                                if let Some(child_str) = child.as_str() {
                                    let new_child = AMTNode::new(
                                        next_id,
                                        AMTNodeType::Leaf,
                                        child_str.to_string(),
                                        2,
                                    );
                                    next_id += 1;
                                    branch.children.push(new_child);
                                }
                            }
                        }
                    }
                }
            }

            // Add relationships
            if let Some(rels) = refine_json
                .get("add_relationships")
                .and_then(|r| r.as_array())
            {
                for rel in rels {
                    if let (Some(from_name), Some(to_name), Some(rel_type)) = (
                        rel.get("from_branch").and_then(|f| f.as_str()),
                        rel.get("to_branch").and_then(|t| t.as_str()),
                        rel.get("type").and_then(|t| t.as_str()),
                    ) {
                        // Find from and to branches
                        let to_id = amt
                            .children
                            .iter()
                            .find(|c| c.content.to_lowercase().contains(&to_name.to_lowercase()))
                            .map(|c| c.id);

                        if let Some(target_id) = to_id {
                            if let Some(from_branch) = amt.children.iter_mut().find(|c| {
                                c.content.to_lowercase().contains(&from_name.to_lowercase())
                            }) {
                                let relation_type = match rel_type {
                                    "depends_on" => AMTRelationType::DependsOn,
                                    "requires" => AMTRelationType::Requires,
                                    "relates_to" => AMTRelationType::RelatesTo,
                                    _ => AMTRelationType::RelatesTo,
                                };

                                from_branch.relationships.push(AMTRelation {
                                    target_id,
                                    relation_type,
                                    confidence: 0.8,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(())
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

        // Only use if 100% match (or very close)
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

        // Generate blueprint from AMT
        let blueprint_prompt = format!(
            r#"Create a blueprint (execution plan) from this AMT.

AMT ROOT: {}
BRANCHES:
{}

Available Pipelines:{}
METHODOLOGIES: {:?}

Generate execution steps. Each step should map to one AMT branch.
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
            "wait_for_graph_update": false
        }}
    ],
    "missing_capabilities": ["capability1", "capability2"]
}}"#,
            amt.content,
            amt.children
                .iter()
                .map(|c| format!(
                    "- {}: {} children, relationships: {:?}",
                    c.content,
                    c.children.len(),
                    c.relationships.len()
                ))
                .collect::<Vec<_>>()
                .join("\n"),
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
                "steps": state.blueprint_steps
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
                "Created new blueprint with {} steps",
                state.blueprint_steps.len()
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

        // STAGE 7: Create task
        let task_input = serde_json::json!({
            "action": "Create",
            "blueprint_id": state.blueprint_id,
            "inputs": {
                "prompt": state.cleaned_prompt,
                "amt_intent": state.amt.as_ref().map(|a| &a.content)
            },
            "workspace_id": state.request.workspace_id,
            "project_id": state.request.project_id,
            "user_id": state.request.user_id,
            "device_id": state.request.device_id,
            "total_steps": state.blueprint_steps.len()
        });

        let task_result = self.executor.execute(5, task_input).await?;
        state.task_id = task_result.get("task_id").and_then(|id| id.as_u64());

        self.record_stage(
            state,
            7,
            "Task Creation",
            state.task_id.is_some(),
            &format!("Task: {:?}", state.task_id),
        );

        // Build dependency graph
        let completed_steps: HashSet<u32> = HashSet::new();
        let steps = state.blueprint_steps.clone();
        let mut all_outputs: Vec<String> = Vec::new();

        // STAGES 6 & 8: Per-step context aggregation and execution
        let mut step_queue: Vec<&BlueprintStep> = steps.iter().collect();
        let mut completed: HashSet<u32> = HashSet::new();
        let mut iterations = 0;
        let max_iterations = steps.len() * 2; // Safety limit

        while !step_queue.is_empty() && iterations < max_iterations {
            iterations += 1;

            // Find steps whose dependencies are satisfied
            let ready_steps: Vec<_> = step_queue
                .iter()
                .filter(|s| s.depends_on.iter().all(|dep| completed.contains(dep)))
                .cloned()
                .collect();

            if ready_steps.is_empty() {
                // Deadlock - force execute first remaining step
                if let Some(step) = step_queue.first() {
                    let step = *step;
                    let result = self.execute_step(state, step, &all_outputs).await?;
                    all_outputs.push(self.extract_output_text(&result.output));
                    state.step_results.push(result);
                    state.step_outputs.insert(
                        step.step_index,
                        serde_json::json!({"output": all_outputs.last()}),
                    );
                    completed.insert(step.step_index);
                    step_queue.retain(|s| s.step_index != step.step_index);
                }
            } else {
                // Execute ready steps (could be parallelized in future)
                for step in ready_steps {
                    let result = self.execute_step(state, step, &all_outputs).await?;
                    all_outputs.push(self.extract_output_text(&result.output));
                    state.step_results.push(result);
                    state.step_outputs.insert(
                        step.step_index,
                        serde_json::json!({"output": all_outputs.last()}),
                    );
                    completed.insert(step.step_index);
                    step_queue.retain(|s| s.step_index != step.step_index);

                    // Update task progress
                    if let Some(task_id) = state.task_id {
                        let progress = (completed.len() as f32 / steps.len() as f32 * 100.0) as u32;
                        let progress_input = serde_json::json!({
                            "action": "UpdateProgress",
                            "task_id": task_id,
                            "step_completed": step.step_index,
                            "progress_percent": progress
                        });
                        let _ = self.executor.execute(5, progress_input).await;
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

        // Update task status
        if let Some(task_id) = state.task_id {
            let status = if state.final_response.is_some() {
                "completed"
            } else {
                "failed"
            };
            let update_input = serde_json::json!({
                "action": "UpdateStatus",
                "task_id": task_id,
                "status": status,
                "total_tokens": state.tokens_used_so_far
            });
            let _ = self.executor.execute(5, update_input).await;
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

        // Per Master Alignment STAGE 13:
        // - experience_memory.StoreExperience()
        // - relationship.RecordInteraction()
        // - emotional_state.UpdateState()
        // - self_model.IntegrateNarrative()
        // - IF significant: collective_consciousness.Prepare()

        // Store experience
        let experience_input = serde_json::json!({
            "action": "StoreExperience",
            "experience_type": if state.final_response.is_some() { "task_success" } else { "task_failure" },
            "summary": format!("Processed: {}", &state.normalized_prompt[..100.min(state.normalized_prompt.len())]),
            "task_id": state.task_id,
            "user_id": state.request.user_id,
            "tags": ["task", "prompt"],
            "significance": if state.final_response.is_some() { 0.5 } else { 0.3 }
        });
        let _ = self.executor.execute(41, experience_input).await; // experience_memory is #41

        // Record interaction for relationship tracking
        let relationship_input = serde_json::json!({
            "action": "RecordInteraction",
            "user_id": state.request.user_id,
            "interaction_type": "task_completion",
            "outcome": if state.final_response.is_some() { "positive" } else { "negative" }
        });
        let _ = self.executor.execute(55, relationship_input).await; // relationship is #55

        // Update emotional state
        let emotion_input = serde_json::json!({
            "action": "ProcessTrigger",
            "trigger_type": if state.final_response.is_some() { "task_success" } else { "task_failure" },
            "source": "orchestrator",
            "intensity": 0.5
        });
        let _ = self.executor.execute(40, emotion_input).await; // emotional_state is #40

        // Integrate into self narrative
        let narrative_input = serde_json::json!({
            "action": "IntegrateNarrative",
            "event_type": "task_completion",
            "summary": &state.normalized_prompt[..50.min(state.normalized_prompt.len())],
            "outcome": if state.final_response.is_some() { "success" } else { "failure" }
        });
        let _ = self.executor.execute(43, narrative_input).await; // self_model is #43

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

        // Per Master Alignment STAGE 14:
        // - Traverse step progression
        // - Generate summary/overview of what was done
        // - Do NOT repeat all context (would exceed limits)
        // - IF consciousness: self_model.ApplyVoiceIdentity()
        // - Update UI (workspace_tab)
        // - consciousness_dashboard.Update()
        // - task_recommendation.Suggest()

        // If consciousness enabled, apply voice identity to response
        if state.request.consciousness_enabled && state.final_response.is_some() {
            // Get voice identity from self_model
            let voice_input = serde_json::json!({
                "action": "GetVoice"
            });

            if let Ok(voice_result) = self.executor.execute(43, voice_input).await {
                if let Some(voice) = voice_result.get("voice") {
                    state.voice_identity = serde_json::from_value(voice.clone()).ok();

                    // Apply voice identity to response (modify tone/style)
                    // This is done via prompt pipeline with voice context
                    if let Some(response) = &state.final_response {
                        if let Some(voice_id) = &state.voice_identity {
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
                                "system_context": "You are applying a voice identity to a response. Maintain the content but adjust the tone and style."
                            });

                            // Only restyle if voice identity differs significantly from neutral
                            let needs_restyle = voice_id.formality < 0.4
                                || voice_id.formality > 0.6
                                || voice_id.warmth < 0.4
                                || voice_id.warmth > 0.6;

                            if needs_restyle {
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
            }

            // Update consciousness dashboard
            let dashboard_input = serde_json::json!({
                "action": "Update",
                "task_completed": true,
                "task_id": state.task_id,
                "success": state.final_response.is_some()
            });
            let _ = self.executor.execute(54, dashboard_input).await; // consciousness_dashboard is #54
        }

        // Generate task recommendations for next steps
        let recommend_input = serde_json::json!({
            "action": "Suggest",
            "context": &state.normalized_prompt[..100.min(state.normalized_prompt.len())],
            "completed_task_id": state.task_id
        });
        let _ = self.executor.execute(23, recommend_input).await; // task_recommendation is #23

        self.record_stage_timed(
            state,
            11,
            "Response Delivery",
            state.final_response.is_some(),
            &format!(
                "Response: {} chars, Voice identity: {}",
                state.final_response.as_ref().map(|r| r.len()).unwrap_or(0),
                state.voice_identity.is_some()
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

    #[tokio::test]
    async fn test_basic_orchestration() {
        let executor = Arc::new(MockExecutor);
        let zsei = Arc::new(MockZSEI);
        let orchestrator = PromptOrchestrator::new(executor, zsei);

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
}
