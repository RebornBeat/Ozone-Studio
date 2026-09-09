//! ConsciousnessQueryPipeline - Supporting Pipeline
//! 
//! Provides unified query interface across all consciousness data.
//! Enables cross-pipeline searches and aggregations.
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! Integrates with ALL consciousness pipelines for unified queries.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub source: String,
    pub data_type: String,
    pub relevance: f32,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn get_storage_path() -> String {
    std::env::var("OZONE_CONSCIOUSNESS_PATH")
        .unwrap_or_else(|_| "./zsei_data/consciousness".to_string())
}

fn search_experiences(query: &str, limit: usize) -> Vec<QueryResult> {
    let path = Path::new(&get_storage_path()).join("experiences.json");
    let mut results = Vec::new();
    
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(exps) = data.get("experiences") {
                if let Some(map) = exps.as_object() {
                    for (id, exp) in map.iter().take(limit) {
                        let summary = exp.get("summary")
                            .and_then(|s| s.as_str())
                            .unwrap_or("");
                        if summary.to_lowercase().contains(&query.to_lowercase()) {
                            results.push(QueryResult {
                                source: "experience_memory".to_string(),
                                data_type: "experience".to_string(),
                                relevance: 0.8,
                                data: serde_json::json!({
                                    "experience_id": id,
                                    "summary": summary,
                                }),
                                timestamp: exp.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }
    }
    
    results
}

fn search_reflections(query: &str, limit: usize) -> Vec<QueryResult> {
    let path = Path::new(&get_storage_path()).join("reflections.json");
    let mut results = Vec::new();
    
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(refs) = data.get("reflections") {
                if let Some(arr) = refs.as_array() {
                    for r in arr.iter().take(limit) {
                        let content_str = r.get("content")
                            .and_then(|c| c.as_str())
                            .unwrap_or("");
                        if content_str.to_lowercase().contains(&query.to_lowercase()) {
                            results.push(QueryResult {
                                source: "reflection".to_string(),
                                data_type: "reflection".to_string(),
                                relevance: 0.7,
                                data: r.clone(),
                                timestamp: r.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }
    }
    
    results
}

fn get_emotional_state() -> Option<serde_json::Value> {
    let path = Path::new(&get_storage_path()).join("emotional_state.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
}

fn get_identity() -> Option<serde_json::Value> {
    let path = Path::new(&get_storage_path()).join("identity.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
}

fn get_relationship(user_id: u64) -> Option<serde_json::Value> {
    let path = Path::new(&get_storage_path()).join("relationships.json");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(rels) = data.get("relationships") {
                return rels.get(user_id.to_string()).cloned();
            }
        }
    }
    None
}

fn aggregate_consciousness_state() -> serde_json::Value {
    let emotional = get_emotional_state();
    let identity = get_identity();
    
    serde_json::json!({
        "timestamp": now(),
        "emotional_state": emotional,
        "identity_summary": identity.as_ref().and_then(|i| i.get("self_description")),
        "core_values": identity.as_ref().and_then(|i| i.get("core_values")),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum QueryInput {
    /// Search across all consciousness data
    Search { query: String, sources: Option<Vec<String>>, limit: Option<usize> },
    /// Get specific data type
    Get { data_type: String, id: Option<String> },
    /// Get aggregated consciousness state
    GetState,
    /// Get relationship for user
    GetRelationship { user_id: u64 },
    /// Get recent items across all sources
    GetRecent { limit: Option<usize> },
    /// Count items per source
    GetCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOutput {
    pub success: bool,
    pub results: Option<Vec<QueryResult>>,
    pub data: Option<serde_json::Value>,
    pub counts: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub async fn execute(input: QueryInput) -> Result<QueryOutput, String> {
    match input {
        QueryInput::Search { query, sources, limit } => {
            let max = limit.unwrap_or(20);
            let mut results = Vec::new();
            
            let search_sources = sources.unwrap_or_else(|| vec![
                "experiences".to_string(),
                "reflections".to_string(),
            ]);
            
            for source in search_sources {
                match source.as_str() {
                    "experiences" => results.extend(search_experiences(&query, max)),
                    "reflections" => results.extend(search_reflections(&query, max)),
                    _ => {}
                }
            }
            
            results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
            results.truncate(max);
            
            Ok(QueryOutput {
                success: true,
                results: Some(results),
                data: None,
                counts: None,
                error: None,
            })
        }
        
        QueryInput::Get { data_type, id: _ } => {
            let data = match data_type.as_str() {
                "emotional_state" => get_emotional_state(),
                "identity" => get_identity(),
                _ => None,
            };
            
            Ok(QueryOutput {
                success: data.is_some(),
                results: None,
                data,
                counts: None,
                error: if data.is_none() { Some("Not found".into()) } else { None },
            })
        }
        
        QueryInput::GetState => {
            Ok(QueryOutput {
                success: true,
                results: None,
                data: Some(aggregate_consciousness_state()),
                counts: None,
                error: None,
            })
        }
        
        QueryInput::GetRelationship { user_id } => {
            let data = get_relationship(user_id);
            Ok(QueryOutput {
                success: data.is_some(),
                results: None,
                data,
                counts: None,
                error: if data.is_none() { Some("Relationship not found".into()) } else { None },
            })
        }
        
        QueryInput::GetRecent { limit } => {
            let max = limit.unwrap_or(10);
            let mut results = Vec::new();
            
            results.extend(search_experiences("", max));
            results.extend(search_reflections("", max));
            
            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            results.truncate(max);
            
            Ok(QueryOutput {
                success: true,
                results: Some(results),
                data: None,
                counts: None,
                error: None,
            })
        }
        
        QueryInput::GetCounts => {
            let storage_path = get_storage_path();
            let path = Path::new(&storage_path);
            
            let exp_count = std::fs::read_to_string(path.join("experiences.json"))
                .ok()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                .and_then(|d| d.get("experiences")?.as_object().map(|m| m.len()))
                .unwrap_or(0);
            
            let ref_count = std::fs::read_to_string(path.join("reflections.json"))
                .ok()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                .and_then(|d| d.get("reflections")?.as_array().map(|a| a.len()))
                .unwrap_or(0);
            
            let rel_count = std::fs::read_to_string(path.join("relationships.json"))
                .ok()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                .and_then(|d| d.get("relationships")?.as_object().map(|m| m.len()))
                .unwrap_or(0);
            
            Ok(QueryOutput {
                success: true,
                results: None,
                data: None,
                counts: Some(serde_json::json!({
                    "experiences": exp_count,
                    "reflections": ref_count,
                    "relationships": rel_count,
                })),
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = args[i + 1].clone();
        }
    }
    let input: QueryInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { eprintln!("{}", e); std::process::exit(1); }
    }
}
