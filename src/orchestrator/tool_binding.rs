//! PROVIDER BINDING — methodology rule → required PROVIDER (MCP server) → blueprint step (2026-11-04).
//!
//! Operator doctrine (docs/METHODOLOGY_BLUEPRINT_TOOLS_REVIEW_2026-11-04.md Appendix A/B; docs/MCP_TOOLS_MODALITY_ARCHITECTURE_REVIEW_2026-11-04.md):
//! a PROVIDER is the MCP server process behind one or more tool names — the unit that is installed, trusted, started and shared.
//! Methodologies list PROVIDERS per rule (`requires: [{kind:"provider", name:"threed-mcp"}]`); blueprint creation translates
//! rule + provider into steps; WHICH TOOL of the provider is called is decided at call time from the step's own context
//! (we know `mesh_analyze` etc. — the methodology does not enumerate them). Containers a call produces record where they
//! were called from (provider, tool, site, task/step/rule — `mcp_graph::CalledFrom`).
//! Legacy/other kinds: `mcp`/`tool` (a tool NAME — resolved to its provider), `capability` (free text matched against provider
//! capability strings), `pipeline`. Run-time keyword search is only a fallback. Either we HAVE a provider or we DON'T:
//!   * never "list the first N registry tools" as a floor (removed from Stage 3);
//!   * a capability that is required but not registered is a MISSING CAPABILITY — captured durably
//!     (`capability_gaps.json`), announced on the ripple (meta event), turned into a methodology gap so the meta-loop
//!     drafts a methodology around it, and surfaced in the Marketplace as WANTED (to be built or downloaded after review).
//!
//! Pieces: [`gather_bindings`] (declared `requires[]` of the gathered methodologies' rules) → [`evaluate`] (registry
//! availability of the PROVIDER + its health → manifest + missing) → [`manifest_block`] (the blueprint prompt section) →
//! [`record_capability_gaps`] (durable + ripple + methodology-gap) → [`record_tool_outcome`] / [`degraded`] (tool health).

use super::StoreAccess;
use serde_json::{json, Value};
use std::sync::Arc;

fn data_dir() -> String {
    std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| "zsei_data".to_string())
}

/// One declared requirement of one rule of one methodology.
#[derive(Debug, Clone)]
pub struct Binding {
    /// "provider" (an MCP server id, e.g. `threed-mcp`) | "mcp"/"tool" (a tool name → its provider) | "capability" (free text) | "pipeline"
    pub kind: String,
    pub name: String,
    pub purpose: Option<String>,
    pub optional: bool,
    pub methodology_id: u64,
    pub methodology_name: String,
    pub rule_index: usize,
    pub rule_text: String,
}

impl Binding {
    /// Stable reference used by blueprint steps (`rule_refs`) and the manifest: `m<id>:r<index>`.
    pub fn rule_ref(&self) -> String {
        format!("m{}:r{}", self.methodology_id, self.rule_index)
    }
}

/// Read a container body file by its `storage.object_store_path`: absolute as given; relative → under the DATA dir first, then as given
/// (relative to the app root / cwd). ONE reader for every methodology consumer — live EU 2026-11-10: 48 of 98 methodologies keep their body
/// only under the app root, and the three separate readers (keyword validation, blueprint rules text, tool-binding `requires`) each read the
/// data dir alone, so those methodologies contributed NO rules to a blueprint and their declared provider requirements were never seen.
pub(crate) fn read_object_file(rel: &str) -> Option<String> {
    if std::path::Path::new(rel).is_absolute() {
        return std::fs::read_to_string(rel).ok();
    }
    std::fs::read_to_string(format!("{}/{}", data_dir(), rel)).ok().or_else(|| std::fs::read_to_string(rel).ok())
}

/// The methodology body JSON (the file `storage.object_store_path` points at).
fn read_methodology_body(container: &Value) -> Option<Value> {
    let rel = container.get("local_state")?.get("storage")?.get("object_store_path")?.as_str()?;
    serde_json::from_str(&read_object_file(rel)?).ok()
}

/// The methodology body JSON of a container value (shared reader: data dir, then app root).
pub(crate) fn read_methodology_body_value(container: &Value) -> Option<Value> {
    read_methodology_body(container)
}

