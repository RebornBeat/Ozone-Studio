//! ZSEI Semantic Hooks
//!
//! This module implements the semantic enrichment layer that runs on top of
//! structural graphs created by modality pipelines. The hooks use LLM capabilities
//! to add semantic understanding to deterministic structural analysis.
//!
//! # Two-Layer Architecture
//!
//! ```text
//! Layer 1: STRUCTURAL (Modality Pipeline)
//! ├── Deterministic analysis (AST, parsing, detection)
//! ├── Creates base graph with structural edges
//! └── Fast, reproducible, cacheable
//!
//! Layer 2: SEMANTIC (ZSEI Hooks)
//! ├── LLM-powered understanding
//! ├── Relationship inference
//! ├── Context and meaning extraction
//! └── Enriches graph with semantic edges
//! ```
//!
//! # Hook Types
//!
//! - `OnGraphCreated`: Triggered when a new graph is created
//! - `OnInferRelationships`: Triggered to infer relationships between nodes
//! - `OnEdgeCompletion`: Triggered when edges need semantic completion
//! - `OnCrossModalityLink`: Triggered when linking graphs across modalities

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::core::{OzoneResult, OzoneError};
use crate::modalities::{
    ModalityGraph, ModalityType, GraphNode, GraphEdge, EdgeType, 
    NodeId, GraphId, QueryResult
};

// =============================================================================
// Hook Types
// =============================================================================

/// Types of ZSEI semantic hooks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZSEIHook {
    /// Triggered when a new graph is created by a modality pipeline
    OnGraphCreated,
    
    /// Triggered to infer semantic relationships between node pairs
    OnInferRelationships,
    
    /// Triggered when edges have incomplete semantic information
    OnEdgeCompletion,
    
    /// Triggered when linking graphs from different modalities
    OnCrossModalityLink,
}

/// Result of hook processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookResult {
    /// Graph was enriched with new semantic edges
    GraphEnriched {
        edges_added: usize,
        nodes_annotated: usize,
    },
    
    /// Relationships were inferred between nodes
    RelationshipsInferred {
        relationships: Vec<InferredRelationship>,
    },
    
    /// Cross-modality links were created
    CrossModalityLinked {
        links_created: usize,
        source_modality: ModalityType,
        target_modality: ModalityType,
    },
    
    /// No action was taken
    NoAction {
        reason: String,
    },
    
    /// Hook processing failed
    Failed {
        error: String,
    },
}

/// An inferred semantic relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredRelationship {
    pub from_node: NodeId,
    pub to_node: NodeId,
    pub relationship_type: EdgeType,
    pub confidence: f32,
    pub reasoning: String,
}

// =============================================================================
// Hook Configuration
// =============================================================================

/// Configuration for hook processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Processing mode
    pub mode: HookMode,
    
    /// LLM endpoint for semantic analysis
    pub llm_endpoint: String,
    
    /// Model to use for semantic analysis
    pub model: String,
    
    /// Maximum tokens for LLM responses
    pub max_tokens: usize,
    
    /// Temperature for LLM
    pub temperature: f32,
    
    /// Whether to cache semantic analysis results
    pub enable_cache: bool,
    
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

/// Processing mode for hooks
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HookMode {
    /// Process hooks immediately and synchronously
    Synchronous,
    
    /// Queue hooks for background processing
    Asynchronous,
    
    /// Batch multiple hooks for efficiency
    Batched {
        batch_size: usize,
        flush_interval_ms: u64,
    },
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            mode: HookMode::Asynchronous,
            llm_endpoint: "http://localhost:8080/v1/chat/completions".to_string(),
            model: "claude-3-sonnet".to_string(),
            max_tokens: 4096,
            temperature: 0.3,
            enable_cache: true,
            cache_ttl_seconds: 3600,
        }
    }
}

// =============================================================================
// Pending Hook
// =============================================================================

