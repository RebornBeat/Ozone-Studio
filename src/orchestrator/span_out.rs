//! SPAN-OUT — the one context fan-out algorithm (2026-11-02).
//!
//! Doctrine: the AMT build, the modality graphs and the cross-modal passes already
//! CAPTURE every relationship; context is never "assigned" to a step. What a step
//! (or a loop, or a ripple reaction) needs is what SPANS OUT from the things it is
//! about, along those captured relationships, ranked by how strongly they connect,
//! until the model window is full — not until an arbitrary count of containers or
//! hops is reached. The old per-step path (`context_aggregation`) took at most 4
//! traversal seeds × depth 2 × 10 results and resolved at most 8 containers; this
//! module replaces those fixed counts with a token budget.
//!
//! Algorithm (best-first, score-ranked):
//! 1. Seeds arrive with a score (1.0 = the thing the step is literally about:
//!    its AMT container, prompt/chunk/file/modality graphs, the triggering
//!    container of a ripple; lower for keyword rediscovery).
//! 2. Pop the highest-scoring unvisited node, fetch it, render it; add it to the
//!    packed context if the budget allows.
//! 3. Expand: every captured relationship (`context.relationships`, weighted by
//!    relation class), structural children, and the parent. A neighbour's
//!    score = node score × relation weight (no confidence term: edges are present or absent).
//! 4. Stop when the token budget is full, the frontier is empty, or the score floor
//!    is reached. A fetch cap exists ONLY as a runaway guard on host work; hitting
//!    it is reported (`fetch_cap_hit`), never silent.
//!
//! Infrastructure containers (Root/User/Workspace/Project/Pipeline/…roots) are never
//! rendered and never expanded through (they connect everything to everything);
//! they may still be SEEDS' parents but do not carry score.
//!
//! The same function serves: step startup, AMT re-expansion, the I-Loop, ripple
//! reactions — wherever "what is connected to this?" is the question.

use super::StoreAccess;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

/// One starting point.
#[derive(Debug, Clone)]
pub struct SpanSeed {
    pub id: u64,
    pub score: f32,
    pub why: String,
}

impl SpanSeed {
    pub fn new(id: u64, score: f32, why: &str) -> Self {
        Self { id, score, why: why.to_string() }
    }
}

/// One container that made it into the packed context.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpanItem {
    pub id: u64,
    pub container_type: String,
    pub name: String,
    pub score: f32,
    pub hops: u16,
    /// How it was reached: "seed:<why>" or "<parent id> -<relation>-> this".
    pub via: String,
    pub tokens: u32,
    /// Set when the container is a modality graph whose graph FILE was fanned out
    /// (entries, call/flow sequence, edge classes) instead of rendered as flat text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fanout: Option<FanoutInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FanoutInfo {
    pub modality: String,
    pub nodes: usize,
    pub edges: usize,
    pub entries: Vec<String>,
    pub sequence_head: Vec<String>,
    pub class_counts: std::collections::HashMap<String, usize>,
    pub lines_total: usize,
    pub lines_packed: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SpanOutResult {
    pub text: String,
    pub items: Vec<SpanItem>,
    pub tokens: u32,
    pub fetched: usize,
    pub budget_filled: bool,
    pub fetch_cap_hit: bool,
    pub frontier_left: usize,
    /// Containers reached (keyword seeds / relationship frontier) but skipped because they live under a DIFFERENT project/workspace.
    pub out_of_scope: usize,
}

const SCORE_FLOOR: f32 = 0.02;
/// Runaway guard on host work (store reads), NOT a context cap. It SCALES with the READING model's budget (operator 2026-11-06:
/// model-agnostic, no fixed limits): a fetched container is worth at least ~50 tokens of context, so a budget of B tokens can use up
/// to B/50 fetches; the floor (400) keeps small budgets exactly as before. `OZONE_SPAN_OUT_MAX_FETCH` still overrides everything.
const DEFAULT_FETCH_CAP: usize = 400;

