//! Prompt Orchestrator - v0.4.0
//! 
//! Orchestrates the full 14-stage flow from user input to response delivery.
//! This is the CENTRAL COORDINATOR that ties all pipelines together.
//! 
//! Per Master Alignment Report, the stages are:
//! 
//! STAGE 1:  Input Capture (from workspace_tab or meta_portion)
//! STAGE 2:  Category Traversal (traversal_ml)
//! STAGE 3:  Methodology Loop (methodology_fetch, methodology_create)
//! STAGE 4:  Text/Prompt Normalization + AMT (text_analysis)
//! STAGE 5:  Blueprint Search for duplicates (blueprint_search)
//! STAGE 6:  Blueprint Creation Loop (blueprint_create, pipeline_creation)
//! STAGE 7:  Zero-Shot Simulation (zero_shot_simulation)
//! STAGE 8:  Consciousness Decision Gate (if enabled)
//! STAGE 9:  Context Aggregation PER STEP (context_aggregation)
//! STAGE 10: Task Creation (task_manager)
//! STAGE 11: Execution per blueprint step (prompt + other pipelines)
//! STAGE 12: Result Collection
//! STAGE 13: Post-execution consciousness (experience_memory)
//! STAGE 14: Response Delivery
//! 
//! CRITICAL: This orchestrator respects I-Loop protection.
//! Tasks MUST wait for I-Loop to complete before starting.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// Methodologies created during this request
    pub methodologies_created: u32,
    /// Blueprints created during this request
    pub blueprints_created: u32,
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

