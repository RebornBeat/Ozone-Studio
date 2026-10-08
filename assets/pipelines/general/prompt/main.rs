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

/// Run a local child process with a hard bound. The host sets
/// OZONE_PIPELINE_TIMEOUT_SECS a margin below its own kill of this process, so
/// the child dies first and cannot be left orphaned. Stdout and stderr drain on
/// their own threads so a large write cannot block on a full pipe. A kill is an
/// error, never partial output.
fn run_bounded_child(cmd: &mut std::process::Command) -> std::io::Result<std::process::Output> {
    use std::io::Read;
    use std::process::Stdio;
    let secs = env::var("OZONE_PIPELINE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(285)
        .max(1);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let (out_tx, out_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let mut out_pipe = child.stdout.take();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = out_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        let _ = out_tx.send(buf);
    });
    let (err_tx, err_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let mut err_pipe = child.stderr.take();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = err_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        let _ = err_tx.send(buf);
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("local child process killed after {secs} seconds (time bound)"),
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let collect = |rx: &std::sync::mpsc::Receiver<Vec<u8>>| {
        rx.recv_timeout(std::time::Duration::from_secs(5)).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "pipe still open 5 seconds after the child exited; a grandchild process may hold it",
            )
        })
    };
    Ok(std::process::Output {
        status,
        stdout: collect(&out_rx)?,
        stderr: collect(&err_rx)?,
    })
}

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
    /// The CANDIDATE model's real context window, sent by the host's
    /// fallback walk per attempt (orchestrator/mod.rs). Without this field
    /// the override silently dropped it, so the pipeline sized budgets and
    /// truncation checks against the BASE config's window even when the
    /// walk had routed the call to a much smaller local model.
    #[serde(default)]
    pub context_length: Option<usize>,
    pub api_endpoint: Option<String>,
    pub api_key_env: Option<String>,
    pub api_key: Option<String>,
    pub wire_protocol: Option<String>,
    pub bitnet_cli_path: Option<String>,
    pub local_model_path: Option<String>,
}

/// Pipeline output
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// OpenAI-compatible path only: diagnostics read from the provider's own
    /// response fields, so an empty answer can be told apart as a provider
    /// fault or as our own output cap. Absent (not false) on other paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_null: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_reasoning: Option<bool>,
    /// Which response field carried the reasoning text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_field: Option<String>,
    /// The reasoning text itself, when present and non-empty — distinct from
    /// `reasoning_field` (which only names the key it came from). Lets a
    /// caller salvage a drafted answer when `response` is empty because our
    /// own cap cut generation short during reasoning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    /// The max_tokens value this call sent, or absent if none was sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens_sent: Option<u32>,
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
    if let Some(cl) = over.context_length {
        if cl > 0 {
            merged.context_length = cl;
        }
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
/// Error text for a non-success HTTP status, shaped "HTTP <code>: <message>".
/// The message is the provider's error.message when it has one, else the raw
/// body. Cut to 300 characters with an explicit marker, so the host reads the
/// status and the cause from one string.
fn http_error_text(status: reqwest::StatusCode, body: &str) -> String {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v["error"]["message"]
                .as_str()
                .or_else(|| v["error"].as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| body.trim().to_string());
    let total = message.chars().count();
    let shown: String = message.chars().take(300).collect();
    if total > 300 {
        format!("HTTP {}: {} ... [truncated, {} chars total]", status.as_u16(), shown, total)
    } else {
        format!("HTTP {}: {}", status.as_u16(), shown)
    }
}

async fn call_anthropic_api(
    endpoint: &str,
    api_key: &str,
    model: &str,
    input: &PromptInput,
) -> Result<PromptOutput, String> {
    // reqwest's default client has NO request timeout — a stalled
    // connection (dead peer, silent rate-limit, network black-hole) hung
    // this call forever with nothing surfacing to the caller. 120s is
    // generous for real generation latency while still bounding it.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    
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
        .map_err(|e| {
            if e.is_timeout() {
                format!("Request timed out (client timeout): {}", e)
            } else {
                format!("Request failed: {}", e)
            }
        })?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(http_error_text(status, &error_text));
    }
    
    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    // Extract response from Anthropic format. Search every content block
    // for the first one with type=="text" rather than blindly indexing 0 —
    // a response can carry non-text blocks first (e.g. a `thinking` block
    // under extended-thinking configs), which would silently read as empty
    // content despite the real answer sitting in a later block.
    let content = result["content"]
        .as_array()
        .and_then(|blocks| {
            blocks
                .iter()
                .find(|b| b["type"].as_str() == Some("text"))
        })
        .and_then(|b| b["text"].as_str())
        .unwrap_or("")
        .to_string();

    let tokens = result["usage"]["output_tokens"]
        .as_u64()
        .map(|t| t as u32);

    let finish_reason = result["stop_reason"]
        .as_str()
        .map(|s| s.to_string());

    // Diagnostic only, never changes behavior: when content still ends up
    // empty despite a successful (2xx) response, capture WHY so a future
    // debugging session doesn't have to re-derive it from scratch — the
    // caller-side retry/fallback (is_unusable_pipeline9_result and callers)
    // already handles the empty case correctly regardless of the reason.
    if content.trim().is_empty() {
        let block_types: Vec<String> = result["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .map(|b| b["type"].as_str().unwrap_or("?").to_string())
                    .collect()
            })
            .unwrap_or_default();
        eprintln!(
            "prompt pipeline: Anthropic response had no usable text content \
             (stop_reason={:?}, content block types={:?})",
            finish_reason, block_types
        );
    }

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
        ..Default::default()
    })
}

