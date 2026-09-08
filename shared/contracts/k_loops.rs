//! K-ALGORITHMS: loop-discipline policies — ordered 1×1 loops, pairwise
//! passes, convergence bounds.
//!
//! Single source of truth, included by the host crate AND pipeline crates
//! via `#[path]` (the k_validation pattern). The loop BODIES stay contextual
//! (guards, carries, scan offsets differ per loop); the DISCIPLINE — how many
//! misses mean exhaustion, how wide a pairwise window reaches, how many pairs
//! are allowed, when a convergence loop must stop — is policy registered here,
//! not magic numbers scattered in bodies.

/// Ordered 1×1 loop policy: extract→validate→advance loops exhaust after N
/// consecutive misses (two-strikes by default).
#[derive(Debug, Clone, Copy)]
pub struct Ordered1x1Policy {
    pub exhaustion_strikes: u32,
}

impl Default for Ordered1x1Policy {
    fn default() -> Self {
        Self { exhaustion_strikes: 2 }
    }
}

/// Tracker for an ordered loop's miss streak. `hit()` records progress,
/// `miss()` records an empty attempt and reports whether the exhaustion
/// threshold is reached.
#[derive(Debug, Clone, Default)]
pub struct ExhaustionTracker {
    strikes: u32,
    policy: Ordered1x1Policy,
}

impl ExhaustionTracker {
    pub fn new(policy: Ordered1x1Policy) -> Self {
        Self { strikes: 0, policy }
    }

    /// A miss: increment the streak. Returns true when exhausted.
    pub fn miss(&mut self) -> bool {
        self.strikes += 1;
        self.strikes >= self.policy.exhaustion_strikes
    }

    /// Progress: reset the streak.
    pub fn hit(&mut self) {
        self.strikes = 0;
    }

    pub fn strikes(&self) -> u32 {
        self.strikes
    }
}

/// Pairwise pass policy: bounded forward window (how far ahead a pair search
/// reaches) and a hard cap on pairs examined (combinatorial guard).
#[derive(Debug, Clone, Copy)]
pub struct PairwisePolicy {
    pub forward_window: usize,
    pub max_pairs: usize,
}

impl Default for PairwisePolicy {
    fn default() -> Self {
        Self { forward_window: 8, max_pairs: 50 }
    }
}

/// Convergence loop policy: bounded passes (safety ceiling).
#[derive(Debug, Clone, Copy)]
pub struct ConvergencePolicy {
    pub max_passes: u32,
}

impl Default for ConvergencePolicy {
    fn default() -> Self {
        Self { max_passes: 2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustion_two_strikes_default() {
        let mut t = ExhaustionTracker::default();
        assert!(!t.miss());
        assert!(t.miss()); // second consecutive miss exhausts
    }

    #[test]
    fn hit_resets_streak() {
        let mut t = ExhaustionTracker::default();
        assert!(!t.miss());
        t.hit();
        assert!(!t.miss()); // streak reset — not exhausted
        assert!(t.miss());
    }
}
