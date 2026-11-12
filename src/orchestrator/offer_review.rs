//! OFFER REVIEW — the zero-shot half of the marketplace gate (operator 2026-11-10: peers notify, this instance reviews and identifies whether to
//! download). ADVISORY ONLY: it records a RECOMMENDATION (`download` / `ignore` / `review`) with a reason in `peer_offer_reviews.json`; the operator's
//! decision (`POST /marketplace/offers/decide`) is what acts. Nothing is ever downloaded because a model said so.
//!
//! Review-before-judge (ZERO_SHOT_EXPANSION_GUIDE §1): the mechanical checks come first and cost no model call —
//!   * the network door already refused every type / shape that is not allowed;
//!   * an offer whose content this store already has (same name + same hash) is recommended `ignore` deterministically.
//! Only the remaining question needs judgment: is this offer useful here? It is asked over strength-N rounds with the SAME unanimity rule as every
//! other gate: `download` only if EVERY round says "useful", `ignore` only if EVERY round says "not useful", anything else is `review` (a human
//! looks). The model judges from the manifest alone (name, keywords, what this instance already has that is similar) — it has not seen the content,
//! and says so in the prompt. Kill switch: `OZONE_OFFER_REVIEW=off`.

use crate::orchestrator::peer_import::{collect_offers, RawOffer};
use crate::orchestrator::{PipelineExecutor, PromptOrchestrator, StoreAccess};
use crate::shared_contracts::k_validation;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const PROMPT_PIPELINE_ID: u64 = 9;

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn read_map(path: &Path) -> Map<String, Value> {
    // (read through durable_write: a snapshot whose write is still being retried is the base)
    crate::durable_write::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Lower-cased alphanumeric tokens of length >= 2 (language-neutral shape rule, no word list).
pub fn token_set(s: &str) -> BTreeSet<String> {
    s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|t| t.chars().count() >= 2).map(String::from).collect()
}

/// The local items most similar to an offer, by token overlap (Jaccard) of name + keywords — what the reviewer is told this instance "already
/// has". Items with no overlap are left out; at most `max` are returned, best first. Deterministic.
pub fn similar_local(offer_name: &str, offer_keywords: &[String], local: &[(String, Vec<String>)], max: usize) -> Vec<String> {
    let offer: BTreeSet<String> = token_set(&format!("{offer_name} {}", offer_keywords.join(" ")));
    let mut scored: Vec<(f32, &String)> = local
        .iter()
        .filter_map(|(name, kws)| {
            let mine: BTreeSet<String> = token_set(&format!("{name} {}", kws.join(" ")));
            let inter = offer.intersection(&mine).count();
            (inter > 0).then(|| (inter as f32 / offer.union(&mine).count().max(1) as f32, name))
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(b.1)));
    scored.into_iter().take(max).map(|(_, n)| n.clone()).collect()
}

/// The recommendation from the rounds' verdicts: unanimity for either extreme, anything else (or nothing) = `review`.
pub fn recommendation(verdicts: &[String]) -> &'static str {
    if verdicts.is_empty() {
        return "review";
    }
    if verdicts.iter().all(|v| v == "useful") {
        "download"
    } else if verdicts.iter().all(|v| v == "not_useful") {
        "ignore"
    } else {
        "review"
    }
}

/// (name, keywords, body hash) of every local methodology — read once per pass, only when an offer needs the comparison.
async fn local_methodologies(store: &Arc<dyn StoreAccess>) -> Vec<(String, Vec<String>, String)> {
    let mut out = Vec::new();
    for id in store.list_ids(Some("Methodology")).await.unwrap_or_default() {
        let Ok(Some(c)) = store.get_container(id).await else { continue };
        let Some(name) = c.pointer("/local_state/metadata/name").and_then(|v| v.as_str()).map(String::from) else { continue };
        let kws: Vec<String> = c.pointer("/local_state/context/keywords").and_then(|k| serde_json::from_value(k.clone()).ok()).unwrap_or_default();
        let hash = c
            .pointer("/local_state/storage/object_store_path")
            .and_then(|v| v.as_str())
            .and_then(crate::orchestrator::tool_binding::read_object_file)
            .map(|b| crate::network::body_hash(b.as_bytes()))
            .unwrap_or_default();
        out.push((name, kws, hash));
    }
    out
}

