//! ExperienceSearchPipeline - Pipeline #38
//! 
//! Search and retrieve relevant past experiences for consciousness processing.
//! Per spec §38: Experience Retrieval System
//! 
//! REQUIRES: `consciousness` feature flag
//! 
//! Integrates with:
//! - experience_memory: Source of experiences
//! - emotional_state: Emotional context for relevance
//! - decision_gate: Provides relevant experiences for decisions

use serde::{Deserialize, Serialize};
use std::path::Path;

fn get_storage_path() -> String {
    std::env::var("OZONE_CONSCIOUSNESS_PATH")
        .unwrap_or_else(|_| "./zsei_data/consciousness".to_string())
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceResult {
    pub experience_id: u64,
    pub timestamp: u64,
    pub experience_type: String,
    pub summary: String,
    pub emotional_significance: f32,
    pub relevance_score: f32,
    pub lessons: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub experience_type: Option<String>,
    pub min_significance: Option<f32>,
    pub tags: Option<Vec<String>>,
    pub time_range: Option<(u64, u64)>,
    pub emotional_context: Option<String>,
    pub limit: Option<usize>,
}

fn load_experiences() -> Vec<ExperienceResult> {
    let path = Path::new(&get_storage_path()).join("experiences.json");
    let mut results = Vec::new();
    
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(exps) = data.get("experiences") {
                if let Some(map) = exps.as_object() {
                    for (id, exp) in map {
                        results.push(ExperienceResult {
                            experience_id: id.parse().unwrap_or(0),
                            timestamp: exp.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0),
                            experience_type: exp.get("experience_type").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                            summary: exp.get("summary").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                            emotional_significance: exp.get("emotional_significance").and_then(|s| s.as_f64()).unwrap_or(0.5) as f32,
                            relevance_score: 0.0,
                            lessons: exp.get("lessons_learned")
                                .and_then(|l| l.as_array())
                                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                .unwrap_or_default(),
                            tags: exp.get("tags")
                                .and_then(|t| t.as_array())
                                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                .unwrap_or_default(),
                        });
                    }
                }
            }
        }
    }
    
    results
}

fn calculate_relevance(exp: &ExperienceResult, query: &SearchQuery) -> f32 {
    let mut score = 0.0;
    let mut factors = 0;
    
    // Text match
    if let Some(ref text) = query.text {
        factors += 1;
        if exp.summary.to_lowercase().contains(&text.to_lowercase()) {
            score += 0.8;
        } else if exp.lessons.iter().any(|l| l.to_lowercase().contains(&text.to_lowercase())) {
            score += 0.6;
        }
    }
    
    // Type match
    if let Some(ref exp_type) = query.experience_type {
        factors += 1;
        if &exp.experience_type == exp_type {
            score += 1.0;
        }
    }
    
    // Tag match
    if let Some(ref tags) = query.tags {
        factors += 1;
        let matched = tags.iter().filter(|t| exp.tags.contains(t)).count();
        score += matched as f32 / tags.len() as f32;
    }
    
    // Significance weight
    score += exp.emotional_significance * 0.2;
    
    // Recency bonus (experiences from last week get boost)
    let one_week = 7 * 24 * 60 * 60;
    if now() - exp.timestamp < one_week {
        score += 0.1;
    }
    
    if factors > 0 {
        score / factors as f32
    } else {
        exp.emotional_significance * 0.5
    }
}

fn search_experiences(query: SearchQuery) -> Vec<ExperienceResult> {
    let mut experiences = load_experiences();
    let limit = query.limit.unwrap_or(10);
    
    // Filter by criteria
    experiences.retain(|exp| {
        // Significance filter
        if let Some(min_sig) = query.min_significance {
            if exp.emotional_significance < min_sig {
                return false;
            }
        }
        
        // Time range filter
        if let Some((start, end)) = query.time_range {
            if exp.timestamp < start || exp.timestamp > end {
                return false;
            }
        }
        
        // Type filter
        if let Some(ref exp_type) = query.experience_type {
            if &exp.experience_type != exp_type {
                return false;
            }
        }
        
        true
    });
    
    // Calculate relevance scores
    for exp in &mut experiences {
        exp.relevance_score = calculate_relevance(exp, &query);
    }
    
    // Sort by relevance
    experiences.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
    
    experiences.truncate(limit);
    experiences
}

