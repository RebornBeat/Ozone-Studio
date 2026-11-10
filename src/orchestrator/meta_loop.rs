//! Real methodology meta-loop: periodically reviews methodology gaps
//! recorded live during orchestration (see amt.rs's
//! PromptOrchestrator::record_methodology_gap, called from
//! enrich_with_zsei_knowledge whenever a real request's keyword signal
//! matched zero methodologies) and, for a gap not already covered by
//! something created since, drafts and persists a real new methodology.
//!
//! TRIGGER (EVENT-DRIVEN since 2026-11-04): wakes on a META RIPPLE — chiefly `MethodologyGap` (a request matched no methodology)
//! and `CapabilityGap` (a required provider is missing: the loop drafts a methodology around the missing capability) — with a
//! minimum gap between passes (default 10 min), a startup catch-up and a 6 h liveness heartbeat (`ripple_class.rs`). The
//! `RefinementConfig.interval_secs` it was built around is retained for compatibility but is no longer a schedule.
//!
//! Lives here (not on TaskManager, where the pre-existing but never-started
//! `start_refinement_daemon` lives) because drafting a genuine methodology
//! requires a real LLM call, and TaskManager only has ZSEI access — no
//! pipeline-executing capability. This loop takes the same PipelineExecutor/
//! StoreAccess contracts the rest of the orchestrator already uses, and
//! respects the same RefinementConfig (enabled/interval_secs) the dormant
//! daemon does rather than inventing a second config surface.
//!
//! Cross-referencing against duplication is real but intentionally simple:
//! before drafting anything, it re-runs the same relevance-filtered
//! search_by_keywords (zsei/query.rs's find_methodologies_by_keywords) the
//! live request path uses — if that now returns a match (created since the
//! gap was recorded, by this loop or anything else), the gap is considered
//! covered and skipped, not duplicated. A full pairwise similarity pass
//! across the whole methodology store is a larger, separate undertaking,
//! not built here.

use crate::config::{AvailableModel, ModelFallbackConfig};
use crate::orchestrator::{ModelConfigOverride, PipelineExecutor, StoreAccess};
use crate::orchestrator::run_spans::{self, PassStats};
use crate::task::RefinementConfig;
use std::sync::Arc;

const PROMPT_PIPELINE_ID: u64 = 9;
const METHODOLOGY_CREATE_PIPELINE_ID: u64 = 12;

/// Start the meta-loop. Returns immediately if disabled; otherwise runs
/// forever, waking on meta ripples (see the module doc) — NOT on `config.interval_secs`.
pub async fn run_methodology_meta_loop(
    executor: Arc<dyn PipelineExecutor>,
    store: Arc<dyn StoreAccess>,
    config: RefinementConfig,
    available_models: Vec<AvailableModel>,
    meta_fallback: ModelFallbackConfig,
) {
    if !config.enabled {
        tracing::info!("Methodology meta-loop disabled (RefinementConfig.enabled = false)");
        return;
    }

    // EVENT-DRIVEN (2026-11-04): wakes on a META RIPPLE — chiefly the MethodologyGap signal
    // `record_methodology_gap` emits — with a minimum gap between passes and a long heartbeat.
    // `config.interval_secs` is retained for compatibility but is no longer a schedule.
    let heartbeat = crate::orchestrator::ripple_class::heartbeat_secs();
    let min_gap = crate::orchestrator::ripple_class::min_gap_secs();
    tracing::info!(
        heartbeat_secs = heartbeat,
        min_gap_secs = min_gap,
        configured_interval_secs = config.interval_secs,
        "Methodology meta-loop starting (event-driven: meta ripple + startup catch-up + heartbeat)"
    );
    let gate = crate::orchestrator::ripple_class::meta_loop_gate();

    loop {
        let trigger = gate.wait_next(heartbeat, min_gap).await;
        tracing::info!(?trigger, "Methodology meta-loop: woken");

        tracing::info!("Methodology meta-loop: starting review pass");
        // TRACE ROOT (trace spine): one root per pass; every executor call
        // inside the pass (and every capture row it writes) carries it.
        let ctx = crate::trace::TraceCtx::new_root("loop-pass");
        let _own_pass = crate::orchestrator::ripple_class::pass_guard(&ctx.trace_id);
        let mut stats = PassStats::default();
        let outcome = crate::trace::with_trace(
            ctx.clone(),
            crate::keyword_program::with_origin(crate::keyword_program::Origin::actor("meta-loop"), async {
                // 1) SUFFICIENCY REVIEW of queued requests → raises "new"/"modify" gaps; 2) act on every open gap.
                if let Err(e) = review_methodology_requests_once(&executor, &store, &available_models, &meta_fallback, &mut stats).await {
                    tracing::warn!(error = %e, "Methodology meta-loop: sufficiency review failed");
                    stats.notes.push(format!("sufficiency review failed: {}", e));
                }
                // 1b) QUALITY AUDIT (E18): thin methodologies raise `modify` gaps that the drafting pass below acts on.
                audit_methodology_quality_once(&store, &mut stats).await;
                let gaps_result = review_methodology_gaps_once(&executor, &store, &available_models, &meta_fallback, &mut stats).await;
                // (the keyword program — rollup, continuous training, settlement — runs in its OWN actors: orchestrator/keyword_loops.rs. It used
                //  to run here and starved this pass for hours: no pass completed after 07:23 UTC on 2026-11-08.)
                gaps_result
            }),
        )
        .await;
        if let Err(e) = outcome {
            tracing::warn!(error = %e, "Methodology meta-loop review pass failed");
            stats.notes.push(format!("pass failed: {}", e));
        }
        if stats.did_work() {
            let _ = run_spans::record_pass_span(&store, "meta", &ctx, stats.to_json()).await;
        } else {
            tracing::debug!("Methodology meta-loop: idle pass — no span recorded (no gaps consumed, no call made)");
        }
        tracing::info!("Methodology meta-loop: review pass complete");
        gate.pass_finished();
    }
}

