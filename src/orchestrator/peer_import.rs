//! PEER IMPORT — the DOWNLOAD half of the marketplace (operator rulings 2026-11-10: notify and gate, never auto-sync).
//!
//! An offer becomes a download only when the operator decided `download` on it (`POST /marketplace/offers/decide` →
//! `peer_offer_decisions.json`). This loop then:
//!   * `methodology` → asks the offering peer for the body (`fetch_request`), reassembles the chunks (`fetch_chunk`), RE-VERIFIES the body hash
//!     (identity = name + hash — an altered or truncated body is rejected), validates it is a JSON object, and imports it through the normal
//!     create path (`store.create_container` → write-time keyword gate → pending → settlement validation) with the body stored BYTE-FOR-BYTE
//!     (so its hash stays equal on both sides). Same name + same hash = already have it; same name + different hash = imported under a distinct
//!     name ("… (from peer xxxxxx)") — both are kept, nothing is ever overwritten.
//!   * `jurisdiction_rule_set` → fetched and VALIDATED (`JurisdictionContentFile`: mandatory disclaimer, rules present) and only STAGED under
//!     `peer_staging/jurisdiction/` — never registered, never active: this is the safety layer, activating it is a human step.
//!   * `tool` / `pipeline` → approval is recorded and NOTHING is fetched: executables need a human, reviewed install.
//! Every outcome is recorded in `peer_import_done.json`; an item imported from a peer is listed in `peer_imported_items.json` so the marketplace
//! bridge does not echo it back. A request with no answer in 15 minutes is marked failed (re-deciding `download` retries it).

use crate::network::{body_hash, NetworkManager};
use crate::orchestrator::StoreAccess;
use crate::types::container::{
    CompressionType, Container, ContainerType, Context, GlobalState, IntegrityData, LocalState, Metadata, Modality, StoragePointers,
    TraversalHints, METHODOLOGY_ROOT_ID,
};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const REQUEST_TIMEOUT_SECS: u64 = 15 * 60;

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn read_map(path: &Path) -> Map<String, Value> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn write_map(path: &Path, m: &Map<String, Value>) {
    let _ = std::fs::write(path, serde_json::to_string_pretty(m).unwrap_or_default());
}

/// `from|item_type|name|hash` → (from, item_type, name, hash). The name may itself contain `|`, so the first two and the last segment are taken
/// from the ends.
pub fn parse_offer_key(key: &str) -> Option<(String, String, String, String)> {
    let parts: Vec<&str> = key.split('|').collect();
    if parts.len() < 4 {
        return None;
    }
    let (from, item_type, hash) = (parts[0], parts[1], parts[parts.len() - 1]);
    let name = parts[2..parts.len() - 1].join("|");
    if from.is_empty() || item_type.is_empty() || name.is_empty() {
        return None;
    }
    Some((from.to_string(), item_type.to_string(), name, hash.to_string()))
}

/// A name that does not collide with `existing` (compared case-insensitively): the original if free, otherwise "<name> (from peer <last 6 of the
/// peer id>)" with a numeric suffix if even that is taken. Both versions are kept — an import never overwrites.
pub fn unique_import_name(name: &str, existing: &HashSet<String>, from_peer: &str) -> String {
    if !existing.contains(&name.to_lowercase()) {
        return name.to_string();
    }
    let tag: String = from_peer.chars().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
    let mut candidate = format!("{name} (from peer {tag})");
    let mut n = 2;
    while existing.contains(&candidate.to_lowercase()) {
        candidate = format!("{name} (from peer {tag} #{n})");
        n += 1;
    }
    candidate
}

/// A methodology body must be a JSON object.
pub fn validate_methodology_body(body: &str) -> Result<(), String> {
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(_)) => Ok(()),
        Ok(_) => Err("the body is not a JSON object".to_string()),
        Err(e) => Err(format!("the body is not valid JSON: {e}")),
    }
}

/// A jurisdiction content file must parse as `JurisdictionContentFile` with a non-empty disclaimer and at least one rule. Returns the rule count.
pub fn validate_jurisdiction_content(body: &str) -> Result<usize, String> {
    let parsed: crate::orchestrator::jurisdiction::JurisdictionContentFile =
        serde_json::from_str(body).map_err(|e| format!("not a jurisdiction content file: {e}"))?;
    if parsed.disclaimer.trim().is_empty() {
        return Err("the mandatory disclaimer is empty".to_string());
    }
    if parsed.rules.is_empty() {
        return Err("the rule set has no rules".to_string());
    }
    Ok(parsed.rules.len())
}

