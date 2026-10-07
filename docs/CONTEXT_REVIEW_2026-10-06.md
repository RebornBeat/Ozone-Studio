# Context handling review — verified against code, 2026-10-06

Reviews ZCode's docs/CONTEXT_HANDLING_AUDIT.md (2026-10-04) and its proposed ContextBudget service. Every site below was checked by reading the code. Nothing here was built or run.

## 1. Sites that handle context today

| Site | Code | Verdict |
|---|---|---|
| AMT lane token packing | amt.rs ~2254 (`lane_budget_tokens = 20_000`) | Verified. Packs by estimated tokens and a member cap. |
| Known-branches guard | amt.rs ~2282 (`> 24_000` chars) | Verified. |
| Methodology block trim | amt.rs ~2291 (`> 8_000` chars, newest kept) | Verified. |
| Host fallback walk context-fit | mod.rs ~3061-3074 | Verified. Models that cannot hold the input defer to the back. Nothing is dropped. |
| Text pipeline rotation context-fit | text/main.rs ~4081-4099 | Verified (ZCode's change, present in the tree). |
| Per-candidate output budget | mod.rs ~2754-2761 | Verified in the comments and the code path. |
| Prompt pipeline `token_budget` | prompt/main.rs:50 | The field exists and defaults to None at :1080. Whether any caller sets it was not verified. |

## 2. Hand-rolled constants (no model awareness)

| Site | Code | Verdict |
|---|---|---|
| Consciousness gate AMT render | stages.rs ~1400 (`budget = 1200`) | Verified. This is the one ZCode put first in the migration order. |
| Blueprint branch summaries | amt.rs ~777 (`take(120)`) | Verified. Char-safe. |
| Cross-reference summaries | (claimed 300 chars) | **Not found** in amt.rs. ZCode's claim is wrong or refers to code elsewhere. |
| Decision-review chunks | decision_review.rs ~240, 247, 332 (`take(200)`, `take(500)`) | Verified. Char-safe. |
| Jurisdiction prompt | jurisdiction.rs:511 (`min(400)`) | Verified, but it was a **byte** slice (see section 3). |
| Simulation predictions | stages.rs ~1420-1432 | Verified: all step predictions are joined with no cap. |
| Assistant digest findings | assistant.rs ~338-353 | Verified: one line per finding, no cap on count. About 1-2k tokens for 63 findings. Modest, not a bomb. |
| Response ladder | response.rs:147 (`take(220)`), :271 (`take(60)`) | Verified. Char-safe. |

## 3. Bug found in this review: byte slices that can panic

Thirteen sites in the orchestrator sliced user or model text by byte index (`&x[..x.len().min(N)]`). If the cut lands inside a multi-byte character (accented letters, CJK, emoji), the slice panics and the request fails. Affected sites included the consciousness gate's task summary (stages.rs ~1458, the user's prompt cut at 600 bytes) and the jurisdiction prompt (jurisdiction.rs:511).

**Fixed:** every such site now calls `crate::orchestrator::prefix_at_char_boundary(s, N)`, which backs up to a character boundary. Parse-checked, not built.

## 4. Gate bugs found in this review (src/pipeline/gate.rs)

- A caller cancelled while queued left its waiter in the queue. The next release granted that dead waiter a slot, and the slot was never returned. **Fixed:** release skips waiters whose sender is closed.
- A test helper passed a closure where a future was expected (a compile error under cfg(test)). **Fixed.**
- A test held the only slot and then awaited a second acquire, so it hung forever. **Rewritten** with spawned waiters and a deterministic order check.
- Added a test that a cancelled waiter does not consume the only slot.
- Not fixed, flagged for a decision: strict tiers can starve Loop calls while User traffic is sustained. Calls are not skipped, but they may not run. An aging rule is one option.
- Not fixed, flagged: a remote pipeline that calls back into the host while holding a slot could deadlock under full capacity. Nested or remote dispatch should bypass admission.

## 5. Review of the ContextBudget plan (ZCode's section 3)

The design is sound in its core: budget = window − reserved output − scaffold; sections with priorities fill in order; trims at paragraph boundaries; each trim is recorded. Gaps the plan does not address:

