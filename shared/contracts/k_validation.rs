//! K-ALGORITHM: consecutive YES/NO validation — the assurance primitive.
//!
//! Single source of truth, included by the host crate AND pipeline crates via
//! `#[path]` (the ozone_serve pattern). Pure logic + policy live here; the
//! executor call stays contextual (each side meters through its own path).
//!
//! Policy note: strength (5 consecutive) is a parameter, not a law of nature —
//! this is the seed of the K-algorithm registry where validation strategies
//! are registered and selectable.

use serde_json::Value;

/// Default assurance strength: N consecutive affirmations required.
pub const DEFAULT_STRENGTH: u32 = 5;

/// THE vote strength every yes/no validation uses: the host exports the live registry preset as
/// `OZONE_VALIDATION_STRENGTH` (config `[k_algorithms] validation_preset` — the preset LADDER:
/// "paranoid-7" / "strict-5" (default) / "lenient-3" / "light-2" / "flash-1"); pipelines inherit it.
/// Falls back to `DEFAULT_STRENGTH` when unset (standalone runs, tests). Never 0.
pub fn configured_strength() -> u32 {
    std::env::var("OZONE_VALIDATION_STRENGTH")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(DEFAULT_STRENGTH)
}

/// Preset name → vote count. The registry and the config both speak these names.
/// Returns None for an unknown name (callers keep their own default).
pub fn preset_strength(name: &str) -> Option<u32> {
    let n = name.trim().to_ascii_lowercase();
    match n.as_str() {
        "paranoid-7" | "paranoid" => Some(7),
        "strict-5" | "strict" => Some(5),
        "lenient-3" | "lenient" => Some(3),
        "light-2" | "light" => Some(2),
        "flash-1" | "flash" => Some(1),
        _ => None,
    }
}

/// Selector for validation strategy strength (registry seed).
#[derive(Debug, Clone, Copy)]
pub struct ValidationPolicy {
    pub strength: u32,
}

impl Default for ValidationPolicy {
    fn default() -> Self {
        Self { strength: DEFAULT_STRENGTH }
    }
}

/// Sampling temperature for validation votes: 0.0 — a yes/no vote on a correct event should come back the same every time; noise would
/// only cause spurious NOs (rejections and re-proposals). (A 0.5 setting was tried and reverted on operator ruling, 2026-11-06.)
pub const VOTE_TEMPERATURE: f64 = 0.0;

/// The strict one-word YES/NO instruction. Delivered as `system_prompt` — the key the prompt pipeline's `PromptInput` actually reads
/// (it was `system_context`, which `PromptInput` never had: serde dropped it, so every vote went out WITHOUT the "answer with one
/// JSON object" instruction and small/reasoning models wrote prose, which parses as non-affirming).
pub const YES_NO_INSTRUCTION: &str = "Answer strictly with one JSON object: {\"answer\": \"YES\"} or {\"answer\": \"NO\"}. No explanation. No markdown.";

/// Build the strict one-word YES/NO request (the standardized question form). The question itself is sent WHOLE; the instruction is
/// ALSO appended to the prompt so a backend that ignores system prompts still receives it.
pub fn yes_no_input(question: &str) -> Value {
    let mut input = serde_json::json!({
        "prompt": format!("{question}\n\n{YES_NO_INSTRUCTION}"),
        "max_tokens": 20,
        "temperature": VOTE_TEMPERATURE,
        "system_prompt": YES_NO_INSTRUCTION
    });
    // `[optimization] vote_reasoning = "off"` (env OZONE_OPT_VOTE_REASONING=off): ask the provider not to spend hidden reasoning on a
    // one-word vote (measured median ~1,000 tokens per vote). Default = provider default (no `reasoning` key sent).
    if std::env::var("OZONE_OPT_VOTE_REASONING").map(|v| v.eq_ignore_ascii_case("off")).unwrap_or(false) {
        input["reasoning"] = serde_json::json!({"enabled": false});
    }
    input
}

/// Parse a YES/NO response body. None = unparseable (treated as non-affirming).
pub fn parse_yes_no(raw: &str) -> Option<bool> {
    let trimmed = raw.trim();
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end < start {
        return None;
    }
    let parsed: Value = serde_json::from_str(&trimmed[start..=end]).ok()?;
    match parsed.get("answer").and_then(|a| a.as_str()) {
        Some(a) if a.eq_ignore_ascii_case("YES") => Some(true),
        Some(a) if a.eq_ignore_ascii_case("NO") => Some(false),
        _ => None,
    }
}

/// OPTIMIZED ROUTE switch (config `[optimization] concurrent_votes` / `profile = "high_end"`, exported by the host as
/// `OZONE_OPT_CONCURRENT_VOTES=1`). Standard route (default, local models, constrained setups) = fully sequential.
pub fn optimized_votes_enabled() -> bool {
    std::env::var("OZONE_OPT_CONCURRENT_VOTES").map(|v| v == "1").unwrap_or(false)
}