fn slug(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() {
        "item".to_string()
    } else {
        s.chars().take(60).collect()
    }
}

/// Reassemble the parts (`seq` → data) of a fetched body; None until every part 0..total is present.
pub fn assemble_parts(parts: &Map<String, Value>, total: usize) -> Option<String> {
    let mut out = String::new();
    for i in 0..total {
        out.push_str(parts.get(&i.to_string())?.as_str()?);
    }
    Some(out)
}

/// One offer as received from a peer (already sanitized by the network door).
#[derive(Debug, Clone)]
pub struct RawOffer {
    /// `from|item_type|name|hash` — the offer's identity and the key of every decision / review / import record
    pub key: String,
    pub from: String,
    pub item_type: String,
    pub name: String,
    pub hash: String,
    pub keywords: Vec<String>,
    pub announced_at: Option<u64>,
    pub received_at: Option<u64>,
}

/// What THIS instance already has, per offer kind, for the deterministic part of the gate (no model). Built from real local state only:
///   * tools      — names in `mcp_tool_registry.json`;
///   * pipelines  — the installed pipeline registry (display name and folder name);
///   * jurisdiction — the scope files under `<data_dir>/jurisdiction/` (`global.json`, `national/<iso>.json`): stem → hash of the file's bytes
///                    (the body hash the announcing side computes from its own body file, so identical content hashes equal).
#[derive(Debug, Default, Clone)]
pub struct LocalOfferIndex {
    pub tool_names: HashSet<String>,
    pub pipeline_names: HashSet<String>,
    pub jurisdiction: std::collections::HashMap<String, String>,
}

impl LocalOfferIndex {
    pub fn load(data_dir: &Path) -> Self {
        let tool_names = std::fs::read_to_string(data_dir.join("mcp_tool_registry.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v.as_array().map(|a| a.iter().filter_map(|e| e["name"].as_str().map(|n| n.trim().to_lowercase())).collect::<HashSet<String>>()))
            .unwrap_or_default();
        let mut pipeline_names: HashSet<String> = HashSet::new();
        for id in crate::pipeline::registry::get_all_pipeline_ids() {
            if let Some(n) = crate::pipeline::registry::get_pipeline_name(id) {
                pipeline_names.insert(n.trim().to_lowercase());
            }
            if let Some(f) = crate::pipeline::registry::get_pipeline_folder(id) {
                pipeline_names.insert(f.trim().to_lowercase());
            }
        }
        let mut jurisdiction = std::collections::HashMap::new();
        let root = data_dir.join("jurisdiction");
        let mut dirs = vec![root.clone(), root.join("national")];
        dirs.dedup();
        for d in dirs {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let path = e.path();
                if path.extension().map_or(true, |x| x != "json") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|x| x.to_str()).map(|x| x.to_lowercase()) else { continue };
                if stem.contains("registry") || stem.contains("candidates") {
                    continue; // not scope files
                }
                let hash = std::fs::read(&path).map(|b| body_hash(&b)).unwrap_or_default();
                jurisdiction.insert(stem, hash);
            }
        }
        Self { tool_names, pipeline_names, jurisdiction }
    }
}

/// Lower-case, non-alphanumerics collapsed to single spaces — the form names are compared in.
pub fn norm_name(s: &str) -> String {
    s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

/// The local status of ONE offer — the labels the UI chips handle: `have` (same name, same body hash), `conflict` (same name, different body
/// hash), `same_name` (same name, and no hash can be compared for this kind — NEVER "identical"), `new`, `not_checked`. Methodologies compare
/// name + hash against `local_methods`; jurisdiction rule sets compare the region key (the name without a "Jurisdiction:" prefix) against the
/// local scope files and, when the offer carries a body hash, the file's hash; tools and pipelines compare names only.
pub fn classify_offer(o: &RawOffer, local_methods: &std::collections::HashMap<String, String>, idx: &LocalOfferIndex) -> &'static str {
    match o.item_type.as_str() {
        "methodology" => match local_methods.get(&o.name) {
            Some(h) if !o.hash.is_empty() && *h == o.hash => "have",
            Some(_) => "conflict",
            None => "new",
        },
        "tool" => {
            if idx.tool_names.contains(&o.name.trim().to_lowercase()) {
                "same_name"
            } else {
                "new"
            }
        }
        "pipeline" => {
            if idx.pipeline_names.contains(&o.name.trim().to_lowercase()) {
                "same_name"
            } else {
                "new"
            }
        }
        "jurisdiction_rule_set" => {
            let lower = o.name.trim().to_lowercase();
            let key = lower.strip_prefix("jurisdiction:").map(str::trim).unwrap_or(&lower).to_string();
            match idx.jurisdiction.get(&key) {
                Some(h) if !o.hash.is_empty() && !h.is_empty() => {
                    if *h == o.hash {
                        "have"
                    } else {
                        "conflict"
                    }
                }
                Some(_) => "same_name",
                None => "new",
            }
        }
        _ => "not_checked",
    }
}

