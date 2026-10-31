//! The rule engine. Rules are DATA (rules/*.json, format in
//! docs/UIDOCUMENT_RULES_GUIDE.md §2): `when` selects nodes with a small
//! declarative vocabulary, `emit` states a flag, an edge or a node, and every
//! emitted fact records the rule id and a confidence class. A rule whose facts
//! are missing emits nothing (absence is honest, never guessed). A few rules
//! name a `builtin` procedure for things data cannot express (accessible name,
//! cascade-derived stacking, stylesheet facts); the rule still carries its own
//! id / confidence / source, so provenance is uniform.
//!
//! Spatial relations (Above / NearTo / Overlaps ...) are NOT computed here: the
//! host does that centrally from `geom` + `frame` (docs/SPATIAL_GEOMETRY_LAYER.md).

use std::collections::{BTreeMap, HashMap, HashSet};

use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::a11y;
use crate::css::{CascadeWin, Sheet, CASCADE_PROPS};
use crate::html::{Dom, Selector, ViewInfo};

const HTML_PACK: &str = include_str!("../rules/html.json");
const CSS_PACK: &str = include_str!("../rules/css.json");

/// Materialized StyleRule nodes per run (the rest are counted, not materialized).
const MAX_STYLE_RULE_NODES: usize = 500;
/// StyledBy edges per run.
const MAX_STYLED_BY_EDGES: usize = 8_000;
const MAX_FLAG_STR: usize = 500;

