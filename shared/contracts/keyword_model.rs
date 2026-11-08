//! KEYWORD MODEL — the shared core of keyword matching, ranking and the custom ML (2026-11-08).
//!
//! Single source of truth, included by the host crate (via `shared/contracts/mod.rs`) AND by pipeline crates (via `#[path]`), exactly like
//! `k_validation.rs`: the host search door uses it IN-PROCESS for inference (a subprocess per search would cost more than the scan itself);
//! the `traversal_ml` pipeline (#17) uses the same code for training / evaluation. Only `serde` + `serde_json` are used — no ML crate.
//!
//! DOCTRINE (docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md):
//! * NO FALSE CONFIDENCE — nothing here reports a probability or a confidence. Everything that leaves this module is an ORDER (a ranking)
//!   or a yes/no decision backed by measured held-out numbers. Model scores are internal sort keys and are never persisted or displayed.
//! * NO STOPWORD LIST — shape rules only (namespaced markers, sentence-like items). Filler handling is learned.
//! * RARITY IS NOT THE ORDERING RULE — the deterministic v0 key is a lexicographic tuple of OBSERVED facts (scope, match class, matched
//!   terms, validation, real hit history, graph distance, recency).
//! * INTERCHANGEABLE BACKENDS — any number of `TermEncoder`s (hashed n-gram, pretrained, embedder) and the learned ranker can be active
//!   at once; their orderings are combined by reciprocal-rank fusion (rank-based, no weights to invent). A backend that is not available
//!   reports `Err` and is simply skipped.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;

// ============================================================================================================
// 1. Tokens and term SHAPE (no lists)
// ============================================================================================================

/// Lower-cased alphanumeric runs of `s` (Unicode-aware). `crate::auth` → ["crate","auth"]; `run_report` → ["run","report"].
pub fn tokens(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            for lc in ch.to_lowercase() {
                cur.push(lc);
            }
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// A NAMESPACED MARKER (`scope:global`, `ws:3`, `call:tool:x`, `trace:ab12`): `ns:value` with a lowercase `[a-z][a-z0-9_-]*` namespace and a
/// non-empty value that does not start with `:` or `//` (so `crate::auth` and `http://x` are NOT markers). Markers are scope/provenance, not meaning.
pub fn is_marker(term: &str) -> bool {
    let t = term.trim();
    if t.chars().any(|c| c.is_whitespace()) {
        return false;
    }
    let Some(colon) = t.find(':') else { return false };
    let (ns, rest) = (&t[..colon], &t[colon + 1..]);
    let mut chars = ns.chars();
    let first_ok = chars.next().map(|c| c.is_ascii_lowercase()).unwrap_or(false);
    if !first_ok || !ns.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
        return false;
    }
    !rest.is_empty() && !rest.starts_with(':') && !rest.starts_with("//")
}

/// A sentence-like item (more than 4 whitespace-separated words) is not a keyword — it is a sentence the extractor returned whole.
pub fn is_sentence_like(term: &str) -> bool {
    term.split_whitespace().count() > 4
}

/// The MEANING terms of a keyword list: not a marker, not sentence-like, not empty; trimmed, de-duplicated case-insensitively, order kept.
pub fn meaning_terms(keywords: &[String]) -> Vec<String> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = Vec::new();
    for k in keywords {
        let t = k.trim();
        if t.is_empty() || is_marker(t) || is_sentence_like(t) {
            continue;
        }
        if seen.insert(t.to_lowercase()) {
            out.push(t.to_string());
        }
    }
    out
}

// ============================================================================================================
// 2. Match classes, rank facts and the deterministic v0 rank key
// ============================================================================================================

/// How a query term matched a container, best first. `Substring` is the last class — a reachable result, never a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MatchClass {
    ExactName = 0,
    ExactTerm = 1,
    WholeToken = 2,
    Substring = 3,
}

/// Result of matching a query against ONE container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchInfo {
    /// The best class over all matched query terms.
    pub best: MatchClass,
    /// Number of DISTINCT query terms that matched anything.
    pub matched_terms: usize,
}

fn whole_token_match(a_tokens: &[String], b_tokens: &[String]) -> bool {
    if a_tokens.is_empty() || b_tokens.is_empty() {
        return false;
    }
    let a_in_b = a_tokens.iter().all(|t| b_tokens.contains(t));
    let b_in_a = b_tokens.iter().all(|t| a_tokens.contains(t));
    a_in_b || b_in_a
}

/// Match `query` against a container's `name` and `terms` (its keywords + topics, markers INCLUDED — a marker-shaped query term can match
/// a marker, a meaning term cannot). `allow_substring` adds the legacy substring class (off for the token strategy's default; the
/// `scan`/`scan-legacy` strategies stay available as selectable baselines). Returns None when nothing matched.
pub fn match_info(query: &[String], name: &str, terms: &[String], allow_substring: bool) -> Option<MatchInfo> {
    let name_l = name.trim().to_lowercase();
    let name_tokens = tokens(&name_l);
    let term_l: Vec<String> = terms.iter().map(|t| t.trim().to_lowercase()).collect();
    let term_tokens: Vec<Vec<String>> = term_l.iter().map(|t| tokens(t)).collect();
    let mut best: Option<MatchClass> = None;
    let mut matched = 0usize;
    let mut seen_q: std::collections::HashSet<String> = std::collections::HashSet::new();
    for q in query {
        let ql = q.trim().to_lowercase();
        if ql.is_empty() || !seen_q.insert(ql.clone()) {
            continue;
        }
        let q_marker = is_marker(&ql);
        let qt = tokens(&ql);
        let mut class: Option<MatchClass> = None;
        let mut bump = |c: MatchClass| {
            class = Some(match class {
                Some(prev) if prev <= c => prev,
                _ => c,
            });
        };
        if !name_l.is_empty() && name_l == ql {
            bump(MatchClass::ExactName);
        } else if !name_tokens.is_empty() && whole_token_match(&qt, &name_tokens) {
            bump(MatchClass::WholeToken);
        } else if allow_substring && !name_l.is_empty() && (name_l.contains(&ql) || ql.contains(&name_l)) {
            bump(MatchClass::Substring);
        }
        for (i, tl) in term_l.iter().enumerate() {
            if tl.is_empty() {
                continue;
            }
            let t_marker = is_marker(tl);
            if t_marker != q_marker {
                continue; // a marker only meets a marker; meaning meets meaning
            }
            if *tl == ql {
                bump(MatchClass::ExactTerm);
            } else if whole_token_match(&qt, &term_tokens[i]) {
                bump(MatchClass::WholeToken);
            } else if allow_substring && (tl.contains(&ql) || ql.contains(tl.as_str())) {
                bump(MatchClass::Substring);
            }
        }
        if let Some(c) = class {
            matched += 1;
            best = Some(match best {
                Some(prev) if prev <= c => prev,
                _ => c,
            });
        }
    }
    best.map(|b| MatchInfo { best: b, matched_terms: matched })
}

/// The OBSERVED facts one match is ranked on. Counts are real event counts (from the hit log rollup); nothing here is a model score.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RankFacts {
    pub in_scope: bool,
    pub class: MatchClass,
    pub matched_terms: usize,
    /// A matched term has a strength-N YES judgment (`keyword_meta.validated == Some(true)`).
    pub validated: bool,
    pub applied: u64,
    pub kept: u64,
    pub packed: u64,
    pub returned: u64,
    pub dropped: u64,
    /// OBSERVED hotness: packed ÷ returned from the real hit counts (0.0 when never returned) — how often being found actually led to being
    /// used. The invented per-node `hotness_score` literals are gone (2026-11-08); this is the only hotness the ranking reads.
    #[serde(default)]
    pub hotness: f32,
    /// Graph distance from the request graph when known.
    pub hops: Option<u32>,
    /// Epoch seconds of the last access (0 = never).
    pub last_accessed: u64,
}

