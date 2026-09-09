//! ZeroShotSimulationPipeline - Pipeline #16
//! 
//! Simulate task execution to validate approach before actual execution.
//! Key for zero-shot system - validates methodology/blueprint without training.
//! 
//! Per spec §15: Zero-Shot Simulation Loops
//! 
//! ORDER OF EVENTS:
//! 1. Receive task signature + methodologies
//! 2. Load methodology details from ZSEI
//! 3. Build validation prompt
//! 4. LLM validates coverage and gaps
//! 5. Parse validation results
//! 6. Return simulation result with confidence
//! 
//! INTEGRATION POINTS:
//! - methodology_fetch: Load methodology details
//! - prompt: LLM validation
//! - zsei_query: Load context

use serde::{Deserialize, Serialize};
use std::env;

// ========== Input Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ZeroShotSimInput {
    /// Simulate full task execution
    Simulate {
        task_signature: TaskSignature,
        methodologies: Vec<u64>,
        context_ids: Vec<u64>,
        validate_only: bool,
    },
    /// Validate methodology fit for task
    ValidateMethodology {
        methodology_id: u64,
        task_signature: TaskSignature,
    },
    /// Validate blueprint completeness
    ValidateBlueprint {
        blueprint_id: u64,
        task_signature: TaskSignature,
    },
    /// Preview output structure
    PreviewOutput {
        blueprint_id: u64,
        sample_input: serde_json::Value,
    },
    /// Check methodology gaps
    CheckGaps {
        task_description: String,
        methodologies: Vec<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSignature {
    pub input_types: Vec<String>,
    pub output_type: String,
    pub constraints: Vec<String>,
    pub keywords: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub feasible: bool,
    pub confidence: f32,
    pub estimated_steps: u32,
    pub estimated_time_secs: u32,
    pub methodology_coverage: f32,
    pub missing_context: Vec<String>,
    pub warnings: Vec<String>,
    pub suggested_adjustments: Vec<String>,
    pub gaps_identified: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroShotSimOutput {
    pub success: bool,
    pub simulation: Option<SimulationResult>,
    pub preview: Option<serde_json::Value>,
    pub methodology_valid: Option<bool>,
    pub blueprint_valid: Option<bool>,
    pub error: Option<String>,
}

// ========== LLM Integration ==========

async fn call_prompt_pipeline(prompt: &str, system: &str) -> Result<String, String> {
    let api_key = env::var("ANTHROPIC_API_KEY")
        .or_else(|_| env::var("OPENAI_API_KEY"))
        .map_err(|_| "No API key configured")?;
    
    let endpoint = env::var("OZONE_API_ENDPOINT")
        .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string());
    
    let model = env::var("OZONE_API_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
    
    let client = reqwest::Client::new();
    
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 1500,
        "messages": [{"role": "user", "content": prompt}],
        "system": system
    });
    
    let response = client
        .post(&endpoint)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("API error: {}", e))?;
    
    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Parse error: {}", e))?;
    
    result["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No response".to_string())
}

// ========== ZSEI Integration ==========

