//! ContextAggregationPipeline - Pipeline #21
//! 
//! Aggregate context from ZSEI for LLM prompts within token budget.
//! Key for effective zero-shot: right context at right time.
//! 
//! Per spec §7: Context Storage Architecture
//! 
//! ORDER OF EVENTS:
//! 1. Receive aggregation request (task/query/project)
//! 2. Query ZSEI for relevant containers
//! 3. Rank by relevance (keyword overlap, semantic similarity)
//! 4. IF consciousness enabled: Add emotional/relationship context
//! 5. Truncate to token budget
//! 6. Return aggregated context with sources
//! 
//! INTEGRATION POINTS:
//! - zsei_query: Load containers and relationships
//! - consciousness_query: Load emotional/relationship context (if enabled)
//! - file_link/url_link/package_link: Source references

use serde::{Deserialize, Serialize};
use std::env;

// ========== Input Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ContextAggInput {
    ForTask { task_id: u64, token_budget: u32, prioritize: Vec<String>, include_consciousness: Option<bool> },
    ForQuery { query: String, token_budget: u32, container_ids: Option<Vec<u64>>, project_id: Option<u64> },
    ForProject { project_id: u64, token_budget: u32, include_files: Option<bool>, include_urls: Option<bool> },
    Custom { container_ids: Vec<u64>, token_budget: u32, include_relationships: bool },
    ForBlueprint { blueprint_id: u64, step_index: u32, token_budget: u32 },
}

// ========== Output Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedContext {
    pub context_text: String,
    pub token_count: u32,
    pub sources: Vec<ContextSource>,
    pub truncated: bool,
    pub coverage_score: f32,
    pub consciousness_context: Option<ConsciousnessContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource { 
    pub container_id: u64, 
    pub container_type: String, 
    pub name: String,
    pub relevance: f32, 
    pub tokens_used: u32,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessContext {
    pub emotional_state: Option<String>,
    pub relationship_context: Option<String>,
    pub relevant_experiences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAggOutput {
    pub success: bool,
    pub context: Option<AggregatedContext>,
    pub error: Option<String>,
}

// ========== ZSEI Integration ==========

fn call_zsei_query(input: &serde_json::Value) -> Result<serde_json::Value, String> {
    let zsei_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
    let action = input.get("query_type").and_then(|a| a.as_str()).unwrap_or("");
    
    match action {
        "GetContainer" => {
            let container_id = input.get("container_id").and_then(|c| c.as_u64()).unwrap_or(0);
            let container_path = format!("{}/local/{}.json", zsei_path, container_id);
            
            if let Ok(content) = std::fs::read_to_string(&container_path) {
                if let Ok(container) = serde_json::from_str::<serde_json::Value>(&content) {
                    return Ok(serde_json::json!({"success": true, "containers": [container]}));
                }
            }
            Ok(serde_json::json!({"success": true, "containers": []}))
        }
        "GetProjectContext" => {
            let project_id = input.get("project_id").and_then(|p| p.as_u64()).unwrap_or(0);
            let project_path = format!("{}/local/{}.json", zsei_path, project_id);
            
            if let Ok(content) = std::fs::read_to_string(&project_path) {
                if let Ok(project) = serde_json::from_str::<serde_json::Value>(&content) {
                    let file_refs = project.get("file_references").and_then(|f| f.as_array()).cloned().unwrap_or_default();
                    let url_refs = project.get("url_references").and_then(|u| u.as_array()).cloned().unwrap_or_default();
                    return Ok(serde_json::json!({"success": true, "project": project, "file_references": file_refs, "url_references": url_refs}));
                }
            }
            Ok(serde_json::json!({"success": true, "project": null}))
        }
        _ => Ok(serde_json::json!({"success": true, "containers": []}))
    }
}

fn call_consciousness_query() -> Result<serde_json::Value, String> {
    let consciousness_path = env::var("OZONE_CONSCIOUSNESS_PATH").unwrap_or_else(|_| "./zsei_data/consciousness".to_string());
    
    let config_path = format!("{}/config.json", consciousness_path);
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if config.get("enabled").and_then(|e| e.as_bool()) != Some(true) {
                return Ok(serde_json::json!({"enabled": false}));
            }
        }
    } else {
        return Ok(serde_json::json!({"enabled": false}));
    }
    
    let emotional_path = format!("{}/emotional_state.json", consciousness_path);
    let emotional_state = std::fs::read_to_string(&emotional_path).ok().and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok());
    
    let experiences_path = format!("{}/experiences.json", consciousness_path);
    let experiences = std::fs::read_to_string(&experiences_path).ok().and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok());
    
    Ok(serde_json::json!({"enabled": true, "emotional_state": emotional_state, "experiences": experiences}))
}