fn fetch_cap(token_budget: u32) -> usize {
    std::env::var("OZONE_SPAN_OUT_MAX_FETCH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| (token_budget as usize / 50).max(DEFAULT_FETCH_CAP))
}

/// How strongly a relation type carries context (class weight only — no edge-confidence term).
/// Per-modality edge types are classified by `graph_fanout::classify` (structure / order /
/// flow / spatial / similarity / cross-modal / provenance); the few legacy container
/// relations keep their tuned weights.
fn relation_weight(rel: &str) -> f32 {
    match rel {
        "Continues" => 0.95,
        "Contains" | "PartOf" | "ChildOf" | "ParentOf" => 0.9,
        "Renders" | "Supports" | "DerivedFrom" | "DependsOn" | "Calls" | "Imports" | "Implements" => 0.85,
        "RelatedTo" | "SimilarTo" | "Locates" | "DepictsSameEntity" | "References" | "Mentions" => 0.8,
        other => super::graph_fanout::classify(other).weight(),
    }
}

const STRUCTURAL_CHILD_WEIGHT: f32 = 0.85;
const STRUCTURAL_PARENT_WEIGHT: f32 = 0.6;

fn is_infrastructure(t: &str) -> bool {
    matches!(
        t,
        "Root"
            | "User"
            | "Workspace"
            | "Project"
            | "Pipeline"
            | "ModalityRoot"
            | "MethodologyRoot"
            | "BlueprintRoot"
            | "PipelineRoot"
            | "ConsciousnessRoot"
            | "ExternalRoot"
            | "PackageRoot"
            | "URLRoot"
            | "JurisdictionRoot"
    )
}

/// Wrapper giving f32 a total order for the max-heap (scores are finite, non-NaN).
#[derive(Debug, Clone)]
struct Frontier {
    score: f32,
    id: u64,
    hops: u16,
    via: String,
}
impl PartialEq for Frontier {
    fn eq(&self, o: &Self) -> bool {
        self.score == o.score && self.id == o.id
    }
}
impl Eq for Frontier {}
impl PartialOrd for Frontier {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Frontier {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.score
            .partial_cmp(&o.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| o.id.cmp(&self.id))
    }
}

fn est_tokens(s: &str) -> u32 {
    ((s.len() as f32) / 4.0).ceil() as u32
}

/// Text fields worth carrying from a container's content file. Whole values only —
/// a value is included entire or not at all (never cut mid-text).
const CONTENT_KEYS: [&str; 8] = [
    "text", "content", "summary", "response", "prompt", "description", "reflection", "note",
];

fn read_content_text(data_dir: &str, rel: &str) -> Option<String> {
    let base = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| data_dir.to_string());
    let raw = std::fs::read_to_string(std::path::Path::new(&base).join(rel)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let mut parts: Vec<String> = Vec::new();
    for k in CONTENT_KEYS {
        if let Some(s) = v.get(k).and_then(|x| x.as_str()) {
            if !s.trim().is_empty() {
                parts.push(format!("{k}: {s}"));
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

struct Rendered {
    container_type: String,
    name: String,
    header_and_keywords: String,
    content: Option<String>,
    expansions: Vec<(u64, f32, String)>, // (target id, relative weight, relation label)
    /// Modality-graph fan-out of the container's graph file (None for ordinary containers).
    digest: Option<super::graph_fanout::GraphDigest>,
}

/// Read a modality graph FILE (`graphs/<modality>_<id>.json`) and digest it per its modality's rule.
fn read_graph_digest(data_dir: &str, rel: &str) -> Option<super::graph_fanout::GraphDigest> {
    let file = std::path::Path::new(rel).file_name()?.to_str()?;
    if !rel.contains("graphs/") && !rel.starts_with("graphs") {
        return None;
    }
    let modality = file.split('_').next().unwrap_or("graph").to_lowercase();
    let base = std::env::var("OZONE_ZSEI_DATA_DIR").unwrap_or_else(|_| data_dir.to_string());
    let raw = std::fs::read_to_string(std::path::Path::new(&base).join(rel)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    super::graph_fanout::digest_graph(&v, &modality)
}

fn render(data_dir: &str, c: &serde_json::Value) -> Option<Rendered> {
    let local = c.get("local_state")?;
    let meta = local.get("metadata")?;
    let container_type = meta.get("container_type").and_then(|t| t.as_str()).unwrap_or("Unknown").to_string();
    let name = meta.get("name").and_then(|n| n.as_str()).unwrap_or("Unnamed").to_string();
    let id = c
        .get("global_state")
        .and_then(|g| g.get("container_id"))
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    let ctx = local.get("context");
    let keywords: Vec<String> = ctx
        .and_then(|c| c.get("keywords"))
        .and_then(|k| k.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let mut expansions: Vec<(u64, f32, String)> = Vec::new();
    if let Some(rels) = ctx.and_then(|c| c.get("relationships")).and_then(|r| r.as_array()) {
        for r in rels {
            let Some(t) = r.get("target_id").and_then(|x| x.as_u64()) else { continue };
            let rel = r.get("relation_type").and_then(|x| x.as_str()).unwrap_or("Related");
            // NO confidence scaling (operator doctrine 2026-11-04, docs/CONTRACTS.md §5.1): a relationship is PRESENT or ABSENT — the stored
            // `confidence` is frequently a model's self-reported (false) number or the 0.5 convention, and validation is the yes/no strength-N
            // loop, so the walk is weighted by the relation's CLASS only.
            expansions.push((t, relation_weight(rel), format!("-{rel}->")));
        }
    }
    if let Some(g) = c.get("global_state") {
        if let Some(children) = g.get("child_ids").and_then(|x| x.as_array()) {
            for ch in children.iter().filter_map(|x| x.as_u64()) {
                expansions.push((ch, STRUCTURAL_CHILD_WEIGHT, "-child->".to_string()));
            }
        }
        if let Some(p) = g.get("parent_id").and_then(|x| x.as_u64()) {
            if p != 0 {
                expansions.push((p, STRUCTURAL_PARENT_WEIGHT, "-parent->".to_string()));
            }
        }
    }

    let summary = local
        .get("file_context")
        .filter(|v| !v.is_null())
        .and_then(|f| f.get("semantic_summary"))
        .and_then(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let store_path = local
        .get("storage")
        .and_then(|s| s.get("object_store_path"))
        .and_then(|p| p.as_str());
    let content_file = store_path.and_then(|rel| read_content_text(data_dir, rel));
    let digest = store_path.and_then(|rel| read_graph_digest(data_dir, rel));
    let content = match (summary, content_file) {
        (Some(s), Some(f)) => Some(format!("{s}\n{f}")),
        (Some(s), None) => Some(s),
        (None, f) => f,
    };
    let header_and_keywords = format!(
        "## {container_type}: {name} (id {id})\nKeywords: {}\n",
        keywords.join(", ")
    );
    Some(Rendered { container_type, name, header_and_keywords, content, expansions, digest })
}

/// Span out from `seeds` into `store` until `token_budget` tokens of rendered
/// context are packed (see the module doc for the algorithm and the stop rules).
pub async fn span_out(
    store: &Arc<dyn StoreAccess>,
    data_dir: &str,
    seeds: Vec<SpanSeed>,
    token_budget: u32,
) -> SpanOutResult {
    span_out_scoped(store, data_dir, seeds, token_budget, None).await
}

/// WORKSPACE / PROJECT SCOPE for a span-out (2026-11-08; live walk 1155 cited fixture graphs from OTHER projects: the keyword
/// rediscovery and the relationship frontier were host-wide). `project_id` wins over `workspace_id`. A container is IN scope when its
/// ancestry reaches the scoped project (or, for a workspace scope, the scoped workspace) OR reaches no project/workspace at all
/// (global knowledge: methodologies, pipelines, consciousness …). It is OUT of scope when it lives under a DIFFERENT project/workspace —
/// such a container is neither rendered nor expanded through. Run-owned seeds (attached files, prompt/chunk/modality graphs, AMT,
/// blueprint, methodologies) are trusted and never filtered; `None`/empty scope = the old host-wide behaviour.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpanScope {
    pub project_id: Option<u64>,
    pub workspace_id: Option<u64>,
}

impl SpanScope {
    fn active(&self) -> bool {
        self.project_id.is_some() || self.workspace_id.is_some()
    }
    /// `(project, workspace)` = the first Project / Workspace ancestor of a container (None when it has none).
    fn admits(&self, found: (Option<u64>, Option<u64>)) -> bool {
        let (proj, ws) = found;
        if let Some(p) = self.project_id {
            return match (proj, ws) {
                (Some(q), _) => q == p,
                (None, Some(w)) => Some(w) == self.workspace_id,
                (None, None) => true,
            };
        }
        if let Some(w) = self.workspace_id {
            return match ws {
                Some(x) => x == w,
                None => proj.is_none(),
            };
        }
        true
    }
}

/// Walk a container's ancestry (cycle-guarded, no depth cap) to its first Project and first Workspace ancestor; memoised per call.
async fn ancestry_scope(
    store: &Arc<dyn StoreAccess>,
    container: &serde_json::Value,
    cache: &mut std::collections::HashMap<u64, (Option<u64>, Option<u64>)>,
) -> (Option<u64>, Option<u64>) {
    // FAST PATH (2026-11-08): the typed scope (`Context.scope`, filled centrally on every write from the marker keywords) answers without
    // walking the ancestry. `global` = visible everywhere = out of no scope.
    {
        let tp = container.pointer("/local_state/context/scope/project_id").and_then(|v| v.as_u64());
        let tw = container.pointer("/local_state/context/scope/workspace_id").and_then(|v| v.as_u64());
        let tg = container.pointer("/local_state/context/scope/global").and_then(|v| v.as_bool()).unwrap_or(false);
        if tp.is_some() || tw.is_some() {
            return (tp, tw);
        }
        if tg {
            return (None, None);
        }
    }
    let mut project: Option<u64> = None;
    let mut workspace: Option<u64> = None;
    let mut visited: HashSet<u64> = HashSet::new();
    let mut cur = container.clone();
    loop {
        let id = cur.pointer("/global_state/container_id").and_then(|v| v.as_u64()).unwrap_or(0);
        if id != 0 && !visited.insert(id) {
            break; // cycle
        }
        if let Some(hit) = cache.get(&id).filter(|_| id != 0) {
            project = project.or(hit.0);
            workspace = workspace.or(hit.1);
            break;
        }
        match cur.pointer("/local_state/metadata/container_type").and_then(|t| t.as_str()) {
            Some("Project") if project.is_none() => project = Some(id),
            Some("Workspace") if workspace.is_none() => workspace = Some(id),
            _ => {}
        }
        let parent = cur.pointer("/global_state/parent_id").and_then(|v| v.as_u64()).unwrap_or(0);
        if parent == 0 || (project.is_some() && workspace.is_some()) {
            break;
        }
        match store.get_container(parent).await {
            Ok(Some(c)) => cur = c,
            _ => break,
        }
    }
    let own = container.pointer("/global_state/container_id").and_then(|v| v.as_u64()).unwrap_or(0);
    if own != 0 {
        cache.insert(own, (project, workspace));
    }
    (project, workspace)
}

pub async fn span_out_scoped(
    store: &Arc<dyn StoreAccess>,
    data_dir: &str,
    seeds: Vec<SpanSeed>,
    token_budget: u32,
    scope: Option<SpanScope>,
) -> SpanOutResult {
    let scope = scope.filter(|s| s.active());
    let mut scope_cache: std::collections::HashMap<u64, (Option<u64>, Option<u64>)> = std::collections::HashMap::new();
    let cap = fetch_cap(token_budget);
    let mut heap: BinaryHeap<Frontier> = BinaryHeap::new();
    let mut seen: HashSet<u64> = HashSet::new();
    let mut result = SpanOutResult::default();
    let mut packed = String::new();

    for s in seeds {
        if seen.insert(s.id) {
            heap.push(Frontier { score: s.score.clamp(0.0, 1.0), id: s.id, hops: 0, via: format!("seed:{}", s.why) });
        }
    }
    // `seen` marks "queued", so a node is scored by the first (strongest, because
    // best-first) path that reaches it.

    while let Some(node) = heap.pop() {
        if result.tokens >= token_budget {
            result.budget_filled = true;
            break;
        }
        if result.fetched >= cap {
            result.fetch_cap_hit = true;
            break;
        }
        result.fetched += 1;
        let Ok(Some(container)) = store.get_container(node.id).await else { continue };
        // scope: run-owned seeds are trusted; keyword seeds and everything reached through relationships must be in scope
        if let Some(sc) = &scope {
            let trusted = node.via.starts_with("seed:") && node.via != "seed:keyword";
            if !trusted {
                let found = ancestry_scope(store, &container, &mut scope_cache).await;
                if !sc.admits(found) {
                    result.out_of_scope += 1;
                    continue; // not rendered, not expanded through
                }
            }
        }
        let Some(r) = render(data_dir, &container) else { continue };
        if is_infrastructure(&r.container_type) {
            continue; // never rendered, never expanded through
        }

        // Pack: header+keywords always (small); content only if it fits whole.
        let head_tokens = est_tokens(&r.header_and_keywords);
        let remaining = token_budget.saturating_sub(result.tokens);
        if head_tokens > remaining {
            result.budget_filled = true;
            break;
        }
        let mut block = r.header_and_keywords.clone();
        let mut tokens = head_tokens;
        if let Some(c) = &r.content {
            let ct = est_tokens(c);
            if tokens + ct <= remaining {
                block.push_str(c);
                block.push('\n');
                tokens += ct;
            }
        }
        // Modality graph: entries -> flow/call sequence -> structure, whole LINES while they fit.
        let mut fanout: Option<FanoutInfo> = None;
        if let Some(d) = &r.digest {
            let mut packed_lines = 0usize;
            let mut gtext = String::new();
            for line in &d.lines {
                let lt = est_tokens(&line.text) + 1;
                if tokens + lt + 12 > remaining {
                    break;
                }
                gtext.push_str(&line.text);
                gtext.push('\n');
                tokens += lt;
                packed_lines += 1;
            }
            block.push_str(&gtext);
            fanout = Some(FanoutInfo {
                modality: d.modality.clone(),
                nodes: d.nodes,
                edges: d.edges,
                entries: d.entries.clone(),
                sequence_head: d.sequence.iter().take(12).cloned().collect(),
                class_counts: d.class_counts.clone(),
                lines_total: d.lines.len(),
                lines_packed: packed_lines,
            });
        }
        block.push_str(&format!("[reached: {} | score {:.2} | hops {}]\n\n", node.via, node.score, node.hops));
        tokens += 12;
        packed.push_str(&block);
        result.tokens += tokens;
        result.items.push(SpanItem {
            id: node.id,
            container_type: r.container_type.clone(),
            name: r.name.clone(),
            score: node.score,
            hops: node.hops,
            via: node.via.clone(),
            tokens,
            fanout,
        });

        for (target, w, label) in r.expansions {
            if seen.contains(&target) {
                continue;
            }
            let score = node.score * w;
            if score < SCORE_FLOOR {
                continue;
            }
            seen.insert(target);
            heap.push(Frontier { score, id: target, hops: node.hops + 1, via: format!("{} {} this", node.id, label) });
        }
    }
    result.frontier_left = heap.len();
    result.text = packed;
    result
}

impl super::PromptOrchestrator {
    /// Span out from everything THIS RUN already captured about the step — the AMT
    /// container, the prompt / chunk / attached-file / modality graphs, the blueprint
    /// and methodologies — plus keyword rediscovery for the step's own terms, along the
    /// captured relationships, until `state.model_context_limit / 4` tokens are packed.
    pub(crate) async fn span_out_for_step(
        &self,
        state: &super::OrchestrationState,
        step: &super::BlueprintStep,
    ) -> SpanOutResult {
        let mut seeds: Vec<SpanSeed> = Vec::new();
        for id in state.file_graphs.values() {
            seeds.push(SpanSeed::new(*id, 1.0, "attached-file graph"));
        }
        for id in state.modality_graphs.values() {
            seeds.push(SpanSeed::new(*id, 1.0, "prompt modality graph"));
        }
        for id in &state.chunk_graph_ids {
            seeds.push(SpanSeed::new(*id, 0.9, "prompt chunk graph"));
        }
        if let Some(id) = state.amt_container_id {
            seeds.push(SpanSeed::new(id, 1.0, "AMT"));
        }
        if let Some(id) = state.blueprint_id {
            seeds.push(SpanSeed::new(id, 0.7, "blueprint"));
        }
        for id in &state.methodologies {
            seeds.push(SpanSeed::new(*id, 0.6, "methodology"));
        }
        // Keyword rediscovery for this step's own terms (branch-derived keywords first).
        // KEYWORD HYGIENE (2026-11-08): the raw whitespace tokens of the step text (stopwords, punctuation attached) used to be searched
        // host-wide; now branch-derived keywords as given + the description's CONTENT words (request_terms, same rule as Stage 2/3).
        // the branch keywords go through the SHAPE rules first (no word list): a whole sentence becomes its tokens, `you (implied)` loses its
        // parenthetical, markers are dropped — live: one training query carried a 13-word sentence and filler as "keywords" (STATE_REVIEW I6)
        let mut kws: Vec<String> = super::request_terms::sanitize_keywords("", step.context_requirements.iter().cloned().collect());
        for w in super::request_terms::content_words(&step.description) {
            if !kws.iter().any(|k| k.eq_ignore_ascii_case(&w)) {
                kws.push(w);
            }
        }
        if !kws.is_empty() {
            // ranked + token-aware (2026-11-08): best match first, so the window fills with the best hits, not an arbitrary slice
            if let Ok(ids) = self.store.search_ranked(&kws, None).await {
                // rank-derived seed scores (ordinal, NOT a confidence): the best hit seeds highest, the last lowest; all hits kept (no cap)
                let n = ids.len().max(1) as f32;
                for (rank, id) in ids.into_iter().enumerate() {
                    seeds.push(SpanSeed::new(id, 0.5 * (1.0 - 0.5 * (rank as f32) / n), "keyword"));
                }
            }
        }
        let budget = (state.model_context_limit / 4).max(256);
        // SCOPE (2026-11-08): a request made inside a project/workspace spans out only through that scope (+ global knowledge).
        let scope = SpanScope { project_id: state.request.project_id, workspace_id: state.request.workspace_id };
        let r = span_out_scoped(&self.store, &self.data_dir, seeds, budget, Some(scope)).await;
        // HIT capture: everything that made it into this step's window was PACKED (used as context)
        for item in &r.items {
            crate::keyword_program::record_event("packed", item.id, &kws, serde_json::json!({"via": item.via, "step": step.step_index}));
        }
        tracing::info!(out_of_scope = r.out_of_scope, 
            step = step.step_index,
            items = r.items.len(),
            tokens = r.tokens,
            fetched = r.fetched,
            budget_filled = r.budget_filled,
            fetch_cap_hit = r.fetch_cap_hit,
            frontier_left = r.frontier_left,
            "span-out for step: context spanned out along captured relationships"
        );
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;

    /// In-memory store: id -> container JSON. Only `get_container` is exercised.
    struct MemStore(HashMap<u64, serde_json::Value>);

    #[async_trait]
    impl StoreAccess for MemStore {
        async fn query(&self, _q: serde_json::Value) -> Result<serde_json::Value, String> { Err("n/a".into()) }
        async fn traverse(&self, _r: serde_json::Value) -> Result<serde_json::Value, String> { Err("n/a".into()) }
        async fn create_container(&self, _p: u64, _c: serde_json::Value) -> Result<u64, String> { Err("n/a".into()) }
        async fn update_container(&self, _i: u64, _u: serde_json::Value) -> Result<(), String> { Err("n/a".into()) }
        async fn get_container(&self, id: u64) -> Result<Option<serde_json::Value>, String> { Ok(self.0.get(&id).cloned()) }
        async fn search_by_keywords(&self, _k: &[String], _t: Option<&str>) -> Result<Vec<u64>, String> { Ok(vec![]) }
        async fn get_categories(&self, _m: &str) -> Result<Vec<u64>, String> { Ok(vec![]) }
    }

    fn node(id: u64, ty: &str, name: &str, rels: &[(u64, &str, f64)], children: &[u64]) -> serde_json::Value {
        serde_json::json!({
            "global_state": { "container_id": id, "parent_id": 0, "child_ids": children, "child_count": children.len(), "version": 1 },
            "local_state": {
                "metadata": { "container_type": ty, "name": name },
                "context": {
                    "keywords": [name],
                    "relationships": rels.iter().map(|(t, r, c)| serde_json::json!({"target_id": t, "relation_type": r, "confidence": c})).collect::<Vec<_>>()
                },
                "storage": { "object_store_path": null }
            }
        })
    }

    fn store() -> Arc<dyn StoreAccess> {
        let mut m = HashMap::new();
        m.insert(1, node(1, "ModalityGraph", "seed", &[(2, "RelatedTo", 1.0)], &[]));
        m.insert(2, node(2, "ModalityGraph", "hop1", &[(3, "SimilarTo", 0.9)], &[]));
        m.insert(3, node(3, "ModalityGraph", "hop2", &[], &[]));
        m.insert(9, node(9, "Project", "project", &[(1, "RelatedTo", 1.0)], &[]));
        Arc::new(MemStore(m))
    }

    #[tokio::test]
    async fn spans_out_past_depth_two_along_relationships() {
        let r = span_out(&store(), ".", vec![SpanSeed::new(1, 1.0, "test")], 10_000).await;
        let ids: Vec<u64> = r.items.iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(r.items[2].hops, 2);
        assert!(r.text.contains("hop2"));
        assert!(!r.budget_filled && !r.fetch_cap_hit);
    }

    #[tokio::test]
    async fn stops_when_the_token_budget_is_full() {
        let r = span_out(&store(), ".", vec![SpanSeed::new(1, 1.0, "test")], 30).await;
        assert!(r.items.len() < 3);
        assert!(r.budget_filled);
    }

    #[tokio::test]
    async fn infrastructure_is_neither_rendered_nor_expanded_through() {
        let r = span_out(&store(), ".", vec![SpanSeed::new(9, 1.0, "project")], 10_000).await;
        assert!(r.items.is_empty());
    }
}
