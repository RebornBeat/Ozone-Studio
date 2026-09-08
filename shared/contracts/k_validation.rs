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

/// Build the strict one-word YES/NO request (the standardized question form).
pub fn yes_no_input(question: &str) -> Value {
    serde_json::json!({
        "prompt": question,
        "max_tokens": 20,
        "temperature": 0.0,
        "system_context": "Answer strictly with one JSON object: {\"answer\": \"YES\"} or {\"answer\": \"NO\"}. No explanation. No markdown."
    })
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

/// K-ALG: require `policy.strength` CONSECUTIVE YES answers.
/// Aborts on the first non-affirming answer. `ask` is the contextual,
/// metered oracle — one invocation per attempt, question already closed over.
pub async fn confirm_consecutive_yes_with<F, Fut>(mut ask: F, policy: &ValidationPolicy) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<bool>>,
{
    for _ in 0..policy.strength {
        match ask().await {
            Some(true) => continue,
            _ => return false,
        }
    }
    true
}

/// K-ALG: require `policy.strength` CONSECUTIVE NO answers.
/// Aborts on the first affirming answer.
pub async fn confirm_consecutive_no_with<F, Fut>(mut ask: F, policy: &ValidationPolicy) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<bool>>,
{
    for _ in 0..policy.strength {
        match ask().await {
            Some(false) => continue,
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

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
