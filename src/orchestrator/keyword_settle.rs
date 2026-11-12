//! KEYWORD SETTLEMENT — the zero-shot expansion that runs LAST (2026-11-08; operator: "the zero-shot expansion is left at the end — once it
//! runs it identifies all true keywords to remain, all that actually fits, and then validates what are hits … no false confidence").
//! docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §5.
//!
//! A meta-loop actor pass, OFF the walk's critical path, incremental per container, no cap on how many containers it settles (progress,
//! not a count — it simply walks every unsettled container, most-searched types first):
//!   * Pass A — deterministic verbatim capture (no model): methodology identifiers (provider/tool names, principle and rule names),
//!     project/workspace roll-up of their children's meaning terms. Idempotent; adds only what is missing. Identifier / roll-up terms are
//!     facts read from the content's own structure and count as VALIDATED by construction.
//!   * Pass V1 — THE VALIDATION GATE for what is already stored (operator 2026-11-10: every stored keyword must be validated): batched
//!     strength-N rounds ("is this a true keyword for THIS container?"); ASYMMETRIC (a drop needs unanimity): a stored term is REMOVED only if
//!     EVERY round says NO, one YES keeps and validates it; a stored term is never removed when no content was readable; an unanswered term
//!     stays pending.
//!   * Pass B — ONE expansion call per container (only types where keyword search has been observed to hit), seeded with VALIDATED terms
//!     only: alternative request phrasings / synonyms / translations. Candidates are NOT stored here.
//!   * Pass V2 — the same gate for the new candidates, in the opposite polarity: a candidate is stored only if EVERY round says YES.
//!   * Pass C/D — a term whose real hit evidence turned negative (dropped > kept + applied) and that was never affirmed gets the strength-N
//!     "is X NOT a true keyword?" vote (unanimous YES removes). Nothing is deleted by a counter alone.
//! A container is SETTLED when every meaning keyword is VALIDATED (`keyword_program::term_is_validated`); a model failure leaves terms pending
//! (retried next pass). JurisdictionRuleSet and bookkeeping types are never gated (`keyword_program::GATED_TYPES`). Kill switch:
//! `OZONE_KEYWORD_SETTLE=off`. Markers (`ns:value`) are never sent to the model and never touched.

use super::meta_loop::{methodology_whole, reader_window};
use crate::config::{AvailableModel, ModelFallbackConfig};
use crate::keyword_program::CONTAINER_TOTALS_KEY;
use crate::orchestrator::run_spans::{self, PassStats};
use crate::orchestrator::{PipelineExecutor, PromptOrchestrator, StoreAccess};
use crate::shared_contracts::{k_validation, keyword_model as km};
use crate::types::container::{Container, ContainerScope, ContainerType, TermMeta};
use crate::types::zsei::ContainerUpdate;
use std::collections::BTreeMap;
use std::sync::Arc;

const PROMPT_PIPELINE_ID: u64 = 9;

/// Container types in settlement order — the most-searched / routing-critical first.
const SETTLE_ORDER: &[&str] = crate::keyword_program::GATED_TYPES; // the types the write-time gate covers = the types settlement validates

/// The provenance a legacy keyword gets when the model call succeeds: modalities/files/tools produce their terms mechanically.
fn legacy_source(container_type: &str) -> &'static str {
    match container_type {
        "ModalityGraph" | "FileReference" | "Pipeline" | "McpEntity" | "Project" | "Workspace" => "verbatim",
        _ => "extracted",
    }
}

/// Reserved `keyword_meta` key (not a keyword) holding a Project/Workspace's ROLL-UP SIGNATURE in `source`: the child set the roll-up last
/// folded. When the child set changes the signature no longer matches and the container is unsettled again — the roll-up refreshes.
pub const ROLLUP_KEY: &str = "*rollup";

/// A signature of a container's child set (count + an order-independent hash of the ids).
pub fn rollup_signature(c: &Container) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut sum: u64 = 0;
    for id in &c.global_state.child_ids {
        h ^= id.wrapping_mul(0x9e3779b97f4a7c15);
        sum = sum.wrapping_add(*id);
    }
    format!("{}:{:016x}:{:x}", c.global_state.child_ids.len(), h, sum)
}

fn is_rollup_type(c: &Container) -> bool {
    matches!(c.local_state.metadata.container_type, ContainerType::Project | ContainerType::Workspace)
}

/// SETTLED = every meaning keyword has provenance in `keyword_meta` AND has passed the validation gate (Pass V), and (Project/Workspace) the roll-up reflects the CURRENT child set.
pub fn is_settled(c: &Container) -> bool {
    let ctx = &c.local_state.context;
    if is_rollup_type(c) && ctx.keyword_meta.get(ROLLUP_KEY).map(|m| m.source.as_str()) != Some(rollup_signature(c).as_str()) {
        return false;
    }
    km::meaning_terms(&ctx.keywords).iter().all(|k| ctx.keyword_meta.get(k).map(|m| !m.source.is_empty()).unwrap_or(false) && crate::keyword_program::term_is_validated(ctx.keyword_meta.get(k)))
}

fn push_unique(list: &mut Vec<String>, term: &str) -> bool {
    let t = term.trim();
    if t.is_empty() || km::is_marker(t) || km::is_sentence_like(t) {
        return false;
    }
    if list.iter().any(|x| x.trim().eq_ignore_ascii_case(t)) {
        return false;
    }
    list.push(t.to_string());
    true
}

/// Node fields that never carry a search identifier (content, bookkeeping, geometry, provenance, nested objects). This is a SCHEMA field list,
/// not a word list: it names the modality graph schema's own bookkeeping keys.
const GRAPH_SKIP_FIELDS: &[&str] = &[
    "content", "provenance", "materialized_path", "position", "created_by_step", "embedding_hint", "version_notes", "node_type", "node_id",
    "properties", "annotations", "semantic_annotations", "cross_modal_refs", "provisional_status", "bounding_box", "location", "geo_location",
    "pixel_location", "version", "range", "time_range", "frame_range",
];