/// Every DECLARED per-rule requirement of the given methodologies (empty when none declare any — legacy methodologies).
pub async fn gather_bindings(store: &Arc<dyn StoreAccess>, methodology_ids: &[u64]) -> Vec<Binding> {
    let mut out: Vec<Binding> = Vec::new();
    for &id in methodology_ids {
        let Ok(Some(c)) = store.get_container(id).await else { continue };
        let mname = c["local_state"]["metadata"]["name"].as_str().unwrap_or("").to_string();
        let Some(body) = read_methodology_body(&c) else { continue };
        let Some(rules) = body.get("decision_rules").and_then(|r| r.as_array()) else { continue };
        for (i, r) in rules.iter().enumerate() {
            let Some(reqs) = r.get("requires").and_then(|x| x.as_array()) else { continue };
            let cond = r.get("condition").and_then(|v| v.as_str()).unwrap_or("");
            let act = r.get("action").or_else(|| r.get("outcome")).and_then(|v| v.as_str()).unwrap_or("");
            for q in reqs {
                let name = q.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                if name.is_empty() {
                    continue;
                }
                out.push(Binding {
                    kind: q.get("kind").and_then(|v| v.as_str()).unwrap_or("provider").to_string(),
                    name,
                    purpose: q.get("purpose").and_then(|v| v.as_str()).map(String::from),
                    optional: q.get("optional").and_then(|v| v.as_bool()).unwrap_or(false),
                    methodology_id: id,
                    methodology_name: mname.clone(),
                    rule_index: i,
                    rule_text: format!("IF {cond} THEN {act}"),
                });
            }
        }
    }
    out
}

/// (name, capabilities, endpoint) of one registry entry — decouples this module from the registry's own type.
pub type RegistryRow = (String, Vec<String>, String);

pub struct Evaluated {
    /// One JSON object per binding: `{kind,name,purpose,optional,methodology_id,methodology_name,rule_index,rule_ref,rule_text,status,provider,source}`
    /// where status = registered | degraded | missing | unchecked (pipelines are validated against the available-pipeline list at blueprint time).
    pub manifest: Vec<Value>,
    /// The subset of the manifest that is `missing` (required AND optional; `optional` is carried so callers can decide).
    pub missing: Vec<Value>,
}

/// The provider a binding resolves to, and every registered tool that provider backs.
pub struct ProviderHit {
    pub provider: String,
    pub tools: Vec<String>,
}

/// provider id → its registered tool names (the registry grouped by server process).
pub fn providers_of(registry: &[RegistryRow]) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut m: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for r in registry {
        m.entry(crate::mcp_supervisor::provider_for_endpoint(&r.2)).or_default().push(r.0.clone());
    }
    m
}

fn find_provider(b: &Binding, registry: &[RegistryRow]) -> Option<ProviderHit> {
    let want = b.name.to_lowercase();
    let providers = providers_of(registry);
    let hit = |id: &str| providers.get(id).map(|tools| ProviderHit { provider: id.to_string(), tools: tools.clone() });
    match b.kind.as_str() {
        "provider" => providers.keys().find(|k| k.eq_ignore_ascii_case(&b.name)).and_then(|k| hit(k)),
        // a tool name resolves to the provider that backs it
        "mcp" | "tool" => registry
            .iter()
            .find(|r| r.0.eq_ignore_ascii_case(&b.name))
            .and_then(|r| hit(&crate::mcp_supervisor::provider_for_endpoint(&r.2))),
        // free-text capability: any provider that has a tool/capability containing it
        "capability" => registry
            .iter()
            .find(|r| r.0.to_lowercase().contains(&want) || r.1.iter().any(|c| c.to_lowercase().contains(&want)))
            .and_then(|r| hit(&crate::mcp_supervisor::provider_for_endpoint(&r.2))),
        _ => None,
    }
}

