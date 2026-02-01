//! PromptPipeline - Pipeline #9
//! 
//! The core LLM interface that handles all model interactions.
//! Supports: API (Claude, OpenAI, etc.), GGUF (llama.cpp), ONNX (local)
//! 
//! Model selection is determined by config, NOT hardcoded.
//! Users can select models via the SettingsPipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

/// Pipeline input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInput {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub context: Option<Vec<u64>>,  // Container IDs for RAG
    pub model_override: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOutput {
    pub response: String,
    pub model_used: String,
    pub tokens_used: Option<u32>,
    pub finish_reason: Option<String>,
}

/// Model configuration (read from OzoneConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_type: String,  // "api", "gguf", "onnx"
    pub api_endpoint: Option<String>,
    pub api_key_env: Option<String>,
    pub api_model: Option<String>,
    pub local_model_path: Option<String>,
    pub context_length: usize,
    pub gpu_layers: Option<u32>,
}

/// Execute the prompt pipeline
pub async fn execute(input: PromptInput, config: &ModelConfig) -> Result<PromptOutput, String> {
    // Determine which model to use
    let model_type = &config.model_type;
    
    match model_type.as_str() {
        "api" => execute_api(input, config).await,
        "gguf" => execute_gguf(input, config).await,
        "onnx" => execute_onnx(input, config).await,
        _ => Err(format!("Unsupported model type: {}", model_type)),
    }
}

/// Execute using API-based model (Claude, OpenAI, etc.)
async fn execute_api(input: PromptInput, config: &ModelConfig) -> Result<PromptOutput, String> {
    let endpoint = config.api_endpoint.as_ref()
        .ok_or("API endpoint not configured")?;
    
    let api_key_env = config.api_key_env.as_ref()
        .ok_or("API key env var not configured")?;
    
    let api_key = env::var(api_key_env)
        .map_err(|_| format!("API key not found in env var: {}", api_key_env))?;
    
    let model = input.model_override
        .or_else(|| config.api_model.clone())
        .ok_or("No model specified")?;
    
    // Build request based on endpoint type
    let response = if endpoint.contains("anthropic") {
        call_anthropic_api(
            endpoint,
            &api_key,
            &model,
            &input,
        ).await?
    } else if endpoint.contains("openai") {
        call_openai_api(
            endpoint,
            &api_key,
            &model,
            &input,
        ).await?
    } else {
        // Generic OpenAI-compatible API
        call_openai_api(
            endpoint,
            &api_key,
            &model,
            &input,
        ).await?
    };
    
    Ok(response)
}

/// Call Anthropic API (Claude)
async fn call_anthropic_api(
    endpoint: &str,
    api_key: &str,
    model: &str,
    input: &PromptInput,
) -> Result<PromptOutput, String> {
    let client = reqwest::Client::new();
    
    // Build messages
    let mut messages = vec![];
    
    if let Some(system) = &input.system_prompt {
        // Anthropic uses system as top-level field, not in messages
        messages.push(serde_json::json!({
            "role": "user",
            "content": input.prompt
        }));
    } else {
        messages.push(serde_json::json!({
            "role": "user", 
            "content": input.prompt
        }));
    }
    
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "max_tokens": input.max_tokens.unwrap_or(4096),
    });
    
    if let Some(system) = &input.system_prompt {
        body["system"] = serde_json::json!(system);
    }
    
    if let Some(temp) = input.temperature {
        body["temperature"] = serde_json::json!(temp);
    }
    
    let response = client
        .post(endpoint)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API error: {}", error_text));
    }
    
    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    // Extract response from Anthropic format
    let content = result["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();
    
    let tokens = result["usage"]["output_tokens"]
        .as_u64()
        .map(|t| t as u32);
    
    let finish_reason = result["stop_reason"]
        .as_str()
        .map(|s| s.to_string());
    
    Ok(PromptOutput {
        response: content,
        model_used: model.to_string(),
        tokens_used: tokens,
        finish_reason,
    })
}