/// The IDENTIFIERS a modality graph FILE carries, generically (all 27 modalities, no per-modality table): every node's `keywords`, `label`
/// and `name`, plus any other short (≤ 3 words, ≤ 40 chars), non-numeric string field not in the schema skip list (e.g. `material_name`,
/// `accession`, `gene_name`, `emitter_class`, `classification`). Most frequent first. Pure.
pub fn graph_identifiers(graph: &serde_json::Value) -> Vec<String> {
    let mut freq: BTreeMap<String, usize> = BTreeMap::new();
    let mut add = |t: &str| {
        let t = t.trim();
        if t.is_empty() || t.len() > 40 || t.split_whitespace().count() > 3 || t.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
            return;
        }
        *freq.entry(t.to_string()).or_insert(0) += 1;
    };
    for n in graph.get("nodes").and_then(|x| x.as_array()).into_iter().flatten() {
        let Some(obj) = n.as_object() else { continue };
        for (k, v) in obj {
            if k == "keywords" {
                for kw in v.as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                    add(kw);
                }
            } else if k == "label" || k == "name" {
                if let Some(t) = v.as_str() {
                    add(t);
                }
            } else if !GRAPH_SKIP_FIELDS.contains(&k.as_str()) {
                if let Some(t) = v.as_str() {
                    add(t);
                }
            }
        }
    }
    let mut ranked: Vec<(String, usize)> = freq.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    km::meaning_terms(&ranked.into_iter().map(|(t, _)| t).collect::<Vec<_>>())
}

/// The file extension of a name when it looks like one (1–8 alphanumerics after the last dot).
fn file_extension(name: &str) -> Option<String> {
    let ext = name.rsplit_once('.')?.1;
    if (1..=8).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric()) && !name.ends_with('.') {
        Some(ext.to_lowercase())
    } else {
        None
    }
}

/// Pass A — deterministic additions `(term, source)` for one container. No model call.
async fn pass_a(store: &Arc<dyn StoreAccess>, id: u64, c: &Container, ctype: &str) -> Vec<(String, &'static str)> {
    let mut adds: Vec<(String, &'static str)> = Vec::new();
    match ctype {
        "Methodology" => {
            if let Some((_name, _kws, body)) = methodology_whole(store, id).await {
                if let Ok(b) = serde_json::from_str::<serde_json::Value>(&body) {
                    // principle names, decision-rule names, and every declared provider/tool name are verbatim identifiers of the methodology
                    for p in b.get("principles").and_then(|x| x.as_array()).into_iter().flatten() {
                        if let Some(n) = p.get("name").and_then(|x| x.as_str()) {
                            adds.push((n.to_string(), "verbatim"));
                        }
                    }
                    for r in b.get("decision_rules").and_then(|x| x.as_array()).into_iter().flatten() {
                        if let Some(n) = r.get("name").and_then(|x| x.as_str()) {
                            adds.push((n.to_string(), "verbatim"));
                        }
                        for q in r.get("requires").and_then(|x| x.as_array()).into_iter().flatten() {
                            if let Some(n) = q.get("name").and_then(|x| x.as_str()) {
                                adds.push((n.to_string(), "identifier"));
                            }
                        }
                    }
                }
            }
        }
        "ModalityGraph" => {
            // the graph FILE's own identifiers (generic across modalities) — back-fills the keyword-less graphs without a per-modality edit
            if let Some(rel) = c.local_state.storage.object_store_path.as_deref() {
                let path = crate::keyword_program::data_dir().join(rel);
                if let Some(g) = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
                    for t in graph_identifiers(&g) { // every identifier — no per-graph term cap (operator 2026-11-11); memory is guarded by the floor wait, not a count
                        adds.push((t, "identifier"));
                    }
                }
            }
        }
        "FileReference" => {
            if let Some(ext) = c.local_state.metadata.name.as_deref().and_then(file_extension) {
                adds.push((ext, "identifier"));
            }
        }
        "Project" | "Workspace" => {
            // roll-up: the meaning terms of the children, most frequent first
            let mut freq: BTreeMap<String, usize> = BTreeMap::new();
            for child in &c.global_state.child_ids {
                if let Ok(Some(v)) = store.get_container(*child).await {
                    if let Ok(cc) = serde_json::from_value::<Container>(v) {
                        for t in km::meaning_terms(&cc.local_state.context.keywords) {
                            *freq.entry(t.to_lowercase()).or_insert(0) += 1;
                        }
                    }
                }
            }
            let mut ranked: Vec<(String, usize)> = freq.into_iter().collect();
            ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            for (t, _) in ranked { // every rolled-up term, most frequent first — no per-container term cap (operator 2026-11-11)
                adds.push((t, "rollup"));
            }
        }
        _ => {}
    }
    adds
}

async fn vote_not_keyword(
    executor: &Arc<dyn PipelineExecutor>,
    container_label: &str,
    term: &str,
    others: &[String],
    stats: &mut PassStats,
) -> bool {
    let strength = k_validation::configured_strength();
    let policy = k_validation::ValidationPolicy { strength };
    let prompt = format!(
        "CONTAINER: {container_label}\nITS KEYWORDS: {}\n\nQUESTION: Is \"{term}\" NOT a true, content-bearing keyword for this container — i.e. it is a pronoun, filler word, markup, a fragment of a sentence, or otherwise says nothing about what the container is about? Answer YES only if it is clearly not a real keyword. Answer NO if it is a real keyword or you are unsure.",
        others.join(", ")
    );
    let ex = executor.clone();
    let calls = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let c2 = calls.clone();
    let yes = k_validation::confirm_consecutive_yes_with(
        move || {
            let input = k_validation::yes_no_input(&prompt);
            let ex = ex.clone();
            let c2 = c2.clone();
            async move {
                c2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                match ex.execute(PROMPT_PIPELINE_ID, input).await {
                    Ok(r) => r.get("response").and_then(|x| x.as_str()).and_then(k_validation::parse_yes_no),
                    Err(_) => None,
                }
            }
        },
        &policy,
    )
    .await;
    stats.calls_made += calls.load(std::sync::atomic::Ordering::Relaxed);
    crate::keyword_program::record_judgment("keyword_keeper", None, term, container_label, "is this NOT a true keyword for the container?", yes, strength);
    yes
}