/// K-ALG: THE consecutive-vote primitive — ONE implementation for the host (AMT validation) and every pipeline (the text modality's
/// 5x order/boundary checks). Requires `policy.strength` answers that ALL equal `want`; the FIRST vote always gates (a first
/// non-match stops after ONE call, on both routes). STANDARD route: the remaining votes run one after another with early abort.
/// OPTIMIZED route (`optimized_votes_enabled()`): when exactly 4 votes remain (strength 5, the strict default) they run
/// CONCURRENTLY — identical, independent asks, same all-must-agree decision. `ask` is the contextual, metered oracle (one call per
/// invocation, question already closed over).
pub async fn confirm_consecutive_with<F, Fut>(mut ask: F, policy: &ValidationPolicy, want: bool) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<bool>>,
{
    if policy.strength == 0 {
        return true;
    }
    if ask().await != Some(want) {
        return false;
    }
    let rest = policy.strength - 1;
    if optimized_votes_enabled() && rest == 4 {
        let (a, b, c, d) = tokio::join!(ask(), ask(), ask(), ask());
        return [a, b, c, d].iter().all(|x| *x == Some(want));
    }
    for _ in 0..rest {
        if ask().await != Some(want) {
            return false;
        }
    }
    true
}

/// K-ALG: require `policy.strength` CONSECUTIVE YES answers (see `confirm_consecutive_with`).
pub async fn confirm_consecutive_yes_with<F, Fut>(ask: F, policy: &ValidationPolicy) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<bool>>,
{
    confirm_consecutive_with(ask, policy, true).await
}

/// K-ALG: require `policy.strength` CONSECUTIVE NO answers (see `confirm_consecutive_with`).
pub async fn confirm_consecutive_no_with<F, Fut>(ask: F, policy: &ValidationPolicy) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<bool>>,
{
    confirm_consecutive_with(ask, policy, false).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn one_primitive_both_routes_same_decision_and_first_vote_gates() {
        // the env switch is process-global: exercise both routes sequentially inside ONE test
        for optimized in ["0", "1"] {
            std::env::set_var("OZONE_OPT_CONCURRENT_VOTES", optimized);
            let policy = ValidationPolicy::default();
            let calls = std::sync::atomic::AtomicU32::new(0);
            let all_yes = confirm_consecutive_with(
                || {
                    calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async { Some(true) }
                },
                &policy,
                true,
            )
            .await;
            assert!(all_yes);
            assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), DEFAULT_STRENGTH, "route {optimized}: a pass costs exactly N votes");
            let calls = std::sync::atomic::AtomicU32::new(0);
            let first_no = confirm_consecutive_with(
                || {
                    calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async { Some(false) }
                },
                &policy,
                true,
            )
            .await;
            assert!(!first_no);
            assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1, "route {optimized}: the first vote gates");
            let n = std::sync::atomic::AtomicU32::new(0);
            let late_no = confirm_consecutive_with(
                || {
                    let k = n.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move { Some(k != 3) }
                },
                &policy,
                true,
            )
            .await;
            assert!(!late_no, "route {optimized}: any dissent fails the check");
        }
        std::env::remove_var("OZONE_OPT_CONCURRENT_VOTES");
    }

    #[test]
    fn configured_strength_reads_env_and_never_returns_zero() {
        // OZONE_VALIDATION_STRENGTH is read by nothing else in this crate's tests
        std::env::remove_var("OZONE_VALIDATION_STRENGTH");
        assert_eq!(configured_strength(), DEFAULT_STRENGTH);
        std::env::set_var("OZONE_VALIDATION_STRENGTH", "3");
        assert_eq!(configured_strength(), 3);
        std::env::set_var("OZONE_VALIDATION_STRENGTH", "0");
        assert_eq!(configured_strength(), DEFAULT_STRENGTH, "0 is never a valid strength");
        std::env::set_var("OZONE_VALIDATION_STRENGTH", "many");
        assert_eq!(configured_strength(), DEFAULT_STRENGTH);
        std::env::remove_var("OZONE_VALIDATION_STRENGTH");
    }

    #[test]
    fn yes_no_input_uses_keys_the_prompt_pipeline_reads() {
        let v = yes_no_input("Is the sky blue?");
        // only PromptInput keys: no dropped fields
        for k in v.as_object().unwrap().keys() {
            assert!(["prompt", "system_prompt", "temperature", "max_tokens", "reasoning"].contains(&k.as_str()), "key {k} would be silently dropped");
        }
        assert!(v["prompt"].as_str().unwrap().starts_with("Is the sky blue?"));
        assert!(v["system_prompt"].as_str().unwrap().contains("\"answer\""));
        assert_eq!(v["temperature"].as_f64().unwrap(), 0.0, "yes/no votes are deterministic: no sampling noise");
    }

    #[tokio::test]
    async fn yes_strength_requires_consecutive() {
        let mut calls = 0u32;
        let policy = ValidationPolicy::default();
        assert!(confirm_consecutive_yes_with(
            || {
                calls += 1;
                async move { Some(true) }
            },
            &policy
        )
        .await);
        assert_eq!(calls, DEFAULT_STRENGTH);
    }

    #[tokio::test]
    async fn first_rejection_aborts() {
        let mut calls = 0u32;
        let policy = ValidationPolicy::default();
        assert!(!confirm_consecutive_yes_with(
            || {
                calls += 1;
                async move { if calls == 1 { Some(true) } else { Some(false) } }
            },
            &policy
        )
        .await);
        assert_eq!(calls, 2); // aborted on the first non-affirming answer
    }

    #[test]
    fn parse_handles_wrapped_json() {
        assert_eq!(parse_yes_no("{\"answer\": \"YES\"}"), Some(true));
        assert_eq!(
            parse_yes_no("sure: {\"answer\": \"NO\"} done"),
            Some(false)
        );
        assert_eq!(parse_yes_no("garbage"), None);
    }
}