/// The window of the model that will READ a detached meta call: the first usable entry of the meta fallback chain (its configured
/// `context_length`, e.g. 32,000 for the `openrouter/free` router), else the host default. Windowing follows the READING model, not the
/// largest configured number — a prompt sized for 128k would overflow a 32k member the router picks.
pub(crate) fn reader_window(available_models: &[AvailableModel], meta_fallback: &ModelFallbackConfig) -> (usize, &'static str) {
    meta_fallback
        .order
        .iter()
        .filter_map(|id| available_models.iter().find(|m| &m.identifier == id))
        .find(|m| !meta_fallback.free_only || m.is_free)
        .map(|m| run_spans::profile_window(m.context_length))
        .unwrap_or_else(run_spans::default_model_window)
}

/// A methodology's current content, WHOLE: (name, keywords, content-file JSON text). Read from the container + its content file.
pub(crate) async fn methodology_whole(store: &Arc<dyn StoreAccess>, id: u64) -> Option<(String, Vec<String>, String)> {
    let c = store.get_container(id).await.ok().flatten()?;
    let name = c.pointer("/local_state/metadata/name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let kws: Vec<String> = c
        .pointer("/local_state/context/keywords")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    // the shared reader: data dir first, then the path as given (see tool_binding::read_object_file)
    let body = c
        .pointer("/local_state/storage/object_store_path")
        .and_then(|v| v.as_str())
        .and_then(crate::orchestrator::tool_binding::read_object_file)
        .unwrap_or_default();
    Some((name, kws, body))
}

/// REFINEMENT NOTIFICATION (operator 2026-11-10). A SOUND methodology is one where ALL of its rules apply to the requests it is gathered for; a
/// rule that does not apply means the methodology is too broad / unrefined and should be refined (e.g. split into focused ones). The rule gate
/// (`stages::judge_rule_applicability`) therefore reports every rule it judges "clearly NOT applicable" here: one durable, append-only line per
/// (request, methodology) with the evidence — never a `new`/`modify` gap (the meta loop would draft a new methodology from those), and nothing
/// is changed or removed. The refinement meta loop reads `methodology_refinement_candidates.jsonl`. Best-effort: an I/O failure never affects a
/// walk.
pub(crate) fn record_refinement_candidate(methodology_id: u64, name: &str, rules: &[(String, String)], not_applicable: &std::collections::BTreeSet<String>, request_terms: &[String]) {
    use std::io::Write;
    if not_applicable.is_empty() {
        return;
    }
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let path = format!("{}/methodology_refinement_candidates.jsonl", data_dir);
    let left_out: Vec<serde_json::Value> =
        rules.iter().filter(|(k, _)| not_applicable.contains(k)).map(|(k, t)| serde_json::json!({"rule": k, "text": t})).collect();
    let line = serde_json::json!({
        "ts_ms": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0),
        "methodology_id": methodology_id,
        "methodology": name,
        "rules_total": rules.len(),
        "rules_not_applicable": left_out,
        "request_terms": request_terms,
        "reason": "rule gate: rules clearly NOT applicable to a request this methodology was gathered for — a sound methodology has no optional rules; refine or split it",
    })
    .to_string();
    if let Err(e) = std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| writeln!(f, "{line}")) {
        tracing::warn!(error = %e, "could not record a methodology refinement candidate");
    }
}

