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

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// The keyword model core (tokens, match classes, v0 rank key, hashed n-gram encoder, ranker, promotion, registry) — the SAME file the host
// search door uses for inference (shared/contracts/keyword_model.rs). This pipeline is where TRAINING and EVALUATION run (taxonomy: the model is
// Ozone's own computation, so it is a pipeline, not an MCP — docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §6.4).
#[path = "../../../../shared/contracts/keyword_model.rs"]
#[allow(dead_code)]
mod keyword_model;

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
        
        TraversalInput::TraverseSemantic { .. } => {
            // 2026-11-08: the former body was a keyword-overlap stub reading a `indices/code_index.json` that nothing writes — it returned an
            // EMPTY success, which looked like "no similar containers". Semantic ranking is now done by the active encoders over candidates the
            // HOST supplies (`RankCandidates`); there is no pipeline-side index to scan. Honest refusal instead of a fake result.
            Ok(TraversalOutput {
                success: false,
                results: vec![],
                total_visited: 0,
                error: Some("TraverseSemantic has no pipeline-side index: use the RankCandidates action with host-supplied candidates (active encoders: see ModelStatus)".to_string()),
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
            let mut unavailable_modes: Vec<String> = Vec::new();
            
            for (mode, weight) in modes.iter().zip(weights.iter()) {
                let results = match mode.as_str() {
                    "structural" => traverse_structural(start_container, 5, max_results, &[]),
                    // the keyword-overlap "semantic" stub was removed 2026-11-08: reported as unavailable, never faked
                    other => {
                        unavailable_modes.push(other.to_string());
                        vec![]
                    }
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
                error: if unavailable_modes.is_empty() {
                    None
                } else {
                    Some(format!("hybrid mode(s) not available from this pipeline: {} (semantic similarity = RankCandidates over host-supplied candidates)", unavailable_modes.join(", ")))
                },
            })
        }
        
        TraversalInput::TraverseMLGuided { .. } => {
            // 2026-11-08: the former body was keyword matching over the same non-existent index, presented as "ML-guided". The real ML path is
            // `RankCandidates` (fused v0 + active encoders + learned ranker) and `TrainModel`; this mode refuses honestly.
            Ok(TraversalOutput {
                success: false,
                results: vec![],
                total_visited: 0,
                error: Some("TraverseMLGuided is replaced by RankCandidates (inference) and TrainModel (training); no pipeline-side index exists".to_string()),
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
            let mut similar_unavailable = false;
            
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
                
                // "similar" has no pipeline-side index (the keyword-overlap stub was removed 2026-11-08); similarity is the active
                // encoders' job over host-supplied candidates (RankCandidates) — reported below, never faked.
                if relation_types.contains(&"similar".to_string()) {
                    similar_unavailable = true;
                }
            }
            
            results.truncate(limit as usize);
            
            Ok(TraversalOutput {
                success: true,
                results,
                total_visited: 0,
                error: if similar_unavailable {
                    Some("relation 'similar' is not available from this pipeline: use RankCandidates with host-supplied candidates".to_string())
                } else {
                    None
                },
            })
        }
    }
}

// ========== KEYWORD MODEL ACTIONS (2026-11-08) ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankCandidateInput {
    pub id: u64,
    pub terms: Vec<String>,
    pub facts: keyword_model::RankFacts,
}

/// Actions of the real model pipeline. A SEPARATE tagged enum so the legacy traversal actions keep parsing unchanged; `main` tries this
/// one first.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ModelAction {
    /// Train the hashed n-gram encoder + ranker on a dataset the host built from the real hit log, evaluate the fused system on its
    /// held-out groups against the deterministic v0 ordering, and register the result (active only if the held-out sign test says yes).
    TrainModel {
        dataset_path: String,
        models_dir: String,
        #[serde(default)]
        params: Option<keyword_model::TrainParams>,
        #[serde(default)]
        now: Option<u64>,
    },
    /// The registry: every model, its kind, status and MEASURED evaluation; which encoders/ranker are active.
    ModelStatus { models_dir: String },
    /// Encode a text with every active, runnable encoder (diagnostics).
    EncodeText { models_dir: String, text: String },
    /// INFERENCE: order host-supplied candidates with v0 + every available active encoder + the active ranker, fused by reciprocal rank.
    RankCandidates { models_dir: String, query_terms: Vec<String>, candidates: Vec<RankCandidateInput> },
    /// PREPARED (not runnable yet): train an embedder at a data scale where that is meaningful.
    TrainEmbedder {
        #[serde(default)]
        spec: Option<serde_json::Value>,
    },
    /// PREPARED (not runnable yet): fine-tune a small pretrained sentence encoder on Ozone's hit pairs.
    FineTunePretrained {
        #[serde(default)]
        spec: Option<serde_json::Value>,
    },
}