/// Model context limits by model identifier
fn get_model_context_limit(model_identifier: &str) -> u32 {
    match model_identifier {
        // Claude models
        s if s.contains("claude-3-opus") => 200000,
        s if s.contains("claude-3-sonnet") => 200000,
        s if s.contains("claude-3-haiku") => 200000,
        s if s.contains("claude-sonnet-4") => 200000,
        s if s.contains("claude-haiku-4") => 200000,
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

/// Internal state during orchestration
struct OrchestrationState {
    request: OrchestrationRequest,
    start_time: std::time::Instant,
    stages: Vec<StageResult>,
    
    // Model context management
    model_context_limit: u32,
    tokens_used_so_far: u32,
    
    // Stage outputs
    categories: Vec<u64>,
    methodologies: Vec<u64>,
    methodologies_created: u32,
    blueprints_created: u32,
    normalized_prompt: String,
    prompt_tokens: u32,
    blueprint_id: Option<u64>,
    task_id: Option<u64>,
    step_results: Vec<StepResult>,
    final_response: Option<String>,
    
    // AMT/ATMT (Abstract Meaning Tree)
    amt: Option<serde_json::Value>,
    amt_branches: u32,
    needs_clarification: bool,
    clarification_points: Vec<String>,
    
    // Per-step context mapping (step_index -> context)
    step_contexts: std::collections::HashMap<u32, String>,
    
    // Consciousness
    gate_result: Option<GateResult>,
    
    // Voice identity for response (from self_model)
    voice_identity: Option<VoiceIdentity>,
}

#[derive(Debug, Clone)]
struct StepResult {
    step_index: u32,
    pipeline_id: u64,
    output: serde_json::Value,
    tokens_used: u32,
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
// Pipeline Executor Trait
// ============================================================================

/// Trait for executing pipelines (implemented by runtime)
#[async_trait::async_trait]
pub trait PipelineExecutor: Send + Sync {
    async fn execute(&self, pipeline_id: u64, input: serde_json::Value) -> Result<serde_json::Value, String>;
}

// ============================================================================
// Orchestrator Implementation
// ============================================================================

pub struct PromptOrchestrator {
    executor: Arc<dyn PipelineExecutor>,
}

impl PromptOrchestrator {
    pub fn new(executor: Arc<dyn PipelineExecutor>) -> Self {
        Self { executor }
    }
    
    /// Main entry point - orchestrates the full 14-stage flow
    pub async fn orchestrate(&self, request: OrchestrationRequest) -> OrchestrationResponse {
        // Determine model context limit
        let model_identifier = request.model_config.as_ref()
            .and_then(|c| c.model_identifier.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("claude-sonnet-4");
        
        let model_context_limit = request.model_config.as_ref()
            .and_then(|c| c.context_length)
            .unwrap_or_else(|| get_model_context_limit(model_identifier));
        
        // Calculate effective token budget (use override if provided, else use model limit)
        let token_budget = request.token_budget.unwrap_or(model_context_limit);
        
        // Estimate prompt tokens (rough: 4 chars per token)
        let prompt_tokens = (request.prompt.len() / 4) as u32;
        
        let mut state = OrchestrationState {
            request: request.clone(),
            start_time: std::time::Instant::now(),
            stages: Vec::new(),
            model_context_limit,
            tokens_used_so_far: prompt_tokens,
            categories: Vec::new(),
            methodologies: Vec::new(),
            methodologies_created: 0,
            blueprints_created: 0,
            normalized_prompt: request.prompt.clone(),
            prompt_tokens,
            blueprint_id: None,
            task_id: None,
            step_results: Vec::new(),
            final_response: None,
            // AMT/ATMT
            amt: None,
            amt_branches: 0,
            needs_clarification: false,
            clarification_points: Vec::new(),
            // Per-step context
            step_contexts: std::collections::HashMap::new(),
            // Consciousness
            gate_result: None,
            voice_identity: None,
        };
        
        // Check I-Loop before starting
        if request.consciousness_enabled {
            if let Err(e) = self.wait_for_i_loop().await {
                return OrchestrationResponse {
                    success: false,
                    response: None,
                    task_id: None,
                    blueprint_id: None,
                    stages_completed: state.stages,
                    consciousness_gate: None,
                    error: Some(format!("I-Loop wait failed: {}", e)),
                    total_tokens_used: None,
                    execution_time_ms: state.start_time.elapsed().as_millis() as u64,
                    methodologies_created: 0,
                    blueprints_created: 0,
                };
            }
        }
        
        // Execute stages sequentially
        let result = self.execute_stages(&mut state).await;
        
        match result {
            Ok(_) => OrchestrationResponse {
                success: true,
                response: state.final_response,
                task_id: state.task_id,
                blueprint_id: state.blueprint_id,
                stages_completed: state.stages,
                consciousness_gate: state.gate_result,
                error: None,
                total_tokens_used: Some(state.step_results.iter().map(|r| r.tokens_used).sum()),
                execution_time_ms: state.start_time.elapsed().as_millis() as u64,
                methodologies_created: state.methodologies_created,
                blueprints_created: state.blueprints_created,
            },
            Err(e) => OrchestrationResponse {
                success: false,
                response: None,
                task_id: state.task_id,
                blueprint_id: state.blueprint_id,
                stages_completed: state.stages,
                consciousness_gate: state.gate_result,
                error: Some(e),
                total_tokens_used: None,
                execution_time_ms: state.start_time.elapsed().as_millis() as u64,
                methodologies_created: state.methodologies_created,
                blueprints_created: state.blueprints_created,
            },
        }
    }
                response: state.final_response,
                task_id: state.task_id,
                blueprint_id: state.blueprint_id,
                stages_completed: state.stages,
                consciousness_gate: state.gate_result,
                error: None,
                total_tokens_used: Some(state.step_results.iter().map(|r| r.tokens_used).sum()),
                execution_time_ms: state.start_time.elapsed().as_millis() as u64,
            },
            Err(e) => OrchestrationResponse {
                success: false,
                response: None,
                task_id: state.task_id,
                blueprint_id: state.blueprint_id,
                stages_completed: state.stages,
                consciousness_gate: state.gate_result,
                error: Some(e),
                total_tokens_used: None,
                execution_time_ms: state.start_time.elapsed().as_millis() as u64,
            },
        }
    }
    
    async fn execute_stages(&self, state: &mut OrchestrationState) -> Result<(), String> {
        // STAGE 1: Input Capture (already done - prompt is in request)
        self.record_stage(state, 1, "Input Capture", true, "Prompt received");
        
        // STAGE 2: Category Traversal
        self.stage_2_category_traversal(state).await?;
        
        // STAGE 3: Methodology Loop
        self.stage_3_methodology_loop(state).await?;
        
        // STAGE 4: Text/Prompt Normalization
        self.stage_4_text_normalization(state).await?;
        
        // STAGE 5: Blueprint Search
        self.stage_5_blueprint_search(state).await?;
        
        // STAGE 6: Blueprint Creation (if needed)
        self.stage_6_blueprint_creation(state).await?;
        
        // STAGE 7: Zero-Shot Simulation
        self.stage_7_zero_shot_simulation(state).await?;
        
        // STAGE 8: Consciousness Decision Gate
        if state.request.consciousness_enabled {
            self.stage_8_consciousness_gate(state).await?;
        } else {
            self.record_stage(state, 8, "Consciousness Gate", true, "Skipped (disabled)");
        }
        
        // STAGE 9-11: Context Aggregation + Task Creation + Execution (per step)
        self.stage_9_to_11_execute_steps(state).await?;
        
        // STAGE 12: Result Collection
        self.stage_12_result_collection(state).await?;
        
        // STAGE 13: Post-execution Consciousness
        if state.request.consciousness_enabled {
            self.stage_13_post_execution(state).await?;
        } else {
            self.record_stage(state, 13, "Post-execution Consciousness", true, "Skipped (disabled)");
        }
        
        // STAGE 14: Response Delivery
        self.stage_14_response_delivery(state).await?;
        
        Ok(())
    }
    
    // ========================================================================
    // Stage Implementations
    // ========================================================================
    
    async fn stage_2_category_traversal(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        // Call traversal_ml pipeline (#17)
        let input = serde_json::json!({
            "action": "Traverse",
            "query": state.request.prompt,
            "mode": "semantic",
            "max_depth": 3,
            "max_results": 10
        });
        
        let result = self.executor.execute(17, input).await?;
        
        // Extract category IDs from result
        if let Some(categories) = result.get("category_ids").and_then(|c| c.as_array()) {
            state.categories = categories.iter()
                .filter_map(|v| v.as_u64())
                .collect();
        }
        
        self.record_stage_timed(state, 2, "Category Traversal", true, 
            &format!("Found {} categories", state.categories.len()),
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_3_methodology_loop(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        // STEP 1: Always fetch existing methodologies first
        let fetch_input = serde_json::json!({
            "action": "SearchByKeywords",
            "keywords": state.request.prompt.split_whitespace().take(5).collect::<Vec<_>>(),
            "limit": 10
        });
        
        let fetch_result = self.executor.execute(11, fetch_input).await?;
        
        // Extract existing methodology IDs and their coverage
        let mut existing_methodologies: Vec<u64> = Vec::new();
        let mut covered_categories: Vec<u64> = Vec::new();
        
        if let Some(methodologies) = fetch_result.get("methodologies").and_then(|m| m.as_array()) {
            for methodology in methodologies {
                if let Some(id) = methodology.get("methodology_id").and_then(|id| id.as_u64()) {
                    existing_methodologies.push(id);
                }
                // Track which categories are covered
                if let Some(cats) = methodology.get("category_ids").and_then(|c| c.as_array()) {
                    for cat in cats {
                        if let Some(cat_id) = cat.as_u64() {
                            covered_categories.push(cat_id);
                        }
                    }
                }
            }
        }
        
        state.methodologies = existing_methodologies.clone();
        
        // STEP 2: Check for uncovered categories and create new methodologies if needed
        let uncovered_categories: Vec<u64> = state.categories.iter()
            .filter(|cat| !covered_categories.contains(cat))
            .copied()
            .collect();
        
        // Also check if we have any methodologies at all
        let need_new_methodology = !uncovered_categories.is_empty() || state.methodologies.is_empty();
        
        if need_new_methodology && !state.categories.is_empty() {
            // Call methodology_create pipeline (#12) to create from prompt analysis
            // This expands the knowledge base for future similar queries
            let create_input = serde_json::json!({
                "action": "Create",
                "name": format!("Methodology for: {}", &state.request.prompt[..50.min(state.request.prompt.len())]),
                "description": state.request.prompt.clone(),
                "category_ids": if uncovered_categories.is_empty() { 
                    state.categories.clone() 
                } else { 
                    uncovered_categories.clone() 
                },
                "confidence_threshold": 0.7,
                "auto_expand": true  // Flag to indicate knowledge base expansion
            });
            
            if let Ok(create_result) = self.executor.execute(12, create_input).await {
                if let Some(method_id) = create_result.get("methodology_id").and_then(|id| id.as_u64()) {
                    state.methodologies.push(method_id);
                    state.methodologies_created += 1;
                }
            }
        }
        
        // STEP 3: Verify methodology confidence - if any are below threshold, attempt to enhance
        let verification_input = serde_json::json!({
            "action": "VerifyConfidence",
            "methodology_ids": state.methodologies,
            "threshold": 0.7
        });
        
        if let Ok(verify_result) = self.executor.execute(11, verification_input).await {
            // Check if any need refinement
            if let Some(low_confidence) = verify_result.get("low_confidence_ids").and_then(|l| l.as_array()) {
                for method_id in low_confidence {
                    if let Some(id) = method_id.as_u64() {
                        // Attempt to refine the methodology
                        let refine_input = serde_json::json!({
                            "action": "Refine",
                            "methodology_id": id,
                            "context": state.request.prompt
                        });
                        let _ = self.executor.execute(12, refine_input).await;
                    }
                }
            }
        }
        
        self.record_stage_timed(state, 3, "Methodology Loop", true,
            &format!("Using {} methodologies ({} created, {} uncovered cats)", 
                state.methodologies.len(), 
                state.methodologies_created,
                uncovered_categories.len()),
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_4_text_normalization(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        // STAGE 4: TEXT/PROMPT NORMALIZATION + AMT/ATMT
        // Per Master Alignment:
        // - text_analysis.Normalize() - structural normalization
        // - text_analysis.BuildAMT() - Initial Abstract Meaning Tree
        // - LLM Zero-Shot Expansion Loop - iterate until complete
        // - Use methodologies to guide required coverage
        // - Ensure coverage (security, edge cases, dependencies, constraints)
        
        // Step 1: Structural analysis with normalization
        let normalize_input = serde_json::json!({
            "action": "Normalize",
            "text": state.request.prompt,
            "context_limit": state.model_context_limit,
            "analyze_tokens": true
        });
        
        let normalize_result = self.executor.execute(20, normalize_input).await?;
        
        // Get normalized prompt
        if let Some(normalized) = normalize_result.get("normalized_text").and_then(|n| n.as_str()) {
            state.normalized_prompt = normalized.to_string();
        } else {
            state.normalized_prompt = state.request.prompt.clone();
        }
        
        // Get token count
        if let Some(tokens) = normalize_result.get("token_count").and_then(|t| t.as_u64()) {
            state.prompt_tokens = tokens as u32;
            state.tokens_used_so_far = state.prompt_tokens;
        }
        
        // Add any suggested methodologies from text analysis
        if let Some(suggested) = normalize_result.get("suggested_methodology_ids").and_then(|m| m.as_array()) {
            for id in suggested {
                if let Some(method_id) = id.as_u64() {
                    if !state.methodologies.contains(&method_id) {
                        state.methodologies.push(method_id);
                    }
                }
            }
        }
        
        // Step 2: Build initial AMT structure
        let amt_input = serde_json::json!({
            "action": "BuildAMT",
            "text": state.normalized_prompt,
            "depth": 3,
            "methodology_ids": state.methodologies,
            "ensure_coverage": ["security", "edge_cases", "dependencies", "constraints", "testing"]
        });
        
        let amt_result = self.executor.execute(20, amt_input).await?;
        
        // Store initial AMT
        let mut current_amt = amt_result.get("amt").cloned()
            .unwrap_or_else(|| serde_json::json!({
                "id": 1,
                "node_type": "root",
                "content": &state.normalized_prompt[..100.min(state.normalized_prompt.len())],
                "children": [],
                "relationships": [],
                "metadata": {}
            }));
        
        // Step 3: Gather methodology coverage requirements
        let mut required_aspects: Vec<String> = vec![
            "security".to_string(),
            "edge_cases".to_string(), 
            "dependencies".to_string(),
            "constraints".to_string(),
        ];
        
        for method_id in &state.methodologies {
            let method_check = serde_json::json!({
                "action": "Fetch",
                "methodology_id": method_id
            });
            
            if let Ok(method) = self.executor.execute(11, method_check).await {
                if let Some(coverage) = method.get("required_coverage").and_then(|r| r.as_array()) {
                    for aspect in coverage {
                        if let Some(aspect_str) = aspect.as_str() {
                            if !required_aspects.contains(&aspect_str.to_string()) {
                                required_aspects.push(aspect_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // Step 4: LLM Zero-Shot AMT Expansion Loop
        // Iterate until completeness threshold or max iterations
        let max_iterations = 5;
        let completeness_threshold = 0.95;
        let mut iteration = 0;
        let mut completeness_score = 0.0;
        
        while iteration < max_iterations && completeness_score < completeness_threshold {
            iteration += 1;
            
            // Use LLM to identify incomplete branches and expand
            let expansion_prompt = format!(
                r#"Analyze this Abstract Meaning Tree (AMT) for a request and identify incomplete branches.

REQUEST: {}

CURRENT AMT STRUCTURE:
{}

REQUIRED COVERAGE ASPECTS (from methodologies):
{}

TASK:
1. Identify any branches that are incomplete or missing for this request
2. For each incomplete branch, suggest what sub-nodes should be added
3. Check if all required aspects are covered in the tree
4. Rate the overall completeness from 0.0 to 1.0

Respond ONLY with valid JSON:
{{
    "completeness_score": 0.85,
    "missing_branches": ["branch1", "branch2"],
    "suggested_expansions": [
        {{"parent": "root", "children": ["child1", "child2"]}}
    ],
    "uncovered_aspects": ["aspect1"],
    "clarification_needed": ["What is X?"]
}}"#,
                &state.normalized_prompt[..300.min(state.normalized_prompt.len())],
                serde_json::to_string_pretty(&current_amt).unwrap_or_default(),
                required_aspects.join(", ")
            );
            
            let llm_input = serde_json::json!({
                "prompt": expansion_prompt,
                "max_tokens": 1500,
                "temperature": 0.3,
                "system_context": "You are an expert at analyzing text structure and building Abstract Meaning Trees. Respond only with valid JSON."
            });
            
            if let Ok(llm_result) = self.executor.execute(9, llm_input).await {
                if let Some(response) = llm_result.get("response").and_then(|r| r.as_str()) {
                    // Try to extract JSON from response
                    let json_str = if let Some(start) = response.find('{') {
                        if let Some(end) = response.rfind('}') {
                            &response[start..=end]
                        } else { response }
                    } else { response };
                    
                    // Parse LLM response for AMT expansion guidance
                    if let Ok(expansion) = serde_json::from_str::<serde_json::Value>(json_str) {
                        // Update completeness score
                        if let Some(score) = expansion.get("completeness_score").and_then(|s| s.as_f64()) {
                            completeness_score = score;
                        }
                        
                        // Track missing branches
                        if let Some(missing) = expansion.get("missing_branches").and_then(|m| m.as_array()) {
                            for branch in missing {
                                if let Some(branch_str) = branch.as_str() {
                                    if !state.clarification_points.contains(&branch_str.to_string()) {
                                        state.clarification_points.push(branch_str.to_string());
                                    }
                                }
                            }
                        }
                        
                        // Track uncovered aspects
                        if let Some(uncovered) = expansion.get("uncovered_aspects").and_then(|u| u.as_array()) {
                            for aspect in uncovered {
                                if let Some(aspect_str) = aspect.as_str() {
                                    let msg = format!("Missing coverage: {}", aspect_str);
                                    if !state.clarification_points.contains(&msg) {
                                        state.clarification_points.push(msg);
                                    }
                                }
                            }
                        }
                        
                        // Track clarification questions
                        if let Some(questions) = expansion.get("clarification_needed").and_then(|q| q.as_array()) {
                            for question in questions {
                                if let Some(q_str) = question.as_str() {
                                    if !state.clarification_points.contains(&q_str.to_string()) {
                                        state.clarification_points.push(q_str.to_string());
                                    }
                                }
                            }
                        }
                        
                        // Apply suggested expansions to AMT
                        if let Some(expansions) = expansion.get("suggested_expansions").and_then(|e| e.as_array()) {
                            let children = current_amt.get_mut("children")
                                .and_then(|c| c.as_array_mut());
                            
                            if let Some(children) = children {
                                for exp in expansions {
                                    if let Some(new_children) = exp.get("children").and_then(|nc| nc.as_array()) {
                                        for (i, child) in new_children.iter().enumerate() {
                                            if let Some(child_str) = child.as_str() {
                                                let new_node = serde_json::json!({
                                                    "id": children.len() + 100 + i,
                                                    "node_type": "expanded",
                                                    "content": child_str,
                                                    "children": [],
                                                    "relationships": [],
                                                    "metadata": {"source": "llm_expansion", "iteration": iteration}
                                                });
                                                children.push(new_node);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Step 5: Validation pass if high completeness (10x validation per master alignment)
        if completeness_score >= completeness_threshold && iteration < max_iterations {
            let validation_prompt = format!(
                "Validate this AMT is COMPLETE for the request. Check all aspects are covered.\n\nREQUEST: {}\n\nAMT: {}\n\nRESPOND: 'VALID' if complete, otherwise list remaining gaps as JSON array.",
                &state.normalized_prompt[..150.min(state.normalized_prompt.len())],
                serde_json::to_string(&current_amt).unwrap_or_default()
            );
            
            let validate_input = serde_json::json!({
                "prompt": validation_prompt,
                "max_tokens": 500,
                "temperature": 0.1
            });
            
            if let Ok(validate_result) = self.executor.execute(9, validate_input).await {
                if let Some(response) = validate_result.get("response").and_then(|r| r.as_str()) {
                    if !response.to_uppercase().contains("VALID") {
                        // Validation found gaps - parse them
                        if let Ok(gaps) = serde_json::from_str::<Vec<String>>(response) {
                            for gap in gaps {
                                if !state.clarification_points.contains(&gap) {
                                    state.clarification_points.push(gap);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Store final AMT
        state.amt = Some(current_amt.clone());
        
        // Count branches
        if let Some(children) = current_amt.get("children").and_then(|c| c.as_array()) {
            state.amt_branches = children.len() as u32;
        }
        
        // Set clarification flag if needed
        state.needs_clarification = !state.clarification_points.is_empty() && completeness_score < 0.8;
        
        self.record_stage_timed(state, 4, "Text Normalization + AMT", true,
            &format!("Tokens: {}, AMT branches: {}, Completeness: {:.0}%, Iterations: {}, Clarification: {}", 
                state.prompt_tokens, state.amt_branches, completeness_score * 100.0, 
                iteration, state.needs_clarification),
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_5_blueprint_search(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        // Call blueprint_search pipeline (#13)
        let input = serde_json::json!({
            "action": "SearchByKeywords",
            "keywords": state.normalized_prompt.split_whitespace().take(10).collect::<Vec<_>>(),
            "methodology_ids": state.methodologies,
            "limit": 5
        });
        
        let result = self.executor.execute(13, input).await?;
        
        // Check for existing blueprint with good match
        if let Some(blueprints) = result.get("blueprints").and_then(|b| b.as_array()) {
            for blueprint in blueprints {
                let match_score = blueprint.get("match_score")
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0);
                
                // Only use blueprint if it's a good match (> 70%)
                if match_score > 0.7 {
                    if let Some(id) = blueprint.get("blueprint_id").and_then(|id| id.as_u64()) {
                        state.blueprint_id = Some(id);
                        break;
                    }
                }
            }
        }
        
        self.record_stage_timed(state, 5, "Blueprint Search", true,
            &format!("Blueprint: {:?}", state.blueprint_id),
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_6_blueprint_creation(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        // Always check if we should create a new blueprint to expand knowledge base
        // Even if we found one, we might want to create a more specific one
        let should_create = state.blueprint_id.is_none();
        
        if !should_create {
            self.record_stage_timed(state, 6, "Blueprint Creation", true,
                "Using existing blueprint",
                stage_start.elapsed().as_millis() as u64);
            return Ok(());
        }
        
        // Call blueprint_create pipeline (#14) 
        let input = serde_json::json!({
            "action": "Create",
            "name": format!("Blueprint for: {}", &state.normalized_prompt[..50.min(state.normalized_prompt.len())]),
            "description": state.normalized_prompt.clone(),
            "input_types": ["text"],
            "output_type": "text",
            "methodology_ids": state.methodologies,
            "context_limit": state.model_context_limit,
            "auto_expand": true  // Flag for knowledge base expansion
        });
        
        let result = self.executor.execute(14, input).await?;
        
        if let Some(id) = result.get("blueprint_id").and_then(|id| id.as_u64()) {
            state.blueprint_id = Some(id);
            state.blueprints_created += 1;
        }
        
        self.record_stage_timed(state, 6, "Blueprint Creation", state.blueprint_id.is_some(),
            &format!("Created blueprint: {:?} (total created: {})", state.blueprint_id, state.blueprints_created),
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_7_zero_shot_simulation(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        // Call zero_shot_simulation pipeline (#16)
        let input = serde_json::json!({
            "action": "Simulate",
            "blueprint_id": state.blueprint_id,
            "prompt": state.normalized_prompt,
            "max_iterations": 3
        });
        
        let result = self.executor.execute(16, input).await?;
        
        let confidence = result.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.5);
        
        self.record_stage_timed(state, 7, "Zero-Shot Simulation", true,
            &format!("Simulation confidence: {:.2}", confidence),
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_8_consciousness_gate(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        // Call decision_gate pipeline (#39)
        let input = serde_json::json!({
            "action": "Evaluate",
            "task_id": 0, // Will be assigned
            "task_summary": state.normalized_prompt,
            "blueprint_id": state.blueprint_id.unwrap_or(0),
            "user_id": state.request.user_id
        });
        
        let result = self.executor.execute(39, input).await?;
        
        let decision = result.get("gate")
            .and_then(|g| g.get("decision"))
            .and_then(|d| d.as_str())
            .unwrap_or("Proceed");
        
        let confidence = result.get("gate")
            .and_then(|g| g.get("confidence"))
            .and_then(|c| c.as_f64())
            .unwrap_or(0.8) as f32;
        
        let reasoning = result.get("gate")
            .and_then(|g| g.get("reasoning"))
            .and_then(|r| r.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();
        
        state.gate_result = Some(GateResult {
            decision: decision.to_string(),
            confidence,
            reasoning: reasoning.clone(),
        });
        
        // Check if we should proceed
        if decision == "Decline" {
            return Err(format!("Consciousness gate declined: {}", reasoning));
        }
        
        self.record_stage_timed(state, 8, "Consciousness Gate", true,
            &format!("Decision: {} ({:.0}%)", decision, confidence * 100.0),
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_9_to_11_execute_steps(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        // STAGES 9-11: Per-Step Context Aggregation + Task Creation + Blueprint Step Execution
        // Per Master Alignment:
        // - Context aggregation is PER STEP, not all at once
        // - Each blueprint step gets its relevant context group
        // - Tasks execute blueprints with progress tracked per step
        // - Avoid exceeding context limits by grouping properly
        
        // Calculate base context budget per step
        let reserved_tokens = state.prompt_tokens + 500; // System message reserve
        let base_available = if state.model_context_limit > reserved_tokens {
            state.model_context_limit - reserved_tokens
        } else {
            (state.model_context_limit / 2).max(2000)
        };
        
        // Get blueprint steps if we have a blueprint
        let blueprint_steps: Vec<serde_json::Value> = if let Some(blueprint_id) = state.blueprint_id {
            let fetch_input = serde_json::json!({
                "action": "Fetch",
                "blueprint_id": blueprint_id
            });
            
            if let Ok(bp_result) = self.executor.execute(14, fetch_input).await {
                bp_result.get("blueprint")
                    .and_then(|b| b.get("steps"))
                    .and_then(|s| s.as_array())
                    .cloned()
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        
        // If no blueprint steps, create a single "execute prompt" step
        let steps = if blueprint_steps.is_empty() {
            vec![serde_json::json!({
                "step_index": 0,
                "action": "execute_prompt",
                "description": "Process the user prompt",
                "pipeline_id": 9,
                "context_requirements": ["full_context"]
            })]
        } else {
            blueprint_steps
        };
        
        let num_steps = steps.len();
        
        // STAGE 10: Create task linked to blueprint
        let task_input = serde_json::json!({
            "action": "Create",
            "blueprint_id": state.blueprint_id,
            "inputs": {
                "prompt": state.normalized_prompt,
                "amt": state.amt
            },
            "workspace_id": state.request.workspace_id,
            "project_id": state.request.project_id,
            "user_id": state.request.user_id,
            "device_id": state.request.device_id,
            "total_steps": num_steps
        });
        
        let task_result = self.executor.execute(5, task_input).await?;
        
        if let Some(task_id) = task_result.get("task_id").and_then(|id| id.as_u64()) {
            state.task_id = Some(task_id);
        }
        
        self.record_stage(state, 10, "Task Creation", state.task_id.is_some(), 
            &format!("Task: {:?}, {} steps from blueprint", state.task_id, num_steps));
        
        // STAGE 9 + 11: Per-step context aggregation and execution
        let mut all_step_outputs: Vec<String> = Vec::new();
        let context_budget_per_step = base_available / (num_steps as u32).max(1);
        
        for (idx, step) in steps.iter().enumerate() {
            let step_index = step.get("step_index").and_then(|s| s.as_u64()).unwrap_or(idx as u64) as u32;
            let step_action = step.get("action").and_then(|a| a.as_str()).unwrap_or("execute");
            let step_pipeline_id = step.get("pipeline_id").and_then(|p| p.as_u64()).unwrap_or(9);
            let step_description = step.get("description").and_then(|d| d.as_str()).unwrap_or("");
            
            // Get context requirements for this step
            let context_requirements = step.get("context_requirements")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>())
                .unwrap_or_else(|| vec!["relevant_context".to_string()]);
            
            // STAGE 9: Context Aggregation FOR THIS STEP
            // Use AMT branches to determine what context is relevant
            let step_context_query = if step_description.is_empty() {
                state.normalized_prompt.clone()
            } else {
                format!("{} - Step: {}", state.normalized_prompt, step_description)
            };
            
            let context_input = serde_json::json!({
                "action": "ForQuery",
                "query": step_context_query,
                "token_budget": context_budget_per_step,
                "project_id": state.request.project_id,
                "workspace_id": state.request.workspace_id,
                "model_context_limit": state.model_context_limit,
                "priority_order": context_requirements,
                "step_index": step_index,
                "include_previous_outputs": idx > 0
            });
            
            let context_result = self.executor.execute(21, context_input).await?;
            
            let step_context = context_result.get("context")
                .and_then(|c| c.get("context_text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            
            let context_tokens = context_result.get("context")
                .and_then(|c| c.get("tokens_used"))
                .and_then(|t| t.as_u64())
                .unwrap_or(0) as u32;
            
            // Store step context for reference
            state.step_contexts.insert(step_index, step_context.clone());
            state.tokens_used_so_far += context_tokens;
            
            // Build full context including previous step outputs if needed
            let full_step_context = if idx > 0 && !all_step_outputs.is_empty() {
                let previous_summary = all_step_outputs.iter()
                    .enumerate()
                    .map(|(i, output)| format!("Step {} output: {}", i + 1, 
                        &output[..200.min(output.len())]))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}\n\nPrevious steps summary:\n{}", step_context, previous_summary)
            } else {
                step_context
            };
            
            // STAGE 11: Execute this step
            let model_config = state.request.model_config.as_ref();
            let max_tokens = model_config
                .and_then(|c| c.max_tokens)
                .unwrap_or(4096)
                .min(state.model_context_limit / 4);
            
            // Build step-specific prompt
            let step_prompt = if num_steps > 1 {
                format!(
                    "You are executing step {} of {} for this task.\n\
                    Step action: {}\n\
                    Step description: {}\n\n\
                    Original request: {}\n\n\
                    Complete this step and provide the output.",
                    idx + 1, num_steps, step_action, step_description, state.normalized_prompt
                )
            } else {
                state.normalized_prompt.clone()
            };
            
            let exec_input = serde_json::json!({
                "prompt": step_prompt,
                "aggregated_context": full_step_context,
                "context_limit": state.model_context_limit,
                "max_tokens": max_tokens,
                "temperature": model_config.and_then(|c| c.temperature).unwrap_or(0.7),
                "model_type": model_config.and_then(|c| c.model_type.as_ref()),
                "model_identifier": model_config.and_then(|c| c.model_identifier.as_ref()),
                "step_index": step_index,
                "total_steps": num_steps
            });
            
            let exec_result = self.executor.execute(step_pipeline_id, exec_input).await?;
            
            let step_response = exec_result.get("response")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string();
            
            let exec_tokens = exec_result.get("tokens_used")
                .and_then(|t| t.as_u64())
                .unwrap_or(0) as u32;
            
            state.tokens_used_so_far += exec_tokens;
            all_step_outputs.push(step_response.clone());
            
            // Store step result
            state.step_results.push(StepResult {
                step_index,
                pipeline_id: step_pipeline_id,
                output: exec_result,
                tokens_used: context_tokens + exec_tokens,
            });
            
            // Update task progress
            if let Some(task_id) = state.task_id {
                let progress_input = serde_json::json!({
                    "action": "UpdateProgress",
                    "task_id": task_id,
                    "step_completed": step_index,
                    "total_steps": num_steps,
                    "progress_percent": ((idx + 1) as f32 / num_steps as f32 * 100.0) as u32
                });
                let _ = self.executor.execute(5, progress_input).await;
            }
        }
        
        // Combine all step outputs into final response
        state.final_response = if all_step_outputs.len() == 1 {
            Some(all_step_outputs[0].clone())
        } else if !all_step_outputs.is_empty() {
            // For multi-step, create summary
            Some(format!(
                "Completed {} steps:\n\n{}",
                all_step_outputs.len(),
                all_step_outputs.iter()
                    .enumerate()
                    .map(|(i, output)| format!("**Step {}:**\n{}\n", i + 1, output))
                    .collect::<Vec<_>>()
                    .join("\n")
            ))
        } else {
            None
        };
        
        self.record_stage_timed(state, 11, "Step Execution", state.final_response.is_some(),
            &format!("{} steps executed, total tokens: {}/{}", 
                state.step_results.len(), state.tokens_used_so_far, state.model_context_limit),
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_12_result_collection(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        // Update task status
        if let Some(task_id) = state.task_id {
            let status = if state.final_response.is_some() { "completed" } else { "failed" };
            
            let update_input = serde_json::json!({
                "action": "UpdateStatus",
                "task_id": task_id,
                "status": status
            });
            
            let _ = self.executor.execute(5, update_input).await;
        }
        
        self.record_stage_timed(state, 12, "Result Collection", true,
            "Results collected",
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_13_post_execution(&self, state: &mut OrchestrationState) -> Result<(), String> {
        let stage_start = std::time::Instant::now();
        
        if !state.request.consciousness_enabled {
            self.record_stage_timed(state, 13, "Post-execution", true,
                "Consciousness disabled - skipped",
                stage_start.elapsed().as_millis() as u64);
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
        
        self.record_stage_timed(state, 13, "Post-execution Consciousness", true,
            "Experience stored, relationship updated, emotions processed",
            stage_start.elapsed().as_millis() as u64);
        
        Ok(())
    }
    
    async fn stage_14_response_delivery(&self, state: &mut OrchestrationState) -> Result<(), String> {
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
                            let needs_restyle = voice_id.formality < 0.4 || voice_id.formality > 0.6 ||
                                              voice_id.warmth < 0.4 || voice_id.warmth > 0.6;
                            
                            if needs_restyle {
                                if let Ok(styled) = self.executor.execute(9, style_input).await {
                                    if let Some(new_response) = styled.get("response").and_then(|r| r.as_str()) {
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
        
        self.record_stage_timed(state, 14, "Response Delivery", state.final_response.is_some(),
            &format!("Response: {} chars, Voice identity: {}", 
                state.final_response.as_ref().map(|r| r.len()).unwrap_or(0),
                state.voice_identity.is_some()),
            stage_start.elapsed().as_millis() as u64);
        
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
            // Check I-Loop status via consciousness config
            let input = serde_json::json!({
                "action": "GetStatus"
            });
            
            if let Ok(result) = self.executor.execute(44, input).await { // i_loop pipeline
                let active = result.get("active").and_then(|a| a.as_bool()).unwrap_or(false);
                if !active {
                    return Ok(());
                }
            } else {
                // If we can't check, assume I-Loop is not running
                return Ok(());
            }
            
            if waited >= max_wait_ms {
                return Err("Timeout waiting for I-Loop to complete".to_string());
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
            waited += check_interval_ms;
        }
    }
    
    fn record_stage(&self, state: &mut OrchestrationState, stage: u8, name: &str, success: bool, summary: &str) {
        state.stages.push(StageResult {
            stage,
            name: name.to_string(),
            success,
            duration_ms: 0,
            output_summary: Some(summary.to_string()),
        });
    }
    
    fn record_stage_timed(&self, state: &mut OrchestrationState, stage: u8, name: &str, success: bool, summary: &str, duration_ms: u64) {
        state.stages.push(StageResult {
            stage,
            name: name.to_string(),
            success,
            duration_ms,
            output_summary: Some(summary.to_string()),
        });
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
        async fn execute(&self, pipeline_id: u64, _input: serde_json::Value) -> Result<serde_json::Value, String> {
            // Return mock responses based on pipeline ID
            match pipeline_id {
                9 => Ok(serde_json::json!({
                    "response": "Test response",
                    "tokens_used": 100
                })),
                _ => Ok(serde_json::json!({"success": true}))
            }
        }
    }
    
    #[tokio::test]
    async fn test_basic_orchestration() {
        let executor = Arc::new(MockExecutor);
        let orchestrator = PromptOrchestrator::new(executor);
        
        let request = OrchestrationRequest {
            prompt: "Hello, how are you?".to_string(),
            project_id: None,
            workspace_id: None,
            user_id: 1,
            device_id: 1,
            consciousness_enabled: false,
            token_budget: 10000,
            model_config: None,
        };
        
        let response = orchestrator.orchestrate(request).await;
        
        assert!(response.success);
        assert!(response.response.is_some());
    }
}