/// The outcome of the validation gate for one container.
#[derive(Debug, Default)]
pub struct Validation {
    /// passed every round — safe to store / mark validated
    pub validated: Vec<String>,
    /// answered NO in some round — removed / never stored
    pub rejected: Vec<String>,
    /// no usable answer (model failure, missing or contradictory entry) — neither validated nor rejected; retried next pass
    pub unresolved: Vec<String>,
}

/// Split `terms` into chunks that fit `max_tokens` of input (about one token per three characters plus list overhead) and hold at most
/// `max_items` (one reply must be able to answer them all). NEVER drops a term: the rest simply goes into the next chunk.
pub fn chunk_terms(terms: &[String], max_tokens: usize, max_items: usize) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    let mut used = 0usize;
    for t in terms {
        let cost = t.chars().count() / 3 + 4;
        if !cur.is_empty() && (used + cost > max_tokens || cur.len() >= max_items.max(1)) {
            out.push(std::mem::take(&mut cur));
            used = 0;
        }
        used += cost;
        cur.push(t.clone());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Read one validation round's reply: 1-based numbers in `yes` / `no` (numbers or numeric strings, in range). A number in BOTH lists is
/// contradictory and counts as NO VOTE, like an absent one — a blank or garbled answer is never read as a verdict.
pub fn parse_round(v: &serde_json::Value, n: usize) -> (std::collections::BTreeSet<usize>, std::collections::BTreeSet<usize>) {
    let nums = |key: &str| -> std::collections::BTreeSet<usize> {
        v.get(key)
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_u64().map(|n| n as usize).or_else(|| x.as_str().and_then(|s| s.trim().parse().ok()))).filter(|i| *i >= 1 && *i <= n).collect())
            .unwrap_or_default()
    };
    let (yes, no) = (nums("yes"), nums("no"));
    let both: std::collections::BTreeSet<usize> = yes.intersection(&no).copied().collect();
    (yes.difference(&both).copied().collect(), no.difference(&both).copied().collect())
}

fn model_json<E>(res: &Result<serde_json::Value, E>) -> Option<serde_json::Value> {
    match res {
        Ok(r) => {
            let text = r.get("response").and_then(|x| x.as_str()).unwrap_or("");
            if text.trim().is_empty() {
                None
            } else {
                serde_json::from_str(PromptOrchestrator::extract_json_object_shared(text).trim()).ok()
            }
        }
        Err(_) => None,
    }
}

/// What one round did to one chunk of terms. Pure (no model, no store) so both modes are unit-tested.
#[derive(Debug, Default, PartialEq)]
pub struct RoundStep {
    /// terms that go on to the next round
    pub next: Vec<String>,
    pub validated: Vec<String>,
    pub rejected: Vec<String>,
    pub unresolved: Vec<String>,
}

/// Apply one round's answers to a chunk. The two modes are deliberately ASYMMETRIC (operator doctrine: a drop needs unanimity):
/// * `existing` (a term ALREADY stored): a YES from any round KEEPS and VALIDATES it (dissent from "drop" is enough); a NO only sends it to
///   the next round — it is REMOVED only if EVERY round said NO.
/// * new candidate (not stored yet): a NO REJECTS it at once; a YES only sends it to the next round — it is STORED only if EVERY round
///   said YES.
/// A term the reply did not answer (absent / contradictory / the call failed) is `unresolved`: neither kept nor dropped, retried next pass.
pub fn step_round(existing: bool, chunk: &[String], yes: &std::collections::BTreeSet<usize>, no: &std::collections::BTreeSet<usize>) -> RoundStep {
    let mut out = RoundStep::default();
    for (i, t) in chunk.iter().enumerate() {
        let n = i + 1;
        let (is_yes, is_no) = (yes.contains(&n), no.contains(&n));
        match (existing, is_yes, is_no) {
            (true, true, _) => out.validated.push(t.clone()),
            (true, false, true) => out.next.push(t.clone()),
            (false, _, true) => out.rejected.push(t.clone()),
            (false, true, false) => out.next.push(t.clone()),
            _ => out.unresolved.push(t.clone()),
        }
    }
    out
}