/// The names of capability gaps this instance still WANTS (`capability_gaps.json`, status `wanted` or `missing`).
pub fn wanted_gap_names() -> Vec<String> {
    crate::orchestrator::tool_binding::capability_gaps()
        .into_iter()
        .filter(|g| matches!(g["status"].as_str(), Some("wanted") | Some("missing")))
        .filter_map(|g| g["name"].as_str().map(String::from))
        .collect()
}

/// A tool or pipeline offer that matches a WANTED capability gap, deterministically (no model): the offer's name equals the gap's name, one
/// contains the other as whole words, or an offer keyword equals the gap's name. Returns the gap name. ADVISORY ONLY — it marks the offer in the
/// view and in the review; it never approves, downloads or installs anything.
pub fn matches_wanted(o: &RawOffer, wanted: &[String]) -> Option<String> {
    if !matches!(o.item_type.as_str(), "tool" | "pipeline") {
        return None;
    }
    let offer = norm_name(&o.name);
    let kws: Vec<String> = o.keywords.iter().map(|k| norm_name(k)).collect();
    let words_in = |hay: &str, needle: &str| -> bool { !needle.is_empty() && format!(" {hay} ").contains(&format!(" {needle} ")) };
    wanted.iter().find(|g| {
        let gn = norm_name(g);
        !gn.is_empty() && (offer == gn || words_in(&offer, &gn) || words_in(&gn, &offer) || kws.iter().any(|k| *k == gn))
    }).cloned()
}

/// Every distinct offer in the marketplace inbox (`announce` rows and the items of `catalog` rows), latest wins. Blueprints and unknown types can
/// never be here (the door refuses them) but are skipped again defensively.
pub fn collect_offers(dir: &Path) -> Vec<RawOffer> {
    let Ok(text) = std::fs::read_to_string(dir.join("peer_marketplace_inbox.jsonl")) else { return Vec::new() };
    let mut offers: std::collections::BTreeMap<String, RawOffer> = Default::default();
    for line in text.lines() {
        let Ok(row) = serde_json::from_str::<Value>(line) else { continue };
        let from = row["from_peer"].as_str().unwrap_or("unknown-peer");
        let manifests: Vec<Value> = match row["envelope"]["kind"].as_str() {
            Some("announce") => vec![row["envelope"].clone()],
            Some("catalog") => row["envelope"]["items"].as_array().cloned().unwrap_or_default(),
            _ => continue,
        };
        for m in manifests {
            let (item_type, name, hash) = (m["item_type"].as_str().unwrap_or(""), m["name"].as_str().unwrap_or(""), m["body_hash"].as_str().unwrap_or(""));
            if name.is_empty() || !crate::network::ALLOWED_ITEM_TYPES.contains(&item_type) {
                continue;
            }
            let key = format!("{from}|{item_type}|{name}|{hash}");
            offers.insert(
                key.clone(),
                RawOffer {
                    key,
                    from: from.to_string(),
                    item_type: item_type.to_string(),
                    name: name.to_string(),
                    hash: hash.to_string(),
                    keywords: m["keywords"].as_array().map(|a| a.iter().filter_map(|k| k.as_str().map(String::from)).collect()).unwrap_or_default(),
                    announced_at: m["announced_at"].as_u64(),
                    received_at: row["received_at"].as_u64(),
                },
            );
        }
    }
    offers.into_values().collect()
}

fn mark_imported(dir: &Path, name: &str, hash: &str) {
    let path = dir.join("peer_imported_items.json");
    let mut m = read_map(&path);
    m.insert(format!("{name}|{hash}"), json!(now_secs()));
    write_map(&path, &m);
}

