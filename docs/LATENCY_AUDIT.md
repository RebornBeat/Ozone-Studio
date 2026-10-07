# Latency audit: where time goes, and what can be cut without dropping anything

Scope: code read at worktree HEAD 5dd9024. No build, no live LLM calls, no servers started. Timings marked "observed" come from ZCode's S12 notes and are not re-measured here.

Operator constraints applied: never skip a model or an attempt, never drop a feature, add no LLM calls, config decides model order.

## 1. The fallback walk

Facts from code:

- **(a) Per-attempt bounds.** `walk_fallback_chain_standalone` (src/orchestrator/mod.rs:2989-3048) has no timeout of its own. Each attempt is bounded by three nested limits:
  - HTTP to the model: 120s in `call_anthropic_api` (assets/pipelines/general/prompt/main.rs:296) and in `call_openai_api` (main.rs:422), which carries OpenRouter. The 120s is hardcoded, not configurable.
  - Adapter watchdog per pipeline invocation: 300s (src/orchestrator/adapters.rs ~95-110, default `model-300` at src/k_registry.rs:115).
  - Outer watchdog wrapping the whole metered call, including the entire walk: 300s (src/orchestrator/mod.rs:2711-2716, same K-registry value).
- **(b) Immediate errors.** The HTTP path has no retry loop (grep of prompt/main.rs finds none). An error returned by the server comes back as soon as the server responds, so 401, 404, 429 and 5xx do not wait for the 120s timeout. The walk moves to the next candidate on any `Err`. Not verified: status-by-status handling inside `call_openai_api`.
- **(c) Sequential.** `for profile in &candidates` awaits each attempt before starting the next (mod.rs ~3010-3045). Attempts never run in parallel.
- **(d) Empty or unusable responses.** `is_unusable_pipeline9_result` (mod.rs:2505-2520) treats an `Err`, an empty response, and a response containing more than one JSON candidate as unusable. The walk moves on without retrying the same model. Each iteration overwrites `result`, so if the last candidate fails, the earlier non-empty response is lost (mod.rs:3044-3047).

Consequence, measured by code reading only: a dead model costs whatever its server takes to fail. ZCode observed about 20s, which is consistent with fast HTTP failures and well under the 120s ceiling. A hanging endpoint costs up to 120s per candidate.

**Finding that conflicts with "never skip":** the outer 300s watchdog covers the whole walk. If the summed attempts exceed 300s, the call returns "watchdog timeout" and every remaining candidate is never tried. Stage 2's 234s (observed) is close to that line. Not verified live whether any walk actually hit it.

## 2. Latency-sensitive paths

| Path | Where | Timeout | Parallel or sequential | Blocks a user request |
|---|---|---|---|---|
| Orchestrate model call and walk | mod.rs:2711, 2989 | 300s outer, 300s adapter, 120s HTTP per attempt | Sequential candidates | Yes |
| Consciousness gate (stage 5, execute 39) | stages.rs:1374; decision_review.rs:179 | Walk limits above; decision-review-specific budget not checked | Sequential walk | Yes |
| AMT lanes, first pass | amt.rs ~2385 (JoinSet) | 300s per lane | Parallel within the pass | Yes, within an orchestrate walk |
| AMT lane retry | amt.rs ~2453 | 300s per lane | Starts only after all first-pass lanes finish | Yes |
| Pipeline subprocess | executor.rs:482 `spawn_blocking(cmd.output())` | None in the executor; only the adapter's 300s | Sequential per call | Yes |
| BitNet CLI | prompt main.rs:938-950 (also GGUF at 558-570) | None; only the 300s adapter | Sequential, model reloaded every call | Yes when used |
| BitNet keep-warm server | prompt main.rs:822 (health, 2s), 846 (request, 300s) | 2s health, 300s request | Sequential; falls back to CLI when unhealthy | Yes when configured |
| zsei store locks | zsei/mod.rs ~165-169 (write across `process().await`), ~321 (read across traverse) | None | Shared RwLock | Yes (source of the freeze) |
| I-loop, meta loop, AMT re-expansion | i_loop.rs:66, meta_loop.rs:56, amt_loop.rs:193 | Sleep intervals from config | Background | No, but holds executor slots and model calls |
| Assistant ticker | consciousness/assistant.rs:528 | Ticker interval | Background | No, same competition |
| Task queue | task/mod.rs:71 (max 5), spawn 1066, 1896 | Queue | Background | No, same competition |
| MCP tool call | mcp.rs:431 (30s HTTP); jurisdiction rules reloaded per call | 30s | Sequential per call | Yes when a user calls a tool |