/// THE GATE. `configured_strength()` rounds; each round is BATCHED (one call carries many terms, chunked to the reading model's window) and
/// asks "is this a true keyword for this container?" per numbered term; only the terms still undecided continue (see `step_round` for the
/// two modes). A STORED term costs ONE round when the model affirms it and N rounds only when it keeps saying NO; a NEW candidate costs one
/// round when refused and N when it is accepted. Each round presents the terms in a different rotation so the votes are not the same
/// prompt repeated. A stored term is NEVER removed when `evidence` is false (no readable content). Every outcome is logged as a labelled
/// sample, with the round it was decided in.
#[allow(clippy::too_many_arguments)]
async fn validate_terms(
    executor: &Arc<dyn PipelineExecutor>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
    container_id: u64,
    label: &str,
    body: &str,
    topics: &[String],
    terms: Vec<String>,
    existing: bool,
    evidence: bool,
    stats: &mut PassStats,
) -> Validation {
    const QUESTION: &str = "is this a true, content-bearing keyword for the container?";
    let strength = k_validation::configured_strength().max(1);
    let (window, window_source) = reader_window(available_models, meta_fallback);
    let want_output = 1500usize;
    let budget = run_spans::input_budget_tokens(window, want_output);
    let mut out = Validation::default();
    let mut alive: Vec<String> = terms;
    for round in 1..=strength {
        if alive.is_empty() {
            break;
        }
        let mut next: Vec<String> = Vec::new();
        for mut chunk in chunk_terms(&alive, (budget / 2).max(64), 200) {
            let rot = ((round - 1) as usize * chunk.len()) / strength as usize;
            let rot = rot.min(chunk.len().saturating_sub(1));
            chunk.rotate_left(rot);
            let list = chunk.iter().enumerate().map(|(i, t)| format!("{}. {}", i + 1, t)).collect::<Vec<_>>().join("\n");
            let head = format!("{}\n\nCONTAINER: {label}\nTOPICS: {}\n\nCANDIDATE KEYWORDS (numbered):\n{list}\n", run_spans::now_line(), topics.join(", "));
            let tail = format!(
                "\n{}\n\nQUESTION: For EACH numbered candidate, is it a TRUE keyword for THIS container — does it name something this container is genuinely about, so that a person searching for it should be shown this container? Answer NO for filler words, pronouns, fragments, markup, the system's own bookkeeping labels, and terms for subject matter the container does not cover. Judge each candidate on its own merit.\nReturn ONLY JSON: {{\"yes\": [numbers], \"no\": [numbers]}} — every number in exactly one list.",
                km::KEYWORD_RULES
            );
            let mut sections = vec![crate::context_budget::ContextSection::new("scaffold-head", 0, head)];
            if !body.is_empty() {
                sections.push(crate::context_budget::ContextSection::new("body", 3, format!("\nCONTENT (whole):\n{body}\n")));
            }
            sections.push(crate::context_budget::ContextSection::new("scaffold-tail", 0, tail));
            let assembled = crate::context_budget::assemble(sections, budget);
            run_spans::record_view_trims("keyword_validate", "", window, window_source, want_output, &assembled);
            let prompt = assembled.text;
            let input = serde_json::json!({
                "prompt": prompt, "max_tokens": want_output, "temperature": 0.1,
                "system_prompt": "You validate search keywords for graph containers. Return only valid JSON."
            });
            stats.calls_made += 1;
            let res = executor.execute(PROMPT_PIPELINE_ID, input).await;
            PromptOrchestrator::capture_loop_model_call("keyword_validate", &prompt, &res);
            match model_json(&res) {
                None => out.unresolved.extend(chunk),
                Some(v) => {
                    let (yes, no) = parse_round(&v, chunk.len());
                    let step = step_round(existing, &chunk, &yes, &no);
                    for t in &step.validated {
                        crate::keyword_program::record_judgment("keyword_validation", Some(container_id), t, label, &format!("{QUESTION} [YES in round {round} of {strength}; stored term kept]"), true, strength);
                    }
                    for t in &step.rejected {
                        crate::keyword_program::record_judgment("keyword_validation", Some(container_id), t, label, &format!("{QUESTION} [NO in round {round} of {strength}; new candidate refused]"), false, strength);
                    }
                    out.validated.extend(step.validated);
                    out.rejected.extend(step.rejected);
                    out.unresolved.extend(step.unresolved);
                    next.extend(step.next);
                }
            }
        }
        alive = next;
    }
    // what is left after every round: a NEW candidate said YES every time (store it); a STORED term said NO every time (remove it)
    for t in alive {
        if existing && !evidence {
            // NO EVIDENCE, NO REMOVAL: the model had nothing but a title to check this term against (live EU 2026-11-10: `ufw`, `fail2ban`,
            // `loopback` were refused for a methodology whose body was never read). A stored term is removed only when the judge could see the
            // container's content; otherwise it stays pending and is retried once the content is readable.
            out.unresolved.push(t);
        } else if existing {
            crate::keyword_program::record_judgment("keyword_validation", Some(container_id), &t, label, &format!("{QUESTION} [NO in all {strength} rounds; stored term removed]"), false, strength);
            out.rejected.push(t);
        } else {
            crate::keyword_program::record_judgment("keyword_validation", Some(container_id), &t, label, &format!("{QUESTION} [YES in all {strength} rounds; new candidate stored]"), true, strength);
            out.validated.push(t);
        }
    }
    out
}

/// Types whose keywords have been OBSERVED to produce search hits — the only ones worth a zero-shot expansion call. (Project / Workspace /
/// ModalityGraph keywords are deterministic roll-ups / graph identifiers and never produced a hit in any source; Category / FileReference
/// keywords are the name / the extension.) Revisit when the hit log shows hits on another type.
fn expandable(ctype: &str) -> bool {
    matches!(ctype, "Methodology" | "Blueprint" | "Pipeline" | "McpEntity")
}

/// Write a validation outcome into the container: passed terms are stored (new) / marked validated (existing); failed terms are removed
/// (existing) / never stored (new) and REMEMBERED as rejected so they are never proposed or stored again; unresolved terms are left alone.
fn apply_validation(c: &mut Container, outcome: &Validation, new_candidates: &[String], ctype: &str, changed: &mut bool) {
    for t in &outcome.validated {
        let is_new = new_candidates.iter().any(|x| x.eq_ignore_ascii_case(t));
        if is_new {
            push_unique(&mut c.local_state.context.keywords, t); // stored only now, after it passed
        }
        let key = c.local_state.context.keywords.iter().find(|k| k.trim().eq_ignore_ascii_case(t)).cloned().unwrap_or_else(|| t.clone());
        let e = c.local_state.context.keyword_meta.entry(key).or_default();
        e.validated = Some(true);
        if e.source.is_empty() {
            e.source = if is_new { "expansion".to_string() } else { legacy_source(ctype).to_string() };
        }
        *changed = true;
    }
    for t in &outcome.rejected {
        let was_stored = c.local_state.context.keywords.iter().any(|k| k.trim().eq_ignore_ascii_case(t));
        c.local_state.context.keywords.retain(|k| !k.trim().eq_ignore_ascii_case(t));
        let e = c.local_state.context.keyword_meta.entry(t.clone()).or_default();
        e.validated = Some(false);
        if e.source.is_empty() {
            e.source = if was_stored { legacy_source(ctype).to_string() } else { "expansion".to_string() };
        }
        *changed = true;
    }
}

