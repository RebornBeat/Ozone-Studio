# Zero-Shot Call Registry — every real intelligence call in this system

> Companion to `docs/CONTRACTS.md` §4 (the pipeline-9 model-call contract
> shape and wire protocols). This document does not re-explain the contract
> — it catalogs every real **call site** that uses it: who calls, why, what
> shape they ask for, and whether the call is protected by this system's
> fallback machinery. See `docs/CONTRACTS.md` for the contract itself.

Pipeline 9 (`Prompt`, `assets/pipelines/general/prompt/main.rs`) is the ONE
place a model backend (Anthropic / OpenRouter-via-chat_completions / BitNet)
is ever actually called. Every call to it is, by construction, a **zero-shot
intelligence call**: no task-specific fine-tuning, no learned weights beyond
whatever general-purpose model is configured — the entire "intelligence" of
the call is the prompt template plus whichever backend answers it. Nothing
else in this codebase produces model-generated judgment; everything else is
deterministic Rust (parsing, keyword matching, thresholds, graph walks).

That makes this registry the map of **where this system currently exercises
real judgment** — and, by the same logic, where fixed/deterministic logic
elsewhere might be a candidate to upgrade to real judgment (§3).

**Real per-call metrics (2026-09-22, user directive: track every model's
real behavior, don't guess)**: every call through `metered_execute_resilient`
now writes one line to `{data_dir}/model_calls/zero_shot_calls.jsonl` —
timestamp, `call_site` (the stable label identifying which row of §1's
table the call came from — every real call site below now passes its own
real label, not a placeholder), `model_used`, `tokens_used`, `retry_count`,
`used_fallback`, `success`, and a truncated `response_preview`. This is the
same proven append-only-JSONL pattern `decision_review.jsonl` already used
to catch the BitNet "confetti" bug (§3.6 of `docs/TOP_DOWN_REVIEW_GUIDE.md`)
— extended broadly rather than building a separate subsystem. It answers
real, measurable questions this registry couldn't before: how often does
call site X actually need a retry or fallback, and does that rate differ
between BitNet and OpenRouter (or any other configured model) — evidence,
not assumption, per methodology 38.

## 1. Direct call sites (18)

All routes go through `self.executor.execute(pipeline_id, input)` or the
`PROMPT_PIPELINE_ID = 9` constant (amt_loop.rs / meta_loop.rs). "Fallback"
below means: on failure OR an `Ok`-but-empty response (`is_unusable_
pipeline9_result`, `src/orchestrator/mod.rs:2282` — a real, named check
because a technically-successful-but-empty response is a confirmed live
OpenRouter behavior, not hypothetical), the caller walks the user's
configured multi-provider chain (`try_fallback_chain` / `try_meta_fallback_
chain` / `walk_fallback_chain_standalone`) rather than accepting the empty
result. (Line re-verified 2026-09-22, drifted from 2272 → 2282.)

**Line numbers and fallback-wired status re-verified 2026-09-22** (line numbers
had drifted after two later implementation passes inserted code earlier in
`stages.rs`/`amt.rs`/`mod.rs`; the fallback-wired column had drifted much
more substantially — the systemic empty-response fix (§10) turned out to
have reached every row below except #15, not just the originally-documented
16 sites).

| # | File:line | Stage / function | What it asks the model | Output shape | Fallback-chain wired? |
|---|---|---|---|---|---|
| 1 | `graphs.rs:497` | `classify_file_graphs_post_creation` (Stage 4a, File Role Classification) | Classify each attached-file graph as primary/supplementary/raw_data relative to prompt intent | JSON array `[{file_path, graph_id, role, reasoning}]` | **✅ Yes** (`metered_execute_resilient`) — §2's gap is RESOLVED |
| 2 | `amt.rs:932` | `build_amt_from_graphs` | Suggest AMT branches per methodology (graph-native path) | JSON branches | **✅ Yes** (`metered_execute_resilient`) |
| 3 | `amt.rs:1246` | `build_amt_layer_by_layer` — intent extraction | Extract new intents not already listed in this AMT layer | JSON | **✅ Yes** |
| 4 | `amt.rs:1409` | `build_amt_layer_by_layer` — branch generation | Suggest branches per methodology (legacy per-chunk path) | JSON | **✅ Yes** |
| 5 | `amt.rs:1593` | `build_amt_layer_by_layer` — detail extraction | Extract details per branch | JSON | **✅ Yes** |
| 6 | `amt.rs:1872` | `build_amt_layer_by_layer` — cross-ref | "Are these two branches related to each other?" | JSON relationship verdict | **✅ Yes** |
| 7 | `amt.rs:2647` | `cross_reference_methodologies_for_layer` — domain ID | Given AMT branches at layer N, identify needed methodology domains | JSON array | **✅ Yes** |
| 8 | `amt.rs:2697` | `cross_reference_methodologies_for_layer` — synthesis | Draft a concise methodology for a named domain | JSON methodology draft | **✅ Yes** |
| 9 | `response.rs:308` | `render_response_graph_tier1` | Render a structured Response Graph into fluent natural language ("constrained surface realization — render only what the graph contains") | free text | **✅ Yes** (unchanged line — didn't drift) |
| 10 | `stages.rs:500` | `stage_3_blueprint_assignment` | Generate the step-by-step blueprint answering the user's request | JSON blueprint steps | **✅ Yes** — §10's "hard-errors-only" finding is RESOLVED |
| 11 | `stages.rs:1062` | `stage_4_zero_shot_simulation` | Feasibility/clarification simulation before execution | JSON | **✅ Yes** — RESOLVED |
| 12 | `stages.rs:1529` | `execute_web_search_step` — query decompose | Break a web-search need into sub-queries | JSON array of strings | **✅ Yes** |
| 13 | `stages.rs:1778` | `execute_step` — context compaction | "Compact context losslessly for facts" | free text | **✅ Yes** (also see #15/#16, the step's own primary call) |
| 14 | `stages.rs:2032` | `execute_step` — compliance judgment | Judge whether a step's output complies with its matched methodology's decision rules (see §4's correction — not jurisdiction) | JSON | **✅ Yes** (plus its own pre-existing bounded retry+backoff, §6 item 4) |
| 15 | `mod.rs:2476` | `confirm_yes_no_orch` | Repeated (`k_validation::confirm_consecutive_yes_with`, strength-N) yes/no confirmation loop — only caller: `amt.rs:632` | `{yes: bool}`-shaped JSON, parsed per-call | **⚠️ Partial** — same-call retry via `is_unusable_pipeline9_result` (no full fallback chain: the closure can't hold `&mut OrchestrationState`, structurally incompatible with the strength-N re-callable loop) |
| 16 | `mod.rs:2674` | `on_step_complete` | AMT alignment review after a step finishes | JSON | **✅ Yes** (`metered_execute_resilient`) |
| 17 | `amt_loop.rs:485` | AMT re-expansion deepening (background loop) | Deepen an unverified AMT node with methodology/relationship guidance | JSON `{details: [...]}` | Own mechanism — cross-**pass** escalation via `meta_fallback.order`, not an in-call fallback walk (see note below); unchanged line |
| 18 | `meta_loop.rs:265` | Methodology meta-loop draft (background loop) | Draft a new methodology from a detected real gap, or `{"skip": true}` | JSON methodology draft | Same cross-pass escalation pattern as #17; unchanged line |

**Plus the highest-volume indirect site**: `stages.rs:1898` (`execute_step`'s
main dispatch, `.execute(step.pipeline_id, exec_input)`) and its sub-step
twin at `stages.rs:1808`. These execute whatever `step.pipeline_id` the
generated blueprint named — but the coercion block (now ~`stages.rs:596-630`)
unconditionally coerces every step/sub-step pipeline_id to `9` (except `56`,
WebSearch, the one deliberate exception), with an honest `tracing::warn!`
when a coercion happens. **In practice, every blueprint step that answers
the user's actual request is a pipeline-9 call.** This is the call
responsible for the "Step Execution" stage in every orchestration run
(confirmed live this session: 18k-25k tokens, 400s-1230s wall time per
request). It IS covered by the fallback chain (`stages.rs:1910-1938`,
`is_unusable_pipeline9_result` + `try_fallback_chain`). (All line numbers
in this paragraph re-verified 2026-09-22; drifted from the originally
documented 1727/1637/476-492/1739-1767.)

**Note on #17/#18's escalation pattern**: `amt_loop.rs` and `meta_loop.rs`
are genuinely detached background loops without an `OrchestrationState`, so
they can't call `try_fallback_chain` directly. Instead they track
`attempts_before` per candidate and inject a `model_override_config`
computed from `meta_fallback.order`, escalating to the next configured
model on the *next* scheduled pass rather than immediately retrying within
the same call. Functionally similar goal (don't get stuck on one broken
backend), different mechanism (cross-pass vs. in-call). Not a bug — just
worth naming as a second, distinct fallback idiom in this codebase.

**Checked and ruled out**: none of the modality pipeline crates (text/code/
math/image) or `zero_shot_simulation`/`file_link`/`url_link`/`package_link`/
`blueprint_create`/`methodology_create`/`context_aggregation`/`voice`/
`web_search` recurse back into pipeline 9 over HTTP. Their `reqwest` usage
is exclusively for the ZSEI container-store API (`OZONE_HOST`, e.g.
`GET/POST /zsei/...`), a completely separate contract (§1, `StoreAccess`,
in CONTRACTS.md).

**Housekeeping aside, not a call site**: pipeline 16 (`ZeroShotSimulation`,
`assets/pipelines/general/zero_shot_simulation/main.rs`) is registered in
`zsei_data/pipelines/index.json` and `src/pipeline/registry.rs:46` but has
**zero live call sites** anywhere in the orchestrator — `stage_4_zero_shot_
simulation` (row #11 above) reimplements the same concern inline via
pipeline 9 directly instead of dispatching to it. Orphaned, not wired to
anything; flagging for whoever next touches pipeline registration, out of
scope to fix here.

**No existing system-wide metering counts pipeline-9 invocations
specifically** (`src/pipeline/remote.rs`'s `call_count` tracks *remote*
pipeline dispatch only, unrelated) — this table is the only current record
of call-site count. Manual count: **18 direct + 2 indirect (always-9-after-
coercion) = 20 real call sites.** `metered_execute_resilient` (§10's fix) is
now used at **19 of these 20** (re-verified 2026-09-22 — every row above
except #15, #17, #18, which have their own documented reasons: #15's
structural closure constraint, #17/#18's cross-pass escalation design).

## 2. A real gap this audit found: file-role classification skips the fallback/empty-response protection

Call site #1 (`graphs.rs:491`, Stage 4a File Role Classification) is the
**only** stage-critical call site that neither checks `is_unusable_
pipeline9_result` nor walks a fallback chain — it treats any `Ok(_)` as
usable and feeds the (possibly empty) `response` string straight into JSON
parsing:

```rust
let result = match self.metered_execute(state, 9, input).await {
    Ok(v) => v,
    Err(e) => { tracing::warn!(...); serde_json::Value::default() }
};
```

This is not hypothetical — it is the exact, already-observed root cause of
a bug the parent session found live this same day: a real boot log showed
this call completing with real non-zero `tokens_used` (986) but an entirely
**empty** `response` field, producing `"EOF while parsing a value at line 1
column 0"` and silently leaving `classified_file_graphs` empty for the rest
of the run (cascading into `root_modality_list.verified_modalities`
containing only `"text"`, undercounting stage 4b's own modality-graph
count). `docs/LIVING_GRAPH_STATUS.md` gap #0 already documents this exact
failure *class* (bypassing the fallback walk → silent empty result) as
"SETTLED 2026-09-16" for the file-graph-creation path — this audit shows
the settlement didn't reach every call site; #1 still has it. Not fixed in
this pass (out of scope — cataloging only), but now precisely named with a
fix shape available: wrap with the same `is_unusable_pipeline9_result` +
`try_fallback_chain` pattern rows #10/#11 already use.

## 3. Candidate locations for new zero-shot intelligence

Read against everything above — places where a decision is currently made
by fixed/deterministic logic that could plausibly benefit from a real
call. Latency is a real cost here, not free: this session observed
pipeline-9 wall times from single-digit milliseconds up to **7+ minutes**
(BitNet cold model load) depending on backend/fallback depth, so this list
is genuinely a proposal to weigh, not a "just add it" list — a high-
frequency hot path is a real reason to say no.

1. **`detect_file_modality` (`graphs.rs:637-683`) — pure extension-lookup,
   no content inspection.** A `.tex` file is *always* routed to math only
   (pipeline 105), even when it's mostly prose (exactly the failure the
   parent session found today: math's `ParseExpression` character-
   tokenized a prose-heavy `.tex` file into garbage single-letter
   "variables"). This function runs once per attached file per request —
   low frequency, cheap to justify a call. A zero-shot classification
   ("does this file's content read as prose-with-embedded-math, a clean
   expression, or pure code?") could route mixed content through BOTH the
   declared modality AND text, rather than the current single-dispatch
   design. This is exactly the gap the parent conversation is separately
   investigating — flagging the precise mechanical fix point.
2. **`is_infrastructure_container_type` / the ≥2-shared-keyword
   cross-relationship threshold** (`link_related_containers`, referenced
   throughout the modality pipelines) — a fixed arithmetic formula
   (`(0.3 + 0.15×shared_count).min(0.9)`) decides whether two containers
   are "related." This runs on *every* graph creation (high frequency —
   not a good candidate for a call per-decision), but a zero-shot judgment
   could be layered as a secondary confirmation only when the keyword
   overlap is borderline (exactly 2, the minimum), rather than for every
   candidate pair — bounding the added latency.
3. **Branch-coverage reconciliation** (`stages.rs:497+`, the "DETERMINISTIC
   BRANCH-COVERAGE RECONCILIATION" block) — decides whether a generated
   blueprint step "covers" an AMT branch via a string-overlap heuristic
   (`overlaps(&branch.content, &s.description)`), synthesizing a fallback
   step when it doesn't. This already sits downstream of a zero-shot call
   (#10 in the table) and runs once per blueprint, so the frequency cost
   is low — a real judgment call ("does this step actually address this
   branch") would likely be more accurate than string overlap, at the cost
   of one more call per under-covered branch.
4. **`execute_web_search_step`'s query decomposition is already zero-shot
   (#12)**, but the search-*need* detection that decides whether to call
   it at all was not traced in this pass — worth a follow-up look at
   whether that gate is itself deterministic keyword-matching (a plausible
   pattern given the rest of this codebase) that could be folded into the
   same call as #12 rather than a separate gate.
5. **Jurisdiction rule matching** (`jurisdiction.rs`, outside this audit's
   direct scope but adjacent) uses `RequireConfirmation` routed through
   pipeline **39** (`decision_gate`), a *different*, dedicated zero-shot
   contract, not pipeline 9 — confirmed by grep, correctly excluded from
   this registry per the task brief. Worth noting as a precedent: this
   codebase already has at least one other pipeline (39) built as a
   dedicated zero-shot decision gate rather than a raw pipeline-9 call,
   which is the more structured shape candidates #2/#3 above could follow
   instead of adding more raw pipeline-9 call sites.

## 4. Correction to row #1's original table entry (found during this expansion pass)

Row #14 (`stages.rs:1861`) was originally described as "Judge jurisdiction rule
compliance against gathered context." That's wrong — verified by reading the
call site in full: it reads `step.methodology_ids` and
`Self::load_methodology_rules_text` (`amt.rs:2180`) — this is a **methodology**
decision-rule compliance check on a completed step's output, unrelated to
jurisdiction. There is no separate jurisdiction-flavored version of this
check anywhere (grepped for `load_jurisdiction_rules_text` — doesn't exist).
Corrected description: **"Judge whether a step's output complies with its
matched methodology's decision rules"**, `{compliant: bool, reason: string}`,
`max_tokens: 150, temperature: 0.1`, with its own real retry loop (2 retries,
150ms×attempt backoff) — the best-designed error handling of any site in §1,
worth noting as a positive pattern, not just gaps. This correction matters
because it means jurisdiction had **zero** pipeline-9 (or any) coverage in
the original table — the real jurisdiction call site is pipeline 39, §6 below.

## 5. Call-site categories

Derived from what the 20 sites actually do, not a predetermined taxonomy:

| Category | Sites | Character |
|---|---|---|
| **Graph & file classification** | #1 | One-shot per attached file; the one site missing output-validation (§2) |
| **AMT building & expansion** | #2–#6, #17 | The largest group — branch/intent/detail generation and cross-ref judgment, all low `max_tokens` (150–700), all temp 0.2–0.3, **none** fallback-chain-wired in-call (background loop #17 uses cross-pass escalation instead, by design) |
| **Methodology drafting / self-improvement** | #8, #18 | Meta-cognitive: the system writing new knowledge about itself. #18 is the only site with a real "decline to draft" output shape (`{"skip": true}`) |
| **Blueprint & execution** | #10, indirect step-execution | The two highest-stakes, highest-token sites — both correctly fallback-chain-wired |
| **Zero-shot simulation / feasibility** | #11 | Pre-execution feasibility gate; fallback-wired; known truncation history (§2) |
| **Methodology compliance check** | #14 (relabeled, §4) | Post-hoc audit of a step's own output; best error-handling of the 20 |
| **Web search support** | #12 | Cheap, degrades gracefully (`Err(_) => vec![query.clone()]`), not fallback-chain-wired but low-stakes by design |
| **Context compaction** | #13 | Free-text, not fallback-wired, but feeds a step already covered by #10/#11's guarantees upstream |
| **Confirmation / alignment loops** | #15, #16 | Repeated-vote and post-step review patterns; both silently drop a failed vote/review rather than escalating |
| **Response rendering** | #9 | The only site rendering *to* the user rather than deciding something — `temperature: 0.4`, the highest in the catalog (reasonable: fluent prose generation, not classification) |

## 6. Quality review — real, specific findings only

Systematic per-site check: prompt clarity, temperature-fit-for-task, output
validation, `max_tokens` adequacy. Findings below are the ones that are
**real and specific**, not generic advice — most of the 20 sites are fine
(clear instructions, JSON schema spelled out, temperature 0.1–0.3 correctly
matched to a classification/extraction task). What's actually wrong:

1. **#1 (`graphs.rs:491`) — already covered in depth in §2.** The one site
   with zero output validation at all.
2. **#8 (`amt.rs:2671`, methodology-draft synthesis) — `max_tokens: 400` for
   a full methodology draft is the same order of magnitude as the *already-
   confirmed* zero-shot-simulation truncation bug** (`stages.rs:948`, #11:
   `max_tokens: 400` in an earlier version of this call truncated a schema-
   conforming response — documented in `docs/LIVING_GRAPH_STATUS.md`/
   CHECKLIST.md as a real, live-observed BitNet failure mode at that exact
   token budget). #8 asks for a full methodology draft (principles +
   heuristics + decision_rules per the canonical schema in
   `methodology_create/main.rs`) in the same 400-token budget that's already
   known to truncate a comparably-shaped structured response on slower/
   longer-form backends. Not confirmed broken live (unlike #11, which *was*
   observed truncating) — flagged by direct analogy to a confirmed failure
   at the same token budget, worth watching or raising to match #11's
   current (fixed elsewhere) budget.
3. **#15/#16 (`mod.rs:2366`, `mod.rs:2488`) both fail by silent vote-drop,
   not retry.** `confirm_yes_no_orch`'s oracle closure: a call error simply
   doesn't count that vote rather than retrying or escalating — for a
   `strength-N` consecutive-confirmation policy, a transient network blip
   can extend how many real calls are needed to reach N consecutive
   successes, but never corrupts the result (a dropped vote isn't a false
   "no") — low severity, correctly fails safe by construction of the voting
   scheme itself, not by an explicit check. `on_step_complete` (#16) has no
   visible error-path handling in the reviewed window — worth a closer read
   if this call's alignment-review result ever gates something consequential
   (not confirmed either way in this pass).
4. **Positive finding, worth keeping**: #14's retry loop (§4) — bounded
   retries with backoff, distinguishing "unparseable" from "call error," and
   a loud `tracing::error!` only after retries exhaust rather than silent
   fallthrough — this is the pattern the other ungated sites (#1 especially)
   should be following.

## 7. Jurisdiction / Compliance zero-shot audit

> **RESOLVED 2026-09-22**: the stub this section documents (`decision_gate`
> always simulating, never calling a real model) is now FIXED —
> `src/orchestrator/decision_review.rs`'s `DecisionReviewExecutor` intercepts
> pipeline 39 and real-model-backs both jurisdiction confirmations and the
> Consciousness Gate, wired at `src/lib.rs:1111-1118`. The finding below is
> kept as the historical record of what was found and why it mattered (this
> doc's own "additive, don't delete" convention) — treat everything in this
> section as **history**, not current state. See `docs/TOP_DOWN_REVIEW_
> GUIDE.md` §3 for the real design that replaced it.

**Not covered by §1 at all — jurisdiction's real "zero-shot" call is
pipeline 39 (`decision_gate`), not pipeline 9.** Traced in full:
`src/orchestrator/jurisdiction.rs::resolve_confirmation_reviews` (line 367)
is the only jurisdiction-adjacent model-backed call in the codebase —
verified by reading the entire file; rule loading, region/scope matching,
and source retrieval are all pure deterministic string/keyword logic, no
other model call anywhere in jurisdiction.rs.

**`resolve_confirmation_reviews` → pipeline 39, `action: "Evaluate"`**
(`jurisdiction.rs:376-387`): sends `task_summary` (the matched rule's
condition + legal source + first 400 chars of the original prompt),
`blueprint_id`, `user_id`. Expects back `{gate: {decision, confidence,
reasoning}}`; `decision == "Decline"` sets `result.blocked = true`; any
other value (including a missing/malformed field) defaults via
`.unwrap_or("Proceed")` to **not blocking**. A call error is recorded as
`"ReviewFailed"` and — by explicit, already-documented design
(`jurisdiction.rs:391-395`) — **also does not block** ("fails open... but
visible, not silently swallowed").

**Major finding — this is not actually a zero-shot call.** Read
`assets/pipelines/consciousness/decision_gate/main.rs` (528 lines) in full,
specifically the `Evaluate` arm (`execute()`, line 394) that
`resolve_confirmation_reviews` invokes. There is **no `reqwest`, no HTTP
client, no reference to pipeline 9, anywhere in this file** (grepped to
confirm). The four assessment functions it runs are:
- `run_ethical_assessment(task_summary, config)` — **`task_summary` is
  passed in but never read in the function body**; returns 3 hardcoded
  principles with fixed scores (0.95/0.90/0.92) and fixed canned reasoning
  strings ("Task does not appear to cause harm"), regardless of what the
  actual task is.
- `run_emotional_response(_task_summary)`, `run_experience_retrieval(
  _task_summary)`, `run_identity_alignment(_task_summary)` — parameter is
  **underscore-prefixed (compiler-confirmed unused)** in all three; each
  returns fixed/empty hardcoded data (`experience.warnings` is always
  `Vec::new()`, `ethical.concerns` is always `Vec::new()`).

`make_decision`'s branches (critical-concern check, ethical-threshold check,
experience-warnings check) are gated on these hardcoded values. With
`GateConfig::default()` (`ethical_threshold: 0.7`, `auto_decline_on_critical_
concern: true`) and the hardcoded ethical score of ~0.923 always exceeding
the 0.7 threshold, and `concerns`/`warnings` always empty: **every
`DecisionGateInput::Evaluate` call returns `GateDecision::Proceed`
unconditionally, for any input, under default configuration** — confirmed
by tracing every branch, not by running it live (this pipeline binary isn't
built and wasn't live-tested this pass, per the same honesty standard as
`docs/CONTRACTS.md`'s pipeline-availability caveats — this is a static-read
finding, worth a live confirmation before treating as fully proven, but the
code path admits no other outcome as written).

**Why this matters specifically for jurisdiction**: `RequireConfirmation`
rules exist in `assets/jurisdiction/global.json` precisely for content
sensitive enough to warrant a real judgment call rather than a blanket
`Block`. The jurisdiction.rs doc comment at line 358-360 calls this "a real
model-based review, not a fabricated approval" — that comment is itself
inaccurate as the code currently stands: it's a fabricated approval, just
not a *hardcoded-true* one at the jurisdiction.rs call site — the fabrication
lives one layer down, inside decision_gate's simulated assessments. Combined
with the `.unwrap_or("Proceed")` default on a malformed/missing `decision`
field (§ above) and the explicit fail-open-on-error design, there is
currently **no configuration under which a `RequireConfirmation` jurisdiction
match can actually be declined**, short of manually lowering
`ethical_threshold` below ~0.92 or feeding `auto_decline_on_critical_concern`
a concern that nothing in this pipeline ever populates. This is the single
most significant finding of this expansion pass — flagged for the user/ZCode
as a real decision: either wire `decision_gate`'s `Evaluate` arm to a genuine
pipeline-9 call using `task_summary` (the parameter is already there, unused),
or knowingly accept it as a placeholder simulation and adjust jurisdiction.rs's
own doc comment + `LIVING_GRAPH_STATUS.md`/CHECKLIST.md's characterization of
"real enforcement" for `RequireConfirmation` accordingly. Not fixed in this
pass — documentation/review only, per scope.

## 8. Missing-categories analysis (verified, not speculative)

Broader functional areas checked directly for zero-shot coverage — reporting
only what was actually verified, one way or the other:

- **Consciousness/ethical decisions: confirmed entirely deterministic, not
  zero-shot, today** — see §7's decision_gate finding. This answers the
  question directly: consciousness's "judgment" is currently simulated
  arithmetic over hardcoded inputs, not a model call, anywhere in that
  pipeline.
- **Jurisdiction rule content staleness: confirmed zero coverage.** Grepped
  `jurisdiction.rs` for `stale`/`refresh`/`re-search`/`last_verified` —
  no hits. Rule content is loaded once from `assets/jurisdiction/*.json` at
  boot with no mechanism (zero-shot or otherwise) to flag when a cached
  legal-source snippet might be out of date.
- **Coordination graph / multi-agent handoff path (task 43/45): confirmed
  zero zero-shot coverage.** Grepped `CoordinationEvent`/`coordination_graph`
  usage and `src/mcp.rs`'s `McpCall`/`call_global` path for any pipeline-9/
  `metered_execute` reference — none found. Handoffs, claims, and notes are
  stored and matched by keyword only.
- **Code-modality output quality beyond tree-sitter structure: confirmed
  zero coverage.** Grepped `assets/pipelines/modalities/code/main.rs` for
  any pipeline-9 call — none. Consistent with — and a likely contributing
  factor to — the separately-found `function_calls`/`calls` extraction gap
  from the parent session's node audit: there's no zero-shot fallback or
  cross-check layered over the structural parse at all, so a tree-sitter
  query gap like that one has nothing to catch it.

## 9. Token-budget audit — `max_tokens` hardcoding vs the real dynamic mechanism (2026-09-22, user-directed)

**The premise the user raised is correct and precisely targeted.** This
codebase already has a real, correctly-built, genuinely model-agnostic
mechanism for sizing token budgets to whatever backend is actually
configured — `state.model_context_limit` (`mod.rs:1511-1521`), resolved per
request from `model_config.context_length` → the request's
`available_models` list → `config.models.context_length` (the real
configured backend), never a hardcoded per-model-family guess. There is
even a documented, already-fixed prior bug (`mod.rs:1366-1375`) for exactly
the failure class this audit is about: `model_context_limit` used to fall
back to a hardcoded 200,000-token ("claude-sonnet-4") assumption regardless
of what was actually configured, so a bare request against a BitNet server
(real context 4096) computed a downstream budget of `200000/4 = 50000` and
sent BitNet's `llama-cli -n 50000` — a simple prompt looked hung for
minutes instead of finishing quickly. That specific bug is fixed.

**But that fix only reached 2 of the 18 real `max_tokens` call sites.**
Every occurrence of `"max_tokens"` across `src/orchestrator/*.rs`, cross-
referenced against §1's table by file:line:

| # (§1 ref) | File:line | Purpose | Current `max_tokens` | Dynamic? |
|---|---|---|---|---|
| — | `stages.rs:1603` | `execute_step` — context compaction | `input_budget` (derived from `model_context_limit`) | **✅ Yes** |
| — | `stages.rs:1688` | `execute_step` — the primary step-execution call itself (highest-volume call in the system, 18-25k tokens/request live) | `state.model_context_limit / 4` | **✅ Yes** |
| #1 | `graphs.rs:486` | File Role Classification | 500 | ❌ flat |
| #2 | `amt.rs:912` | `build_amt_from_graphs` branch suggestion | 600 | ❌ flat |
| #3 | `amt.rs:1218` | AMT intent extraction | 500 | ❌ flat |
| #4 | `amt.rs:1378` | AMT branch generation (legacy path) | 600 | ❌ flat |
| #5 | `amt.rs:1562` | AMT detail extraction | 700 | ❌ flat |
| #6 | `amt.rs:1841` | AMT branch cross-reference | 150 | ❌ flat |
| #7 | `amt.rs:2616` | Methodology domain identification | 200 | ❌ flat |
| #8 | `amt.rs:2666` | Methodology draft synthesis | 400 | ❌ flat |
| #9 | `response.rs:304` | Response Graph → natural language rendering | 800 | ❌ flat |
| #10 | `stages.rs:339` | Blueprint assignment (generates the step plan) | 1000 | ❌ flat |
| #11 | `stages.rs:943` | Zero-shot simulation | 800 | ❌ flat — **see below, this is the one already known to have truncated live** |
| #12 | `stages.rs:1361` | Web-search query decompose | 200 | ❌ flat |
| #14 | `stages.rs:1845` | Methodology compliance judgment | 150 | ❌ flat |
| #16 | `mod.rs:2483` | AMT alignment review (`on_step_complete`) | 300 | ❌ flat |
| #17 | `amt_loop.rs:444` | AMT re-expansion deepening (background loop) | 400 | ❌ flat |
| #18 | `meta_loop.rs:242` | Methodology meta-loop draft (background loop) | 800 | ❌ flat |

**16 of 18 sites are flat literals with no relationship to
`state.model_context_limit` at all** — same number regardless of whether
the configured backend is a 4k-context local GGUF model or a 200k-context
cloud model.

**This is the concrete case the user is describing, confirmed by the
codebase's own history, not inferred.** `stages.rs:943` (#11, zero-shot
simulation) is documented in `CHECKLIST.md`'s 2026-09-20 BitNet coherency
sweep as having genuinely truncated a schema-conforming response at
`max_tokens: 400`, "fixed" by raising it to the current flat `800`. That
fix landed two call sites away from `stages.rs:1688`, which solves the
identical problem correctly by deriving from `model_context_limit` instead
of picking a new constant. Raising 400→800 happens to track roughly what
`model_context_limit / 4` or `/ 8` would already give **for a BitNet-sized
context (~4096)** — which is exactly the tell: **these numbers read as
tuned against BitNet's specific observed behavior, not computed from
whatever model is actually configured**, matching the user's framing
precisely ("not hacking for BitNet when BitNet is just one of many local
models"). The same flat 400-1000 values would:
- **Under-use a large-context cloud model** (Anthropic/OpenRouter at 32k-
  200k) — capping every one of these 16 calls at a few hundred tokens no
  matter how much real budget is actually available.
- **Still risk truncating on a smaller-than-BitNet local model** (e.g. a
  2k-context GGUF) — the exact failure class `stages.rs:943` already lived
  through once at 400, with nothing to prevent it recurring at a different
  model size on any of these other 16 sites.

**Not fixed this pass — per explicit user direction, this is a capture/
audit only.** The real fix, when undertaken, is mechanical and has an
existing correct precedent to copy: replace each flat literal with a
`state.model_context_limit`-derived fraction sized to that call's real
output shape (a short JSON verdict like #6/#14 needs a much smaller
fraction than a full blueprint or methodology draft like #10/#8), the same
pattern already proven live at `stages.rs:1688`/`1603`. Worth deciding
alongside this: whether a sensible **floor** (so a tiny local model's
budget doesn't shrink a short-JSON call to the point of failure) and/or
**ceiling** (so a huge-context cloud model doesn't get asked for an
absurdly large response on a call that only ever needs a few hundred
tokens) should be layered on top of the raw fraction — `mod.rs:1901`'s
existing `(model_context_limit / 4).max(256)` is a real, already-shipped
example of exactly that floor pattern, worth reusing rather than
reinventing.

## 10. CORRECTION (2026-09-22, user-caught live) — the empty-response-despite-real-tokens bug is systemic, not one call site

**§1's "Fallback-chain wired?" column was materially misleading for #10 and
#11.** Live evidence, twice: `graphs.rs:491` (File Role Classification)
returned `tokens_used=Some(986)` then `tokens_used=Some(965)` on two
separate real orchestration runs, both with `response=""` — confirmed on
the second occurrence to be a genuine live OpenRouter call (a real
`OPENROUTER_API_KEY` was exported that session, the call completed in ~7s
with no fallback-WARN logged beforehand), ruling out "BitNet noise" as the
explanation.

**Real root cause, confirmed by reading the wire-protocol handlers
directly**: `assets/pipelines/general/prompt/main.rs`'s `call_openai_api`
(line 427-430) and `call_anthropic_api` (line ~343-346) each extract
response text and token count as two **independent** operations on the
same API response:
```rust
let content = result["choices"][0]["message"]["content"]
    .as_str().unwrap_or("").to_string();   // silently "" on ANY shape mismatch
let tokens = result["usage"]["total_tokens"].as_u64()...; // succeeds independently
```
A null `content`, a refusal field, a reasoning-only response, or `content`
shaped as an array instead of a string (all real OpenRouter free-tier
behaviors) all silently produce `response: ""` while `tokens_used` reports
real spend regardless. Same gap in both handlers.

**The scope correction**: checked exactly which of the ~20 real call sites
call `is_unusable_pipeline9_result` (`mod.rs:2272` — treats an `Ok`-but-
empty response the same as a hard error) *before* accepting a result.
**Only one does**: the primary step-execution dispatch
(`stages.rs:1737-1767` — the highest-volume call in the system, the one
that answers every real user request). Its own comment already names this
exact failure mode: *"Confirmed live this session (repeatedly, via the
methodology meta-loop): pipeline 9 (especially OpenRouter) can return `Ok`
with a genuinely empty `response` field — a real, recurring backend
behavior, not a hard error."* `stages.rs:356` (#10) and `stages.rs:948`
(#11) — the two sites §1 originally labeled "Yes" — only call
`try_fallback_chain` from an `Err(e) =>` match arm, with **no
`is_unusable_pipeline9_result` check anywhere** — they would silently
accept an empty-but-`Ok` response exactly like `graphs.rs:491` does. This
distinction was missed by two independent review passes (this doc's
original author-fork, and both CC's and ZCode's cross-verification of each
other's work) — both confirmed the "Yes" label was *present* without
checking what specifically it guarded against.

**Net: ~19 of ~20 real zero-shot call sites in this codebase are
structurally exposed to silently accepting a real-token-spend-but-empty-
content result**, with exactly one correctly-guarded reference
implementation already built, proven, and ready to copy. The fix pattern
(already live at `stages.rs:1737-1767`): wrap the call in an
`is_unusable_pipeline9_result` check, retry a bounded number of times, then
walk the fallback chain — never silently accept an empty result as if it
were a genuine "nothing to say" answer. Not implemented at the other 19
sites this pass — flagged to the user given severity (this degrades live
production results, not just a sizing inefficiency), awaiting a decision on
fix-now vs. continue-documenting.

## 11. Does relationship data reach downstream stages, or is it write-only per request? (2026-09-22, investigation only)

> **RESOLVED 2026-09-22 (same day, later pass)**: the gap this section
> documents (AMT building and blueprint assignment never seeing a request's
> own file relationships) is FIXED. `file_relationship_summary` (real
> function, `src/orchestrator/stages.rs:208`) does exactly the "cheapest-
> correct fix" recommended at the end of this section — a direct per-id
> container fetch of `state.file_graphs`'s known ids, reading each one's
> real `relationships` array — and is now called from `build_amt_from_
> graphs` (`amt.rs:460-461`), `build_amt_layer_by_layer` (`amt.rs:1152-
> 1153`), and `stage_3_blueprint_assignment` (`stages.rs:391-392`). The
> `context_aggregation` seeding improvement (this section's other
> recommendation) also landed: `ForStep` gained `known_seed_ids: Vec<u64>`
> (`context_aggregation/main.rs:74`), merged into `traverse_from_seeds`
> alongside the keyword-search seeds. Kept below as the historical record of
> the investigation that found and scoped the fix.

The user's framing: text/relationship data is what lets content "find a
place to belong." This session already proved real `SimilarTo` edges get
created among a request's own attached files (`state.file_graphs`,
`src/orchestrator/mod.rs` STEP 0, `link_to_existing: true`). This section
traces whether that relationship data, once created, is actually READ by
anything else in the same request, or whether it's created and then
effectively write-only.

**Every real read site of `state.file_graphs` after STEP 0, checked
directly:**
- `graphs.rs:447` (File Role Classification prompt) — builds
  `file_summaries` from `path/graph_id/modality` only. **No relationship
  data.**
- `amt.rs:2119-2137` (`FileLayerContext`, fed into AMT building) —
  `file_path/modality/role/graph_id` only. **No relationship data.**
- `amt.rs:2139-2151` (`GraphLayerContext`, also fed into AMT building) —
  includes `cross_modal_edge_count` from `SessionGraphState`, but that's a
  bare **count**, not the actual edges (no target_id, confidence, or
  `discovered_via`). This is also for `state.modality_graphs` (the
  stage-4b mechanism), not `state.file_graphs` (STEP 0) — a different map
  entirely.
- `stages.rs:742-746` (`files_desc`, blueprint assignment prompt) — role
  labels from `classified_file_graphs` only. **No relationship data.**

**Conclusion: neither AMT building nor blueprint assignment — the two
stages that decide WHAT WORK TO DO — ever see the real relationship edges
among a request's own attached files.** They know the files exist, their
modality, and their classified role; they do not know "file A is
0.9-confident related to file B." The zero-shot simulation stage's
`related_names` (already documented in §1) is a fresh, independent,
whole-store keyword search — also not `state.file_graphs`-specific.

**One real exception, found in `context_aggregation`**
(`assets/pipelines/general/context_aggregation/main.rs`, used in stages
9-11 to assemble what an executing step actually sees): this pipeline DOES
walk real relationship edges — `traverse_from_seeds` (line ~215) issues a
real `ZSEIQuery::Traverse` (`mode: Structural`) from keyword-search seeds,
and its own comment states this "walks real Relation edges from the flat
keyword-search seeds, so genuinely related content... can be discovered
even when the step's own keywords only directly matched one side of that
relationship." **This directly contradicts `docs/LIVING_GRAPH_STATUS.md`
gap #1's "zero call sites" claim for the TraversalEngine mechanism** — not
re-investigated in full (out of this task's scope), but worth a fresh
look: either that gap is now stale (this looks like a same-session fix,
per the comment's own "fixed earlier tonight" phrasing), or `Traverse`
here routes through a different mechanism than the specific
`TraversalEngine` struct gap #1 names — needs disambiguating, not assumed
either way.

**But even this real mechanism is keyword-seeded, not
`state.file_graphs`-direct**: `seed_ids = search_containers_by_keywords(...)`
(line ~691) uses AMT-branch keywords + step description words — it does
NOT take `state.file_graphs`'s exact known graph_ids as seeds. In practice
this often works (a just-created file graph's own keywords usually
resemble the prompt's extracted keywords), but it's probabilistic
rediscovery, not a guaranteed direct link to the exact containers this
request itself just created.

**Recommended cheapest-correct fix, not implemented (investigation only
per scope)**: for AMT building and blueprint assignment specifically,
`state.file_graphs`'s graph_ids are already exactly known — no
search/discovery needed. A direct per-id container fetch (existing
`StoreAccess`/`ZSEIQuery::GetContainer` pattern, cheap, no traversal
required since the ids are in hand) reading each one's own
`relationships` array, then threading a short summary ("File A ↔ File B:
SimilarTo 0.9, via TextAnalysis") into the AMT-building and
blueprint-assignment prompts, closes the most consequential gap
(what-to-do decisions being relationship-blind) far more cheaply than
wiring the full `TraversalEngine` into two more stages. For
`context_aggregation`, the cheap improvement is additive: seed
`traverse_from_seeds` with `state.file_graphs`'s exact graph_ids
alongside the existing keyword-search seeds, rather than relying on
keyword rediscovery alone.

**CC independent verification (2026-09-22)**: read every cited site
directly before accepting this section. Confirmed exactly as reported:
`state.file_graphs` read sites (`amt.rs:2128`, `graphs.rs:447`,
`stages.rs:33`) — no relationship data at any of them; `jurisdiction_desc`
(below, §12) is rich but doesn't include `warnings`/`confirmations`.
**The TraversalEngine contradiction is real and resolved, not just
flagged**: traced the full call chain myself — `context_aggregation`'s
`traverse_from_seeds` issues `ZSEIQuery::Traverse`, which `src/zsei/
query.rs:129-131` dispatches to `traversal.traverse(...)`, which is the
literal `TraversalEngine` struct `docs/LIVING_GRAPH_STATUS.md` gap #1 calls
dead code — `TraversalMode::Structural` routes to
`TraversalEngine::structural_traversal` (`src/zsei/traversal.rs:48/154`).
This is a real, live call path, not a coincidentally-named parallel
mechanism. **`LIVING_GRAPH_STATUS.md` gap #1 is now stale** — corrected
there, not just noted here (see that doc's own delta section).

## 12. Context-depth audit — does jurisdiction/consciousness/AMT/graph-relationship context actually reach the calls that should see it? (2026-09-22, user-directed)

> **RESOLVED 2026-09-22 (same day, later pass)**: both confirmed real gaps
> below are FIXED. Gap #1 (`jurisdiction_desc` missing `warnings`/
> `confirmations`) and gap #2 (jurisdiction never reaching AMT/blueprint) are
> both closed by the same `jurisdiction_summary`/`file_relationship_summary`
> implementation pass referenced in §11's resolution note above —
> `jurisdiction_summary` (`stages.rs:135`) now renders the full
> `JurisdictionGateResult` including warnings/confirmations, and is called
> from all three "what work to do" functions (`amt.rs:460`, `amt.rs:1152`,
> `stages.rs:391`), not just the simulation prompt. Kept below as the
> historical record of the investigation.

User's framing: the Zero-Shot Simulation stage "is missing a lot," and consciousness/jurisdiction need to genuinely "run through the blueprint and AMT," not sit as thin disconnected checkpoints. Traced the real stage order and every prompt construction to separate confirmed gaps from things that are already fine.

**Real stage order** (`stages.rs:9-97`, confirmed by reading `execute_stages` directly): Jurisdiction Gate (stage 0, fully resolves matched/warnings/confirmations here) → Text Normalization → Gather Methodologies → File Classification → Initial Graph Creation → **Build AMT** (stage 5) → **Blueprint Assignment** (stage 6) → **Zero-Shot Simulation** (stage 7, this is where §1's row #11 lives) → **Consciousness Gate** (stage 8, the real decision). This ordering matters: jurisdiction's real outcome is fully known **before** AMT and blueprint even start; consciousness's real decision does **not exist yet** until after simulation. The two have structurally different "how much could realistically be threaded in earlier" ceilings — treating them as the same kind of gap would be wrong.

**Confirmed already fine, not a gap**: `jurisdiction_desc` (`stages.rs:757-790`, the simulation prompt's jurisdiction section) is genuinely rich, not a placeholder — it renders up to 5 real matched rules with their actual `condition`, `action`, and `source`, not just a match count. A prior pass (its own comment cites "user directive: pass as much real data as possible") already did real work here.

**Confirmed real gap #1 (cheap fix, same function, same `state`)**: `jurisdiction_desc` itself never surfaces `state.jurisdiction_gate_result`'s other two real fields — `warnings: Vec<String>` (the actual human-readable Warn messages) and `confirmations: Vec<(JurisdictionRule, GateResult)>` (each RequireConfirmation review's real `decision`/`reasoning`, once `decision_gate` is real — see §7). Right now the simulation sees *that* something was matched and its raw condition/action, but never the warning text or the confirmation review's own reasoning. Fix is pure string-building in the existing `match &state.jurisdiction_gate_result` block — no new call, no signature change.

**Confirmed real gap #2 (cheap fix, all three functions already take `state: &mut OrchestrationState`)**: jurisdiction's fully-resolved outcome **never reaches AMT building or blueprint assignment at all** — grepped `jurisdiction`/`state.jurisdiction_gate_result` across `amt.rs` and `stages.rs`: the only two hits outside jurisdiction.rs itself are `stages.rs:757` and `:920` (the simulation prompt). `build_amt_from_graphs` (`amt.rs:428`), `build_amt_layer_by_layer` (`amt.rs:1099`), and `stage_3_blueprint_assignment` (`stages.rs:121`) all already receive `state: &mut OrchestrationState` as a parameter — `state.jurisdiction_gate_result` is trivially reachable, it's just never read there. Practical effect: if a request matches a Warn or RequireConfirmation jurisdiction rule, neither the AMT branches nor the blueprint steps are shaped by that at all — the plan gets built in total ignorance, and only the simulation (which critiques a plan already finalized) ever sees it, too late to actually influence what gets planned. This is exactly the "run through the blueprint and AMT" gap the user named.

**Consciousness — thinner, but for a structural reason, not neglect**: `consciousness_desc` (`stages.rs:820-821`) is a one-line placeholder because there is genuinely nothing richer to say yet — the real Consciousness Gate decision happens at stage 8, strictly after simulation (stage 7). The boolean `state.request.consciousness_enabled` is the maximum real signal that exists before then. Threading that same boolean into AMT/blueprint (stages 5/6, both technically reachable the same way as jurisdiction above) is possible but low-value — it adds no information beyond what's already implicit in the request. **Not the same class of gap as jurisdiction's.** Separately confirmed as a positive: `state.request.consciousness_enabled` *does* reach step-execution context (`stages.rs:1471`, `context_aggregation`'s `ForStep` action, `"include_consciousness"` field) — consciousness isn't universally isolated, just isolated from AMT/blueprint specifically, same gap shape as jurisdiction's.

**Jurisdiction matching against the raw prompt is intentional, not a bug**: `jurisdiction.rs:269-273`'s own comment explains it matches `state.request.prompt.to_lowercase()` directly because this runs before `prompt_normalization`, specifically so a `Block` action can stop the request before any real processing happens. This is correct as designed — a fast-fail safety gate can't wait for AMT to exist. A **genuinely new** idea (not a wiring gap, a real architectural proposal): a *second*, later jurisdiction pass for Warn/RequireConfirmation tiers only (never Block, which must stay early) using the AMT's real intent breakdown instead of raw-prompt keywords — could catch subtler policy-relevant framing the first pass's keyword match misses. Flagging as a candidate, not recommending it be built without a real decision — it's new surface area, not "surface what's already there."

**Net**: 2 confirmed, cheap, mechanically-reachable gaps (jurisdiction → AMT/blueprint, and `jurisdiction_desc`'s missing `warnings`/`confirmations` fields) — both pure "read more of `state`" fixes, no new zero-shot call needed, no signature changes. 1 apparent gap (consciousness thinness) that's actually a structural non-issue given stage ordering. 1 new architectural proposal (a second, AMT-aware jurisdiction pass) explicitly flagged as a future decision, not a fix.

## 13. Silent truncation/dropping audit — does successful parsing ever drop real model output? (2026-09-22, user-directed)

The BitNet "confetti" bug (§3.6 of `TOP_DOWN_REVIEW_GUIDE.md`) proved a real class of failure: a technically-successful call can still lose real model output to bad extraction. This section audits every real JSON-extraction implementation in the codebase against the same risk — verified by direct read, not assumed from the confetti fix's framing.

**UPDATE (2026-09-22, later same day) — the systemic gap is now CLOSED, not just documented.** Everything below this line was true when first written; all three "not yet fixed" items have since landed:

- **`amt_loop.rs`/`meta_loop.rs`'s own extractors — FIXED.** Both gained real confetti detection (`extract_all_json_objects`, >1 candidate = confetti) wired into each file's own existing retry mechanism (cross-pass `attempts_before` escalation for `amt_loop.rs`, cycle-`continue` for `meta_loop.rs`). Directly relevant: `meta_loop.rs`'s version of this bug meant a leading empty `{}` in a confetti burst would silently read `has_decision_rules`/`has_heuristics` as false and discard a genuinely real BitNet draft later in the same response as "no real content" — indistinguishable from BitNet actually producing a shell. This is a plausible real explanation for this session's earlier "meta-loop discarding drafts as shells" observation — a mechanical extraction bug, not necessarily a model capability gap.
- **`stage_4_zero_shot_simulation`'s schema-drift — FIXED.** Real schema-flexible parsing added: tries the requested `step_predictions` array shape first, falls back to scanning `step_N`-keyed data (handling both structured and plain-string drifted values) when the array comes back empty, sorts by step index, logs when the recovery path fires. The long-documented BitNet drift no longer silently loses prediction data.
- **The systemic exposure across ALL `metered_execute_resilient` sites — CLOSED at the chokepoint, not per-site.** `is_unusable_pipeline9_result` (`mod.rs:2320`) now ALSO treats a confetti-shaped response (>1 non-empty candidate, object or array shape — new shared `extract_all_json_candidates_shared`, `mod.rs`, generalizing `decision_review.rs`'s object-only version to both shapes since some call sites parse arrays) as unusable, not just an empty one. Since every one of the ~15 real `metered_execute_resilient` call sites (plus the primary step-execution dispatch) already routes its retry/fallback decision through this one function, this retroactively protects all of them against confetti — including the two that use `parse_json_object` (`stage_3_blueprint_assignment`, `stage_4_zero_shot_simulation`) — WITHOUT touching those call sites individually. Confetti is now caught and retried/escalated before `parse_json_object`'s fragile span-based parse ever runs on it.
- **Real residual scope, correctly not claimed as fixed**: this closes the CONFETTI class specifically (multi-candidate responses) everywhere. It does NOT fix `parse_json_object`'s own naive `find('{')`-to-`rfind('}')` span logic for a genuinely single, non-confetti, malformed response (e.g. truncated JSON, a stray unmatched brace inside a single answer) — that narrower residual risk still exists at `stage_3_blueprint_assignment`/`stage_4_zero_shot_simulation` specifically, and converging `parse_json_object` onto the same balanced-scan discipline the other three implementations now share remains real, unclaimed follow-on work.
- Verified: `decision_review.rs`'s primary-attempt confetti check is unaffected (still its own local logic). Its FALLBACK step (`walk_fallback_chain_standalone`, which already calls `is_unusable_pipeline9_result` per-candidate) now gets confetti protection for the first time — previously only its primary attempt was checked, a fallback candidate's confetti response would have been silently accepted.

---

Original audit below, kept for history per this doc's additive convention:

**Four independent extractor implementations existed, not one shared one — confirmed by reading each (now three of the four gaps closed, see update above):**

1. **`decision_review.rs`'s `extract_all_json_objects`** — balanced-brace scan, returns ALL non-empty parseable candidates, empty `{}` explicitly filtered (`parsed.as_object().map(|o| !o.is_empty())`). The most robust of the four; this is where the confetti fix landed first.
2. **`amt_loop.rs:628`'s `extract_json_object`** — balanced-brace scan, first-candidate-only. **Now confetti-protected** (see update above); the extraction function itself is unchanged, but a real `extract_all_json_objects`-based detection gate runs before it's trusted.
3. **`meta_loop.rs:387`'s `extract_json_object`** — same, **now confetti-protected** the same way.
4. **`mod.rs:3124`'s `parse_json_object`** — the least robust of the four (naive first-`{`-to-last-`}` span, not a balanced scan), feeding `stage_3_blueprint_assignment` and `stage_4_zero_shot_simulation`. **Now transitively confetti-protected** via `is_unusable_pipeline9_result` (see update above) — confetti never reaches it. Its own naive-span logic for a single malformed response remains the real residual gap.

**The new general-purpose capture** (`metered_execute_resilient`'s `capture_zero_shot_call`, `{data_dir}/model_calls/zero_shot_calls.jsonl`, `mod.rs:2438-2523`, real `call_site`/model/tokens/retries/used_fallback/success fields) makes any remaining failures at these sites *visible* going forward, complementing the fixes above rather than substituting for them.

## Files

- `assets/pipelines/general/prompt/main.rs` — the contract implementation (see CONTRACTS.md §4)
- `assets/pipelines/consciousness/decision_gate/main.rs` — pipeline 39, the jurisdiction `RequireConfirmation` + Consciousness Gate contract (§6-7)
- `src/orchestrator/mod.rs` — `metered_execute`, `try_fallback_chain`, `try_meta_fallback_chain`, `walk_fallback_chain_standalone`, `is_unusable_pipeline9_result`, `confirm_yes_no_orch`, `on_step_complete`
- `src/orchestrator/stages.rs` — blueprint assignment, zero-shot simulation, step execution + coercion, web search decompose, methodology compliance check (§4)
- `src/orchestrator/amt.rs` — AMT layer building, methodology cross-reference, `load_methodology_rules_text`
- `src/orchestrator/amt_loop.rs`, `src/orchestrator/meta_loop.rs` — background-loop call sites with cross-pass escalation
- `src/orchestrator/response.rs` — response-graph rendering
- `src/orchestrator/jurisdiction.rs` — `resolve_confirmation_reviews`, `categorize_jurisdiction_matches` (§6)