/// Availability of every binding against the registry (and provider health).
pub fn evaluate(bindings: &[Binding], registry: &[RegistryRow]) -> Evaluated {
    let mut manifest = Vec::new();
    let mut missing = Vec::new();
    for b in bindings {
        let hit = match b.kind.as_str() {
            "pipeline" => None,
            _ => find_provider(b, registry),
        };
        let status = match (&hit, b.kind.as_str()) {
            (_, "pipeline") => "unchecked",
            (Some(h), _) if degraded(&h.provider) => "degraded",
            (Some(_), _) => "registered",
            (None, _) => "missing",
        };
        let entry = json!({
            "kind": b.kind, "name": b.name, "purpose": b.purpose, "optional": b.optional,
            "methodology_id": b.methodology_id, "methodology_name": b.methodology_name,
            "rule_index": b.rule_index, "rule_ref": b.rule_ref(), "rule_text": b.rule_text,
            "status": status,
            // the PROVIDER (MCP server) that satisfies it, and every tool it backs (which one is called is decided at call time)
            "provider": hit.as_ref().map(|h| h.provider.clone()),
            "tools": hit.as_ref().map(|h| h.tools.clone()).unwrap_or_default(),
            "source": "methodology",
        });
        if status == "missing" {
            missing.push(entry.clone());
        }
        manifest.push(entry);
    }
    Evaluated { manifest, missing }
}

/// The blueprint-prompt section (RULE → REQUIRED CAPABILITIES). Honest when nothing is declared.
pub fn manifest_block(manifest: &[Value]) -> String {
    if manifest.is_empty() {
        return "(no gathered methodology rule declares a required provider — use only the REGISTERED CAPABILITIES above, or list what is needed in missing_capabilities)".to_string();
    }
    let mut lines: Vec<String> = Vec::new();
    for m in manifest {
        lines.push(format!(
            "- [{}] {} → {} \"{}\" [{}{}]{}{}",
            m["rule_ref"].as_str().unwrap_or("?"),
            m["rule_text"].as_str().unwrap_or(""),
            m["kind"].as_str().unwrap_or("provider"),
            m["name"].as_str().unwrap_or(""),
            m["status"].as_str().unwrap_or("unchecked"),
            if m["optional"].as_bool().unwrap_or(false) { ", optional" } else { "" },
            m["purpose"].as_str().map(|p| format!(" — {p}")).unwrap_or_default(),
            m["tools"].as_array().filter(|t| !t.is_empty()).map(|t| format!(" (provider tools: {})", t.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", "))).unwrap_or_default(),
        ));
    }
    lines.join("\n")
}

/// The provider (MCP server id) that backs a registered tool name — used to stamp "called from" provenance.
pub async fn provider_of_tool(tool: &str) -> Option<String> {
    let entries = crate::mcp::registry_entries().await?;
    entries.iter().find(|t| t.name == tool).map(|t| crate::mcp_supervisor::provider_for_endpoint(&t.endpoint))
}

// ---------------------------------------------------------------------------------------------
// MISSING CAPABILITIES — captured, announced, and turned into methodology + tool requests
// ---------------------------------------------------------------------------------------------

pub struct GapInput {
    /// mcp | tool | capability | pipeline
    pub kind: String,
    pub name: String,
    /// methodology-binding | blueprint | simulation | step
    pub source: String,
    /// Free text: which rule / which request / which step needed it.
    pub detail: String,
}