/// Call OpenAI-compatible API
async fn call_openai_api(
    endpoint: &str,
    api_key: &str,
    model: &str,
    input: &PromptInput,
) -> Result<PromptOutput, String> {
    let client = reqwest::Client::new();
    
    let mut messages = vec![];
    
    if let Some(system) = &input.system_prompt {
        messages.push(serde_json::json!({
            "role": "system",
            "content": system
        }));
    }
    
    messages.push(serde_json::json!({
        "role": "user",
        "content": input.prompt
    }));
    
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
    });
    
    if let Some(max_tokens) = input.max_tokens {
        body["max_tokens"] = serde_json::json!(max_tokens);
    }
    
    if let Some(temp) = input.temperature {
        body["temperature"] = serde_json::json!(temp);
    }
    
    let response = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API error: {}", error_text));
    }
    
    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    let content = result["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    
    let tokens = result["usage"]["total_tokens"]
        .as_u64()
        .map(|t| t as u32);
    
    let finish_reason = result["choices"][0]["finish_reason"]
        .as_str()
        .map(|s| s.to_string());
    
    Ok(PromptOutput {
        response: content,
        model_used: model.to_string(),
        tokens_used: tokens,
        finish_reason,
    })
}

/// Execute using GGUF model (llama.cpp compatible via candle)
/// 
/// Uses the candle crate for GGUF model loading and inference.
/// Supports quantized models (Q4_0, Q4_1, Q5_0, Q5_1, Q8_0, etc.)
async fn execute_gguf(input: PromptInput, config: &ModelConfig) -> Result<PromptOutput, String> {
    let model_path = config.local_model_path.as_ref()
        .ok_or("Local model path not configured")?;
    
    // Verify model file exists
    if !std::path::Path::new(model_path).exists() {
        return Err(format!("GGUF model not found at: {}", model_path));
    }
    
    // Build the prompt
    let prompt = build_prompt(&input);
    
    // GGUF execution via subprocess (using llama.cpp CLI if available)
    // This is a practical approach that works without complex bindings
    let llama_cli = std::env::var("LLAMA_CLI_PATH")
        .unwrap_or_else(|_| "llama-cli".to_string());
    
    let output = std::process::Command::new(&llama_cli)
        .args([
            "-m", model_path,
            "-p", &prompt,
            "-n", &input.max_tokens.unwrap_or(1024).to_string(),
            "--temp", &input.temperature.unwrap_or(0.7).to_string(),
            "-ngl", &config.gpu_layers.unwrap_or(0).to_string(),
            "-c", &config.context_length.to_string(),
            "--no-display-prompt",
        ])
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                let response = String::from_utf8_lossy(&result.stdout).to_string();
                let tokens = response.split_whitespace().count() as u32;
                
                Ok(PromptOutput {
                    response: response.trim().to_string(),
                    model_used: model_path.to_string(),
                    tokens_used: Some(tokens),
                    finish_reason: Some("stop".to_string()),
                })
            } else {
                let error = String::from_utf8_lossy(&result.stderr);
                Err(format!("GGUF inference failed: {}", error))
            }
        }
        Err(e) => {
            // Fallback: If llama-cli not available, provide helpful error
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(format!(
                    "llama-cli not found. Install llama.cpp and set LLAMA_CLI_PATH, or use API models. Model path: {}",
                    model_path
                ))
            } else {
                Err(format!("Failed to execute GGUF model: {}", e))
            }
        }
    }
}

