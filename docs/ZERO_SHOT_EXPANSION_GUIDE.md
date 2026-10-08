# Zero-Shot Intelligence Expansion Guide — every site to build, every context to thread, every registry it touches

> Fifth doctrine doc. CONTRACTS = algorithms; ZERO_SHOT_CALL_REGISTRY = every
> existing call; BUILDER_REGISTRY = the builders that orchestrate them;
> TOP_DOWN_REVIEW_GUIDE = how context flows to gates; CONTEXT_REGISTRY = the
> sources/consumers/carriers; **this doc = the expansion plan: every place
> we're adding real intelligence, what context each needs, which registry
> catalog it, and the build order.**

---

## 1. The doctrine for expansion

The mechanical-vs-judgment classifier (CONTEXT_REGISTRY §6) applies in both
directions: mechanical calls stay mechanical (plain is the efficiency), and
judgment calls get graph context (the AMT is the source of truth). Every
expansion below is a **judgment call that's currently done by deterministic
logic worse than a model would do it** — not context bolted onto something
that doesn't need it.

**Carry-through rule**: every new call site wires through
`metered_execute_resilient` (retry + fallback + capture + budget), sets
`_budget_fraction` for its output shape, and emits `capture_zero_shot_call`
metrics. No exceptions.

**Review-before-judge rule (added 2026-10-08, operator-directed, learned the
hard way on E8/confetti)**: a detected anomaly — confetti, an empty
response, a repeated failure pattern, any of this doctrine's own "candidate
locations" — is a TRAP that catches real defects, not an automatic trigger
to build judgment-call infrastructure around it. Confetti was initially
treated as something to *adjudicate* (E8's cross-model judge); reviewing
WHY it actually happened instead found a plain, mechanical defect (`"system_
context"` vs. the real `"system_prompt"` key — see E8's entry and
`CHECKLIST.md` 2026-10-08) present at every one of the 6 real confetti call
sites, a 6-for-6 correlation. Fixing that one key name plausibly does more
to reduce confetti than any judge ever could; the judge is still a
legitimate safety net for whatever residual ambiguity remains once real
bugs are fixed, but building it FIRST — before reviewing root cause — would
have been solving the wrong layer. **The required order, going forward,
for any anomaly this doctrine's detection mechanisms surface**: (1) review
— read the actual call-site code the anomaly traces back to, looking for a
plain, fixable, mechanical defect (wrong field/key name, missing logic, a
dead code path, anything a direct read would catch); (2) if found, fix it
directly — this is cheaper, more certain, and closes the actual gap,
exactly like the file-name/call-site corrections elsewhere in this doc; (3)
only once real defects are ruled out or fixed does "does this need a real
judgment call" become the live question — and the answer may still be yes
(residual, genuine ambiguity a human or a bug fix can't resolve), but it is
answered AFTER review, never assumed as the default response to an
anomaly. A new zero-shot call site should never be the first thing built
in response to a detected problem; it's what's left to build once the
problem is actually understood.

---

## 2. The five intelligence expansions (registry §3 candidates → real builds)

### E1: Content-aware modality routing (was `detect_file_modality`)

| | |
|---|---|
| **Currently** | pure extension-lookup (`graphs.rs:637-683`) — a .tex file routes to math only, even when it's mostly prose. This is the root cause of math's isolation gap; P4 fixed the symptom downstream. |
| **Build** | before dispatching, a zero-shot classification call: "given this file's first 2000 chars, classify as: clean-expression / prose-with-embedded-math / pure-code / prose / other" → route to BOTH the declared modality AND text when prose-fraction is significant. |
| **Context needed** | the file's raw content preview + the prompt (what the file is FOR) |
| **Output shape** | `{"classification": "...", "prose_fraction": 0.0-1.0, "route_to": [100, 105]}` |
| **Budget fraction** | 0.05 (short JSON classification) |
| **Call frequency** | once per attached file per request — low, justified |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY (new #21), BUILDER_REGISTRY §8 (process_modality's dispatch input), CONTEXT_REGISTRY S4 (file_graphs gain a routing provenance marker) |
| **Tests** | unit: mixed-prose .tex routes to [100, 105]; pure-expression routes to [105] only; code routes to [101] only |

### E2: Borderline relationship confirmation

| | |
|---|---|
| **Currently** | `link_related_containers`' arithmetic formula `(0.3 + 0.15×shared_count).min(0.9)` decides relationship at exactly the ≥2-shared-term threshold — the confidence formula is a guess, never measured. |
| **Build** | when `shared_count == 2` exactly (the minimum that qualifies), ask a model: "Container A is about [name/keywords]. Container B is about [name/keywords]. Are they genuinely related for the purpose of [scope]? Answer Proceed/Decline + reasoning." Only fires on borderline candidates — frequency is bounded by the linking pass's floor hits. |
| **Context needed** | both containers' names + keywords + topics (already fetched in the linking pass) + the matched terms |
| **Output shape** | `{"decision": "related"|"unrelated", "confidence": ..., "reasoning": "..."}` |
| **Budget fraction** | 0.03 |
| **Call frequency** | per borderline candidate (shared_count == exactly 2) — bounded by floor hits per linking pass, typically 0-5 |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY (new #22), BUILDER_REGISTRY §8 (link_related_containers's scoring stage), CONTEXT_REGISTRY §3 (the mixing map's edge-creation rows gain a judgment-provenance marker) |
| **Tests** | unit: borderline candidate with model "unrelated" → no edge; borderline with "related" → edge; shared_count ≥ 3 → no call (the formula is trusted above the floor) |

### E3: Branch-coverage reconciliation

| | |
|---|---|
| **Currently** | `stages.rs:497+` uses `overlaps(&branch.content, &s.description)` — a string-overlap heuristic decides whether a blueprint step "covers" an AMT branch. When it doesn't, a fallback step is synthesized. Real judgment would be more accurate. |
| **Build** | for each under-covered branch (the heuristic says no match), ask: "Blueprint step: [step description]. AMT branch: [branch content]. Does this step address this branch? Answer covered/not-covered + reasoning." Only fires on branches the heuristic missed — frequency is bounded by actual gaps. |
| **Context needed** | the blueprint step's action + description + pipeline_id; the AMT branch's content + children outline; the request's cleaned_prompt |
| **Output shape** | `{"covered": true|false, "reasoning": "...", "suggested_step": "..."}` |
| **Budget fraction** | 0.05 |
| **Call frequency** | per under-covered branch per request — typically 0-3, bounded by the reconciliation pass |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY (new #23), BUILDER_REGISTRY §6 (blueprint assignment's reconciliation stage), CONTEXT_REGISTRY S3→S2 (the blueprint↔AMT cross-reference becomes explicit) |
| **Tests** | unit: branch the heuristic missed but the model says "covered" → no fallback step; branch genuinely uncovered → fallback step synthesized; heuristic-passed branches never trigger the call |

### E4: Web-search-need gate trace + possible fold

> **RESOLVED (traced) 2026-10-08**: there is no separate gate, deterministic
> or otherwise. Grepped every reference to pipeline 56 in `stages.rs`
> (coercion-exemption at line 829, dispatch at line 2302) and the whole repo
> for `needs_search`/`search_needed`/`requires_search`/`should_search` (zero
> hits) — confirmed the "does this step need search" decision is made
> entirely inside the #10 blueprint-assignment zero-shot call itself, which
> writes `pipeline_id: 56` directly when it decides a step needs search. The
> "fold a deterministic gate into #12" branch of this entry doesn't apply —
> there's no deterministic gate to fold; it was already zero-shot the whole
> time. Nothing to build here. Kept below as the historical record of the
> original open question.

| | |
|---|---|
| **Currently** | registry §3 #4: "the search-need detection that decides whether to call #12 at all was not traced." Unknown whether it's deterministic keyword-matching. |
| **Build** | (1) TRACE: find the search-need detection mechanism; document it in the registry. (2) DECIDE: if it's keyword-matching, fold the need-detection into the decomposition call (#12) — one call returns both `{"search_needed": bool, "sub_queries": [...]}`. If it's already model-backed, just document. |
| **Context needed** | the step's description + the request's signals |
| **Output shape** | `{"search_needed": bool, "sub_queries": ["..."]}` (superset of #12's current shape) |
| **Budget fraction** | 0.03 |
| **Call frequency** | per step that might need search — bounded by step count |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY (#12 upgraded in-place, or new #24 if the gate is separate), CONTEXT_REGISTRY S3→S7 |
| **Tests** | trace-first: document the current mechanism; then unit test the folded shape |

### E5: Pipeline-39-style dedicated contracts for candidates #2 and #3

| | |
|---|---|
| **Currently** | pipeline 39 (decision_gate) was the precedent for a *dedicated* zero-shot decision pipeline — but the implementation was a hardcoded stub (now replaced by the real wrapper). The structural insight holds: candidates #2 and #3 would benefit from the same shape — a named pipeline with its own input/output contract, not more raw pipeline-9 call sites. |
| **Build** | NOT separate binaries (that was the stub's problem). Instead: named free functions in the orchestrator (like `real_decision_review`) that wrap pipeline-9 calls with prompt construction + extraction + gates specific to each judgment domain. The "pipeline" identity comes from the function name and its registry entry, not from a separate binary. |
| **Registries touched** | BUILDER_REGISTRY (each named function gets its own section), ZERO_SHOT_CALL_REGISTRY (each gets its own entry), TOP_DOWN_REVIEW_GUIDE (each documents its context-assembly block) |
| **Note** | this is an architectural shape decision, not a separate build item — E2 and E3 ARE the implementations of this pattern |

### E6: Cross-file call resolution (code modality)

Surfaced by the graph/relationship audit's code-modality fork, right after it fixed the dead `Calls`-edge wiring.

| | |
|---|---|
| **Currently** | `Calls` edges (now real, see the audit entry in CHECKLIST.md) are mechanically same-file-only — `extract_function_calls` structurally cannot see cross-file/cross-module calls at all, not just unwired. A callee defined in another file is silently never linked. |
| **Build** | when a callee name doesn't resolve against the same file's `function_name_to_id` map, issue a zero-shot call: given the callee name + the caller file's import list + a shortlist of candidate exported-function containers from a cross-file `SearchContainersByKeywords`/`link_related_containers` pass, classify which real container/function is the true target. |
| **Context needed** | callee name, caller's import list, up to N candidate exported-function containers from cross-file search |
| **Output shape** | `{"resolved": true/false, "target_container_id": ..., "target_function": ..., "confidence": 0.0-1.0}` |
| **Budget fraction** | ~0.05 (short JSON) |
| **Call frequency** | once per unresolved callee per file — bounded, since same-file resolution (mechanical, free) handles the common case first |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY (new entry), CONTEXT_REGISTRY (code modality's cross-file call context), BUILDER_REGISTRY (code-graph section) |
| **Tests** | unit: same-file calls never trigger this path (mechanical resolution wins); a cross-file call with an unambiguous exported match resolves; a cross-file call with no match returns `resolved: false`, not a hallucinated target |

### E7: LLM-based proof-step reference resolution (math modality)

Surfaced by the graph/relationship audit's math-modality fork, right after it replaced the fabricated `dependencies: vec![i]` with real regex-based citation detection (`extract_step_references`).

| | |
|---|---|
| **Currently** | the regex pass only catches explicit numbered/phrase citations ("step N", "equation N", "(N)", "the previous step"). It cannot resolve implicit references with no number ("by the earlier bound we derived"), named-theorem citations matched against the proof's own detected theorem names, or unconventionally-phrased references. Left unresolved, it stays honestly empty rather than fabricating — this expansion is about closing the remaining real gap, not fixing a bug. |
| **Build** | run *after* the regex pass, only for steps it left with zero citations — batch all such steps from one proof into a single call (same batching shape as E2), given the full step list (statement + step_number) and any already-detected axiom/theorem names, resolve implicit/named references the regex pass structurally cannot reach. |
| **Context needed** | full proof's step list (statement + step_number, bounded by proof length), already-detected axiom/theorem names |
| **Output shape** | `{step_number: usize, cited_steps: Vec<usize>, cited_theorems: Vec<String>, confidence: f32}[]` |
| **Budget fraction** | scales with proof length; bounded since it only runs for regex-empty steps |
| **Call frequency** | once per `AnalyzeProof` action (not per-step) — a real gap-filler, not a duplicate of the regex pass |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY (new entry), CONTEXT_REGISTRY (math modality's proof-reference context), BUILDER_REGISTRY (math-graph section) |
| **Tests** | unit: a step with an explicit citation never reaches this path (regex wins); a step with only implicit reference language resolves via the zero-shot call; a step with genuinely no reference (a true axiom/given) returns empty, not a hallucinated citation |

### E8: Confetti-candidate adjudication — the operator's correction, backed by real data: confetti is the system's prime, reliable SIGNAL for where judgment calls are missing, not a walk-local quirk

**CORRECTED 2026-10-08, same day as first written.** The first version of this entry
(kept below, struck through in spirit not text, per this doc's additive convention)
evaluated confetti-rescue too narrowly — as one local heuristic at the walk's single
rescue site — and concluded against a zero-shot fix. The operator pushed back directly:
confetti is not a walk-local quirk, it's the system's own **mechanical, zero-false-
negative signal for exactly the moments regex/extraction has hit a hard ceiling** — by
construction, confetti means the extractor found ≥2 syntactically complete, independently
parseable JSON candidates in one response; no regex can ever disambiguate which one is
"right," because the ambiguity is in what the candidates MEAN relative to the task, not in
their syntax. That's a sharper, more defensible case for the doctrine than most of E1-E7.

**Real data, checked before revising this (not re-argued from first principles alone)**:
grepped `zsei_data/model_calls/model_ledger.jsonl` for `cause=confetti`-attributed rows —
**33 real confetti events across 6 distinct call sites**, not one:
`assistant_check_up` (11), `i_loop_reflection` (10), `meta_loop_draft` (8),
`zero_shot_simulation` (2), `amt_reexpansion` (1), `blueprint_assignment` (1). The top two
are BACKGROUND consciousness loops, not the user-facing walk at all — confirming this is
systemic, not localized to where the walk's rescue mechanism happens to live. By model:
28 BitNet, but **3 `openrouter/free` and 2 `nvidia/nemotron-3.5-lightning:free`** — confetti
is real on cloud/API models too, just more frequent on BitNet; not a BitNet-only artifact
that a local workaround can quietly absorb.

**The "same model judging itself" objection from the first version doesn't hold up.**
Read `next_step` (`mod.rs:2861-2864`) directly: `Outcome::OtherError(_) =>
NextStep::MoveOn { reason: "other error", ... }` — confetti is handled completely
generically today, at every one of these 6 sites except the walk's own rescue. The
candidates are discarded, cause is attributed to `Model`, and the NEXT attempt starts from
a blind, fresh prompt — no awareness that a prior attempt produced N semantically distinct
candidates. The system already has multi-model fallback configured; it simply isn't using
it to show the next model the ambiguous candidates — it wastes a full, blind regeneration
instead. There is no new infrastructure needed to get a genuinely different judge: the
fallback-chain machinery that already exists is the natural carrier.

| | |
|---|---|
| **Currently** | confetti (`is_unusable_pipeline9_result`'s shared chokepoint, `extract_all_json_candidates_shared(text).len() > 1`) triggers one of two outcomes, both mechanical: at the walk's `run_one_candidate`, blind positional promotion of candidate `[0]`; everywhere else (6+ confirmed sites), blind discard + a fresh, blind retry via `NextStep::MoveOn`. Neither resolves the actual ambiguity — one guesses positionally, the other throws the signal away entirely. |
| **Build** | at the chokepoint, when `is_unusable_pipeline9_result` fires specifically because of confetti (not empty-response, not a hard error — those have no candidate set to adjudicate), issue ONE new zero-shot call: "a prior attempt at [call-site-specific task framing] produced several possibly-contradictory candidates: [the extracted candidates]. Which one (by index) actually answers the task, or does none? Respond `{"chosen_index": int\|null, "reasoning": "..."}`." Route this call through the EXISTING fallback-chain/multi-model machinery so the judge is naturally a model different from whichever one produced the confetti — reuse, not new infrastructure. |
| **Why this is a FAMILY of builds, not one generic mechanism** | per the operator's own framing — each call site needs its own task-framing in the adjudication prompt, the same way E1-E7 are each separately scoped (AMT branch confetti needs different context than blueprint-step confetti or methodology-draft confetti). The shared part is the chokepoint detection + the fallback-chain routing; the task-specific part is the prompt template per call site. Natural build order: the walk's own rescue site first (replace blind `[0]` with real adjudication — highest-traffic, already has a rescue path to improve), then the top two real-data sites (`assistant_check_up`, `i_loop_reflection` — background consciousness loops, 21 of 33 real events between them), then the remainder as volume justifies. |
| **Context needed** | the extracted candidate list (already computed free by the chokepoint's own detection — no new extraction work) + whatever task context that specific call site already has in scope (varies per site, same as E1-E7) |
| **Output shape** | `{"chosen_index": int\|null, "reasoning": "..."}` — `null` is a real, honest answer (none of the candidates actually address the task), not forced to pick something |
| **Budget fraction** | small (short JSON judgment) — bounded by how often confetti actually fires (33 real events observed; rare relative to total call volume, so cost is bounded by construction, not estimation) |
| **Call frequency** | only on confirmed confetti (not empty-response, not hard-error) — a narrow, well-scoped trigger condition the system already computes today |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY (new entry per site wired), BUILDER_REGISTRY (chokepoint section), CONTEXT_REGISTRY (adjudication-provenance marker) |
| **Tests** | unit: a 2-candidate confetti response where one candidate clearly answers the task → that index chosen; a confetti response where neither candidate is relevant → `chosen_index: null`, not forced; the walk's own rescue site specifically: compare outcomes against the old blind-`[0]` behavior on the same captured candidate sets, using the `confetti_rescued` markers already being written |
| **Status** | **First increment BUILT 2026-10-08** (operator go: "continue with all"), at the walk's own rescue site only (`run_one_candidate`, `mod.rs:3592+`). `run_one_candidate` gained an `other_model: Option<&AvailableModel>` parameter; the sequential fallback-walk call site passes the first OTHER real candidate in the pool (`candidates.iter().find(|c| c.identifier != profile.identifier)`) — never the same model that produced the confetti; the parallel-batch dispatch call site passes `None` for now (scoped out of this increment — see next row). On confetti, when a judge is available, one small adjudication call (`max_tokens: 200`, `temperature: 0.1`) asks it to pick the correct candidate by index or answer `null` if none are correct; recorded via `record_with_text(..., "confetti_adjudication", ...)` so it's visible in the ledger going forward. Any failure (parse error, call error, out-of-range index, honest `null`) falls back to the original first-candidate rescue — strictly no worse than before. Compiled clean (`cargo check --release`, 4m44s, zero new warnings/errors). **NOT YET DEPLOYED** — production is a separate, already-known-stale binary; deploying is a distinct, operator-gated action per this project's standing "don't touch production without approval" rule. **Remaining scope, not yet built**: the parallel-batch dispatch path (still `None`/unchanged), and the 5 other real call sites found in the ledger data (`assistant_check_up`, `i_loop_reflection`, `meta_loop_draft`, `zero_shot_simulation`, `amt_reexpansion`, `blueprint_assignment`) — each uses the generic `OtherError → MoveOn` path today, not this mechanism; wiring them is call-site-specific work (each needs its own task-context framing, same as E1-E7), not a copy-paste of the walk's version. |

---

<details>
<summary>First version of this entry (2026-10-08, same day, superseded above — kept per this doc's additive convention, not deleted)</summary>

The operator asked directly: is confetti-rescue (`mod.rs:3592-3618`, promotes the first
non-empty JSON candidate when a response contains several) right to stay static, or should
it be a real judgment call? Evaluated against this doc's own doctrine (§1) rather than
answered reflexively.

| | |
|---|---|
| **Currently** | when pipeline 9 returns >1 distinct JSON candidate in one response (the documented live BitNet pattern: `Decide 0.9 → Reject 0.8 → Proceed 0.7 → ...`, descending self-revision), the walk unconditionally promotes candidate `[0]` (first) to be the response. This is a judgment call — "which of these is the model's real/intended answer" — made by an arbitrary positional rule with **zero verified evidence it's right more often than wrong**. |
| **Why NOT a plain zero-shot fix (the ORIGINAL, too-narrow finding)** | the natural "judge" call would go through the SAME unreliable generation path that produced the confetti — asking a flaky process to grade its own confused output isn't guaranteed better. **This turned out to be too conservative**: it only considered the walk's single site, not the 5 other real call sites confetti occurs at, and it didn't check whether the fallback chain already provides a different-model path for free (it does — see the corrected entry above). |
| **Original recommendation (superseded)** | do NOT build a plain same-model zero-shot judge call; wait and measure instead. **Superseded by the corrected entry above once real ledger data (33 events, 6 call sites, 3 different models) was actually checked rather than reasoned about in the abstract.** |

</details>

---

## 3. The context-depth completions (audit findings → real builds)

### C1: S1 standing-context threading into generation calls (#2/#3/#4/#7)

| | |
|---|---|
| **Currently** | these four sites build per-request AMT structure without ever seeing the project's main AMT — project-amnesiac generation (CONTEXT_REGISTRY §6 audit finding). |
| **Build** | a `standing_context_block(state, store)` helper (like `jurisdiction_summary`) that direct-fetches the project's main AMT outline (name + topics + branch summaries), threaded into all four sites exactly like the S5/S4 threading already landed. The block is assembled once per request and reused. |
| **Sites** | amt.rs:901 (§2's branch generation), amt.rs:1241 (§3's intent extraction), amt.rs:1379 (§3's branch generation), amt.rs:2621 (§4's domain ID) |
| **Registries touched** | CONTEXT_REGISTRY (wiring matrix: S1 column gains ✅ at #2/#3/#4/#7), BUILDER_REGISTRY §2/§3/§4 (context-received fields updated), ZERO_SHOT_CALL_REGISTRY #2/#3/#4/#7 (context field updated) |
| **Tests** | unit: a project with a main AMT → generation prompts carry the outline; a project without → empty block, no change |

### C2: #16 on_step_complete blueprint-step context

| | |
|---|---|
| **Currently** | alignment review sees the AMT summary but not the blueprint step it's aligning to. |
| **Build** | thread `step.action` + `step.description` + `step.pipeline_id` into the alignment-review prompt (all available in `execute_step`'s scope). |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY #16 (context field updated), CONTEXT_REGISTRY wiring matrix (S3 column gains ✅ at #16) |

### C3: #8/#18 existing-methodology awareness

| | |
|---|---|
| **Currently** | methodology synthesis (#8) and meta-loop drafting (#18) work without knowing what methodologies already exist — #8 can draft duplicates, #18 drafts blind (shell 30420 was this exact failure). |
| **Build** | before the draft call, search the methodology store for the gap keywords (already have `search_by_keywords`) and include the nearest-match methodology names + keywords in the prompt: "Existing methodologies that partially cover this: [...] — draft only if yours adds real substance beyond these." |
| **Sites** | amt.rs:2666 (#8 synthesis), meta_loop.rs:242 (#18 draft) |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY #8/#18 (context field updated), CONTEXT_REGISTRY S6 column |
| **Tests** | unit: a draft request where an existing methodology covers 80% → prompt carries it → the model either refines or skips; a genuinely novel domain → no nearest-match noise |

### C4: Text modality — extractor retrofits

| | |
|---|---|
| **Currently** | 7 of 13 real `extract_json_from_response` call sites now have confetti detection + bounded retry: the original 2 (`extract_entities_from_text`, `extract_topics_attempt`) plus 5 more landed this pass — the section-detection state machine (`extract_next_section_event`), grammar-relationship extraction (`extract_grammar_relationships_from_text`, also gained per-element drop logging instead of silent `filter_map` loss), the sentence grammar tree and the cross-sentence-relationships+coreference call inside `extract_grammar_from_graphs` (the latter is the sole carrier of both Contradicts-class relationships and coreference chains in one payload — highest-value site in the file, protected with plain confetti-detection+retry since the two sub-structures are fields of one object, not splittable documents), and `analyze_sentiment`. 6 lower-frequency sites remain unfixed: `extract_next_sentence` (~2561), `extract_next_paragraph` (~3268), `extract_next_modality` (~3520), `clean_chunk` (~3862), `list_sentences_for_paragraph` (~5290), and the generic `parse_json_array` helper (~7003). |
| **Build** | same pattern: `extract_all_json_candidates` retrofitted to each remaining site. Mechanical, bounded. |
| **Owner** | CC (its lane — the fork that added the helper should do the retrofits) |

### C5: `extract_json_from_response` shared upgrade (amt.rs/stages.rs)

| | |
|---|---|
| **Currently** | the naive first-{-to-last-} extractor is used pervasively across amt.rs and stages.rs beyond the two sites CC's confetti fix landed at. `is_unusable_pipeline9_result`'s confetti detection now catches multi-candidate responses BEFORE they reach this extractor (the chokepoint fix), but single-response leading-{} cases still land here. |
| **Build** | replace `Self::extract_json_from_response` with the shared `extract_all_json_candidates_shared` + first-non-empty pattern. One function change, every call site inherits. |
| **Decision needed** | is_unusable_pipeline9_result already catches confetti transitively; the residual exposure is single-response-with-leading-{}. Worth doing, but not urgent — the chokepoint catches the worst case. |
| **Owner** | either — after the restart + live test confirm the current batch is stable |

### C6: Unresolved/divergent zero-shot judgment as first-class persistent state (not a dropped return value)

> **RESOLVED 2026-09-27, confirmed stale-doc 2026-10-08**: this was built
> end-to-end before this note was written — `assets/pipelines/shared/
> capture.rs` (111 lines, "C6-minimal, 2026-09-27"), pulled into text/code/
> math's main.rs via `#[path = "../../shared/capture.rs"] mod capture;`
> (no Cargo dependency needed — pipeline binaries are intentionally
> excluded from the workspace, so textual module inclusion rather than a
> shared crate answers the doc's own open question below). Writes
> `{data_dir}/model_calls/pipeline_zero_shot_calls.jsonl`, registered as
> **CONTEXT_REGISTRY.md S12**. text is wired at its single `llm_execute`
> choke point (all 13 extractor/vote sites); math is wired at its one
> real site (E7's `resolve_implicit_step_references`, dormant until E7
> ships — capture lands with it, not after); code/image make zero
> pipeline-9 calls today so have nothing to capture (code has the `#[path]`
> include present-but-dormant; image doesn't, since it never needed it —
> neither is a gap). Consumer side is also live: `src/consciousness/
> review.rs`'s `read_pipeline_zero_shot_capture_store` + `GET /capture/
> pipeline-zero-shot-calls` (`src/grpc/mod.rs`). One real bug found and
> fixed while re-verifying this entry (2026-10-08): `review.rs`'s
> `PipelineZeroShotRow` declared a field named `error` that the writer
> never emits (the real error text lands in `response_preview`) — silently
> always `None` under `#[serde(default)]`, and the S12 finding never cited
> error text at all (unlike the sibling S13 class, which already did).
> Fixed: field renamed to `response_preview` to match the real writer key,
> and the failing-pipelines finding now cites the newest real error
> verbatim, same pattern S13 already used. Kept below as the historical
> record of the original ask, per this doc's own additive convention —
> treat everything below as **history**, not an open decision.

Raised directly by the operator during the C4 retrofits: "confetti" has been used as the retry-and-discard label for two genuinely different situations, and only one of them is actually noise.

| | |
|---|---|
| **Currently** | when a zero-shot call exhausts its retries (confetti-exhausted, parse-exhausted, or a hard error) at ANY real call site — all 3 modality pipelines and the orchestrator's own confetti-retry sites alike — the outcome is identical: one `eprintln!` (ephemeral, stderr-only, gone once the process log scrolls) and an empty/`None` return to the caller. The content is permanently, silently lost — not queryable, not revisitable by a later pass, another model, or a reconciliation step. This is universal, not specific to the 5 sites just fixed in text/main.rs: the 3 modality pipeline binaries never route their zero-shot calls through the orchestrator's `capture_zero_shot_call`/`zero_shot_calls.jsonl` machinery at all (that only exists in the main binary, not the separately-executed pipeline binaries) — so today, literally zero modality-level zero-shot calls are captured anywhere durable, success or failure. `decision_review.rs`'s `ReviewPending` is the one place in the codebase that already gets this right: a genuinely durable, human/model-revisitable state instead of a dropped value. |
| **The real distinction** | true confetti (one model, one call, self-contradicting inside a single unterminated completion — the documented Decide→Reject→Proceed→Fail→Accept case) has no independent signal to preserve across its candidates; retry-then-drop-on-exhaustion is correct for this case specifically. But "exhausted after retry" and "genuinely divergent judgments across attempts or models" are NOT noise — they're exactly the kind of state the system's own persistent containers, graph edges, AMT branches, and multi-model routing exist to hold and reconcile. Collapsing both cases to the same dropped-value-plus-stderr-line treatment throws away real state in the second case. |
| **Build** | a shared `unresolved_zero_shot.jsonl` capture — a plain-file-append helper callable from BOTH the orchestrator and the standalone modality pipeline binaries (no shared orchestrator state required) — written on exhausted-retry give-up at every real call site, not just text's 5. Fields: `ts`, `call_site`, `pipeline`, `container_id`/`chunk_id`/`project_id` (whatever identifying context the call site already has in scope — no new context-fetching needed), `attempts`, `raw_responses` (every failed candidate seen, not just the last), and either the reconstructable prompt or a reference sufficient to retry later. |
| **Consumer** | the consciousness review pass (already reads the capture store + task store) is the natural first reader — surfacing "N unresolved extractions aged past threshold" as a real insight, the same pattern it already uses for aged `ReviewPending` entries. A dedicated retry-sweep pass (re-attempt later, possibly via a different model once multi-model routing lands) is a longer-term second consumer. |
| **Ties to** | the same underlying gap the cross-modal/AMT audit found for branch lineage (`Continues` relations exist in the AMT tree's own state but were never mirrored into `Context.relationships`, making them invisible to traversal) — both are cases of real system state existing momentarily and then becoming invisible to everything else. A durable unresolved-judgment record and a properly graph-mirrored branch/lineage edge are the same class of fix: make transient real state persistent and queryable instead of letting it evaporate. |
| **Budget fraction** | n/a — capture write, not a model call |
| **Call frequency** | one write per exhausted-retry call — currently rare in absolute terms (retries mostly succeed), but currently ZERO of these are captured anywhere, so the marginal value of building this is high relative to its cost |
| **Registries touched** | CONTEXT_REGISTRY (new S12 source), ZERO_SHOT_CALL_REGISTRY (every call site's fallback-status column gains a "captures on exhaustion: yes/no" flag), BUILDER_REGISTRY (consciousness pass's source list grows) |
| **Tests** | unit: exhausted retry writes exactly one line with all required fields; a later pass can read it back and reconstruct enough to retry; concurrent writers from 2 modality pipelines + the orchestrator don't corrupt the file (append-only, same discipline as the other JSONL stores) |
| **Decision needed** | real architecture decision spanning all 3 modality pipelines plus the orchestrator (shared store schema, who writes, who reads, whether pipeline binaries get a shared small capture crate rather than duplicating the append logic 3x) — flagged for the operator, not unilaterally built |

### C7: Fork-delta summary at merge-back (AMT lineage)

Unlocked by the graph/relationship audit's cross-modal/AMT fork, which mirrored AMT fork/main lineage into `Context.relationships` as real `RelationType::ForkOf`/`ContinuedBy` edges (previously invisible to traversal entirely). This is a context-depth completion, not a new capability — the traversable lineage edge is what's newly available; the summary call itself is straightforward once the edge exists.

| | |
|---|---|
| **Currently** | finding a fork's parent AMT required filename scanning (no graph edge existed); now `ForkOf` makes it a one-hop traversal. Nothing yet consumes this to produce a human-readable delta. |
| **Build** | given a fork container id, traverse its `ForkOf` edge to the parent, pull both AMT trees' verified branch content (already real, no new capture needed), and issue one zero-shot call producing a structured diff at merge-back time. |
| **Context needed** | both AMT trees' verified branches (existing data) + the new `ForkOf` edge to locate the parent without filename scanning |
| **Output shape** | `{added: [...], changed: [...], removed: [...], summary: string}` |
| **Budget fraction** | scales with branch size; one call per merge-back, not per-step |
| **Call frequency** | once per merge-back event |
| **Registries touched** | ZERO_SHOT_CALL_REGISTRY (new entry), BUILDER_REGISTRY (AMT merge-back section, once `context_mirror`'s missing `merge_back` counterpart — a separately flagged, not-yet-built gap — actually exists to trigger this from) |
| **Decision needed** | depends on `context_mirror`'s `merge_back`/graft counterpart being built first (still an open, operator-gated architecture item) — this is the natural next consumer once that lands, not buildable in isolation today |

---

## 4. The consciousness review pass (CC's fork built it; wiring + testing next)

CC's fork built `src/consciousness/review.rs` — the reviewer that reads the
capture store (S10) + task store + traverses the graph, emitting insight
containers. Two open items:

1. **Periodic loop vs manual-trigger-only**: currently manual HTTP only
   (grpc/mod.rs:616). Decision: add a third scheduled loop (like
   amt_loop/meta_loop), or fold into an existing loop's cadence. User
   decision — the 24h/1800s interval question applies here too.
2. **Live test**: once reviews exist in the capture store (after the
   restart + live test), trigger the consciousness pass and verify it
   emits insight containers with cited evidence.

---

## 5. Build order (dependencies, not priority)

```
Restart (binary 11:54 — everything compiled, none of it live yet)
  │
  ├─→ Live test: RC prompt → real Proceed/Decline + capture line
  │     └─→ If success: the capture store has real data
  │           └─→ Consciousness pass v1 live test (CC's fork + loop decision)
  │
  ├─→ E1 (content-aware routing) — closes math isolation at the root
  ├─→ C1 (S1 standing context into #2/#3/#4/#7) — closes generation amnesia
  ├─→ E2 (borderline confirmation) — independent, can parallel with E1/C1
  ├─→ E3 (branch-coverage reconciliation) — independent
  ├─→ C3 (#8/#18 methodology awareness) — independent
  ├─→ E4 (web-search-need trace + fold) — trace first, then decide
  ├─→ C2 (#16 blueprint context) — small, independent
  ├─→ C5 (extractor upgrade) — after stability confirmed
  ├─→ E6 (cross-file call resolution) — after code's Calls-edge wiring (landed) is build-verified
  ├─→ E7 (proof-step reference resolution) — after math's real-dependency fix (landed) is build-verified
  └─→ C7 (fork-delta summary) — blocked on context_mirror's merge_back counterpart (not yet built)
```

E1 and C1 should land together (both close the math-isolation /
project-amnesia gaps at complementary layers). E2/E3/C3 are independent and
can follow in any order. C5 waits for stability. E6/E7 are the direct
follow-ons to this session's graph/relationship audit fixes (code's real
Calls-edge wiring, math's real dependency derivation) — both mechanical
fixes landed already, these are the zero-shot layer on top. C7 is blocked
on a real, separately-tracked architecture gap (`merge_back`) and isn't
buildable in isolation.

---

## 6. Registry updates per expansion (so nothing lands undocumented)

Every expansion MUST update:
1. `ZERO_SHOT_CALL_REGISTRY.md` — new row in §1's table (call site, purpose, output shape, fallback status) + §5's category table if it's a new category
2. `BUILDER_REGISTRY.md` — the builder section whose function gained the call (context-received field updated)
3. `CONTEXT_REGISTRY.md` — wiring matrix row (source → consumer → carrier → status)
4. `TOP_DOWN_REVIEW_GUIDE.md` — if the expansion introduces a new gate or changes the doctrine

No exceptions — undocumented call sites are how the leading-{} bug class
stayed invisible for days.
