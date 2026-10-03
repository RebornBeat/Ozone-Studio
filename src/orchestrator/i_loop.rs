//! The I-Loop — real self-reflection cycle (docs/ACTING_LOOP_GUIDE.md §5).
//! Registered as metadata since early in this project (pipeline_id 44,
//! `src/bootstrap.rs`) but never implemented — confirmed real gap, closed
//! here.
//!
//! Shape decision (§5 named two real options, neither decided there): this
//! build takes Option 1 — a long, safe real interval mirroring
//! `amt_loop`/`meta_loop`'s own proven 1800s convention — rather than
//! Option 2's event-driven trigger model, whose exact real trigger signals
//! ("a project's AMT just grew", "a real user session just ended") were
//! explicitly left undecided in the guide and would need their own design
//! pass to pin down precisely. Reuses the SAME generic prompt pipeline
//! (`PROMPT_PIPELINE_ID = 9`) every other loop in this codebase already
//! calls, rather than standing up a whole separate compiled pipeline crate
//! for pipeline_id 44's "i_loop" folder — a materially smaller, safer unit
//! of new code for the same real outcome (a real, running reflection
//! cycle); the dedicated pipeline crate remains a real, separate future
//! upgrade if ever wanted.
//!
//! Budget safety (the guide's own central finding): `config.toml`'s
//! documented `i_loop_interval_ms` defaults to 60000 (60s) — taken
//! literally, that's 1,440 real LLM calls/day on its own, more than the
//! entire ~1,000/day budget. This loop honors the configured interval but
//! clamps it to a real safe floor (`MIN_INTERVAL_SECS`) rather than ever
//! running the documented default literally.

use crate::config::{AvailableModel, ModelFallbackConfig};
use crate::orchestrator::{ModelConfigOverride, PipelineExecutor, StoreAccess};
use crate::types::container::{
    CompressionType, Container, Context, ContainerType, GlobalState, IntegrityData, LocalState,
    Metadata, Modality, StoragePointers, TraversalHints, SHARED_CONTEXT_ROOT_ID,
};
use std::sync::Arc;

const PROMPT_PIPELINE_ID: u64 = 9;

/// Real safety floor — 1800s (30 min) matches `amt_loop`/`meta_loop`'s own
/// proven interval exactly, never the documented-but-budget-incompatible
/// 60s default.
const MIN_INTERVAL_SECS: u64 = 1800;

pub async fn run_i_loop(
    executor: Arc<dyn PipelineExecutor>,
    store: Arc<dyn StoreAccess>,
    enabled: bool,
    configured_interval_ms: u64,
    available_models: Vec<AvailableModel>,
    meta_fallback: ModelFallbackConfig,
) {
    if !enabled {
        tracing::info!("I-Loop disabled (ConsciousnessConfig.enabled = false)");
        return;
    }
    let configured_secs = configured_interval_ms / 1000;
    let interval_secs = configured_secs.max(MIN_INTERVAL_SECS);
    if configured_secs < MIN_INTERVAL_SECS {
        tracing::warn!(
            configured_secs,
            clamped_to = interval_secs,
            "I-Loop: configured i_loop_interval_ms is below the real budget-safety floor — clamped"
        );
    }
    tracing::info!(interval_secs, "I-Loop starting");

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        tracing::info!("I-Loop: starting reflection pass");
        if let Err(e) = run_one_reflection(&executor, &store, &available_models, &meta_fallback).await {
            tracing::warn!(error = %e, "I-Loop reflection pass failed");
        }
    }
}