/// Settle ONE container. Returns Ok(true) when something was written.
async fn settle_container(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
    id: u64,
    ctype: &str,
    stats: &mut PassStats,
) -> Result<bool, String> {
    let Some(v) = store.get_container(id).await? else { return Ok(false) };
    let mut c: Container = serde_json::from_value(v).map_err(|e| e.to_string())?;
    let name = c.local_state.metadata.name.clone().unwrap_or_default();
    let label = format!("{ctype} \"{name}\"");
    let mut changed = false;

    // ---- Pass A (deterministic)
    let adds_a = pass_a(store, id, &c, ctype).await;
    for (term, source) in adds_a {
        if push_unique(&mut c.local_state.context.keywords, &term) {
            let key = c.local_state.context.keywords.last().cloned().unwrap_or(term);
            let deterministic = matches!(source, "identifier" | "rollup"); // read from the content's own structure — a fact, not a guess
            c.local_state.context.keyword_meta.entry(key).or_insert_with(|| TermMeta { source: source.to_string(), validated: if deterministic { Some(true) } else { None }, hits: Default::default() });
            changed = true;
        }
    }

    // roll-up signature: remember WHICH child set Pass A folded (refreshes when children are added or removed)
    if is_rollup_type(&c) {
        let sig = rollup_signature(&c);
        let slot = c.local_state.context.keyword_meta.entry(ROLLUP_KEY.to_string()).or_default();
        if slot.source != sig {
            slot.source = sig;
            changed = true;
        }
    }

    let topics = c.local_state.context.topics.clone();
    let body = if ctype == "Methodology" { methodology_whole(store, id).await.map(|(_, _, b)| b).unwrap_or_default() } else { String::new() };
    let mut model_ok = true;

    // ---- Pass V1 — THE VALIDATION GATE for what is ALREADY stored (operator 2026-11-10: every stored keyword must be validated). Nothing
    // unvalidated may seed the expansion below, so the existing terms are judged first. A term that fails any strength-N round is REMOVED;
    // a model failure leaves it pending (the container stays unsettled and is retried next pass).
    let pending: Vec<String> = km::meaning_terms(&c.local_state.context.keywords)
        .into_iter()
        .filter(|k| !crate::keyword_program::term_is_validated(c.local_state.context.keyword_meta.get(k)))
        .collect();
    if !pending.is_empty() {
        let outcome = validate_terms(executor, available_models, meta_fallback, id, &label, &body, &topics, pending, true, !body.is_empty(), stats).await;
        apply_validation(&mut c, &outcome, &[], ctype, &mut changed);
        if !outcome.unresolved.is_empty() {
            model_ok = false;
        }
    }

    // ---- Pass B — EXPANSION (one call), seeded with VALIDATED terms only, and only for the types where keyword search has been observed to
    // hit (measured 2026-11-10: Project / Workspace / ModalityGraph expansion terms — 3,597 of 11,186 locally — never produced one hit).
    // Candidates are NOT stored here: they go to Pass V2 first.
    let mut new_candidates: Vec<String> = Vec::new();
    if expandable(ctype) && model_ok {
        let seeds: Vec<String> = km::meaning_terms(&c.local_state.context.keywords);
        let (window, window_source) = reader_window(available_models, meta_fallback);
        let want_output = 800usize;
        let budget = run_spans::input_budget_tokens(window, want_output);
        let head = format!(
            "{}\n\nA container in a graph store needs good SEARCH keywords. {}\n\nSearch matches whole words/phrases (no stemming), so think of how a person would actually ASK for this thing.\n\nCONTAINER: {label}\nVALIDATED KEYWORDS: {}\nTOPICS: {}\n",
            run_spans::now_line(),
            km::KEYWORD_RULES,
            seeds.join(", "),
            topics.join(", ")
        );
        let tail = "\nReturn ONLY JSON: {\"expansion\": [\"...\"]}\n- expansion: alternative request phrasings, synonyms, aliases and translations in the content's own language(s) that are NOT already listed and NOT trivial rewordings of a listed term. An empty list is correct when there is nothing genuinely new.";
        let mut sections = vec![crate::context_budget::ContextSection::new("scaffold-head", 0, head)];
        if !body.is_empty() {
            sections.push(crate::context_budget::ContextSection::new("body", 3, format!("\nCONTENT (whole):\n{body}\n")));
        }
        sections.push(crate::context_budget::ContextSection::new("scaffold-tail", 0, tail.to_string()));
        let assembled = crate::context_budget::assemble(sections, budget);
        run_spans::record_view_trims("keyword_settle", "", window, window_source, want_output, &assembled);
        let prompt = assembled.text;
        let input = serde_json::json!({
            "prompt": prompt, "max_tokens": want_output, "temperature": 0.1,
            "system_prompt": "You improve search keywords for graph containers. Return only valid JSON."
        });
        stats.calls_made += 1;
        let res = executor.execute(PROMPT_PIPELINE_ID, input).await;
        PromptOrchestrator::capture_loop_model_call("keyword_settle", &prompt, &res);
        match model_json(&res) {
            None => model_ok = false,
            Some(p) => {
                let expansion: Vec<String> = p.get("expansion").and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.trim().to_string())).collect()).unwrap_or_default();
                for t in expansion {
                    if t.is_empty() || t.split_whitespace().count() > 3 || km::is_marker(&t) || km::is_sentence_like(&t) {
                        continue;
                    }
                    let ctx = &c.local_state.context;
                    let already = ctx.keywords.iter().any(|k| k.trim().eq_ignore_ascii_case(&t));
                    let rejected_before = ctx.keyword_meta.get(&t).or_else(|| ctx.keyword_meta.get(&t.to_lowercase())).map(|m| m.validated == Some(false)).unwrap_or(false);
                    if !already && !rejected_before && !new_candidates.iter().any(|x| x.eq_ignore_ascii_case(&t)) {
                        new_candidates.push(t);
                    }
                }
            }
        }
    }

    // ---- Pass V2 — the SAME gate for the new candidates: stored only if they pass every round; never stored otherwise
    if !new_candidates.is_empty() {
        let outcome = validate_terms(executor, available_models, meta_fallback, id, &label, &body, &topics, new_candidates.clone(), false, !body.is_empty(), stats).await;
        apply_validation(&mut c, &outcome, &new_candidates, ctype, &mut changed);
        if !outcome.unresolved.is_empty() {
            model_ok = false;
        }
    }

    // ---- Pass C/D: terms whose REAL hit evidence turned negative and that were never affirmed get the unanimous-drop judgment
    let current: Vec<String> = km::meaning_terms(&c.local_state.context.keywords);
    let mut candidates: Vec<String> = Vec::new();
    for k in &current {
        if let Some(m) = c.local_state.context.keyword_meta.get(k) {
            if m.hits.dropped > m.hits.kept + m.hits.applied && m.validated != Some(true) {
                candidates.push(k.clone());
            }
        }
    }
    if model_ok || !candidates.is_empty() {
        for term in candidates {
            let unanimous = vote_not_keyword(executor, &label, &term, &current, stats).await;
            if unanimous {
                c.local_state.context.keywords.retain(|k| !k.trim().eq_ignore_ascii_case(&term));
                let e = c.local_state.context.keyword_meta.entry(term.clone()).or_default();
                e.validated = Some(false);
                if e.source.is_empty() {
                    e.source = legacy_source(ctype).to_string();
                }
            } else {
                let e = c.local_state.context.keyword_meta.entry(term.clone()).or_default();
                e.validated = Some(true);
                if e.source.is_empty() {
                    e.source = legacy_source(ctype).to_string();
                }
            }
            changed = true;
        }
    }

    // ---- provenance for every remaining legacy keyword — ONLY after the model call succeeded (a failure leaves the container unsettled)
    if model_ok {
        let src = legacy_source(ctype);
        for k in km::meaning_terms(&c.local_state.context.keywords) {
            let slot = c.local_state.context.keyword_meta.entry(k).or_default();
            if slot.source.is_empty() {
                slot.source = src.to_string();
                changed = true;
            }
        }
    } else {
        stats.failures += 1;
    }

    // totals key is never a keyword: make sure it did not acquire a source by accident
    if let Some(star) = c.local_state.context.keyword_meta.get_mut(CONTAINER_TOTALS_KEY) {
        star.source.clear();
    }
    if !changed {
        return Ok(false);
    }
    let upd = ContainerUpdate { metadata: None, context: Some(c.local_state.context.clone()), storage: None, hints: None };
    store.update_container(id, serde_json::to_value(&upd).map_err(|e| e.to_string())?).await?;
    Ok(true)
}

