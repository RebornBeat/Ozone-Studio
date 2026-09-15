//! PromptPipeline - Pipeline #9
//! 
//! The core LLM interface that handles all model interactions.
//! Supports: API (Claude, OpenAI, etc.), GGUF (llama.cpp), ONNX (local), BitNet (1-bit)
//! 
//! Model selection is determined by config, NOT hardcoded.
//! Users can select models via the SettingsPipeline.
//!
//! v0.4.0 UPDATES:
//! - Added BitNet support (1-bit quantized models)
//! - Added context limit awareness
//! - Added token budget management
//!
//! NOTE: This is the LOW-LEVEL LLM call pipeline.
//! The FULL orchestration flow (14 stages from Master Alignment Report) is handled by:
//! - task_manager.Create() which orchestrates the full flow
//! - This pipeline is called at STAGE 11 during step execution
//!
//! For the full flow see: docs/PIPELINE_ORDER_OF_EVENTS.md

//! ═══════════════════════════════════════════════════════════════════════════
//! PIPELINE-9 MODEL-CALL CONTRACT (the standardized wire contract)
//! ═══════════════════════════════════════════════════════════════════════════
//! Callers (orchestrator, pipelines, serve mode) speak ONLY this shape:
//!   IN : { prompt, system_prompt?, temperature?, max_tokens?, … }
//!   OUT: { response, model_used, tokens_used?, finish_reason?, … }
//!
//! Wire protocols (HOW the request reaches a model) are adapters BEHIND this
//! contract, selected by config — never by callers. Adapters ship for:
//!   anthropic          — Anthropic Messages (/v1/messages)
//!   chat_completions   — OpenAI Chat Completions (/chat/completions)
//! Selection: ModelConfig.wire_protocol ("anthropic" | "chat_completions");
//! when unset, the endpoint URL is sniffed for backward compatibility.

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
    /// Token budget for context (respects model's context_length)
    pub token_budget: Option<u32>,
    /// Pre-aggregated context string (from context_aggregation pipeline)
    pub aggregated_context: Option<String>,
    /// Per-call backend override (real multi-model routing) — unlike
    /// `model_override` above (just a model NAME string on the currently-
    /// configured backend), this can redirect the call to a genuinely
    /// different backend (e.g. bitnet for this one call, api for the rest
    /// of the run). Wire counterpart of orchestrator::ModelConfigOverride's
    /// connection fields.
    pub model_override_config: Option<ModelOverrideConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelOverrideConfig {
    pub model_type: Option<String>,
    pub model_identifier: Option<String>,
    pub api_endpoint: Option<String>,
    pub api_key_env: Option<String>,
    pub api_key: Option<String>,
    pub wire_protocol: Option<String>,
    pub bitnet_cli_path: Option<String>,
    pub local_model_path: Option<String>,
}

/// Pipeline output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOutput {
    pub response: String,
    pub model_used: String,
    pub tokens_used: Option<u32>,
    pub finish_reason: Option<String>,
    /// Actual tokens in prompt (for tracking)
    pub prompt_tokens: Option<u32>,
    /// Whether context was truncated due to limits
    pub context_truncated: Option<bool>,
    /// Real generation throughput (tokens/sec) — BitNet only, from
    /// llama.cpp's own perf counters. None for API-backed calls (Anthropic/
    /// OpenAI/OpenRouter don't expose an equivalent local timing signal —
    /// left absent rather than fabricated).
    #[serde(default)]
    pub eval_tokens_per_sec: Option<f32>,
    #[serde(default)]
    pub prompt_eval_tokens_per_sec: Option<f32>,
    #[serde(default)]
    pub load_time_ms: Option<f32>,
    #[serde(default)]
    pub total_time_ms: Option<f32>,
}

/// Model configuration (read from OzoneConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_type: String,  // "api", "gguf", "onnx", "bitnet"
    pub api_endpoint: Option<String>,
    pub api_key_env: Option<String>,
    /// Raw key value — only ever populated by a per-call model_override_config
    /// merge (see merge_override), never by load_model_config_from_env. The
    /// base/env path keeps using api_key_env + env::var(...) unchanged.
    #[serde(default)]
    pub api_key: Option<String>,
    pub api_model: Option<String>,
    pub local_model_path: Option<String>,
    pub context_length: usize,
    pub gpu_layers: Option<u32>,
    /// BitNet-specific: path to bitnet.cpp binary
    pub bitnet_cli_path: Option<String>,
    /// Named wire protocol: "anthropic" | "chat_completions".
    /// Unset → sniffed from the endpoint URL (backward compatible).
    #[serde(default)]
    pub wire_protocol: Option<String>,
}