/// Call OpenAI-compatible API
async fn call_openai_api(
    endpoint: &str,
    api_key: &str,
    model: &str,
    input: &PromptInput,
) -> Result<PromptOutput, String> {
    // See call_anthropic_api's comment — same missing-timeout bug, same fix.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    
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
        .map_err(|e| {
            if e.is_timeout() {
                format!("Request timed out (client timeout): {}", e)
            } else {
                format!("Request failed: {}", e)
            }
        })?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(http_error_text(status, &error_text));
    }
    
    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    let message = &result["choices"][0]["message"];
    // content may be JSON null or absent; both mean "no content field".
    let content_null = message.get("content").map_or(true, |c| c.is_null());
    let content = message["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let tokens = result["usage"]["total_tokens"]
        .as_u64()
        .map(|t| t as u32);

    let finish_reason = result["choices"][0]["finish_reason"]
        .as_str()
        .map(|s| s.to_string());

    // Reasoning models carry their text in `reasoning` (OpenRouter) or
    // `reasoning_content` (other providers). Record which one carried it.
    let reasoning_field: Option<String> = ["reasoning", "reasoning_content"]
        .iter()
        .find(|f| message[**f].as_str().map_or(false, |s| !s.trim().is_empty()))
        .map(|f| f.to_string());
    // The actual reasoning text, not just which field carried it — a reasoning
    // model that exhausted our max_tokens cap still often drafted the real
    // answer here before being cut off (2026-10-08: this was previously
    // discarded after only computing reasoning_field/has_reasoning, so a
    // host-side salvage attempt reading for the text had nothing to read).
    let reasoning_text: Option<String> = reasoning_field
        .as_deref()
        .and_then(|f| message[f].as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());
    let has_reasoning = reasoning_field.is_some();
    let completion_tokens = result["usage"]["completion_tokens"].as_u64();
    let reasoning_tokens = result["usage"]["completion_tokens_details"]["reasoning_tokens"].as_u64();
    let max_tokens_sent = input.max_tokens;

    // Report the real model that actually served this call, not just the
    // requested identifier — this matters specifically for "auto" routing
    // (e.g. OpenRouter's "openrouter/auto"), where the caller genuinely
    // doesn't know in advance which underlying model will handle the
    // request. OpenRouter's response includes the resolved model at the
    // top level; without reading it back, model_used always echoed
    // "openrouter/auto" regardless of what actually ran, making any
    // per-call model switch invisible even though it was really happening.
    let actual_model = result["model"].as_str().unwrap_or(model).to_string();

    // Diagnostic only, never changes behavior: the host decides retry and
    // fallback from the Ok result. These lines say WHY the content is empty,
    // from the provider's own fields.
    if content.trim().is_empty() {
        let message_preview = serde_json::to_string(message).unwrap_or_default();
        let message_preview: String = message_preview.chars().take(300).collect();
        eprintln!(
            "prompt pipeline: OpenAI-compatible response had no usable content \
             (finish_reason={:?}, message={})",
            finish_reason, message_preview
        );
        let absent = || "absent".to_string();
        eprintln!(
            "prompt pipeline: empty response model={} served={} finish_reason={} content_null={} has_reasoning={} completion_tokens={} reasoning_tokens={} max_tokens_sent={}",
            model,
            actual_model,
            finish_reason.as_deref().unwrap_or("absent"),
            content_null,
            has_reasoning,
            completion_tokens.map_or_else(absent, |v| v.to_string()),
            reasoning_tokens.map_or_else(absent, |v| v.to_string()),
            max_tokens_sent.map_or_else(absent, |v| v.to_string()),
        );
    }

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
        content_null: Some(content_null),
        has_reasoning: Some(has_reasoning),
        reasoning_field,
        reasoning_text,
        completion_tokens,
        reasoning_tokens,
        max_tokens_sent,
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
    let output = run_bounded_child(std::process::Command::new(&llama_cli)
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
        );
    
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
                    ..Default::default()
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
    
    let output = run_bounded_child(std::process::Command::new("python3")
        .args(["-c", &python_script])
        );
    
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
                    ..Default::default()
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
/// KEEP-WARM BITNET via llama-server (throughput lever #2, guide §10 —
/// the per-call `llama-cli` spawn pays ~25-40s model load EVERY call;
/// measured: 64.1s cold → 38.0s page-cache warm → target: sub-2s warm
/// server completions). When OZONE_LLAMA_SERVER_URL is set AND the server
/// health-checks OK, generation goes over HTTP to the persistent server
/// (OpenAI-compatible /v1/chat/completions) instead of spawning a fresh
/// CLI. Falls through to the spawn path on any server problem — the
/// warm server is an optimization, never a dependency. Launch story
/// (operator/harness): llama-server -m <model> -c 8192 --host 127.0.0.1
/// --port 8081 — the same BitNet i2_s model file the CLI path uses.
async fn execute_bitnet_server(
    input: PromptInput,
    config: &ModelConfig,
    server_url: &str,
) -> Option<Result<PromptOutput, String>> {
    let health = reqwest::Client::new()
        .get(format!("{server_url}/health"))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await;
    if health.as_ref().map(|r| !r.status().is_success()).unwrap_or(true) {
        return None; // server not up — caller falls back to the spawn path
    }

    let effective_ctx = config.context_length.min(8192);
    let max_tokens = input
        .max_tokens
        .unwrap_or(512)
        .min((effective_ctx / 2) as u32)
        .to_string();
    let temp = input.temperature.unwrap_or(0.7).to_string();
    let prompt = build_prompt(&input);

    let body = serde_json::json!({
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens.parse::<u64>().unwrap_or(512),
        "temperature": input.temperature.unwrap_or(0.7),
    });
    let resp = reqwest::Client::new()
        .post(format!("{server_url}/v1/chat/completions"))
        .json(&body)
        .timeout(std::time::Duration::from_secs(300))
        .send()
        .await
        .ok()?;
    let v: serde_json::Value = resp.json().await.ok()?;
    let content = v["choices"][0]["message"]["content"].as_str()?.to_string();
    let tokens_used = v["usage"]["completion_tokens"].as_u64().map(|t| t as u32);
    let prompt_tokens = v["usage"]["prompt_tokens"].as_u64().map(|t| t as u32);

    Some(Ok(PromptOutput {
        response: content,
        model_used: format!("bitnet-server:{}", config.local_model_path.as_deref().unwrap_or("bitnet")),
        tokens_used,
        finish_reason: v["choices"][0]["finish_reason"].as_str().map(String::from),
        prompt_tokens,
        context_truncated: None,
        eval_tokens_per_sec: None,
        prompt_eval_tokens_per_sec: None,
        load_time_ms: Some(0.0), // warm server: zero load
        total_time_ms: None,
        ..Default::default()
    }))
}

async fn execute_bitnet(input: PromptInput, config: &ModelConfig) -> Result<PromptOutput, String> {
    // KEEP-WARM path (guide §10 lever #2): warm llama-server first, spawn
    // fallback second. Measured motivation: cold spawn = 25-40s load every
    // call; a persistent server makes short completions ~10-30x faster.
    if let Some(url) = std::env::var("OZONE_LLAMA_SERVER_URL").ok().filter(|s| !s.is_empty()) {
        if let Some(result) = execute_bitnet_server(input.clone(), config, &url).await {
            return result;
        }
        eprintln!("BitNet keep-warm server unavailable at {url} — falling back to CLI spawn");
    }

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
        run_bounded_child(std::process::Command::new(&clone_cli)
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
            )
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
                    ..Default::default()
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