impl RankFacts {
    pub fn new(class: MatchClass, matched_terms: usize) -> Self {
        Self { in_scope: true, class, matched_terms, validated: false, applied: 0, kept: 0, packed: 0, returned: 0, dropped: 0, hotness: 0.0, hops: None, last_accessed: 0 }
    }
}

/// The deterministic v0 ordering: lexicographic over observed facts, BEST FIRST (`Less` = ranks earlier). No weights, no scores.
pub fn cmp_v0(a: &RankFacts, b: &RankFacts) -> Ordering {
    b.in_scope
        .cmp(&a.in_scope)
        .then(a.class.cmp(&b.class))
        .then(b.matched_terms.cmp(&a.matched_terms))
        .then(b.validated.cmp(&a.validated))
        .then(b.applied.cmp(&a.applied))
        .then(b.kept.cmp(&a.kept))
        .then(a.dropped.cmp(&b.dropped))
        .then_with(|| b.hotness.partial_cmp(&a.hotness).unwrap_or(Ordering::Equal))
        .then(b.packed.cmp(&a.packed))
        .then(b.returned.cmp(&a.returned))
        .then_with(|| match (a.hops, b.hops) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        })
        .then(b.last_accessed.cmp(&a.last_accessed))
}

/// Rank `items` (id + facts) by v0, stable (ties keep input order), returning ids best-first.
pub fn rank_v0(items: &[(u64, RankFacts)]) -> Vec<u64> {
    let mut idx: Vec<usize> = (0..items.len()).collect();
    idx.sort_by(|&i, &j| cmp_v0(&items[i].1, &items[j].1).then(i.cmp(&j)));
    idx.into_iter().map(|i| items[i].0).collect()
}

// ============================================================================================================
// 3. Fusion of interchangeable backends
// ============================================================================================================

/// The standard reciprocal-rank-fusion constant (Cormack et al. 2009) — a published constant, not a tuned weight.
pub const RRF_K: f64 = 60.0;

/// Combine several rankings (each best-first, over overlapping id sets) into one: score(id) = Σ 1/(RRF_K + rank). Ids missing from a
/// ranking contribute nothing for it. Ties keep the order of first appearance. Rank-based, so backends need no common scale.
pub fn reciprocal_rank_fusion(rankings: &[Vec<u64>]) -> Vec<u64> {
    let mut score: BTreeMap<u64, f64> = BTreeMap::new();
    let mut first_seen: Vec<u64> = Vec::new();
    for r in rankings {
        for (rank, id) in r.iter().enumerate() {
            if !score.contains_key(id) {
                first_seen.push(*id);
            }
            *score.entry(*id).or_insert(0.0) += 1.0 / (RRF_K + (rank as f64) + 1.0);
        }
    }
    let pos: BTreeMap<u64, usize> = first_seen.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut ids = first_seen;
    ids.sort_by(|a, b| {
        score[b].partial_cmp(&score[a]).unwrap_or(Ordering::Equal).then(pos[a].cmp(&pos[b]))
    });
    ids
}

// ============================================================================================================
// 4. Encoders — the interchangeable backend interface
// ============================================================================================================

/// A text → vector backend. Backends are interchangeable and can run together; one that cannot run returns `Err` and is skipped.
pub trait TermEncoder: Send + Sync {
    fn id(&self) -> &str;
    /// "hashed_ngram" | "pretrained" | "embedder".
    fn kind(&self) -> &'static str;
    fn dim(&self) -> usize;
    /// The (unnormalised) vector for `text`.
    fn encode(&self, text: &str) -> Result<Vec<f32>, String>;
    /// Cosine similarity of two texts' vectors (0.0 when either vector is the zero vector).
    fn similarity(&self, a: &str, b: &str) -> Result<f32, String> {
        let (x, y) = (self.encode(a)?, self.encode(b)?);
        Ok(cosine(&x, &y))
    }
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na <= 0.0 || nb <= 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn fnv1a(bytes: &[u8], seed: u64) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ seed;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// FASTTEXT-STYLE HASHED CHARACTER-N-GRAM ENCODER, pure Rust, trainable. A text is split into tokens; each token (wrapped in `<` `>`) yields
/// every character n-gram of length `nmin..=nmax` plus the whole token, each hashed into one of `buckets` rows of `dim` floats; the text
/// vector is the mean of those rows. Character n-grams make it language-neutral and robust to inflection, typos and unseen words.
/// Initialisation is deterministic (hash-derived), so an untrained encoder is reproducible.
#[derive(Debug, Clone)]
pub struct HashedNgramEncoder {
    pub id: String,
    pub dim: usize,
    pub buckets: usize,
    pub nmin: usize,
    pub nmax: usize,
    pub version: u32,
    pub trained_on_samples: u64,
    pub weights: Vec<f32>,
}

/// JSON header stored beside the binary weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashedNgramHeader {
    pub id: String,
    pub dim: usize,
    pub buckets: usize,
    pub nmin: usize,
    pub nmax: usize,
    pub version: u32,
    pub trained_on_samples: u64,
}

impl HashedNgramEncoder {
    pub const DEFAULT_DIM: usize = 64;
    pub const DEFAULT_BUCKETS: usize = 1 << 16;

    pub fn new(id: &str, dim: usize, buckets: usize) -> Self {
        let dim = dim.max(1);
        let buckets = buckets.max(1);
        let mut weights = vec![0.0f32; dim * buckets];
        for b in 0..buckets {
            let mut st = fnv1a(&(b as u64).to_le_bytes(), 0x9e3779b97f4a7c15) | 1;
            for d in 0..dim {
                let r = xorshift(&mut st);
                // uniform in (-0.5, 0.5) / dim
                weights[b * dim + d] = ((r >> 40) as f32 / (1u64 << 24) as f32 - 0.5) / dim as f32;
            }
        }
        Self { id: id.to_string(), dim, buckets, nmin: 3, nmax: 5, version: 1, trained_on_samples: 0, weights }
    }

    /// The bucket indices a text maps to (duplicates kept: a repeated n-gram counts repeatedly, like the mean it feeds).
    pub fn features(&self, text: &str) -> Vec<usize> {
        let mut out = Vec::new();
        for tok in tokens(text) {
            let wrapped: Vec<char> = std::iter::once('<').chain(tok.chars()).chain(std::iter::once('>')).collect();
            out.push((fnv1a(tok.as_bytes(), 0x1234_5678) % self.buckets as u64) as usize);
            for n in self.nmin..=self.nmax {
                if wrapped.len() < n {
                    break;
                }
                for w in wrapped.windows(n) {
                    let s: String = w.iter().collect();
                    out.push((fnv1a(s.as_bytes(), n as u64) % self.buckets as u64) as usize);
                }
            }
        }
        out
    }

    /// Mean of the feature rows (the unnormalised text vector) — zero vector for an empty text.
    pub fn raw(&self, feats: &[usize]) -> Vec<f32> {
        let mut v = vec![0.0f32; self.dim];
        if feats.is_empty() {
            return v;
        }
        for &b in feats {
            let row = &self.weights[b * self.dim..(b + 1) * self.dim];
            for d in 0..self.dim {
                v[d] += row[d];
            }
        }
        let inv = 1.0 / feats.len() as f32;
        for x in v.iter_mut() {
            *x *= inv;
        }
        v
    }

