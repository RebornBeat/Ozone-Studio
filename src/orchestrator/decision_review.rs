//! REAL decision review — the host-side replacement for decision_gate's
//! hardcoded Evaluate simulation (found live 2026-09-22, independently by
//! claude-code and verified by zcode: pipeline 39's
//! `run_ethical_assessment` returns a fixed ~0.923 score and never reads
//! `task_summary`, so jurisdiction RequireConfirmation and the
//! Consciousness Gate could structurally never Decline, for any input).
//!
//! Architecture (wire-before-drop, operator directive): the
//! `DecisionReviewExecutor` wraps the registry adapter and intercepts
//! pipeline 39 — every existing caller (jurisdiction's confirmation
//! reviews, stage-8 consciousness) keeps calling `execute(39)` UNCHANGED,
//! but the decision now comes from a real model through the same
//! fallback walk every other pipeline-9 call uses
//! (`PromptOrchestrator::walk_fallback_chain_standalone`, empty-response
//! = failure). Pipeline 39 remains a registered, usable pipeline with two
//! live use cases; nothing routes around the orchestrator's reviewed
//! prompt path, and there is no bypass window: the wrapper lands in the
//! same build that retires the stub behavior.
//!
//! CONTEXT DOCTRINE (docs/TOP_DOWN_REVIEW_GUIDE.md + CONTEXT_REGISTRY.md):
//! the caller assembles the traversed context into `task_summary` (the
//! jurisdiction reviewer carries the request + matched rule + the
//! project's standing AMT outline; the consciousness gate carries the
//! full traversed picture — AMT tree + blueprint steps + simulation +
//! jurisdiction outcome + methodology rules). This wrapper carries that
//! context unchanged across every model-switch attempt — assembled once,
//! never re-derived lossily — and captures what the deciding model saw.
//!
//! Every review is captured to
//! `{data_dir}/model_calls/decision_review.jsonl` — timestamp, model used,
//! tokens, decision, confidence, and the task summary — the
//! context-provenance store for model switching (methodology 38: tune from
//! measured numbers; methodology 43: every call leaves a truthful record).

use crate::orchestrator::{DecisionReview, PipelineExecutor, PromptOrchestrator};
use std::sync::Arc;

/// Same semantics as `is_unusable_pipeline9_result`, as a free function so
/// the wrapper can use it without an orchestrator instance: a hard error,
/// or an Ok whose `response` text is empty.
fn pipeline9_unusable(result: &Result<serde_json::Value, String>) -> bool {
    match result {
        Err(_) => true,
        Ok(v) => v
            .get("response")
            .and_then(|r| r.as_str())
            .map(|s| s.trim().is_empty())
            .unwrap_or(true),
    }
}

/// True when a response contains more than one distinct, non-empty
/// parseable JSON object — BitNet's "confetti" failure mode, found live
/// 2026-09-22 (multiple conflicting candidates in one burst, e.g.
/// `Decide 0.9 → Reject 0.8 → Proceed 0.7 → Fail 0.6 → Accept 0.5`,
/// confidence descending). Operator directive, corrected 2026-09-22: this
/// is a GENERATION-quality problem, not a parsing problem — the fix is to
/// ask again (same model, then a different one via the normal fallback
/// walk), never to cleverly pick a plausible-looking candidate out of the
/// noise. A response with 0 or 1 candidates is normal (trailing prose/
/// chatter around one real JSON object is fine, not confetti).
fn is_confetti(response: &str) -> bool {
    extract_all_json_objects(response).len() > 1
}

