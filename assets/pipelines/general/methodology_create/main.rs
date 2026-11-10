//! MethodologyCreatePipeline - Pipeline #12
//! Create and update methodologies in ZSEI.
//! Methodologies are reusable approaches that can be shared via consensus.

use serde::{Deserialize, Serialize};
use std::env;

#[path = "../../shared/ozone_serve.rs"]
mod ozone_serve;

// Keyword SHAPE rules (shared — single source): markers, whole sentences, empties and duplicates are not keywords. No word list.
#[path = "../../../../shared/contracts/keyword_model.rs"]
#[allow(dead_code)]
mod keyword_model;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum MethodologyCreateInput {
    Create {
        name: String,
        description: String,
        category_id: u64,
        principles: Vec<PrincipleInput>,
        heuristics: Vec<HeuristicInput>,
        decision_rules: Vec<DecisionRuleInput>,
        keywords: Vec<String>,
        topics: Vec<String>,
        /// Task this methodology was distilled from, if any — carried into
        /// the persisted container's metadata so meta-work traces back to
        /// the real task that produced it.
        #[serde(default)]
        task_id: Option<u64>,
    },
    Update {
        methodology_id: u64,
        name: Option<String>,
        description: Option<String>,
        add_principles: Option<Vec<PrincipleInput>>,
        remove_principles: Option<Vec<String>>,
        add_heuristics: Option<Vec<HeuristicInput>>,
        /// New decision rules (with their per-rule providers) appended to the existing ones.
        #[serde(default)]
        add_decision_rules: Option<Vec<DecisionRuleInput>>,
        /// RETIRE decision rules (2026-11-10): each entry is the rule's `condition` text, matched EXACTLY (trimmed) against the CURRENT body —
        /// never by index (positions shift). Refuses (error, nothing written) when an entry matches nothing or when the methodology would be
        /// left with no real decision rule and no real heuristic. See [`retire_rules`].
        #[serde(default)]
        remove_decision_rules: Option<Vec<String>>,
        /// RETIRE heuristics: each entry is the heuristic's `when_to_apply` text (the condition `add_heuristics` stored), matched exactly.
        #[serde(default)]
        remove_heuristics: Option<Vec<String>>,
        add_keywords: Option<Vec<String>>,
        remove_keywords: Option<Vec<String>>,
    },
    Delete { methodology_id: u64 },
    Propose { methodology_id: u64 }, // Propose for network consensus
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipleInput { pub name: String, pub description: String, pub priority: u8 }

/// `confidence` is accepted but never consumed (confirmed: no read site
/// anywhere in this file — the canonical mapping below doesn't carry it
/// into the persisted Heuristic shape either). Real bug found live
/// 2026-09-20: a required, unused `f32` field meant a genuine, substantive
/// meta-loop draft (BitNet, non-placeholder, passed every content gate)
/// still hard-failed deserialization with "missing field `confidence`"
/// whenever the model's real JSON omitted it — discarding real work for a
/// field nothing downstream reads. `#[serde(default)]` on an `Option`
/// removes the gate without fabricating a confidence value that was never
/// reported (same honest-absence idiom as `GateResult.confidence`
/// elsewhere in this codebase).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicInput {
    pub condition: String,
    pub action: String,
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRuleInput {
    pub name: String,
    pub condition: String,
    pub outcome: String,
    /// PROVIDERS this rule needs (2026-11-04, operator doctrine: methodologies list providers per rule; blueprint creation turns
    /// each into a provider STEP). Stored as `requires` on the canonical DecisionRule (src/types/methodology.rs `ToolRequirement`).
    #[serde(default)]
    pub requires: Vec<ToolRequirementInput>,
}

/// One provider/capability a rule needs. `kind`: "provider" (MCP server id, e.g. `threed-mcp`) | "mcp"/"tool" (a tool name —
/// resolved to its provider) | "capability" (free text) | "pipeline".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequirementInput {
    #[serde(default = "default_requirement_kind")]
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub optional: bool,
}