enum Imported {
    Have,
    Created { id: u64, name: String },
}

async fn import_methodology(store: &Arc<dyn StoreAccess>, dir: &Path, name: &str, hash: &str, body: &str, keywords: &[String], from: &str) -> Result<Imported, String> {
    validate_methodology_body(body)?;
    // what already exists (names), and whether the SAME content is already here
    let mut existing: HashSet<String> = HashSet::new();
    for id in store.list_ids(Some("Methodology")).await.unwrap_or_default() {
        let Ok(Some(c)) = store.get_container(id).await else { continue };
        let Some(n) = c.pointer("/local_state/metadata/name").and_then(|v| v.as_str()) else { continue };
        if n.eq_ignore_ascii_case(name) {
            let same = c
                .pointer("/local_state/storage/object_store_path")
                .and_then(|v| v.as_str())
                .and_then(crate::orchestrator::tool_binding::read_object_file)
                .map(|b| body_hash(b.as_bytes()) == hash)
                .unwrap_or(false);
            if same {
                return Ok(Imported::Have);
            }
        }
        existing.insert(n.to_lowercase());
    }
    let final_name = unique_import_name(name, &existing, from);
    // the body is written EXACTLY as received, so its hash is the same on both sides
    let rel = format!("methodologies_imported/{}-{}.json", slug(&final_name), hash.chars().take(8).collect::<String>());
    std::fs::create_dir_all(dir.join("methodologies_imported")).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(&rel), body).map_err(|e| e.to_string())?;
    // listed BEFORE the create, so the marketplace bridge (which sees the created event) does not publish it straight back
    mark_imported(dir, &final_name, hash);
    let now = now_secs();
    let container = Container {
        global_state: GlobalState { container_id: 0, parent_id: METHODOLOGY_ROOT_ID, child_ids: vec![], child_count: 0, version: 1 },
        local_state: LocalState {
            metadata: Metadata {
                container_type: ContainerType::Methodology,
                modality: Modality::Unknown,
                created_at: now,
                updated_at: now,
                provenance: format!("peer-import:{}", from.chars().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect::<String>()),
                permissions: 0,
                owner_id: 0,
                name: Some(final_name.clone()),
                materialized_path: None,
            },
            context: Context {
                categories: vec![],
                methodologies: vec![],
                keywords: keywords.to_vec(),
                topics: vec![],
                relationships: vec![],
                learned_associations: vec![],
                keyword_meta: Default::default(),
                scope: Default::default(),
                embedding: None,
            },
            storage: StoragePointers { db_shard_id: None, vector_index_ref: None, object_store_path: Some(rel), compression_type: CompressionType::None },
            hints: TraversalHints::default(),
            integrity: IntegrityData::default(),
            file_context: None,
            code_context: None,
            text_context: None,
            external_ref: None,
        },
    };
    let json = serde_json::to_value(&container).map_err(|e| e.to_string())?;
    // the keyword write-gate registers the keywords as PENDING; settlement validates them — an imported item gets no shortcut
    let id = crate::keyword_program::with_origin(crate::keyword_program::Origin::actor("peer-import"), store.create_container(METHODOLOGY_ROOT_ID, json)).await?;
    Ok(Imported::Created { id, name: final_name })
}