fn find_similar(experience_id: u64, limit: usize) -> Vec<ExperienceResult> {
    let experiences = load_experiences();
    
    let target = experiences.iter().find(|e| e.experience_id == experience_id);
    if target.is_none() {
        return Vec::new();
    }
    let target = target.unwrap();
    
    let mut similar: Vec<_> = experiences.iter()
        .filter(|e| e.experience_id != experience_id)
        .map(|e| {
            let mut similarity = 0.0;
            
            // Same type
            if e.experience_type == target.experience_type {
                similarity += 0.3;
            }
            
            // Shared tags
            let shared_tags = e.tags.iter().filter(|t| target.tags.contains(t)).count();
            if !target.tags.is_empty() {
                similarity += 0.3 * (shared_tags as f32 / target.tags.len() as f32);
            }
            
            // Similar significance
            similarity += 0.2 * (1.0 - (e.emotional_significance - target.emotional_significance).abs());
            
            // Temporal proximity
            let time_diff = (e.timestamp as i64 - target.timestamp as i64).unsigned_abs();
            let one_month = 30 * 24 * 60 * 60;
            if time_diff < one_month {
                similarity += 0.2;
            }
            
            let mut exp = e.clone();
            exp.relevance_score = similarity;
            exp
        })
        .collect();
    
    similar.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
    similar.truncate(limit);
    similar
}

fn get_by_type(experience_type: &str, limit: usize) -> Vec<ExperienceResult> {
    let mut experiences = load_experiences();
    experiences.retain(|e| e.experience_type == experience_type);
    experiences.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    experiences.truncate(limit);
    experiences
}

fn get_recent(limit: usize) -> Vec<ExperienceResult> {
    let mut experiences = load_experiences();
    experiences.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    experiences.truncate(limit);
    experiences
}

fn get_most_significant(limit: usize) -> Vec<ExperienceResult> {
    let mut experiences = load_experiences();
    experiences.sort_by(|a, b| b.emotional_significance.partial_cmp(&a.emotional_significance).unwrap());
    experiences.truncate(limit);
    experiences
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SearchInput {
    /// Full search with query
    Search { query: SearchQuery },
    /// Find similar experiences
    FindSimilar { experience_id: u64, limit: Option<usize> },
    /// Get by type
    GetByType { experience_type: String, limit: Option<usize> },
    /// Get recent
    GetRecent { limit: Option<usize> },
    /// Get most significant
    GetMostSignificant { limit: Option<usize> },
    /// Get by ID
    GetById { experience_id: u64 },
    /// Get lessons from relevant experiences
    GetLessons { query: String, limit: Option<usize> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOutput {
    pub success: bool,
    pub results: Option<Vec<ExperienceResult>>,
    pub experience: Option<ExperienceResult>,
    pub lessons: Option<Vec<String>>,
    pub total_count: Option<usize>,
    pub error: Option<String>,
}

pub async fn execute(input: SearchInput) -> Result<SearchOutput, String> {
    match input {
        SearchInput::Search { query } => {
            let results = search_experiences(query);
            let count = results.len();
            Ok(SearchOutput {
                success: true,
                results: Some(results),
                experience: None,
                lessons: None,
                total_count: Some(count),
                error: None,
            })
        }
        SearchInput::FindSimilar { experience_id, limit } => {
            let results = find_similar(experience_id, limit.unwrap_or(5));
            let count = results.len();
            Ok(SearchOutput {
                success: true,
                results: Some(results),
                experience: None,
                lessons: None,
                total_count: Some(count),
                error: None,
            })
        }
        SearchInput::GetByType { experience_type, limit } => {
            let results = get_by_type(&experience_type, limit.unwrap_or(10));
            let count = results.len();
            Ok(SearchOutput {
                success: true,
                results: Some(results),
                experience: None,
                lessons: None,
                total_count: Some(count),
                error: None,
            })
        }
        SearchInput::GetRecent { limit } => {
            let results = get_recent(limit.unwrap_or(10));
            let count = results.len();
            Ok(SearchOutput {
                success: true,
                results: Some(results),
                experience: None,
                lessons: None,
                total_count: Some(count),
                error: None,
            })
        }
        SearchInput::GetMostSignificant { limit } => {
            let results = get_most_significant(limit.unwrap_or(10));
            let count = results.len();
            Ok(SearchOutput {
                success: true,
                results: Some(results),
                experience: None,
                lessons: None,
                total_count: Some(count),
                error: None,
            })
        }
        SearchInput::GetById { experience_id } => {
            let experiences = load_experiences();
            let exp = experiences.into_iter().find(|e| e.experience_id == experience_id);
            Ok(SearchOutput {
                success: exp.is_some(),
                results: None,
                experience: exp,
                lessons: None,
                total_count: None,
                error: None,
            })
        }
        SearchInput::GetLessons { query, limit } => {
            let results = search_experiences(SearchQuery {
                text: Some(query),
                experience_type: None,
                min_significance: Some(0.5),
                tags: None,
                time_range: None,
                emotional_context: None,
                limit,
            });
            
            let lessons: Vec<String> = results.iter()
                .flat_map(|e| e.lessons.clone())
                .collect();
            
            Ok(SearchOutput {
                success: true,
                results: None,
                experience: None,
                lessons: Some(lessons),
                total_count: Some(lessons.len()),
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
    let input: SearchInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { eprintln!("{}", e); std::process::exit(1); }
    }
}