fn default_requirement_kind() -> String {
    "provider".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyCreateOutput {
    pub success: bool,
    pub methodology_id: Option<u64>,
    pub version: Option<u32>,
    pub proposal_id: Option<u64>,
    pub error: Option<String>,
    /// Update with removals only: exactly what was retired (`{"decision_rules": [...], "heuristics": [...]}`, the whole removed entries) and the
    /// path of the body that was replaced (`previous_body_path` — pointing the container's object_store_path back at it reverses the update).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_body_path: Option<String>,
}

const METHODOLOGY_ROOT_ID: u64 = 2;

fn ozone_host() -> String {
    env::var("OZONE_HOST").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

async fn zsei_query(query: serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/zsei/query", ozone_host())).header("x-ozone-trace", std::env::var("OZONE_TRACE_ID").unwrap_or_default()).header("x-ozone-origin", std::env::var("OZONE_ORIGIN").unwrap_or_default())
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

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Write the full methodology content (principles/heuristics/decision_rules —
/// Container has no generic slot for this nested structure, same constraint
/// pipeline 100's modality graphs and the orchestrator's AMT tree hit) to
/// disk, referenced via storage.object_store_path. Real keywords/topics/name
/// go into the container's own metadata/context, not fabricated.
fn write_content_file(id_hint: &str, content: &serde_json::Value) -> Option<String> {
    let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
    let dir = format!("{}/methodologies_created", data_dir);
    let _ = std::fs::create_dir_all(&dir);
    let file_name = format!("methodology_{}_{}.json", now_secs(), id_hint);
    let path = format!("{}/{}", dir, file_name);
    let json = serde_json::to_string_pretty(content).ok()?;
    std::fs::write(&path, json).ok()?;
    Some(format!("methodologies_created/{}", file_name))
}

fn build_container_json(
    parent_id: u64,
    name: &str,
    keywords: &[String],
    topics: &[String],
    object_store_path: Option<String>,
    task_id: Option<u64>,
) -> serde_json::Value {
    let now = now_secs();
    serde_json::json!({
        "global_state": {
            "container_id": 0,
            "child_count": 0,
            "version": 1,
            "parent_id": parent_id,
            "child_ids": []
        },
        "local_state": {
            "metadata": {
                "container_type": "Methodology",
                "modality": "Unknown",
                "created_at": now,
                "updated_at": now,
                "provenance": match task_id {
                    Some(t) => format!("methodology_create:task:{}", t),
                    None => "methodology_create".to_string(),
                },
                "permissions": 0,
                "owner_id": 0,
                "name": name,
                "materialized_path": null
            },
            "context": {
                "categories": [],
                "methodologies": [],
                "keywords": keywords,
                "topics": topics,
                "relationships": [],
                "learned_associations": [],
                "embedding": null
            },
            "storage": {
                "db_shard_id": null,
                "vector_index_ref": null,
                "object_store_path": object_store_path,
                "compression_type": "None"
            },
            "hints": {
                "access_frequency": 0,
                "hotness_score": 0.0,
                "last_accessed": 0,
                "centroid": null,
                "ml_prediction_weight": 0.0
            },
            "integrity": {
                "content_hash": vec![0u8; 32],
                "semantic_fingerprint": [],
                "last_verified": now,
                "integrity_score": 1.0,
                "version_history": []
            },
            "file_context": null,
            "code_context": null,
            "text_context": null,
            "external_ref": null
        }
    })
}

pub async fn execute(input: MethodologyCreateInput) -> Result<MethodologyCreateOutput, String> {
    match input {
        MethodologyCreateInput::Create {
            name,
            description,
            category_id,
            principles,
            heuristics,
            decision_rules,
            keywords,
            topics,
            task_id,
        } => {
            // Stored in the canonical shape src/types/methodology.rs's
            // Principle/Heuristic/DecisionRule use (and that
            // PromptOrchestrator::load_methodology_rules_text, added this
            // session, actually reads back) rather than a raw dump of this
            // action's own PrincipleInput/HeuristicInput/DecisionRuleInput
            // DTOs. Those DTOs use different field names for the same
            // concepts (HeuristicInput{condition,action,confidence} vs
            // Heuristic{when_to_apply,description,...};
            // DecisionRuleInput{name,condition,outcome} vs
            // DecisionRule{condition,action,...}) — dumping them directly
            // was a real, silent bug: every heuristic and decision_rule
            // created through this action would have been invisible to the
            // reader (field names never matched, so every value read back
            // as empty and got filtered out). No information is lost or
            // invented here, only relabeled into the one shape the rest of
            // the system actually reads.
            let canonical_principles: Vec<serde_json::Value> = principles
                .iter()
                .map(|p| serde_json::json!({
                    "statement": p.name,
                    "rationale": p.description,
                    "applicability": Vec::<String>::new(),
                }))
                .collect();
            let canonical_heuristics: Vec<serde_json::Value> = heuristics
                .iter()
                .map(|h| serde_json::json!({
                    "name": h.condition,
                    "when_to_apply": h.condition,
                    "description": h.action,
                    "examples": Vec::<String>::new(),
                }))
                .collect();
            let canonical_decision_rules: Vec<serde_json::Value> = decision_rules
                .iter()
                .map(|d| serde_json::json!({
                    "condition": d.condition,
                    "action": d.outcome,
                    "priority": 5,
                    "exceptions": Vec::<String>::new(),
                    // per-rule providers (empty = none declared)
                    "requires": d.requires.iter().filter(|r| !r.name.trim().is_empty()).map(|r| serde_json::json!({
                        "kind": r.kind, "name": r.name.trim(), "purpose": r.purpose, "optional": r.optional,
                    })).collect::<Vec<_>>(),
                }))
                .collect();
            let content = serde_json::json!({
                "name": name,
                "description": description,
                "category_id": category_id,
                "principles": canonical_principles,
                "heuristics": canonical_heuristics,
                "decision_rules": canonical_decision_rules,
            });
            let object_store_path = write_content_file("new", &content);
            // CREATION-SIDE KEYWORD HYGIENE (2026-11-08): the keywords a methodology is BORN with are shape-cleaned here (markers, whole sentences,
            // empties, case-insensitive duplicates dropped) so the settlement pass is a cleanup, not the first line of defence. Filler words that are
            // well-formed single terms are judged later by the unanimous strength-N keeper vote (no word list anywhere).
            let keywords = keyword_model::meaning_terms(&keywords);
            let container = build_container_json(
                METHODOLOGY_ROOT_ID,
                &name,
                &keywords,
                &topics,
                object_store_path,
                task_id,
            );
            let result = zsei_query(serde_json::json!({
                "CreateContainer": {"parent_id": METHODOLOGY_ROOT_ID, "container": container}
            }))
            .await?;
            let id = result
                .get("ContainerID")
                .and_then(|v| v.as_u64())
                .ok_or("CreateContainer returned no container id")?;
            Ok(MethodologyCreateOutput {
                success: true,
                methodology_id: Some(id),
                version: Some(1),
                proposal_id: None,
                error: None,
                removed: None,
                previous_body_path: None,
            })
        }
        MethodologyCreateInput::Update {
            methodology_id,
            name,
            description,
            add_principles,
            remove_principles,
            add_heuristics,
            add_decision_rules,
            remove_decision_rules,
            remove_heuristics,
            add_keywords,
            remove_keywords,
        } => {
            // Real merge: fetch the existing container, apply the requested
            // deltas, write back — not a fabricated version bump.
            let existing = zsei_query(serde_json::json!({
                "GetContainer": {"container_id": methodology_id}
            }))
            .await?;
            let container = existing
                .get("Container")
                .ok_or("Methodology container not found")?;
            let mut keywords: Vec<String> = container
                .pointer("/local_state/context/keywords")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            if let Some(add) = add_keywords {
                keywords.extend(add);
            }
            if let Some(remove) = remove_keywords {
                keywords.retain(|k| !remove.contains(k));
            }
            keywords.sort();
            keywords.dedup();

            let existing_name = container
                .pointer("/local_state/metadata/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let existing_topics: Vec<String> = container
                .pointer("/local_state/context/topics")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let existing_path: Option<String> = container
                .pointer("/local_state/storage/object_store_path")
                .and_then(|v| v.as_str())
                .map(String::from);

            // REAL READ-MERGE-WRITE (2026-11-05): the previous body wrote a deltas-only file that the container never pointed at, so
            // content additions were silently lost. Now: read the methodology's CURRENT content file, merge the additions/removals into
            // it (nothing existing is dropped), write it as a NEW versioned file (the old file is kept — history), and point the
            // container's storage at the new file (the host's UpdateContainer snapshots the prior version).
            let data_dir = env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string());
            let mut content: serde_json::Value = existing_path
                .as_ref()
                .and_then(|rel| {
                    // data dir first, then the path as given (bootstrap-seeded methodologies keep their body under the APP ROOT — read only from the
                    // data dir this fell back to an EMPTY body and the update then wrote a file holding ONLY the additions, dropping every existing rule)
                    std::fs::read_to_string(format!("{}/{}", data_dir, rel)).ok().or_else(|| std::fs::read_to_string(rel).ok())
                })
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_else(|| serde_json::json!({"principles": [], "heuristics": [], "decision_rules": []}));
            if !content.is_object() {
                content = serde_json::json!({"principles": [], "heuristics": [], "decision_rules": []});
            }
            let mut content_changed = false;
            if let Some(n) = &name { content["name"] = serde_json::json!(n); content_changed = true; }
            if let Some(d) = &description { content["description"] = serde_json::json!(d); content_changed = true; }
            if let Some(remove) = &remove_principles {
                if let Some(arr) = content.get_mut("principles").and_then(|v| v.as_array_mut()) {
                    arr.retain(|p| !p.get("statement").and_then(|x| x.as_str()).map(|st| remove.iter().any(|r| r == st)).unwrap_or(false));
                    content_changed = true;
                }
            }
            // RETIRE rules FIRST (against the CURRENT body), then the additions are applied, and the guard looks at the FINAL shape — all on the
            // in-memory copy: a refusal returns before anything is written (no file, no container update).
            let mut removed_record: Option<serde_json::Value> = None;
            let removing = remove_decision_rules.as_ref().map(|v| !v.is_empty()).unwrap_or(false) || remove_heuristics.as_ref().map(|v| !v.is_empty()).unwrap_or(false);
            if removing {
                let (rec, _) = retire_rules(
                    &mut content,
                    remove_decision_rules.as_deref().unwrap_or(&[]),
                    remove_heuristics.as_deref().unwrap_or(&[]),
                )?;
                removed_record = Some(rec);
                content_changed = true;
            }
            if let Some(add) = &add_principles {
                let arr = content["principles"].as_array().cloned().unwrap_or_default();
                let mut arr = arr;
                for p in add { arr.push(serde_json::json!({"statement": p.name, "rationale": p.description, "applicability": Vec::<String>::new()})); }
                content["principles"] = serde_json::json!(arr);
                content_changed = true;
            }
            if let Some(add) = &add_heuristics {
                let mut arr = content["heuristics"].as_array().cloned().unwrap_or_default();
                for h in add { arr.push(serde_json::json!({"name": h.condition, "when_to_apply": h.condition, "description": h.action, "examples": Vec::<String>::new()})); }
                content["heuristics"] = serde_json::json!(arr);
                content_changed = true;
            }
            if let Some(add) = &add_decision_rules {
                let mut arr = content["decision_rules"].as_array().cloned().unwrap_or_default();
                for d in add {
                    arr.push(serde_json::json!({
                        "condition": d.condition, "action": d.outcome, "priority": 5, "exceptions": Vec::<String>::new(),
                        "requires": d.requires.iter().filter(|r| !r.name.trim().is_empty()).map(|r| serde_json::json!({
                            "kind": r.kind, "name": r.name.trim(), "purpose": r.purpose, "optional": r.optional,
                        })).collect::<Vec<_>>(),
                    }));
                }
                content["decision_rules"] = serde_json::json!(arr);
                content_changed = true;
            }
            if removing && !has_real_rule(&content) {
                return Err(format!(
                    "refusing to retire: methodology {} would be left with no decision rule and no heuristic carrying real text (a methodology never ends empty) — nothing was written",
                    methodology_id
                ));
            }
            let new_path = if content_changed { write_content_file(&methodology_id.to_string(), &content) } else { None };

            let updates = serde_json::json!({
                "metadata": {
                    "container_type": "Methodology",
                    "modality": "Unknown",
                    "created_at": container.pointer("/local_state/metadata/created_at").cloned().unwrap_or(serde_json::json!(now_secs())),
                    "updated_at": now_secs(),
                    "provenance": container.pointer("/local_state/metadata/provenance").cloned().unwrap_or(serde_json::json!("methodology_create")),
                    "permissions": 0,
                    "owner_id": 0,
                    "name": name.clone().unwrap_or(existing_name),
                    "materialized_path": null
                },
                "context": {
                    "categories": [],
                    "methodologies": [],
                    "keywords": keywords,
                    "topics": existing_topics,
                    "relationships": [],
                    "learned_associations": [],
                    "embedding": null
                }
            });
            let mut updates = updates;
            if let Some(np) = &new_path {
                updates["storage"] = serde_json::json!({
                    "db_shard_id": container.pointer("/local_state/storage/db_shard_id").cloned().unwrap_or(serde_json::Value::Null),
                    "vector_index_ref": container.pointer("/local_state/storage/vector_index_ref").cloned().unwrap_or(serde_json::Value::Null),
                    "object_store_path": np,
                    "compression_type": container.pointer("/local_state/storage/compression_type").cloned().unwrap_or(serde_json::json!("None")),
                });
            }
            zsei_query(serde_json::json!({
                "UpdateContainer": {"container_id": methodology_id, "updates": updates}
            }))
            .await?;
            Ok(MethodologyCreateOutput {
                success: true,
                methodology_id: Some(methodology_id),
                version: Some(container.pointer("/global_state/version").and_then(|v| v.as_u64()).unwrap_or(1) as u32 + 1),
                proposal_id: None,
                error: None,
                removed: removed_record,
                previous_body_path: if removing { existing_path.clone() } else { None },
            })
        }
        MethodologyCreateInput::Delete { methodology_id } => {
            zsei_query(serde_json::json!({
                "DeleteContainer": {"container_id": methodology_id}
            }))
            .await?;
            Ok(MethodologyCreateOutput {
                success: true,
                methodology_id: Some(methodology_id),
                version: None,
                proposal_id: None,
                error: None,
                removed: None,
                previous_body_path: None,
            })
        }
        MethodologyCreateInput::Propose { methodology_id: _ } => {
            // No real network-consensus proposal storage exists anywhere in
            // this codebase yet (pipeline #27 "Consensus" is a distinct,
            // unbuilt feature) — returning a fabricated proposal_id here
            // would misrepresent a P2P feature that doesn't exist. Report
            // honestly rather than fake success.
            Err("Methodology proposal/consensus is not implemented yet — publishing to paired peers needs the peer catalog exchange (signed offers, review states): docs/METHODOLOGY_BLUEPRINT_TOOLS_REVIEW_2026-11-04.md §6; the Marketplace tab previews exactly what would be shared".to_string())
        }
    }
}

/// A rule/heuristic counts only when it carries real text (empty and the drafting placeholders "..." / "…" do not) — the same definition as
/// the host's `rule_items` (src/orchestrator/amt.rs), restated here because this crate does not link the host.
fn real_text(t: &str) -> bool {
    let t = t.trim();
    !t.is_empty() && t != "..." && t != "…"
}

/// True when the body still has at least one decision rule (condition + action/outcome) or heuristic (when_to_apply + description) with real text.
fn has_real_rule(content: &serde_json::Value) -> bool {
    let text = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let rules = content.get("decision_rules").and_then(|r| r.as_array()).map(|a| {
        a.iter().any(|r| {
            let action = if r.get("action").is_some() { text(r, "action") } else { text(r, "outcome") };
            real_text(&text(r, "condition")) && real_text(&action)
        })
    });
    let heur = content.get("heuristics").and_then(|h| h.as_array()).map(|a| a.iter().any(|h| real_text(&text(h, "when_to_apply")) && real_text(&text(h, "description"))));
    rules.unwrap_or(false) || heur.unwrap_or(false)
}

/// RETIRE decision rules / heuristics from a methodology body, in memory. Each requested entry is the rule's `condition` (decision rules) or
/// `when_to_apply` (heuristics) text, compared EXACTLY after trimming — never by position (positions shift on every removal and the
/// blueprint citations `m<id>:r<n>` follow positions). Every stored entry whose text equals a requested text is removed (identical text is the
/// same rule); nothing else is touched. Returns `(record of what was removed, count)`.
/// REFUSES with an error — leaving `content` UNCHANGED — when a requested text matches nothing (a typo or a stale request must not silently
/// do nothing), or when the request is empty. The caller applies the "never ends empty" guard on the FINAL body (after additions).
fn retire_rules(content: &mut serde_json::Value, remove_rules: &[String], remove_heuristics: &[String]) -> Result<(serde_json::Value, usize), String> {
    let wanted_rules: Vec<String> = remove_rules.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    let wanted_heur: Vec<String> = remove_heuristics.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    if wanted_rules.is_empty() && wanted_heur.is_empty() {
        return Err("remove_decision_rules / remove_heuristics were given but carry no text".to_string());
    }
    let mut work = content.clone(); // all-or-nothing: only a fully successful retirement replaces `content`
    let mut removed_rules: Vec<serde_json::Value> = Vec::new();
    let mut removed_heur: Vec<serde_json::Value> = Vec::new();
    for (field, key, wanted, out) in [
        ("decision_rules", "condition", &wanted_rules, &mut removed_rules),
        ("heuristics", "when_to_apply", &wanted_heur, &mut removed_heur),
    ] {
        for text in wanted.iter() {
            let arr = work.get_mut(field).and_then(|v| v.as_array_mut());
            let Some(arr) = arr else {
                return Err(format!("the body has no {field}; cannot retire {text:?}"));
            };
            let before = arr.len();
            let mut hit: Vec<serde_json::Value> = Vec::new();
            arr.retain(|e| {
                let t = e.get(key).and_then(|x| x.as_str()).unwrap_or("").trim();
                if t == text.as_str() {
                    hit.push(e.clone());
                    false
                } else {
                    true
                }
            });
            if arr.len() == before {
                return Err(format!("no {field} entry has {key} exactly {text:?} — nothing was written"));
            }
            out.extend(hit);
        }
    }
    let n = removed_rules.len() + removed_heur.len();
    *content = work;
    Ok((serde_json::json!({"decision_rules": removed_rules, "heuristics": removed_heur}), n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> serde_json::Value {
        serde_json::json!({
            "principles": [{"statement": "p"}],
            "heuristics": [{"name": "w1", "when_to_apply": "w1", "description": "d1"}],
            "decision_rules": [
                {"condition": "a", "action": "do a"},
                {"condition": "b", "action": "do b"},
                {"condition": "c", "outcome": "do c"}
            ]
        })
    }

    #[test]
    fn retire_removes_exactly_the_named_rule_by_text_not_position() {
        let mut c = body();
        let (rec, n) = retire_rules(&mut c, &["b".to_string()], &[]).unwrap();
        assert_eq!(n, 1);
        assert_eq!(rec["decision_rules"][0]["action"], "do b");
        let left: Vec<&str> = c["decision_rules"].as_array().unwrap().iter().map(|r| r["condition"].as_str().unwrap()).collect();
        assert_eq!(left, vec!["a", "c"], "only b is gone; the others keep their text and order");
        assert_eq!(c["heuristics"].as_array().unwrap().len(), 1, "heuristics untouched");
        assert_eq!(c["principles"].as_array().unwrap().len(), 1, "principles untouched");
    }

    #[test]
    fn retire_a_heuristic_by_when_to_apply() {
        let mut c = body();
        let (_, n) = retire_rules(&mut c, &[], &["w1".to_string()]).unwrap();
        assert_eq!(n, 1);
        assert!(c["heuristics"].as_array().unwrap().is_empty());
        assert_eq!(c["decision_rules"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn retire_refuses_a_text_that_matches_nothing_and_changes_nothing() {
        let mut c = body();
        let before = c.clone();
        let e = retire_rules(&mut c, &["a".to_string(), "no such rule".to_string()], &[]).unwrap_err();
        assert!(e.contains("no such rule"), "{e}");
        assert_eq!(c, before, "all-or-nothing: the matching first entry was NOT removed either");
    }

    #[test]
    fn retire_is_exact_not_substring_and_trims_the_request() {
        let mut c = body();
        assert!(retire_rules(&mut c, &["a ".to_string()], &[]).is_ok(), "surrounding whitespace in the request is ignored");
        let mut c2 = body();
        assert!(retire_rules(&mut c2, &["do a".to_string()], &[]).is_err(), "the action text is not the condition; a substring is not a match");
        let mut c3 = serde_json::json!({"decision_rules": [{"condition": "alpha beta", "action": "x"}]});
        assert!(retire_rules(&mut c3, &["alpha".to_string()], &[]).is_err(), "a prefix does not match");
    }

    #[test]
    fn retire_with_no_text_is_an_error() {
        let mut c = body();
        assert!(retire_rules(&mut c, &["  ".to_string()], &[]).is_err());
        assert!(retire_rules(&mut c, &[], &[]).is_err());
    }

    #[test]
    fn a_methodology_never_ends_empty() {
        let mut c = serde_json::json!({"decision_rules": [{"condition": "only", "action": "x"}], "heuristics": []});
        retire_rules(&mut c, &["only".to_string()], &[]).unwrap();
        assert!(!has_real_rule(&c), "the guard the Update arm applies to the FINAL body sees no real rule left");
        let mut c2 = body();
        retire_rules(&mut c2, &["a".to_string()], &[]).unwrap();
        assert!(has_real_rule(&c2));
        let placeholders = serde_json::json!({"decision_rules": [{"condition": "...", "action": "x"}], "heuristics": [{"when_to_apply": "w", "description": "…"}]});
        assert!(!has_real_rule(&placeholders), "placeholders are not rules");
    }

    #[test]
    fn update_input_accepts_the_removal_fields_and_stays_backward_compatible() {
        let with: MethodologyCreateInput = serde_json::from_value(serde_json::json!({
            "action": "Update", "methodology_id": 7, "remove_decision_rules": ["a"], "remove_heuristics": ["w1"]
        }))
        .unwrap();
        match with {
            MethodologyCreateInput::Update { remove_decision_rules, remove_heuristics, .. } => {
                assert_eq!(remove_decision_rules.unwrap(), vec!["a".to_string()]);
                assert_eq!(remove_heuristics.unwrap(), vec!["w1".to_string()]);
            }
            _ => panic!("not an Update"),
        }
        let old: MethodologyCreateInput = serde_json::from_value(serde_json::json!({"action": "Update", "methodology_id": 7})).unwrap();
        assert!(matches!(old, MethodologyCreateInput::Update { remove_decision_rules: None, remove_heuristics: None, .. }));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_json = String::new();
    for i in 1..args.len() { if args[i] == "--input" && i + 1 < args.len() { input_json = ozone_input_arg(&args[i + 1]); } }
    // The host passes the full PipelineInput envelope {data, context} — every
    // real invocation from the orchestrator is wrapped this way. Unwrap
    // `data` when present; a bare MethodologyCreateInput (direct CLI use)
    // also still works.
    let parsed_value: serde_json::Value = serde_json::from_str(&input_json)
        .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let data_json = match parsed_value.get("data") {
        Some(d) => serde_json::to_string(d).unwrap_or_else(|_| input_json.clone()),
        None => input_json.clone(),
    };
    let input: MethodologyCreateInput = serde_json::from_str(&data_json).unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(execute(input)) {
        Ok(o) => println!("{}", serde_json::to_string(&o).unwrap()),
        Err(e) => { println!("{}", serde_json::json!({"success": false, "error": e})); std::process::exit(1); }
    }
}

/// `--input @<path>`: the host stages oversize inputs (> 100 KB) in a file because a
/// single argv string is capped at 128 KiB (E2BIG). Plain values pass through unchanged.
#[allow(dead_code)]
fn ozone_input_arg(raw: &str) -> String {
    match raw.strip_prefix('@') {
        Some(path) if std::path::Path::new(path).is_file() => std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Failed to read staged input {}: {}", path, e);
            std::process::exit(1);
        }),
        _ => raw.to_string(),
    }
}