/// One pass: start the approved downloads, expire stale requests, and process what the inbox brought back.
async fn import_pass(network: &Arc<tokio::sync::RwLock<NetworkManager>>, store: &Arc<dyn StoreAccess>, dir: &Path) -> Result<(), String> {
    let decisions = read_map(&dir.join("peer_offer_decisions.json"));
    let done_path = dir.join("peer_import_done.json");
    let pending_path = dir.join("peer_fetch_pending.json");
    let parts_path = dir.join("peer_fetch_parts.json");
    let mut done = read_map(&done_path);
    let mut pending = read_map(&pending_path);
    let mut parts = read_map(&parts_path);

    // 1. start approved downloads
    for (key, d) in decisions.iter() {
        if d["decision"].as_str() != Some("download") || done.contains_key(key) || pending.contains_key(key) {
            continue;
        }
        let Some((from, item_type, name, hash)) = parse_offer_key(key) else { continue };
        match item_type.as_str() {
            "tool" | "pipeline" => {
                done.insert(key.clone(), json!({"status": "approval_only", "at": now_secs(), "detail": "executables and tools need a human, reviewed install — approval recorded, nothing was fetched"}));
            }
            "methodology" | "jurisdiction_rule_set" => {
                let Ok(peer) = from.parse::<libp2p::PeerId>() else {
                    done.insert(key.clone(), json!({"status": "failed", "at": now_secs(), "detail": "the offering peer id is not valid"}));
                    continue;
                };
                let req = json!({"kind": "fetch_request", "item_type": item_type, "name": name, "body_hash": hash}).to_string();
                if network.read().await.send_to_connected_peer(&peer, req.as_bytes()).await.is_ok() {
                    pending.insert(key.clone(), json!({"requested_at": now_secs(), "from": from, "item_type": item_type, "name": name, "body_hash": hash}));
                }
            }
            other => {
                done.insert(key.clone(), json!({"status": "rejected", "at": now_secs(), "detail": format!("unknown item type {other}")}));
            }
        }
    }

    // 2. expire requests nobody answered
    let stale: Vec<String> = pending.iter().filter(|(_, v)| now_secs().saturating_sub(v["requested_at"].as_u64().unwrap_or(0)) > REQUEST_TIMEOUT_SECS).map(|(k, _)| k.clone()).collect();
    for k in stale {
        pending.remove(&k);
        parts.remove(&k);
        done.insert(k, json!({"status": "failed", "at": now_secs(), "detail": "no response from the peer (it may be offline or no longer publishes this item); decide download again to retry"}));
    }

    // 3. what the inbox brought back for requests WE made
    let offset_path = dir.join("peer_import.offset");
    let offset: usize = std::fs::read_to_string(&offset_path).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let mut advanced = offset;
    if let Ok(content) = std::fs::read_to_string(dir.join("peer_marketplace_inbox.jsonl")) {
        if content.len() > offset && content.is_char_boundary(offset) {
            let fresh = &content[offset..];
            let upto = fresh.rfind('\n').map(|i| i + 1).unwrap_or(0);
            advanced = offset + upto;
            for line in fresh[..upto].lines() {
                let Ok(row) = serde_json::from_str::<Value>(line) else { continue };
                let env = &row["envelope"];
                let kind = env["kind"].as_str().unwrap_or("");
                if kind != "fetch_chunk" && kind != "fetch_error" {
                    continue;
                }
                let (item_type, name, hash) = (env["item_type"].as_str().unwrap_or(""), env["name"].as_str().unwrap_or(""), env["body_hash"].as_str().unwrap_or(""));
                // only answers to a request WE made are accepted (matched on type + name + hash)
                let Some(key) = pending.iter().find(|(_, v)| v["item_type"] == item_type && v["name"] == name && v["body_hash"] == hash).map(|(k, _)| k.clone()) else { continue };
                if kind == "fetch_error" {
                    pending.remove(&key);
                    parts.remove(&key);
                    done.insert(key, json!({"status": "failed", "at": now_secs(), "detail": env["reason"].as_str().unwrap_or("the peer could not serve it")}));
                    continue;
                }
                let (seq, total) = (env["seq"].as_u64().unwrap_or(0) as usize, env["total"].as_u64().unwrap_or(0) as usize);
                if total == 0 || seq >= total {
                    continue;
                }
                let entry = parts.entry(key.clone()).or_insert_with(|| json!({"total": total, "parts": {}, "keywords": []}));
                entry["total"] = json!(total);
                entry["parts"][seq.to_string()] = env["data"].clone();
                if let Some(k) = env["keywords"].as_array() {
                    entry["keywords"] = json!(k);
                }
                let got = entry["parts"].as_object().cloned().unwrap_or_default();
                let Some(body) = assemble_parts(&got, total) else { continue };
                let keywords: Vec<String> = entry["keywords"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
                parts.remove(&key);
                let info = pending.remove(&key).unwrap_or_default();
                let from = info["from"].as_str().unwrap_or("").to_string();
                let outcome = if body_hash(body.as_bytes()) != hash {
                    json!({"status": "failed", "detail": "the downloaded content does not match the announced hash (altered or truncated) — rejected"})
                } else if item_type == "methodology" {
                    match import_methodology(store, dir, name, hash, &body, &keywords, &from).await {
                        Ok(Imported::Have) => json!({"status": "have", "detail": "the same content is already in this store"}),
                        Ok(Imported::Created { id, name }) => json!({"status": "imported", "local_id": id, "local_name": name, "detail": "imported through the normal create path; its keywords are pending validation"}),
                        Err(e) => json!({"status": "failed", "detail": e}),
                    }
                } else {
                    match validate_jurisdiction_content(&body) {
                        Ok(n) => {
                            let rel = format!("peer_staging/jurisdiction/{}-{}.json", slug(name), hash.chars().take(8).collect::<String>());
                            let _ = std::fs::create_dir_all(dir.join("peer_staging/jurisdiction"));
                            match std::fs::write(dir.join(&rel), &body) {
                                Ok(_) => json!({"status": "staged", "path": rel, "detail": format!("validated ({n} rule(s), disclaimer present); STAGED only — not registered, not active; activating a jurisdiction rule set is a human step")}),
                                Err(e) => json!({"status": "failed", "detail": e.to_string()}),
                            }
                        }
                        Err(e) => json!({"status": "rejected", "detail": e}),
                    }
                };
                let mut o = outcome;
                o["at"] = json!(now_secs());
                done.insert(key, o);
            }
        }
    }
    write_map(&done_path, &done);
    write_map(&pending_path, &pending);
    write_map(&parts_path, &parts);
    if advanced != offset {
        let _ = std::fs::write(&offset_path, advanced.to_string());
    }
    Ok(())
}

/// The download loop (spawned only when `marketplace_accept_offers` is on — or the deprecated alias `auto_broadcast_sync`). No model call; polls every 30 s.
pub fn spawn_peer_import_loop(network: Arc<tokio::sync::RwLock<NetworkManager>>, store: Arc<dyn StoreAccess>, data_dir: String) {
    tokio::spawn(async move {
        let dir = PathBuf::from(&data_dir);
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
        tick.tick().await;
        loop {
            tick.tick().await;
            if let Err(e) = import_pass(&network, &store, &dir).await {
                tracing::warn!(error = %e, "peer import pass failed");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    fn offer(item_type: &str, name: &str, hash: &str, kws: &[&str]) -> RawOffer {
        RawOffer {
            key: format!("p|{item_type}|{name}|{hash}"), from: "p".into(), item_type: item_type.into(), name: name.into(), hash: hash.into(),
            keywords: kws.iter().map(|k| k.to_string()).collect(), announced_at: None, received_at: None,
        }
    }

    #[test]
    fn classify_offer_labels_every_kind_and_never_claims_identical_without_a_hash() {
        let mut idx = LocalOfferIndex::default();
        idx.tool_names.insert("document-mcp".into());
        idx.pipeline_names.insert("textanalysis".into());
        idx.jurisdiction.insert("mk".into(), "aaaaaaaaaaaaaaaa".into());
        let none = std::collections::HashMap::new();
        assert_eq!(classify_offer(&offer("tool", "Document-MCP", "", &[]), &none, &idx), "same_name");
        assert_eq!(classify_offer(&offer("tool", "mesh-mcp", "", &[]), &none, &idx), "new");
        assert_eq!(classify_offer(&offer("pipeline", "TextAnalysis", "", &[]), &none, &idx), "same_name");
        assert_eq!(classify_offer(&offer("pipeline", "Voice2", "", &[]), &none, &idx), "new");
        assert_eq!(classify_offer(&offer("jurisdiction_rule_set", "Jurisdiction: mk", "aaaaaaaaaaaaaaaa", &[]), &none, &idx), "have");
        assert_eq!(classify_offer(&offer("jurisdiction_rule_set", "Jurisdiction: mk", "bbbbbbbbbbbbbbbb", &[]), &none, &idx), "conflict");
        assert_eq!(classify_offer(&offer("jurisdiction_rule_set", "Jurisdiction: mk", "", &[]), &none, &idx), "same_name");
        assert_eq!(classify_offer(&offer("jurisdiction_rule_set", "Jurisdiction: ie", "", &[]), &none, &idx), "new");
        assert_eq!(classify_offer(&offer("blueprint", "x", "", &[]), &none, &idx), "not_checked");
    }

    #[test]
    fn matches_wanted_is_deterministic_advisory_and_only_for_tools_and_pipelines() {
        let wanted = vec!["code_callgraph".to_string(), "threed-mcp".to_string()];
        assert_eq!(matches_wanted(&offer("tool", "Code Callgraph", "", &[]), &wanted).as_deref(), Some("code_callgraph"));
        assert_eq!(matches_wanted(&offer("tool", "mesh", "", &["threed-mcp"]), &wanted).as_deref(), Some("threed-mcp"));
        assert_eq!(matches_wanted(&offer("tool", "unrelated", "", &["other"]), &wanted), None);
        assert_eq!(matches_wanted(&offer("methodology", "code callgraph", "", &[]), &wanted), None, "a methodology offer is never matched to a tool gap");
    }

    use super::*;

    #[test]
    fn offer_keys_parse_from_the_ends_so_a_name_may_contain_a_pipe() {
        let (from, ty, name, hash) = parse_offer_key("12D3KooWabc|methodology|Plan A | B|00ff00ff00ff00ff").unwrap();
        assert_eq!((from.as_str(), ty.as_str(), name.as_str(), hash.as_str()), ("12D3KooWabc", "methodology", "Plan A | B", "00ff00ff00ff00ff"));
        assert!(parse_offer_key("too|short").is_none());
    }

    #[test]
    fn an_import_never_overwrites_it_picks_a_distinct_name() {
        let mut existing: HashSet<String> = HashSet::new();
        assert_eq!(unique_import_name("API Design", &existing, "12D3KooWxyz123456"), "API Design");
        existing.insert("api design".to_string());
        let n = unique_import_name("API Design", &existing, "12D3KooWxyz123456");
        assert_eq!(n, "API Design (from peer 123456)");
        existing.insert(n.to_lowercase());
        assert_eq!(unique_import_name("API Design", &existing, "12D3KooWxyz123456"), "API Design (from peer 123456 #2)");
    }

    #[test]
    fn a_fetched_body_reassembles_exactly_and_only_when_every_part_is_present() {
        let body = "héllo — ünïcode ✓ ".repeat(300);
        let chunks = crate::network::split_chunks(&body, 200);
        let mut parts = Map::new();
        for (i, c) in chunks.iter().enumerate().skip(1) {
            parts.insert(i.to_string(), json!(c));
        }
        assert!(assemble_parts(&parts, chunks.len()).is_none(), "part 0 is missing");
        parts.insert("0".to_string(), json!(chunks[0]));
        let got = assemble_parts(&parts, chunks.len()).unwrap();
        assert_eq!(got, body);
        assert_eq!(body_hash(got.as_bytes()), body_hash(body.as_bytes()), "identity survives the transfer");
    }

    #[test]
    fn validation_rejects_what_must_not_be_imported() {
        assert!(validate_methodology_body("{\"name\":\"x\"}").is_ok());
        assert!(validate_methodology_body("[1,2]").is_err() && validate_methodology_body("not json").is_err());
        assert!(validate_jurisdiction_content("{\"disclaimer\":\"\",\"rules\":[]}").is_err(), "an empty disclaimer is refused");
        assert!(validate_jurisdiction_content("{\"disclaimer\":\"engineering starting point\",\"rules\":[]}").is_err(), "no rules is refused");
        assert!(validate_jurisdiction_content("{\"rules\":[]}").is_err(), "a missing disclaimer fails to parse");
    }

    #[test]
    fn slugs_are_filesystem_safe() {
        assert_eq!(slug("VPS Secure Deployment — hardened (EU)"), "vps-secure-deployment-hardened-eu");
        assert_eq!(slug("///"), "item");
    }

    #[test]
    fn collected_offers_come_from_announce_and_catalog_rows_latest_wins_and_never_a_blueprint() {
        let dir = std::env::temp_dir().join(format!("ozone-offers-{}", now_secs()));
        std::fs::create_dir_all(&dir).unwrap();
        let rows = [
            json!({"received_at": 1, "from_peer": "P1", "envelope": {"kind":"announce","item_type":"methodology","name":"A","body_hash":"00ff00ff00ff00ff","keywords":["x"]}}),
            json!({"received_at": 2, "from_peer": "P1", "envelope": {"kind":"catalog","items":[
                {"item_type":"methodology","name":"A","body_hash":"00ff00ff00ff00ff","keywords":["x","y"]},
                {"item_type":"blueprint","name":"B","body_hash":"00ff00ff00ff00ff"}]}}),
        ];
        std::fs::write(dir.join("peer_marketplace_inbox.jsonl"), rows.iter().map(|r| r.to_string()).collect::<Vec<_>>().join("\n") + "\n").unwrap();
        let offers = collect_offers(&dir);
        assert_eq!(offers.len(), 1, "the blueprint is never an offer; the duplicate collapses");
        assert_eq!(offers[0].keywords, vec!["x", "y"], "the latest row wins");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