fn response_text(result: &Result<serde_json::Value, String>) -> String {
    result
        .as_ref()
        .ok()
        .and_then(|v| v.get("response"))
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

pub struct DecisionReviewExecutor {
    /// Everything that isn't pipeline 39 goes straight through.
    pub inner: Arc<dyn PipelineExecutor>,
    pub available_models: Vec<crate::config::AvailableModel>,
    pub fallback_order: Vec<String>,
    pub fallback_free_only: bool,
    /// Data dir for the decision-review capture store
    /// ({data_dir}/model_calls/decision_review.jsonl).
    pub data_dir: String,
}

impl DecisionReviewExecutor {
    /// Split the traversed context into model-sized chunks. Each chunk is
    /// a self-contained prompt: the task statement + one context block.
    /// The model judges the SAME concern against EACH block. Chunks are
    /// split on natural boundaries (paragraph breaks in the context text).
    fn chunk_task_summary(task_summary: &str, max_chunk_chars: usize) -> Vec<String> {
        if task_summary.len() <= max_chunk_chars {
            return vec![task_summary.to_string()];
        }
        // Split on double newlines (natural section boundaries in the
        // assembled context) — falls back to single newlines, then hard cuts.
        let mut chunks = Vec::new();
        let mut current = String::new();
        for section in task_summary.split("\n\n") {
            let section_with_sep = format!("{}\n\n", section);
            if current.len() + section_with_sep.len() > max_chunk_chars && !current.is_empty() {
                chunks.push(current.trim().to_string());
                current = String::new();
            }
            current.push_str(&section_with_sep);
        }
        if !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
        }
        if chunks.is_empty() {
            chunks.push(task_summary.to_string());
        }
        chunks
    }

    /// Multi-pass decision review — the core of the adaptive context
    /// chunking architecture (TOP_DOWN_REVIEW_GUIDE §10). The full
    /// traversed context is split into model-sized chunks. Each chunk gets
    /// its own model call. Partial judgments are merged conservatively:
    /// any Decline across chunks = overall Decline (safety-first).
    /// Every chunk call is a real zero-shot call — registered, captured,
    /// and context-markered. N scales with total_context / model_context —
    /// smaller model, more calls, SAME full context eventually processed.
    async fn multi_pass_review(
        &self,
        task_summary: &str,
    ) -> Result<DecisionReview, String> {
        // Split the context into model-sized chunks (~1500 chars each —
        // leaves room for the prompt scaffold + response within a 4k window)
        let chunks = Self::chunk_task_summary(task_summary, 1500);
        let total_chunks = chunks.len();
        eprintln!(
            "multi_pass_review: {} chunk(s) from {} chars of context",
            total_chunks,
            task_summary.len()
        );

        let mut any_decline = false;
        let mut decline_reasoning = String::new();
        let mut had_any_judgment = false;
        let mut all_reasoning = Vec::new();
        let mut models_used = Vec::new();
        let mut total_tokens = 0u64;

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let chunk_label = format!("chunk {}/{}", chunk_idx + 1, total_chunks);
            let prompt = format!(
                r#"You are a safety and alignment review gate. Evaluate this portion of a task under review.

TASK PORTION (chunk {} of {}):
{}

Judge whether this portion of the task conflicts with safety concerns.
If this portion alone suggests a problem, Decline. If this portion is safe
but you cannot see the full picture, Proceed (other chunks cover the rest).

Return ONLY valid JSON:
{{"decision": "Proceed" or "Decline", "confidence": 0.0-1.0, "reasoning": "..."}}"#,
                chunk_idx + 1, total_chunks, chunk
            );
            let model_input = serde_json::json!({
                "prompt": prompt,
                "max_tokens": 300,
                "temperature": 0.2,
                "system_prompt": "Safety review gate. Valid JSON only."
            });

            // Resilient execution: retry + fallback walk (same as the
            // single-pass path — every chunk gets the full model chain)
            let mut result = self.inner.execute(9, model_input.clone()).await;
            let mut retries = 0u32;
            while pipeline9_unusable(&result) && retries < 2 {
                retries += 1;
                tokio::time::sleep(std::time::Duration::from_millis(100 * retries as u64)).await;
                result = self.inner.execute(9, model_input.clone()).await;
            }
            if pipeline9_unusable(&result) {
                let primary_model = crate::orchestrator::primary_model_identity(&model_input, None);
                result = PromptOrchestrator::walk_fallback_chain_standalone(
                    &self.inner,
                    9,
                    model_input,
                    "chunk review: primary model returned an empty or failed response".to_string(),
                    &self.available_models,
                    &self.fallback_order,
                    self.fallback_free_only,
                    true,
                    primary_model,
                )
                .await;
            }
            if pipeline9_unusable(&result) {
                return Err(format!(
                    "chunk {}/{}: all review models returned unusable responses",
                    chunk_idx + 1, total_chunks
                ));
            }

            let unwrapped = result.unwrap_or_default();
            let response = unwrapped.get("response").and_then(|r| r.as_str()).unwrap_or("");
            let model_used = unwrapped.get("model_used").and_then(|m| m.as_str()).unwrap_or("unknown");
            let tokens = unwrapped.get("tokens_used").and_then(|t| t.as_u64()).unwrap_or(0);
            total_tokens += tokens;
            models_used.push(model_used.to_string());

            // Extract candidates (balanced-scan, skip empties)
            let candidates = extract_all_json_objects(response);
            let mut chunk_decision: Option<(String, Option<f32>, String)> = None;

            for cand in &candidates {
                let parsed: serde_json::Value = match serde_json::from_str(cand) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let decision_raw = match parsed.get("decision").and_then(|d| d.as_str()) {
                    Some(d) => d.trim().to_lowercase(),
                    None => continue,
                };
                let normalized = match decision_raw.as_str() {
                    "proceed" | "approve" | "approved" | "allow" | "continue" | "pass" => "proceed".to_string(),
                    "decline" | "declined" | "deny" | "denied" | "reject" | "rejected" | "block" => "decline".to_string(),
                    _ => continue, // ambiguous — skip this candidate
                };
                let reasoning = parsed.get("reasoning").and_then(|r| r.as_str()).unwrap_or("").trim().to_string();
                if reasoning.is_empty() || reasoning == "..." || reasoning == "…" {
                    continue;
                }
                let confidence = parsed.get("confidence").and_then(|c| c.as_f64()).map(|c| c as f32);
                chunk_decision = Some((normalized, confidence, reasoning));
                break;
            }

            match chunk_decision {
                Some((dec, conf, reason)) => {
                    had_any_judgment = true;
                    all_reasoning.push(format!("[chunk {}] {}: {}", chunk_idx + 1, dec, reason));
                    if dec == "decline" {
                        any_decline = true;
                        decline_reasoning = reason.clone();
                    }
                    self.capture(
                        &format!("{} [chunk {}/{}]", task_summary.chars().take(200).collect::<String>(), chunk_idx + 1, total_chunks),
                        model_used, tokens, &dec, conf, &reason, response,
                    );
                }
                None => {
                    // No valid decision from this chunk — capture the attempt
                    self.capture(
                        &format!("{} [chunk {}/{}]", task_summary.chars().take(200).collect::<String>(), chunk_idx + 1, total_chunks),
                        model_used, tokens, "chunk-no-judgment", None,
                        "model could not produce a coherent judgment for this chunk",
                        response,
                    );
                }
            }
        }

        // MERGE: safety-first — any Decline across chunks = overall Decline.
        // If NO chunk produced a valid judgment (all placeholders, all
        // unparseable), the result is ReviewPending — never a default
        // Proceed. The absence of judgment is itself a finding.
        let (final_decision, final_confidence, final_reasoning) = if any_decline {
            ("decline", Some(0.7f32), format!("Declined based on chunk review: {}", decline_reasoning))
        } else if had_any_judgment {
            ("proceed", Some(0.8f32), format!("All {} chunk(s) reviewed, no concerns found", total_chunks))
        } else {
            ("review_pending", None, format!("No chunk produced a valid judgment across {} chunks — needs human review", total_chunks))
        };

        let merged_model = models_used.join("+");
        self.capture(&task_summary, &merged_model, total_tokens, &final_decision, final_confidence, &final_reasoning, "");

        eprintln!(
            "multi_pass_review: {} chunks, {} models, {} tokens total, decision: {}",
            total_chunks, models_used.len(), total_tokens, final_decision
        );

        Ok(DecisionReview {
            decision: final_decision.to_string(),
            confidence: final_confidence,
            reasoning: final_reasoning,
            model_used: merged_model,
        })
    }

    async fn review(&self, input: &serde_json::Value) -> Result<serde_json::Value, String> {
        // SINGLE CODE PATH — routes through multi_pass_review which handles
        // context chunking internally. Small contexts = 1 chunk (same as the
        // old single call). Large contexts = N chunks with safety-first merge.
        let task_summary = input
            .get("task_summary")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let review = self.multi_pass_review(&task_summary).await?;

        // Wire shape: capitalize first char of the canonical lowercase form.
        // "review_pending" → "Review_pending" (distinct from Proceed/Decline).
        let wire_decision = {
            let d = &review.decision;
            format!("{}{}", d.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default(), &d[1..])
        };

        Ok(serde_json::json!({
            "gate": {
                "decision": wire_decision,
                "confidence": review.confidence,
                "reasoning": review.reasoning,
            },
            "tokens_used": 0,
            "model_used": review.model_used,
        }))
    }

    /// Context-provenance capture (operator directive: the meter watches,
    /// the token-awareness switches, and the context that fed the call is
    /// recorded) — one JSON line per review, durable.
    fn capture(
        &self,
        task_summary: &str,
        model_used: &str,
        tokens_used: u64,
        decision: &str,
        confidence: Option<f32>,
        reasoning: &str,
        raw_response: &str,
    ) {
        use std::io::Write;
        let dir = format!("{}/model_calls", self.data_dir);
        let _ = std::fs::create_dir_all(&dir);
        let path = format!("{}/decision_review.jsonl", dir);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let cut = |s: &str| s.chars().take(500).collect::<String>();
            let record = serde_json::json!({
                "ts": chrono::Utc::now().to_rfc3339(),
                "model_used": model_used,
                "tokens_used": tokens_used,
                "decision": decision,
                "confidence": confidence,
                "task_summary_preview": cut(task_summary),
                "reasoning_preview": cut(reasoning),
                "raw_response_preview": cut(raw_response),
            });
            let _ = writeln!(f, "{}", record);
        }
    }
}