    /// The pooled vector of a SET of terms (a container's meaning terms): features of all terms concatenated.
    pub fn features_of_terms(&self, terms: &[String]) -> Vec<usize> {
        let mut f = Vec::new();
        for t in terms {
            f.extend(self.features(t));
        }
        f
    }

    /// One SGD step on a (query, container) pair: logistic loss on the dot product of the two mean vectors. `label` true = relevant.
    /// Rows are clamped to unit norm afterwards so training cannot blow up.
    pub fn train_pair(&mut self, query_feats: &[usize], container_feats: &[usize], label: bool, lr: f32) -> f32 {
        if query_feats.is_empty() || container_feats.is_empty() {
            return 0.0;
        }
        let u = self.raw(query_feats);
        let v = self.raw(container_feats);
        let s: f32 = u.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
        let p = 1.0 / (1.0 + (-s).exp());
        let y = if label { 1.0 } else { 0.0 };
        let g = p - y;
        let loss = -(if label { p.max(1e-7).ln() } else { (1.0 - p).max(1e-7).ln() });
        let (nu, nv) = (query_feats.len() as f32, container_feats.len() as f32);
        let mut touched: Vec<usize> = Vec::new();
        for &b in query_feats {
            let row = &mut self.weights[b * self.dim..(b + 1) * self.dim];
            for d in 0..self.dim {
                row[d] -= lr * g * v[d] / nu;
            }
            touched.push(b);
        }
        for &b in container_feats {
            let row = &mut self.weights[b * self.dim..(b + 1) * self.dim];
            for d in 0..self.dim {
                row[d] -= lr * g * u[d] / nv;
            }
            touched.push(b);
        }
        touched.sort_unstable();
        touched.dedup();
        for b in touched {
            let row = &mut self.weights[b * self.dim..(b + 1) * self.dim];
            let norm: f32 = row.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 1.0 {
                for x in row.iter_mut() {
                    *x /= norm;
                }
            }
        }
        loss
    }

    pub fn header(&self) -> HashedNgramHeader {
        HashedNgramHeader {
            id: self.id.clone(),
            dim: self.dim,
            buckets: self.buckets,
            nmin: self.nmin,
            nmax: self.nmax,
            version: self.version,
            trained_on_samples: self.trained_on_samples,
        }
    }

    /// Write `<stem>.json` (header) and `<stem>.bin` (little-endian f32 weights) under `dir`.
    pub fn save(&self, dir: &Path, stem: &str) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let header = serde_json::to_string_pretty(&self.header()).map_err(|e| e.to_string())?;
        let mut bytes: Vec<u8> = Vec::with_capacity(self.weights.len() * 4);
        for w in &self.weights {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        // temp + rename so a reader never sees a half-written model
        let bin_tmp = dir.join(format!("{stem}.bin.tmp"));
        std::fs::write(&bin_tmp, &bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&bin_tmp, dir.join(format!("{stem}.bin"))).map_err(|e| e.to_string())?;
        let json_tmp = dir.join(format!("{stem}.json.tmp"));
        std::fs::write(&json_tmp, header).map_err(|e| e.to_string())?;
        std::fs::rename(&json_tmp, dir.join(format!("{stem}.json"))).map_err(|e| e.to_string())
    }

    pub fn load(dir: &Path, stem: &str) -> Result<Self, String> {
        let header: HashedNgramHeader =
            serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{stem}.json"))).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let bytes = std::fs::read(dir.join(format!("{stem}.bin"))).map_err(|e| e.to_string())?;
        let expected = header.dim * header.buckets * 4;
        if bytes.len() != expected {
            return Err(format!("model weights are {} bytes, header says {}", bytes.len(), expected));
        }
        let weights: Vec<f32> = bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        Ok(Self {
            id: header.id,
            dim: header.dim,
            buckets: header.buckets,
            nmin: header.nmin,
            nmax: header.nmax,
            version: header.version,
            trained_on_samples: header.trained_on_samples,
            weights,
        })
    }
}

impl TermEncoder for HashedNgramEncoder {
    fn id(&self) -> &str {
        &self.id
    }
    fn kind(&self) -> &'static str {
        "hashed_ngram"
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn encode(&self, text: &str) -> Result<Vec<f32>, String> {
        Ok(self.raw(&self.features(text)))
    }
}

/// PREPARED backend: a pretrained sentence encoder (e.g. a 384-dim multilingual MiniLM-class model fine-tuned on Ozone's hit pairs).
/// Needs (1) third-party weights (an OPERATOR decision) and (2) an inference runtime (a dependency decision). Until both exist it reports
/// `Err` and the fusion skips it — the slot, the registry entry and the training contract are in place so it can come online without
/// touching callers. Never fabricates a vector.
#[derive(Debug, Clone)]
pub struct PretrainedEncoder {
    pub id: String,
    pub dim: usize,
    pub weights_ref: Option<String>,
}

impl TermEncoder for PretrainedEncoder {
    fn id(&self) -> &str {
        &self.id
    }
    fn kind(&self) -> &'static str {
        "pretrained"
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn encode(&self, _text: &str) -> Result<Vec<f32>, String> {
        Err(match &self.weights_ref {
            None => "pretrained encoder not available: no weights registered and no inference runtime linked (operator decision pending)".to_string(),
            Some(w) => format!("pretrained encoder not available: weights '{w}' registered but no inference runtime is linked into this build"),
        })
    }
}

/// PREPARED backend: an embedder trained from scratch / distilled on the real graph at a scale where that is meaningful (see the guide's
/// data reality). Same contract: `Err` until a trained artifact exists.
#[derive(Debug, Clone)]
pub struct EmbedderEncoder {
    pub id: String,
    pub dim: usize,
    pub artifact: Option<String>,
}

impl TermEncoder for EmbedderEncoder {
    fn id(&self) -> &str {
        &self.id
    }
    fn kind(&self) -> &'static str {
        "embedder"
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn encode(&self, _text: &str) -> Result<Vec<f32>, String> {
        Err("embedder not available: no trained artifact is registered as active".to_string())
    }
}

// ============================================================================================================
// 5. Learned ranker (pairwise logistic regression over the observed facts + encoder similarities)
// ============================================================================================================

const FACT_FEATURES: usize = 14;

fn ln1p_u(x: u64) -> f32 {
    ((x as f64) + 1.0).ln() as f32
}

/// Feature vector for one match: class one-hot (4), ln(1+matched_terms), validated, ln1p of applied/kept/packed/returned/dropped, observed hotness, in_scope,
/// graph proximity 1/(1+hops), then one cosine similarity per active encoder (`sims`).
pub fn rank_features(f: &RankFacts, sims: &[f32]) -> Vec<f32> {
    let mut x = Vec::with_capacity(FACT_FEATURES + sims.len());
    for c in [MatchClass::ExactName, MatchClass::ExactTerm, MatchClass::WholeToken, MatchClass::Substring] {
        x.push(if f.class == c { 1.0 } else { 0.0 });
    }
    x.push(ln1p_u(f.matched_terms as u64));
    x.push(if f.validated { 1.0 } else { 0.0 });
    x.push(ln1p_u(f.applied));
    x.push(ln1p_u(f.kept));
    x.push(ln1p_u(f.packed));
    x.push(ln1p_u(f.returned));
    x.push(ln1p_u(f.dropped));
    x.push(f.hotness);
    x.push(if f.in_scope { 1.0 } else { 0.0 });
    x.push(match f.hops {
        Some(h) => 1.0 / (1.0 + h as f32),
        None => 0.0,
    });
    x.extend_from_slice(sims);
    x
}

