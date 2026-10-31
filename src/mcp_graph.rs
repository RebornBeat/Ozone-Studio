//! MCP graph persistence: a tool output that carries a `graph` block becomes
//! real ZSEI containers. Containment is the native parent/child link
//! (`parent_id` / `child_ids`); typed relations (Above, NearTo, InFrontOf,
//! Overlaps, ...) are `Relation`s in `context.relationships`, which Structural
//! traversal follows. Entity attributes sit behind `object_store_path`, the
//! same content-pointer pattern context_mirror uses.
//!
//! Block shape (all keys validated here, never trusted):
//!   { "nodes": [{"key", "kind", "label", "parent"?, "attributes"?}],
//!     "edges": [{"from", "to", "relation", "confidence"?}] }
//!
//! Allowed relations: Contains, PartOf, RelatedTo, References, ImportsFrom,
//! CallsTo, and the spatial set Above, Below, NearTo, InFrontOf, Overlaps,
//! LeftOf, RightOf, Within, Encloses (geometric containment is Encloses/Within;
//! `Contains` stays the structural hierarchy relation).
//!
//! GEOMETRY (shared spatial layer, docs/SPATIAL_GEOMETRY_LAYER.md): a node whose
//! `attributes.geom` is `{"frame": str, "kind": "box", "x","y","w","h"}` gets its
//! spatial relations computed HERE, centrally, by `compute_spatial_relations`
//! (same-frame boxes only; bounded by MAX_EDGES; truncation is recorded in the
//! summary note). Tools emit geometry, not relations.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::types::container::{
    CompressionType, Container, ContainerType, Context, DiscoveryMethod, GlobalState,
    IntegrityData, LocalState, Metadata, Modality, Relation, RelationType, StoragePointers,
    TraversalHints,
};
use crate::types::zsei::{ContainerUpdate, ZSEIQuery, ZSEIQueryResult};
use crate::zsei::ZSEI;

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphSummary {
    pub root_id: u64,
    pub entities: usize,
    pub relations: usize,
    /// Honest record of any bounding applied to untrusted tool output
    /// (oversized label / attributes shortened). `None` = nothing was cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncation_note: Option<String>,
}

/// Bounds on UNTRUSTED tool output. A graph over either count cap is
/// REFUSED with an explicit error (never partially persisted and never
/// silently trimmed): trimming nodes would orphan their parents/edges.
const MAX_NODES: usize = 5_000;
const MAX_EDGES: usize = 20_000;
/// Per-field bounds: shortened with a recorded note, never silently.
const MAX_LABEL_CHARS: usize = 200;
const MAX_KIND_CHARS: usize = 40;
const MAX_ATTR_BYTES: usize = 64 * 1024;

/// Keywords that other subsystems react to. A tool-supplied `kind` must never
/// be able to forge one (it becomes a container keyword, and graph events
/// carry keywords as scope): `finding` / `tool_call` / `config_change` feed
/// the ripple actors; `proj:<id>` / `file:<id>` feed the AMT ripple sync.
/// ':' is stripped from kinds entirely, which also kills the scoped forms.
const RESERVED_KEYWORDS: [&str; 3] = ["finding", "tool_call", "config_change"];

/// Char-boundary-safe prefix.
fn take_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// A tool-supplied `kind` (or tool name) made safe to use as a keyword.
fn sanitize_keyword(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .take(MAX_KIND_CHARS)
        .collect();
    if cleaned.is_empty() || RESERVED_KEYWORDS.contains(&cleaned.to_ascii_lowercase().as_str()) {
        "entity".to_string()
    } else {
        cleaned
    }
}

pub async fn persist_from_output(
    zsei: &ZSEI,
    data_dir: &str,
    tool: &str,
    agent: &str,
    output: &Value,
) -> Option<Result<GraphSummary, String>> {
    persist_from_output_scoped(zsei, data_dir, tool, agent, output, None, None).await
}