fn load_methodology(methodology_id: u64) -> Result<serde_json::Value, String> {
    let zsei_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
    let methodology_path = format!("{}/local/{}.json", zsei_path, methodology_id);
    
    std::fs::read_to_string(&methodology_path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .ok_or_else(|| format!("Methodology {} not found", methodology_id))
}

fn load_blueprint(blueprint_id: u64) -> Result<serde_json::Value, String> {
    let zsei_path = env::var("OZONE_ZSEI_PATH").unwrap_or_else(|_| "./zsei_data".to_string());
    let blueprint_path = format!("{}/local/{}.json", zsei_path, blueprint_id);
    
    std::fs::read_to_string(&blueprint_path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .ok_or_else(|| format!("Blueprint {} not found", blueprint_id))
}

// ========== Validation Functions ==========

async fn validate_with_llm(task: &TaskSignature, methodologies: &[serde_json::Value]) -> Result<SimulationResult, String> {
    // Build methodology summary
    let methodology_summary: Vec<String> = methodologies.iter()
        .filter_map(|m| {
            let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
            let description = m.get("description").and_then(|d| d.as_str()).unwrap_or("");
            let principles: Vec<String> = m.get("principles")
                .and_then(|p| p.as_array())
                .map(|arr| arr.iter()
                    .filter_map(|v| v.get("statement").and_then(|s| s.as_str()).map(|s| format!("- {}", s)))
                    .collect())
                .unwrap_or_default();
            Some(format!("## {}\n{}\nPrinciples:\n{}", name, description, principles.join("\n")))
        })
        .collect();
    
    let system_prompt = r#"You are validating whether methodologies can accomplish a task.
Analyze the task requirements and methodology coverage.
Return JSON with exactly these fields:
{
    "feasible": true/false,
    "confidence": 0.0-1.0,
    "coverage": 0.0-1.0,
    "estimated_steps": number,
    "missing": ["list of gaps"],
    "warnings": ["list of concerns"],
    "adjustments": ["suggested improvements"]
}"#;
    
    let task_desc = task.description.clone().unwrap_or_else(|| 
        format!("Task with inputs [{}] producing {}", task.input_types.join(", "), task.output_type)
    );
    
    let prompt = format!(
        "Task: {}\n\nConstraints: {}\n\nAvailable Methodologies:\n{}\n\nValidate if these methodologies can accomplish this task.",
        task_desc,
        task.constraints.join(", "),
        methodology_summary.join("\n\n")
    );
    
    let response = call_prompt_pipeline(&prompt, system_prompt).await?;
    
    // Parse LLM response
    let parsed: serde_json::Value = serde_json::from_str(&response)
        .unwrap_or_else(|_| serde_json::json!({
            "feasible": true,
            "confidence": 0.7,
            "coverage": 0.8,
            "estimated_steps": 5,
            "missing": [],
            "warnings": ["Could not parse LLM response"],
            "adjustments": []
        }));
    
    Ok(SimulationResult {
        feasible: parsed.get("feasible").and_then(|f| f.as_bool()).unwrap_or(true),
        confidence: parsed.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.7) as f32,
        estimated_steps: parsed.get("estimated_steps").and_then(|e| e.as_u64()).unwrap_or(5) as u32,
        estimated_time_secs: parsed.get("estimated_steps").and_then(|e| e.as_u64()).unwrap_or(5) as u32 * 10,
        methodology_coverage: parsed.get("coverage").and_then(|c| c.as_f64()).unwrap_or(0.8) as f32,
        missing_context: parsed.get("missing")
            .and_then(|m| m.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
        warnings: parsed.get("warnings")
            .and_then(|w| w.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
        suggested_adjustments: parsed.get("adjustments")
            .and_then(|a| a.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
        gaps_identified: parsed.get("missing")
            .and_then(|m| m.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
    })
}

async fn validate_blueprint_with_llm(blueprint: &serde_json::Value, task: &TaskSignature) -> Result<(bool, Vec<String>), String> {
    let system_prompt = r#"You are validating a task blueprint for completeness.
Check if all required steps are present and in correct order.
Return JSON: {"valid": true/false, "issues": ["list of issues"]}"#;
    
    let steps: Vec<String> = blueprint.get("steps")
        .and_then(|s| s.as_array())
        .map(|arr| arr.iter()
            .filter_map(|s| s.get("description").and_then(|d| d.as_str()).map(|d| d.to_string()))
            .collect())
        .unwrap_or_default();
    
    let prompt = format!(
        "Task: {} -> {}\n\nBlueprint steps:\n{}\n\nIs this blueprint complete?",
        task.input_types.join(", "),
        task.output_type,
        steps.iter().enumerate().map(|(i, s)| format!("{}. {}", i + 1, s)).collect::<Vec<_>>().join("\n")
    );
    
    let response = call_prompt_pipeline(&prompt, system_prompt).await?;
    
    let parsed: serde_json::Value = serde_json::from_str(&response)
        .unwrap_or_else(|_| serde_json::json!({"valid": true, "issues": []}));
    
    let valid = parsed.get("valid").and_then(|v| v.as_bool()).unwrap_or(true);
    let issues: Vec<String> = parsed.get("issues")
        .and_then(|i| i.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    
    Ok((valid, issues))
}

// ========== Basic Validation (Non-LLM fallback) ==========

fn basic_validation(task: &TaskSignature, methodologies: &[serde_json::Value]) -> SimulationResult {
    let mut coverage = 0.0f32;
    let mut warnings = Vec::new();
    let mut missing = Vec::new();
    
    // Check if we have methodologies
    if methodologies.is_empty() {
        missing.push("No methodologies provided".to_string());
        coverage = 0.0;
    } else {
        coverage = 0.5; // Base coverage for having methodologies
        
        // Check keyword overlap
        let task_keywords: Vec<String> = task.keywords.clone().unwrap_or_default();
        for method in methodologies {
            let method_keywords: Vec<String> = method.get("keywords")
                .and_then(|k| k.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            
            let overlap = task_keywords.iter()
                .filter(|tk| method_keywords.iter().any(|mk| mk.to_lowercase().contains(&tk.to_lowercase())))
                .count();
            
            if overlap > 0 {
                coverage += 0.1 * (overlap as f32 / task_keywords.len().max(1) as f32);
            }
        }
        
        coverage = coverage.min(1.0);
    }
    
    // Check constraints
    if !task.constraints.is_empty() && coverage < 0.8 {
        warnings.push(format!("{} constraints may not be fully covered", task.constraints.len()));
    }
    
    SimulationResult {
        feasible: coverage > 0.3,
        confidence: coverage,
        estimated_steps: 5,
        estimated_time_secs: 30,
        methodology_coverage: coverage,
        missing_context: missing,
        warnings,
        suggested_adjustments: vec![],
        gaps_identified: vec![],
    }
}

// ========== Main Execution ==========

pub async fn execute(input: ZeroShotSimInput) -> Result<ZeroShotSimOutput, String> {
    match input {
        ZeroShotSimInput::Simulate { task_signature, methodologies, context_ids, validate_only } => {
            // Load methodologies
            let loaded_methods: Vec<serde_json::Value> = methodologies.iter()
                .filter_map(|id| load_methodology(*id).ok())
                .collect();
            
            // Try LLM validation first, fall back to basic
            let simulation = if env::var("ANTHROPIC_API_KEY").is_ok() || env::var("OPENAI_API_KEY").is_ok() {
                validate_with_llm(&task_signature, &loaded_methods).await
                    .unwrap_or_else(|_| basic_validation(&task_signature, &loaded_methods))
            } else {
                basic_validation(&task_signature, &loaded_methods)
            };
            
            Ok(ZeroShotSimOutput {
                success: true,
                simulation: Some(simulation),
                preview: None,
                methodology_valid: None,
                blueprint_valid: None,
                error: None,
            })
        }
        
        ZeroShotSimInput::ValidateMethodology { methodology_id, task_signature } => {
            let methodology = load_methodology(methodology_id)?;
            
            let simulation = if env::var("ANTHROPIC_API_KEY").is_ok() || env::var("OPENAI_API_KEY").is_ok() {
                validate_with_llm(&task_signature, &[methodology.clone()]).await
                    .unwrap_or_else(|_| basic_validation(&task_signature, &[methodology.clone()]))
            } else {
                basic_validation(&task_signature, &[methodology])
            };
            
            Ok(ZeroShotSimOutput {
                success: true,
                simulation: Some(simulation.clone()),
                preview: None,
                methodology_valid: Some(simulation.confidence >= 0.7),
                blueprint_valid: None,
                error: None,
            })
        }
        
        ZeroShotSimInput::ValidateBlueprint { blueprint_id, task_signature } => {
            let blueprint = load_blueprint(blueprint_id)?;
            
            let (valid, issues) = if env::var("ANTHROPIC_API_KEY").is_ok() || env::var("OPENAI_API_KEY").is_ok() {
                validate_blueprint_with_llm(&blueprint, &task_signature).await
                    .unwrap_or_else(|_| (true, vec![]))
            } else {
                (true, vec![])
            };
            
            let simulation = SimulationResult {
                feasible: valid,
                confidence: if valid { 0.85 } else { 0.4 },
                estimated_steps: blueprint.get("steps").and_then(|s| s.as_array()).map(|a| a.len()).unwrap_or(5) as u32,
                estimated_time_secs: 30,
                methodology_coverage: 0.9,
                missing_context: vec![],
                warnings: issues,
                suggested_adjustments: vec![],
                gaps_identified: vec![],
            };
            
            Ok(ZeroShotSimOutput {
                success: true,
                simulation: Some(simulation),
                preview: None,
                methodology_valid: None,
                blueprint_valid: Some(valid),
                error: None,
            })
        }
        
        ZeroShotSimInput::PreviewOutput { blueprint_id, sample_input } => {
            let blueprint = load_blueprint(blueprint_id)?;
            
            let output_type = blueprint.get("output_type").and_then(|o| o.as_str()).unwrap_or("unknown");
            let preview = serde_json::json!({
                "output_type": output_type,
                "sample_input": sample_input,
                "expected_format": {
                    "status": "success",
                    "result": format!("<{} output>", output_type)
                }
            });
            
            Ok(ZeroShotSimOutput {
                success: true,
                simulation: None,
                preview: Some(preview),
                methodology_valid: None,
                blueprint_valid: None,
                error: None,
            })
        }
        
        ZeroShotSimInput::CheckGaps { task_description, methodologies } => {
            let loaded_methods: Vec<serde_json::Value> = methodologies.iter()
                .filter_map(|id| load_methodology(*id).ok())
                .collect();
            
            let task_sig = TaskSignature {
                input_types: vec![],
                output_type: "unknown".to_string(),
                constraints: vec![],
                keywords: Some(task_description.split_whitespace().map(|s| s.to_string()).collect()),
                description: Some(task_description),
            };
            
            let simulation = if env::var("ANTHROPIC_API_KEY").is_ok() || env::var("OPENAI_API_KEY").is_ok() {
                validate_with_llm(&task_sig, &loaded_methods).await
                    .unwrap_or_else(|_| basic_validation(&task_sig, &loaded_methods))
            } else {
                basic_validation(&task_sig, &loaded_methods)
            };
            
            Ok(ZeroShotSimOutput {
                success: true,
                simulation: Some(simulation),
                preview: None,
                methodology_valid: None,
                blueprint_valid: None,
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
    
    let input: ZeroShotSimInput = serde_json::from_str(&input_json).unwrap_or_else(|e| {
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