/// METHODOLOGY QUALITY AUDIT (E18, 2026-11-07). Deterministic, no model call: a methodology with fewer than 3 usable keywords (after
/// `request_terms` hygiene) feeds the Stage 3 floor and keyword search badly; one with no decision rule or heuristic carrying real text
/// cannot steer a plan. Each such methodology raises ONE `modify` gap (target = its id, reason starts "quality audit") — the existing
/// modify pass drafts the additions (nothing existing is dropped). Idempotent: a methodology that already has a quality-audit gap
/// (open or handled) is not raised again, so a methodology the modify pass could not improve does not loop.
async fn audit_methodology_quality_once(store: &Arc<dyn StoreAccess>, stats: &mut PassStats) {
    // EVERY methodology, not just the root's children: live EU 2026-11-10 — the root has 55 children and they are exactly the 55 methodologies
    // WITH rules; the 43 without (auto-synthesized, parented elsewhere) were never audited, so nothing ever repaired them.
    let children: Vec<u64> = match store.list_ids(Some("Methodology")).await {
        Ok(ids) if !ids.is_empty() => ids,
        _ => {
            let Ok(Some(root)) = store.get_container(2).await else { return };
            let Some(kids) = root.pointer("/global_state/child_ids").and_then(|c| serde_json::from_value::<Vec<u64>>(c.clone()).ok()) else {
                return;
            };
            kids
        }
    };
    let existing: Vec<serde_json::Value> = std::fs::read_to_string(gaps_log_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let audited = |id: u64| {
        existing.iter().any(|g| {
            g.get("target_id").and_then(|t| t.as_u64()) == Some(id)
                && g.get("reason").and_then(|r| r.as_str()).map(|r| r.starts_with("quality audit")).unwrap_or(false)
        })
    };
    let mut raised = 0usize;
    for id in children {
        if audited(id) {
            continue;
        }
        let Some((name, kws, body)) = methodology_whole(store, id).await else { continue };
        if name.starts_with("[retired]") || name.to_lowercase().contains("retired") {
            continue;
        }
        let mut problems: Vec<&str> = Vec::new();
        if crate::orchestrator::request_terms::sanitize_keywords("", kws.clone()).len() < 3 {
            problems.push("fewer than 3 usable keywords");
        }
        let parsed: Option<serde_json::Value> = serde_json::from_str(&body).ok();
        // the same structured reader the blueprint uses: a methodology counts as having rules only if the blueprint would actually render one
        let has_rule = parsed.as_ref().map(|b| !crate::orchestrator::PromptOrchestrator::rule_items(b).is_empty()).unwrap_or(false);
        if parsed.is_some() && !has_rule {
            problems.push("no decision rule or heuristic with real text");
        }
        if problems.is_empty() {
            continue;
        }
        let reason = format!("quality audit: {}", problems.join("; "));
        let request = format!("Quality audit of methodology \"{}\": {}. Add real keywords and concrete decision rules without removing anything existing.", name, problems.join("; "));
        crate::orchestrator::PromptOrchestrator::record_methodology_gap_ex(&kws, &[], "modify", Some(id), &reason, &request);
        raised += 1;
    }
    if raised > 0 {
        tracing::info!(raised, "Methodology quality audit: thin methodologies raised modify gaps");
        stats.notes.push(format!("quality audit raised {} modify gap(s)", raised));
    }
}

/// METHODOLOGY SUFFICIENCY REVIEW (operator 2026-11-05: "we should never skip due to no methodology ... even if we find some we
/// still identify if we need new ones or to modify the ones we have ... like we do missing capabilities"). For every queued
/// request (Stage 3 queues ALL of them in `methodology_reviews.json`): one zero-shot, YES/NO-style judgement over the request
/// WHOLE and the gathered methodologies WHOLE — is a NEW methodology needed (yes/no + why), does any EXISTING one need
/// modification (which, why)? Verdicts become gaps (`kind` new/modify) that the drafting pass below acts on. No confidence number
/// is asked for. Failures retry next pass; a review is never silently dropped.
async fn review_methodology_requests_once(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
    stats: &mut PassStats,
) -> Result<(), String> {
    let data_dir = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let path = format!("{}/methodology_reviews.json", data_dir);
    let mut reviews: Vec<serde_json::Value> = match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("parse {}: {}", path, e))?,
        Err(_) => return Ok(()),
    };
    let mut changed = false;
    for rv in reviews.iter_mut() {
        if rv.get("handled").and_then(|h| h.as_bool()).unwrap_or(false) {
            continue;
        }
        let attempts = rv.get("attempts").and_then(|a| a.as_u64()).unwrap_or(0);
        if attempts >= 10 {
            rv["handled"] = serde_json::json!(true);
            rv["outcome"] = serde_json::json!("failed_after_max_attempts");
            changed = true;
            continue;
        }
        rv["attempts"] = serde_json::json!(attempts + 1);
        changed = true;
        let request = rv.get("request").and_then(|q| q.as_str()).unwrap_or("").to_string();
        let keywords: Vec<String> = rv.get("keywords").and_then(|k| serde_json::from_value(k.clone()).ok()).unwrap_or_default();
        let topics: Vec<String> = rv.get("topics").and_then(|k| serde_json::from_value(k.clone()).ok()).unwrap_or_default();
        let ids: Vec<u64> = rv.get("gathered_ids").and_then(|k| serde_json::from_value(k.clone()).ok()).unwrap_or_default();

        let (window, window_source) = reader_window(available_models, meta_fallback);
        let want_output = 1200usize;
        let budget = run_spans::input_budget_tokens(window, want_output);
        let head = format!(
            "{}\n\nA real request was received. Decide whether the methodologies the system gathered for it are SUFFICIENT.\n\nREQUEST (whole):\n",
            crate::orchestrator::run_spans::now_line()
        );
        let gathered_hdr = "\n\nGATHERED METHODOLOGIES (id, name, content whole):\n";
        let tail_text = "\nAnswer with ONE JSON object, no prose:\n{\"new_needed\": true|false, \"new_reason\": \"why a NEW methodology is needed (or empty)\", \"new_keywords\": [\"keywords the new methodology should be found by\"], \"modify\": [{\"id\": <a gathered methodology id>, \"reason\": \"what must be ADDED to or CHANGED in it, in light of this request\"}]}\nnew_needed is true ONLY if the request needs guidance no gathered methodology provides. List a methodology under modify ONLY if it is relevant but is missing something this request reveals. If everything is sufficient: {\"new_needed\": false, \"new_reason\": \"\", \"new_keywords\": [], \"modify\": []}. Do not rate your certainty.";
        // CHUNK, never cut: the gathered methodologies are packed WHOLE into window-sized groups (greedy, in gather order); each group is
        // one call. A single methodology larger than a window cannot be split further and is the only thing `assemble` may still trim
        // (recorded). Verdicts merge: modify = union; new_needed only if EVERY group says nothing in it provides the guidance.
        let mut blocks: Vec<(u64, String)> = Vec::new();
        for id in &ids {
            if let Some((name, kws, body)) = methodology_whole(store, *id).await {
                blocks.push((*id, format!("--- METHODOLOGY {id}: {name}\nkeywords: {}\n{body}\n", kws.join(", "))));
            }
        }
        let known_ids: Vec<u64> = blocks.iter().map(|b| b.0).collect();
        let fixed_tokens = crate::context_budget::estimate_tokens(&head)
            + crate::context_budget::estimate_tokens(&request)
            + crate::context_budget::estimate_tokens(gathered_hdr)
            + crate::context_budget::estimate_tokens(&tail_text)
            + 200; // part-note reserve
        let room = budget.saturating_sub(fixed_tokens).max(1);
        let mut groups: Vec<Vec<(u64, String)>> = Vec::new();
        let (mut cur, mut cur_t): (Vec<(u64, String)>, usize) = (Vec::new(), 0);
        for b in blocks {
            let t = crate::context_budget::estimate_tokens(&b.1);
            if !cur.is_empty() && cur_t + t > room {
                groups.push(std::mem::take(&mut cur));
                cur_t = 0;
            }
            cur_t += t;
            cur.push(b);
        }
        if !cur.is_empty() {
            groups.push(cur);
        }
        if groups.is_empty() {
            groups.push(Vec::new()); // nothing gathered: one call, "(none were gathered)"
        }
        let n_groups = groups.len();
        let mut verdicts: Vec<serde_json::Value> = Vec::new();
        let mut call_failed = false;
        for (gi, group) in groups.iter().enumerate() {
            let part_note = if n_groups > 1 {
                format!("\n(PART {} of {} of the gathered methodologies. Judge ONLY the methodologies shown here: list under modify only ids shown here; new_needed is true only if NONE of the methodologies shown here provides the guidance — the other parts are judged separately.)\n", gi + 1, n_groups)
            } else {
                String::new()
            };
            let mut sections = vec![
                crate::context_budget::ContextSection::new("scaffold-head", 0, head.clone()),
                crate::context_budget::ContextSection::new("request", 1, request.clone()),
                crate::context_budget::ContextSection::new("scaffold-gathered", 0, format!("{gathered_hdr}{part_note}")),
            ];
            for (_, blk) in group {
                sections.push(crate::context_budget::ContextSection::new("methodology", 3, blk.clone()));
            }
            if group.is_empty() {
                sections.push(crate::context_budget::ContextSection::new("none", 0, "(none were gathered)\n"));
            }
            sections.push(crate::context_budget::ContextSection::new("scaffold-tail", 0, tail_text));
            let assembled = crate::context_budget::assemble(sections, budget);
            run_spans::record_view_trims("meta_loop_methodology_review", "", window, window_source, want_output, &assembled);
            let prompt = assembled.text;
            let input = serde_json::json!({
                "prompt": prompt, "max_tokens": want_output, "temperature": 0.1,
                "system_prompt": "You judge whether the system's methodologies suffice for a request. Return only valid JSON."
            });
            stats.calls_made += 1;
            let res = executor.execute(PROMPT_PIPELINE_ID, input).await;
            crate::orchestrator::PromptOrchestrator::capture_loop_model_call("meta_loop_methodology_review", &prompt, &res);
            let text = match res {
                Ok(v) => v.get("response").and_then(|r| r.as_str()).unwrap_or("").to_string(),
                Err(e) => {
                    tracing::warn!(error = %e, part = gi + 1, "Methodology review: model call failed — whole review retried next pass");
                    call_failed = true;
                    break;
                }
            };
            let cands = extract_all_json_objects(&text);
            if text.trim().is_empty() || cands.len() > 1 {
                call_failed = true;
                break;
            }
            match serde_json::from_str::<serde_json::Value>(cands.into_iter().next().unwrap_or_default().trim()) {
                Ok(v) => verdicts.push(v),
                Err(_) => {
                    call_failed = true;
                    break;
                }
            }
        }
        if call_failed || verdicts.is_empty() {
            stats.failures += 1;
            continue; // attempts already counted; retried next pass
        }
        // merge the per-group verdicts into one
        let all_new = verdicts.iter().all(|v| v.get("new_needed").and_then(|b| b.as_bool()).unwrap_or(false));
        let first_new = verdicts.iter().find(|v| v.get("new_needed").and_then(|b| b.as_bool()).unwrap_or(false));
        let mut modify_all: Vec<serde_json::Value> = Vec::new();
        for v in &verdicts {
            if let Some(a) = v.get("modify").and_then(|m| m.as_array()) {
                modify_all.extend(a.iter().cloned());
            }
        }
        let verdict = serde_json::json!({
            "new_needed": all_new,
            "new_reason": first_new.and_then(|v| v.get("new_reason")).cloned().unwrap_or(serde_json::json!("")),
            "new_keywords": first_new.and_then(|v| v.get("new_keywords")).cloned().unwrap_or(serde_json::json!([])),
            "modify": modify_all,
        });
        let mut raised = 0usize;
        if verdict.get("new_needed").and_then(|b| b.as_bool()).unwrap_or(false) {
            let reason = verdict.get("new_reason").and_then(|r| r.as_str()).unwrap_or("").to_string();
            let kws: Vec<String> = verdict
                .get("new_keywords")
                .and_then(|k| serde_json::from_value(k.clone()).ok())
                .filter(|k: &Vec<String>| !k.is_empty())
                .unwrap_or_else(|| keywords.clone());
            if !kws.is_empty() {
                crate::orchestrator::PromptOrchestrator::record_methodology_gap_ex(&kws, &topics, "new", None, if reason.is_empty() { "new methodology needed for this request" } else { &reason }, &request);
                raised += 1;
            }
        }
        if let Some(arr) = verdict.get("modify").and_then(|m| m.as_array()) {
            for m in arr {
                let Some(id) = m.get("id").and_then(|i| i.as_u64()) else { continue };
                if !known_ids.contains(&id) {
                    continue; // only methodologies that were actually gathered can be modified
                }
                let reason = m.get("reason").and_then(|r| r.as_str()).unwrap_or("").to_string();
                if reason.trim().is_empty() {
                    continue;
                }
                let mut kws = keywords.clone();
                kws.push(format!("modify:{id}"));
                crate::orchestrator::PromptOrchestrator::record_methodology_gap_ex(&kws, &topics, "modify", Some(id), &reason, &request);
                raised += 1;
            }
        }
        rv["handled"] = serde_json::json!(true);
        rv["outcome"] = serde_json::json!(if raised > 0 { "gaps_raised" } else { "sufficient" });
        rv["raised"] = serde_json::json!(raised);
        stats.successes += 1;
        changed = true;
        tracing::info!(raised, "Methodology review: request judged ({})", if raised > 0 { "new/modify gaps raised" } else { "sufficient" });
    }
    if changed {
        let json = serde_json::to_string_pretty(&reviews).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| format!("write {}: {}", path, e))?;
    }
    Ok(())
}

