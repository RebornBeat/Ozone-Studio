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
    /// Section S — step-scoped aggregation: container context (query-scoped)
    /// MERGED with the session's own text-graph reconstruction, session content
    /// first (highest priority), trimmed to the budget.
    ForStep {
        query: String,
        /// Session graph reconstruction (validated sentence text), pre-built
        /// by the orchestrator from its chunk graphs.
        session_context: Option<String>,
        token_budget: u32,
        project_id: Option<u64>,
        priority_order: Vec<String>,
        /// Global consciousness state (emotional state + relevant
        /// experiences) — a SEPARATE layer from the project-scoped container
        /// text above, carried in its own `consciousness_context` field
        /// rather than mixed into `context_text`, so project scoping stays
        /// clean regardless of whether this is set. Only meaningful when the
        /// orchestration request has consciousness_enabled=true; the caller
        /// (orchestrator) decides, this pipeline doesn't guess.
        #[serde(default)]
        include_consciousness: bool,
        /// Coordination layer (task 43) — agent notes/decisions/handoffs/
        /// claims from the /SharedContext graph, scoped to global + this
        /// workspace (+ project). Same separate-layer doctrine: carried in
        /// its own `coordination_context` field, never mixed into
        /// `context_text`. The caller decides; this pipeline doesn't guess.
        #[serde(default)]
        workspace_id: Option<u64>,
        #[serde(default)]
        include_coordination: bool,
    },
    /// Section S — reconstruct provided texts at a token limit, honoring
    /// sentence boundaries (greedy packing, ~4 chars/token). Session chunk
    /// data is passed IN — aggregation never reaches into session state.
    ReconstructAtLimit {
        texts: Vec<String>,
        token_budget: u32,
    },
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
    /// Coordination layer (task 43) — scoped agent notes/decisions from the
    /// /SharedContext graph. Separate from context_text by the same
    /// doctrine as consciousness_context.
    #[serde(default)]
    pub coordination_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource { 
    pub container_id: u64, 
    pub container_type: String, 
    pub name: String,
    pub relevance: f32, 
    pub tokens_used: u32,
    pub snippet: String,
    /// How this container was found: "keyword-scan", "traversal",
    /// "coordination", etc. — provenance for the context-gathering path.
    #[serde(default)]
    pub source: String,
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

/// Real ZSEI access — POSTs a properly-tagged ZSEIQuery to the host's
/// /zsei/query. Previously this pipeline read raw files directly off disk
/// with invented action names ("GetContainersByKeywords") that matched
/// nothing in the real ZSEIQuery enum, so every keyword-search-based action
/// (ForStep — the one real orchestration actually uses per step) silently
/// returned empty results on every call. ZSEIQuery has no #[serde(tag=...)],
/// so the wire format is externally-tagged: {"VariantName": {fields...}}.
fn ozone_host() -> String {
    env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/zsei/query", ozone_host()))
        .json(&serde_json::json!({"query": query, "session_token": ""}))
        .send()
        .await
        .map_err(|e| format!("zsei query request failed: {}", e))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("zsei query response parse failed: {}", e))?;
    if body.get("success").and_then(|s| s.as_bool()) != Some(true) {
        return Err(body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("zsei query failed")
            .to_string());
    }
    Ok(body.get("result").cloned().unwrap_or(serde_json::Value::Null))
}

/// Fetch one container's full content by id. Returns None on any failure
/// (not-found, host unreachable, etc.) — callers treat a miss as "no
/// context from this source" rather than aborting the whole aggregation.
async fn fetch_container(container_id: u64) -> Option<serde_json::Value> {
    let result = zsei_query(serde_json::json!({"GetContainer": {"container_id": container_id}}))
        .await
        .ok()?;
    result.get("Container").cloned()
}