fn gaps_path() -> String {
    format!("{}/capability_gaps.json", data_dir())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// All recorded capability gaps (the Marketplace "wanted" list).
pub fn capability_gaps() -> Vec<Value> {
    std::fs::read_to_string(gaps_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// Durably record required-but-unavailable capabilities. For every NEW one (first sighting) this also
///   1. emits the `CapabilityGap` meta ripple (wakes the meta loops — no timer),
///   2. records a METHODOLOGY GAP (keywords from the capability) so the meta-loop drafts a methodology around it,
///   3. leaves a `status:"wanted"` row the Marketplace shows (build or download-after-review the tool/MCP).
/// A gap whose capability is registered later is closed by [`close_satisfied_gaps`]. Returns how many were NEW.
pub fn record_capability_gaps(gaps: &[GapInput]) -> usize {
    if gaps.is_empty() {
        return 0;
    }
    let mut rows = capability_gaps();
    let mut new_count = 0usize;
    let now = now_secs();
    for g in gaps {
        let name = g.name.trim();
        if name.is_empty() {
            continue;
        }
        let key = (g.kind.to_lowercase(), name.to_lowercase());
        let existing = rows.iter_mut().find(|r| {
            r["kind"].as_str().map(|k| k.to_lowercase()) == Some(key.0.clone()) && r["name"].as_str().map(|n| n.to_lowercase()) == Some(key.1.clone())
        });
        match existing {
            Some(r) => {
                let count = r["count"].as_u64().unwrap_or(0) + 1;
                r["count"] = json!(count);
                r["last_seen"] = json!(now);
                if r["status"].as_str() == Some("satisfied") {
                    r["status"] = json!("wanted"); // it vanished again
                }
                if let Some(d) = r["details"].as_array_mut() {
                    d.push(json!({ "source": g.source, "detail": g.detail, "at": now })); // full sighting history (no cap)
                }
            }
            None => {
                new_count += 1;
                rows.push(json!({
                    "kind": g.kind, "name": name, "status": "wanted", "count": 1,
                    "first_seen": now, "last_seen": now,
                    "details": [{ "source": g.source, "detail": g.detail, "at": now }],
                }));
                let keywords: Vec<String> = std::iter::once("capability".to_string())
                    .chain(name.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() > 2).map(|w| w.to_lowercase()))
                    .collect();
                super::PromptOrchestrator::record_methodology_gap(&keywords, &["missing capability".to_string()]);
            }
        }
    }
    // no row cap (no-cap doctrine, 2026-11-06): a missing capability is a durable "wanted" record — never evicted by age.
    if let Ok(json) = serde_json::to_string_pretty(&rows) {
        let path = gaps_path();
        if let Some(dir) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, json);
    }
    if new_count > 0 {
        tracing::warn!(new = new_count, "missing capabilities recorded — wanted in the Marketplace, methodology gap queued for the meta-loop");
        super::ripple_class::emit_capability_gap();
    }
    new_count
}

/// Record what the walk DECIDED about a gap (operator 2026-11-06: missing capabilities are acted on in the walk, not just recorded):
/// `status` = "not_needed" (a yes/no judgement said the request does not require it) | "methodology_drafted" (a methodology that achieves it
/// was created on the spot) | "wanted" (still needed and could not be closed). History is kept in `resolutions`; matching is by name.
pub fn set_gap_status(name: &str, status: &str, reason: &str, by: &str) {
    let mut rows = capability_gaps();
    let now = now_secs();
    let mut changed = false;
    for r in rows.iter_mut() {
        if r["name"].as_str().map(|n| n.eq_ignore_ascii_case(name.trim())) != Some(true) {
            continue;
        }
        r["status"] = json!(status);
        if !r["resolutions"].is_array() {
            r["resolutions"] = json!([]);
        }
        if let Some(a) = r["resolutions"].as_array_mut() {
            a.push(json!({ "status": status, "reason": reason, "by": by, "at": now }));
        }
        changed = true;
    }
    if changed {
        if let Ok(json) = serde_json::to_string_pretty(&rows) {
            let _ = std::fs::write(gaps_path(), json);
        }
    }
}

/// Record a LIFECYCLE EVENT on a gap (e.g. "replanned", "verified") WITHOUT changing its `status`: one more entry in `resolutions` (the gap's
/// history). Idempotent per event name (the first occurrence is the record; a second call changes nothing). The work-order tree
/// (`orchestrator/work_order.rs`) ticks its checklist items from these real entries.
pub fn note_gap_event(name: &str, event: &str, reason: &str, by: &str) {
    let mut rows = capability_gaps();
    let now = now_secs();
    let mut changed = false;
    for r in rows.iter_mut() {
        if r["name"].as_str().map(|n| n.eq_ignore_ascii_case(name.trim())) != Some(true) {
            continue;
        }
        if !r["resolutions"].is_array() {
            r["resolutions"] = json!([]);
        }
        if let Some(a) = r["resolutions"].as_array_mut() {
            if a.iter().any(|x| x["status"].as_str() == Some(event)) {
                continue;
            }
            a.push(json!({ "status": event, "reason": reason, "by": by, "at": now }));
            changed = true;
        }
    }
    if changed {
        if let Ok(json) = serde_json::to_string_pretty(&rows) {
            let _ = std::fs::write(gaps_path(), json);
        }
    }
}

/// Link a gap to its work-order task (`work_order_task` = the current one; every earlier one stays in `work_order_history` — a rerun is a NEW
/// task, the old one is never reused or edited).
pub fn link_gap_task(name: &str, task_id: u64) {
    let mut rows = capability_gaps();
    let mut changed = false;
    for r in rows.iter_mut() {
        if r["name"].as_str().map(|n| n.eq_ignore_ascii_case(name.trim())) != Some(true) {
            continue;
        }
        if r["work_order_task"].as_u64() == Some(task_id) {
            continue;
        }
        if let Some(old) = r["work_order_task"].as_u64() {
            if !r["work_order_history"].is_array() {
                r["work_order_history"] = json!([]);
            }
            if let Some(h) = r["work_order_history"].as_array_mut() {
                h.push(json!(old));
            }
        }
        r["work_order_task"] = json!(task_id);
        changed = true;
    }
    if changed {
        if let Ok(json) = serde_json::to_string_pretty(&rows) {
            let _ = std::fs::write(gaps_path(), json);
        }
    }
}

/// Mark `wanted` gaps whose capability is now registered as `satisfied` (keeps the history; reopens if it vanishes).
pub fn close_satisfied_gaps(registry: &[RegistryRow]) {
    let mut rows = capability_gaps();
    let mut changed = false;
    for r in rows.iter_mut() {
        if r["status"].as_str() != Some("wanted") {
            continue;
        }
        let b = Binding {
            kind: r["kind"].as_str().unwrap_or("provider").to_string(),
            name: r["name"].as_str().unwrap_or("").to_string(),
            purpose: None,
            optional: false,
            methodology_id: 0,
            methodology_name: String::new(),
            rule_index: 0,
            rule_text: String::new(),
        };
        if find_provider(&b, registry).is_some() {
            r["status"] = json!("satisfied");
            r["satisfied_at"] = json!(now_secs());
            changed = true;
        }
    }
    if changed {
        if let Ok(json) = serde_json::to_string_pretty(&rows) {
            let _ = std::fs::write(gaps_path(), json);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// TOOL HEALTH — a failing tool marks the bindings that depend on it `degraded`
// ---------------------------------------------------------------------------------------------

fn health_path() -> String {
    format!("{}/tool_health.json", data_dir())
}

fn read_health() -> serde_json::Map<String, Value> {
    std::fs::read_to_string(health_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// Record one real call outcome against its PROVIDER (the last 10 are kept per provider; `tool` is kept for diagnosis).
pub fn record_tool_outcome(provider: &str, tool: &str, ok: bool, error: Option<&str>) {
    let mut h = read_health();
    let entry = h.entry(provider.to_string()).or_insert_with(|| json!({ "recent": [] }));
    if let Some(a) = entry["recent"].as_array_mut() {
        a.push(json!({ "tool": tool, "ok": ok, "at": now_secs(), "error": error.map(String::from) }));
        let n = a.len();
        if n > 10 {
            a.drain(0..n - 10);
        }
    }
    if let Ok(json) = serde_json::to_string_pretty(&Value::Object(h)) {
        let _ = std::fs::write(health_path(), json);
    }
}

/// True when at least 3 of the provider's last 10 calls were recorded and at least half failed.
pub fn degraded(provider: &str) -> bool {
    let h = read_health();
    let Some(a) = h.get(provider).and_then(|e| e["recent"].as_array()) else { return false };
    let fails = a.iter().filter(|r| r["ok"].as_bool() == Some(false)).count();
    a.len() >= 3 && fails * 2 >= a.len()
}

/// Health summary for the Marketplace: `{provider: {calls, failures, degraded, last_error}}`.
pub fn health_summary() -> Value {
    let h = read_health();
    let mut out = serde_json::Map::new();
    for (tool, e) in h.iter() {
        let a = e["recent"].as_array().cloned().unwrap_or_default();
        let fails = a.iter().filter(|r| r["ok"].as_bool() == Some(false)).count();
        out.insert(
            tool.clone(),
            json!({
                "calls": a.len(), "failures": fails, "degraded": degraded(tool),
                "last_error": a.iter().rev().find_map(|r| r["error"].as_str()).map(String::from),
            }),
        );
    }
    Value::Object(out)
}

// ---------------------------------------------------------------------------------------------
// LEARN ONCE, BIND FOREVER — a search-selected tool that really worked becomes a PROPOSED binding
// ---------------------------------------------------------------------------------------------

fn proposals_path() -> String {
    format!("{}/tool_binding_proposals.json", data_dir())
}

/// A provider picked by run-time SEARCH answered a step for real: remember (methodology ids of the step, provider, step text) so the
/// meta-loop / operator can promote it to a declared per-rule `requires[]` binding. Proposals are never auto-applied.
pub fn record_binding_proposal(methodology_ids: &[u64], provider: &str, step_text: &str) {
    let tool = provider;
    if methodology_ids.is_empty() || tool.is_empty() {
        return;
    }
    let mut rows: Vec<Value> = std::fs::read_to_string(proposals_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    for &m in methodology_ids {
        if let Some(r) = rows.iter_mut().find(|r| r["methodology_id"].as_u64() == Some(m) && r["tool"].as_str() == Some(tool)) {
            r["successes"] = json!(r["successes"].as_u64().unwrap_or(0) + 1);
            r["last_seen"] = json!(now_secs());
        } else {
            rows.push(json!({
                "methodology_id": m, "provider": tool, "tool": tool, "successes": 1, "status": "proposed",
                "example_step": step_text,
                "first_seen": now_secs(), "last_seen": now_secs(),
            }));
        }
    }
    // no row cap (no-cap doctrine, 2026-11-06): binding proposals are durable records.
    if let Ok(json) = serde_json::to_string_pretty(&rows) {
        let _ = std::fs::write(proposals_path(), json);
    }
}

pub fn binding_proposals() -> Vec<Value> {
    std::fs::read_to_string(proposals_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(kind: &str, name: &str) -> Binding {
        Binding { kind: kind.into(), name: name.into(), purpose: Some("why".into()), optional: false, methodology_id: 7, methodology_name: "M".into(), rule_index: 2, rule_text: "IF a THEN b".into() }
    }
    /// threed-mcp (port 3250) backs two tools; mesh-mcp (3276) one.
    fn reg() -> Vec<RegistryRow> {
        vec![
            ("clock_generate".into(), vec!["3d".into(), "blender".into()], "http://127.0.0.1:3250/call".into()),
            ("cad_clock_generate".into(), vec!["cad".into()], "http://127.0.0.1:3250/call".into()),
            ("mesh_analyze".into(), vec!["mesh".into()], "http://127.0.0.1:3276/call".into()),
        ]
    }

    #[test]
    fn provider_bindings_resolve_to_the_server_and_list_its_tools() {
        let e = evaluate(&[b("provider", "threed-mcp"), b("provider", "no-such-mcp"), b("mcp", "mesh_analyze"), b("capability", "blender"), b("pipeline", "9")], &reg());
        assert_eq!(e.manifest.len(), 5);
        assert_eq!(e.manifest[0]["status"], "registered");
        assert_eq!(e.manifest[0]["provider"], "threed-mcp");
        assert_eq!(e.manifest[0]["tools"].as_array().unwrap().len(), 2);
        assert_eq!(e.manifest[1]["status"], "missing");
        assert_eq!(e.manifest[2]["provider"], "mesh-mcp"); // a tool name resolves to ITS provider
        assert_eq!(e.manifest[3]["provider"], "threed-mcp"); // capability text resolves to a provider
        assert_eq!(e.manifest[4]["status"], "unchecked");
        assert_eq!(e.missing.len(), 1);
        assert_eq!(e.manifest[0]["rule_ref"], "m7:r2");
    }

    #[test]
    fn manifest_block_is_honest_when_nothing_is_declared() {
        assert!(manifest_block(&[]).contains("no gathered methodology rule declares"));
        let e = evaluate(&[b("provider", "threed-mcp")], &reg());
        let blk = manifest_block(&e.manifest);
        assert!(blk.contains("[m7:r2]") && blk.contains("clock_generate"));
    }
}