1. **The token estimate is chars/4.** That is crude for code, math and CJK, where the ratio is far off. A real per-family estimate or a conservative factor is needed before any budget is enforced.
2. **The walk changes the model.** A prompt built once is sized for one window. Budgets must be re-assembled per candidate, which the plan implies but does not state.
3. **Trim events volume.** One marker per trim, per call, per candidate could flood the event hub. Aggregate per call.
4. **Per-site priorities are not specified.** The plan lists sites but not which sections outrank which, for example the gate's AMT render versus its blueprint steps.
5. **Shared contract for pipelines.** Pipelines are separate crates. Sharing works through the same `#[path]` include the validator uses, but no shared file exists yet.
6. **No acceptance test is defined.** Proposed test: a prompt larger than a 4k model's window must trim, record the trim, and still complete the call.

## 6. Recommended order (unchanged in substance)

1. Consciousness gate: priority sections for the AMT render, blueprint steps, jurisdiction and simulation.
2. Assistant digest: cap by count, newest first.
3. Simulation predictions: cap by budget.
4. Decision review, jurisdiction, cross-reference, blueprint renders.
5. Aggregated trim markers to the orchestration event hub.

## 7. Not verified

- Whether any caller sets `token_budget`.
- The "67 of 68 containers enriched" claim in ZCode's registry (needs a store query).
- Whether the context-fit pre-order actually fired in the verification walk (ZCode's log grep was not checked against the file).

## 8. Completed coverage (every site in ZCode's audit, plus the ones found in review)

| Site | Verdict | Evidence |
|---|---|---|
| Lane token packing | Verified, model-independent (fixed 20k) | amt.rs ~2254 |
| Known-branches guard | Verified, fixed 24k chars | amt.rs ~2282 |
| Methodology trim | Verified, fixed 8k chars | amt.rs ~2291 |
| Host walk pre-order and deferral | Verified | mod.rs ~3061-3074 |
| Text rotation pre-order | Verified | text/main.rs ~4081-4099 |
| Per-candidate output budget | Verified | mod.rs ~2754-2761 |
| Prompt token_budget (stage 7 simulation) | Verified model-aware: `model_context_limit / 4` | stages.rs ~1917 |
| Prompt token_budget defaults (amt 3167, mod 4021) | Test-only fixtures, not production | — |
| Consciousness gate render | Verified, fixed 1200 chars | stages.rs ~1400 |
| Blueprint branch summaries | Verified, fixed 120 chars | amt.rs ~777 |
| Cross-reference summaries | NOT SUPPORTED: no 300-char cap exists in the cross-reference prompt | amt.rs cross_reference_methodologies_for_layer |
| Decision review chunks | Verified, fixed 200/500 chars | decision_review.rs ~240, 247, 332 |
| Jurisdiction prompt | Verified, fixed 400 bytes (byte slice, now char-safe) | jurisdiction.rs:511 |
| Simulation predictions | Verified: uncapped join of step predictions | stages.rs ~1420-1432 |
| Assistant digest findings | Verified: one line per finding, no count cap | assistant.rs ~338-353 |
| Response ladder | Verified, fixed 220/60 chars | response.rs:147, 271 |
| Gate and blueprint tool listings | BUG FOUND AND FIXED: the summary was computed and then discarded (`capability_summary: None`), so every prompt said no tools were registered | mod.rs ~1618 and ~1678 |
| Registered-tools summary (once delivered) | Unbounded: one line per tool, 121 tools today. Becomes a budgeted site once injected | mcp global_registry_summary |
| Traversal budgets | Defaults: max_depth 10, max_results 100, budget default; jurisdiction requests use the defaults | types/zsei.rs ~23-40, jurisdiction.rs 154, 736, 1017 |
| Relevance policy | Env-configured (OZONE_RELEVANCE_POLICY), graph-first default; consumed by pipelines | k_registry.rs, lib.rs |
| Pipeline budget default | Flat default 100_000 when the request omits token_budget | grpc/mod.rs ~1893 |

Claims still unverified: ZCode's per-site priorities (none are specified in code), the registry's "67 of 68 containers enriched" (needs a store query).

## 9. Conclusion

The context review now covers every site in the audit and the sites found in review. Two fixes landed from it (the discarded tool summary, the byte-slice panics). One ZCode claim is unsupported (cross-reference 300 chars). ContextBudget is not built. Before the build, the tool summary needs a cap, and the grpc default needs a decision.