/// Keyword search — the query real per-step context aggregation needs.
/// Returns container ids only; caller resolves each via fetch_container.
async fn search_containers_by_keywords(keywords: &[&str], container_type: Option<&str>) -> Vec<u64> {
    let result = match zsei_query(serde_json::json!({
        "SearchContainersByKeywords": {
            "keywords": keywords,
            "container_type": container_type,
        }
    }))
    .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    result
        .get("Containers")
        .and_then(|c| c.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default()
}

// Real graph traversal from keyword-search seeds — task 56 (confirmed live
// 2026-09-15: TraversalEngine, a real and substantial mechanism, had zero
// live callers anywhere in the orchestration request path; every context
// assembly instead used the flat search above and stopped there). Mirrors
// the exact working pattern proven tonight in
// src/orchestrator/jurisdiction.rs's load_jurisdiction_rules: seed search
// finds real starting points (unchanged, still a targeted lookup, not a
// type-blind scan), then a real TraversalMode::Structural request per seed
// walks the graph — structural_traversal (fixed earlier tonight) now
// follows BOTH parent/child structure AND real Relation edges, so this
// genuinely reaches connected-but-not-nested content, not just a tree
// walk. This is meaningfully more useful tonight than when task 56 was
// first scoped: text and code modality (assets/pipelines/modalities/
// {text,code}/main.rs) now write real bidirectional SimilarTo edges
// between genuinely related content whenever a graph is created, so a
// traversal from one matched container can now actually reach a second,
// genuinely related one that the flat keyword search alone would have
// missed.
//
// Deliberately bounded — this runs on every step of every live request,
// not a one-off background job (unlike jurisdiction's occasional gate
// check). Small max_depth/max_results per seed, and only the first
// MAX_TRAVERSAL_SEEDS keyword hits are ever traversed at all, so a step
// with many keyword matches can't fan out into an expensive traversal
// storm — resolve_containers' own MAX_RESOLVED_CONTAINERS cap (8) bounds
// the final result regardless, but bounding the traversal REQUEST itself
// (not just its output) keeps the host-side work itself cheap too.
const MAX_TRAVERSAL_SEEDS: usize = 4;
const TRAVERSAL_MAX_DEPTH: u16 = 2;
const TRAVERSAL_MAX_RESULTS: u32 = 10;

async fn traverse_from_seeds(seed_ids: &[u64]) -> Vec<u64> {
    let mut discovered: Vec<u64> = Vec::new();
    for &seed_id in seed_ids.iter().take(MAX_TRAVERSAL_SEEDS) {
        let request = serde_json::json!({
            "Traverse": {
                "start_container": seed_id,
                "mode": "Structural",
                "filters": [],
                "max_depth": TRAVERSAL_MAX_DEPTH,
                "max_results": TRAVERSAL_MAX_RESULTS,
                "budget": {
                    "max_hops": TRAVERSAL_MAX_DEPTH,
                    "max_containers": TRAVERSAL_MAX_RESULTS,
                    "max_latency_ms": 2000
                },
                "use_ml": false,
                "include_methodologies": false,
                "include_external_refs": false,
                "keyword_filter": null,
                "topic_filter": null
            }
        });
        let result = match zsei_query(request).await {
            Ok(r) => r,
            Err(_) => continue, // one seed's traversal failing shouldn't drop the others
        };
        // zsei_query returns the raw ZSEIQueryResult enum's wire shape —
        // {"TraversalResult": {containers: [...], ...}} — same
        // enum-tag-then-fields pattern fetch_container's "Container" and
        // search_containers_by_keywords' "Containers" already unwrap below.
        let containers = result
            .get("TraversalResult")
            .and_then(|t| t.get("containers"))
            .and_then(|c| c.as_array());
        if let Some(containers) = containers {
            for id in containers.iter().filter_map(|v| v.as_u64()) {
                if id != seed_id && !discovered.contains(&id) {
                    discovered.push(id);
                }
            }
        }
    }
    discovered
}

async fn fetch_project_context(project_id: u64) -> Option<serde_json::Value> {
    let result = zsei_query(serde_json::json!({"GetProjectContext": {"project_id": project_id}}))
        .await
        .ok()?;
    result.get("Container").cloned()
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

/// Section S primitive: greedily pack whole sentences from `texts` until
/// `token_budget` (~4 chars/token) is reached. Sentence boundaries honored —
/// no mid-sentence truncation.
fn reconstruct_texts_at_limit(texts: &[String], token_budget: u32) -> String {
    let budget_chars = (token_budget as usize).saturating_mul(4);
    let mut out = String::new();
    'outer: for text in texts {
        let mut sentences: Vec<String> = Vec::new();
        let mut current = String::new();
        for ch in text.chars() {
            current.push(ch);
            if matches!(ch, '.' | '!' | '?') && current.trim().len() > 1 {
                sentences.push(current.trim().to_string());
                current.clear();
            }
        }
        if !current.trim().is_empty() {
            sentences.push(current.trim().to_string());
        }
        for s in sentences {
            if out.len() + s.len() + 1 > budget_chars {
                break 'outer;
            }
            out.push_str(&s);
            out.push(' ');
        }
        out.push('\n');
    }
    out.trim().to_string()
}

/// Navigates the REAL Container shape (src/types/container.rs):
/// `{global_state: {container_id, ...}, local_state: {metadata: {container_type,
/// name}, context: {keywords}, file_context?, code_context?, text_context?}}`
/// — the previous version assumed a flat `{container_id, container_type,
/// name, content, semantic_summary, keywords}` shape that never matched any
/// real container ZSEI actually produces.
/// Container types that are system self-description/organizational
/// infrastructure, never real content — a registered pipeline's own name
/// and keywords, a structural root, a bare user/workspace/project node.
/// Confirmed live 2026-09-15: search_containers_by_keywords's untyped scan
/// (container_type: None) let these leak into step background context
/// whenever a query happened to share any keyword with a pipeline's own
/// generic self-description (e.g. "environment", "data") — a weak model
/// given a flood of "## Pipeline: X\nKeywords: Y" entries pattern-matched
/// the format and hallucinated more of them instead of answering the real
/// question. These types are never legitimate "background context" for a
/// user's question and are filtered out before context assembly, not just
/// at display time — this is the same fix regardless of which keyword
/// caused a given match, not a patch for one specific colliding keyword.
fn is_infrastructure_container_type(t: &str) -> bool {
    matches!(
        t,
        "Root" | "User" | "Workspace" | "Project"
            | "Pipeline"
            | "ModalityRoot" | "MethodologyRoot" | "BlueprintRoot" | "PipelineRoot"
            | "ConsciousnessRoot" | "ExternalRoot" | "PackageRoot"
            | "JurisdictionRoot"
    )
}

fn build_context_from_containers(containers: &[serde_json::Value], budget: u32) -> (String, Vec<ContextSource>, bool) {
    let mut context_parts: Vec<String> = Vec::new();
    let mut sources: Vec<ContextSource> = Vec::new();
    let mut total_tokens: u32 = 0;
    let mut truncated = false;

    for container in containers {
        let skip_type = container
            .get("local_state")
            .and_then(|l| l.get("metadata"))
            .and_then(|m| m.get("container_type"))
            .and_then(|t| t.as_str())
            .map(is_infrastructure_container_type)
            .unwrap_or(false);
        if skip_type {
            continue;
        }
        let global = container.get("global_state");
        let local = container.get("local_state");
        let container_id = global
            .and_then(|g| g.get("container_id"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0);
        let metadata = local.and_then(|l| l.get("metadata"));
        let container_type = metadata
            .and_then(|m| m.get("container_type"))
            .and_then(|t| t.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let name = metadata
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Unnamed")
            .to_string();
        let keywords: Vec<String> = local
            .and_then(|l| l.get("context"))
            .and_then(|c| c.get("keywords"))
            .and_then(|k| k.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let file_ctx = local.and_then(|l| l.get("file_context")).filter(|v| !v.is_null());
        let code_ctx = local.and_then(|l| l.get("code_context")).filter(|v| !v.is_null());
        let text_ctx = local.and_then(|l| l.get("text_context")).filter(|v| !v.is_null());

        let content = if let Some(file_ctx) = file_ctx {
            let path = file_ctx.get("file_path").and_then(|p| p.as_str()).unwrap_or(&name);
            let summary = file_ctx.get("semantic_summary").and_then(|s| s.as_str()).unwrap_or("");
            if !summary.is_empty() {
                format!("## File: {}\n{}\n\n", path, summary)
            } else {
                format!("## File: {}\nKeywords: {}\n\n", path, keywords.join(", "))
            }
        } else if let Some(code_ctx) = code_ctx {
            let functions: Vec<&str> = code_ctx
                .get("functions")
                .and_then(|f| f.as_array())
                .map(|arr| arr.iter().filter_map(|f| f.get("name").and_then(|n| n.as_str())).collect())
                .unwrap_or_default();
            let classes: Vec<&str> = code_ctx
                .get("classes")
                .and_then(|c| c.as_array())
                .map(|arr| arr.iter().filter_map(|c| c.get("name").and_then(|n| n.as_str())).collect())
                .unwrap_or_default();
            format!(
                "## Code: {}\nFunctions: {}\nClasses: {}\n\n",
                name,
                functions.join(", "),
                classes.join(", ")
            )
        } else if text_ctx.is_some() {
            format!("## Document: {}\nKeywords: {}\n\n", name, keywords.join(", "))
        } else {
            format!("## {}: {}\nKeywords: {}\n\n", container_type, name, keywords.join(", "))
        };

        let content_tokens = estimate_tokens(&content);
        if total_tokens + content_tokens > budget {
            truncated = true;
            let remaining = budget.saturating_sub(total_tokens);
            if remaining > 50 {
                let (trunc_content, _) = truncate_to_budget(&content, remaining);
                sources.push(ContextSource { container_id, container_type: container_type.to_string(), name: name.to_string(), relevance: 0.8, tokens_used: estimate_tokens(&trunc_content), snippet: trunc_content.chars().take(100).collect(), source: "keyword-scan".to_string() });
                context_parts.push(trunc_content);
            }
            break;
        }
        
        context_parts.push(content.clone());
        sources.push(ContextSource { container_id, container_type: container_type.to_string(), name: name.to_string(), relevance: 0.9, tokens_used: content_tokens, snippet: content.chars().take(100).collect(), source: "traversal".to_string() });
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

/// Resolve keyword-search hits into full container content, capped so one
/// step's context fetch can't fan out into an unbounded number of HTTP
/// round trips.
const MAX_RESOLVED_CONTAINERS: usize = 8;

async fn resolve_containers(ids: &[u64]) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for &id in ids.iter().take(MAX_RESOLVED_CONTAINERS) {
        if let Some(c) = fetch_container(id).await {
            out.push(c);
        }
    }
    out
}


/// Coordination layer (task 43): pull scoped agent events from the
/// /SharedContext graph — scope:global always, plus this workspace's (and
/// this project's) own events — newest first, hard-capped. Returns None
/// when nothing relevant exists (the layer is omitted, not empty-padded).
async fn build_coordination_layer(
    workspace_id: Option<u64>,
    project_id: Option<u64>,
) -> Option<String> {
    // Seed searches per scope bucket, unioned.
    let mut ids: Vec<u64> = Vec::new();
    for keywords in [
        vec!["scope:global"],
        match workspace_id {
            Some(ws) => vec![format!("ws:{}", ws).leak() as &str],
            None => vec![],
        },
        match (workspace_id, project_id) {
            (Some(ws), Some(p)) => vec![format!("proj:{}-{}", ws, p).leak() as &str],
            _ => vec![],
        },
    ] {
        if keywords.is_empty() {
            continue;
        }
        for id in search_containers_by_keywords(&keywords, Some("CoordinationEvent")).await {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    if ids.is_empty() {
        return None;
    }

    let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let containers = resolve_containers(&ids).await;
    let mut rows: Vec<(u64, String)> = Vec::new();
    for c in &containers {
        let meta = c.get("local_state").and_then(|l| l.get("metadata"));
        let name = meta
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let provenance = meta
            .and_then(|m| m.get("provenance"))
            .and_then(|p| p.as_str())
            .unwrap_or("unknown")
            .to_string();
        let created = meta
            .and_then(|m| m.get("created_at"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        // Body file (full context) via the container's own storage pointer.
        let ptr = c
            .get("local_state")
            .and_then(|l| l.get("storage"))
            .and_then(|s| s.get("object_store_path"))
            .and_then(|p| p.as_str())
            .map(String::from);
        let mut body = String::new();
        if let Some(ptr) = ptr {
            let full = if std::path::Path::new(&ptr).is_absolute() {
                ptr.clone()
            } else {
                format!("{}/{}", data_dir, ptr)
            };
            if let Ok(raw) = env::var("OZONE_HOST") {
                let _ = raw; // body is file-backed; host fetch unnecessary
            }
            if let Ok(raw) = std::fs::read_to_string(&full) {
                if let Ok(j) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(b) = j.get("body").and_then(|b| b.as_str()) {
                        body = b.chars().take(240).collect();
                    }
                }
            }
        }
        rows.push((
            created,
            format!("[{}] {} ({}): {}", "coordination", provenance, name, body),
        ));
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    if rows.is_empty() {
        return None;
    }

    const MAX_CHARS: usize = 1600;
    let mut layer = String::from("[Coordination context — agent notes/decisions, newest first]\n");
    for (_, line) in rows {
        if layer.len() + line.len() + 1 > MAX_CHARS {
            break;
        }
        layer.push_str(&line);
        layer.push('\n');
    }
    Some(layer)
}

pub async fn execute(input: ContextAggInput) -> Result<ContextAggOutput, String> {
    match input {
        ContextAggInput::ForTask { task_id, token_budget, include_consciousness, .. } => {
            let containers = match fetch_container(task_id).await {
                Some(c) => vec![c],
                None => Vec::new(),
            };
            let (context_text, sources, truncated) = build_context_from_containers(&containers, token_budget);

            let consciousness_context = if include_consciousness.unwrap_or(true) {
                let consciousness_data = call_consciousness_query()?;
                build_consciousness_context(&consciousness_data)
            } else { None };

            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: context_text.clone(), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: if containers.is_empty() { 0.0 } else { 0.85 }, consciousness_context, coordination_context: None }), error: None })
        }

        ContextAggInput::ForQuery { query, token_budget, container_ids, project_id } => {
            let mut all_containers: Vec<serde_json::Value> = Vec::new();
            if let Some(ids) = container_ids {
                all_containers.extend(resolve_containers(&ids).await);
            }
            if let Some(pid) = project_id {
                if let Some(project) = fetch_project_context(pid).await {
                    all_containers.push(project);
                }
            }
            let (mut context_text, sources, truncated) = build_context_from_containers(&all_containers, token_budget);
            context_text = format!("Query: {}\n\n{}", query, context_text);
            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: context_text.clone(), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: 0.9, consciousness_context: None, coordination_context: None }), error: None })
        }

        ContextAggInput::ForProject { project_id, token_budget, include_files: _, include_urls: _ } => {
            // File/URL reference expansion needs GetFileReferences/
            // GetExternalReferences (real ZSEIQuery variants) — not wired up
            // yet, so this currently returns the project container itself
            // only. Better to under-return than to fabricate references.
            let mut all_containers: Vec<serde_json::Value> = Vec::new();
            if let Some(project) = fetch_project_context(project_id).await {
                all_containers.push(project);
            }
            let (context_text, sources, truncated) = build_context_from_containers(&all_containers, token_budget);
            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: context_text.clone(), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: 0.85, consciousness_context: None, coordination_context: None }), error: None })
        }

        ContextAggInput::Custom { container_ids, token_budget, .. } => {
            let all_containers = resolve_containers(&container_ids).await;
            let (context_text, sources, truncated) = build_context_from_containers(&all_containers, token_budget);
            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: context_text.clone(), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: 0.9, consciousness_context: None, coordination_context: None }), error: None })
        }

        ContextAggInput::ForBlueprint { blueprint_id, step_index, token_budget } => {
            let containers = match fetch_container(blueprint_id).await {
                Some(c) => vec![c],
                None => Vec::new(),
            };
            let (context_text, sources, truncated) = build_context_from_containers(&containers, token_budget);
            Ok(ContextAggOutput { success: true, context: Some(AggregatedContext { context_text: format!("Blueprint step {} context:\n{}", step_index, context_text), token_count: estimate_tokens(&context_text), sources, truncated, coverage_score: 0.85, consciousness_context: None, coordination_context: None }), error: None })
        }

        ContextAggInput::ForStep {
            query,
            session_context,
            token_budget,
            project_id,
            priority_order,
            include_consciousness,
            workspace_id,
            include_coordination,
        } => {
            // Container context (store side) — real keyword search now:
            // search for matching ids, then resolve each to full content.
            // (project_id scoping: SearchContainersByKeywords has no
            // project_id filter today — see the ZSEIQuery definition; this
            // searches host-wide until that's added. Flagged, not silently
            // pretended to work.)
            //
            // priority_order carries this step's AMT-branch-derived keywords
            // (the orchestrator's blueprint reconciliation pass extracts them
            // from the specific branch this step addresses — see
            // stage_3_blueprint_assignment) — previously computed and
            // threaded through but discarded here, so every step's container
            // search was generic (whole-prompt + step description words)
            // regardless of which branch it targets. Given first priority so
            // branch-specific terms survive the 12-keyword cap even when the
            // query itself is long.
            let mut keyword_list: Vec<&str> = priority_order.iter().map(|s| s.as_str()).collect();
            keyword_list.truncate(12);
            for w in query.split_whitespace() {
                if keyword_list.len() >= 12 {
                    break;
                }
                if !keyword_list.contains(&w) {
                    keyword_list.push(w);
                }
            }
            let seed_ids = search_containers_by_keywords(&keyword_list, None).await;
            // Real graph traversal (task 56) — walks real Relation edges
            // from the flat keyword-search seeds, so genuinely related
            // content (e.g. a related function/file linked via a real
            // SimilarTo edge from text/code modality's cross-relationship
            // linking) can be discovered even when the step's own keywords
            // only directly matched one side of that relationship. Merged
            // with the flat seeds, not a replacement for them — traversal
            // starts FROM real matches, it doesn't substitute for finding
            // them. Infrastructure-type filtering below applies uniformly
            // to the merged set regardless of which path found a given id.
            let traversed_ids = traverse_from_seeds(&seed_ids).await;
            let mut ids = seed_ids;
            for id in traversed_ids {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            let containers = resolve_containers(&ids).await;
            let (container_text, sources, mut truncated) =
                build_context_from_containers(&containers, token_budget);

            // …merged with the SESSION graph reconstruction, session first.
            let mut context_text = String::new();
            let mut token_count = 0u32;
            if let Some(session) = &session_context {
                if !session.trim().is_empty() {
                    context_text.push_str("[Session context]\n");
                    context_text.push_str(session);
                    context_text.push_str("\n\n");
                    token_count += (session.len() as u32) / 4;
                }
            }
            context_text.push_str(&container_text);
            token_count += (container_text.len() as u32) / 4;
            if token_count > token_budget {
                truncated = true;
                let keep = (token_budget.saturating_mul(4)) as usize;
                if context_text.len() > keep {
                    context_text.truncate(keep);
                }
            }

            // Global consciousness — a distinct layer, never merged into
            // context_text, so project scoping stays uncontaminated whether
            // or not this is populated. Only fetched when the caller
            // (orchestrator, which knows consciousness_enabled) asks for it.
            let consciousness_context = if include_consciousness {
                call_consciousness_query()
                    .ok()
                    .and_then(|data| build_consciousness_context(&data))
            } else {
                None
            };

            // Coordination layer (task 43) — scoped agent history from the
            // /SharedContext graph: global events + this workspace's (+ this
            // project's). Own layer, newest first, capped hard.
            let coordination_context = if include_coordination {
                build_coordination_layer(workspace_id, project_id).await
            } else {
                None
            };

            Ok(ContextAggOutput {
                success: true,
                context: Some(AggregatedContext {
                    context_text,
                    token_count,
                    sources,
                    truncated,
                    coverage_score: 1.0,
                    consciousness_context,
                    coordination_context,
                }),
                error: None,
            })
        }

        ContextAggInput::ReconstructAtLimit { texts, token_budget } => {
            let reconstructed = reconstruct_texts_at_limit(&texts, token_budget);
            Ok(ContextAggOutput {
                success: true,
                context: Some(AggregatedContext {
                    context_text: reconstructed,
                    token_count: token_budget,
                    sources: vec![],
                    truncated: false,
                    coverage_score: 1.0,
                    consciousness_context: None,
                    coordination_context: None,
                }),
                error: None,
            })
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = args[i + 1].clone(); } }
    // The host passes the full PipelineInput envelope {data, context} — every
    // real invocation from the orchestrator is wrapped this way (see
    // RegistryExecutorAdapter::execute / invoke_pipeline). Unwrap `data` when
    // present; a bare ContextAggInput (direct CLI use) also still works. This
    // was previously missing here (unlike pipeline 9's and pipeline 100's
    // main()), so every real orchestrator call failed with "missing field
    // `action`" — the tag `main.rs` needs was nested one level under `data`,
    // not at the top level of the envelope this parsed directly.
    let parsed_value: serde_json::Value = serde_json::from_str(&input_json)
        .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let data_json = match parsed_value.get("data") {
        Some(d) => serde_json::to_string(d).unwrap_or_else(|_| input_json.clone()),
        None => input_json.clone(),
    };
    let input: ContextAggInput = serde_json::from_str(&data_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}
