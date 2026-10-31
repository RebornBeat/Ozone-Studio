//! Graph assembly in the exact shape src/mcp_graph.rs accepts:
//!   nodes: {"key","kind","label","parent"?,"attributes"?}
//!   edges: {"from","to","relation","confidence"?}
//! (plus `id` = key, and provenance fields the host ignores). The host REFUSES a
//! graph over 5,000 nodes / 20,000 edges or with an unknown relation name, so
//! this module truncates itself (recorded in `bounds`) and maps relation names
//! unless `relation_mode` is "semantic" (use that once the host accepts the
//! UiDocument relation names).
//!
//! Spatial relations are NOT produced here. Nodes carry `geom` + `frame`
//! (pass-through of what a live feed supplied); the host computes relations.

use std::collections::{BTreeMap, HashSet};

use serde_json::{json, Map, Value};

use crate::html::Dom;
use crate::rules::Facts;

pub const MAX_NODES: usize = 5_000;
pub const MAX_EDGES: usize = 20_000;
const MAX_LABEL: usize = 200;
const MAX_ATTR_VALUE: usize = 500;

/// Relation names the host (mcp_graph::relation_from_name) accepts today.
const HOST_RELATIONS: &[&str] = &[
    "Contains", "PartOf", "RelatedTo", "References", "ImportsFrom", "CallsTo", "Above", "Below", "NearTo", "InFrontOf", "Overlaps",
];

/// Attributes copied onto the node (everything else, notably style / on* / data-*, is dropped).
const KEPT_ATTRS: &[&str] = &[
    "id", "class", "name", "type", "href", "src", "role", "alt", "title", "placeholder", "for", "value", "action", "method",
    "rel", "target", "lang", "content", "charset", "tabindex", "disabled", "required", "checked", "selected", "hidden",
    "contenteditable", "autocomplete", "maxlength", "min", "max", "pattern", "form", "aria-label", "aria-labelledby",
];

/// Extra attributes copied onto SVG nodes (flavor svg): the geometry-bearing ones.
const SVG_KEPT_ATTRS: &[&str] = &[
    "x", "y", "width", "height", "viewbox", "preserveaspectratio", "d", "points", "cx", "cy", "r", "rx", "ry", "x1", "y1", "x2", "y2",
    "transform", "fill", "stroke", "stroke-width", "opacity", "display", "visibility", "font-size", "text-anchor",
];