/// Act on a `modify` gap: draft the ADDITIONS to an existing methodology (nothing existing is dropped) and apply them through
/// pipeline 12 `Update` (real read-merge-write, versioned).
async fn modify_methodology_for_gap(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
    gap: &mut serde_json::Value,
    stats: &mut PassStats,
) -> bool {
    let target = gap.get("target_id").and_then(|t| t.as_u64()).unwrap_or(0);
    let reason = gap.get("reason").and_then(|r| r.as_str()).unwrap_or("").to_string();
    let request = gap.get("request").and_then(|r| r.as_str()).unwrap_or("").to_string();
    let attempts = gap.get("attempts").and_then(|a| a.as_u64()).unwrap_or(0);
    if attempts >= 10 {
        gap["handled"] = serde_json::json!(true);
        gap["outcome"] = serde_json::json!("failed_after_max_attempts");
        return true;
    }
    gap["attempts"] = serde_json::json!(attempts + 1);
    let Some((name, kws, body)) = methodology_whole(store, target).await else {
        gap["handled"] = serde_json::json!(true);
        gap["outcome"] = serde_json::json!("target_missing");
        return true;
    };
    let (window, window_source) = reader_window(available_models, meta_fallback);
    let want_output = 1200usize;
    let budget = run_spans::input_budget_tokens(window, want_output);
    let assembled = crate::context_budget::assemble(
        vec![
            crate::context_budget::ContextSection::new("head", 0, format!("{}\n\nAn existing methodology needs ADDITIONS.\n\nREQUEST (whole):\n", crate::orchestrator::run_spans::now_line())),
            crate::context_budget::ContextSection::new("request", 1, request),
            crate::context_budget::ContextSection::new("why", 0, format!("\n\nWHAT IS MISSING: {reason}\n\nEXISTING METHODOLOGY {target}: {name}\nkeywords: {}\n", kws.join(", "))),
            crate::context_budget::ContextSection::new("body", 3, body),
            crate::context_budget::ContextSection::new(
                "keyword-rules",
                0,
                format!("\n\nKEYWORD RULES (any \"add_keywords\" you give are validated one by one before they are stored): {}", crate::shared_contracts::keyword_model::KEYWORD_RULES),
            ),
            crate::context_budget::ContextSection::new(
                "tail",
                0,
                "\n\nReturn ONE JSON object with ONLY the additions (existing content is kept as is): {\"skip\": false, \"add_principles\": [{\"name\": \"...\", \"description\": \"...\", \"priority\": 5}], \"add_heuristics\": [{\"condition\": \"...\", \"action\": \"...\"}], \"add_decision_rules\": [{\"name\": \"...\", \"condition\": \"...\", \"outcome\": \"...\", \"requires\": [{\"kind\": \"provider\", \"name\": \"<mcp provider id>\", \"purpose\": \"...\"}]}], \"add_keywords\": [\"...\"]}. Add only real, specific content. If nothing should be added: {\"skip\": true}.",
            ),
        ],
        budget,
    );
    run_spans::record_view_trims("meta_loop_methodology_modify", "", window, window_source, want_output, &assembled);
    let prompt = assembled.text;
    let input = serde_json::json!({"prompt": prompt, "max_tokens": want_output, "temperature": 0.2,
        "system_prompt": "Extend methodologies with real, specific additions only. Return only valid JSON."});
    stats.calls_made += 1;
    let res = executor.execute(PROMPT_PIPELINE_ID, input).await;
    crate::orchestrator::PromptOrchestrator::capture_loop_model_call("meta_loop_methodology_modify", &prompt, &res);
    let text = match res {
        Ok(v) => v.get("response").and_then(|r| r.as_str()).unwrap_or("").to_string(),
        Err(_) => {
            stats.failures += 1;
            return true;
        }
    };
    let cands = extract_all_json_objects(&text);
    if text.trim().is_empty() || cands.len() > 1 {
        stats.failures += 1;
        return true;
    }
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(cands.into_iter().next().unwrap_or_default().trim()) else {
        stats.failures += 1;
        return true;
    };
    if parsed.get("skip").and_then(|s| s.as_bool()).unwrap_or(false) {
        gap["handled"] = serde_json::json!(true);
        gap["outcome"] = serde_json::json!("model_judged_nothing_to_add");
        return true;
    }
    let mut upd = serde_json::json!({"action": "Update", "methodology_id": target});
    for k in ["add_principles", "add_heuristics", "add_decision_rules", "add_keywords"] {
        if let Some(v) = parsed.get(k).filter(|v| v.as_array().map(|a| !a.is_empty()).unwrap_or(false)) {
            upd[k] = v.clone();
        }
    }
    if upd.as_object().map(|o| o.len()).unwrap_or(0) <= 2 {
        stats.failures += 1;
        return true; // nothing real drafted — retried next pass
    }
    match executor.execute(METHODOLOGY_CREATE_PIPELINE_ID, upd).await {
        Ok(_) => {
            tracing::info!(methodology_id = target, "Methodology meta-loop: existing methodology extended (versioned read-merge-write)");
            gap["handled"] = serde_json::json!(true);
            gap["outcome"] = serde_json::json!("methodology_updated");
            stats.successes += 1;
        }
        Err(e) => {
            stats.failures += 1;
            tracing::warn!(error = %e, "Methodology meta-loop: Update call failed, will retry next cycle");
        }
    }
    true
}