/// A hook waiting to be processed
#[derive(Debug, Clone)]
pub struct PendingHook {
    pub id: String,
    pub hook_type: ZSEIHook,
    pub graph_id: GraphId,
    pub modality: ModalityType,
    pub params: HashMap<String, serde_json::Value>,
    pub priority: u8,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// =============================================================================
// Hook Processor
// =============================================================================

/// Processes ZSEI semantic hooks
pub struct ZSEIHookProcessor {
    config: HookConfig,
    
    /// Pending hooks queue (for async/batched modes)
    pending_hooks: RwLock<Vec<PendingHook>>,
    
    /// Sender for async processing
    hook_sender: Option<mpsc::Sender<PendingHook>>,
    
    /// Cache for semantic analysis results
    cache: RwLock<HashMap<String, CachedAnalysis>>,
    
    /// LLM client for semantic analysis
    llm_client: Arc<dyn LLMClient>,
}

/// Cached semantic analysis
#[derive(Debug, Clone)]
struct CachedAnalysis {
    result: HookResult,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Trait for LLM interaction
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn complete(&self, prompt: &str, config: &LLMRequestConfig) -> OzoneResult<String>;
}

#[derive(Debug, Clone)]
pub struct LLMRequestConfig {
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub response_format: Option<String>,
}

impl ZSEIHookProcessor {
    /// Create a new hook processor
    pub fn new(config: HookConfig, llm_client: Arc<dyn LLMClient>) -> Self {
        Self {
            config,
            pending_hooks: RwLock::new(Vec::new()),
            hook_sender: None,
            cache: RwLock::new(HashMap::new()),
            llm_client,
        }
    }
    
    /// Start the async processing loop (for async/batched modes)
    pub async fn start_processing_loop(&mut self) -> OzoneResult<()> {
        match self.config.mode {
            HookMode::Synchronous => {
                // No background loop needed
                Ok(())
            }
            HookMode::Asynchronous => {
                let (tx, rx) = mpsc::channel(100);
                self.hook_sender = Some(tx);
                self.spawn_async_processor(rx).await;
                Ok(())
            }
            HookMode::Batched { batch_size, flush_interval_ms } => {
                let (tx, rx) = mpsc::channel(100);
                self.hook_sender = Some(tx);
                self.spawn_batch_processor(rx, batch_size, flush_interval_ms).await;
                Ok(())
            }
        }
    }
    