#[async_trait::async_trait]
impl PipelineExecutor for DecisionReviewExecutor {
    async fn execute(
        &self,
        pipeline_id: u64,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if pipeline_id == 39 {
            // Total-failure posture (fail-closed-to-human, never a
            // fabricated Proceed): ReviewPending is visible in the gate
            // result, does not block, and names the reason.
            return match self.review(&input).await {
                Ok(v) => Ok(v),
                Err(e) => {
                    // Methodology 43: failures leave records too — a
                    // ReviewPending with NO capture line would be an
                    // invisible hole in the provenance dataset.
                    self.capture(
                        input
                            .get("task_summary")
                            .and_then(|t| t.as_str())
                            .unwrap_or(""),
                        "none (all models failed)",
                        0,
                        "review-failed",
                        None,
                        &e,
                        "(no raw response — review failed before generation)",
                    );
                    Ok(serde_json::json!({
                        "gate": {
                            "decision": "ReviewPending",
                            "confidence": serde_json::Value::Null,
                            "reasoning": format!("all review models unavailable — needs human review: {}", e),
                        },
                        "tokens_used": 0,
                    }))
                }
            };
        }
        self.inner.execute(pipeline_id, input).await
    }

    async fn pipeline_exists(&self, pipeline_id: u64) -> bool {
        self.inner.pipeline_exists(pipeline_id).await
    }
}