/// The learned ranker. Its output is a SORT KEY used internally to order candidates; it is never persisted or shown as a score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticRanker {
    pub id: String,
    pub version: u32,
    /// Number of encoder similarities this ranker was trained with — it is only usable with exactly that many.
    pub n_sims: usize,
    pub weights: Vec<f32>,
    pub trained_on_groups: u64,
    /// The ids of the encoders whose similarities fed this ranker, IN ORDER — the ranker is only used when exactly these encoders are available.
    #[serde(default)]
    pub encoder_ids: Vec<String>,
}

impl LogisticRanker {
    pub fn new(id: &str, n_sims: usize) -> Self {
        Self { id: id.to_string(), version: 1, n_sims, weights: vec![0.0; FACT_FEATURES + n_sims], trained_on_groups: 0, encoder_ids: Vec::new() }
    }

    pub fn sort_key(&self, x: &[f32]) -> f32 {
        self.weights.iter().zip(x.iter()).map(|(w, v)| w * v).sum()
    }

    /// Order `items` (id, facts, sims) best-first. `None` when the similarity count does not match what the ranker was trained with.
    pub fn rank(&self, items: &[(u64, RankFacts, Vec<f32>)]) -> Option<Vec<u64>> {
        if items.iter().any(|(_, _, s)| s.len() != self.n_sims) {
            return None;
        }
        let mut keyed: Vec<(usize, f32)> =
            items.iter().enumerate().map(|(i, (_, f, s))| (i, self.sort_key(&rank_features(f, s)))).collect();
        keyed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal).then(a.0.cmp(&b.0)));
        Some(keyed.into_iter().map(|(i, _)| items[i].0).collect())
    }
}

/// One query's candidates with labels: `(facts, sims, relevant)`; `group` identifies the query (e.g. trace id + step).
#[derive(Debug, Clone)]
pub struct RankGroup {
    pub group: String,
    pub items: Vec<(u64, RankFacts, Vec<f32>, bool)>,
}

/// Train a ranker with PAIRWISE logistic loss inside each group (every positive vs every negative; all pairs when ≤ `max_pairs_per_group`,
/// otherwise a deterministic sample). Groups with no positive or no negative carry no ordering signal and are skipped.
pub fn train_ranker(groups: &[RankGroup], n_sims: usize, epochs: usize, lr: f32, l2: f32, max_pairs_per_group: usize, seed: u64) -> LogisticRanker {
    let mut r = LogisticRanker::new("ranker", n_sims);
    let mut st = seed | 1;
    let usable: Vec<&RankGroup> = groups
        .iter()
        .filter(|g| g.items.iter().any(|i| i.3) && g.items.iter().any(|i| !i.3) && g.items.iter().all(|i| i.2.len() == n_sims))
        .collect();
    r.trained_on_groups = usable.len() as u64;
    for _ in 0..epochs {
        for g in &usable {
            let pos: Vec<usize> = (0..g.items.len()).filter(|&i| g.items[i].3).collect();
            let neg: Vec<usize> = (0..g.items.len()).filter(|&i| !g.items[i].3).collect();
            let total = pos.len() * neg.len();
            let mut pairs: Vec<(usize, usize)> = Vec::new();
            if total <= max_pairs_per_group.max(1) {
                for &p in &pos {
                    for &n in &neg {
                        pairs.push((p, n));
                    }
                }
            } else {
                for _ in 0..max_pairs_per_group.max(1) {
                    let p = pos[(xorshift(&mut st) % pos.len() as u64) as usize];
                    let n = neg[(xorshift(&mut st) % neg.len() as u64) as usize];
                    pairs.push((p, n));
                }
            }
            for (p, n) in pairs {
                let xp = rank_features(&g.items[p].1, &g.items[p].2);
                let xn = rank_features(&g.items[n].1, &g.items[n].2);
                let delta: f32 = r.weights.iter().zip(xp.iter().zip(xn.iter())).map(|(w, (a, b))| w * (a - b)).sum();
                let g_coef = -1.0 / (1.0 + delta.exp()); // d/d delta of log(1+exp(-delta))
                for i in 0..r.weights.len() {
                    r.weights[i] -= lr * (g_coef * (xp[i] - xn[i]) + l2 * r.weights[i]);
                }
            }
        }
    }
    r
}

// ============================================================================================================
// 6. Held-out evaluation and the yes/no PROMOTION rule
// ============================================================================================================

/// R-precision of an ordering: of the first R positions (R = number of relevant items), the fraction that are relevant.
pub fn r_precision(order_labels: &[bool]) -> Option<f64> {
    let r = order_labels.iter().filter(|l| **l).count();
    if r == 0 {
        return None;
    }
    let hit = order_labels.iter().take(r).filter(|l| **l).count();
    Some(hit as f64 / r as f64)
}

/// Exact one-sided binomial tail P(X ≥ k) for X ~ Binomial(n, 0.5).
pub fn sign_test_p(wins: u64, losses: u64) -> f64 {
    let n = wins + losses;
    if n == 0 {
        return 1.0;
    }
    // sum_{i=k}^{n} C(n,i) / 2^n, computed in log space for stability
    let mut ln_fact = vec![0.0f64; (n + 1) as usize];
    for i in 1..=n as usize {
        ln_fact[i] = ln_fact[i - 1] + (i as f64).ln();
    }
    let mut p = 0.0f64;
    for i in wins..=n {
        let ln_c = ln_fact[n as usize] - ln_fact[i as usize] - ln_fact[(n - i) as usize];
        p += (ln_c - (n as f64) * std::f64::consts::LN_2).exp();
    }
    p.min(1.0)
}

/// The recorded outcome of a held-out comparison between a candidate model ordering and the deterministic v0 ordering on the SAME groups.
/// Every number is measured; `promote` is the yes/no the registry acts on. No confidence anywhere.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionDecision {
    pub promote: bool,
    pub groups_evaluated: u64,
    pub wins: u64,
    pub losses: u64,
    pub ties: u64,
    pub p_value: f64,
    pub model_r_precision: f64,
    pub baseline_r_precision: f64,
    /// Why the answer is what it is (e.g. "no held-out group with both a positive and a negative").
    pub reason: String,
}

/// Compare `model_order` against `v0_order` per held-out group: a group is a WIN when the model's R-precision is higher, a LOSS when lower.
/// Promote iff wins > losses AND the exact sign-test p-value < `alpha` (0.05 by convention). `orders` = per group, the labels of the items in
/// (model order, v0 order).
pub fn decide_promotion(orders: &[(Vec<bool>, Vec<bool>)], alpha: f64) -> PromotionDecision {
    let (mut wins, mut losses, mut ties) = (0u64, 0u64, 0u64);
    let (mut sum_m, mut sum_b, mut n) = (0.0f64, 0.0f64, 0u64);
    for (m, b) in orders {
        let (Some(pm), Some(pb)) = (r_precision(m), r_precision(b)) else { continue };
        if m.iter().all(|l| *l) {
            continue; // no negative: carries no ordering signal
        }
        n += 1;
        sum_m += pm;
        sum_b += pb;
        if pm > pb + 1e-12 {
            wins += 1;
        } else if pb > pm + 1e-12 {
            losses += 1;
        } else {
            ties += 1;
        }
    }
    if n == 0 {
        return PromotionDecision {
            promote: false,
            groups_evaluated: 0,
            wins: 0,
            losses: 0,
            ties: 0,
            p_value: 1.0,
            model_r_precision: 0.0,
            baseline_r_precision: 0.0,
            reason: "no held-out group with both a relevant and an irrelevant candidate — nothing to evaluate on".to_string(),
        };
    }
    let p = sign_test_p(wins, losses);
    let promote = wins > losses && p < alpha;
    PromotionDecision {
        promote,
        groups_evaluated: n,
        wins,
        losses,
        ties,
        p_value: p,
        model_r_precision: sum_m / n as f64,
        baseline_r_precision: sum_b / n as f64,
        reason: if promote {
            "beats the deterministic v0 ordering on held-out real groups (exact sign test)".to_string()
        } else if wins <= losses {
            "does not beat the deterministic v0 ordering on held-out real groups".to_string()
        } else {
            "wins more often than it loses but not significantly — more held-out groups are needed".to_string()
        },
    }
}