fn gaps_log_path() -> String {
    let data_dir =
        std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    format!("{}/methodology_gaps.json", data_dir)
}

async fn review_methodology_gaps_once(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
    stats: &mut PassStats,
) -> Result<(), String> {
    let path = gaps_log_path();

    let mut gaps: Vec<serde_json::Value> = match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("parse {}: {}", path, e))?,
        Err(_) => return Ok(()), // nothing recorded yet — not an error
    };

    let mut any_change = false;

    // PER-PASS BOUND (parity with amt_loop's MAX_CANDIDATES_PER_PASS = 12;
    // efficiency sweep Tier 2 #6): a burst of many fresh gaps used to run
    // arbitrarily long in ONE pass, pushing every subsequent pass's start
    // later. Bounded waves, oldest first (store order); leftovers are named
    // loudly and run on the next tick — never silently dropped.
    const MAX_GAPS_PER_PASS: usize = 12;
    let mut attempted_this_pass = 0usize;
    let mut left_for_next_pass = 0usize;

    for gap in gaps.iter_mut() {
        if gap.get("handled").and_then(|h| h.as_bool()).unwrap_or(false) {
            continue;
        }
        if attempted_this_pass >= MAX_GAPS_PER_PASS {
            left_for_next_pass += 1;
            continue;
        }
        attempted_this_pass += 1;
        stats.candidates_consumed += 1;

        let keywords: Vec<String> = gap
            .get("keywords")
            .and_then(|k| serde_json::from_value(k.clone()).ok())
            .unwrap_or_default();
        let topics: Vec<String> = gap
            .get("topics")
            .and_then(|t| serde_json::from_value(t.clone()).ok())
            .unwrap_or_default();

        if keywords.is_empty() {
            continue;
        }

        // A gap raised by the sufficiency review KNEW what already exists: `modify` extends a named methodology; `new` skips the
        // "already covered by keyword match" shortcut below (the review judged the matches insufficient).
        let gap_kind = gap.get("kind").and_then(|k| k.as_str()).unwrap_or("new").to_string();
        let raised_by_review = !gap.get("reason").and_then(|r| r.as_str()).unwrap_or("").is_empty();
        if gap_kind == "modify" {
            if modify_methodology_for_gap(executor, store, available_models, meta_fallback, gap, stats).await {
                any_change = true;
            }
            continue;
        }

        // Cross-reference for duplication: has this gap already been
        // covered since it was recorded (by this loop on a prior cycle, or
        // by anything else)?
        let existing = store
            .search_by_keywords(&keywords, Some("Methodology"))
            .await
            .unwrap_or_default();
        // Retired shells still carry the gap's keywords verbatim (that is
        // HOW they were created — the model parroted the signal), so a
        // keyword-only coverage match keeps re-covering gaps with garbage.
        // A methodology whose name carries the retirement marker is not
        // coverage (found live 2026-09-20: shell 30420 re-covered its gaps
        // within one pass of them being re-opened).
        let mut existing = existing;
        if !existing.is_empty() {
            let mut live = Vec::new();
            for id in existing {
                let retired = store
                    .get_container(id)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|c| {
                        c.get("local_state")?.get("metadata")?.get("name")?.as_str().map(|s| s.to_string())
                    })
                    .map(|name| name.starts_with("[placeholder shell"))
                    .unwrap_or(false);
                if !retired {
                    live.push(id);
                }
            }
            existing = live;
        }
        if !existing.is_empty() && !raised_by_review {
            tracing::info!(
                ?keywords,
                covered_by = ?existing,
                "Methodology gap now covered by an existing methodology — marking handled"
            );
            gap["handled"] = serde_json::json!(true);
            any_change = true;
            continue;
        }

        // Retry cap — confirmed live this session that without one, a gap
        // whose draft call keeps coming back empty/invalid (a real,
        // observed OpenRouter behavior for some content, not a bug in this
        // loop) gets retried forever: every interval, indefinitely, burning
        // a real API call each time with no escalation and no visible
        // signal anything is actually wrong. Track attempts on the gap
        // record itself (persisted, so this survives a restart — the
        // count is never lost/reset just because the process was). After
        // MAX_DRAFT_ATTEMPTS, stop retrying and mark it failed (not
        // silently "handled" as if resolved) so it's visibly inspectable
        // rather than either retried forever or quietly dropped.
        const MAX_DRAFT_ATTEMPTS: u64 = 10;
        let attempts = gap.get("attempts").and_then(|a| a.as_u64()).unwrap_or(0);
        if attempts >= MAX_DRAFT_ATTEMPTS {
            tracing::warn!(
                ?keywords,
                attempts,
                "Methodology meta-loop: gap exceeded max draft attempts — marking failed, not retrying further"
            );
            gap["handled"] = serde_json::json!(true);
            gap["outcome"] = serde_json::json!("failed_after_max_attempts");
            any_change = true;
            continue;
        }
        gap["attempts"] = serde_json::json!(attempts + 1);
        any_change = true;

        // Model fallback escalation on retry — the first-ever attempt
        // (attempts == 0) uses the primary/default model as before;
        // attempt 2 onward escalates through the real meta_fallback chain
        // (config.toml's [models.meta_fallback] — this loop is genuinely
        // detached background work, the exact case that config was always
        // reserved for) rather than blindly repeating a primary that keeps
        // returning empty/invalid responses. CYCLES through the fallback
        // candidates (never back to the primary, which has already failed
        // at least once by then) rather than clamping to the last one —
        // confirmed live 2026-09-15: with candidates [bitnet-i2_s,
        // openrouter/free], clamping meant every attempt from #4 onward
        // retried the SAME broken openrouter/free (daily quota exhausted)
        // forever, never giving bitnet-i2_s — a real, working fallback —
        // another turn. That wasted 6 of the 10-attempt retry budget on a
        // guaranteed-fail target for one real gap. Cycling gives every
        // fallback a fair rotation on every retry instead.
        let fallback_candidates: Vec<&AvailableModel> = meta_fallback
            .order
            .iter()
            .filter_map(|id| available_models.iter().find(|m| &m.identifier == id))
            .filter(|m| !meta_fallback.free_only || m.is_free)
            .collect();
        let escalated_model = if attempts > 0 && !fallback_candidates.is_empty() {
            let idx = ((attempts - 1) as usize) % fallback_candidates.len();
            Some(fallback_candidates[idx])
        } else {
            None
        };

        tracing::info!(
            ?keywords,
            attempt = attempts + 1,
            fallback_model = ?escalated_model.map(|m| m.identifier.as_str()),
            "Methodology meta-loop: drafting a new methodology for a real gap"
        );

        // CONTEXT BUDGET (operator ruling: budgets follow the model window,
        // every cut recorded): keywords/topics are the only variable inputs;
        // the scaffold is protected and the topics trim before the keywords.
        // All-fits output is byte-identical to the old format! prompt.
        let (window, window_source) = match escalated_model {
            Some(m) => run_spans::profile_window(m.context_length),
            None => run_spans::default_model_window(),
        };
        let want_output = 800usize;
        let budget = run_spans::input_budget_tokens(window, want_output);
        let assembled = crate::context_budget::assemble(
            vec![
                crate::context_budget::ContextSection::new(
                    "scaffold-head",
                    0,
                    format!("{}\n\nA real user request's keyword signal repeatedly matched no existing methodology:\n\nKEYWORDS: ", crate::orchestrator::run_spans::now_line()),
                ),
                crate::context_budget::ContextSection::new("keywords", 2, keywords.join(", ")),
                crate::context_budget::ContextSection::new("scaffold-mid", 0, "\nTOPICS: "),
                crate::context_budget::ContextSection::new("topics", 4, topics.join(", ")),
                crate::context_budget::ContextSection::new(
                    "review-context",
                    1,
                    if raised_by_review {
                        format!(
                            "\n\nA sufficiency review judged the existing methodologies INSUFFICIENT for this request:\nWHY A NEW METHODOLOGY IS NEEDED: {}\nREQUEST (whole): {}\n",
                            gap.get("reason").and_then(|r| r.as_str()).unwrap_or(""),
                            gap.get("request").and_then(|r| r.as_str()).unwrap_or("")
                        )
                    } else {
                        String::new()
                    },
                ),
                crate::context_budget::ContextSection::new(
                    "keyword-rules",
                    0,
                    format!("\n\nKEYWORD RULES (the \"keywords\" you give are validated one by one before they are stored): {}", crate::shared_contracts::keyword_model::KEYWORD_RULES),
                ),
                crate::context_budget::ContextSection::new(
                    "scaffold-tail",
                    0,
                    "\n\nIf — and only if — this is a genuinely coherent, real, methodology-worthy\npattern (specific enough to give concrete, actionable guidance, not vague or\noverly broad), draft ONE real methodology for it. Every decision rule must be\na concrete \"if X then do Y\" a person or system could actually follow — not a\nrestatement of the keywords. Return ONLY valid JSON:\n{\n    \"name\": \"short methodology name\",\n    \"description\": \"one sentence description\",\n    \"keywords\": [\"...\", \"...\"],\n    \"topics\": [\"...\", \"...\"],\n    \"principles\": [{\"name\": \"...\", \"description\": \"...\", \"priority\": 1}],\n    \"heuristics\": [{\"condition\": \"...\", \"action\": \"...\"}],\n    \"decision_rules\": [{\"name\": \"...\", \"condition\": \"...\", \"outcome\": \"...\", \"requires\": [{\"kind\": \"provider\", \"name\": \"<MCP provider id this rule needs, e.g. threed-mcp>\", \"purpose\": \"why\", \"optional\": false}]}]\n}\n`requires` lists the MCP PROVIDERS a rule needs in order to be carried out (omit or leave [] when the rule needs none). If the keywords name a MISSING CAPABILITY (they start with \"capability\"), draft the methodology around how that capability should be provided and declare it in `requires`; never invent a provider id you were not given.\nIf this keyword set is too vague, too broad, or not actually a coherent\nmethodology-worthy pattern, return exactly: {\"skip\": true}",
                ),
            ],
            budget,
        );
        run_spans::record_view_trims(
            "meta_loop_draft",
            escalated_model.map(|m| m.identifier.as_str()).unwrap_or(""),
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
                "Methodology meta-loop: gap view trimmed to the model's window (recorded, markers emitted)"
            );
        }
        let draft_prompt = assembled.text;

        let mut draft_input = serde_json::json!({
            "prompt": draft_prompt,
            "max_tokens": 800,
            "temperature": 0.3,
            "system_prompt": "Draft real, practical methodologies only when the pattern genuinely warrants one. Return only valid JSON."
        });
        if let Some(profile) = escalated_model {
            let override_cfg = ModelConfigOverride {
                model_type: Some(profile.model_type.clone()),
                model_identifier: Some(profile.identifier.clone()),
                max_tokens: None,
                temperature: None,
                context_length: Some(profile.context_length as u32),
                api_endpoint: profile.api_endpoint.clone(),
                api_key_env: profile.api_key_env.clone(),
                api_key: profile.api_key.clone(),
                wire_protocol: profile.wire_protocol.clone(),
                bitnet_cli_path: profile.bitnet_cli_path.clone(),
                local_model_path: profile.local_model_path.clone(),
            };
            if let Ok(v) = serde_json::to_value(&override_cfg) {
                draft_input["model_override_config"] = v;
            }
        }

        stats.calls_made += 1;
        let draft_result = executor.execute(PROMPT_PIPELINE_ID, draft_input).await;
        crate::orchestrator::PromptOrchestrator::capture_loop_model_call("meta_loop_draft", &draft_prompt, &draft_result);
        let result = match draft_result {
            Ok(v) => v,
            Err(e) => {
                stats.failures += 1;
                tracing::warn!(error = %e, ?keywords, "Methodology meta-loop: draft LLM call failed, will retry next cycle");
                continue;
            }
        };

        let response_text = result.get("response").and_then(|r| r.as_str()).unwrap_or("");
        if response_text.trim().is_empty() {
            // A blank draft is a FAILED call (retried next cycle) — it used to flow into
            // extract_json_object -> "{}" and be logged as "draft had no real rules".
            stats.failures += 1;
            tracing::warn!(?keywords, "Methodology meta-loop: draft response was blank — failed attempt, will retry next cycle");
            continue;
        }
        // BitNet "confetti" (found live 2026-09-22 in decision_review.rs,
        // same vulnerability confirmed here): a response can contain
        // MULTIPLE conflicting JSON candidates (a leading empty `{}`,
        // prose, a real draft, more JSON). The old single-candidate
        // extract_json_object returns the first candidate that merely
        // parses — an empty `{}` parses fine, so `has_decision_rules`/
        // `has_heuristics` below would both silently read false, and a
        // genuinely real draft later in the response gets discarded and
        // logged as "no real decision_rules or heuristics — discarding
        // rather than persisting a shell" — indistinguishable from BitNet
        // actually producing a shell, when it may not have. Detect
        // confetti explicitly and treat it as a real failure to retry,
        // not a shell to reject.
        let candidates = extract_all_json_objects(response_text);
        if candidates.len() > 1 {
            tracing::warn!(
                count = candidates.len(),
                ?keywords,
                "Methodology meta-loop: draft response contained conflicting JSON candidates (confetti) — refusing to guess which is real, will retry next cycle"
            );
            continue;
        }
        let json_str = candidates.into_iter().next().unwrap_or_else(|| extract_json_object(response_text));
        let parsed: serde_json::Value = match serde_json::from_str(json_str.trim()) {
            Ok(v) => v,
            Err(_) => {
                tracing::warn!(raw = %response_text, ?keywords, "Methodology meta-loop: draft response wasn't valid JSON, will retry next cycle");
                continue;
            }
        };

        if parsed.get("skip").and_then(|s| s.as_bool()).unwrap_or(false) {
            tracing::info!(?keywords, "Methodology meta-loop: model judged this not methodology-worthy — marking handled");
            gap["handled"] = serde_json::json!(true);
            any_change = true;
            continue;
        }

        // Require at least one real decision_rule or heuristic before
        // persisting — a "methodology" with only principles and no
        // actionable rule is exactly the low-content shell this whole
        // effort exists to avoid creating more of.
        let has_decision_rules = parsed
            .get("decision_rules")
            .and_then(|d| d.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        let has_heuristics = parsed
            .get("heuristics")
            .and_then(|h| h.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if !has_decision_rules && !has_heuristics {
            tracing::warn!(?keywords, "Methodology meta-loop: draft had no real decision_rules or heuristics — discarding rather than persisting a shell");
            continue;
        }

        // CONTENT-DEPTH gate (live catch 2026-09-20): methodology 30420
        // passed the length gate above with one heuristic whose every field
        // was literal "..." — placeholder shells are not content, and
        // marking a gap "covered" by one corrupts the gap queue. At least
        // one rule/heuristic must carry REAL text.
        fn is_placeholder_text(s: &str) -> bool {
            let t = s.trim();
            t.is_empty() || t == "..." || t == "…"
        }
        fn entry_has_real_text(entry: &serde_json::Value, fields: &[&str]) -> bool {
            fields
                .iter()
                .any(|f| entry.get(f).and_then(|v| v.as_str()).map(|s| !is_placeholder_text(s)).unwrap_or(false))
        }
        let has_real_content = parsed
            .get("decision_rules")
            .and_then(|d| d.as_array())
            .map(|a| a.iter().any(|e| entry_has_real_text(e, &["condition", "outcome", "name", "description"])))
            .unwrap_or(false)
            || parsed
                .get("heuristics")
                .and_then(|h| h.as_array())
                .map(|a| a.iter().any(|e| entry_has_real_text(e, &["condition", "action", "when_to_apply", "description", "name"])))
                .unwrap_or(false);
        if !has_real_content {
            tracing::warn!(?keywords, "Methodology meta-loop: draft's rules/heuristics are pure placeholders (\"...\") — discarding rather than persisting a shell");
            continue;
        }

        let create_input = serde_json::json!({
            "action": "Create",
            "name": parsed.get("name").and_then(|n| n.as_str()).unwrap_or("Untitled Methodology"),
            "description": parsed.get("description").and_then(|d| d.as_str()).unwrap_or(""),
            "category_id": 0,
            "principles": parsed.get("principles").cloned().unwrap_or(serde_json::json!([])),
            "heuristics": parsed.get("heuristics").cloned().unwrap_or(serde_json::json!([])),
            "decision_rules": parsed.get("decision_rules").cloned().unwrap_or(serde_json::json!([])),
            "keywords": parsed.get("keywords").cloned().unwrap_or(serde_json::json!(keywords)),
            "topics": parsed.get("topics").cloned().unwrap_or(serde_json::json!(topics)),
        });

        match executor
            .execute(METHODOLOGY_CREATE_PIPELINE_ID, create_input)
            .await
        {
            Ok(create_result) => {
                let new_id = create_result.get("methodology_id").and_then(|m| m.as_u64());
                tracing::info!(
                    methodology_id = ?new_id,
                    ?keywords,
                    "Methodology meta-loop: created a new methodology for a real gap"
                );
                gap["handled"] = serde_json::json!(true);
                any_change = true;
                stats.successes += 1;
            }
            Err(e) => {
                stats.failures += 1;
                tracing::warn!(error = %e, ?keywords, "Methodology meta-loop: create call failed, will retry next cycle");
            }
        }
    }

    if left_for_next_pass > 0 {
        stats.notes.push(format!(
            "per-pass gap bound hit: {} attempted, {} left for the next tick",
            attempted_this_pass, left_for_next_pass
        ));
        tracing::info!(
            attempted = attempted_this_pass,
            left_for_next_pass,
            bound = MAX_GAPS_PER_PASS,
            "meta_loop pass hit its per-pass gap bound — the rest run on the next tick (batch coverage, nothing dropped)"
        );
    }

    if any_change {
        let json = serde_json::to_string_pretty(&gaps).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| format!("write {}: {}", path, e))?;
    }

    Ok(())
}