/// Names of the registered tools (`mcp_tool_registry.json`, an array of `{name, …}`); empty when the file is absent.
fn tool_names(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("mcp_tool_registry.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_array().map(|a| a.iter().filter_map(|e| e["name"].as_str().map(String::from)).collect()))
        .unwrap_or_default()
}

/// Ask the model for ONE verdict about an offer. None = no usable answer (the offer is retried on the next pass).
async fn one_round(executor: &Arc<dyn PipelineExecutor>, o: &RawOffer, similar: &[String], conflict: bool, wanted: Option<&str>) -> Option<(String, String)> {
    let prompt = format!(
        "An instance of Ozone-Studio (a graph-based, model-agnostic system) was OFFERED an item by another instance.\n\nOFFER: type {}, name \"{}\", keywords: {}\n{}{}\nWHAT THIS INSTANCE ALREADY HAS THAT LOOKS SIMILAR:\n{}\n\nQUESTION: Is this offer useful to add to this instance? Answer \"useful\" only if it clearly adds something this instance does not already have; \"not_useful\" if it duplicates what is already here or its subject has nothing to do with this instance; otherwise \"unsure\". You are judging from the manifest alone (name and keywords) — you have NOT seen the content.\nReturn ONLY JSON: {{\"verdict\": \"useful\" | \"not_useful\" | \"unsure\", \"reason\": \"one short sentence\"}}",
        o.item_type.replace('_', " "),
        o.name,
        if o.keywords.is_empty() { "(none)".to_string() } else { o.keywords.join(", ") },
        if conflict { "NOTE: this instance already has an item with this name but DIFFERENT content.\n" } else { "" },
        wanted.map(|g| format!("NOTE: this instance has a WANTED capability gap named \"{g}\" (something it needs and cannot do today) that this offer may satisfy — that is a reason it could be useful.\n")).unwrap_or_default(),
        if similar.is_empty() { "  (nothing similar)".to_string() } else { similar.iter().map(|n| format!("  - {n}")).collect::<Vec<_>>().join("\n") },
    );
    let input = json!({"prompt": prompt, "max_tokens": 200, "temperature": 0.1, "system_prompt": "You review marketplace offers for an instance. Return only valid JSON."});
    let res = executor.execute(PROMPT_PIPELINE_ID, input).await.ok()?;
    let text = res.get("response").and_then(|x| x.as_str()).unwrap_or("");
    if text.trim().is_empty() {
        return None;
    }
    let v: Value = serde_json::from_str(PromptOrchestrator::extract_json_object_shared(text).trim()).ok()?;
    let verdict = v["verdict"].as_str()?.to_string();
    if !matches!(verdict.as_str(), "useful" | "not_useful" | "unsure") {
        return None;
    }
    Some((verdict, v["reason"].as_str().unwrap_or("").chars().filter(|c| !c.is_control()).take(300).collect()))
}

async fn review_pass(executor: &Arc<dyn PipelineExecutor>, store: &Arc<dyn StoreAccess>, dir: &Path) {
    let reviews_path = dir.join("peer_offer_reviews.json");
    let mut reviews = read_map(&reviews_path);
    let decisions = read_map(&dir.join("peer_offer_decisions.json"));
    // every offer that has neither a recommendation yet nor an operator decision
    let todo: Vec<RawOffer> = collect_offers(dir).into_iter().filter(|o| !reviews.contains_key(&o.key) && !decisions.contains_key(&o.key)).collect();
    if todo.is_empty() {
        return;
    }
    let local = local_methodologies(store).await;
    let strength = k_validation::configured_strength().max(1);
    // what this instance has per kind (tools, installed pipelines, jurisdiction scope files) and the capabilities it still WANTS
    let idx = crate::orchestrator::peer_import::LocalOfferIndex::load(dir);
    let wanted = crate::orchestrator::peer_import::wanted_gap_names();
    for o in todo {
        // 1. mechanical first: identical content already here = nothing to download
        let same_name: Vec<&(String, Vec<String>, String)> = local.iter().filter(|(n, _, _)| n.eq_ignore_ascii_case(&o.name)).collect();
        if o.item_type == "methodology" && !o.hash.is_empty() && same_name.iter().any(|(_, _, h)| *h == o.hash) {
            reviews.insert(o.key.clone(), json!({"recommendation": "ignore", "reason": "this instance already has identical content", "by": "deterministic", "at": now_secs()}));
            continue;
        }
        // 2. the zero-shot question, strength-N rounds; the first round that disagrees with the first answer ends it (no unanimity possible)
        let conflict = o.item_type == "methodology" && !same_name.is_empty();
        // what this instance already has OF THE SAME KIND: methodologies for a methodology offer, registered tool names for a tool offer;
        // pipelines compare with the installed pipeline registry and jurisdiction sets with the local scope files; other kinds have no local index (the prompt says so rather than listing unrelated items). No cap on the list —
        // names only, so it is small, and a cut would hide exactly the duplicate the reviewer must see.
        let pool: Vec<(String, Vec<String>)> = match o.item_type.as_str() {
            "methodology" => local.iter().map(|(n, k, _)| (n.clone(), k.clone())).collect(),
            "tool" => tool_names(dir).into_iter().map(|n| (n, Vec::new())).collect(),
            "pipeline" => idx.pipeline_names.iter().map(|n| (n.clone(), Vec::new())).collect(),
            "jurisdiction_rule_set" => idx.jurisdiction.keys().map(|n| (n.clone(), Vec::new())).collect(),
            _ => Vec::new(),
        };
        let similar = similar_local(&o.name, &o.keywords, &pool, usize::MAX);
        // ADVISORY: a tool / pipeline offer that matches a wanted capability gap (deterministic by name / keyword); it only informs the vote
        let matches_wanted = crate::orchestrator::peer_import::matches_wanted(&o, &wanted);
        let mut verdicts: Vec<String> = Vec::new();
        let mut reason = String::new();
        let mut failed = false;
        for _ in 0..strength {
            match one_round(executor, &o, &similar, conflict, matches_wanted.as_deref()).await {
                Some((v, r)) => {
                    if reason.is_empty() {
                        reason = r;
                    }
                    let differs = verdicts.first().map(|f| *f != v).unwrap_or(false);
                    verdicts.push(v);
                    if differs {
                        break;
                    }
                }
                None => {
                    failed = true;
                    break;
                }
            }
        }
        if failed && verdicts.is_empty() {
            continue; // no usable answer — retried next pass
        }
        let rec = recommendation(&verdicts);
        crate::keyword_program::record_judgment("offer_review", None, &o.name, &o.from, "is this marketplace offer useful to add to this instance?", rec == "download", strength);
        reviews.insert(o.key.clone(), json!({"recommendation": rec, "reason": reason, "rounds": verdicts, "strength": strength, "matches_wanted": matches_wanted, "by": "zero-shot", "at": now_secs()}));
    }
    // Durable replace (retried until written).
    crate::durable_write::replace(&reviews_path, serde_json::to_string_pretty(&reviews).unwrap_or_default().into_bytes());
}

/// The review loop (spawned only when `marketplace_accept_offers` is on — or the deprecated alias `auto_broadcast_sync`). Polls every 2 minutes; a model failure leaves an
/// offer unreviewed (retried). Advisory: it never decides.
pub fn spawn_offer_review_loop(executor: Arc<dyn PipelineExecutor>, store: Arc<dyn StoreAccess>, data_dir: String) {
    tokio::spawn(async move {
        let dir = PathBuf::from(&data_dir);
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(120));
        tick.tick().await;
        loop {
            tick.tick().await;
            if matches!(std::env::var("OZONE_OFFER_REVIEW").ok().as_deref(), Some("off") | Some("0") | Some("false")) {
                continue;
            }
            review_pass(&executor, &store, &dir).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendation_needs_unanimity_for_either_extreme() {
        let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(recommendation(&v(&["useful", "useful", "useful"])), "download");
        assert_eq!(recommendation(&v(&["not_useful", "not_useful"])), "ignore");
        assert_eq!(recommendation(&v(&["useful", "unsure"])), "review");
        assert_eq!(recommendation(&v(&["useful", "not_useful"])), "review");
        assert_eq!(recommendation(&[]), "review", "no answer is never a recommendation to act");
    }

    #[test]
    fn similar_local_ranks_by_overlap_and_leaves_out_the_unrelated() {
        let local = vec![
            ("API Design".to_string(), vec!["api".to_string(), "rest".to_string()]),
            ("Gardening Basics".to_string(), vec!["plants".to_string()]),
            ("API Design Review".to_string(), vec!["api".to_string(), "design".to_string(), "review".to_string()]),
        ];
        let got = similar_local("API design guide", &["api".to_string(), "design".to_string(), "review".to_string()], &local, 8);
        assert_eq!(got.first().map(String::as_str), Some("API Design Review"), "the larger overlap ranks first");
        assert!(got.contains(&"API Design".to_string()) && !got.contains(&"Gardening Basics".to_string()));
        assert_eq!(similar_local("API", &[], &local, 1).len(), 1, "the cap is honoured");
    }

    #[test]
    fn tokens_are_language_neutral_shapes_not_a_word_list() {
        let t = token_set("Réseau-API (v2) a");
        assert!(t.contains("réseau") && t.contains("api") && t.contains("v2"));
        assert!(!t.contains("a"), "single characters are not tokens");
    }
}