fn default_phase() -> u8 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackMeta {
    pub id: String,
    pub version: String,
    pub flavor: String,
    #[serde(default)]
    pub spec_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct When {
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub not_selector: Option<String>,
    #[serde(default)]
    pub ancestor: Option<String>,
    #[serde(default)]
    pub not_ancestor: Option<String>,
    #[serde(default)]
    pub has_attr: Vec<String>,
    #[serde(default)]
    pub no_attr: Vec<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub text_matches: Option<String>,
    /// computed-style equality tests, e.g. {"display":"none"}
    #[serde(default)]
    pub style: BTreeMap<String, String>,
    /// computed-style presence tests, e.g. ["position"]
    #[serde(default)]
    pub style_has: Vec<String>,
    /// "once" => the rule runs one time with no matched node (document-level builtins).
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NodeEmit {
    pub kind: String,
    #[serde(default)]
    pub label_from: Option<String>,
    #[serde(default)]
    pub props: BTreeMap<String, String>,
    #[serde(default)]
    pub edge: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Emit {
    /// flag name (template: `meta:{attr:name}`)
    #[serde(default)]
    pub flag: Option<String>,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub value_from: Option<String>,
    /// "document" => set the flag on the Document node instead of the matched node
    #[serde(default)]
    pub scope: Option<String>,
    /// relation name (ChildOf, LabelFor, FormOwns, LinksTo, Embeds, Includes, StyledBy, Controls)
    #[serde(default)]
    pub edge: Option<String>,
    /// endpoint: "$match" | "$parent" | "$document" | {"selector":..} | {"nearest_ancestor":..} | {"descendant":..}
    #[serde(default)]
    pub from: Option<Value>,
    #[serde(default)]
    pub to: Option<Value>,
    #[serde(default)]
    pub node: Option<NodeEmit>,
    #[serde(default)]
    pub builtin: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub id: String,
    pub family: String,
    #[serde(default)]
    pub when: When,
    pub emit: Emit,
    /// certain | derived | heuristic
    pub confidence: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    /// Evaluation order across packs (lower first). Derivations that read other
    /// rules' flags (effective visibility, interactability) run in a later phase.
    #[serde(default = "default_phase")]
    pub phase: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pack {
    pub pack: PackMeta,
    pub rules: Vec<Rule>,
}

pub fn parse_pack(src: &str) -> Result<Pack, String> {
    serde_json::from_str::<Pack>(src).map_err(|e| format!("rule pack parse error: {}", e))
}

/// Packs for a flavor. The css pack applies to html documents and live-DOM too.
pub fn load_builtin(flavor: &str) -> Result<Vec<Pack>, String> {
    match flavor {
        "html" | "dom-live" => Ok(vec![parse_pack(HTML_PACK)?, parse_pack(CSS_PACK)?]),
        "css" => Ok(vec![parse_pack(CSS_PACK)?]),
        other => Err(format!("no rule packs for flavor '{}'", other)),
    }
}

// ───────────────────────── facts ─────────────────────────

#[derive(Debug, Clone)]
pub struct FlagVal {
    pub value: Value,
    pub rule: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Default)]
pub struct NodeFacts {
    pub flags: BTreeMap<String, FlagVal>,
}

#[derive(Debug, Clone)]
pub struct ExtraNode {
    pub key: String,
    pub parent_key: String,
    pub kind: String,
    pub label: String,
    pub props: BTreeMap<String, Value>,
    pub rule: String,
    pub confidence: String,
}

#[derive(Debug, Clone)]
pub struct EdgeOut {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub rule: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// Parallel to dom.nodes (index 0 = Document).
    pub nodes: Vec<NodeFacts>,
    pub extra: Vec<ExtraNode>,
    pub edges: Vec<EdgeOut>,
    pub rules_applied: BTreeMap<String, usize>,
    pub notes: Vec<String>,
}

impl Facts {
    pub fn flag(&self, idx: usize, name: &str) -> Option<&FlagVal> {
        self.nodes.get(idx).and_then(|n| n.flags.get(name))
    }
    pub fn flag_bool(&self, idx: usize, name: &str) -> Option<bool> {
        self.flag(idx, name).and_then(|f| f.value.as_bool())
    }
    pub fn flag_str(&self, idx: usize, name: &str) -> Option<String> {
        self.flag(idx, name).and_then(|f| f.value.as_str().map(|s| s.to_string()))
    }
}

// ───────────────────────── helpers ─────────────────────────

fn is_safe_ident(s: &str) -> bool {
    !s.is_empty() && s.len() <= 100 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn clean_flag_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == ':' || *c == '.')
        .take(60)
        .collect()
}

fn bound(s: &str) -> String {
    s.chars().take(MAX_FLAG_STR).collect()
}

fn has_scheme(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    for c in chars {
        if c == ':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
            return false;
        }
    }
    false
}

fn origin_of(base: &str) -> Option<String> {
    let scheme_end = base.find("://")?;
    let rest = &base[scheme_end + 3..];
    let host_end = rest.find(|c| c == '/' || c == '?' || c == '#').unwrap_or(rest.len());
    Some(base[..scheme_end + 3 + host_end].to_string())
}

/// Minimal URL resolution (no network, no dependency). Unknown shapes return the input unchanged.
pub fn resolve_url(base: Option<&str>, value: &str) -> String {
    let v = value.trim();
    if v.is_empty() || has_scheme(v) {
        return bound(v);
    }
    let base = match base {
        Some(b) if !b.is_empty() => b,
        _ => return bound(v),
    };
    let out = if let Some(rest) = v.strip_prefix("//") {
        let scheme = base.split("://").next().unwrap_or("https");
        format!("{}://{}", scheme, rest)
    } else if v.starts_with('/') {
        match origin_of(base) {
            Some(o) => format!("{}{}", o, v),
            None => v.to_string(),
        }
    } else if v.starts_with('#') {
        let no_frag = base.split('#').next().unwrap_or(base);
        format!("{}{}", no_frag, v)
    } else {
        let no_frag = base.split('#').next().unwrap_or(base);
        let no_query = no_frag.split('?').next().unwrap_or(no_frag);
        match no_query.rfind('/') {
            Some(p) if p > no_query.find("://").map_or(0, |i| i + 2) => format!("{}{}", &no_query[..p + 1], v),
            _ => format!("{}/{}", no_query.trim_end_matches('/'), v),
        }
    };
    bound(&out)
}

/// Injection-style phrases aimed at an AI reader. Detection only sets a flag:
/// nothing is removed or rewritten, and the text stays `untrusted`.
const SUSPICIOUS: [(&str, &str); 8] = [
    ("ignore-previous", r"(?i)\b(ignore|disregard|forget|override)\b.{0,40}\b(previous|prior|above|earlier|all|any|your)\b.{0,30}\b(instructions?|prompts?|rules?|guidelines?|directions?)\b"),
    ("role-reassign", r"(?i)\byou are (now |no longer )?(an? )?(ai|assistant|language model|chatbot|llm|agent)\b"),
    ("reveal-system-prompt", r"(?i)\b(reveal|print|show|repeat|output)\b.{0,30}\b(system|developer|hidden) (prompt|message|instructions?)\b"),
    ("act-as", r"(?i)\b(act|behave|respond|pretend) as (if you (are|were)|an?|the)\b"),
    ("tool-directive", r"(?i)\b(call|run|execute|invoke|use) (the |this )?(tool|function|command|terminal|browser)\b.{0,60}\b(now|immediately|first)\b"),
    ("exfiltrate", r"(?i)\b(send|post|email|upload|exfiltrate)\b.{0,50}\b(password|credentials?|cookies?|tokens?|api[ _-]?keys?|secrets?)\b"),
    ("addresses-ai", r"(?i)\b(attention|note to|message for|instructions? for) (the )?(ai|assistant|llm|agent|model)\b"),
    ("conceal-from-user", r"(?i)\bdo not (tell|inform|mention|reveal)( this to)? the user\b"),
];

struct Compiled<'a> {
    rule: &'a Rule,
    sel: Option<Selector>,
    not_sel: Option<Selector>,
    anc: Option<Selector>,
    not_anc: Option<Selector>,
    text_re: Option<Regex>,
}