/// Marker-heavy container types whose legacy rows carry `ws:` / `proj:` / `scope:global` ONLY as keywords.
const SCOPE_BACKFILL_TYPES: &[&str] = &["CoordinationEvent", "Derived", "ChatTurn", "ChatSession", "McpEntity", "McpResult"];

/// TYPED-SCOPE BACKFILL (2026-11-08): every create/update now derives `Context.scope` from the marker keywords centrally; legacy containers
/// that nobody has touched since have markers but an empty typed scope. This pass sends each such container ONE context-only update (the
/// store derives the scope on write). Deterministic, no model call, idempotent (a container with a typed scope or no markers is skipped).
/// Returns how many containers were updated.
pub async fn backfill_scope_once(store: &Arc<dyn StoreAccess>, stats: &mut PassStats) -> usize {
    let mut n = 0usize;
    for &t in SCOPE_BACKFILL_TYPES {
        let Ok(ids) = store.list_ids(Some(t)).await else { continue };
        for id in ids {
            let Ok(Some(v)) = store.get_container(id).await else { continue };
            let Ok(c) = serde_json::from_value::<Container>(v) else { continue };
            let ctx = &c.local_state.context;
            if !ctx.scope.is_empty() || ContainerScope::from_markers(&ctx.keywords).is_empty() {
                continue;
            }
            let upd = ContainerUpdate { metadata: None, context: Some(ctx.clone()), storage: None, hints: None };
            if let Ok(uv) = serde_json::to_value(&upd) {
                if store.update_container(id, uv).await.is_ok() {
                    n += 1;
                }
            }
        }
    }
    if n > 0 {
        stats.candidates_consumed += n as u64;
        stats.notes.push(format!("typed-scope backfill: {n} container(s)"));
        tracing::info!(updated = n, "keyword program: typed scope backfilled from marker keywords");
    }
    n
}

