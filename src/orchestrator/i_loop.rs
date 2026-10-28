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
use crate::orchestrator::run_spans::{self, PassStats};
use std::sync::Arc;

const PROMPT_PIPELINE_ID: u64 = 9;

/// Fingerprint of the signal set the last paid reflection was composed from.
/// An identical set (same unhandled candidates, same handled outcomes) would
/// produce the same reflection again at full price, so the pass is skipped
/// (logged, never silent) until the signal set changes. In-memory: after a
/// restart the first pass reflects once more, which is the safe direction.
static LAST_REFLECTED_FINGERPRINT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Clears the consciousness I-Loop-active flag when dropped (see
/// `run_one_reflection`).
struct ILoopActiveGuard;

impl Drop for ILoopActiveGuard {
    fn drop(&mut self) {
        crate::consciousness::set_i_loop_active(false);
    }
}

/// How many already-handled candidate outcomes are offered to the reflection
/// view (newest first) before the context budget trims them. Large on
/// purpose: the budget — not a fixed cap — decides what fits; this only
/// bounds the in-memory list built from the (already 500-entry-bounded)
/// candidate log.
const HANDLED_OUTCOMES_OFFERED: usize = 500;

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
        // TRACE ROOT (trace spine): one root per pass; every executor call
        // inside the pass (and every capture row it writes) carries it.
        let ctx = crate::trace::TraceCtx::new_root("loop-pass");
        let mut stats = PassStats::default();
        let outcome = crate::trace::with_trace(
            ctx.clone(),
            run_one_reflection(&executor, &store, &available_models, &meta_fallback, &mut stats),
        )
        .await;
        if let Err(e) = outcome {
            tracing::warn!(error = %e, "I-Loop reflection pass failed");
            stats.notes.push(format!("pass failed: {}", e));
        }
        if stats.did_work() {
            let _ = run_spans::record_pass_span(&store, "i_loop", &ctx, stats.to_json()).await;
        } else {
            tracing::debug!("I-Loop: idle pass — no span recorded (nothing consumed, no call made)");
        }
    }
}

/// Recently HANDLED candidates, newest first, as `[outcome] source` lines.
/// The I-Loop used to read only UNHANDLED candidates, but `amt_loop` is woken
/// instantly by the same events and consumes them first, so by the I-Loop's
/// tick the unhandled list was almost always empty (always "idle" on EU).
/// What happened to those signals is exactly what a reflection should see.
/// Read-only: nothing is marked handled here — that stays `amt_loop`'s.
fn recent_handled_outcomes() -> Vec<String> {
    crate::orchestrator::amt_candidates::load_all()
        .into_iter()
        .rev()
        .filter(|c| c.get("handled").and_then(|h| h.as_bool()) == Some(true))
        .take(HANDLED_OUTCOMES_OFFERED)
        .filter_map(|c| {
            let source = c.get("source").and_then(|s| s.as_str())?;
            let outcome = c
                .get("outcome")
                .and_then(|o| o.as_str())
                .unwrap_or("handled");
            Some(format!("[{}] {}", outcome, source))
        })
        .collect()
}

