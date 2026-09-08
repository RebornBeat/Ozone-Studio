//! K-ALGORITHM REGISTRY — the typed facade over every swappable algorithm
//! family. Algorithms are registered BY KIND + NAME with selectable defaults:
//!
//! | Kind | Family | Shipped implementations |
//! |---|---|---|
//! | `validation` | assurance presets | `strict-5` (default), `lenient-3`, `paranoid-7` |
//! | `ordered_loop` | 1×1 loop discipline | `two-strikes` (default), `three-strikes` |
//! | `pairwise` | pairwise passes | `default` (window 8 / 50 pairs), `wide` (16 / 100) |
//! | `convergence` | refinement bounds | `fast` (2 passes, default), `deep` (5) |
//! | `search` | store search strategies | `scan` (default), `exact` — trait registry, extensible |
//!
//! Store backends (StoreAccess) and wire protocols (pipeline-9 adapters) have
//! their own contracts — see docs/CONTRACTS.md.
//!
//! Taxonomy + preset registry live in shared/contracts (std-only, embedded by
//! pipelines too); this facade assembles the families host-side with the
//! process-global instance.

use std::sync::Arc;

use crate::shared_contracts::k_loops::{
    ConvergencePolicy, Ordered1x1Policy, PairwisePolicy,
};
use crate::shared_contracts::k_registry::{KAlgorithmKind, NamedPresets};
use crate::shared_contracts::k_validation::ValidationPolicy;
use crate::zsei::search::SearchRegistry;

/// The typed algorithm registry. One instance per process (see [`global`]).
pub struct KAlgorithms {
    /// `validation` presets: strength = consecutive affirmations required.
    pub validation: NamedPresets<ValidationPolicy>,
    /// `ordered_loop` presets: misses before exhaustion.
    pub ordered_loop: NamedPresets<Ordered1x1Policy>,
    /// `pairwise` presets: forward window + pair cap.
    pub pairwise: NamedPresets<PairwisePolicy>,
    /// `convergence` presets: max refinement passes.
    pub convergence: NamedPresets<ConvergencePolicy>,
    /// `search` strategies — trait-object family (scan/exact/…).
    pub search: Arc<SearchRegistry>,
}

impl KAlgorithms {
    /// Build with the shipped presets + strategies.
    pub fn new() -> Self {
        let mut validation =
            NamedPresets::new(KAlgorithmKind::Validation, "strict-5", ValidationPolicy { strength: 5 });
        validation.register("lenient-3", ValidationPolicy { strength: 3 });
        validation.register("paranoid-7", ValidationPolicy { strength: 7 });

        let mut ordered_loop = NamedPresets::new(
            KAlgorithmKind::OrderedLoop,
            "two-strikes",
            Ordered1x1Policy { exhaustion_strikes: 2 },
        );
        ordered_loop.register("three-strikes", Ordered1x1Policy { exhaustion_strikes: 3 });

        let mut pairwise = NamedPresets::new(
            KAlgorithmKind::Pairwise,
            "default",
            PairwisePolicy { forward_window: 8, max_pairs: 50 },
        );
        pairwise.register(
            "wide",
            PairwisePolicy { forward_window: 16, max_pairs: 100 },
        );

        let mut convergence =
            NamedPresets::new(KAlgorithmKind::Convergence, "fast", ConvergencePolicy { max_passes: 2 });
        convergence.register("deep", ConvergencePolicy { max_passes: 5 });

        Self {
            validation,
            ordered_loop,
            pairwise,
            convergence,
            search: Arc::new(SearchRegistry::new()),
        }
    }

    /// Process-global instance. Config overrides (CONFIG REVIEW todo) apply
    /// on first init; selection afterwards is data-driven.
    pub fn global() -> &'static KAlgorithms {
        static GLOBAL: std::sync::OnceLock<KAlgorithms> = std::sync::OnceLock::new();
        GLOBAL.get_or_init(KAlgorithms::new)
    }
}

impl Default for KAlgorithms {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_presets_resolve() {
        let k = KAlgorithms::new();
        assert_eq!(
            k.validation.get(Some("strict-5")).map(|p| p.strength),
            Some(5)
        );
        assert_eq!(
            k.ordered_loop.default_preset().exhaustion_strikes,
            2
        );
        assert_eq!(
            k.pairwise.get(Some("wide")).map(|p| p.forward_window),
            Some(16)
        );
        assert_eq!(
            k.convergence.get(Some("deep")).map(|p| p.max_passes),
            Some(5)
        );
    }

    #[test]
    fn kinds_cover_every_family() {
        let k = KAlgorithms::new();
        assert_eq!(k.validation.kind(), KAlgorithmKind::Validation);
        assert_eq!(k.ordered_loop.kind(), KAlgorithmKind::OrderedLoop);
        assert_eq!(k.pairwise.kind(), KAlgorithmKind::Pairwise);
        assert_eq!(k.convergence.kind(), KAlgorithmKind::Convergence);
    }
}