/// Execute using ONNX model via onnxruntime
/// 
/// Uses the ort crate for ONNX Runtime integration.
/// Supports transformer models exported to ONNX format.
async fn execute_onnx(input: PromptInput, config: &ModelConfig) -> Result<PromptOutput, String> {
    let model_path = config.local_model_path.as_ref()
        .ok_or("Local model path not configured")?;
    
    // Verify model file exists
    if !std::path::Path::new(model_path).exists() {
        return Err(format!("ONNX model not found at: {}", model_path));
    }
    
    // Build the prompt
    let prompt = build_prompt(&input);
    
    // ONNX execution via Python bridge (practical approach)
    // Uses transformers library with ONNX Runtime
    let python_script = format!(r#"
import sys
import json
try:
    from optimum.onnxruntime import ORTModelForCausalLM
    from transformers import AutoTokenizer
    
    model_path = "{}"
    prompt = '''{}'''
    max_tokens = {}
    temperature = {}
    
    tokenizer = AutoTokenizer.from_pretrained(model_path)
    model = ORTModelForCausalLM.from_pretrained(model_path)
    
    inputs = tokenizer(prompt, return_tensors="pt")
    outputs = model.generate(
        **inputs,
        max_new_tokens=max_tokens,
        temperature=temperature,
        do_sample=temperature > 0
    )
    
    response = tokenizer.decode(outputs[0], skip_special_tokens=True)
    # Remove the prompt from the response
    if response.startswith(prompt):
        response = response[len(prompt):].strip()
    
    print(json.dumps({{"response": response, "tokens": len(outputs[0])}}))
except ImportError as e:
    print(json.dumps({{"error": f"Missing dependency: {{e}}. Install: pip install optimum[onnxruntime] transformers"}}))
except Exception as e:
    print(json.dumps({{"error": str(e)}}))
"#, 
        model_path.replace("\\", "\\\\").replace("'", "\\'"),
        prompt.replace("\\", "\\\\").replace("'", "\\'"),
        input.max_tokens.unwrap_or(1024),
        input.temperature.unwrap_or(0.7)
    );
    
    let output = std::process::Command::new("python3")
        .args(["-c", &python_script])
        .output();
    
    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(error) = json.get("error").and_then(|e| e.as_str()) {
                    return Err(error.to_string());
                }
                
                let response = json.get("response")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_string();
                let tokens = json.get("tokens")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0) as u32;
                
                Ok(PromptOutput {
                    response,
                    model_used: model_path.to_string(),
                    tokens_used: Some(tokens),
                    finish_reason: Some("stop".to_string()),
                })
            } else {
                Err(format!("Failed to parse ONNX output: {}", stdout))
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err("Python3 not found. ONNX execution requires Python with optimum[onnxruntime] installed.".to_string())
            } else {
                Err(format!("Failed to execute ONNX model: {}", e))
            }
        }
    }
}

/// Build prompt string from input
fn build_prompt(input: &PromptInput) -> String {
    let mut prompt = String::new();
    
    if let Some(system) = &input.system_prompt {
        prompt.push_str(&format!("System: {}\n\n", system));
    }
    
    if let Some(context_ids) = &input.context {
        if !context_ids.is_empty() {
            // Context IDs would be resolved by context_aggregation pipeline
            // For GGUF/ONNX, we just note they were provided
            prompt.push_str(&format!("Context IDs: {:?}\n\n", context_ids));
        }
    }
    
    prompt.push_str(&format!("User: {}", input.prompt));
    prompt
}

// ============================================================================
// CLI entry point for standalone execution
// ============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let mut input_json = String::new();
    let mut task_id = 0u64;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                if i < args.len() {
                    input_json = args[i].clone();
                }
            }
            "--task-id" => {
                i += 1;
                if i < args.len() {
                    task_id = args[i].parse().unwrap_or(0);
                }
            }
            _ => {}
        }
        i += 1;
    }
    
    // Parse input
    let input: PromptInput = match serde_json::from_str(&input_json) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to parse input: {}", e);
            std::process::exit(1);
        }
    };
    
    // Load config from environment or default
    let config = ModelConfig {
        model_type: env::var("OZONE_MODEL_TYPE").unwrap_or_else(|_| "api".into()),
        api_endpoint: env::var("OZONE_API_ENDPOINT").ok(),
        api_key_env: Some(env::var("OZONE_API_KEY_ENV").unwrap_or_else(|_| "ANTHROPIC_API_KEY".into())),
        api_model: env::var("OZONE_API_MODEL").ok(),
        local_model_path: env::var("OZONE_LOCAL_MODEL_PATH").ok(),
        context_length: env::var("OZONE_CONTEXT_LENGTH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100000),
        gpu_layers: env::var("OZONE_GPU_LAYERS")
            .ok()
            .and_then(|s| s.parse().ok()),
    };
    
    // Execute
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(execute(input, &config));
    
    // Output result as JSON
    match result {
        Ok(output) => {
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        Err(e) => {
            let error_output = serde_json::json!({
                "error": e,
                "success": false
            });
            println!("{}", error_output);
            std::process::exit(1);
        }
    }
}