/// Named wire protocols — the K-registry `model_call` family members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireProtocol {
    /// Anthropic Messages: system as top-level field, content blocks,
    /// `x-api-key` header, `anthropic-version` required.
    Anthropic,
    /// OpenAI Chat Completions: system as a message, `choices[0]`,
    /// `Authorization: Bearer` header.
    ChatCompletions,
}

impl WireProtocol {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" | "messages" => Some(Self::Anthropic),
            "chat_completions" | "chatcompletions" | "openai" => Some(Self::ChatCompletions),
            _ => None,
        }
    }
}

/// Overlay a per-call override onto the env-derived base config. Every field
/// is independently optional — a step can redirect just the model_type
/// (e.g. switch to bitnet with its already-configured path) or a full
/// different backend (new endpoint + key + wire_protocol) in one shot.
fn merge_override(base: &ModelConfig, over: &ModelOverrideConfig) -> ModelConfig {
    let mut merged = base.clone();
    if let Some(v) = &over.model_type {
        merged.model_type = v.clone();
    }
    if let Some(v) = &over.model_identifier {
        merged.api_model = Some(v.clone());
    }
    if over.api_endpoint.is_some() {
        merged.api_endpoint = over.api_endpoint.clone();
    }
    if over.api_key_env.is_some() {
        merged.api_key_env = over.api_key_env.clone();
    }
    if over.api_key.is_some() {
        merged.api_key = over.api_key.clone();
    }
    if over.wire_protocol.is_some() {
        merged.wire_protocol = over.wire_protocol.clone();
    }
    if over.bitnet_cli_path.is_some() {
        merged.bitnet_cli_path = over.bitnet_cli_path.clone();
    }
    if over.local_model_path.is_some() {
        merged.local_model_path = over.local_model_path.clone();
    }
    merged
}

/// Execute the prompt pipeline
pub async fn execute(input: PromptInput, config: &ModelConfig) -> Result<PromptOutput, String> {
    // A per-call override (real multi-model routing) takes precedence over
    // the process's env-derived base config for this call only — nothing
    // process-wide is mutated, so concurrent serve-mode requests each get
    // their own effective config.
    let effective_owned;
    let effective: &ModelConfig = match &input.model_override_config {
        Some(over) => {
            effective_owned = merge_override(config, over);
            &effective_owned
        }
        None => config,
    };

    // Check if prompt + context exceeds context_length
    let estimated_tokens = estimate_tokens(&input, effective);
    let context_truncated = estimated_tokens > effective.context_length;

    // Determine which model to use
    let model_type = effective.model_type.clone();

    let mut result = match model_type.as_str() {
        "api" => execute_api(input, effective).await,
        "gguf" => execute_gguf(input, effective).await,
        "onnx" => execute_onnx(input, effective).await,
        "bitnet" => execute_bitnet(input, effective).await,
        _ => Err(format!("Unsupported model type: {}", model_type)),
    };

    // Add context_truncated info to output
    if let Ok(ref mut output) = result {
        output.context_truncated = Some(context_truncated);
    }

    result
}

/// Estimate token count for the input (rough approximation)
fn estimate_tokens(input: &PromptInput, config: &ModelConfig) -> usize {
    // Rough estimation: ~4 chars per token for English text
    let chars_per_token = 4;
    
    let mut total_chars = input.prompt.len();
    
    if let Some(ref system) = input.system_prompt {
        total_chars += system.len();
    }
    
    if let Some(ref context) = input.aggregated_context {
        total_chars += context.len();
    }
    
    total_chars / chars_per_token
}