// ============================================================================================================
// 7. Model registry (interchangeable backends, graph-citizen metadata) and training-pair helpers
// ============================================================================================================

/// One registered model. `status`: "inactive" | "validating" | "active" | "prepared" | "deprecated". `prepared` = slot exists, backend not
/// yet runnable (pretrained / embedder). `evaluation` is the MEASURED held-out comparison that justified the status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    /// "hashed_ngram" | "ranker" | "pretrained" | "embedder".
    pub kind: String,
    pub status: String,
    #[serde(default)]
    pub file_stem: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub dim: usize,
    #[serde(default)]
    pub trained_on_samples: u64,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub evaluation: Option<PromotionDecision>,
    #[serde(default)]
    pub notes: String,
    /// Container id of this model's graph citizen (`TraversalMLModel` container), once registered.
    #[serde(default)]
    pub container_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelRegistry {
    #[serde(default)]
    pub models: Vec<ModelEntry>,
}

impl ModelRegistry {
    pub fn path(dir: &Path) -> std::path::PathBuf {
        dir.join("registry.json")
    }

    pub fn load(dir: &Path) -> Self {
        std::fs::read_to_string(Self::path(dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let tmp = dir.join("registry.json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, Self::path(dir)).map_err(|e| e.to_string())
    }

    /// Insert or replace by id.
    pub fn upsert(&mut self, entry: ModelEntry) {
        match self.models.iter_mut().find(|m| m.id == entry.id) {
            Some(slot) => *slot = entry,
            None => self.models.push(entry),
        }
    }

    pub fn active_of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a ModelEntry> + 'a {
        self.models.iter().filter(move |m| m.kind == kind && m.status == "active")
    }

    /// Ensure the PREPARED slots (pretrained, embedder) exist so the next backend to come online has a place and a record.
    pub fn ensure_prepared_slots(&mut self) {
        for (id, kind, dim, note) in [
            (
                "pretrained-finetune",
                "pretrained",
                384usize,
                "prepared: fine-tune a small pretrained sentence encoder on Ozone's hit pairs — needs third-party weights (operator decision) and an inference runtime",
            ),
            (
                "embedder",
                "embedder",
                384usize,
                "prepared: a trained embedder (distilled/from scratch) once the store holds enough real labelled data",
            ),
        ] {
            if !self.models.iter().any(|m| m.id == id) {
                self.models.push(ModelEntry {
                    id: id.to_string(),
                    kind: kind.to_string(),
                    status: "prepared".to_string(),
                    file_stem: String::new(),
                    version: 0,
                    dim,
                    trained_on_samples: 0,
                    created_at: 0,
                    evaluation: None,
                    notes: note.to_string(),
                    container_id: None,
                });
            }
        }
    }
}

/// Load every ACTIVE, runnable encoder from the registry directory (hashed n-gram today; pretrained/embedder entries stay `prepared` and are
/// skipped until a runnable backend is registered as active).
pub fn load_active_encoders(dir: &Path) -> Vec<Box<dyn TermEncoder>> {
    let reg = ModelRegistry::load(dir);
    let mut out: Vec<Box<dyn TermEncoder>> = Vec::new();
    for m in reg.active_of_kind("hashed_ngram") {
        if let Ok(enc) = HashedNgramEncoder::load(dir, &m.file_stem) {
            out.push(Box::new(enc));
        }
    }
    for m in reg.active_of_kind("pretrained") {
        out.push(Box::new(PretrainedEncoder { id: m.id.clone(), dim: m.dim, weights_ref: if m.file_stem.is_empty() { None } else { Some(m.file_stem.clone()) } }));
    }
    for m in reg.active_of_kind("embedder") {
        out.push(Box::new(EmbedderEncoder { id: m.id.clone(), dim: m.dim, artifact: if m.file_stem.is_empty() { None } else { Some(m.file_stem.clone()) } }));
    }
    out
}

/// Load the active ranker (if any).
pub fn load_active_ranker(dir: &Path) -> Option<LogisticRanker> {
    let reg = ModelRegistry::load(dir);
    let m = reg.active_of_kind("ranker").next()?;
    let text = std::fs::read_to_string(dir.join(format!("{}.json", m.file_stem))).ok()?;
    serde_json::from_str(&text).ok()
}

/// A (query terms ↔ container terms) training pair with its group and label (see `train_encoder`).
#[derive(Debug, Clone)]
pub struct PairSample {
    pub group: String,
    pub query_terms: Vec<String>,
    pub container_terms: Vec<String>,
    pub label: bool,
}

/// Train a hashed n-gram encoder on labelled pairs for `epochs` passes (deterministic order shuffle). Returns the mean loss of the last epoch.
pub fn train_encoder(enc: &mut HashedNgramEncoder, samples: &[PairSample], epochs: usize, lr: f32, seed: u64) -> f32 {
    let prepared: Vec<(Vec<usize>, Vec<usize>, bool)> = samples
        .iter()
        .map(|s| (enc.features_of_terms(&s.query_terms), enc.features_of_terms(&s.container_terms), s.label))
        .collect();
    let mut order: Vec<usize> = (0..prepared.len()).collect();
    let mut st = seed | 1;
    let mut last_mean = 0.0f32;
    for _ in 0..epochs {
        for i in (1..order.len()).rev() {
            let j = (xorshift(&mut st) % (i as u64 + 1)) as usize;
            order.swap(i, j);
        }
        let mut sum = 0.0f32;
        for &i in &order {
            let (q, c, y) = &prepared[i];
            sum += enc.train_pair(q, c, *y, lr);
        }
        last_mean = if order.is_empty() { 0.0 } else { sum / order.len() as f32 };
    }
    enc.trained_on_samples = samples.len() as u64;
    enc.version += 1;
    last_mean
}

// ============================================================================================================
// 8. Ordering with interchangeable backends, the training dataset, and the train-and-evaluate routine
// ============================================================================================================

/// One search candidate: its id, its meaning terms (keywords + topics + name tokens as the caller supplies them) and its observed facts.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: u64,
    pub terms: Vec<String>,
    pub facts: RankFacts,
}

/// THE ordering used at inference AND at evaluation: the deterministic v0 order, plus one ordering per AVAILABLE encoder (by cosine
/// similarity, ties broken by v0 position), plus the learned ranker's ordering when it is usable with the encoders present — all combined by
/// reciprocal-rank fusion. An encoder that errors on any text (e.g. a prepared-but-not-runnable backend) is skipped entirely; a ranker whose
/// encoders are not all present is skipped. With nothing else available the result IS the v0 order. Returns ids best-first.
pub fn order_candidates(query_terms: &[String], cands: &[Candidate], encoders: &[&dyn TermEncoder], ranker: Option<&LogisticRanker>) -> Vec<u64> {
    let v0_items: Vec<(u64, RankFacts)> = cands.iter().map(|c| (c.id, c.facts)).collect();
    let v0 = rank_v0(&v0_items);
    if cands.len() < 2 || (encoders.is_empty() && ranker.is_none()) {
        return v0;
    }
    let v0_pos: BTreeMap<u64, usize> = v0.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let q_text = query_terms.join(" ");
    let mut rankings: Vec<Vec<u64>> = vec![v0.clone()];
    let mut sims_by_encoder: Vec<(String, Vec<f32>)> = Vec::new();
    for enc in encoders {
        let Ok(qv) = enc.encode(&q_text) else { continue };
        let mut sims: Vec<f32> = Vec::with_capacity(cands.len());
        let mut ok = true;
        for c in cands {
            match enc.encode(&c.terms.join(" ")) {
                Ok(cv) => sims.push(cosine(&qv, &cv)),
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        let mut idx: Vec<usize> = (0..cands.len()).collect();
        idx.sort_by(|&a, &b| {
            sims[b].partial_cmp(&sims[a]).unwrap_or(Ordering::Equal).then(v0_pos[&cands[a].id].cmp(&v0_pos[&cands[b].id]))
        });
        rankings.push(idx.iter().map(|&i| cands[i].id).collect());
        sims_by_encoder.push((enc.id().to_string(), sims));
    }
    if let Some(r) = ranker {
        let mut cols: Vec<&Vec<f32>> = Vec::new();
        let mut usable = true;
        for eid in &r.encoder_ids {
            match sims_by_encoder.iter().find(|(id, _)| id == eid) {
                Some((_, col)) => cols.push(col),
                None => {
                    usable = false;
                    break;
                }
            }
        }
        if usable && cols.len() == r.n_sims {
            let items: Vec<(u64, RankFacts, Vec<f32>)> =
                cands.iter().enumerate().map(|(i, c)| (c.id, c.facts, cols.iter().map(|col| col[i]).collect())).collect();
            if let Some(o) = r.rank(&items) {
                rankings.push(o);
            }
        }
    }
    if rankings.len() == 1 {
        v0
    } else {
        reciprocal_rank_fusion(&rankings)
    }
}

/// A labelled candidate in a training/evaluation group. Facts are a SNAPSHOT of what was known BEFORE the query ran (no label leakage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetCandidate {
    pub id: u64,
    pub terms: Vec<String>,
    pub facts: RankFacts,
    pub label: bool,
}

/// One query with its labelled candidates. `heldout` groups are the LAST in time and are never trained on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetGroup {
    pub group: String,
    pub heldout: bool,
    pub query_terms: Vec<String>,
    pub candidates: Vec<DatasetCandidate>,
}

pub fn write_dataset(path: &Path, groups: &[DatasetGroup]) -> Result<(), String> {
    let mut out = String::new();
    for g in groups {
        out.push_str(&serde_json::to_string(g).map_err(|e| e.to_string())?);
        out.push('\n');
    }
    let tmp = path.with_extension("jsonl.tmp");
    std::fs::write(&tmp, out).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn read_dataset(path: &Path) -> Result<Vec<DatasetGroup>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read dataset {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        out.push(serde_json::from_str::<DatasetGroup>(line).map_err(|e| format!("bad dataset line: {e}"))?);
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)] // a caller may override only some parameters (the E2E found a partial `params` object was rejected)
pub struct TrainParams {
    pub dim: usize,
    pub buckets: usize,
    pub encoder_epochs: usize,
    pub encoder_lr: f32,
    pub ranker_epochs: usize,
    pub ranker_lr: f32,
    pub ranker_l2: f32,
    pub max_pairs_per_group: usize,
    pub seed: u64,
    /// Significance level of the promotion sign test (0.05 by convention).
    pub alpha: f64,
}

impl Default for TrainParams {
    fn default() -> Self {
        Self {
            dim: HashedNgramEncoder::DEFAULT_DIM,
            buckets: HashedNgramEncoder::DEFAULT_BUCKETS,
            encoder_epochs: 8,
            encoder_lr: 0.2,
            ranker_epochs: 20,
            ranker_lr: 0.05,
            ranker_l2: 1e-4,
            max_pairs_per_group: 64,
            seed: 1,
            alpha: 0.05,
        }
    }
}

/// The outcome of one training run — every number measured on real groups; `activated` is the yes/no the registry acted on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainReport {
    pub trained: bool,
    pub train_groups: u64,
    pub heldout_groups: u64,
    pub labelled_pairs: u64,
    pub encoder_id: String,
    pub ranker_id: String,
    pub final_encoder_loss: f32,
    pub decision: PromotionDecision,
    pub activated: bool,
    pub models: Vec<ModelEntry>,
    pub message: String,
}

