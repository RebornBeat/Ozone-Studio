//! K-ALGORITHM REGISTRY — the typed facade over every swappable algorithm
//! family. Algorithms are registered BY KIND + NAME with selectable defaults:
//!
//! | Kind | Family | Shipped implementations |
//! |---|---|---|
//! | `validation` | assurance presets | `strict-5` (default), `lenient-3`, `paranoid-7` |
//! | `ordered_loop` | 1×1 loop discipline | `two-strikes` (default), `three-strikes` |
//! | `pairwise` | pairwise passes | `default` (window 8 / 50 pairs), `wide` (16 / 100) |
//! | `convergence` | refinement bounds | `fast` (2 passes, default), `deep` (5) |
//! | `search` | store search strategies | `scan` (default), `scan-legacy` (preserved pre-fix behavior), `exact` — trait registry, extensible |
//!
//! Store backends (StoreAccess) and wire protocols (pipeline-9 adapters) have
//! their own contracts — see docs/CONTRACTS.md.
//!
//! Taxonomy + preset registry live in shared/contracts (std-only, embedded by
//! pipelines too); this facade assembles the families host-side with the
//! process-global instance.

use std::sync::{Arc, RwLock};

use crate::shared_contracts::k_loops::{
    ConvergencePolicy, Ordered1x1Policy, PairwisePolicy,
};
use crate::shared_contracts::k_registry::{KAlgorithmKind, NamedPresets};
use crate::shared_contracts::k_validation::ValidationPolicy;
use crate::zsei::search::SearchRegistry;

/// The typed algorithm registry. One instance per process (see [`global`]).
/// Fields are `RwLock`-wrapped so `set_default` can actually be called at
/// runtime (config load, `/config/set`) — the registry existed with a
/// `set_default` API long before anything could reach it through the
/// `&'static` global. Only `convergence` and `pairwise` currently have a
/// live consumer (`orchestrator/amt.rs`); `validation`/`ordered_loop` are
/// wrapped for structural consistency but changing their default has no
/// observable effect yet — not exposed in config/UI for that reason.
pub struct KAlgorithms {
    /// `validation` presets: strength = consecutive affirmations required.
    pub validation: RwLock<NamedPresets<ValidationPolicy>>,
    /// `ordered_loop` presets: misses before exhaustion.
    pub ordered_loop: RwLock<NamedPresets<Ordered1x1Policy>>,
    /// `pairwise` presets: forward window + pair cap.
    pub pairwise: RwLock<NamedPresets<PairwisePolicy>>,
    /// `convergence` presets: max refinement passes.
    pub convergence: RwLock<NamedPresets<ConvergencePolicy>>,
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
            validation: RwLock::new(validation),
            ordered_loop: RwLock::new(ordered_loop),
            pairwise: RwLock::new(pairwise),
            convergence: RwLock::new(convergence),
            search: Arc::new(SearchRegistry::new()),
        }
    }

    /// Process-global instance. Selection is data-driven at runtime via
    /// `set_convergence_preset`/`set_pairwise_preset` (called from
    /// OzoneConfig.k_algorithms at boot and on `/config/set`).
    pub fn global() -> &'static KAlgorithms {
        static GLOBAL: std::sync::OnceLock<KAlgorithms> = std::sync::OnceLock::new();
        GLOBAL.get_or_init(KAlgorithms::new)
    }

    /// Switch the live default convergence preset. Returns false (no-op) if
    /// `name` isn't a registered preset — never panics.
    pub fn set_convergence_preset(&self, name: &str) -> bool {
        self.convergence.write().map(|mut p| p.set_default(name)).unwrap_or(false)
    }

    /// Switch the live default pairwise preset. Returns false (no-op) if
    /// `name` isn't a registered preset.
    pub fn set_pairwise_preset(&self, name: &str) -> bool {
        self.pairwise.write().map(|mut p| p.set_default(name)).unwrap_or(false)
    }

    /// Current default preset names, for `/config/get` and Settings —
    /// (convergence_default, pairwise_default).
    pub fn current_presets(&self) -> (String, String) {
        let convergence = self
            .convergence
            .read()
            .map(|p| p.default_name().to_string())
            .unwrap_or_else(|_| "fast".to_string());
        let pairwise = self
            .pairwise
            .read()
            .map(|p| p.default_name().to_string())
            .unwrap_or_else(|_| "default".to_string());
        (convergence, pairwise)
    }

    /// Every registered preset name per exposed family, for the Settings
    /// dropdown options.
    pub fn available_presets(&self) -> (Vec<String>, Vec<String>) {
        let convergence = self
            .convergence
            .read()
            .map(|p| p.names().into_iter().map(String::from).collect())
            .unwrap_or_default();
        let pairwise = self
            .pairwise
            .read()
            .map(|p| p.names().into_iter().map(String::from).collect())
            .unwrap_or_default();
        (convergence, pairwise)
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
        // Presets became runtime-switchable (RwLock-wrapped) — lock for reads.
        let validation = k.validation.read().unwrap();
        let ordered_loop = k.ordered_loop.read().unwrap();
        let pairwise = k.pairwise.read().unwrap();
        let convergence = k.convergence.read().unwrap();
        assert_eq!(
            validation.get(Some("strict-5")).map(|p| p.strength),
            Some(5)
        );
        assert_eq!(ordered_loop.default_preset().exhaustion_strikes, 2);
        assert_eq!(pairwise.get(Some("wide")).map(|p| p.forward_window), Some(16));
        assert_eq!(convergence.get(Some("deep")).map(|p| p.max_passes), Some(5));
    }

    #[test]
    fn kinds_cover_every_family() {
        let k = KAlgorithms::new();
        let validation = k.validation.read().unwrap();
        let ordered_loop = k.ordered_loop.read().unwrap();
        let pairwise = k.pairwise.read().unwrap();
        let convergence = k.convergence.read().unwrap();
        assert_eq!(validation.kind(), KAlgorithmKind::Validation);
        assert_eq!(ordered_loop.kind(), KAlgorithmKind::OrderedLoop);
        assert_eq!(pairwise.kind(), KAlgorithmKind::Pairwise);
        assert_eq!(convergence.kind(), KAlgorithmKind::Convergence);
    }
}
