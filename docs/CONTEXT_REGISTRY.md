# Context Registry — every context source, every consumer, every carrier

> Fourth doc in the stack's operational half (after CONTRACTS, ZERO_SHOT_
> CALL_REGISTRY, BUILDER_REGISTRY; companion to TOP_DOWN_REVIEW_GUIDE).
> Those docs catalog algorithms, model calls, and builders. **This doc
> catalogs CONTEXT itself**: every source it lives in, every consumer that
> needs it, the carrier that moves it, and the capture that proves it
> arrived. The doctrine it serves is TOP_DOWN_REVIEW_GUIDE §1: the AMT is
> the source of truth; gates review the traversed plan; context is always
> carried; every call leaves a truthful record of what context it saw.

---

## 1. Context sources (where context lives)

| # | Source | Container/field form | Produced at | Notes |
|---|---|---|---|---|
| S1 | **Project main AMT** (standing context) | `amt_p{project}_*.json` + ZSEI container (amt-main keyword) | every project orchestration (persist_amt_container, islands + Continues lineage) | what the project IS across sessions — the scope-defining context |
| S2 | **Per-request AMT fork** | `state.amt` + container | stage 5 | intents → branches → details → cross-refs; merges back to S1 when verified |
| S3 | **Blueprint (the goal)** | `state.blueprint_steps` + container | stage 6 | steps with pipeline_id, action, context_requirements, model_override |
| S4 | **File graphs + relationships** | `state.file_graphs` + ModalityGraph containers | stage 2 STEP 0 | real SimilarTo edges text↔code (live-verified, 2 runs) |
| S5 | **Jurisdiction gate result** | `state.jurisdiction_gate_result` | stage 0 | matched rules (condition/action/source), warnings, confirmation reviews |
| S6 | **Methodology rule text** | containers via `load_methodology_rules_text` | stage 3 match, grown mid-request by §4 of BUILDER_REGISTRY | the rules plans are drafted under |
| S7 | **Simulation predictions** | stage 7 output (registry #11) → thinking log / step context | stage 7 | feasibility, risks, per-step predictions |
| S8 | **Consciousness gate result** | `state.gate_result` | stage 8 | decision/confidence/reasoning (stub until the real review lands) |
| S9 | **Coordination events** | /SharedContext containers | both agents, any time | handoffs, claims, findings — mirrored with scope keywords |
| S10 | **Decision-review capture store** | `{data_dir}/model_calls/decision_review.jsonl` | every real gate review | 8 fields: ts/model/tokens/decision/confidence/task_summary_preview/reasoning_preview/**raw_response_preview** (500 chars) — the full provenance chain per review |
| S11 | **General zero-shot capture store** | `{data_dir}/model_calls/zero_shot_calls.jsonl` | every call through `metered_execute_resilient` | call_site/model/tokens/retries/used_fallback/success/response_preview (500 chars) + **context markers**: amt_container_id, blueprint_id, project_id, prompt_preview — every raw thought correlated to its graph position |
| S12 | **Pipeline zero-shot capture store** (C6-minimal, 2026-09-27) | `{data_dir}/model_calls/pipeline_zero_shot_calls.jsonl` | every model call made INSIDE a modality pipeline binary (`assets/pipelines/shared/capture.rs`, `#[path]`-embedded; text wired at its single `llm_execute` choke point — all 13 extractor/vote sites, each retry attempt its own row; math wired at its one dead-code E7 site; code dormant, zero calls) | ts/pipeline/call_site/model/tokens/success/response_preview/prompt_preview — closes the invisible-failure class where a failed/confetti extraction used to leave ONE stderr line, an empty keyword list in the persisted graph, and a downstream isolation failure with no upstream record. Read side: `GET /capture/pipeline-zero-shot-calls?pipeline=&call_site=&model_used=&success=` (`success=false` IS the invisible-failure view). Data-dir note: pipelines resolve `OZONE_CAPTURE_DATA_DIR` → `OZONE_ZSEI_DATA_DIR` → `"zsei_data"`; identical to the host's `general.data_dir` under the normal layout, and the split is visible in the path itself if an operator ever diverges them |

## 2. The wiring matrix — source → consumer → carrier → status

Consumers are the model-call sites that NEED context (cataloged in
ZERO_SHOT_CALL_REGISTRY; builder sections in BUILDER_REGISTRY). "Carrier"
is the mechanism that moves context into the prompt.

| Consumer (registry # / builder §) | Sources wired | Carrier | Status |
|---|---|---|---|
| #4 AMT branch generation (§2/§3) | S5, S4 | thread-through: `jurisdiction_summary` + `file_relationship_summary` (stages.rs:135-260, amt.rs:460/1152) | ✅ live (verified this pass) |
| #10 Blueprint assignment (§6) | S5, S4, S2, S3-out, S6, pipelines+models | thread-through + full AMT render + methodology rule text (stages.rs:391-415+) | ✅ richest consumer, live |
| #11 Zero-shot simulation (stage 7) | S5 detail, S1/S2 outline, related graph containers, signals, entities, files, consciousness state | assembled prompt (stages.rs:795-898 — full AMT tree render, traversal-backed related containers) | ✅ live (this session's enrichment) |
| #17 AMT re-expansion deepening (amt_loop) | S6 (methodology guidance), S2 (related branches), S1 (the tree being grown) | guidance block + related-branch block + container signals | ✅ live (thin-tree aware) |
| #18 Meta-loop draft (meta_loop) | S6 gap keywords, topics | minimal — keywords + topics only | ⚠️ thinnest consumer — enrichment candidate (related methodologies as nearest-misses) |
| **Stage 8 Consciousness gate** | **S1, S2, S3, S5, S6 (S7 not yet — see below)** | **traversal-assembled review context: AMT render + blueprint steps + jurisdiction_summary + methodology rules, assembled into `task_summary` (`stages.rs:1113-1182`)** | ✅ live (verified this session by direct read) — one gap: stage 7's simulation output (S7) is never stored on `OrchestrationState`, so despite the code comment claiming 5 sections, only 4 actually land; not fixed yet |
| **Jurisdiction confirmation reviews** | **rule + request + S1 standing context** | **direct fetch: project → `amt-main` child → name+topics → threaded into `task_summary` (`jurisdiction.rs:287-329`, `:410-434`)** | ✅ live (verified this session by direct read) |
| Context aggregation ForStep (stages 9-11) | S2/S4 via traversal + keyword seeds + `known_seed_ids` (direct) | traverse_from_seeds + seed merge (context_aggregation main.rs) | ✅ live (known_seed_ids landed) |
| #14 Methodology compliance check | S6 | rule text into the judgment prompt | ✅ live |
| #16 on_step_complete alignment review | S2 (AMT state) | amt summary in input | ✅ live (thin) |

**Carriers, as a closed set** (every movement of context is one of these):
1. **Thread-through summary** — a helper renders state into a prompt block
   (`jurisdiction_summary`, `file_relationship_summary`, the simulation's
   assembled blocks). Cheap, deterministic, ids-in-hand.
2. **Direct container fetch** — per-id GetContainer when the ids are known
   exactly (file_graphs, project AMT) — no rediscovery.
3. **Traversal** — relationship-edge walks when discovery IS the point
   (`traverse_from_seeds`, `link_related_containers`' neighborhood walk).
4. **Capture** — the jsonl store(s) recording what a call saw: S10
   (decision reviews specifically) and, as of 2026-09-22, S11 (every
   call through `metered_execute_resilient`) — the same carrier, widened.

## 3. The mixing map — which contexts cross via graph EDGES vs prompt threads

This is the honest answer to "did we wire the mixing between all of them?":

| Pair | Via graph edges? | Via prompt threads? | Notes |
|---|---|---|---|
| text ↔ code ↔ math (content graphs) | ✅ **live-verified** — SimilarTo edges, real confidences (0.9/0.75/0.45), provenance | ✅ via file_relationship_summary (landed) | the original living-graph proof |
| content ↔ jurisdiction rulesets | ✅ **implemented AND fired on real data (2026-09-22, zcode)**: `lib.rs:559-599` (registration-time derivation, inside `AppRuntime::new()`) plus a one-time idempotent enrichment pass (`lib.rs:662-775`, `if kws.len() > 2 { continue; }` skips already-enriched containers) that retroactively derived real content keywords for every pre-existing thin scope container. Verified directly on disk (2026-09-22): **67 of 68 `JurisdictionRuleSet` containers now carry rich, content-derived keywords** — e.g. container 1111 (Canada) → `['ca','personal','data','child','minor','consumer','artificial','intelligence','accessibility']`, container 1102 (Norway) → `['no','personal','data','child']`. Content↔jurisdiction edge-mixing can now genuinely fire through the existing uncapped linking. | ✅ via jurisdiction_summary threading (landed) | **GAP-C1: CLOSED** — implemented, wired, and confirmed firing on real data, not just reachable code |
| content ↔ methodologies | partial: methodology_ids on containers (records), no SimilarTo-style edges | ✅ via load_methodology_rules_text into builders | edge-wiring candidate (GAP-C2, lower priority — the record link already works) |
| content ↔ coordination (/SharedContext) | mirrors create containers per event (not content↔event edges) | ✅ coordination_context layer in ForStep | scoped mirroring live |
| AMT ↔ blueprint | structural (blueprint is BUILT from AMT) | ✅ full render in blueprint prompt | inherent |
| blueprint/simulation ↔ consciousness | ✅ **live (2026-09-22)** — stage-8's `task_summary` now includes a real "SIMULATION PREDICTIONS" section (`stages.rs:1113-1182`), sourced from `state.simulation_result` populated in `stage_4_zero_shot_simulation` | ✅ full traversed picture assembled into `task_summary` | the guide's core redesign — DONE, not just being wired |
| project main AMT ↔ confirmation reviews | ✅ **live (2026-09-22)** — `jurisdiction.rs:287-329` direct-fetches the project's main AMT (name+topics) and threads it into every confirmation review (`jurisdiction.rs:410-434`) | ✅ standing context in every review | DONE, not just being wired |

## 4. Carry-through mechanics — how context survives model switching

1. **Assembled once, before the first attempt.** The traversed context
   blocks are built from `state` + direct fetches BEFORE any model call;
   the fallback walk changes the model, never the context.
2. **Token-aware budgets — FIXED (2026-09-22, zcode; verified directly by
   claude-code).** The intent: the effective model's context changes when
   the walk switches (`ModelConfigOverride.context_length`); budgets
   should derive from the effective context, never a flat constant tuned
   against one model (registry §9's finding: 16/18 sites were flat).
   `derive_output_budget` + `metered_execute_resilient` (`mod.rs:2371-
   2412`) originally computed the budget ONCE from the primary model's
   context before `try_fallback_chain` ran, then reused it unchanged for
   whatever model the walk landed on — the exact bug this mechanism
   exists to prevent. Now fixed: `_budget_fraction` survives into the
   fallback walk (no longer consumed early), and `walk_fallback_chain_
   standalone`'s per-candidate loop re-derives `max_tokens` from each
   real candidate's own `context_length` before every attempt (`mod.rs`,
   confirmed by direct read of both edit sites). Real infrastructure,
   real bug, now genuinely re-deriving per attempt as originally intended.
3. **Capture proves arrival — two real stores now, not one.**
   `{data_dir}/model_calls/decision_review.jsonl` records per pipeline-39
   review: `ts`/`model_used`/`tokens_used`/`decision`/`confidence`/
   `task_summary_preview`/`reasoning_preview` — exactly 7 fields, verified
   directly against `decision_review.rs`'s real `capture()`; no AMT/
   blueprint/rule context markers exist in it yet (a real, open
   enrichment, not a correction of something broken — see §5 item 9).
   **New, broader capture (2026-09-22)**: `{data_dir}/model_calls/
   zero_shot_calls.jsonl` extends the same proven pattern from one call
   site to every site that routes through `metered_execute_resilient`
   (`mod.rs:2438-2530`ish, `call_site` label + `capture_zero_shot_call`) —
   per-call `call_site`, model, tokens, retry count, whether the fallback
   chain was used, and success, for every adopting site. This is carrier
   #4 (capture) genuinely widened from one consumer to the whole registry,
   not a new fifth carrier — same mechanism, broader surface. Top-down
   test blueprints (TD-1/2/3, TOP_DOWN_REVIEW_GUIDE §4) assert FROM
   `decision_review.jsonl` today; extending them to also read
   `zero_shot_calls.jsonl` once it has real accumulated data is a natural,
   not-yet-done follow-on (its own honest_note already says the assertion
   mechanism itself doesn't exist yet, independent of which store it'd
   read).

## 5. The completion list (full TODO, captured)

1. **P1 — DONE**, real decision review, context-aware (all 4 parts verified live this session by direct read of source, not taken on report):
   - (a) DecisionReviewExecutor wrapper intercepting execute(39) + capture
     store + ReviewPending posture. ✅ real.
   - (b) Jurisdiction confirmations carry standing context (S1 outline +
     S4 relationships) — thread state/store. ✅ real.
   - (c) Stage-8 consciousness carries the traversed picture — ✅ real for
     S2 render + S3 steps + S5 + S6 rules, but **S7 (simulation
     predictions) is NOT included** despite the code comment claiming it —
     stage 7's output is never stored on `OrchestrationState`. 4 of the
     claimed 5 sections land, not 5. Not fixed yet.
   - (d) lib.rs wiring (wrap the orchestrator's adapter); build + suite. ✅
     real — 81/81 tests independently re-run and confirmed green.
2. **P2 — DONE (2026-09-22, zcode; verified directly by claude-code).**
   `derive_output_budget` (`mod.rs:2371`) and `metered_execute_resilient`
   originally froze the budget from the primary model's context before
   the fallback walk ran. Fixed: `_budget_fraction` survives into the
   walk, and each fallback candidate re-derives `max_tokens` from its own
   real `context_length` — matches §4 item 2's design exactly now.
3. **P3** — BUILDER_REGISTRY completion: persist_amt_container (§: island
   builder), the AMT growth phase (amt_loop deepening), context_mirror
   pair; correct the stale "only convergence and pairwise" consumer note
   (relevance is live via OZONE_RELEVANCE_POLICY).
4. **P4 — DONE (2026-09-22, zcode).** Math keyword supplement:
   `extract_content_keywords` (`math/main.rs:1572-1588`) does real
   text-extraction over raw content (stopword-filtered, ≥4 chars, capped
   at 12), populated onto `ParseResult.content_keywords`
   (`math/main.rs:1685`), merged **content-first** into
   `derive_math_keywords` (`math/main.rs:84-97`) — confirmed in code, not
   just claimed. Not yet live-tested end-to-end (needs a real orchestration
   run with a prose+math attachment to confirm actual cross-linking).
5. **GAP-C1 — CLOSED (2026-09-22, zcode).** Both halves now real and
   confirmed on real data: registration-time derivation (`lib.rs:559-599`)
   plus a one-time idempotent enrichment pass (`lib.rs:662-775`) that
   retroactively enriched every pre-existing thin scope container. Verified
   directly on disk: 67 of 68 `JurisdictionRuleSet` containers now carry
   real, content-derived keywords. Content↔jurisdiction edge-mixing is now
   genuinely observable, not just theoretically possible.
6. **GAP-C2** (lower priority) — content↔methodology edge wiring.
7. **TD-1/2/3** — the top-down context-assurance blueprints, asserted from
   the capture store.
8. **User-gated**: VoiceConfig fields; 24h interval; crash-cause capture;
   sim schema-flexible parsing; second AMT-aware jurisdiction pass.
9. **S11 (general zero-shot capture, `zero_shot_calls.jsonl`) — DONE
   (2026-09-22).** Widens carrier #4 from one consumer (S10) to every real
   call site routing through `metered_execute_resilient`. Real, scoped
   follow-on once it has accumulated data: (a) add AMT/blueprint/rule
   context markers to S10 itself (the open question from §4 item 3), (b)
   extend `src/consciousness/review.rs`'s `read_capture_store` (currently
   S10-only, real function confirmed unchanged this session) to also read
   S11 — a genuinely new, checkable insight class becomes possible once
   it does: per-call-site, per-model reliability ("this call site has a
   40% retry rate on BitNet vs 5% on OpenRouter") that S10 alone can't
   answer, since S10 only covers pipeline-39 reviews. Not built; a real,
   well-scoped next step, not a completion blocker for anything above.
10. **BitNet confetti → retry-not-parse-around redesign — DONE (2026-09-22,
    operator-directed correction, TOP_DOWN_REVIEW_GUIDE.md §3.6).** Real,
    separate, scoped follow-on named there and repeated here for
    completeness-list visibility: whether OTHER zero-shot call sites in
    `docs/ZERO_SHOT_CALL_REGISTRY.md` that parse multi-candidate or
    otherwise-ambiguous model output should get the same retry-on-
    detected-ambiguity treatment is a real, un-started audit.

## Files

- `docs/TOP_DOWN_REVIEW_GUIDE.md` — the doctrine this registry implements
- `docs/ZERO_SHOT_CALL_REGISTRY.md` — the consumers (every model call)
- `docs/BUILDER_REGISTRY.md` — the builders (context assembly points)
- `src/orchestrator/stages.rs` — jurisdiction_summary (:135),
  file_relationship_summary (:208), stage-7/8 prompt assembly
- `src/orchestrator/decision_review.rs` — the wrapper + capture store (new)
- `assets/pipelines/general/context_aggregation/main.rs` — ForStep
  traversal + known_seed_ids
- `src/orchestrator/decision_review.rs` tests — the capture/gate contract

---

## 6. Per-call-site context audit — mechanical vs informed-judgment (2026-09-22)

The operator's challenge, answered honestly: **zero-shot calls are meant to
be mechanical — adding context to all of them is inefficiency, not
intelligence.** The classifier: does this call make a JUDGMENT that depends
on what the project/plan means (→ informed-judgment, needs graph context),
or does it transform/extract/verify what's already in front of it (→
mechanical, plain is correct)? Auditing all 20 registry sites:

| # | Site | Class | Context today | Missing (only if judgment) |
|---|---|---|---|---|
| 1 | File role classification | judgment (role relative to INTENT) | prompt + path/modality/graph_id | adequate; reliability was the bug (fixed: metered_execute_resilient) |
| 2 | Graph-native branch suggestion | **judgment** | S5+S4 (landed), S6 rules | **S1 — project main AMT. Project-amnesiac: proposes branches as if the project started from zero** |
| 3 | Intent extraction | **judgment** | layer context | **S1 — same amnesia** |
| 4 | Branch generation (legacy) | **judgment** | S5+S4 (landed), S6 | **S1 — same amnesia** |
| 5 | Detail extraction | mechanical | branch + chunk evidence | none — extracts what's in front of it |
| 6 | Branch cross-ref (pairwise) | mechanical | the two branches | none — pairwise relatedness is self-contained |
| 7 | Methodology domain ID | judgment | branches | **S1 + existing methodology index** (which domains the project already covers) |
| 8 | Methodology synthesis | **judgment** | the domain | **existing methodology list** — drafts blind; the meta-loop discovers duplication only AFTER creation (shell 30420 was this exact failure) |
| 9 | Response rendering | mechanical | the Response Graph | none — renders what the graph contains |
| 10 | Blueprint assignment | **judgment** | the richest: S5/S4/S2/S3-out/S6/pipelines/models | near-complete; S1 standing patterns optional (AMT already carries merged S1 content) |
| 11 | Zero-shot simulation | **judgment** | full tree + signals + related containers + jurisdiction detail + files + consciousness state | near-complete (this session's enrichment) |
| 12 | Web-search decompose | mechanical | the search need | none |
| 13 | Context compaction | mechanical | the context itself | none |
| 14 | Methodology compliance check | judgment (bounded) | the rule text + step output | complete for its purpose |
| 15 | Yes/no confirmation votes | mechanical (bounded) | the question | none — strength-N voting is the design |
| 16 | on_step_complete alignment review | **judgment** | amt summary | **S3 blueprint step + S7 predictions** — reviews alignment without the plan it aligns to |
| 17 | AMT deepening | **judgment** | S6 guidance + S2 related branches + container signals (landed) | near-complete |
| 18 | Meta-loop draft | **judgment** | keywords/topics only | **existing methodology list** (nearest-misses) — the thinnest judgment consumer |
| — | Step execution (indirect, the big one) | **judgment** | S2/S4 traversal + seeds + step context (context_aggregation) | the richest data path, live |

**The systematic finding**: S1 (project main AMT) is the missing source for
every GENERATION-time judgment call (#2/#3/#4/#7). The island/merge-back
machinery exists at PERSIST time, but GENERATION time is project-amnesiac —
history is recorded (Continues lineage) instead of informing generation.
The fix shape: a standing-context block (main AMT root + branch outline,
direct-fetched from the known project container) threaded into the four
generation sites, exactly like the S5/S4 threading already landed. #8/#18
need the existing-methodology list for the same reason. #16 needs the
blueprint step. That's the whole audit — everything else is correctly
mechanical, and leaving it plain IS the efficiency.

---

## 7. Consciousness — the window from the task manager into the context system

The operator's framing, captured as architecture: **consciousness is not
another gate — it is the reviewer that watches the whole flow.** It has
data to review now: tasks (the task store), calls (S10, the capture
store), steps (per-step context records), and the graphs themselves
(traversal). Its job, distinct from every point-gate:

1. **Review**: tasks, their steps, the model calls made (S10's recorded
   context markers, models, decisions), and the graph structures those
   calls produced.
2. **Identify**: context that WAS needed and missing (why a call produced
   a shell — no methodology list; why a review was Pending — all models
   rate-limited), and context that may be needed NEXT (a task entering a
   regulated domain with no jurisdiction content loaded).
3. **Answer why and why not** — on its own, from the records, with the
   same citation discipline as every other review (container ids, capture
   lines, task ids — never folklore).

**Going live on the graph**: consciousness observations become real
containers (the ConsciousnessStore's ZSEI integration is already enabled
at boot) under the consciousness root, with scope keywords and ripple
events like every other write — so its insights are traversable context
for everything downstream, including the gates this guide redesigns.

**Start-testing shape** (buildable now, once P1's wrapper lands): a
consciousness review pass reads S10's decision_review.jsonl + the task
store + a traversal from the project AMT, and emits insight containers:
context gaps found (call sites whose capture lines show missing context
markers), model-coherence observations (same context, different models,
divergent decisions), and pending-review follow-ups (ReviewPending items
aging without human resolution). Each insight cites its evidence lines.