/// ONE-TIME LEGACY HOTNESS SCRUB (2026-11-08): `hints.hotness_score` is now the single stored source of hotness and the ranking reads it as is, so
/// every value written before the rule "hotness = observed use only" must go. A container whose stored hotness is non-zero but that has NO observed
/// traced returns (`keyword_meta["*"].hits.returned_traced == 0`) holds an invented value (a modality builder's grade, an experience's emotional
/// significance …) and is reset to 0.0 with a hints-only update. Idempotent; a marker file records completion so later cycles skip the walk.
/// Returns how many containers were reset.
pub async fn scrub_legacy_hotness_once(store: &Arc<dyn StoreAccess>, stats: &mut PassStats) -> usize {
    let marker = crate::keyword_program::data_dir().join("keyword_hotness_scrub.done");
    if marker.exists() {
        return 0;
    }
    let Ok(ids) = store.list_ids(None).await else { return 0 };
    let mut n = 0usize;
    let mut failed = 0usize;
    for id in ids {
        let Ok(Some(v)) = store.get_container(id).await else { continue };
        let Ok(c) = serde_json::from_value::<Container>(v) else { continue };
        let h = &c.local_state.hints;
        let observed = c.local_state.context.keyword_meta.get(crate::keyword_program::CONTAINER_TOTALS_KEY).map(|m| m.hits.returned_traced).unwrap_or(0);
        if h.hotness_score == 0.0 || observed > 0 {
            continue;
        }
        let mut hints = h.clone();
        hints.hotness_score = 0.0;
        let upd = ContainerUpdate { metadata: None, context: None, storage: None, hints: Some(hints) };
        match serde_json::to_value(&upd) {
            Ok(uv) => {
                if store.update_container(id, uv).await.is_ok() {
                    n += 1;
                } else {
                    failed += 1;
                }
            }
            Err(_) => failed += 1,
        }
    }
    if failed == 0 {
        crate::durable_write::replace(&marker, format!("{n} containers reset").into_bytes()); // durable: retried until written
    }
    if n > 0 {
        stats.candidates_consumed += n as u64;
        stats.notes.push(format!("legacy hotness scrub: {n} container(s) reset to 0.0"));
        tracing::info!(reset = n, failed, "keyword program: legacy invented hotness scrubbed (stored hotness is now observed use only)");
    }
    n
}

/// One settlement pass over every unsettled container (most-searched types first). No count cap; a container whose model call fails is
/// retried on the next pass.
pub async fn settle_once(
    executor: &Arc<dyn PipelineExecutor>,
    store: &Arc<dyn StoreAccess>,
    available_models: &[AvailableModel],
    meta_fallback: &ModelFallbackConfig,
    stats: &mut PassStats,
) {
    let mut settled = 0usize;
    // (the typed-scope backfill runs in the housekeeping actor, keyword_loops.rs)
    for &ctype in SETTLE_ORDER {
        let Ok(ids) = store.list_ids(Some(ctype)).await else { continue };
        for id in ids {
            let unsettled = match store.get_container(id).await {
                Ok(Some(v)) => serde_json::from_value::<Container>(v).map(|c| !is_settled(&c) || needs_rejudge(&c)).unwrap_or(false),
                _ => false,
            };
            if !unsettled {
                continue;
            }
            stats.candidates_consumed += 1;
            match settle_container(executor, store, available_models, meta_fallback, id, ctype, stats).await {
                Ok(true) => {
                    settled += 1;
                    stats.successes += 1;
                    stats.candidate_container_ids.push(id);
                }
                Ok(false) => {}
                Err(e) => {
                    stats.failures += 1;
                    tracing::warn!(container = id, error = %e, "keyword settlement: container not settled this pass");
                }
            }
        }
    }
    if settled > 0 {
        tracing::info!(settled, "keyword settlement: containers settled (verbatim capture + expansion + unanimous-drop judgment)");
        stats.notes.push(format!("keyword settlement: {settled} container(s) settled"));
    }
}

