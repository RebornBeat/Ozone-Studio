//! TraversalMLPipeline - Pipeline #17
//! 
//! ML-guided traversal of ZSEI containers.
//! Per spec §16: ML Traversal System
//! 
//! Traversal Modes:
//! - Structural: Follow parent-child relationships
//! - Semantic: Follow embedding similarity
//! - Contextual: Follow usage patterns
//! - Hybrid: Combine multiple modes
//! - ML-Guided: Use trained model for path prediction

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;

// ========== Input Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TraversalInput {
    /// Structural traversal (parent-child)
    TraverseStructural {
        start_container: u64,
        max_depth: u16,
        max_results: u32,
        filters: Option<Vec<TraversalFilter>>,
    },
    /// Semantic traversal (embedding similarity)
    TraverseSemantic {
        start_container: u64,
        similarity_threshold: f32,
        max_results: u32,
    },
    /// Contextual traversal (usage patterns)
    TraverseContextual {
        start_container: u64,
        context_type: String,  // "code", "text", "methodology"
        max_results: u32,
    },
    /// Hybrid traversal (multiple modes)
    TraverseHybrid {
        start_container: u64,
        modes: Vec<String>,
        weights: Vec<f32>,
        max_results: u32,
    },
    /// ML-guided traversal
    TraverseMLGuided {
        query: String,
        modality: Option<String>,
        max_results: u32,
    },
    /// Find path between containers
    FindPath {
        source: u64,
        target: u64,
        max_hops: u16,
    },
    /// Get related containers
    GetRelated {
        container_id: u64,
        relation_types: Vec<String>,
        limit: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalFilter {
    pub field: String,
    pub operator: String,  // equals, contains, gt, lt
    pub value: serde_json::Value,
}

// ========== Output Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub container_id: u64,
    pub distance: f32,
    pub path: Vec<u64>,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalOutput {
    pub success: bool,
    pub results: Vec<TraversalResult>,
    pub total_visited: u32,
    pub error: Option<String>,
}

// ========== ZSEI Interface ==========

fn load_container(container_id: u64) -> Option<ContainerInfo> {
    let zsei_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
    let path = format!("{}/local/{}.json", zsei_path, container_id);
    
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
            return Some(ContainerInfo {
                container_id,
                parent_id: data.get("parent_id").and_then(|p| p.as_u64()),
                child_ids: data.get("child_ids")
                    .and_then(|c| c.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
                    .unwrap_or_default(),
                keywords: data.get("content")
                    .and_then(|c| c.get("keywords"))
                    .and_then(|k| k.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default(),
                modality: data.get("modality").and_then(|m| m.as_str()).unwrap_or("unknown").to_string(),
            });
        }
    }
    None
}

#[derive(Debug)]
struct ContainerInfo {
    container_id: u64,
    parent_id: Option<u64>,
    child_ids: Vec<u64>,
    keywords: Vec<String>,
    modality: String,
}

// ========== Traversal Functions ==========

fn traverse_structural(
    start: u64,
    max_depth: u16,
    max_results: u32,
    filters: &[TraversalFilter],
) -> Vec<TraversalResult> {
    let mut results = Vec::new();
    let mut visited: HashSet<u64> = HashSet::new();
    let mut queue: VecDeque<(u64, u16, Vec<u64>)> = VecDeque::new();
    
    queue.push_back((start, 0, vec![start]));
    visited.insert(start);
    
    while let Some((current, depth, path)) = queue.pop_front() {
        if results.len() >= max_results as usize {
            break;
        }
        
        if let Some(info) = load_container(current) {
            // Apply filters
            let passes_filters = filters.iter().all(|f| {
                match f.field.as_str() {
                    "modality" => {
                        if f.operator == "equals" {
                            f.value.as_str().map(|v| v == info.modality).unwrap_or(false)
                        } else { true }
                    }
                    "keywords" => {
                        if f.operator == "contains" {
                            f.value.as_str().map(|v| info.keywords.iter().any(|k| k.contains(v))).unwrap_or(false)
                        } else { true }
                    }
                    _ => true
                }
            });
            
            if passes_filters {
                results.push(TraversalResult {
                    container_id: current,
                    distance: depth as f32,
                    path: path.clone(),
                    relevance_score: 1.0 - (depth as f32 / max_depth as f32),
                });
            }
            
            // Add children to queue
            if depth < max_depth {
                for child_id in info.child_ids {
                    if !visited.contains(&child_id) {
                        visited.insert(child_id);
                        let mut new_path = path.clone();
                        new_path.push(child_id);
                        queue.push_back((child_id, depth + 1, new_path));
                    }
                }
            }
        }
    }
    
    results
}

fn traverse_semantic(
    start: u64,
    similarity_threshold: f32,
    max_results: u32,
) -> Vec<TraversalResult> {
    // In a full implementation, this would:
    // 1. Get embedding for start container
    // 2. Search embedding index for similar containers
    // 3. Return sorted by similarity
    
    // Simplified: Use keyword overlap as proxy for semantic similarity
    let mut results = Vec::new();
    
    if let Some(start_info) = load_container(start) {
        let zsei_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
        let index_path = format!("{}/indices/code_index.json", zsei_path);
        
        if let Ok(content) = std::fs::read_to_string(&index_path) {
            if let Ok(index) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(containers) = index.get("containers").and_then(|c| c.as_array()) {
                    for container in containers.iter().take(max_results as usize * 2) {
                        if let Some(id) = container.get("container_id").and_then(|c| c.as_u64()) {
                            if id == start { continue; }
                            
                            let keywords: Vec<String> = container.get("keywords")
                                .and_then(|k| k.as_array())
                                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                .unwrap_or_default();
                            
                            // Calculate keyword overlap
                            let overlap = start_info.keywords.iter()
                                .filter(|k| keywords.contains(k))
                                .count() as f32;
                            let max_keywords = start_info.keywords.len().max(keywords.len()).max(1) as f32;
                            let similarity = overlap / max_keywords;
                            
                            if similarity >= similarity_threshold {
                                results.push(TraversalResult {
                                    container_id: id,
                                    distance: 1.0 - similarity,
                                    path: vec![start, id],
                                    relevance_score: similarity,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(max_results as usize);
    
    results
}

fn find_path(source: u64, target: u64, max_hops: u16) -> Option<Vec<u64>> {
    let mut visited: HashSet<u64> = HashSet::new();
    let mut queue: VecDeque<(u64, Vec<u64>)> = VecDeque::new();
    
    queue.push_back((source, vec![source]));
    visited.insert(source);
    
    while let Some((current, path)) = queue.pop_front() {
        if current == target {
            return Some(path);
        }
        
        if path.len() > max_hops as usize {
            continue;
        }
        
        if let Some(info) = load_container(current) {
            // Check parent
            if let Some(parent_id) = info.parent_id {
                if !visited.contains(&parent_id) {
                    visited.insert(parent_id);
                    let mut new_path = path.clone();
                    new_path.push(parent_id);
                    queue.push_back((parent_id, new_path));
                }
            }
            
            // Check children
            for child_id in info.child_ids {
                if !visited.contains(&child_id) {
                    visited.insert(child_id);
                    let mut new_path = path.clone();
                    new_path.push(child_id);
                    queue.push_back((child_id, new_path));
                }
            }
        }
    }
    
    None
}

// ========== Main Execution ==========

pub async fn execute(input: TraversalInput) -> Result<TraversalOutput, String> {
    match input {
        TraversalInput::TraverseStructural { start_container, max_depth, max_results, filters } => {
            let filters = filters.unwrap_or_default();
            let results = traverse_structural(start_container, max_depth, max_results, &filters);
            
            Ok(TraversalOutput {
                success: true,
                results,
                total_visited: 0,
                error: None,
            })
        }
        
        TraversalInput::TraverseSemantic { start_container, similarity_threshold, max_results } => {
            let results = traverse_semantic(start_container, similarity_threshold, max_results);
            
            Ok(TraversalOutput {
                success: true,
                results,
                total_visited: 0,
                error: None,
            })
        }
        
        TraversalInput::TraverseContextual { start_container, context_type, max_results } => {
            // Filter by modality
            let filters = vec![TraversalFilter {
                field: "modality".to_string(),
                operator: "equals".to_string(),
                value: serde_json::json!(context_type),
            }];
            let results = traverse_structural(start_container, 5, max_results, &filters);
            
            Ok(TraversalOutput {
                success: true,
                results,
                total_visited: 0,
                error: None,
            })
        }
        
        TraversalInput::TraverseHybrid { start_container, modes, weights, max_results } => {
            let mut all_results: HashMap<u64, TraversalResult> = HashMap::new();
            
            for (mode, weight) in modes.iter().zip(weights.iter()) {
                let results = match mode.as_str() {
                    "structural" => traverse_structural(start_container, 5, max_results, &[]),
                    "semantic" => traverse_semantic(start_container, 0.3, max_results),
                    _ => vec![],
                };
                
                for mut r in results {
                    r.relevance_score *= weight;
                    all_results.entry(r.container_id)
                        .and_modify(|e| e.relevance_score += r.relevance_score)
                        .or_insert(r);
                }
            }
            
            let mut results: Vec<_> = all_results.into_values().collect();
            results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(max_results as usize);
            
            Ok(TraversalOutput {
                success: true,
                results,
                total_visited: 0,
                error: None,
            })
        }
        
        TraversalInput::TraverseMLGuided { query, modality, max_results } => {
            // ML-guided uses keyword matching as simplified implementation
            let keywords: Vec<&str> = query.split_whitespace().collect();
            let zsei_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
            let index_path = format!("{}/indices/code_index.json", zsei_path);
            
            let mut results = Vec::new();
            
            if let Ok(content) = std::fs::read_to_string(&index_path) {
                if let Ok(index) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(containers) = index.get("containers").and_then(|c| c.as_array()) {
                        for container in containers {
                            if let Some(id) = container.get("container_id").and_then(|c| c.as_u64()) {
                                let container_keywords: Vec<String> = container.get("keywords")
                                    .and_then(|k| k.as_array())
                                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_lowercase())).collect())
                                    .unwrap_or_default();
                                
                                let matches = keywords.iter()
                                    .filter(|k| container_keywords.iter().any(|ck| ck.contains(&k.to_lowercase())))
                                    .count();
                                
                                if matches > 0 {
                                    results.push(TraversalResult {
                                        container_id: id,
                                        distance: 0.0,
                                        path: vec![id],
                                        relevance_score: matches as f32 / keywords.len().max(1) as f32,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            
            results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(max_results as usize);
            
            Ok(TraversalOutput {
                success: true,
                results,
                total_visited: 0,
                error: None,
            })
        }
        
        TraversalInput::FindPath { source, target, max_hops } => {
            if let Some(path) = find_path(source, target, max_hops) {
                Ok(TraversalOutput {
                    success: true,
                    results: vec![TraversalResult {
                        container_id: target,
                        distance: path.len() as f32 - 1.0,
                        path,
                        relevance_score: 1.0,
                    }],
                    total_visited: 0,
                    error: None,
                })
            } else {
                Ok(TraversalOutput {
                    success: false,
                    results: vec![],
                    total_visited: 0,
                    error: Some("No path found".to_string()),
                })
            }
        }
        
        TraversalInput::GetRelated { container_id, relation_types, limit } => {
            let mut results = Vec::new();
            
            if let Some(info) = load_container(container_id) {
                // Get parent if requested
                if relation_types.contains(&"parent".to_string()) {
                    if let Some(parent_id) = info.parent_id {
                        results.push(TraversalResult {
                            container_id: parent_id,
                            distance: 1.0,
                            path: vec![container_id, parent_id],
                            relevance_score: 1.0,
                        });
                    }
                }
                
                // Get children if requested
                if relation_types.contains(&"child".to_string()) {
                    for child_id in info.child_ids.iter().take(limit as usize) {
                        results.push(TraversalResult {
                            container_id: *child_id,
                            distance: 1.0,
                            path: vec![container_id, *child_id],
                            relevance_score: 1.0,
                        });
                    }
                }
                
                // Get similar if requested
                if relation_types.contains(&"similar".to_string()) {
                    let similar = traverse_semantic(container_id, 0.5, limit);
                    results.extend(similar);
                }
            }
            
            results.truncate(limit as usize);
            
            Ok(TraversalOutput {
                success: true,
                results,
                total_visited: 0,
                error: None,
            })
        }
    }
}

// ========== CLI Entry Point ==========

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    
    let input: TraversalInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => {
            println!("{}", serde_json::json!({"success": false, "error": e}));
            std::process::exit(1);
        }
    }
}