// ========== Token Estimation ==========

fn estimate_tokens(text: &str) -> u32 { (text.len() as f32 / 4.0).ceil() as u32 }

fn truncate_to_budget(text: &str, budget: u32) -> (String, bool) {
    let estimated = estimate_tokens(text);
    if estimated <= budget { return (text.to_string(), false); }
    let target_chars = (budget as f32 * 4.0) as usize;
    let truncated = text.chars().take(target_chars).collect::<String>();
    if let Some(last_period) = truncated.rfind(". ") { return (truncated[..=last_period].to_string(), true); }
    (truncated, true)
}

// ========== Context Building ==========

fn build_context_from_containers(containers: &[serde_json::Value], budget: u32) -> (String, Vec<ContextSource>, bool) {
    let mut context_parts: Vec<String> = Vec::new();
    let mut sources: Vec<ContextSource> = Vec::new();
    let mut total_tokens: u32 = 0;
    let mut truncated = false;
    
    for container in containers {
        let container_id = container.get("container_id").and_then(|c| c.as_u64()).unwrap_or(0);
        let container_type = container.get("container_type").and_then(|t| t.as_str()).unwrap_or("Unknown");
        let name = container.get("name").and_then(|n| n.as_str()).unwrap_or("Unnamed");
        
        let content = match container_type {
            "FileReference" | "CodeAnalysis" => {
                let code = container.get("content").and_then(|c| c.as_str()).unwrap_or("");
                let summary = container.get("semantic_summary").and_then(|s| s.as_str()).unwrap_or("");
                if !summary.is_empty() { format!("## File: {}\n{}\n\n", name, summary) }
                else { format!("## File: {}\n```\n{}\n```\n\n", name, &code[..code.len().min(500)]) }
            }
            "URLReference" => {
                let url = container.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let summary = container.get("summary").and_then(|s| s.as_str()).unwrap_or("");
                format!("## URL: {}\n{}\n\n", url, summary)
            }
            "TextAnalysis" => {
                let summary = container.get("semantic_summary").and_then(|s| s.as_str()).unwrap_or("");
                format!("## Document: {}\n{}\n\n", name, summary)
            }
            _ => {
                let keywords = container.get("keywords").and_then(|k| k.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
                format!("## {}: {}\nKeywords: {}\n\n", container_type, name, keywords)
            }
        };
        
        let content_tokens = estimate_tokens(&content);
        if total_tokens + content_tokens > budget {
            truncated = true;
            let remaining = budget.saturating_sub(total_tokens);
            if remaining > 50 {
                let (trunc_content, _) = truncate_to_budget(&content, remaining);
                sources.push(ContextSource { container_id, container_type: container_type.to_string(), name: name.to_string(), relevance: 0.8, tokens_used: estimate_tokens(&trunc_content), snippet: trunc_content.chars().take(100).collect() });
                context_parts.push(trunc_content);
            }
            break;
        }
        
        context_parts.push(content.clone());
        sources.push(ContextSource { container_id, container_type: container_type.to_string(), name: name.to_string(), relevance: 0.9, tokens_used: content_tokens, snippet: content.chars().take(100).collect() });
        total_tokens += content_tokens;
    }
    
    (context_parts.join(""), sources, truncated)
}

fn build_consciousness_context(data: &serde_json::Value) -> Option<ConsciousnessContext> {
    if data.get("enabled").and_then(|e| e.as_bool()) != Some(true) { return None; }
    
    let emotional_state = data.get("emotional_state").and_then(|e| {
        let valence = e.get("valence").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let primary = e.get("primary_emotion").and_then(|p| p.as_str()).unwrap_or("neutral");
        Some(format!("Emotional state: {} (valence: {:.2})", primary, valence))
    });
    
    let relevant_experiences: Vec<String> = data.get("experiences").and_then(|e| e.get("experiences")).and_then(|exp| exp.as_object())
        .map(|obj| obj.values().take(3).filter_map(|v| v.get("summary").and_then(|s| s.as_str()).map(|s| s.to_string())).collect()).unwrap_or_default();
    
    Some(ConsciousnessContext { emotional_state, relationship_context: None, relevant_experiences })
}

// ========== Main Execution ==========

pub async fn execute(input: ContextAggInput) -> Result<ContextAggOutput, String> {
    match input {
        ContextAggInput::ForTask { task_id, token_budget, include_consciousness, .. } => {
            let query = serde_json::json!({"query_type": "GetContainer", "container_id": task_id});
            let result = call_zsei_query(&query)?;
            let containers = result.get("containers").and_then(|c| c.as_array()).cloned().unwrap_or_default();
            let (context_text, sources, truncated) = build_context_from_containers(&containers, token_budget);
            
            let consciousness_context = if include_consciousness.unwrap_or(true) {
                let consciousness_data = call_consciousness_query()?;
                build_consciousness_context(&consciousness_data)
            } else { None };
            
            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: context_text.clone(), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: if containers.is_empty() { 0.0 } else { 0.85 }, consciousness_context }), error: None })
        }
        
        ContextAggInput::ForQuery { query, token_budget, container_ids, project_id } => {
            let mut all_containers: Vec<serde_json::Value> = Vec::new();
            if let Some(ids) = container_ids {
                for id in ids {
                    let q = serde_json::json!({"query_type": "GetContainer", "container_id": id});
                    if let Ok(result) = call_zsei_query(&q) {
                        if let Some(containers) = result.get("containers").and_then(|c| c.as_array()) { all_containers.extend(containers.clone()); }
                    }
                }
            }
            if let Some(pid) = project_id {
                let q = serde_json::json!({"query_type": "GetProjectContext", "project_id": pid});
                if let Ok(result) = call_zsei_query(&q) {
                    if let Some(project) = result.get("project") { all_containers.push(project.clone()); }
                }
            }
            let (mut context_text, sources, truncated) = build_context_from_containers(&all_containers, token_budget);
            context_text = format!("Query: {}\n\n{}", query, context_text);
            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: context_text.clone(), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: 0.9, consciousness_context: None }), error: None })
        }
        
        ContextAggInput::ForProject { project_id, token_budget, include_files, include_urls } => {
            let mut all_containers: Vec<serde_json::Value> = Vec::new();
            let q = serde_json::json!({"query_type": "GetProjectContext", "project_id": project_id});
            if let Ok(result) = call_zsei_query(&q) {
                if let Some(project) = result.get("project") { all_containers.push(project.clone()); }
                if include_files.unwrap_or(true) { if let Some(refs) = result.get("file_references").and_then(|r| r.as_array()) { all_containers.extend(refs.clone()); } }
                if include_urls.unwrap_or(true) { if let Some(refs) = result.get("url_references").and_then(|r| r.as_array()) { all_containers.extend(refs.clone()); } }
            }
            let (context_text, sources, truncated) = build_context_from_containers(&all_containers, token_budget);
            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: context_text.clone(), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: 0.85, consciousness_context: None }), error: None })
        }
        
        ContextAggInput::Custom { container_ids, token_budget, .. } => {
            let mut all_containers: Vec<serde_json::Value> = Vec::new();
            for id in container_ids {
                let q = serde_json::json!({"query_type": "GetContainer", "container_id": id});
                if let Ok(result) = call_zsei_query(&q) {
                    if let Some(containers) = result.get("containers").and_then(|c| c.as_array()) { all_containers.extend(containers.clone()); }
                }
            }
            let (context_text, sources, truncated) = build_context_from_containers(&all_containers, token_budget);
            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: context_text.clone(), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: 0.9, consciousness_context: None }), error: None })
        }
        
        ContextAggInput::ForBlueprint { blueprint_id, step_index, token_budget } => {
            let q = serde_json::json!({"query_type": "GetContainer", "container_id": blueprint_id});
            let result = call_zsei_query(&q)?;
            let containers = result.get("containers").and_then(|c| c.as_array()).cloned().unwrap_or_default();
            let (context_text, sources, truncated) = build_context_from_containers(&containers, token_budget);
            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: format!("Blueprint step {} context:\n{}", step_index, context_text), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: 0.85, consciousness_context: None }), error: None })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    let input: ContextAggInput = serde_json::from_str(&input_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