/// Pass D trigger: a keeper whose real evidence turned negative (dropped > kept + applied) and was never affirmed is re-judged.
fn needs_rejudge(c: &Container) -> bool {
    let ctx = &c.local_state.context;
    km::meaning_terms(&ctx.keywords).iter().any(|k| {
        ctx.keyword_meta.get(k).map(|m| m.hits.dropped > m.hits.kept + m.hits.applied && m.validated != Some(true)).unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::container::{Context, LocalState};

    fn container_with(keywords: &[&str], meta: &[(&str, &str)]) -> Container {
        let mut c = Container {
            global_state: Default::default(),
            local_state: LocalState::default(),
        };
        c.local_state.context = Context { keywords: keywords.iter().map(|s| s.to_string()).collect(), ..Default::default() };
        for (k, src) in meta {
            c.local_state.context.keyword_meta.insert(k.to_string(), TermMeta { source: src.to_string(), validated: None, hits: Default::default() });
        }
        c
    }

    #[test]
    fn settled_means_every_meaning_keyword_is_validated_and_markers_are_ignored() {
        assert!(!is_settled(&container_with(&["static analysis"], &[])));
        assert!(!is_settled(&container_with(&["static analysis", "scope:global"], &[("static analysis", "extracted")])), "provenance alone is NOT enough — it must pass the gate");
        let mut ok = container_with(&["static analysis", "scope:global"], &[("static analysis", "extracted")]);
        ok.local_state.context.keyword_meta.get_mut("static analysis").unwrap().validated = Some(true);
        assert!(is_settled(&ok));
        assert!(is_settled(&container_with(&["scope:global"], &[]))); // markers only: nothing to settle
        assert!(is_settled(&container_with(&["src/main.rs"], &[("src/main.rs", "identifier")])), "a deterministic graph fact is validated by construction");
        assert!(!is_settled(&container_with(&["a", "b"], &[("a", "verbatim")])));
        let mut rejected = container_with(&["x"], &[("x", "expansion")]);
        rejected.local_state.context.keyword_meta.get_mut("x").unwrap().validated = Some(false);
        assert!(!is_settled(&rejected), "a rejected term still sitting in keywords is not settled (the gate removes it)");
    }

    #[test]
    fn push_unique_rejects_markers_sentences_and_duplicates() {
        let mut l: Vec<String> = vec!["Static Analysis".to_string()];
        assert!(!push_unique(&mut l, "static analysis"));
        assert!(!push_unique(&mut l, "scope:global"));
        assert!(!push_unique(&mut l, "how the whole sentence goes on and on"));
        assert!(push_unique(&mut l, "linting"));
        assert_eq!(l.len(), 2);
    }

    #[test]
    fn project_rollup_refreshes_when_the_child_set_changes() {
        let mut c = container_with(&[], &[]);
        c.local_state.metadata.container_type = ContainerType::Project;
        c.global_state.child_ids = vec![1, 2, 3];
        assert!(!is_settled(&c), "no signature recorded yet");
        c.local_state.context.keyword_meta.insert(ROLLUP_KEY.to_string(), TermMeta { source: rollup_signature(&c), validated: None, hits: Default::default() });
        assert!(is_settled(&c));
        c.global_state.child_ids.push(4);
        assert!(!is_settled(&c), "a new child makes the roll-up stale");
        assert_ne!(rollup_signature(&c), {
            let mut d = c.clone();
            d.global_state.child_ids = vec![1, 2, 3, 5];
            rollup_signature(&d)
        });
    }

    #[test]
    fn graph_identifiers_are_generic_and_skip_bookkeeping() {
        let g = serde_json::json!({"nodes": [
            {"node_id": 1, "node_type": "Function", "name": "validate_token", "content": "a long body of text that is not an identifier at all", "keywords": ["auth", "token"], "position": 3},
            {"node_id": 2, "node_type": "Protein", "label": "LacI", "accession": "P03023", "material_name": "steel", "hotness_score": 0.5, "properties": {"x": "y"}},
            {"node_id": 3, "name": "validate_token", "keywords": ["auth"], "provenance": "text-analysis"},
        ]});
        let ids = graph_identifiers(&g);
        assert_eq!(ids[0..2], ["auth".to_string(), "validate_token".to_string()], "most frequent first: {ids:?}");
        for expected in ["token", "LacI", "P03023", "steel"] {
            assert!(ids.iter().any(|t| t == expected), "{expected} missing from {ids:?}");
        }
        assert!(!ids.iter().any(|t| t == "Function" || t == "Protein" || t == "text-analysis" || t.contains("long body")), "bookkeeping/content leaked: {ids:?}");
        assert_eq!(file_extension("a.go").as_deref(), Some("go"));
        assert_eq!(file_extension("Makefile"), None);
        assert_eq!(file_extension("weird."), None);
    }

    #[test]
    fn rejudge_triggers_only_on_net_negative_real_evidence() {
        let mut c = container_with(&["noise"], &[("noise", "extracted")]);
        assert!(!needs_rejudge(&c));
        c.local_state.context.keyword_meta.get_mut("noise").unwrap().hits.dropped = 3;
        assert!(needs_rejudge(&c));
        c.local_state.context.keyword_meta.get_mut("noise").unwrap().validated = Some(true);
        assert!(!needs_rejudge(&c));
    }

    #[test]
    fn parse_round_reads_numbers_and_treats_contradiction_or_absence_as_no_vote() {
        let v = serde_json::json!({"yes": [1, "3", 9], "no": [2, 3]});
        let (yes, no) = parse_round(&v, 4);
        assert_eq!(yes.into_iter().collect::<Vec<_>>(), vec![1], "3 is in both lists (contradictory), 9 is out of range");
        assert_eq!(no.into_iter().collect::<Vec<_>>(), vec![2]);
        let (y2, n2) = parse_round(&serde_json::json!({"verdict": "maybe"}), 3);
        assert!(y2.is_empty() && n2.is_empty(), "a garbled answer is never a verdict");
    }

    #[test]
    fn chunk_terms_never_drops_a_term_and_respects_both_limits() {
        let terms: Vec<String> = (0..50).map(|i| format!("keyword number {i}")).collect();
        let by_items = chunk_terms(&terms, 1_000_000, 7);
        assert!(by_items.iter().all(|c| c.len() <= 7) && by_items.iter().map(|c| c.len()).sum::<usize>() == 50);
        let by_tokens = chunk_terms(&terms, 40, 1_000);
        assert!(by_tokens.len() > 1 && by_tokens.iter().map(|c| c.len()).sum::<usize>() == 50, "a small window splits, it never truncates");
        let flat: Vec<String> = by_tokens.into_iter().flatten().collect();
        assert_eq!(flat, terms, "order preserved, nothing lost");
        assert_eq!(chunk_terms(&["x".to_string()], 0, 0).len(), 1, "a single term always fits somewhere");
    }

    #[test]
    fn a_stored_term_is_removed_only_if_every_round_says_no_and_a_new_one_is_stored_only_if_every_round_says_yes() {
        use std::collections::BTreeSet;
        let chunk: Vec<String> = ["ufw", "fail2ban", "noise"].iter().map(|s| s.to_string()).collect();
        let set = |v: &[usize]| v.iter().copied().collect::<BTreeSet<usize>>();
        // EXISTING: round 1 says NO to ufw and noise, YES to fail2ban
        let r1 = step_round(true, &chunk, &set(&[2]), &set(&[1, 3]));
        assert_eq!(r1.validated, vec!["fail2ban"], "one YES keeps and validates a stored term");
        assert_eq!(r1.next, vec!["ufw", "noise"], "a NO alone only sends it to the next round — it is NOT removed");
        assert!(r1.rejected.is_empty() && r1.unresolved.is_empty());
        // NEW candidate: the same answers
        let n1 = step_round(false, &chunk, &set(&[2]), &set(&[1, 3]));
        assert_eq!(n1.rejected, vec!["ufw", "noise"], "a NO refuses a new candidate at once");
        assert_eq!(n1.next, vec!["fail2ban"], "a YES alone only sends it to the next round");
        // an unanswered term is neither kept nor dropped
        let u = step_round(true, &chunk, &set(&[]), &set(&[1]));
        assert_eq!(u.unresolved, vec!["fail2ban", "noise"]);
    }
}