fn next_version(reg: &ModelRegistry, kind: &str) -> u32 {
    reg.models.iter().filter(|m| m.kind == kind).map(|m| m.version).max().unwrap_or(0) + 1
}

/// TRAIN the hashed n-gram encoder and the ranker on the TRAIN groups, EVALUATE the fused system (v0 + encoder + ranker, exactly as it would
/// run at search time) against the deterministic v0 ordering on the HELD-OUT groups, and register the result: both models become `active`
/// (replacing the previous active pair, which is marked `deprecated`) iff the held-out sign test says yes; otherwise they are saved as
/// `inactive` with the measured evaluation and the previous active models keep serving. Never fabricates a metric; refuses honestly when
/// there is nothing to learn from.
pub fn train_and_evaluate(groups: &[DatasetGroup], dir: &Path, now: u64, params: &TrainParams) -> Result<TrainReport, String> {
    let usable = |g: &&DatasetGroup| g.candidates.iter().any(|c| c.label) && g.candidates.iter().any(|c| !c.label);
    let train: Vec<&DatasetGroup> = groups.iter().filter(|g| !g.heldout).filter(usable).collect();
    let held: Vec<&DatasetGroup> = groups.iter().filter(|g| g.heldout).filter(usable).collect();
    let none = |msg: &str| PromotionDecision {
        promote: false,
        groups_evaluated: 0,
        wins: 0,
        losses: 0,
        ties: 0,
        p_value: 1.0,
        model_r_precision: 0.0,
        baseline_r_precision: 0.0,
        reason: msg.to_string(),
    };
    let mut reg = ModelRegistry::load(dir);
    reg.ensure_prepared_slots();
    if train.is_empty() {
        reg.save(dir)?;
        return Ok(TrainReport {
            trained: false,
            train_groups: 0,
            heldout_groups: held.len() as u64,
            labelled_pairs: 0,
            encoder_id: String::new(),
            ranker_id: String::new(),
            final_encoder_loss: 0.0,
            decision: none("no training group with both a relevant and an irrelevant candidate yet — keep collecting real hits"),
            activated: false,
            models: reg.models.clone(),
            message: "nothing trained: the hit log has no usable labelled groups yet".to_string(),
        });
    }
    // ---- encoder
    let mut pairs: Vec<PairSample> = Vec::new();
    for g in &train {
        for c in &g.candidates {
            if !c.terms.is_empty() && !g.query_terms.is_empty() {
                pairs.push(PairSample { group: g.group.clone(), query_terms: g.query_terms.clone(), container_terms: c.terms.clone(), label: c.label });
            }
        }
    }
    let enc_version = next_version(&reg, "hashed_ngram");
    let mut enc = HashedNgramEncoder::new(&format!("hashed-ngram-v{enc_version}"), params.dim, params.buckets);
    let loss = train_encoder(&mut enc, &pairs, params.encoder_epochs, params.encoder_lr, params.seed);
    enc.version = enc_version;
    // ---- ranker (similarity of the new encoder is its one learned input)
    let sim_of = |q: &[String], terms: &[String]| -> f32 {
        match (enc.encode(&q.join(" ")), enc.encode(&terms.join(" "))) {
            (Ok(a), Ok(b)) => cosine(&a, &b),
            _ => 0.0,
        }
    };
    let rank_groups: Vec<RankGroup> = train
        .iter()
        .map(|g| RankGroup {
            group: g.group.clone(),
            items: g.candidates.iter().map(|c| (c.id, c.facts, vec![sim_of(&g.query_terms, &c.terms)], c.label)).collect(),
        })
        .collect();
    let mut ranker = train_ranker(&rank_groups, 1, params.ranker_epochs, params.ranker_lr, params.ranker_l2, params.max_pairs_per_group, params.seed);
    let rk_version = next_version(&reg, "ranker");
    ranker.id = format!("ranker-v{rk_version}");
    ranker.version = rk_version;
    ranker.encoder_ids = vec![enc.id.clone()];
    // ---- held-out evaluation of the FUSED system vs v0
    let mut orders: Vec<(Vec<bool>, Vec<bool>)> = Vec::new();
    for g in &held {
        let cands: Vec<Candidate> = g.candidates.iter().map(|c| Candidate { id: c.id, terms: c.terms.clone(), facts: c.facts }).collect();
        let label_of: BTreeMap<u64, bool> = g.candidates.iter().map(|c| (c.id, c.label)).collect();
        let encs: [&dyn TermEncoder; 1] = [&enc];
        let fused = order_candidates(&g.query_terms, &cands, &encs, Some(&ranker));
        let v0_items: Vec<(u64, RankFacts)> = cands.iter().map(|c| (c.id, c.facts)).collect();
        let v0 = rank_v0(&v0_items);
        orders.push((
            fused.iter().map(|id| *label_of.get(id).unwrap_or(&false)).collect(),
            v0.iter().map(|id| *label_of.get(id).unwrap_or(&false)).collect(),
        ));
    }
    let decision = if held.is_empty() {
        none("no held-out groups with both a relevant and an irrelevant candidate — the models are saved but cannot be promoted yet")
    } else {
        decide_promotion(&orders, params.alpha)
    };
    // ---- persist artifacts (always: the evaluation must be inspectable) and update the registry
    enc.save(dir, &enc.id)?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.json", ranker.id)), serde_json::to_string_pretty(&ranker).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let status = if decision.promote { "active" } else { "inactive" };
    if decision.promote {
        for m in reg.models.iter_mut() {
            if (m.kind == "hashed_ngram" || m.kind == "ranker") && m.status == "active" {
                m.status = "deprecated".to_string();
            }
        }
    }
    reg.upsert(ModelEntry {
        id: enc.id.clone(),
        kind: "hashed_ngram".to_string(),
        status: status.to_string(),
        file_stem: enc.id.clone(),
        version: enc_version,
        dim: enc.dim,
        trained_on_samples: pairs.len() as u64,
        created_at: now,
        evaluation: Some(decision.clone()),
        notes: format!("trained on {} labelled pairs from {} groups; held-out {} groups", pairs.len(), train.len(), held.len()),
        container_id: None,
    });
    reg.upsert(ModelEntry {
        id: ranker.id.clone(),
        kind: "ranker".to_string(),
        status: status.to_string(),
        file_stem: ranker.id.clone(),
        version: rk_version,
        dim: 0,
        trained_on_samples: ranker.trained_on_groups,
        created_at: now,
        evaluation: Some(decision.clone()),
        notes: format!("pairwise logistic ranker over observed facts + 1 encoder similarity ({})", enc.id),
        container_id: None,
    });
    reg.save(dir)?;
    Ok(TrainReport {
        trained: true,
        train_groups: train.len() as u64,
        heldout_groups: held.len() as u64,
        labelled_pairs: pairs.len() as u64,
        encoder_id: enc.id.clone(),
        ranker_id: ranker.id.clone(),
        final_encoder_loss: loss,
        activated: decision.promote,
        message: decision.reason.clone(),
        decision,
        models: reg.models.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> String {
        x.to_string()
    }

    #[test]
    fn tokens_and_shape_need_no_list() {
        assert_eq!(tokens("crate::auth run_report"), vec!["crate", "auth", "run", "report"]);
        assert!(is_marker("scope:global") && is_marker("call:tool:chem_analyze") && is_marker("trace:7f26"));
        assert!(!is_marker("crate::auth") && !is_marker("http://example.com") && !is_marker("static analysis") && !is_marker("tool_call"));
        assert!(is_sentence_like("how the zz original works in two sentences") && !is_sentence_like("static analysis tools"));
        let m = meaning_terms(&[s("scope:global"), s("Static Analysis"), s("static analysis"), s("a b c d e f"), s("")]);
        assert_eq!(m, vec![s("Static Analysis")]);
    }

    #[test]
    fn whole_token_matching_kills_the_substring_flood() {
        let terms = vec![s("profile"), s("decode"), s("code review")];
        // substring would match profile/decode for "file"/"code"; whole-token only matches the real token
        assert!(match_info(&[s("file")], "x", &terms, false).is_none());
        let m = match_info(&[s("code")], "x", &terms, false).unwrap();
        assert_eq!(m.best, MatchClass::WholeToken);
        assert!(match_info(&[s("file")], "x", &terms, true).is_some()); // the legacy class is still reachable on request
        assert_eq!(match_info(&[s("code review")], "x", &terms, false).unwrap().best, MatchClass::ExactTerm);
        assert_eq!(match_info(&[s("Run Report")], "run report", &[], false).unwrap().best, MatchClass::ExactName);
        // a meaning query never matches a marker and vice versa
        assert!(match_info(&[s("global")], "x", &[s("scope:global")], false).is_none());
        assert!(match_info(&[s("scope:global")], "x", &[s("scope:global")], false).is_some());
    }

    #[test]
    fn v0_orders_by_observed_facts() {
        let mut a = RankFacts::new(MatchClass::ExactTerm, 1);
        let mut b = RankFacts::new(MatchClass::WholeToken, 3);
        assert_eq!(cmp_v0(&a, &b), Ordering::Less); // class beats term count
        b.class = MatchClass::ExactTerm;
        assert_eq!(cmp_v0(&a, &b), Ordering::Greater); // same class: more matched terms first
        a.matched_terms = 3;
        a.applied = 2;
        assert_eq!(cmp_v0(&a, &b), Ordering::Less); // then real hit history
        a.in_scope = false;
        assert_eq!(cmp_v0(&a, &b), Ordering::Greater); // scope dominates everything
        let items = vec![(1, b), (2, a), (3, RankFacts::new(MatchClass::Substring, 9))];
        // scope dominates: the out-of-scope item (2) ranks after even a substring match that is in scope (3)
        assert_eq!(rank_v0(&items), vec![1u64, 3, 2]);
    }

    #[test]
    fn observed_hotness_is_read_for_ranking() {
        let mut cold = RankFacts::new(MatchClass::ExactTerm, 1);
        let mut hot = RankFacts::new(MatchClass::ExactTerm, 1);
        cold.returned = 10;
        hot.returned = 10;
        hot.hotness = 0.8;
        assert_eq!(cmp_v0(&hot, &cold), Ordering::Less, "all else equal, the container whose finds were actually used ranks first");
        // hotness never overrides a stronger fact
        hot.class = MatchClass::WholeToken;
        assert_eq!(cmp_v0(&cold, &hot), Ordering::Less);
        assert_eq!(rank_v0(&[(1, cold), (2, { let mut h = cold; h.hotness = 0.5; h })]), vec![2, 1]);
    }

    #[test]
    fn fusion_combines_rankings_without_scales() {
        let fused = reciprocal_rank_fusion(&[vec![1, 2, 3], vec![3, 2, 1], vec![2, 3, 1]]);
        assert_eq!(fused[0], 2); // 2 is never worse than second
        assert_eq!(fused.len(), 3);
        assert_eq!(reciprocal_rank_fusion(&[vec![7, 8], vec![9]]), vec![7, 9, 8]);
    }

    #[test]
    fn encoder_is_deterministic_and_learns_a_pairing() {
        let a = HashedNgramEncoder::new("t", 16, 4096);
        let b = HashedNgramEncoder::new("t", 16, 4096);
        assert_eq!(a.encode("run_report").unwrap(), b.encode("run_report").unwrap());
        let mut enc = HashedNgramEncoder::new("t", 16, 4096);
        let pos = |q: &str, c: &[&str]| PairSample { group: "g".into(), query_terms: vec![s(q)], container_terms: c.iter().map(|x| s(x)).collect(), label: true };
        let neg = |q: &str, c: &[&str]| PairSample { group: "g".into(), query_terms: vec![s(q)], container_terms: c.iter().map(|x| s(x)).collect(), label: false };
        let samples = vec![
            pos("static analysis", &["code review", "linting"]),
            pos("analyse static code", &["linting", "code review"]),
            neg("static analysis", &["cooking recipe", "pasta"]),
            neg("analyse static code", &["pasta", "cooking recipe"]),
        ];
        let before = enc.similarity("static analysis", "linting code review").unwrap() - enc.similarity("static analysis", "pasta cooking recipe").unwrap();
        train_encoder(&mut enc, &samples, 60, 0.5, 7);
        let after = enc.similarity("static analysis", "linting code review").unwrap() - enc.similarity("static analysis", "pasta cooking recipe").unwrap();
        assert!(after > before, "training must pull the related pair closer than the unrelated one ({before} -> {after})");
        assert!(enc.trained_on_samples == 4 && enc.version == 2);
    }

    #[test]
    fn encoder_roundtrips_through_disk() {
        let dir = std::env::temp_dir().join(format!("kwm-test-{}", std::process::id()));
        let enc = HashedNgramEncoder::new("disk", 8, 512);
        enc.save(&dir, "m1").unwrap();
        let back = HashedNgramEncoder::load(&dir, "m1").unwrap();
        assert_eq!(enc.weights, back.weights);
        assert_eq!(back.dim, 8);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ranker_learns_that_applied_history_matters() {
        let mk = |id: u64, applied: u64, rel: bool| {
            let mut f = RankFacts::new(MatchClass::WholeToken, 1);
            f.applied = applied;
            (id, f, Vec::<f32>::new(), rel)
        };
        let groups: Vec<RankGroup> = (0..20)
            .map(|g| RankGroup { group: format!("g{g}"), items: vec![mk(1, 0, false), mk(2, 5, true), mk(3, 0, false), mk(4, 3, true)] })
            .collect();
        let r = train_ranker(&groups, 0, 30, 0.1, 0.0, 16, 3);
        let ranked = r.rank(&[(10, RankFacts::new(MatchClass::WholeToken, 1), vec![]), (11, { let mut f = RankFacts::new(MatchClass::WholeToken, 1); f.applied = 4; f }, vec![])]).unwrap();
        assert_eq!(ranked[0], 11);
        assert!(r.rank(&[(1, RankFacts::new(MatchClass::WholeToken, 1), vec![0.5])]).is_none(), "sim-count mismatch must refuse, not guess");
    }

    #[test]
    fn promotion_is_a_measured_yes_no() {
        assert!((sign_test_p(10, 0) - 1.0 / 1024.0).abs() < 1e-9);
        assert!((sign_test_p(0, 0) - 1.0).abs() < 1e-12);
        // 8 wins, 0 losses → significant; 3 wins 2 losses → not
        let win = (vec![true, false, false], vec![false, true, false]);
        let many: Vec<(Vec<bool>, Vec<bool>)> = (0..8).map(|_| win.clone()).collect();
        let d = decide_promotion(&many, 0.05);
        assert!(d.promote && d.wins == 8 && d.losses == 0 && d.model_r_precision > d.baseline_r_precision);
        let mixed: Vec<(Vec<bool>, Vec<bool>)> =
            (0..3).map(|_| win.clone()).chain((0..2).map(|_| (win.1.clone(), win.0.clone()))).collect();
        assert!(!decide_promotion(&mixed, 0.05).promote);
        let none = decide_promotion(&[], 0.05);
        assert!(!none.promote && none.groups_evaluated == 0);
    }

    #[test]
    fn train_and_evaluate_runs_and_registers_honestly() {
        let dir = std::env::temp_dir().join(format!("kwm-train-{}", std::process::id()));
        let mk = |id: u64, label: bool, term: &str| DatasetCandidate {
            id,
            terms: vec![term.to_string()],
            facts: RankFacts::new(MatchClass::WholeToken, 1),
            label,
        };
        let group = |n: usize, heldout: bool| DatasetGroup {
            group: format!("g{n}"),
            heldout,
            query_terms: vec!["static analysis".to_string()],
            candidates: vec![mk(1, true, "code review"), mk(2, false, "pasta recipe"), mk(3, true, "linting"), mk(4, false, "garden")],
        };
        let groups: Vec<DatasetGroup> = (0..12).map(|n| group(n, n >= 9)).collect();
        let small = TrainParams { dim: 8, buckets: 1024, encoder_epochs: 4, ranker_epochs: 5, ..TrainParams::default() };
        let report = train_and_evaluate(&groups, &dir, 1, &small).unwrap();
        assert!(report.trained && report.train_groups == 9 && report.heldout_groups == 3);
        assert!(ModelRegistry::load(&dir).models.iter().any(|m| m.id == report.encoder_id));
        // nothing to learn from → an honest refusal, models untouched
        let empty = train_and_evaluate(&[], &dir, 2, &small).unwrap();
        assert!(!empty.trained && !empty.activated);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn order_candidates_falls_back_to_v0_without_runnable_backends() {
        let c = |id: u64, class: MatchClass| Candidate { id, terms: vec!["x".into()], facts: RankFacts::new(class, 1) };
        let cands = vec![c(1, MatchClass::WholeToken), c(2, MatchClass::ExactTerm)];
        let pre = PretrainedEncoder { id: "p".into(), dim: 384, weights_ref: None };
        let encs: [&dyn TermEncoder; 1] = [&pre];
        assert_eq!(order_candidates(&["x".to_string()], &cands, &encs, None), vec![2, 1]);
        assert_eq!(order_candidates(&["x".to_string()], &cands, &[], None), vec![2, 1]);
    }

    #[test]
    fn prepared_backends_refuse_instead_of_fabricating() {
        let p = PretrainedEncoder { id: "p".into(), dim: 384, weights_ref: None };
        assert!(p.encode("x").is_err());
        let e = EmbedderEncoder { id: "e".into(), dim: 384, artifact: None };
        assert!(e.similarity("a", "b").is_err());
        let mut reg = ModelRegistry::default();
        reg.ensure_prepared_slots();
        reg.ensure_prepared_slots();
        assert_eq!(reg.models.len(), 2);
        assert!(reg.models.iter().all(|m| m.status == "prepared"));
    }
}