/// Node kind for SVG elements (flavor svg); None falls back to the html kinds.
fn svg_kind(tag: &str) -> Option<&'static str> {
    match tag {
        "svg" => Some("Graphic"),
        "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" | "path" => Some("Shape"),
        "text" | "tspan" => Some("TextRun"),
        "g" | "defs" | "symbol" | "switch" => Some("Group"),
        "use" => Some("Reference"),
        "image" => Some("Media"),
        "foreignobject" => Some("Region"),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RelationMode {
    Host,
    Semantic,
}

pub struct Built {
    pub graph: Value,
    /// Keys of the emitted nodes (document order).
    pub emitted: Vec<String>,
    /// Edges that touch a context-only node (live deltas): not part of `graph`,
    /// returned so a stateful consumer can attach them to earlier nodes.
    pub external_edges: Vec<Value>,
}

fn clip(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn conf_number(class: &str) -> f64 {
    match class {
        "certain" => 1.0,
        "derived" => 0.8,
        _ => 0.5,
    }
}

fn host_relation(name: &str) -> Option<String> {
    match name {
        "ChildOf" => None, // native containment: the node's `parent`
        "LabelFor" | "FormOwns" | "Controls" => Some("RelatedTo".to_string()),
        "LinksTo" | "Embeds" | "Includes" | "StyledBy" => Some("References".to_string()),
        other if HOST_RELATIONS.contains(&other) => Some(other.to_string()),
        _ => Some("RelatedTo".to_string()),
    }
}

fn kind_for(tag: &str, facts: &Facts, i: usize) -> &'static str {
    match tag {
        "form" => "Form",
        "input" | "select" | "textarea" => "Field",
        "a" | "area" => {
            if facts.flag(i, "href").is_some() {
                "Link"
            } else {
                "Element"
            }
        }
        "img" | "picture" | "video" | "audio" | "canvas" | "svg" => "Media",
        "iframe" | "frame" | "embed" | "object" => "Frame",
        _ => {
            if facts.flag(i, "landmark").is_some() {
                "Region"
            } else {
                "Element"
            }
        }
    }
}

fn flags_json(facts: &Facts, i: usize) -> (Value, Value) {
    let mut flags = Map::new();
    let mut prov = Map::new();
    if let Some(nf) = facts.nodes.get(i) {
        for (name, f) in &nf.flags {
            flags.insert(name.clone(), f.value.clone());
            prov.insert(name.clone(), json!({"rule": f.rule, "confidence": f.confidence}));
        }
    }
    (Value::Object(flags), Value::Object(prov))
}

fn label_for_element(dom: &Dom, facts: &Facts, i: usize) -> String {
    let n = &dom.nodes[i];
    let snippet = facts
        .flag_str(i, "name")
        .or_else(|| if n.text.is_empty() { None } else { Some(n.text.clone()) })
        .or_else(|| n.attrs.get("id").cloned())
        .unwrap_or_default();
    if snippet.is_empty() {
        clip(&n.tag, MAX_LABEL)
    } else {
        clip(&format!("{} {}", n.tag, clip(&snippet, 80)), MAX_LABEL)
    }
}

pub fn build(dom: &Dom, facts: &Facts, mode: RelationMode, include_context: bool, geometry_reason: &str) -> Built {
    build_ext(dom, facts, mode, include_context, geometry_reason, false)
}

/// `svg_mode` (flavor svg, UNCOMPILED pending the build gate): SVG node kinds, the
/// geometry-bearing attributes, and exact geometry lifted from the `svg_geom` /
/// `svg_geom_box` / `svg_geom_absent` flags into `attributes.geom` / `geom_box` /
/// `geom_absent` (+ `geom_provenance`). A live feed's box (`DomNode.geom`) still wins.
pub fn build_ext(dom: &Dom, facts: &Facts, mode: RelationMode, include_context: bool, geometry_reason: &str, svg_mode: bool) -> Built {
    let mut nodes: Vec<Value> = Vec::new();
    let mut truncation: Vec<String> = Vec::new();
    let mut dropped_nodes = 0usize;
    let mut with_geom = 0usize;
    let mut without_geom = 0usize;

    // Document node
    let (doc_flags, doc_prov) = flags_json(facts, 0);
    let doc_label = facts
        .flag_str(0, "title")
        .or_else(|| dom.url.clone())
        .unwrap_or_else(|| "document".to_string());
    nodes.push(json!({
        "key": "doc", "id": "doc", "kind": "UiDocument", "label": clip(&doc_label, MAX_LABEL),
        "attributes": {
            "url": dom.url, "flags": doc_flags, "provenance": doc_prov,
            "rule": "html.struct.element", "confidence": "certain", "untrusted": true,
        },
    }));

    for i in 1..dom.nodes.len() {
        if dom.is_context[i] && !include_context {
            continue;
        }
        if nodes.len() >= MAX_NODES {
            dropped_nodes += 1;
            continue;
        }
        let n = &dom.nodes[i];
        let mut attrs = Map::new();
        attrs.insert("tag".to_string(), json!(n.tag));
        if !n.text.is_empty() {
            attrs.insert("text".to_string(), json!(clip(&n.text, MAX_ATTR_VALUE)));
        }
        let is_password = n.tag == "input" && n.attrs.get("type").map_or(false, |t| t.eq_ignore_ascii_case("password"));
        let mut kept = Map::new();
        for k in KEPT_ATTRS.iter() {
            if *k == "value" && (is_password || n.attrs.get("type").map_or(false, |t| t.eq_ignore_ascii_case("hidden"))) {
                continue;
            }
            if let Some(v) = n.attrs.get(*k) {
                kept.insert(k.to_string(), json!(clip(v, MAX_ATTR_VALUE)));
            }
        }
        if svg_mode {
            for k in SVG_KEPT_ATTRS.iter() {
                if let Some(v) = n.attrs.get(*k) {
                    kept.insert(k.to_string(), json!(clip(v, MAX_ATTR_VALUE)));
                }
            }
        }
        attrs.insert("attrs".to_string(), Value::Object(kept));
        let (mut flags, mut prov) = flags_json(facts, i);
        let mut svg_geom: Option<Value> = None;
        let mut svg_box: Option<Value> = None;
        let mut svg_absent: Option<String> = None;
        let mut svg_prov: Option<Value> = None;
        if let (Some(fm), Some(pm)) = (flags.as_object_mut(), prov.as_object_mut()) {
            svg_geom = fm.remove("svg_geom");
            svg_box = fm.remove("svg_geom_box");
            svg_absent = fm.remove("svg_geom_absent").and_then(|v| v.as_str().map(|s| s.to_string()));
            svg_prov = pm.remove("svg_geom");
            pm.remove("svg_geom_box");
            pm.remove("svg_geom_absent");
        }
        match (&n.geom, svg_geom) {
            (Some(g), _) => {
                attrs.insert("geom".to_string(), g.to_json());
                with_geom += 1;
            }
            (None, Some(g)) => {
                attrs.insert("geom".to_string(), g);
                if let Some(b) = svg_box {
                    attrs.insert("geom_box".to_string(), b);
                }
                if let Some(p) = svg_prov {
                    attrs.insert("geom_provenance".to_string(), p);
                }
                with_geom += 1;
            }
            (None, None) => {
                attrs.insert("geom_absent".to_string(), json!(svg_absent.unwrap_or_else(|| geometry_reason.to_string())));
                without_geom += 1;
            }
        }
        attrs.insert("flags".to_string(), flags);
        attrs.insert("provenance".to_string(), prov);
        attrs.insert("rule".to_string(), json!("html.struct.element"));
        attrs.insert("confidence".to_string(), json!("certain"));
        attrs.insert("untrusted".to_string(), json!(true));
        let parent_key = match n.parent {
            Some(p) if p != 0 => dom.nodes[p].key.clone(),
            _ => "doc".to_string(),
        };
        nodes.push(json!({
            "key": n.key, "id": n.key,
            "kind": if svg_mode { svg_kind(&n.tag).unwrap_or_else(|| kind_for(&n.tag, facts, i)) } else { kind_for(&n.tag, facts, i) },
            "label": label_for_element(dom, facts, i),
            "parent": parent_key,
            "attributes": Value::Object(attrs),
        }));
    }

    for x in &facts.extra {
        if nodes.len() >= MAX_NODES {
            dropped_nodes += 1;
            continue;
        }
        let mut attrs = Map::new();
        for (k, v) in &x.props {
            attrs.insert(k.clone(), v.clone());
        }
        attrs.insert("rule".to_string(), json!(x.rule));
        attrs.insert("confidence".to_string(), json!(x.confidence));
        attrs.insert("untrusted".to_string(), json!(true));
        nodes.push(json!({
            "key": x.key, "id": x.key, "kind": x.kind, "label": x.label,
            "parent": x.parent_key, "attributes": Value::Object(attrs),
        }));
    }

    let emitted: Vec<String> = nodes.iter().filter_map(|n| n.get("key").and_then(|k| k.as_str()).map(|s| s.to_string())).collect();
    let emitted_set: HashSet<&str> = emitted.iter().map(|s| s.as_str()).collect();
    // keys that exist in the DOM but were not emitted because they are context-only
    let context_keys: HashSet<&str> = (1..dom.nodes.len()).filter(|i| dom.is_context[*i]).map(|i| dom.nodes[i].key.as_str()).collect();

    let mut edges: Vec<Value> = Vec::new();
    let mut external: Vec<Value> = Vec::new();
    let mut dropped_edges = 0usize;
    for e in &facts.edges {
        let relation = match mode {
            RelationMode::Semantic => Some(e.relation.clone()),
            RelationMode::Host => host_relation(&e.relation),
        };
        let relation = match relation {
            Some(r) => r,
            None => continue,
        };
        let mut edge = json!({
            "from": e.from, "to": e.to, "relation": relation,
            "confidence": conf_number(&e.confidence), "confidence_class": e.confidence, "rule": e.rule,
        });
        if relation != e.relation {
            edge["semantic"] = json!(e.relation);
        }
        let from_ok = emitted_set.contains(e.from.as_str());
        let to_ok = emitted_set.contains(e.to.as_str());
        if from_ok && to_ok {
            if edges.len() >= MAX_EDGES {
                dropped_edges += 1;
            } else {
                edges.push(edge);
            }
        } else if context_keys.contains(e.from.as_str()) || context_keys.contains(e.to.as_str()) {
            external.push(edge);
        } else {
            dropped_edges += 1;
        }
    }

    if dropped_nodes > 0 {
        truncation.push(format!("{} node(s) beyond the {}-node host limit were dropped (document order, later nodes first)", dropped_nodes, MAX_NODES));
    }
    if dropped_edges > 0 {
        truncation.push(format!("{} edge(s) dropped (edge limit {} or an endpoint was not emitted)", dropped_edges, MAX_EDGES));
    }
    let mut notes: Vec<String> = dom.notes.clone();
    notes.extend(facts.notes.iter().cloned());
    let rule_hits: BTreeMap<String, usize> = facts.rules_applied.clone();
    let mode_name = if mode == RelationMode::Host { "host" } else { "semantic" };
    let bounds = json!({
        "nodes": nodes.len(), "edges": edges.len(),
        "max_nodes": MAX_NODES, "max_edges": MAX_EDGES,
        "truncated": !truncation.is_empty(), "truncation": truncation,
        "geometry": {"with": with_geom, "without": without_geom, "reason_when_absent": geometry_reason},
        "relation_mode": mode_name,
        "notes": notes, "rule_hits": rule_hits,
    });
    Built {
        graph: json!({"nodes": nodes, "edges": edges, "bounds": bounds}),
        emitted,
        external_edges: external,
    }
}