/// Confetti detection (found live 2026-09-22, `decision_review.rs`; same
/// vulnerability confirmed and fixed here): scans the WHOLE response for
/// every balanced, parseable `{...}` candidate, not just the first, and
/// returns only the non-empty ones (a bare `{}` carries nothing and would
/// otherwise masquerade as "the model said nothing useful"). More than one
/// candidate means the response is confetti — multiple conflicting JSON
/// objects in one generation — which the caller treats as a real failure
/// requiring retry, not a shell draft to reject.
fn extract_all_json_objects(s: &str) -> Vec<String> {
    let bytes = s.as_bytes();
    let mut found = Vec::new();
    let mut start = 0usize;
    while start < bytes.len() {
        if bytes[start] != b'{' {
            start += 1;
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
        match end {
            Some(end) => {
                let candidate = &s[start..=end];
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(candidate) {
                    if parsed.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
                        found.push(candidate.to_string());
                    }
                }
                start = end + 1;
            }
            None => start += 1,
        }
    }
    found
}

/// Extract the first well-formed JSON object from a raw LLM response.
/// Small local models (BitNet i2_s — live-observed 2026-09-20) wrap usable
/// JSON in prompt echo, `skip:` scaffolding, code fences, and hallucinated
/// follow-up instructions; a naive first-`{`-to-last-`}` slice spans all of
/// that noise and fails to parse every time ("trailing characters"). Scans
/// every `{` as a candidate start, tracks brace depth string-aware, and
/// returns the first candidate that both balances AND parses as JSON — an
/// answer embedded in noise is rescued instead of burned as a failed
/// attempt.
fn extract_json_object(s: &str) -> String {
    let bytes = s.as_bytes();
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
            let candidate = &s[start..=end];
            if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                return candidate.to_string();
            }
        }
    }
    "{}".to_string()
}

/// Awaiting drain (2026-11-01 doctrine): wake every walk parked on a
/// meta/consciousness gate whose backoff elapsed so it re-asks meta now.
/// Bounded to 12 items per pass. Walks re-judge themselves (stage 8 loop);
/// items with no live walk (e.g. after restart) stay `awaiting` and are
/// surfaced by `awaiting::pending_actions` for the resume driver.
/// Returns the number of due items woken.
pub fn drain_awaiting_once(data_dir: &str) -> usize {
    let due = super::awaiting::due_items(data_dir, 12);
    if !due.is_empty() {
        super::awaiting::await_wake().notify_waiters();
    }
    due.len()
}