pub struct Engine<'a> {
    dom: &'a Dom,
    sheets: &'a [Sheet],
    computed: &'a [BTreeMap<String, CascadeWin>],
    facts: Facts,
    edge_set: HashSet<(String, String, String)>,
    extra_counter: usize,
    id_index: HashMap<String, usize>,
    suspicious: Vec<(&'static str, Regex)>,
    /// (sheet index, rule index) -> StyleRule node key
    style_rule_nodes: Vec<((usize, usize), String)>,
    style_rules_uncounted: usize,
}

/// Run every rule of `packs` (ordered by phase, then pack order, then rule order).
pub fn run(dom: &Dom, packs: &[Pack], sheets: &[Sheet], computed: &[BTreeMap<String, CascadeWin>]) -> Facts {
    let mut id_index = HashMap::new();
    for i in 1..dom.nodes.len() {
        if let Some(id) = dom.nodes[i].attrs.get("id") {
            id_index.entry(id.clone()).or_insert(i);
        }
    }
    let suspicious: Vec<(&'static str, Regex)> = SUSPICIOUS
        .iter()
        .filter_map(|(name, pat)| Regex::new(pat).ok().map(|re| (*name, re)))
        .collect();
    let mut engine = Engine {
        dom,
        sheets,
        computed,
        facts: Facts { nodes: vec![NodeFacts::default(); dom.nodes.len()], ..Facts::default() },
        edge_set: HashSet::new(),
        extra_counter: 0,
        id_index,
        suspicious,
        style_rule_nodes: Vec::new(),
        style_rules_uncounted: 0,
    };
    let mut ordered: Vec<&Rule> = Vec::new();
    for pack in packs {
        for r in &pack.rules {
            ordered.push(r);
        }
    }
    ordered.sort_by_key(|r| r.phase); // stable: keeps pack/rule order within a phase
    for rule in ordered {
        engine.apply_rule(rule);
    }
    engine.facts
}

impl<'a> Engine<'a> {
    fn compile(&mut self, rule: &'a Rule) -> Option<Compiled<'a>> {
        let w = &rule.when;
        let mut ok = true;
        let mut parse = |s: &Option<String>, notes: &mut Vec<String>| -> Option<Selector> {
            match s {
                Some(src) => {
                    let sel = Selector::parse(src);
                    if sel.unsupported {
                        notes.push(format!("rule {} has an unsupported selector '{}' and was skipped", rule.id, src));
                        ok = false;
                    }
                    Some(sel)
                }
                None => None,
            }
        };
        let sel = parse(&w.selector, &mut self.facts.notes);
        let not_sel = parse(&w.not_selector, &mut self.facts.notes);
        let anc = parse(&w.ancestor, &mut self.facts.notes);
        let not_anc = parse(&w.not_ancestor, &mut self.facts.notes);
        let text_re = match &w.text_matches {
            Some(p) => match Regex::new(&format!("(?i){}", p)) {
                Ok(re) => Some(re),
                Err(_) => {
                    self.facts.notes.push(format!("rule {} has an invalid text_matches and was skipped", rule.id));
                    ok = false;
                    None
                }
            },
            None => None,
        };
        if !ok {
            return None;
        }
        Some(Compiled { rule, sel, not_sel, anc, not_anc, text_re })
    }

    fn apply_rule(&mut self, rule: &'a Rule) {
        let comp = match self.compile(rule) {
            Some(c) => c,
            None => return,
        };
        if rule.when.scope.as_deref() == Some("once") {
            self.emit(&comp, None);
            return;
        }
        let n = self.dom.nodes.len();
        for i in 1..n {
            if self.node_ok(&comp, i) {
                self.emit(&comp, Some(i));
            }
        }
    }

    fn node_ok(&self, c: &Compiled, i: usize) -> bool {
        let w = &c.rule.when;
        let dom = self.dom;
        if let Some(s) = &c.sel {
            if !s.is_match(dom, i) {
                return false;
            }
        }
        if let Some(s) = &c.not_sel {
            if s.is_match(dom, i) {
                return false;
            }
        }
        if let Some(s) = &c.anc {
            if !dom.ancestors(i).iter().any(|a| s.is_match(dom, *a)) {
                return false;
            }
        }
        if let Some(s) = &c.not_anc {
            if dom.ancestors(i).iter().any(|a| s.is_match(dom, *a)) {
                return false;
            }
        }
        for a in &w.has_attr {
            if !dom.nodes[i].attrs.contains_key(a) {
                return false;
            }
        }
        for a in &w.no_attr {
            if dom.nodes[i].attrs.contains_key(a) {
                return false;
            }
        }
        if let Some(r) = &w.role {
            if self.facts.flag_str(i, "role").as_deref() != Some(r.as_str()) {
                return false;
            }
        }
        if let Some(re) = &c.text_re {
            if !re.is_match(&dom.nodes[i].text) {
                return false;
            }
        }
        for (p, v) in &w.style {
            let ok = self
                .computed
                .get(i)
                .and_then(|m| m.get(p))
                .map_or(false, |win| win.value == v.to_ascii_lowercase());
            if !ok {
                return false;
            }
        }
        for p in &w.style_has {
            if !self.computed.get(i).map_or(false, |m| m.contains_key(p)) {
                return false;
            }
        }
        true
    }

    // ── value specs ──

    fn spec(&self, spec: &str, node: Option<usize>) -> Option<Value> {
        let dom = self.dom;
        if let Some(rest) = spec.strip_prefix("const:") {
            return Some(Value::String(rest.to_string()));
        }
        if let Some(rest) = spec.strip_prefix("json:") {
            return serde_json::from_str::<Value>(rest).ok();
        }
        let i = node?;
        let n = &dom.nodes[i];
        if spec == "tag" {
            return Some(Value::String(n.tag.clone()));
        }
        if spec == "text" {
            return if n.text.is_empty() { None } else { Some(Value::String(bound(&n.text))) };
        }
        if spec == "tag-digit" {
            return n.tag.chars().last().and_then(|c| c.to_digit(10)).map(|d| json!(d));
        }
        if let Some(name) = spec.strip_prefix("attr:") {
            return n.attrs.get(name).map(|v| Value::String(bound(v)));
        }
        if let Some(name) = spec.strip_prefix("bool:attr:") {
            return Some(Value::Bool(n.attrs.contains_key(name)));
        }
        if let Some(name) = spec.strip_prefix("int:attr:") {
            return n.attrs.get(name).and_then(|v| v.trim().parse::<i64>().ok()).map(|v| json!(v));
        }
        if let Some(name) = spec.strip_prefix("split:attr:") {
            return n.attrs.get(name).map(|v| json!(v.split_whitespace().take(50).collect::<Vec<_>>()));
        }
        if let Some(name) = spec.strip_prefix("url:") {
            return n.attrs.get(name).map(|v| Value::String(resolve_url(dom.url.as_deref(), v)));
        }
        if spec == "len:raw" {
            return Some(json!(n.raw_len));
        }
        if spec == "hash:raw" {
            return if n.raw_len > 0 { Some(Value::String(format!("{:016x}", n.raw_hash))) } else { None };
        }
        if spec == "forward:raw" {
            return if n.raw.is_empty() { None } else { Some(Value::String(n.raw.clone())) };
        }
        if spec == "forwardto:raw" {
            return if n.raw.is_empty() { None } else { Some(Value::String("code".to_string())) };
        }
        if let Some(prop) = spec.strip_prefix("cascade:") {
            return self.computed.get(i).and_then(|m| m.get(prop)).map(|w| Value::String(bound(&w.source)));
        }
        if let Some(prop) = spec.strip_prefix("cascadeval:") {
            return self.computed.get(i).and_then(|m| m.get(prop)).map(|w| Value::String(bound(&w.value)));
        }
        if let Some(prop) = spec.strip_prefix("cascadeint:") {
            return self
                .computed
                .get(i)
                .and_then(|m| m.get(prop))
                .and_then(|w| w.value.trim().parse::<i64>().ok())
                .map(|v| json!(v));
        }
        None
    }

    fn render(&self, t: &str, node: Option<usize>, safe_ident: bool) -> Option<String> {
        let mut out = String::new();
        let mut rest = t;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let close = after.find('}')?;
            let v = self.spec(&after[..close], node)?;
            let s = match v {
                Value::String(s) => s,
                other => other.to_string(),
            };
            if safe_ident && !is_safe_ident(s.trim_start_matches('#')) {
                return None;
            }
            out.push_str(&s);
            rest = &after[close + 1..];
        }
        out.push_str(rest);
        Some(out)
    }

    fn endpoint(&self, spec: &Value, node: Option<usize>) -> Option<usize> {
        let dom = self.dom;
        match spec {
            Value::String(s) if s == "$match" => node,
            Value::String(s) if s == "$document" => Some(0),
            Value::String(s) if s == "$parent" => node.and_then(|n| dom.nodes[n].parent).filter(|p| *p != 0),
            Value::Object(o) => {
                if let Some(t) = o.get("selector").and_then(|v| v.as_str()) {
                    let rendered = self.render(t, node, true)?;
                    if let Some(id) = rendered.strip_prefix('#') {
                        if is_safe_ident(id) {
                            return self.id_index.get(id).copied();
                        }
                    }
                    let sel = Selector::parse(&rendered);
                    if sel.unsupported {
                        return None;
                    }
                    return (1..dom.nodes.len()).find(|i| sel.is_match(dom, *i));
                }
                if let Some(t) = o.get("nearest_ancestor").and_then(|v| v.as_str()) {
                    let sel = Selector::parse(t);
                    return node.and_then(|n| dom.ancestors(n).into_iter().find(|a| sel.is_match(dom, *a)));
                }
                if let Some(t) = o.get("descendant").and_then(|v| v.as_str()) {
                    let sel = Selector::parse(t);
                    return node.and_then(|n| dom.descendants(n).into_iter().find(|d| sel.is_match(dom, *d)));
                }
                None
            }
            _ => None,
        }
    }

    // ── emitting ──

    fn count(&mut self, rule: &Rule) {
        *self.facts.rules_applied.entry(rule.id.clone()).or_insert(0) += 1;
    }

    fn set_flag(&mut self, idx: usize, name: &str, value: Value, rule: &Rule, confidence: &str) {
        let name = clean_flag_name(name);
        if name.is_empty() {
            return;
        }
        self.facts.nodes[idx].flags.insert(
            name,
            FlagVal { value, rule: rule.id.clone(), confidence: confidence.to_string() },
        );
        self.count(rule);
    }

    fn add_edge(&mut self, from: usize, to: usize, relation: &str, rule: &Rule) {
        if from == to {
            return;
        }
        let (fk, tk) = (self.dom.nodes[from].key.clone(), self.dom.nodes[to].key.clone());
        self.add_edge_keys(fk, tk, relation, rule);
    }

    fn add_edge_keys(&mut self, from: String, to: String, relation: &str, rule: &Rule) {
        if from == to {
            return;
        }
        if self.edge_set.insert((from.clone(), to.clone(), relation.to_string())) {
            self.facts.edges.push(EdgeOut {
                from,
                to,
                relation: relation.to_string(),
                rule: rule.id.clone(),
                confidence: rule.confidence.clone(),
            });
            self.count(rule);
        }
    }

    fn new_extra(&mut self, parent_key: String, kind: &str, label: String, props: BTreeMap<String, Value>, rule: &Rule) -> String {
        self.extra_counter += 1;
        let key = format!("x{}", self.extra_counter);
        self.facts.extra.push(ExtraNode {
            key: key.clone(),
            parent_key,
            kind: kind.to_string(),
            label: label.chars().take(200).collect(),
            props,
            rule: rule.id.clone(),
            confidence: rule.confidence.clone(),
        });
        self.count(rule);
        key
    }

    fn emit(&mut self, c: &Compiled, node: Option<usize>) {
        let rule = c.rule;
        let e = &rule.emit;
        if let Some(b) = &e.builtin {
            self.builtin(rule, b, node);
            return;
        }
        if let Some(flag) = &e.flag {
            let value = match (&e.value, &e.value_from) {
                (Some(v), _) => Some(v.clone()),
                (None, Some(vf)) => self.spec(vf, node),
                (None, None) => Some(Value::Bool(true)),
            };
            if let (Some(name), Some(value)) = (self.render(flag, node, false), value) {
                let target = if e.scope.as_deref() == Some("document") { 0 } else { node.unwrap_or(0) };
                self.set_flag(target, &name, value, rule, &rule.confidence.clone());
            }
        }
        if let Some(rel) = &e.edge {
            let from_spec = e.from.clone().unwrap_or_else(|| Value::String("$match".to_string()));
            if let Some(to_spec) = &e.to {
                if let (Some(f), Some(t)) = (self.endpoint(&from_spec, node), self.endpoint(to_spec, node)) {
                    self.add_edge(f, t, rel, rule);
                }
            }
        }
        if let (Some(ne), Some(i)) = (&e.node, node) {
            let label = ne
                .label_from
                .as_ref()
                .and_then(|spec| self.spec(spec, Some(i)))
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| ne.kind.clone());
            let mut props: BTreeMap<String, Value> = BTreeMap::new();
            for (k, spec) in &ne.props {
                if let Some(v) = self.spec(spec, Some(i)) {
                    props.insert(k.clone(), v);
                }
            }
            let parent_key = self.dom.nodes[i].key.clone();
            let key = self.new_extra(parent_key.clone(), &ne.kind, label, props, rule);
            if let Some(rel) = &ne.edge {
                self.add_edge_keys(parent_key, key, rel, rule);
            }
        }
    }

    // ── builtins ──

    fn builtin(&mut self, rule: &Rule, name: &str, node: Option<usize>) {
        match name {
            "node_per_element" => {} // documentation rule: the engine creates one node per element itself
            "role" => {
                if let Some(i) = node {
                    if let Some((role, conf)) = a11y::role_of(self.dom, i) {
                        self.set_flag(i, "role", Value::String(role), rule, conf);
                    }
                }
            }
            "accname" => {
                if let Some(i) = node {
                    if let Some((nm, conf)) = a11y::accname(self.dom, i) {
                        self.set_flag(i, "name", Value::String(nm), rule, conf);
                    }
                }
            }
            "suspicious_text" => {
                if let Some(i) = node {
                    self.suspicious_text(rule, i);
                }
            }
            "inline_handlers" => {
                if let Some(i) = node {
                    let names: Vec<String> = self.dom.nodes[i]
                        .attrs
                        .keys()
                        .filter(|k| k.len() > 2 && k.starts_with("on") && k[2..].chars().all(|c| c.is_ascii_alphabetic()))
                        .cloned()
                        .collect();
                    if !names.is_empty() {
                        self.set_flag(i, "inline_handlers", json!(names), rule, &rule.confidence.clone());
                    }
                }
            }
            "visibility_effective" => {
                if let Some(i) = node {
                    // display:none (and [hidden], head/script/...) removes the whole subtree;
                    // visibility:hidden hides only the element itself (children may override).
                    let own_none = self.facts.flag_bool(i, "display_none") == Some(true);
                    let vis_hidden = self.facts.flag_bool(i, "visibility_hidden") == Some(true);
                    let parent = self.dom.nodes[i].parent.unwrap_or(0);
                    let parent_hidden = parent != 0 && self.facts.flag_bool(parent, "subtree_hidden") == Some(true);
                    let subtree = own_none || parent_hidden;
                    self.set_flag(i, "subtree_hidden", Value::Bool(subtree), rule, &rule.confidence.clone());
                    self.set_flag(i, "effectively_visible", Value::Bool(!subtree && !vis_hidden), rule, &rule.confidence.clone());
                }
            }
            "interactable" => {
                if let Some(i) = node {
                    if self.facts.flag_bool(i, "interactable_base") == Some(true) {
                        let ok = self.facts.flag_bool(i, "disabled") != Some(true)
                            && self.facts.flag_bool(i, "effectively_visible") != Some(false)
                            && self.facts.flag_bool(i, "inert") != Some(true)
                            && self.facts.flag_bool(i, "pointer_events_none") != Some(true);
                        self.set_flag(i, "interactable", Value::Bool(ok), rule, &rule.confidence.clone());
                    }
                }
            }
            "stacking_context" => {
                if let Some(i) = node {
                    let pos = self.computed.get(i).and_then(|m| m.get("position")).map(|w| w.value.clone());
                    let z = self.computed.get(i).and_then(|m| m.get("z-index")).and_then(|w| w.value.trim().parse::<i64>().ok());
                    let opacity = self
                        .computed
                        .get(i)
                        .and_then(|m| m.get("opacity"))
                        .and_then(|w| w.value.trim().parse::<f64>().ok());
                    let positioned = matches!(pos.as_deref(), Some("relative") | Some("absolute") | Some("fixed") | Some("sticky"));
                    let ctx = matches!(pos.as_deref(), Some("fixed") | Some("sticky")) || (positioned && z.is_some()) || opacity.map_or(false, |o| o < 1.0);
                    if pos.is_some() || z.is_some() || opacity.is_some() {
                        self.set_flag(i, "stacking_context", Value::Bool(ctx), rule, &rule.confidence.clone());
                    }
                }
            }
            "stylesheet_facts" => self.stylesheet_facts(rule),
            "styled_by" => self.styled_by(rule),
            other => {
                self.facts.notes.push(format!("rule {} names unknown builtin '{}'", rule.id, other));
            }
        }
    }

    fn suspicious_text(&mut self, rule: &Rule, i: usize) {
        let dom = self.dom;
        let n = &dom.nodes[i];
        let mut texts: Vec<&str> = vec![n.text.as_str()];
        for k in ["alt", "title", "aria-label", "placeholder"] {
            if let Some(v) = n.attrs.get(k) {
                texts.push(v.as_str());
            }
        }
        let is_password = n.tag == "input" && n.attrs.get("type").map_or(false, |t| t.eq_ignore_ascii_case("password"));
        if !is_password {
            if let Some(v) = n.attrs.get("value") {
                texts.push(v.as_str());
            }
        }
        let mut hit: Option<&'static str> = None;
        'outer: for t in texts {
            if t.is_empty() {
                continue;
            }
            for (name, re) in &self.suspicious {
                if re.is_match(t) {
                    hit = Some(*name);
                    break 'outer;
                }
            }
        }
        if let Some(p) = hit {
            self.set_flag(i, "suspicious_instruction_text", Value::Bool(true), rule, "heuristic");
            self.set_flag(i, "suspicious_pattern", Value::String(p.to_string()), rule, "heuristic");
        }
    }

    fn stylesheet_facts(&mut self, rule: &Rule) {
        let sheets = self.sheets;
        let mut materialized = 0usize;
        for sheet in sheets {
            let mut props: BTreeMap<String, Value> = BTreeMap::new();
            props.insert("facts".to_string(), sheet.summary());
            let sheet_key = self.new_extra("doc".to_string(), "StyleSheet", sheet.label.clone(), props, rule);
            for (ri, r) in sheet.rules.iter().enumerate() {
                if materialized >= MAX_STYLE_RULE_NODES {
                    self.style_rules_uncounted += 1;
                    continue;
                }
                let mut p: BTreeMap<String, Value> = BTreeMap::new();
                p.insert("selector".to_string(), Value::String(r.selector_src.chars().take(200).collect()));
                p.insert("declaration_count".to_string(), json!(r.decls.len()));
                let decl_list: Vec<String> = r
                    .decls
                    .iter()
                    .take(20)
                    .map(|d| format!("{}: {}{}", d.prop, d.value.chars().take(60).collect::<String>(), if d.important { " !important" } else { "" }))
                    .collect();
                p.insert("declarations".to_string(), json!(decl_list));
                if let Some(m) = &r.media {
                    p.insert("conditional".to_string(), Value::String(m.chars().take(120).collect()));
                    p.insert("merged_into_cascade".to_string(), Value::Bool(false));
                }
                if let Some(l) = &r.layer {
                    p.insert("layer".to_string(), Value::String(l.clone()));
                }
                if r.selector.unsupported {
                    p.insert("selector_unsupported".to_string(), Value::Bool(true));
                }
                let key = self.new_extra(sheet_key.clone(), "StyleRule", r.selector_src.chars().take(80).collect(), p, rule);
                self.style_rule_nodes.push(((sheet.index, ri), key));
                materialized += 1;
            }
        }
        if self.style_rules_uncounted > 0 {
            self.facts.notes.push(format!(
                "{} style rule(s) beyond the {}-node cap were counted in StyleSheet facts but not materialized as nodes",
                self.style_rules_uncounted, MAX_STYLE_RULE_NODES
            ));
        }
    }

    /// rule -> matched element (direction as written in UIDOCUMENT_RULES_GUIDE).
    fn styled_by(&mut self, rule: &Rule) {
        let sheets = self.sheets;
        let nodes = self.style_rule_nodes.clone();
        let mut added = 0usize;
        for ((si, ri), key) in nodes {
            let r = match sheets.iter().find(|s| s.index == si).and_then(|s| s.rules.get(ri)) {
                Some(r) => r,
                None => continue,
            };
            if r.media.is_some() || r.selector.unsupported {
                continue;
            }
            for i in 1..self.dom.nodes.len() {
                if added >= MAX_STYLED_BY_EDGES {
                    self.facts.notes.push(format!("StyledBy edges capped at {}", MAX_STYLED_BY_EDGES));
                    return;
                }
                if r.selector.is_match(self.dom, i) {
                    let to = self.dom.nodes[i].key.clone();
                    let before = self.facts.edges.len();
                    self.add_edge_keys(key.clone(), to, "StyledBy", rule);
                    if self.facts.edges.len() > before {
                        added += 1;
                    }
                }
            }
        }
    }
}

/// Facts the reasoner view needs, extracted from the engine output.
pub fn view_info(dom: &Dom, facts: &Facts) -> ViewInfo {
    let n = dom.nodes.len();
    let mut v = ViewInfo {
        title: facts.flag_str(0, "title"),
        visible: vec![true; n],
        name: vec![None; n],
        interactable: vec![false; n],
        suspicious: vec![false; n],
    };
    for i in 1..n {
        v.visible[i] = facts.flag_bool(i, "effectively_visible").unwrap_or(true);
        v.name[i] = facts.flag_str(i, "name");
        v.interactable[i] = facts.flag_bool(i, "interactable").unwrap_or(false);
        v.suspicious[i] = facts.flag_bool(i, "suspicious_instruction_text").unwrap_or(false);
    }
    v
}

/// Names of the cascade properties (re-exported for tests / docs).
pub fn cascade_props() -> &'static [&'static str] {
    &CASCADE_PROPS
}
