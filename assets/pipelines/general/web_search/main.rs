//! OZONE Studio - Web Search Pipeline (ID: 56)
//!
//! Real web search via a configured external provider (Brave Search API by
//! default — a simple API-key-header REST call, no OAuth dance, generous
//! free tier, well-documented JSON shape). ALSO provides real current
//! date/time, which needs zero configuration and is always available.
//!
//! HONESTY CONTRACT (mirrors this codebase's established convention for
//! BitNet tok/s — assets/pipelines/general/prompt/main.rs's
//! parse_bitnet_metrics — and voice confidence — assets/pipelines/general/
//! voice/main.rs: real value or None, NEVER a fabricated stand-in):
//! when no provider is configured/enabled, or the real HTTP call fails,
//! this pipeline returns success:false with a plain "unavailable" error —
//! it never invents plausible-sounding search results. A caller (the
//! orchestrator, or ultimately an end user) must be able to trust that
//! every `SearchResult` in a non-empty `results` list came from a real
//! HTTP response.
//!
//! AMT-style decomposition of multi-part queries is deliberately NOT done
//! in this pipeline — pipelines in this codebase are dumb, independent
//! units; only the orchestrator makes LLM calls to decide/split work (see
//! src/orchestrator/amt.rs's build_amt_layer_by_layer for the established
//! pattern). Query decomposition lives in the orchestrator
//! (execute_web_search_step in src/orchestrator/stages.rs), which calls
//! this pipeline once per real sub-query and synthesizes. This keeps this
//! pipeline simple and keeps the "does this need splitting" judgment where
//! every other such judgment already lives.

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSearchAction {
    Search {
        query: String,
        #[serde(default = "default_max_results")]
        max_results: u32,
    },
    CurrentDateTime,
}

fn default_max_results() -> u32 {
    5
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSearchInput {
    pub action: WebSearchAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WebSearchOutput {
    pub success: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub results: Vec<SearchResult>,
    pub provider_used: Option<String>,
    pub current_datetime_utc: Option<String>,
}

fn env_flag(name: &str) -> bool {
    env::var(name).map(|v| v == "true").unwrap_or(false)
}

/// Real Brave Search API call. https://api.search.brave.com — API-key auth
/// via the `X-Subscription-Token` header, plain GET with a `q` query param.
/// Only wire format actually implemented today; OZONE_WEB_SEARCH_PROVIDER
/// exists for future providers but anything other than "brave" currently
/// falls through to the same honest "unavailable" path as no provider at
/// all, rather than silently guessing at a different API shape.
async fn search_brave(query: &str, max_results: u32, endpoint: &str, api_key: &str) -> WebSearchOutput {
    let client = reqwest::Client::new();
    let resp = match client
        .get(endpoint)
        .header("Accept", "application/json")
        .header("X-Subscription-Token", api_key)
        .query(&[("q", query), ("count", &max_results.to_string())])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return WebSearchOutput {
                success: false,
                error: Some(format!("Web search request failed: {}", e)),
                ..Default::default()
            };
        }
    };

    if !resp.status().is_success() {
        return WebSearchOutput {
            success: false,
            error: Some(format!("Web search provider returned HTTP {}", resp.status())),
            ..Default::default()
        };
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return WebSearchOutput {
                success: false,
                error: Some(format!("Web search response parse failed: {}", e)),
                ..Default::default()
            };
        }
    };

    // Brave's real response shape: {"web": {"results": [{"title","url","description"}, ...]}}
    let results: Vec<SearchResult> = body
        .get("web")
        .and_then(|w| w.get("results"))
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    Some(SearchResult {
                        title: item.get("title")?.as_str()?.to_string(),
                        url: item.get("url")?.as_str()?.to_string(),
                        snippet: item
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .take(max_results as usize)
                .collect()
        })
        .unwrap_or_default();

    WebSearchOutput {
        success: true,
        error: None,
        results,
        provider_used: Some("brave".to_string()),
        current_datetime_utc: None,
    }
}

async fn execute(input: WebSearchInput) -> WebSearchOutput {
    match input.action {
        WebSearchAction::CurrentDateTime => WebSearchOutput {
            success: true,
            error: None,
            results: Vec::new(),
            provider_used: None,
            current_datetime_utc: Some(chrono::Utc::now().to_rfc3339()),
        },
        WebSearchAction::Search { query, max_results } => {
            let enabled = env_flag("OZONE_WEB_SEARCH_ENABLED");
            let provider = env::var("OZONE_WEB_SEARCH_PROVIDER").unwrap_or_else(|_| "brave".to_string());
            let endpoint = env::var("OZONE_WEB_SEARCH_ENDPOINT")
                .unwrap_or_else(|_| "https://api.search.brave.com/res/v1/web/search".to_string());
            let api_key_env = env::var("OZONE_WEB_SEARCH_API_KEY_ENV")
                .unwrap_or_else(|_| "BRAVE_SEARCH_API_KEY".to_string());
            let api_key = env::var(&api_key_env).ok();

            if !enabled {
                return WebSearchOutput {
                    success: false,
                    error: Some("Web search unavailable — disabled in config (web_search.enabled = false)".to_string()),
                    ..Default::default()
                };
            }
            let Some(api_key) = api_key.filter(|k| !k.is_empty()) else {
                return WebSearchOutput {
                    success: false,
                    error: Some(format!(
                        "Web search unavailable — no API key found in ${} (set the real key there, or web_search.api_key_env to point at whichever env var holds it)",
                        api_key_env
                    )),
                    ..Default::default()
                };
            };

            if provider != "brave" {
                return WebSearchOutput {
                    success: false,
                    error: Some(format!(
                        "Web search unavailable — provider '{}' is configured but this pipeline only implements the 'brave' wire format today",
                        provider
                    )),
                    ..Default::default()
                };
            }

            search_brave(&query, max_results, &endpoint, &api_key).await
        }
    }
}

fn parse_cli_input() -> Result<WebSearchInput, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--input" && i + 1 < args.len() {
            input_json = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }
    let raw = input_json.ok_or("no --input provided")?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    // The host wraps every real invocation as {"data": ..., "context": ...}
    // (see PipelineExecutor::invoke_pipeline) — unwrap it, same convention
    // every other pipeline's main() already follows.
    let inner = v.get("data").cloned().unwrap_or(v);
    serde_json::from_value(inner).map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() {
    let input = match parse_cli_input() {
        Ok(i) => i,
        Err(e) => {
            let out = WebSearchOutput {
                success: false,
                error: Some(format!("Failed to parse input: {}", e)),
                ..Default::default()
            };
            println!("{}", serde_json::to_string(&out).unwrap_or_default());
            return;
        }
    };
    let output = execute(input).await;
    println!("{}", serde_json::to_string(&output).unwrap_or_default());
}