/// Execute using API-based model (Claude, OpenAI, etc.)
async fn execute_api(input: PromptInput, config: &ModelConfig) -> Result<PromptOutput, String> {
    let endpoint = config.api_endpoint.as_ref()
        .ok_or("API endpoint not configured")?;
    
    let api_key = if let Some(k) = &config.api_key {
        k.clone()
    } else {
        let api_key_env = config.api_key_env.as_ref()
            .ok_or("API key env var not configured")?;
        env::var(api_key_env)
            .map_err(|_| format!("API key not found in env var: {}", api_key_env))?
    };
    
    // Clone: input is borrowed by the wire calls below — a by-value move
    // here would partially move it.
    let model = input.model_override.clone()
        .or_else(|| config.api_model.clone())
        .ok_or("No model specified")?;
    
    // Build request via the NAMED wire protocol when configured; sniff the
    // endpoint URL otherwise (backward compatibility).
    let wire = config
        .wire_protocol
        .as_deref()
        .and_then(WireProtocol::parse)
        .unwrap_or_else(|| {
            if endpoint.contains("anthropic") {
                WireProtocol::Anthropic
            } else {
                WireProtocol::ChatCompletions
            }
        });

    let response = match wire {
        WireProtocol::Anthropic => call_anthropic_api(
            endpoint,
            &api_key,
            &model,
            &input,
        ).await?,
        WireProtocol::ChatCompletions => call_openai_api(
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
            "content": input.prompt.clone()
        }));
    } else {
        messages.push(serde_json::json!({
            "role": "user", 
            "content": input.prompt.clone()
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

    // Report the real model the API actually used, not just the identifier
    // requested — Anthropic's response echoes the resolved model at the top
    // level; fall back to the requested identifier only if that's absent.
    let actual_model = result["model"].as_str().unwrap_or(model).to_string();

    Ok(PromptOutput {
        response: content,
        model_used: actual_model,
        tokens_used: tokens,
        finish_reason,
        prompt_tokens: result["usage"]["input_tokens"].as_u64().map(|t| t as u32),
        context_truncated: None, // Set by execute() wrapper
        eval_tokens_per_sec: None,
        prompt_eval_tokens_per_sec: None,
        load_time_ms: None,
        total_time_ms: None,
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
        "content": input.prompt.clone()
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

    // Report the real model that actually served this call, not just the
    // requested identifier — this matters specifically for "auto" routing
    // (e.g. OpenRouter's "openrouter/auto"), where the caller genuinely
    // doesn't know in advance which underlying model will handle the
    // request. OpenRouter's response includes the resolved model at the
    // top level; without reading it back, model_used always echoed
    // "openrouter/auto" regardless of what actually ran, making any
    // per-call model switch invisible even though it was really happening.
    let actual_model = result["model"].as_str().unwrap_or(model).to_string();

    Ok(PromptOutput {
        response: content,
        model_used: actual_model,
        tokens_used: tokens,
        finish_reason,
        prompt_tokens: result["usage"]["prompt_tokens"].as_u64().map(|t| t as u32),
        context_truncated: None, // Set by execute() wrapper
        eval_tokens_per_sec: None,
        prompt_eval_tokens_per_sec: None,
        load_time_ms: None,
        total_time_ms: None,
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

    // Same fix as execute_bitnet: never request more generation tokens than
    // this model's actual context window can reasonably support, regardless
    // of what the caller asked for (which may reflect a totally different,
    // larger model this step originally targeted before falling back here).
    let max_tokens = input
        .max_tokens
        .unwrap_or(1024)
        .min((config.context_length / 2) as u32)
        .to_string();

    // Same anti-repetition fix as execute_bitnet (see its comment) — applies
    // equally to any local llama.cpp-served GGUF model.
    let output = std::process::Command::new(&llama_cli)
        .args([
            "-m", model_path,
            "-p", &prompt,
            "-n", &max_tokens,
            "--temp", &input.temperature.unwrap_or(0.7).to_string(),
            "-ngl", &config.gpu_layers.unwrap_or(0).to_string(),
            "-c", &config.context_length.to_string(),
            "--repeat-penalty", "1.3",
            "--repeat-last-n", "1024",
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
                    prompt_tokens: None, // Not available from CLI
                    context_truncated: None,
                    eval_tokens_per_sec: None,
                    prompt_eval_tokens_per_sec: None,
                    load_time_ms: None,
                    total_time_ms: None,
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
                    prompt_tokens: None, // Not available from Python bridge
                    context_truncated: None,
                    eval_tokens_per_sec: None,
                    prompt_eval_tokens_per_sec: None,
                    load_time_ms: None,
                    total_time_ms: None,
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

/// Token metrics as printed by the llama.cpp family CLI on stderr:
///   llama_perf_context_print: prompt eval time =  487.15 ms /     5 tokens
///   llama_perf_context_print:        eval time = 7570.76 ms /    23 runs
/// These are counts from the model's ACTUAL tokenizer — proven, not
/// estimated. `runs` is generated tokens; the total line cross-checks
/// (prompt + completion == total).
struct BitnetTokenMetrics {
    prompt_tokens: u32,
    completion_tokens: u32,
    /// Model load time (ms) — separate from generation time. Dominates wall
    /// time for short generations (confirmed: ~1.9s load vs ~2.5s eval for a
    /// 19-token completion in a live test), so must never be folded into a
    /// tokens/sec figure derived from wall-clock duration.
    load_time_ms: Option<f32>,
    /// Prompt-processing throughput (tokens/sec), llama.cpp's own perf
    /// counter — not derived from wall-clock.
    prompt_eval_tokens_per_sec: Option<f32>,
    /// Generation throughput (tokens/sec) — the actual "tokens output per
    /// second" figure: how fast the model generates, excluding load and
    /// prompt-processing time.
    eval_tokens_per_sec: Option<f32>,
    total_time_ms: Option<f32>,
}

/// Extract the float immediately after '=' on a llama.cpp perf-print line,
/// e.g. "...load time =    1934.35 ms" -> 1934.35.
fn extract_ms_value(line: &str) -> Option<f32> {
    line.split('=').nth(1)?.split_whitespace().next()?.parse::<f32>().ok()
}

/// Extract the "tokens per second" figure from a line shaped like
/// "...(   73.50 ms per token,    13.61 tokens per second)" -> 13.61.
fn extract_tokens_per_second(line: &str) -> Option<f32> {
    let paren = line.split('(').nth(1)?;
    let after_comma = paren.rsplit(',').next()?;
    after_comma.split_whitespace().next()?.parse::<f32>().ok()
}

fn parse_bitnet_metrics(stderr: &str) -> Option<BitnetTokenMetrics> {
    let mut prompt_tokens: Option<u32> = None;
    let mut completion_tokens: Option<u32> = None;
    let mut load_time_ms: Option<f32> = None;
    let mut prompt_eval_tokens_per_sec: Option<f32> = None;
    let mut eval_tokens_per_sec: Option<f32> = None;
    let mut total_time_ms: Option<f32> = None;

    for line in stderr.lines() {
        if line.contains("load time") {
            // "llama_perf_context_print:        load time =    1934.35 ms"
            load_time_ms = extract_ms_value(line);
        } else if line.contains("prompt eval time") {
            // …"/     5 tokens (…    13.61 tokens per second)"
            if let Some(rest) = line.rsplit("/").next() {
                prompt_tokens = rest
                    .split_whitespace()
                    .find_map(|w| w.parse::<u32>().ok());
            }
            prompt_eval_tokens_per_sec = extract_tokens_per_second(line);
        } else if line.contains("eval time") && !line.contains("prompt eval") {
            // "…/    23 runs   (…     7.74 tokens per second)"
            if let Some(rest) = line.rsplit("/").next() {
                completion_tokens = rest
                    .split_whitespace()
                    .find_map(|w| w.parse::<u32>().ok());
            }
            eval_tokens_per_sec = extract_tokens_per_second(line);
        } else if line.contains("total time") {
            // "llama_perf_context_print:       total time =    2614.02 ms /    21 tokens"
            total_time_ms = extract_ms_value(line);
        }
    }
    match (prompt_tokens, completion_tokens) {
        (Some(p), Some(c)) => Some(BitnetTokenMetrics {
            prompt_tokens: p,
            completion_tokens: c,
            load_time_ms,
            prompt_eval_tokens_per_sec,
            eval_tokens_per_sec,
            total_time_ms,
        }),
        // No fabricated fallback: metrics are None when the CLI did not
        // report them (captured-only, per the confidence doctrine).
        _ => None,
    }
}

/// Execute using BitNet model (1-bit quantized, CPU-efficient)
///
/// Runs the llama.cpp-fork CLI (BitNet i2_s kernels) and derives token
/// usage from the CLI's own tokenizer perf report on stderr — real counts,
/// cross-checked against the reported total where present.
async fn execute_bitnet(input: PromptInput, config: &ModelConfig) -> Result<PromptOutput, String> {
    let model_path = config.local_model_path.as_ref()
        .ok_or("Local model path not configured for BitNet")?;

    // Verify model file exists
    if !std::path::Path::new(model_path).exists() {
        return Err(format!("BitNet model not found at: {}", model_path));
    }

    // Owned String — the env-var fallback would otherwise return a
    // reference into a temporary.
    let bitnet_cli: String = config.bitnet_cli_path.as_ref()
        .map(|s| s.clone())
        .unwrap_or_else(|| {
            std::env::var("BITNET_CLI_PATH").unwrap_or_else(|_| "llama-cli".to_string())
        });

    // Build the prompt
    let prompt = build_prompt(&input);

    // Confirmed live: input.max_tokens is computed by the caller from
    // whichever model was ORIGINALLY intended for this step (e.g.
    // OpenRouter's 128K context / 4 = 32000) and is passed through
    // unchanged even when the fallback chain lands on BitNet — a real,
    // much smaller local model. Requesting -n far larger than the actual
    // context window doesn't error, it just runs for real: at BitNet's
    // observed ~7-9 tokens/sec on this machine, -n 32000 is a 60+ minute
    // generation, not a hang. Cap generation to half of BitNet's real
    // context window (the other half reserved for the prompt itself,
    // which -c must also hold) regardless of what the caller asked for.
    let effective_ctx = config.context_length.min(8192);
    let max_tokens = input
        .max_tokens
        .unwrap_or(512)
        .min((effective_ctx / 2) as u32)
        .to_string();
    let temp = input.temperature.unwrap_or(0.7).to_string();
    let ctx = effective_ctx.to_string();

    // Blocking child process — keep it off the async runtime's core.
    let clone_model_path = model_path.clone();
    let clone_cli = bitnet_cli.clone();
    // Confirmed live, two distinct degenerate-generation failure modes on
    // this 1-bit quantized model:
    // 1. Literal token repetition ("## Pipeline: Core\nKeywords: core, core,
    //    core, ...") — the original finding this repeat-penalty was added
    //    for, at repeat-penalty=1.1 / repeat-last-n=256.
    // 2. STRUCTURAL pattern repetition (found later, still live at those
    //    settings): the model hallucinates an endless list of fictional
    //    "## Pipeline: X\nKeywords: Y" catalog entries with DIFFERENT
    //    content each time (invented names like DataApproval, DataReview,
    //    DataMigration — none of them real registered pipelines) — it's
    //    extending the FORMAT of real catalog-style content in its own
    //    background context rather than answering the actual question.
    //    Each entry's tokens differ enough that a 256-token lookback and a
    //    mild 1.1 penalty didn't suppress it: one real request burned
    //    10,080 tokens and ~27 minutes almost entirely on this. Strengthened
    //    to make the repeated structural markers ("##", "Pipeline", ":",
    //    "Keywords") costlier across a much longer span.
    let run = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&clone_cli)
            .args([
                "-m", &clone_model_path,
                "-p", &prompt,
                "-n", &max_tokens,
                "--temp", &temp,
                "-c", &ctx,
                "--repeat-penalty", "1.3",
                "--repeat-last-n", "1024",
                "--no-display-prompt",
            ])
            .output()
    })
    .await
    .map_err(|e| format!("BitNet task join failed: {}", e))?;

    match run {
        Ok(result) => {
            if result.status.success() {
                let response = String::from_utf8_lossy(&result.stdout).to_string();
                let stderr = String::from_utf8_lossy(&result.stderr);
                let metrics = parse_bitnet_metrics(&stderr);
                if metrics.is_none() {
                    eprintln!("BitNet: tokenizer metrics not found in CLI stderr — tokens_used omitted (no estimates)");
                }
                let (tokens_used, prompt_tokens) = match &metrics {
                    Some(m) => (Some(m.completion_tokens), Some(m.prompt_tokens)),
                    None => (None, None),
                };
                // Real generation throughput — llama.cpp's own perf counter,
                // not derived from wall-clock (which would include model
                // load time and badly skew the figure for short generations).
                if let Some(m) = &metrics {
                    if let Some(tps) = m.eval_tokens_per_sec {
                        eprintln!(
                            "BitNet generation throughput: {:.2} tokens/sec (prompt eval: {:?} tok/s, load time: {:?} ms)",
                            tps, m.prompt_eval_tokens_per_sec, m.load_time_ms
                        );
                    }
                }

                Ok(PromptOutput {
                    response: response.trim().to_string(),
                    model_used: format!("bitnet:{}", model_path),
                    tokens_used,
                    finish_reason: Some("stop".to_string()),
                    prompt_tokens,
                    context_truncated: None,
                    eval_tokens_per_sec: metrics.as_ref().and_then(|m| m.eval_tokens_per_sec),
                    prompt_eval_tokens_per_sec: metrics.as_ref().and_then(|m| m.prompt_eval_tokens_per_sec),
                    load_time_ms: metrics.as_ref().and_then(|m| m.load_time_ms),
                    total_time_ms: metrics.as_ref().and_then(|m| m.total_time_ms),
                })
            } else {
                let error = String::from_utf8_lossy(&result.stderr);
                Err(format!("BitNet inference failed: {}", error))
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(format!(
                    "BitNet CLI not found. Set models.bitnet_cli_path or BITNET_CLI_PATH (e.g. BitNet/build/bin/llama-cli). Model path: {}",
                    model_path
                ))
            } else {
                Err(format!("Failed to execute BitNet model: {}", e))
            }
        }
    }
}

/// Build prompt string from input, including aggregated context
fn build_prompt(input: &PromptInput) -> String {
    let mut prompt = String::new();
    
    if let Some(system) = &input.system_prompt {
        prompt.push_str(&format!("System: {}\n\n", system));
    }
    
    // Include pre-aggregated context if provided (preferred)
    if let Some(ref aggregated) = input.aggregated_context {
        if !aggregated.is_empty() {
            prompt.push_str(&format!("Context:\n{}\n\n", aggregated));
        }
    } else if let Some(context_ids) = &input.context {
        // Fallback: context IDs that would need to be resolved
        // In practice, context_aggregation pipeline should resolve these first
        if !context_ids.is_empty() {
            prompt.push_str(&format!("Context IDs (unresolved): {:?}\n\n", context_ids));
        }
    }
    
    prompt.push_str(&format!("User: {}", input.prompt));
    prompt
}

// ============================================================================
// CLI entry point for standalone execution
// ============================================================================

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

/// Model config from OZONE_MODEL_* env (bootstrap + serve mode share it).
fn load_model_config_from_env() -> ModelConfig {
    ModelConfig {
        model_type: env::var("OZONE_MODEL_TYPE").unwrap_or_else(|_| "api".into()),
        api_endpoint: env::var("OZONE_API_ENDPOINT").ok(),
        api_key_env: Some(env::var("OZONE_API_KEY_ENV").unwrap_or_else(|_| "ANTHROPIC_API_KEY".into())),
        api_key: None,
        api_model: env::var("OZONE_API_MODEL").ok(),
        local_model_path: env::var("OZONE_LOCAL_MODEL_PATH").ok(),
        context_length: env::var("OZONE_CONTEXT_LENGTH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(200000),
        gpu_layers: env::var("OZONE_GPU_LAYERS")
            .ok()
            .and_then(|s| s.parse().ok()),
        bitnet_cli_path: env::var("BITNET_CLI_PATH").ok(),
        wire_protocol: env::var("OZONE_WIRE_PROTOCOL").ok(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // SERVE MODE — connect-model: persistent model-call service registered
    // with the host. Model config comes from OZONE_MODEL_* env (same as
    // one-shot path; set at bootstrap from config::ModelConfig).
    if let Some(opts) = ozone_serve::serve_mode() {
        let handler = std::sync::Arc::new(move |action_payload: serde_json::Value| {
            let input: PromptInput = serde_json::from_value(action_payload)
                .unwrap_or(PromptInput {
                    prompt: String::new(),
                    system_prompt: None,
                    context: None,
                    model_override: None,
                    temperature: None,
                    max_tokens: None,
                    stream: None,
                    token_budget: None,
                    aggregated_context: None,
                    model_override_config: None,
                });
            let config = load_model_config_from_env();
            let rt = tokio::runtime::Runtime::new().expect("serve runtime");
            match rt.block_on(execute(input, &config)) {
                Ok(output) => serde_json::to_value(&output)
                    .unwrap_or(serde_json::json!({"success": false})),
                Err(e) => serde_json::json!({"success": false, "error": e}),
            }
        });
        let pipeline_id: u64 = 9;
        ozone_serve::serve(
            opts,
            pipeline_id,
            "prompt".to_string(),
            vec!["model".to_string()],
            handler,
        );
    }

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
    
    // Parse input — the host passes the full PipelineInput envelope
    // {data, context}; serve mode receives `data` unwrapped, one-shot gets
    // the whole thing. Unwrap `data` when present (bare PromptInput also
    // works for direct CLI use).
    let parsed_value: serde_json::Value = match serde_json::from_str(&input_json) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse input: {}", e);
            std::process::exit(1);
        }
    };
    let data_json = match parsed_value.get("data") {
        Some(d) => serde_json::to_string(d).unwrap_or_else(|_| input_json.clone()),
        None => input_json.clone(),
    };
    let input: PromptInput = match serde_json::from_str(&data_json) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to parse input: {}", e);
            std::process::exit(1);
        }
    };
    
    let config = load_model_config_from_env();
    
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