fn run_model_action(action: ModelAction) -> serde_json::Value {
    use std::path::Path;
    match action {
        ModelAction::TrainModel { dataset_path, models_dir, params, now } => {
            let groups = match keyword_model::read_dataset(Path::new(&dataset_path)) {
                Ok(g) => g,
                Err(e) => return serde_json::json!({"success": false, "error": e}),
            };
            let now = now.unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
            match keyword_model::train_and_evaluate(&groups, Path::new(&models_dir), now, &params.unwrap_or_default()) {
                Ok(report) => serde_json::json!({"success": true, "report": report}),
                Err(e) => serde_json::json!({"success": false, "error": e}),
            }
        }
        ModelAction::ModelStatus { models_dir } => {
            let mut reg = keyword_model::ModelRegistry::load(Path::new(&models_dir));
            reg.ensure_prepared_slots();
            let active: Vec<String> = reg.models.iter().filter(|m| m.status == "active").map(|m| m.id.clone()).collect();
            serde_json::json!({"success": true, "registry": reg, "active": active})
        }
        ModelAction::EncodeText { models_dir, text } => {
            let encs = keyword_model::load_active_encoders(Path::new(&models_dir));
            let mut out = Vec::new();
            for e in &encs {
                match e.encode(&text) {
                    Ok(v) => out.push(serde_json::json!({"id": e.id(), "kind": e.kind(), "dim": e.dim(), "vector": v})),
                    Err(err) => out.push(serde_json::json!({"id": e.id(), "kind": e.kind(), "available": false, "error": err})),
                }
            }
            serde_json::json!({"success": true, "encoders": out})
        }
        ModelAction::RankCandidates { models_dir, query_terms, candidates } => {
            let dir = Path::new(&models_dir);
            let encs = keyword_model::load_active_encoders(dir);
            let ranker = keyword_model::load_active_ranker(dir);
            let cands: Vec<keyword_model::Candidate> =
                candidates.into_iter().map(|c| keyword_model::Candidate { id: c.id, terms: c.terms, facts: c.facts }).collect();
            let refs: Vec<&dyn keyword_model::TermEncoder> = encs.iter().map(|b| b.as_ref()).collect();
            let order = keyword_model::order_candidates(&query_terms, &cands, &refs, ranker.as_ref());
            serde_json::json!({
                "success": true,
                "order": order,
                "backends": {
                    "v0": true,
                    "encoders": encs.iter().map(|e| e.id().to_string()).collect::<Vec<_>>(),
                    "ranker": ranker.as_ref().map(|r| r.id.clone()),
                }
            })
        }
        ModelAction::TrainEmbedder { spec } => serde_json::json!({
            "success": false,
            "available": false,
            "error": "TrainEmbedder is PREPARED, not runnable: it needs a labelled corpus large enough to learn from (see docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §6.3) and a trained artifact to register as an active `embedder` backend",
            "requires": ["dataset built from the real hit log (keyword_training_set.jsonl)", "enough real labelled groups for a held-out evaluation", "artifact registered through ModelRegistry (kind=embedder)"],
            "spec_received": spec,
        }),
        ModelAction::FineTunePretrained { spec } => serde_json::json!({
            "success": false,
            "available": false,
            "error": "FineTunePretrained is PREPARED, not runnable: it needs third-party pretrained weights (an operator decision) and an inference runtime linked into the build",
            "requires": ["operator approval to download a small multilingual sentence encoder (384-dim, matching embedding_dimension)", "an inference runtime (dependency decision)", "dataset from the real hit log + a held-out evaluation"],
            "spec_received": spec,
        }),
    }
}

// ========== CLI Entry Point ==========

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = ozone_input_arg(&args[i + 1]);
        }
    }
    
    // The host's executor hands pipelines the `{data, context}` ENVELOPE; accept it or a flat input.
    let raw: serde_json::Value = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let inner = raw.get("data").cloned().unwrap_or(raw);
    match serde_json::from_value::<ModelAction>(inner.clone()) {
        Ok(model_action) => {
            println!("{}", serde_json::to_string(&run_model_action(model_action)).unwrap_or_else(|e| format!("{{\"success\":false,\"error\":\"{e}\"}}")));
            return;
        }
        Err(e) => {
            // An input that NAMES a model action but does not fit it must say so — falling through to the legacy traversal parser produced a
            // misleading "unknown variant `TrainModel`" error (found by the E2E).
            let named = inner.get("action").and_then(|a| a.as_str()).unwrap_or("");
            if ["TrainModel", "ModelStatus", "EncodeText", "RankCandidates", "TrainEmbedder", "FineTunePretrained"].contains(&named) {
                println!("{}", serde_json::json!({"success": false, "error": format!("{named}: invalid input: {e}")}));
                std::process::exit(1);
            }
        }
    }
    let input: TraversalInput = serde_json::from_value(inner).unwrap_or_else(|e| {
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

/// `--input @<path>`: the host stages oversize inputs (> 100 KB) in a file because a
/// single argv string is capped at 128 KiB (E2BIG). Plain values pass through unchanged.
#[allow(dead_code)]
fn ozone_input_arg(raw: &str) -> String {
    match raw.strip_prefix('@') {
        Some(path) if std::path::Path::new(path).is_file() => std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Failed to read staged input {}: {}", path, e);
            std::process::exit(1);
        }),
        _ => raw.to_string(),
    }
}