    async fn spawn_async_processor(&self, mut rx: mpsc::Receiver<PendingHook>) {
        let llm_client = Arc::clone(&self.llm_client);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            while let Some(hook) = rx.recv().await {
                // Process each hook as it arrives
                if let Err(e) = Self::process_single_hook(&hook, &llm_client, &config).await {
                    tracing::error!("Failed to process hook {:?}: {}", hook.hook_type, e);
                }
            }
        });
    }
    
    async fn spawn_batch_processor(
        &self, 
        mut rx: mpsc::Receiver<PendingHook>,
        batch_size: usize,
        flush_interval_ms: u64,
    ) {
        let llm_client = Arc::clone(&self.llm_client);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(batch_size);
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_millis(flush_interval_ms)
            );
            
            loop {
                tokio::select! {
                    Some(hook) = rx.recv() => {
                        batch.push(hook);
                        if batch.len() >= batch_size {
                            Self::process_batch(&batch, &llm_client, &config).await;
                            batch.clear();
                        }
                    }
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            Self::process_batch(&batch, &llm_client, &config).await;
                            batch.clear();
                        }
                    }
                }
            }
        });
    }
    
    // =========================================================================
    // Public Hook Triggers
    // =========================================================================
    
    /// Trigger OnGraphCreated hook
    pub async fn on_graph_created(
        &self,
        graph: &mut ModalityGraph,
    ) -> OzoneResult<HookResult> {
        let hook = PendingHook {
            id: uuid::Uuid::new_v4().to_string(),
            hook_type: ZSEIHook::OnGraphCreated,
            graph_id: graph.id.clone(),
            modality: graph.modality,
            params: HashMap::new(),
            priority: 1,
            created_at: chrono::Utc::now(),
        };
        
        match self.config.mode {
            HookMode::Synchronous => {
                self.process_graph_created(graph).await
            }
            _ => {
                if let Some(sender) = &self.hook_sender {
                    sender.send(hook).await.map_err(|e| {
                        OzoneError::Hook(format!("Failed to queue hook: {}", e))
                    })?;
                }
                Ok(HookResult::NoAction { 
                    reason: "Queued for async processing".to_string() 
                })
            }
        }
    }
    
    /// Trigger OnInferRelationships hook
    pub async fn on_infer_relationships(
        &self,
        graph: &mut ModalityGraph,
        node_pairs: &[(NodeId, NodeId)],
    ) -> OzoneResult<HookResult> {
        let mut params = HashMap::new();
        params.insert(
            "node_pairs".to_string(),
            serde_json::to_value(node_pairs).unwrap_or_default()
        );
        
        let hook = PendingHook {
            id: uuid::Uuid::new_v4().to_string(),
            hook_type: ZSEIHook::OnInferRelationships,
            graph_id: graph.id.clone(),
            modality: graph.modality,
            params,
            priority: 2,
            created_at: chrono::Utc::now(),
        };
        
        match self.config.mode {
            HookMode::Synchronous => {
                self.process_infer_relationships(graph, node_pairs).await
            }
            _ => {
                if let Some(sender) = &self.hook_sender {
                    sender.send(hook).await.map_err(|e| {
                        OzoneError::Hook(format!("Failed to queue hook: {}", e))
                    })?;
                }
                Ok(HookResult::NoAction { 
                    reason: "Queued for async processing".to_string() 
                })
            }
        }
    }
    
    /// Trigger OnCrossModalityLink hook
    pub async fn on_cross_modality_link(
        &self,
        source_graph: &mut ModalityGraph,
        target_graph: &ModalityGraph,
    ) -> OzoneResult<HookResult> {
        let mut params = HashMap::new();
        params.insert(
            "target_graph_id".to_string(),
            serde_json::Value::String(target_graph.id.0.clone())
        );
        params.insert(
            "target_modality".to_string(),
            serde_json::to_value(target_graph.modality).unwrap_or_default()
        );
        
        let hook = PendingHook {
            id: uuid::Uuid::new_v4().to_string(),
            hook_type: ZSEIHook::OnCrossModalityLink,
            graph_id: source_graph.id.clone(),
            modality: source_graph.modality,
            params,
            priority: 3,
            created_at: chrono::Utc::now(),
        };
        
        match self.config.mode {
            HookMode::Synchronous => {
                self.process_cross_modality_link(source_graph, target_graph).await
            }
            _ => {
                if let Some(sender) = &self.hook_sender {
                    sender.send(hook).await.map_err(|e| {
                        OzoneError::Hook(format!("Failed to queue hook: {}", e))
                    })?;
                }
                Ok(HookResult::NoAction { 
                    reason: "Queued for async processing".to_string() 
                })
            }
        }
    }
    
    // =========================================================================
    // Hook Processing Implementation
    // =========================================================================
    
    async fn process_single_hook(
        hook: &PendingHook,
        llm_client: &Arc<dyn LLMClient>,
        config: &HookConfig,
    ) -> OzoneResult<HookResult> {
        tracing::debug!("Processing hook: {:?} for graph {:?}", hook.hook_type, hook.graph_id);
        
        // This would load the graph from storage and process
        // For now, return placeholder
        Ok(HookResult::NoAction { 
            reason: "Hook processing requires storage access".to_string() 
        })
    }
    
    async fn process_batch(
        hooks: &[PendingHook],
        llm_client: &Arc<dyn LLMClient>,
        config: &HookConfig,
    ) {
        tracing::debug!("Processing batch of {} hooks", hooks.len());
        
        // Group hooks by graph for efficiency
        let mut by_graph: HashMap<&GraphId, Vec<&PendingHook>> = HashMap::new();
        for hook in hooks {
            by_graph.entry(&hook.graph_id).or_default().push(hook);
        }
        
        // Process each graph's hooks together
        for (graph_id, graph_hooks) in by_graph {
            if let Err(e) = Self::process_graph_hooks(graph_id, &graph_hooks, llm_client, config).await {
                tracing::error!("Failed to process hooks for graph {:?}: {}", graph_id, e);
            }
        }
    }
    
    async fn process_graph_hooks(
        graph_id: &GraphId,
        hooks: &[&PendingHook],
        llm_client: &Arc<dyn LLMClient>,
        config: &HookConfig,
    ) -> OzoneResult<()> {
        // Load graph, process all hooks, save once
        // This is a placeholder for actual implementation
        Ok(())
    }
    
    /// Process OnGraphCreated: Add semantic annotations to new graph
    async fn process_graph_created(
        &self,
        graph: &mut ModalityGraph,
    ) -> OzoneResult<HookResult> {
        let prompt = self.build_semantic_analysis_prompt(graph);
        
        let llm_config = LLMRequestConfig {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            response_format: Some("json".to_string()),
        };
        
        let response = self.llm_client.complete(&prompt, &llm_config).await?;
        
        // Parse LLM response and add semantic edges
        let semantic_edges = self.parse_semantic_response(&response, graph)?;
        
        // Add edges to graph
        let edges_added = semantic_edges.len();
        for edge in semantic_edges {
            graph.edges.push(edge);
        }
        
        // Annotate nodes with semantic metadata
        let nodes_annotated = self.annotate_nodes_semantically(graph, &response)?;
        
        graph.updated_at = chrono::Utc::now();
        
        Ok(HookResult::GraphEnriched {
            edges_added,
            nodes_annotated,
        })
    }
    
    /// Process OnInferRelationships: Infer semantic relationships between node pairs
    async fn process_infer_relationships(
        &self,
        graph: &mut ModalityGraph,
        node_pairs: &[(NodeId, NodeId)],
    ) -> OzoneResult<HookResult> {
        let prompt = self.build_relationship_inference_prompt(graph, node_pairs);
        
        let llm_config = LLMRequestConfig {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            response_format: Some("json".to_string()),
        };
        
        let response = self.llm_client.complete(&prompt, &llm_config).await?;
        
        // Parse relationships from response
        let relationships = self.parse_relationship_response(&response, node_pairs)?;
        
        // Add inferred edges to graph
        for rel in &relationships {
            if rel.confidence >= 0.7 {
                graph.edges.push(GraphEdge {
                    from_node: rel.from_node.clone(),
                    to_node: rel.to_node.clone(),
                    edge_type: rel.relationship_type,
                    weight: rel.confidence,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert(
                            "reasoning".to_string(),
                            serde_json::Value::String(rel.reasoning.clone())
                        );
                        meta.insert(
                            "source".to_string(),
                            serde_json::Value::String("zsei_inference".to_string())
                        );
                        meta
                    },
                });
            }
        }
        
        graph.updated_at = chrono::Utc::now();
        
        Ok(HookResult::RelationshipsInferred { relationships })
    }
    
    /// Process OnCrossModalityLink: Create semantic links between different modality graphs
    async fn process_cross_modality_link(
        &self,
        source_graph: &mut ModalityGraph,
        target_graph: &ModalityGraph,
    ) -> OzoneResult<HookResult> {
        let prompt = self.build_cross_modality_prompt(source_graph, target_graph);
        
        let llm_config = LLMRequestConfig {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            response_format: Some("json".to_string()),
        };
        
        let response = self.llm_client.complete(&prompt, &llm_config).await?;
        
        // Parse cross-modality links
        let links = self.parse_cross_modality_response(&response, source_graph, target_graph)?;
        
        // Add links to source graph
        let links_created = links.len();
        for link in links {
            source_graph.edges.push(link);
        }
        
        source_graph.updated_at = chrono::Utc::now();
        
        Ok(HookResult::CrossModalityLinked {
            links_created,
            source_modality: source_graph.modality,
            target_modality: target_graph.modality,
        })
    }
    
    // =========================================================================
    // Prompt Building
    // =========================================================================
    
    fn build_semantic_analysis_prompt(&self, graph: &ModalityGraph) -> String {
        let modality_context = match graph.modality {
            ModalityType::Code => "This is a code graph with functions, classes, and imports.",
            ModalityType::Text => "This is a text graph with entities, topics, and sections.",
            ModalityType::Math => "This is a math graph with expressions, variables, and proofs.",
            _ => "This is a structural graph from analysis.",
        };
        
        // Build node summary
        let node_summary: Vec<String> = graph.nodes.iter()
            .take(50) // Limit for prompt size
            .map(|(id, node)| {
                format!("Node {}: type={}, content={}", 
                    id.0, 
                    node.node_type,
                    serde_json::to_string(&node.content).unwrap_or_default()
                        .chars().take(100).collect::<String>()
                )
            })
            .collect();
        
        format!(r#"
Analyze the following structural graph and identify semantic relationships.

Modality: {:?}
Context: {}

Nodes (sample):
{}

Existing structural edges: {} edges

Task: Identify semantic relationships between nodes that are NOT captured by structural analysis.
For example:
- RelatesTo: Conceptually related but not structurally connected
- Supports: One node provides evidence/support for another
- Contradicts: Nodes express conflicting information
- Extends: One node extends/elaborates on another
- SimilarTo: Nodes are semantically similar

Respond with JSON:
{{
  "semantic_edges": [
    {{
      "from_node": <node_id>,
      "to_node": <node_id>,
      "edge_type": "<RelatesTo|Supports|Contradicts|Extends|SimilarTo>",
      "confidence": <0.0-1.0>,
      "reasoning": "<why this relationship exists>"
    }}
  ],
  "node_annotations": [
    {{
      "node_id": <node_id>,
      "semantic_role": "<description>",
      "importance": <0.0-1.0>
    }}
  ]
}}
"#, 
            graph.modality,
            modality_context,
            node_summary.join("\n"),
            graph.edges.len()
        )
    }
    
    fn build_relationship_inference_prompt(
        &self, 
        graph: &ModalityGraph,
        node_pairs: &[(NodeId, NodeId)],
    ) -> String {
        let pairs_detail: Vec<String> = node_pairs.iter()
            .filter_map(|(from_id, to_id)| {
                let from_node = graph.nodes.get(from_id)?;
                let to_node = graph.nodes.get(to_id)?;
                Some(format!(
                    "Pair: Node {} ({}) <-> Node {} ({})",
                    from_id.0, from_node.node_type,
                    to_id.0, to_node.node_type
                ))
            })
            .collect();
        
        format!(r#"
Infer semantic relationships between the following node pairs.

Graph modality: {:?}

Node pairs to analyze:
{}

For each pair, determine:
1. What semantic relationship exists (if any)
2. Confidence level (0.0-1.0)
3. Reasoning

Respond with JSON:
{{
  "relationships": [
    {{
      "from_node": <node_id>,
      "to_node": <node_id>,
      "relationship_type": "<RelatesTo|Supports|Contradicts|Extends|Specializes|Generalizes|SimilarTo|None>",
      "confidence": <0.0-1.0>,
      "reasoning": "<explanation>"
    }}
  ]
}}
"#,
            graph.modality,
            pairs_detail.join("\n")
        )
    }
    
    fn build_cross_modality_prompt(
        &self,
        source_graph: &ModalityGraph,
        target_graph: &ModalityGraph,
    ) -> String {
        format!(r#"
Identify semantic links between two graphs of different modalities.

Source Graph:
- Modality: {:?}
- Nodes: {} nodes
- Key node types: {}

Target Graph:
- Modality: {:?}
- Nodes: {} nodes
- Key node types: {}

Find nodes in the source that:
- Describe nodes in the target
- Implement concepts from the target
- Represent the same information
- Document or explain the target

Respond with JSON:
{{
  "cross_links": [
    {{
      "source_node": <node_id>,
      "target_node": <node_id>,
      "link_type": "<Describes|Implements|Represents|Documents|Illustrates>",
      "confidence": <0.0-1.0>,
      "reasoning": "<explanation>"
    }}
  ]
}}
"#,
            source_graph.modality,
            source_graph.nodes.len(),
            self.get_node_type_summary(source_graph),
            target_graph.modality,
            target_graph.nodes.len(),
            self.get_node_type_summary(target_graph)
        )
    }
    
    fn get_node_type_summary(&self, graph: &ModalityGraph) -> String {
        let mut type_counts: HashMap<&str, usize> = HashMap::new();
        for node in graph.nodes.values() {
            *type_counts.entry(&node.node_type).or_default() += 1;
        }
        
        let mut types: Vec<_> = type_counts.into_iter().collect();
        types.sort_by(|a, b| b.1.cmp(&a.1));
        types.into_iter()
            .take(5)
            .map(|(t, c)| format!("{}({})", t, c))
            .collect::<Vec<_>>()
            .join(", ")
    }
    
    // =========================================================================
    // Response Parsing
    // =========================================================================
    
    fn parse_semantic_response(
        &self,
        response: &str,
        graph: &ModalityGraph,
    ) -> OzoneResult<Vec<GraphEdge>> {
        let parsed: serde_json::Value = serde_json::from_str(response)
            .map_err(|e| OzoneError::Hook(format!("Failed to parse LLM response: {}", e)))?;
        
        let mut edges = Vec::new();
        
        if let Some(semantic_edges) = parsed.get("semantic_edges").and_then(|v| v.as_array()) {
            for edge_val in semantic_edges {
                let from_id = edge_val.get("from_node")
                    .and_then(|v| v.as_u64())
                    .map(NodeId);
                let to_id = edge_val.get("to_node")
                    .and_then(|v| v.as_u64())
                    .map(NodeId);
                let edge_type_str = edge_val.get("edge_type")
                    .and_then(|v| v.as_str());
                let confidence = edge_val.get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32;
                let reasoning = edge_val.get("reasoning")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                if let (Some(from), Some(to), Some(edge_type_str)) = (from_id, to_id, edge_type_str) {
                    let edge_type = match edge_type_str {
                        "RelatesTo" => EdgeType::RelatesTo,
                        "Supports" => EdgeType::Supports,
                        "Contradicts" => EdgeType::Contradicts,
                        "Extends" => EdgeType::Extends,
                        "SimilarTo" => EdgeType::SimilarTo,
                        "Specializes" => EdgeType::Specializes,
                        "Generalizes" => EdgeType::Generalizes,
                        _ => continue,
                    };
                    
                    edges.push(GraphEdge {
                        from_node: from,
                        to_node: to,
                        edge_type,
                        weight: confidence,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert(
                                "reasoning".to_string(),
                                serde_json::Value::String(reasoning.to_string())
                            );
                            meta.insert(
                                "source".to_string(),
                                serde_json::Value::String("zsei_semantic".to_string())
                            );
                            meta
                        },
                    });
                }
            }
        }
        
        Ok(edges)
    }
    
    fn annotate_nodes_semantically(
        &self,
        graph: &mut ModalityGraph,
        response: &str,
    ) -> OzoneResult<usize> {
        let parsed: serde_json::Value = serde_json::from_str(response)
            .map_err(|e| OzoneError::Hook(format!("Failed to parse LLM response: {}", e)))?;
        
        let mut annotated = 0;
        
        if let Some(annotations) = parsed.get("node_annotations").and_then(|v| v.as_array()) {
            for ann in annotations {
                let node_id = ann.get("node_id")
                    .and_then(|v| v.as_u64())
                    .map(NodeId);
                
                if let Some(id) = node_id {
                    if let Some(node) = graph.nodes.get_mut(&id) {
                        if let Some(role) = ann.get("semantic_role") {
                            node.metadata.insert("semantic_role".to_string(), role.clone());
                        }
                        if let Some(importance) = ann.get("importance") {
                            node.metadata.insert("importance".to_string(), importance.clone());
                        }
                        annotated += 1;
                    }
                }
            }
        }
        
        Ok(annotated)
    }
    
    fn parse_relationship_response(
        &self,
        response: &str,
        node_pairs: &[(NodeId, NodeId)],
    ) -> OzoneResult<Vec<InferredRelationship>> {
        let parsed: serde_json::Value = serde_json::from_str(response)
            .map_err(|e| OzoneError::Hook(format!("Failed to parse LLM response: {}", e)))?;
        
        let mut relationships = Vec::new();
        
        if let Some(rels) = parsed.get("relationships").and_then(|v| v.as_array()) {
            for rel in rels {
                let from_id = rel.get("from_node")
                    .and_then(|v| v.as_u64())
                    .map(NodeId);
                let to_id = rel.get("to_node")
                    .and_then(|v| v.as_u64())
                    .map(NodeId);
                let rel_type_str = rel.get("relationship_type")
                    .and_then(|v| v.as_str());
                let confidence = rel.get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32;
                let reasoning = rel.get("reasoning")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                if let (Some(from), Some(to), Some(rel_type_str)) = (from_id, to_id, rel_type_str) {
                    if rel_type_str == "None" {
                        continue;
                    }
                    
                    let relationship_type = match rel_type_str {
                        "RelatesTo" => EdgeType::RelatesTo,
                        "Supports" => EdgeType::Supports,
                        "Contradicts" => EdgeType::Contradicts,
                        "Extends" => EdgeType::Extends,
                        "Specializes" => EdgeType::Specializes,
                        "Generalizes" => EdgeType::Generalizes,
                        "SimilarTo" => EdgeType::SimilarTo,
                        _ => continue,
                    };
                    
                    relationships.push(InferredRelationship {
                        from_node: from,
                        to_node: to,
                        relationship_type,
                        confidence,
                        reasoning,
                    });
                }
            }
        }
        
        Ok(relationships)
    }
    
    fn parse_cross_modality_response(
        &self,
        response: &str,
        source_graph: &ModalityGraph,
        target_graph: &ModalityGraph,
    ) -> OzoneResult<Vec<GraphEdge>> {
        let parsed: serde_json::Value = serde_json::from_str(response)
            .map_err(|e| OzoneError::Hook(format!("Failed to parse LLM response: {}", e)))?;
        
        let mut edges = Vec::new();
        
        if let Some(links) = parsed.get("cross_links").and_then(|v| v.as_array()) {
            for link in links {
                let source_id = link.get("source_node")
                    .and_then(|v| v.as_u64())
                    .map(NodeId);
                let target_id = link.get("target_node")
                    .and_then(|v| v.as_u64())
                    .map(NodeId);
                let link_type_str = link.get("link_type")
                    .and_then(|v| v.as_str());
                let confidence = link.get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5) as f32;
                let reasoning = link.get("reasoning")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                if let (Some(source), Some(target), Some(link_type_str)) = (source_id, target_id, link_type_str) {
                    let edge_type = match link_type_str {
                        "Describes" => EdgeType::Describes,
                        "Implements" => EdgeType::Implements,
                        "Represents" => EdgeType::Represents,
                        "Documents" => EdgeType::Documents,
                        "Illustrates" => EdgeType::Illustrates,
                        _ => continue,
                    };
                    
                    edges.push(GraphEdge {
                        from_node: source,
                        to_node: target,
                        edge_type,
                        weight: confidence,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert(
                                "target_graph".to_string(),
                                serde_json::Value::String(target_graph.id.0.clone())
                            );
                            meta.insert(
                                "target_modality".to_string(),
                                serde_json::to_value(target_graph.modality).unwrap_or_default()
                            );
                            meta.insert(
                                "reasoning".to_string(),
                                serde_json::Value::String(reasoning.to_string())
                            );
                            meta
                        },
                    });
                }
            }
        }
        
        Ok(edges)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hook_config_default() {
        let config = HookConfig::default();
        assert!(matches!(config.mode, HookMode::Asynchronous));
        assert!(config.enable_cache);
    }
    
    #[test]
    fn test_edge_type_classification() {
        assert!(EdgeType::RelatesTo.is_semantic());
        assert!(EdgeType::Describes.is_cross_modality());
        assert!(!EdgeType::RelatesTo.is_structural());
    }
}