/// UNCOMPILED pending the build gate. Same as `persist_from_output`, but the
/// created containers also carry the host-supplied scope keywords `ws:<id>` /
/// `proj:<id>` (the same form graph_events scoping uses). Scope comes from the
/// authenticated caller, never from the untrusted graph block: `kind` values
/// still have ':' stripped, so a tool cannot forge a scope keyword.
pub async fn persist_from_output_scoped(
    zsei: &ZSEI,
    data_dir: &str,
    tool: &str,
    agent: &str,
    output: &Value,
    workspace_id: Option<u64>,
    project_id: Option<u64>,
) -> Option<Result<GraphSummary, String>> {
    let graph = output
        .get("graph")
        .or_else(|| output.get("output").and_then(|o| o.get("graph")))?;
    let mut scope: Vec<String> = Vec::new();
    if let Some(w) = workspace_id {
        scope.push(format!("ws:{w}"));
    }
    if let Some(p) = project_id {
        scope.push(format!("proj:{p}"));
    }
    Some(persist(zsei, data_dir, tool, agent, graph, &scope).await)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 80
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn relation_from_name(name: &str) -> Option<RelationType> {
    Some(match name {
        "Contains" => RelationType::Contains,
        "PartOf" => RelationType::PartOf,
        "RelatedTo" => RelationType::RelatedTo,
        "References" => RelationType::References,
        "ImportsFrom" => RelationType::ImportsFrom,
        "CallsTo" => RelationType::CallsTo,
        "Above" => RelationType::Above,
        "Below" => RelationType::Below,
        "NearTo" => RelationType::NearTo,
        "InFrontOf" => RelationType::InFrontOf,
        "Overlaps" => RelationType::Overlaps,
        // UNCOMPILED pending the build gate (shared geometry layer):
        "LeftOf" => RelationType::LeftOf,
        "RightOf" => RelationType::RightOf,
        "Within" => RelationType::Within,
        "Encloses" => RelationType::Encloses,
        "Touches" => RelationType::Touches,
        "Behind" => RelationType::Behind,
        "Aligned" => RelationType::Aligned,
        "Supports" => RelationType::Supports,
        _ => return None,
    })
}

fn modality_for(tool: &str) -> Modality {
    // G5 partial (2026-10-31): prefix affinity for the registered families.
    // The designed successor reads the tool's `modality:<name>` capability
    // tag from the registry (D18) — this table is the honest interim.
    let image = ["visual_", "yolo_", "depth_", "shape_", "pose_", "screenshot_", "ocr_"];
    let text = ["web_", "search", "text_", "translate_", "summarize_"];
    let code = ["code_", "git_", "repo_", "ast_"];
    let exec = ["terminal_", "shell_", "process_"];
    let coord = ["context_", "note_", "task_", "file_claim", "handoff"];
    let dom = ["domorpher_", "uidocument_", "page_"];
    let (tag, fallback) = if image.iter().any(|p| tool.starts_with(p)) {
        (Some(Modality::Image), "image")
    } else if text.iter().any(|p| tool.starts_with(p)) {
        (Some(Modality::Text), "text")
    } else if code.iter().any(|p| tool.starts_with(p)) {
        (Some(Modality::Code), "code")
    } else if exec.iter().any(|p| tool.starts_with(p)) {
        (None, "execution")
    } else if coord.iter().any(|p| tool.starts_with(p)) {
        (None, "coordination")
    } else if dom.iter().any(|p| tool.starts_with(p)) {
        (Some(Modality::UiDocument), "uidocument")
    } else {
        (None, "unknown")
    };
    let _ = fallback;
    tag.unwrap_or(Modality::Unknown)
}

fn build_container(
    container_type: ContainerType,
    modality: Modality,
    name: &str,
    path: String,
    keywords: Vec<String>,
    agent: &str,
    object_store_path: Option<String>,
    parent_id: u64,
) -> Container {
    let now = now_secs();
    Container {
        global_state: GlobalState {
            container_id: 0,
            parent_id,
            child_ids: vec![],
            child_count: 0,
            version: 1,
        },
        local_state: LocalState {
            metadata: Metadata {
                container_type,
                modality,
                created_at: now,
                updated_at: now,
                provenance: agent.to_string(),
                permissions: 0,
                owner_id: 0,
                name: Some(name.to_string()),
                materialized_path: Some(path),
            },
            context: Context {
                categories: vec![],
                methodologies: vec![],
                keywords,
                topics: vec!["mcp".to_string()],
                relationships: vec![],
                learned_associations: vec![],
                embedding: None,
            },
            storage: StoragePointers {
                db_shard_id: None,
                vector_index_ref: None,
                object_store_path,
                compression_type: CompressionType::None,
            },
            hints: TraversalHints::default(),
            integrity: IntegrityData::default(),
            file_context: None,
            code_context: None,
            text_context: None,
            external_ref: None,
        },
    }
}

async fn create(zsei: &ZSEI, parent_id: u64, container: Container) -> Result<u64, String> {
    match zsei
        .query(ZSEIQuery::CreateContainer { parent_id, container })
        .await
    {
        Ok(ZSEIQueryResult::ContainerID(id)) => Ok(id),
        Ok(_) => Err("unexpected CreateContainer result".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

async fn persist(
    zsei: &ZSEI,
    data_dir: &str,
    tool: &str,
    agent: &str,
    graph: &Value,
    scope_keywords: &[String],
) -> Result<GraphSummary, String> {
    let nodes = graph
        .get("nodes")
        .and_then(|n| n.as_array())
        .ok_or("graph.nodes must be an array")?;
    let mut edges = graph
        .get("edges")
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default();
    if nodes.is_empty() {
        return Err("graph has no nodes".to_string());
    }
    if nodes.len() > MAX_NODES {
        return Err(format!(
            "graph has {} nodes; the limit for untrusted tool output is {MAX_NODES} (refused, nothing persisted)",
            nodes.len()
        ));
    }
    if edges.len() > MAX_EDGES {
        return Err(format!(
            "graph has {} edges; the limit for untrusted tool output is {MAX_EDGES} (refused, nothing persisted)",
            edges.len()
        ));
    }
    let mut notes: Vec<String> = Vec::new();

    let mut keys: HashMap<&str, Option<&str>> = HashMap::new();
    for node in nodes {
        let key = node
            .get("key")
            .and_then(|k| k.as_str())
            .ok_or("every node needs a string key")?;
        if !valid_key(key) {
            return Err(format!("node key '{key}' must be [A-Za-z0-9_-], max 80 chars"));
        }
        let parent = node.get("parent").and_then(|p| p.as_str());
        if keys.insert(key, parent).is_some() {
            return Err(format!("duplicate node key '{key}'"));
        }
    }
    for (key, parent) in &keys {
        if let Some(pk) = *parent {
            if !keys.contains_key(pk) {
                return Err(format!("node '{key}' has unknown parent '{pk}'"));
            }
        }
        let mut cur: Option<&str> = *parent;
        let mut steps = 0usize;
        while let Some(p) = cur {
            steps += 1;
            if steps > keys.len() {
                return Err(format!("parent cycle through node '{key}'"));
            }
            cur = keys.get(p).copied().flatten();
        }
    }
    // UNCOMPILED pending the build gate. Central spatial relations: nodes that
    // carry `attributes.geom` get Overlaps/Encloses/Within/NearTo/Above/Below/
    // LeftOf/RightOf computed from their geometry. Runs after key validation
    // (keys are unique and valid) and before edge validation (generated edges
    // are validated like tool-supplied ones). Total edges stay <= MAX_EDGES.
    {
        let room = MAX_EDGES.saturating_sub(edges.len());
        let (spatial, spatial_note) = compute_spatial_relations(nodes, room);
        if !spatial.is_empty() {
            edges.extend(spatial);
        }
        if let Some(n) = spatial_note {
            notes.push(n);
        }
    }
    for edge in &edges {
        let from = edge.get("from").and_then(|v| v.as_str()).unwrap_or("");
        let to = edge.get("to").and_then(|v| v.as_str()).unwrap_or("");
        let name = edge.get("relation").and_then(|v| v.as_str()).unwrap_or("");
        if !keys.contains_key(from) {
            return Err(format!("edge source '{from}' is not a node"));
        }
        if !keys.contains_key(to) {
            return Err(format!("edge target '{to}' is not a node"));
        }
        if relation_from_name(name).is_none() {
            // PER-EDGE REJECTION (D9, 2026-10-31): an unknown relation name
            // skips that edge (recorded in notes) — it never rejects the
            // whole graph block.
            notes.push(format!(
                "edge skipped: unknown relation '{name}' ({from} -> {to})"
            ));
            continue;
        }
    }

    let root_container = build_container(
        ContainerType::McpResult,
        modality_for(tool),
        &format!("{tool} result"),
        format!("/mcp/{tool}"),
        {
            let mut kw = vec![sanitize_keyword(tool), "mcp_result".to_string()];
            kw.extend(scope_keywords.iter().cloned());
            kw
        },
        agent,
        None,
        zsei.root_id(),
    );
    let root_id = create(zsei, zsei.root_id(), root_container).await?;
    let dir = format!("{}/mcp_graph/{}", data_dir, root_id);
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("create graph dir: {e}"))?;

    let mut key_to_id: HashMap<String, u64> = HashMap::new();
    let mut pending: Vec<&Value> = nodes.iter().collect();
    while !pending.is_empty() {
        let batch = std::mem::take(&mut pending);
        let before = batch.len();
        for node in batch {
            let key = node
                .get("key")
                .and_then(|k| k.as_str())
                .ok_or("every node needs a string key")?;
            if !valid_key(key) {
                return Err(format!("node key '{key}' must be [A-Za-z0-9_-], max 80 chars"));
            }
            let raw_kind = node.get("kind").and_then(|k| k.as_str()).unwrap_or("entity");
            let kind = sanitize_keyword(raw_kind);
            if kind != raw_kind {
                notes.push(format!("node '{key}': kind '{}' normalized to '{kind}'", take_chars(raw_kind, 60)));
            }
            let raw_label = node.get("label").and_then(|l| l.as_str()).unwrap_or(key);
            let label_owned: String = if raw_label.chars().count() > MAX_LABEL_CHARS {
                notes.push(format!("node '{key}': label shortened to {MAX_LABEL_CHARS} chars"));
                take_chars(raw_label, MAX_LABEL_CHARS)
            } else {
                raw_label.to_string()
            };
            let label: &str = &label_owned;
            let attributes_value = {
                let raw = node.get("attributes").cloned().unwrap_or(Value::Null);
                let size = serde_json::to_vec(&raw).map(|v| v.len()).unwrap_or(0);
                if size > MAX_ATTR_BYTES {
                    notes.push(format!("node '{key}': attributes ({size} bytes) replaced by a bounded preview"));
                    let text = serde_json::to_string(&raw).unwrap_or_default();
                    serde_json::json!({
                        "truncated": true,
                        "original_bytes": size,
                        "preview": take_chars(&text, 4_000),
                    })
                } else {
                    raw
                }
            };
            let parent_id = match node.get("parent").and_then(|p| p.as_str()) {
                None => root_id,
                Some(pk) => match key_to_id.get(pk) {
                    Some(id) => *id,
                    None => {
                        pending.push(node);
                        continue;
                    }
                },
            };

            let rel_path = format!("mcp_graph/{}/{}.json", root_id, key);
            let attrs = serde_json::json!({
                "key": key,
                "kind": kind,
                "label": label,
                "tool": tool,
                "attributes": attributes_value,
            });
            tokio::fs::write(
                format!("{}/{}", data_dir, rel_path),
                serde_json::to_vec_pretty(&attrs).map_err(|e| e.to_string())?,
            )
            .await
            .map_err(|e| format!("write entity content {key}: {e}"))?;

            let container = build_container(
                ContainerType::McpEntity,
                modality_for(tool),
                label,
                format!("/mcp/{tool}/{root_id}/{key}"),
                {
                    let mut kw = vec![sanitize_keyword(tool), kind.clone()];
                    kw.extend(scope_keywords.iter().cloned());
                    kw
                },
                agent,
                Some(rel_path),
                parent_id,
            );
            let id = create(zsei, parent_id, container).await?;
            key_to_id.insert(key.to_string(), id);
        }
        if pending.len() == before {
            return Err("graph has a missing parent key or a parent cycle".to_string());
        }
    }

    let mut relations = 0usize;
    for edge in &edges {
        let from = edge.get("from").and_then(|v| v.as_str()).unwrap_or("");
        let to = edge.get("to").and_then(|v| v.as_str()).unwrap_or("");
        let name = edge.get("relation").and_then(|v| v.as_str()).unwrap_or("");
        let confidence = edge
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0) as f32;
        let from_id = *key_to_id
            .get(from)
            .ok_or_else(|| format!("edge source '{from}' is not a node"))?;
        let to_id = *key_to_id
            .get(to)
            .ok_or_else(|| format!("edge target '{to}' is not a node"))?;
        let relation_type = match relation_from_name(name) {
            Some(r) => r,
            None => {
                notes.push(format!(
                    "edge skipped: unknown relation '{name}' ({from} -> {to})"
                ));
                continue;
            }
        };

        let source = zsei
            .get_container(from_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("edge source container vanished")?;
        let mut context = source.local_state.context.clone();
        let already = context
            .relationships
            .iter()
            .any(|r| r.target_id == to_id && r.relation_type == relation_type);
        if already {
            continue;
        }
        context.relationships.push(Relation {
            target_id: to_id,
            relation_type,
            confidence,
            discovered_via: DiscoveryMethod::ToolOutput,
            graph_hops: None,
        });
        zsei.query(ZSEIQuery::UpdateContainer {
            container_id: from_id,
            updates: ContainerUpdate {
                context: Some(context),
                ..Default::default()
            },
        })
        .await
        .map_err(|e| e.to_string())?;
        relations += 1;
    }

    let truncation_note = if notes.is_empty() {
        None
    } else {
        let shown = notes.iter().take(20).cloned().collect::<Vec<_>>().join("; ");
        Some(if notes.len() > 20 {
            format!("{shown}; … and {} more", notes.len() - 20)
        } else {
            shown
        })
    };
    Ok(GraphSummary {
        root_id,
        entities: key_to_id.len(),
        relations,
        truncation_note,
    })
}

// ============================================================================
// CENTRAL SPATIAL RELATIONS (shared geometry layer) — UNCOMPILED pending the
// build gate. Pure functions, no I/O. See docs/SPATIAL_GEOMETRY_LAYER.md.
// ============================================================================

/// Nearest neighbours per box that get NearTo / directional edges.
const SPATIAL_NEAR_K: usize = 4;
/// A box covering more grid cells than this is treated as "big" and compared
/// against every box instead of being indexed cell by cell.
const SPATIAL_MAX_CELLS_PER_BOX: i64 = 1024;

#[derive(Debug, Clone)]
struct GeomBox {
    key: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl GeomBox {
    fn x2(&self) -> f64 {
        self.x + self.w
    }
    fn y2(&self) -> f64 {
        self.y + self.h
    }
    /// Area of the intersection; 0.0 when the boxes only touch or are apart.
    fn overlap_area(&self, o: &GeomBox) -> f64 {
        let ox = self.x2().min(o.x2()) - self.x.max(o.x);
        let oy = self.y2().min(o.y2()) - self.y.max(o.y);
        if ox > 0.0 && oy > 0.0 {
            ox * oy
        } else {
            0.0
        }
    }
    fn encloses(&self, o: &GeomBox) -> bool {
        self.x <= o.x && self.y <= o.y && self.x2() >= o.x2() && self.y2() >= o.y2()
    }
}

/// `(frame, x, y, w, h)` from a graph node's `attributes.geom` (or a top-level
/// `geom`). Only `kind == "box"` with a frame and finite, non-negative size.
fn parse_geom(node: &Value) -> Option<(String, f64, f64, f64, f64)> {
    let g = node
        .get("attributes")
        .and_then(|a| a.get("geom"))
        .or_else(|| node.get("geom"))?;
    if g.get("kind").and_then(|k| k.as_str()).unwrap_or("box") != "box" {
        return None;
    }
    let frame = g.get("frame").and_then(|f| f.as_str())?.to_string();
    let num = |k: &str| g.get(k).and_then(|v| v.as_f64()).filter(|v| v.is_finite());
    let x = num("x")?;
    let y = num("y")?;
    let w = num("w")?;
    let h = num("h")?;
    if w < 0.0 || h < 0.0 {
        return None;
    }
    Some((frame, x, y, w, h))
}

fn inverse_relation(rel: &'static str) -> &'static str {
    match rel {
        "Above" => "Below",
        "Below" => "Above",
        "LeftOf" => "RightOf",
        "RightOf" => "LeftOf",
        "Encloses" => "Within",
        "Within" => "Encloses",
        other => other, // Overlaps / NearTo are symmetric
    }
}

#[allow(clippy::too_many_arguments)]
fn push_spatial_edge(
    out: &mut Vec<Value>,
    seen: &mut HashSet<(usize, usize, &'static str)>,
    frame: &str,
    boxes: &[GeomBox],
    i: usize,
    j: usize,
    rel: &'static str,
    confidence: f64,
) {
    if !seen.insert((i, j, rel)) {
        return;
    }
    out.push(serde_json::json!({
        "from": boxes[i].key,
        "to": boxes[j].key,
        "relation": rel,
        "confidence": confidence,
        "frame": frame,
        "derived": "spatial",
    }));
}

/// Relations for the boxes of ONE frame. Exact relations (Overlaps / Encloses /
/// Within) go to `exact`, proximity relations (NearTo + directional) to `near`.
/// Returns true when generation stopped early at the edge budget.
fn frame_relations(
    frame: &str,
    boxes: &[GeomBox],
    max_edges: usize,
    exact: &mut Vec<Value>,
    near: &mut Vec<Value>,
) -> bool {
    let n = boxes.len();
    if n < 2 {
        return false;
    }
    let avg = boxes.iter().map(|b| (b.w + b.h) / 2.0).sum::<f64>() / n as f64;
    // Uniform grid; the cell size doubles as the NearTo distance bound.
    let cell = (avg * 2.0).max(8.0);
    let cell_of = |v: f64| -> i64 { (v / cell).floor() as i64 };

    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let mut big: Vec<usize> = Vec::new();
    for (i, b) in boxes.iter().enumerate() {
        let (x0, x1, y0, y1) = (cell_of(b.x), cell_of(b.x2()), cell_of(b.y), cell_of(b.y2()));
        let span = (x1 - x0 + 1).saturating_mul(y1 - y0 + 1);
        if span > SPATIAL_MAX_CELLS_PER_BOX {
            big.push(i);
            continue;
        }
        for cx in x0..=x1 {
            for cy in y0..=y1 {
                grid.entry((cx, cy)).or_default().push(i);
            }
        }
    }
    let big_set: HashSet<usize> = big.iter().copied().collect();

    let mut seen: HashSet<(usize, usize, &'static str)> = HashSet::new();
    for i in 0..n {
        let a = &boxes[i];
        let mut cand: Vec<usize> = Vec::new();
        if big_set.contains(&i) {
            cand.extend(0..n);
        } else {
            for cx in cell_of(a.x - cell)..=cell_of(a.x2() + cell) {
                for cy in cell_of(a.y - cell)..=cell_of(a.y2() + cell) {
                    if let Some(v) = grid.get(&(cx, cy)) {
                        cand.extend(v.iter().copied());
                    }
                }
            }
            cand.extend(big.iter().copied());
        }
        cand.sort_unstable();
        cand.dedup();

        let mut nearby: Vec<(f64, usize)> = Vec::new();
        for &j in &cand {
            if j == i {
                continue;
            }
            let b = &boxes[j];
            if a.overlap_area(b) > 0.0 {
                if j > i {
                    // exact relation, once per unordered pair
                    let (r_ab, r_ba) = if a.encloses(b) && b.encloses(a) {
                        ("Overlaps", "Overlaps")
                    } else if a.encloses(b) {
                        ("Encloses", "Within")
                    } else if b.encloses(a) {
                        ("Within", "Encloses")
                    } else {
                        ("Overlaps", "Overlaps")
                    };
                    push_spatial_edge(exact, &mut seen, frame, boxes, i, j, r_ab, 1.0);
                    push_spatial_edge(exact, &mut seen, frame, boxes, j, i, r_ba, 1.0);
                    if exact.len() >= max_edges {
                        return true;
                    }
                }
                continue;
            }
            let gx = (a.x.max(b.x) - a.x2().min(b.x2())).max(0.0);
            let gy = (a.y.max(b.y) - a.y2().min(b.y2())).max(0.0);
            let dist = gx.hypot(gy);
            if dist <= cell {
                nearby.push((dist, j));
            }
        }
        nearby.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
        for &(dist, j) in nearby.iter().take(SPATIAL_NEAR_K) {
            let b = &boxes[j];
            let conf = (1.0 - dist / cell).clamp(0.1, 1.0);
            let gx = (a.x.max(b.x) - a.x2().min(b.x2())).max(0.0);
            let gy = (a.y.max(b.y) - a.y2().min(b.y2())).max(0.0);
            let mut rels: Vec<&'static str> = vec!["NearTo"];
            if gy > 0.0 && gx == 0.0 {
                rels.push(if a.y2() <= b.y { "Above" } else { "Below" });
            } else if gx > 0.0 && gy == 0.0 {
                rels.push(if a.x2() <= b.x { "LeftOf" } else { "RightOf" });
            }
            for rel in rels {
                push_spatial_edge(near, &mut seen, frame, boxes, i, j, rel, conf);
                push_spatial_edge(near, &mut seen, frame, boxes, j, i, inverse_relation(rel), conf);
            }
        }
    }
    false
}

/// Spatial relations for every pair of graph nodes whose `attributes.geom` is a
/// box in the SAME frame (cross-frame needs a frame transform; not handled
/// here). Nodes without geometry, with a non-box geometry, or with a degenerate
/// (zero-area) box produce no spatial edges. Bounded: at most `max_edges` edges
/// are returned; exact relations (Overlaps/Encloses/Within) take priority over
/// proximity relations (NearTo/Above/Below/LeftOf/RightOf, nearest-4 per box,
/// both directions so traversal works from either end). The returned note is
/// `Some` iff anything was dropped to honour the cap.
pub fn compute_spatial_relations(nodes: &[Value], max_edges: usize) -> (Vec<Value>, Option<String>) {
    let mut by_frame: HashMap<String, Vec<GeomBox>> = HashMap::new();
    for node in nodes {
        let key = match node.get("key").and_then(|k| k.as_str()) {
            Some(k) => k,
            None => continue,
        };
        if let Some((frame, x, y, w, h)) = parse_geom(node) {
            if w > 0.0 && h > 0.0 {
                by_frame.entry(frame).or_default().push(GeomBox {
                    key: key.to_string(),
                    x,
                    y,
                    w,
                    h,
                });
            }
        }
    }
    let mut frames: Vec<&String> = by_frame.keys().collect();
    frames.sort();

    let mut exact: Vec<Value> = Vec::new();
    let mut near: Vec<Value> = Vec::new();
    let mut truncated = false;
    for frame in frames {
        if let Some(boxes) = by_frame.get(frame) {
            if frame_relations(frame, boxes, max_edges, &mut exact, &mut near) {
                truncated = true;
            }
        }
        if exact.len() >= max_edges {
            truncated = true;
            break;
        }
    }
    let total = exact.len() + near.len();
    let mut out = exact;
    if out.len() > max_edges {
        out.truncate(max_edges);
    }
    let room = max_edges.saturating_sub(out.len());
    if near.len() > room {
        truncated = true;
        near.truncate(room);
    }
    out.extend(near);
    let note = if truncated {
        Some(format!(
            "spatial relations truncated: {} computed (at least), {} kept (edge cap {})",
            total,
            out.len(),
            max_edges
        ))
    } else {
        None
    };
    (out, note)
}

#[cfg(test)]
mod spatial_tests {
    use super::*;
    use serde_json::json;

    fn node(key: &str, frame: &str, x: f64, y: f64, w: f64, h: f64) -> Value {
        json!({"key": key, "kind": "element", "label": key,
               "attributes": {"geom": {"frame": frame, "kind": "box", "x": x, "y": y, "w": w, "h": h}}})
    }

    fn has(edges: &[Value], from: &str, to: &str, rel: &str) -> bool {
        edges.iter().any(|e| {
            e["from"] == from && e["to"] == to && e["relation"] == rel
        })
    }

    #[test]
    fn partial_overlap_is_symmetric() {
        let nodes = vec![node("a", "page", 0.0, 0.0, 100.0, 100.0), node("b", "page", 50.0, 50.0, 100.0, 100.0)];
        let (e, note) = compute_spatial_relations(&nodes, 1000);
        assert!(note.is_none());
        assert!(has(&e, "a", "b", "Overlaps") && has(&e, "b", "a", "Overlaps"));
    }

    #[test]
    fn containment_is_encloses_and_within_not_contains() {
        let nodes = vec![node("outer", "page", 0.0, 0.0, 200.0, 200.0), node("inner", "page", 10.0, 10.0, 50.0, 50.0)];
        let (e, _) = compute_spatial_relations(&nodes, 1000);
        assert!(has(&e, "outer", "inner", "Encloses"));
        assert!(has(&e, "inner", "outer", "Within"));
        assert!(!e.iter().any(|x| x["relation"] == "Contains"));
    }

    #[test]
    fn stacked_boxes_are_above_below_and_near() {
        let nodes = vec![node("top", "page", 0.0, 0.0, 100.0, 20.0), node("bottom", "page", 0.0, 30.0, 100.0, 20.0)];
        let (e, _) = compute_spatial_relations(&nodes, 1000);
        assert!(has(&e, "top", "bottom", "Above"));
        assert!(has(&e, "bottom", "top", "Below"));
        assert!(has(&e, "top", "bottom", "NearTo"));
    }

    #[test]
    fn side_by_side_boxes_are_left_right() {
        let nodes = vec![node("l", "page", 0.0, 0.0, 20.0, 50.0), node("r", "page", 30.0, 0.0, 20.0, 50.0)];
        let (e, _) = compute_spatial_relations(&nodes, 1000);
        assert!(has(&e, "l", "r", "LeftOf"));
        assert!(has(&e, "r", "l", "RightOf"));
    }

    #[test]
    fn different_frames_never_relate() {
        let nodes = vec![node("a", "page", 0.0, 0.0, 100.0, 100.0), node("b", "screenshot", 0.0, 0.0, 100.0, 100.0)];
        let (e, _) = compute_spatial_relations(&nodes, 1000);
        assert!(e.is_empty());
    }

    #[test]
    fn missing_or_degenerate_geometry_yields_nothing() {
        let nodes = vec![
            json!({"key": "nogeom", "kind": "element"}),
            node("zero", "page", 0.0, 0.0, 0.0, 10.0),
            node("ok", "page", 0.0, 0.0, 10.0, 10.0),
        ];
        let (e, _) = compute_spatial_relations(&nodes, 1000);
        assert!(e.is_empty());
    }

    #[test]
    fn far_apart_boxes_are_not_related() {
        let nodes = vec![node("a", "page", 0.0, 0.0, 10.0, 10.0), node("b", "page", 5000.0, 5000.0, 10.0, 10.0)];
        let (e, _) = compute_spatial_relations(&nodes, 1000);
        assert!(e.is_empty());
    }

    #[test]
    fn edge_cap_is_honoured_and_recorded() {
        let mut nodes = Vec::new();
        for i in 0..30 {
            nodes.push(node(&format!("n{i}"), "page", 0.0, 0.0, 100.0, 100.0)); // all overlap each other
        }
        let (e, note) = compute_spatial_relations(&nodes, 50);
        assert!(e.len() <= 50);
        assert!(note.is_some());
    }

    #[test]
    fn relation_names_are_all_accepted_by_the_graph_block() {
        for name in ["Overlaps", "Encloses", "Within", "Above", "Below", "LeftOf", "RightOf", "NearTo"] {
            assert!(relation_from_name(name).is_some(), "{name} must be an allowed relation");
        }
    }
}