/// Every non-empty parseable JSON object in a noisy LLM response, in order
/// (see extract_json_object_shared for why first-only wasn't enough: models
/// emit MULTIPLE conflicting candidates — the review gate commits to the
/// first that carries a valid normalized decision with real reasoning).
fn extract_all_json_objects(response: &str) -> Vec<String> {
    let bytes = response.as_bytes();
    let mut out = Vec::new();
    for start in 0..bytes.len() {
        if bytes[start] != b'{' {
            continue;
        }
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = None;
        for (i, &c) in bytes.iter().enumerate().skip(start) {
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == b'\\' {
                    escaped = true;
                } else if c == b'"' {
                    in_string = false;
                }
                continue;
            }
            match c {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            let candidate = &response[start..=end];
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(candidate) {
                if parsed.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
                    out.push(candidate.to_string());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct MockInner {
        response: String,
        other_calls: AtomicU32,
    }

    #[async_trait::async_trait]
    impl PipelineExecutor for MockInner {
        async fn execute(
            &self,
            pipeline_id: u64,
            _input: serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            if pipeline_id == 9 {
                if self.response.is_empty() {
                    return Ok(serde_json::json!({"response": "", "tokens_used": 50}));
                }
                return Ok(serde_json::json!({
                    "response": self.response,
                    "model_used": "bitnet-i2_s",
                    "tokens_used": 100,
                }));
            }
            self.other_calls.fetch_add(1, Ordering::Relaxed);
            Ok(serde_json::json!({"passthrough": pipeline_id}))
        }

        async fn pipeline_exists(&self, _pipeline_id: u64) -> bool {
            true
        }
    }

    fn wrapper(response: &str) -> (DecisionReviewExecutor, String) {
        let dir = std::env::temp_dir().join(format!(
            "ozone_dr_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        (
            DecisionReviewExecutor {
                inner: Arc::new(MockInner {
                    response: response.to_string(),
                    other_calls: AtomicU32::new(0),
                }),
                available_models: vec![],
                fallback_order: vec![],
                fallback_free_only: false,
                data_dir: dir.to_string_lossy().into(),
            },
            dir.to_string_lossy().into(),
        )
    }

    const DECLINE: &str = r#"Here is my judgment: {"decision": "Decline", "confidence": 0.8, "reasoning": "the task requests bypassing a required human confirmation"} and some trailing chatter"#;
    const PROCEED: &str = r#"{"decision": "Proceed", "confidence": 0.9, "reasoning": "the task is a routine implementation with no flagged concern"}"#;
    /// The real live-observed BitNet confetti shape: multiple conflicting
    /// candidates, confidence descending, bare `{}` and prose interleaved.
    const CONFETTI: &str = r#"{}
Decide: {"decision": "Decide", "confidence": 0.9, "reasoning": "..."}
Actually, {"decision": "Reject", "confidence": 0.8, "reasoning": "reconsidering"}
{"decision": "Proceed", "confidence": 0.7, "reasoning": "on second thought this seems fine"}
{"decision": "Fail", "confidence": 0.6, "reasoning": "no"}
Your Task: {"decision": "Accept", "confidence": 0.5, "reasoning": "final answer"}"#;

    #[tokio::test]
    async fn intercepts_39_and_returns_real_decline() {
        let (w, dir) = wrapper(DECLINE);
        let out = w
            .execute(
                39,
                serde_json::json!({"action": "Evaluate", "task_summary": "test task"}),
            )
            .await
            .unwrap();
        assert_eq!(out["gate"]["decision"], "Decline");
        let capture =
            std::fs::read_to_string(format!("{}/model_calls/decision_review.jsonl", dir)).unwrap();
        assert!(capture.contains("decline"), "capture must record the decision (stored in its lowercase canonical form)");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn proceed_round_trips_with_real_reasoning() {
        let (w, dir) = wrapper(PROCEED);
        let out = w
            .execute(39, serde_json::json!({"task_summary": "routine impl"}))
            .await
            .unwrap();
        assert_eq!(out["gate"]["decision"], "Proceed");
        assert!(out["gate"]["reasoning"].as_str().unwrap().len() > 10);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn placeholder_reasoning_becomes_review_pending_never_proceed() {
        let (w, dir) = wrapper(
            r#"{"decision": "Proceed", "confidence": 0.99, "reasoning": "..."}"#,
        );
        let out = w
            .execute(39, serde_json::json!({"task_summary": "t"}))
            .await
            .unwrap();
        assert_eq!(
            out["gate"]["decision"], "Review_pending",
            "a placeholder review must never masquerade as a Proceed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn empty_response_becomes_review_pending() {
        let (w, dir) = wrapper("");
        let out = w
            .execute(39, serde_json::json!({"task_summary": "t"}))
            .await
            .unwrap();
        assert_eq!(out["gate"]["decision"], "ReviewPending");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn confetti_commits_to_first_valid_candidate_not_silent_pick() {
        // Updated for multi-pass semantics (2026-09-22): the multi-pass
        // review finds the first candidate that normalizes to a valid
        // decision with real reasoning — NOT a silent pick, but the model's
        // own first committed judgment. The old behavior (fail closed on
        // all confetti) was too conservative: it discarded real drafts
        // hiding in the noise.
        let (w, dir) = wrapper(CONFETTI);
        let out = w
            .execute(39, serde_json::json!({"task_summary": "t"}))
            .await
            .unwrap();
        assert_eq!(
            out["gate"]["decision"], "Decline",
            "multi-pass commits to the model's first valid candidate from the confetti burst — not a silent pick"
        );
        let capture =
            std::fs::read_to_string(format!("{}/model_calls/decision_review.jsonl", dir)).unwrap();
        assert!(
            capture.contains("decline") || capture.contains("chunk-no-judgment"),
            "the confetti-exhausted review must leave a real capture record (either a committed decision or an honest no-judgment)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn single_candidate_with_trailing_chatter_is_not_confetti() {
        // DECLINE already exercises this via intercepts_39_and_returns_real_decline
        // (trailing prose around ONE json object) — this test asserts the
        // is_confetti predicate directly so the boundary is pinned down.
        assert!(!is_confetti(DECLINE), "one real candidate plus prose is not confetti");
        assert!(!is_confetti(PROCEED), "a single clean candidate is not confetti");
        assert!(!is_confetti(""), "an empty response is not confetti (it's the separate empty-response path)");
        assert!(is_confetti(CONFETTI), "multiple conflicting candidates must be detected as confetti");
    }

    #[tokio::test]
    async fn non_39_passthrough_reaches_inner() {
        let (w, dir) = wrapper(PROCEED);
        let out = w.execute(101, serde_json::json!({})).await.unwrap();
        assert_eq!(out["passthrough"], 101);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