async fn run_one_reflection(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
) -> Result<(), String> {
    // Ground the reflection in something real: the most recent unhandled
    // AMT candidates (what the acting loop has been noticing) — never a
    // context-free "how do you feel" prompt.
    let recent: Vec<serde_json::Value> = crate::orchestrator::amt_candidates::unhandled()
        .into_iter()
        .rev()
        .take(5)
        .collect();
    let recent_summary = if recent.is_empty() {
        "No unhandled findings or re-expansion candidates right now.".to_string()
    } else {
        recent
            .iter()
            .filter_map(|c| c.get("source").and_then(|s| s.as_str()))
            .collect::<Vec<_>>()
            .join("\n- ")
    };

    let prompt = format!(
        r#"This is a periodic self-reflection cycle, not a response to a user request.

RECENT SIGNALS THE SYSTEM HAS NOTICED:
- {}

In 2-4 sentences, reflect honestly on what these signals suggest (if anything) about how the system is operating right now — patterns worth naming, not generic reassurance. If nothing notable stands out, say so plainly rather than manufacturing insight.

Return ONLY valid JSON:
{{"reflection": "your 2-4 sentence reflection"}}"#,
        recent_summary
    );

    let mut input = serde_json::json!({
        "prompt": prompt,
        "max_tokens": 300,
        "temperature": 0.4,
        "system_context": "Periodic self-reflection. Be honest and specific, not performative. Return only valid JSON."
    });
    // Prefer a free model for this low-stakes periodic reflection — same
    // budget-discipline reasoning as amt_loop/meta_loop's own
    // meta_fallback convention (local+free by default for detached meta
    // work, not live request-answering).
    // Chain-order resolution (quality AND speed): openrouter/free first,
    // BitNet as offline backstop — was `find(is_free)` = always BitNet.
    if let Some(default_model) = crate::orchestrator::PromptOrchestrator::resolve_meta_model(&available_models, &meta_fallback) {
        let override_cfg = ModelConfigOverride {
            model_type: Some(default_model.model_type.clone()),
            model_identifier: Some(default_model.identifier.clone()),
            max_tokens: None,
            temperature: None,
            context_length: Some(default_model.context_length as u32),
            api_endpoint: default_model.api_endpoint.clone(),
            api_key_env: default_model.api_key_env.clone(),
            api_key: default_model.api_key.clone(),
            wire_protocol: default_model.wire_protocol.clone(),
            bitnet_cli_path: default_model.bitnet_cli_path.clone(),
            local_model_path: default_model.local_model_path.clone(),
        };
        if let Ok(v) = serde_json::to_value(&override_cfg) {
            input["model_override_config"] = v;
        }
    }

    let result = executor.execute(PROMPT_PIPELINE_ID, input).await;
    // Doctrine #35: the loop's model calls are captured like every other
    // (S11, capture_loop_model_call) — serving model/tokens/failures leave
    // a trace instead of vanishing with the loop's detached context.
    crate::orchestrator::PromptOrchestrator::capture_loop_model_call("i_loop_reflection", &prompt, &result);
    let result = result?;
    let response = result.get("response").and_then(|r| r.as_str()).unwrap_or("");
    if response.trim().is_empty() {
        return Err("I-Loop: prompt pipeline returned an empty response".to_string());
    }
    let reflection = extract_reflection(response).unwrap_or_else(|| response.trim().to_string());
    if reflection.trim().is_empty() {
        tracing::info!("I-Loop: reflection was empty after extraction — not persisting");
        return Ok(());
    }

    persist_reflection(store, &reflection).await
}

/// Same tolerant-brace-scan idiom `amt_loop.rs`'s `extract_all_json_objects`
/// uses for small-model noise — a real, proven pattern, not reinvented
/// differently here.
fn extract_reflection(response: &str) -> Option<String> {
    let bytes = response.as_bytes();
    for start in 0..bytes.len() {
        if bytes[start] != b'{' {
            continue;
        }
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = None;
        for (i, &c) in bytes.iter().enumerate().skip(start) {
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == b'\\' {
                    escaped = true;
                } else if c == b'"' {
                    in_string = false;
                }
                continue;
            }
            match c {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            let candidate = &response[start..=end];
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(candidate) {
                if let Some(r) = v.get("reflection").and_then(|r| r.as_str()) {
                    return Some(r.to_string());
                }
            }
        }
    }
    None
}

/// Persist the reflection as a real, discoverable `CoordinationEvent`
/// under `SharedContext` — `kind:"i_loop_reflection"` — so it rides the
/// SAME living graph ripple as everything else
/// (docs/ACTING_LOOP_GUIDE.md's own "the finding becomes part of the
/// living knowledge structure" philosophy), not a private log file nobody
/// else can see.
async fn persist_reflection(store: &Arc<dyn StoreAccess>, reflection: &str) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let store_dir = std::path::PathBuf::from(&data_dir).join("shared_context");
    std::fs::create_dir_all(&store_dir).map_err(|e| e.to_string())?;
    let rel_path = format!("shared_context/i-loop-reflection-{}.json", now);
    std::fs::write(
        std::path::PathBuf::from(&data_dir).join(&rel_path),
        serde_json::to_string_pretty(&serde_json::json!({
            "kind": "i_loop_reflection",
            "reflection": reflection,
            "recorded_at": now,
        }))
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let container = Container {
        global_state: GlobalState {
            container_id: 0,
            parent_id: SHARED_CONTEXT_ROOT_ID,
            child_ids: vec![],
            child_count: 0,
            version: 1,
        },
        local_state: LocalState {
            metadata: Metadata {
                container_type: ContainerType::CoordinationEvent,
                modality: Modality::Unknown,
                created_at: now,
                updated_at: now,
                provenance: "i-loop".to_string(),
                permissions: 0,
                owner_id: 0,
                name: Some(format!("I-Loop reflection ({})", now)),
                materialized_path: Some(format!("/SharedContext/global/i_loop_reflection/{}", now)),
            },
            context: Context {
                categories: vec![],
                methodologies: vec![],
                keywords: vec![
                    "i_loop_reflection".to_string(),
                    "i-loop".to_string(),
                    "scope:global".to_string(),
                ],
                topics: vec!["consciousness".to_string()],
                relationships: vec![],
                learned_associations: vec![],
                embedding: None,
            },
            storage: StoragePointers {
                db_shard_id: None,
                vector_index_ref: None,
                object_store_path: Some(rel_path),
                compression_type: CompressionType::None,
            },
            hints: TraversalHints::default(),
            integrity: IntegrityData::default(),
            file_context: None,
            code_context: None,
            text_context: None,
            external_ref: None,
        },
    };
    let container_json = serde_json::to_value(&container).map_err(|e| e.to_string())?;
    store.create_container(SHARED_CONTEXT_ROOT_ID, container_json).await?;
    tracing::info!(reflection = %reflection, "I-Loop: reflection persisted");
    Ok(())
}