async fn run_one_reflection(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
    stats: &mut PassStats,
) -> Result<(), String> {
    // Ground the reflection in something real: the AMT candidates the acting
    // loop has been noticing (newest first, unhandled) plus what happened to
    // the recently handled ones — never a context-free "how do you feel"
    // prompt. The old `.take(5)` count cap is gone: the context budget below
    // decides how many fit (operator ruling: budgets follow the model
    // window, no fixed caps).
    let unhandled_all: Vec<serde_json::Value> = crate::orchestrator::amt_candidates::unhandled()
        .into_iter()
        .rev()
        .collect();
    for c in &unhandled_all {
        if let Some(id) = c.get("container_id").and_then(|v| v.as_u64()) {
            stats.candidate_container_ids.push(id);
        }
    }
    let unhandled_lines: Vec<String> = unhandled_all
        .iter()
        .filter_map(|c| c.get("source").and_then(|s| s.as_str()).map(String::from))
        .collect();
    let handled_lines = recent_handled_outcomes();
    stats.candidates_consumed = (unhandled_lines.len() + handled_lines.len()) as u64;

    // IDLE SKIP (efficiency sweep Tier 2 #4 — assistant.rs is the native
    // good-pattern example): with nothing to reflect ON, a full-price LLM
    // call is guaranteed waste. Skip the call entirely; the persistence path
    // below also skips — the next tick tries again.
    if unhandled_lines.is_empty() && handled_lines.is_empty() {
        tracing::debug!("I-Loop: no candidates (unhandled or handled) this tick — reflection skipped (idle, zero cost)");
        return Ok(());
    }
    // CHANGE DETECTION: the same signal set as the last paid reflection would
    // yield the same reflection again — skip, loudly, until it changes.
    let fingerprint = blake3::hash(
        format!("{}\n--\n{}", unhandled_lines.join("\n"), handled_lines.join("\n")).as_bytes(),
    )
    .to_hex()
    .to_string();
    if let Ok(last) = LAST_REFLECTED_FINGERPRINT.lock() {
        if last.as_deref() == Some(fingerprint.as_str()) {
            tracing::info!(
                unhandled = unhandled_lines.len(),
                handled = handled_lines.len(),
                "I-Loop: signal set unchanged since the last reflection — paid call skipped (change detection)"
            );
            stats.notes.push("signal set unchanged since last reflection — skipped".to_string());
            return Ok(());
        }
    }

    // Model + window for THIS call (needed before the prompt so the view is
    // budgeted for the model that will read it).
    let meta_model =
        crate::orchestrator::PromptOrchestrator::resolve_meta_model(&available_models, &meta_fallback);
    let (window, window_source) = match meta_model {
        Some(m) => run_spans::profile_window(m.context_length),
        None => run_spans::default_model_window(),
    };
    let want_output = 300usize;
    let budget = run_spans::input_budget_tokens(window, want_output);

    let unhandled_block = if unhandled_lines.is_empty() {
        "(no unhandled signals this tick — see the recently handled outcomes below)".to_string()
    } else {
        unhandled_lines.join("\n- ")
    };
    let handled_block = if handled_lines.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nRECENTLY HANDLED BY THE ACTING LOOP (what happened to earlier signals, newest first):\n- {}",
            handled_lines.join("\n- ")
        )
    };
    use crate::context_budget::{assemble, ContextSection};
    let assembled = assemble(
        vec![
            ContextSection::new(
                "scaffold-head",
                0,
                "This is a periodic self-reflection cycle, not a response to a user request.\n\nRECENT SIGNALS THE SYSTEM HAS NOTICED:\n- ",
            ),
            ContextSection::new("unhandled-signals", 3, unhandled_block),
            ContextSection::new("handled-outcomes", 5, handled_block),
            ContextSection::new(
                "scaffold-tail",
                0,
                "\n\nIn 2-4 sentences, reflect honestly on what these signals suggest (if anything) about how the system is operating right now — patterns worth naming, not generic reassurance. If nothing notable stands out, say so plainly rather than manufacturing insight.\n\nReturn ONLY valid JSON:\n{\"reflection\": \"your 2-4 sentence reflection\"}",
            ),
        ],
        budget,
    );
    run_spans::record_view_trims(
        "i_loop_reflection",
        meta_model.map(|m| m.identifier.as_str()).unwrap_or(""),
        window,
        window_source,
        want_output,
        &assembled,
    );
    if !assembled.trims.is_empty() {
        tracing::warn!(
            trims = assembled.trims.len(),
            budget_tokens = budget,
            window,
            "I-Loop: signal view trimmed to the model's window (recorded, markers emitted)"
        );
    }
    let prompt = assembled.text;

    // Output budget follows the model window (the same principle the
    // assistant digest uses: digest_budget = meta_window / 2). The I-Loop's
    // reflection is shorter than a digest, so 1/4 of the window is generous
    // enough for the JSON reflection + the reasoning phase that precedes it.
    let output_budget = meta_model
        .as_ref()
        .map(|m| (m.context_length as usize / 4).max(500))
        .unwrap_or(1000);

    let mut input = serde_json::json!({
        "prompt": prompt,
        // BUDGET FOLLOWS THE MODEL WINDOW (operator: "there is no limit —
        // it's traversable like AMT"): the output budget derives from the
        // meta model's context_length, not a fixed constant. A 128k model
        // gets a generous budget; a 4k model gets a proportionally smaller
        // one. The reasoning-model truncation fix: reasoning models like
        // nemotron produce chain-of-thought before content — 300 tokens
        // was exhausted by reasoning alone (content=null, length-truncated),
        // consuming 10-11k total tokens with 0 usable output.
        "max_tokens": output_budget,
        "temperature": 0.4,
        "system_prompt": "Periodic self-reflection. Be honest and specific, not performative. Return only valid JSON."
    });
    // Prefer a free model for this low-stakes periodic reflection — same
    // budget-discipline reasoning as amt_loop/meta_loop's own
    // meta_fallback convention (local+free by default for detached meta
    // work, not live request-answering).
    // Chain-order resolution (quality AND speed): openrouter/free first,
    // BitNet as offline backstop — was `find(is_free)` = always BitNet.
    if let Some(default_model) = meta_model {
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

    // Mark the I-Loop active for the duration of the reflection (the task
    // manager's consciousness observer waits on this flag). The drop guard
    // clears it on EVERY exit path — error returns, early returns, panics
    // that unwind — so a failed reflection can never leave the flag stuck on.
    crate::consciousness::set_i_loop_active(true);
    let _i_loop_active = ILoopActiveGuard;
    stats.calls_made += 1;
    // SAME-MODEL RETRY (2026-10-27): free-tier reasoning models sometimes
    // exhaust their output budget on chain-of-thought (finish_reason=length,
    // content=null) — a single retry catches the transient case.
    let result = match executor.execute(PROMPT_PIPELINE_ID, input.clone()).await {
        Ok(r) => Ok(r),
        Err(_) => {
            tracing::info!("I-Loop: first reflection attempt errored — retrying once");
            executor.execute(PROMPT_PIPELINE_ID, input.clone()).await
        }
    };
    // Doctrine #35: the loop's model calls are captured like every other
    // (S11, capture_loop_model_call) — serving model/tokens/failures leave
    // a trace instead of vanishing with the loop's detached context.
    crate::orchestrator::PromptOrchestrator::capture_loop_model_call("i_loop_reflection", &prompt, &result);
    let result = match result {
        Ok(r) => r,
        Err(e) => {
            stats.failures += 1;
            return Err(e);
        }
    };
    let response = result.get("response").and_then(|r| r.as_str()).unwrap_or("");
    if response.trim().is_empty() {
        stats.failures += 1;
        return Err("I-Loop: prompt pipeline returned an empty response".to_string());
    }
    let reflection = extract_reflection(response).unwrap_or_else(|| response.trim().to_string());
    if reflection.trim().is_empty() {
        tracing::info!("I-Loop: reflection was empty after extraction — not persisting");
        stats.notes.push("reflection empty after extraction — not persisted".to_string());
        return Ok(());
    }

    // The paid call produced a usable reflection: remember the signal set it
    // was composed from (change detection for the next pass).
    if let Ok(mut last) = LAST_REFLECTED_FINGERPRINT.lock() {
        *last = Some(fingerprint);
    }
    stats.successes += 1;
    persist_reflection(store, &reflection, &stats.candidate_container_ids).await
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
async fn persist_reflection(
    store: &Arc<dyn StoreAccess>,
    reflection: &str,
    signal_container_ids: &[u64],
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let store_dir = std::path::PathBuf::from(&data_dir).join("shared_context");
    std::fs::create_dir_all(&store_dir).map_err(|e| e.to_string())?;
    let rel_path = format!("shared_context/i-loop-reflection-{}.json", now);
    // Trace fields go in the stored BODY only (never the container keywords —
    // keywords become graph-event scope keywords).
    let mut body = serde_json::json!({
        "kind": "i_loop_reflection",
        "reflection": reflection,
        "recorded_at": now,
        "signal_container_ids": signal_container_ids,
    });
    crate::trace::stamp(&mut body);
    std::fs::write(
        std::path::PathBuf::from(&data_dir).join(&rel_path),
        serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?,
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
