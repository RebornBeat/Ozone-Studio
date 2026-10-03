//! K-ALGORITHM REGISTRY TAXONOMY — the kinds algorithms register under.
//!
//! Everything interchangeable is registered BY KIND (search, validation,
//! ordered loops, pairwise passes, convergence, ...) — never hardcoded, never
//! nameless. This file is shared (std-only) so host and pipelines agree on
//! the taxonomy; strategy implementations register under their kind.
//!
//! Guarantees (project invariants):
//! - Registration is by KIND + NAME; one default per kind; unknown names fall
//!   back to the default rather than erroring.
//! - Selection is data (a name), swappable at runtime or by config.
//! - Nothing here fabricates scores — strategies return measured results.

/// The algorithm families. Every swappable algorithm belongs to exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KAlgorithmKind {
    /// Assurance primitives: consecutive YES/NO validation, n-of-m voting, …
    Validation,
    /// Ordered 1×1 loop disciplines: exhaustion strikes, retry/escalation.
    OrderedLoop,
    /// Pairwise pass shapes: forward windows, pair caps.
    Pairwise,
    /// Convergence bounds: max passes of refinement loops.
    Convergence,
    /// Store search strategies: keyword scan, exact match, traversal,
    /// embedding-based, …
    Search,
    /// Extraction strategies (reserved: entity/topic extraction variants).
    Extraction,
    /// Model-call strategies (reserved: rendering ladders, prompt shapes).
    ModelCall,
    /// Model-call watchdogs: per-call timeout policies — a hung call must
    /// fail LOUDLY after the policy's budget, never freeze its caller
    /// (doctrine #35's other half: no invisible work either).
    Watchdog,
    /// Relationship-path relevance: how new graphs link into the living
    /// graph (neighborhood walk depth, per-scope shared-term floors).
    Relevance,
    /// Candidate selection shapes: how a ranked subset is chosen from a
    /// larger candidate set (pairwise-ranked batching vs per-candidate
    /// passes) — e.g. methodology selection for AMT branch discovery.
    Selection,
}

impl KAlgorithmKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            KAlgorithmKind::Validation => "validation",
            KAlgorithmKind::OrderedLoop => "ordered_loop",
            KAlgorithmKind::Pairwise => "pairwise",
            KAlgorithmKind::Convergence => "convergence",
            KAlgorithmKind::Search => "search",
            KAlgorithmKind::Extraction => "extraction",
            KAlgorithmKind::ModelCall => "model_call",
            KAlgorithmKind::Watchdog => "watchdog",
            KAlgorithmKind::Relevance => "relevance",
            KAlgorithmKind::Selection => "selection",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "validation" => KAlgorithmKind::Validation,
            "ordered_loop" => KAlgorithmKind::OrderedLoop,
            "pairwise" => KAlgorithmKind::Pairwise,
            "convergence" => KAlgorithmKind::Convergence,
            "search" => KAlgorithmKind::Search,
            "extraction" => KAlgorithmKind::Extraction,
            "model_call" => KAlgorithmKind::ModelCall,
            "watchdog" => KAlgorithmKind::Watchdog,
            "relevance" => KAlgorithmKind::Relevance,
            "selection" => KAlgorithmKind::Selection,
            _ => return None,
        })
    }
}

/// A named-preset registry for value-type algorithms (policies, presets).
/// Strategy families with complex behavior use trait-object registries
/// instead (e.g. the host's SearchRegistry); this covers the Copy/Clone
/// policy families so their magic numbers live as NAMED presets, selectable
/// by name, never scattered through call sites.
///
/// std-only by design: pipelines embed it via `#[path]` like every shared
/// contract.
/// How a candidate set is processed (operator, 2026-10-02: "why cap to 8?
/// we really shouldn't — batch the methodologies, they're small, multiple
/// fit in a call ... ensure search is optimized to run through it ALL").
/// EVERY candidate participates: relevance-ranked for order, then chunked
/// into batches of `batch_size`, one pass per batch — full coverage, a
/// fraction of the calls. `batch_size: 1` = the preserved per-candidate
/// legacy shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionPolicy {
    pub batch_size: usize,
}

/// Per-model-call watchdog budget. `u64::MAX` disables (never recommended —
/// a hung call then freezes its caller forever; found live in the Stage-4b
/// AMT branch-discovery hang, 2026-10-01).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WatchdogPolicy {
    pub per_call_timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct NamedPresets<T: Clone> {
    kind: KAlgorithmKind,
    presets: Vec<(String, T)>,
    default: String,
}

impl<T: Clone> NamedPresets<T> {
    pub fn new(kind: KAlgorithmKind, default_name: &str, default_value: T) -> Self {
        Self {
            kind,
            presets: vec![(default_name.to_string(), default_value)],
            default: default_name.to_string(),
        }
    }

    /// Register (or replace) a named preset.
    pub fn register(&mut self, name: &str, value: T) {
        if let Some(slot) = self.presets.iter_mut().find(|(n, _)| n == name) {
            slot.1 = value;
        } else {
            self.presets.push((name.to_string(), value));
        }
    }

    /// Select the default preset by name. Returns false if unknown (the
    /// previous default stays — selection failures never panic).
    pub fn set_default(&mut self, name: &str) -> bool {
        if self.presets.iter().any(|(n, _)| n == name) {
            self.default = name.to_string();
            true
        } else {
            false
        }
    }

    /// Resolve a preset: named if present, else the default.
    pub fn get(&self, name: Option<&str>) -> Option<&T> {
        let name = name.unwrap_or(&self.default);
        self.presets.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// The default preset (always present — the constructor seeds one).
    pub fn default_preset(&self) -> &T {
        self.get(None).expect("default preset is always registered")
    }

    pub fn kind(&self) -> KAlgorithmKind {
        self.kind
    }

    /// Name of the currently-selected default preset.
    pub fn default_name(&self) -> &str {
        &self.default
    }

    pub fn names(&self) -> Vec<&str> {
        self.presets.iter().map(|(n, _)| n.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_register_select_and_fall_back() {
        let mut p: NamedPresets<u32> =
            NamedPresets::new(KAlgorithmKind::Validation, "strict-5", 5);
        p.register("lenient-3", 3);

        assert_eq!(p.get(None).copied(), Some(5)); // default
        assert_eq!(p.get(Some("lenient-3")).copied(), Some(3));
        assert!(p.set_default("lenient-3"));
        assert_eq!(p.get(None).copied(), Some(3));
        assert!(!p.set_default("nope")); // unknown name keeps the old default
        assert_eq!(p.get(Some("nope")).copied(), None); // unknown query → None
        assert_eq!(p.kind(), KAlgorithmKind::Validation);
    }

    #[test]
    fn taxonomy_round_trips() {
        for kind in [
            KAlgorithmKind::Validation,
            KAlgorithmKind::OrderedLoop,
            KAlgorithmKind::Pairwise,
            KAlgorithmKind::Convergence,
            KAlgorithmKind::Search,
            KAlgorithmKind::Extraction,
            KAlgorithmKind::ModelCall,
            KAlgorithmKind::Watchdog,
            KAlgorithmKind::Selection,
        ] {
            assert_eq!(KAlgorithmKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(KAlgorithmKind::parse("bogus"), None);
    }
}
