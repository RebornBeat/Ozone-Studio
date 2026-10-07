//! Per-call lane splitting for branch-discovery lanes.
//!
//! A lane carries its member batch and shared prompt blocks. The fallback walk
//! renders the lane prompt for each candidate's window. Only when a candidate
//! cannot hold the full prompt are the members halved into parts; each part runs
//! as its own call for that candidate, and the part responses are merged in member
//! order. A member that cannot fit even alone is reported, never dropped: that
//! candidate fails and the walk continues with the next one.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::orchestrator::PipelineExecutor;

pub(crate) type Member = (u64, String, String);

pub(crate) struct LaneShared {
    pub intents: String,
    pub jurisdiction_ctx: String,
    pub standing_ctx: String,
    pub file_relationships: String,
    pub methodology_summaries_block: String,
    pub known_json: String,
}

#[derive(Clone)]
pub(crate) struct LaneSpec {
    pub members: Vec<Member>,
    pub shared: Arc<LaneShared>,
}

tokio::task_local! {
    pub(crate) static LANE_SPEC: LaneSpec;
}

pub(crate) fn current_lane_spec() -> Option<LaneSpec> {
    LANE_SPEC.try_with(|s| s.clone()).ok()
}

pub(crate) fn render_branch_lane_prompt(batch: &[Member], shared: &LaneShared) -> String {
    let batch_listing = batch
        .iter()
        .map(|(id, name, desc)| {
            format!("- {} (container {}): {}", name, id, crate::orchestrator::prefix_at_char_boundary(&desc, 200))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let known_json = shared.known_json.as_str();
    let jurisdiction_ctx = shared.jurisdiction_ctx.as_str();
    let standing_ctx = shared.standing_ctx.as_str();
    let file_relationships = shared.file_relationships.as_str();
    let methodology_summaries_block = shared.methodology_summaries_block.as_str();
    format!(
        r#"You are applying the following methodologies to a set of user intents.

METHODOLOGIES (apply ALL of them; attribute each suggested branch to the methodology that requires it via "methodology"):
{batch_listing}

USER INTENTS:
{}

{methodology_summaries_block}

ALREADY IDENTIFIED BRANCHES (do NOT repeat these):
{known_json}

JURISDICTION CONTEXT: {jurisdiction_ctx}
{standing_ctx}
{file_relationships}

Based on these methodologies, what additional branches (sub-components, requirements, or considerations) should be addressed for each intent?
Only suggest branches NOT already in the known list.

Return ONLY valid JSON:
{{
    "branches": [
        {{
            "branch": "specific branch description",
            "parent_intent": "the intent this branch belongs to",
            "methodology": "the EXACT methodology name (from METHODOLOGIES) requiring this branch",
            "rationale": "why this methodology requires this branch"
        }}
    ]
}}
If no new branches apply, return: {{"branches": []}}"#,
        shared.intents,
        methodology_summaries_block = methodology_summaries_block,
    )
}

fn approx_tokens(text: &str) -> u64 {
    (text.len() / 4 + 1) as u64
}

fn fits(prompt: &str, max_tokens: u64, window: u64) -> bool {
    approx_tokens(prompt) + max_tokens <= window
}

fn parse_object(text: &str) -> Value {
    if let Ok(v) = serde_json::from_str::<Value>(text.trim()) {
        return v;
    }
    match (text.find('{'), text.rfind('}')) {
        (Some(a), Some(b)) if b > a => serde_json::from_str(&text[a..=b]).unwrap_or_else(|_| json!({})),
        _ => json!({}),
    }
}

fn place(
    spec: &LaneSpec,
    members: &[Member],
    window: u64,
    max_tokens: u64,
    out: &mut Vec<Vec<Member>>,
    lost: &mut Vec<u64>,
) {
    if members.is_empty() {
        return;
    }
    let prompt = render_branch_lane_prompt(members, &spec.shared);
    if fits(&prompt, max_tokens, window) {
        out.push(members.to_vec());
        return;
    }
    if members.len() == 1 {
        lost.push(members[0].0);
        return;
    }
    let mid = members.len() / 2;
    place(spec, &members[..mid], window, max_tokens, out, lost);
    place(spec, &members[mid..], window, max_tokens, out, lost);
}

/// Runs one lane for one fallback candidate. When the full prompt fits the candidate's
/// window this is exactly the executor call the walk makes today.
pub(crate) async fn run_candidate(
    executor: &Arc<dyn PipelineExecutor>,
    pipeline_id: u64,
    input: &Value,
    spec: &LaneSpec,
    window: u64,
) -> Result<Value, String> {
    let max_tokens = input.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let full = input.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    if window == 0 || fits(full, max_tokens, window) {
        return executor.execute(pipeline_id, input.clone()).await;
    }

    let mut parts: Vec<Vec<Member>> = Vec::new();
    let mut lost: Vec<u64> = Vec::new();
    place(spec, &spec.members, window, max_tokens, &mut parts, &mut lost);
    if !lost.is_empty() {
        return Err(format!(
            "lane members {lost:?} cannot fit a {window}-token window even alone; this candidate fails and the walk continues"
        ));
    }

    let mut responses: Vec<Value> = Vec::with_capacity(parts.len());
    for part in &parts {
        let mut sub = input.clone();
        if let Some(obj) = sub.as_object_mut() {
            obj.insert("prompt".to_string(), json!(render_branch_lane_prompt(part, &spec.shared)));
        }
        match executor.execute(pipeline_id, sub).await {
            Ok(v) => responses.push(v),
            Err(e) => return Err(format!("lane split part of {} members failed: {e}", part.len())),
        }
    }

    let mut branches: Vec<Value> = Vec::new();
    let mut tokens = 0u64;
    let mut model = String::new();
    for v in &responses {
        let text = v.get("response").and_then(|r| r.as_str()).unwrap_or("");
        if text.trim().is_empty() {
            return Err("lane split part returned an empty response".to_string());
        }
        let Some(arr) = parse_object(text).get("branches").and_then(|b| b.as_array()).cloned() else {
            return Err("lane split part returned no parsable branches array; the walk continues".to_string());
        };
        branches.extend(arr);
        tokens += v.get("tokens_used").and_then(|t| t.as_u64()).unwrap_or(0);
        if let Some(m) = v.get("model_used").and_then(|m| m.as_str()) {
            model = m.to_string();
        }
    }
    tracing::info!(
        pipeline_id,
        window,
        members = spec.members.len(),
        parts = parts.len(),
        "LANE SPLIT: lane members split across calls so each part fits this candidate's window"
    );
    Ok(json!({
        "response": json!({"branches": branches}).to_string(),
        "model_used": model,
        "tokens_used": tokens,
        "lane_split": {"members": spec.members.len(), "parts": parts.len(), "window_tokens": window},
    }))
}