## 3. Candidates

**C1. Fast-fail on errors that cannot succeed on retry (401, 404, model not found).**
- Changes: nothing in the code. The walk already moves on at once on any `Err`, and the HTTP path does not retry.
- Does not change: which attempts run, or the chain order.
- Evidence: section 1(b).
- Decision: rejected. There is nothing to fail faster. A change would only matter if `call_openai_api` waited on a response body for these statuses, which is not verified.

**C2. Per-error-class timeouts exposed in config, today's values as defaults.**
- Changes: makes the 120s and 300s values configurable.
- Does not change: behavior at the current defaults.
- Risk: lowering a timeout to cut dead-model waits would also cut slow-but-working models, which is a dropped attempt. Rejected as a default. The user can set it in config.

**C3. User-tier calls not queued behind background work.**
- Changes: the ordered gate with origin tiers (User, Lane, Loop). The gate exists in the parent's tree (src/pipeline/gate.rs, wired into the executor, not in this worktree).
- Does not change: whether any call runs.
- Decision: not implemented here. Verify once built, because the wiring is uncompiled.

**C4. Config flag to reorder the walk by observed success, default off.**
- Changes: attempts are reordered so recently failing models go last. Every candidate still runs.
- Spec (not implemented): add `#[serde(default)] reorder_by_health: bool` to `ModelFallbackConfig` (src/config/mod.rs:674-684). Keep a process-wide consecutive-failure map keyed by model identifier. Stable-sort `candidates` by failure count, with a success resetting the count. Never filter.
- Callers that would need it: mod.rs:2449-2470, decision_review.rs:179, meta_loop.rs, amt_loop.rs. A process-wide map avoids changing their signatures.
- Decision: not implemented. It touches four call sites and cannot be compiled here, so a mistake would only surface in the final build.
- Expected effect: dead models stop sitting at the front of every call. Evidence needed: S12 per-model failure counts (observed), and a live run to confirm.

**C5. Decouple the outer 300s watchdog from the walk.**
- Changes: the walk would be bounded per attempt, not in total, so later candidates stay reachable.
- Does not change: the per-attempt bounds.
- Risk: a call that legitimately uses many attempts can take longer than 300s in total. This is a timing policy, so the user owns it.
- Decision: recommended, not implemented.

**C6. Kill the subprocess when the watchdog abandons a call.**
- Changes: `executor.rs:482` would use `tokio::process::Command` with `kill_on_drop(true)`, not `spawn_blocking(cmd.output())`. The adapter comment already notes that the child "may outlive this future".
- Does not change: the calls, or the attempts.
- Evidence: the adapter comment (adapters.rs ~100-110). A timed-out child keeps its memory until it exits on its own.
- Decision: recommended, not implemented. It changes the execution primitive, and the change can't be tested here.

**C7. Retry each failed AMT lane as soon as it fails, not after the whole first pass.**
- Changes: scheduling only. The same lanes and retries run.
- Decision: not implemented. amt.rs has uncommitted changes in the parent's tree.

**C8. Keep the earlier non-empty response when the last candidate fails.**
- Changes: if every candidate fails, return the best earlier response instead of the last error.
- Does not change: the attempts.
- Risk: it changes which output a caller sees, so it's a quality decision.
- Decision: recommended, not implemented.

## 4. Not verified without a live run

- The observed ~20s per dead model and the S12 failure counts.
- Whether any real walk has hit the 300s outer watchdog.
- Status-by-status error handling inside `call_openai_api`.
- The decision-review budget used by the consciousness gate.
- The actual RAM held by orphaned subprocesses after a timeout (C6).
