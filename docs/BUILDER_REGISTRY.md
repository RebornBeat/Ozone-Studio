# Builder Registry — every function that constructs real structure

> Third doc in a three-doc stack, each one level up the stack from the last:
> `docs/CONTRACTS.md` catalogs the swappable **algorithm families** (K-Algorithm
> presets, store/pipeline/wire contracts); `docs/ZERO_SHOT_CALL_REGISTRY.md`
> catalogs every individual **model call** those algorithms and builders make;
> this doc catalogs the **builders** — the higher-level functions that
> orchestrate zero-shot calls, K-Algorithm presets, and real graph data into
> actual structure (AMT trees, blueprints, modality graphs). Read bottom-up
> (a call, inside a builder, inside a contract) or top-down (this doc first)
> — both are the same system.

A "builder" here means: a function that **decides/constructs** real structure,
as opposed to one that just executes already-decided work. Concretely: AMT
tree construction, blueprint assignment, and modality-graph creation. Pure
getters, queries, and step-execution dispatch are out of scope — see
`ZERO_SHOT_CALL_REGISTRY.md` for those.

Every claim below is a direct source read against the current codebase
(2026-09-22), not carried over from either companion doc's earlier snapshots
— where this doc's finding differs from an older claim in the other two
(e.g. "not yet reaching AMT" vs. "now reaching AMT"), that's because the
context-wiring fix landed *between* those docs being written and this one.

## 1. The gateway — `build_amt` (`amt.rs:9-101`)

**Purpose**: Stage 5's single entry point. Routes to one of two real builders
based on data readiness, persists the result as a real ZSEI container (never
just left in transient per-request state), and registers a re-expansion
candidate when the persisted tree still has unverified content — the trigger
that feeds the AMT re-expansion background loop (`amt_loop.rs`).

**Routing** (`amt.rs:15-26`): `AmtBuildMode::GraphTraversal` (→
`build_amt_from_graphs`) when any processed chunk carries sentence nodes with
real grammar relationships (Path 2 / OMEX-native output); `AmtBuildMode::
ChunkZeroShot` (→ `build_amt_layer_by_layer`) otherwise. This is the real,
current version of the "one gateway, two routes" design `docs/AMT_EXPANSION.
md` describes (that doc also names a third route, `Continuation`, which is
lineage/fork provenance on top of these two, not a third builder — see §5,
`merge_back_to_main`).

**Zero-shot calls it makes directly**: none — pure routing + persistence.
**Context it receives**: N/A (dispatcher only).
**K-Algorithm presets used**: none directly (both routed-to builders use
`convergence`, see §2/§3).

## 2. `build_amt_from_graphs` (`amt.rs:428-1113`) — graph-native path

**Purpose**: Builds AMT structure directly from real sentence-graph evidence
(coreference chains, cross-sentence relationship edges, shared grammar
subjects/objects) via union-find pool clustering, rather than asking a model
to invent structure from scratch. Falls back to `build_amt_layer_by_layer`
(`amt.rs:448`) if the chunk graph turns out empty — the router's own
readiness check isn't perfectly precise, so this is a real, live-reachable
fallback path, not dead code.

**Zero-shot calls it makes**: registry #2 (`amt.rs:927`, "Suggest branches
per methodology," `max_tokens: 600`, `temperature: 0.3`) — the only real
model call in this builder; pool clustering and boundary evaluation are pure
Rust (union-find, term-overlap), with one *additional* zero-shot call for
ambiguous adjacent-pool boundary merges (a "same neighborhood?" judgment,
inside the boundary-evaluation loop, `amt.rs:620+` — not in the main
registry table as a separately numbered row; same call shape, worth adding
to `ZERO_SHOT_CALL_REGISTRY.md` §1 as a follow-up, not done here — out of
this doc's scope to edit that table).

**Context it receives** (computed once at `amt.rs:460-461`, reused across
the per-methodology loop): `Self::jurisdiction_summary(state)` and
`self.file_relationship_summary(state).await` — both wired in tonight,
confirmed live by direct read. Interpolated into the branch-generation
prompt specifically (`amt.rs:901-902`, `JURISDICTION CONTEXT: {jurisdiction_
ctx}` / `{file_relationships}`) — **not** into the boundary-evaluation
prompt. This is a precise, real distinction: the context wiring targets the
"what work should exist" decision (branch generation), not the "does this
evidence cohere" decision (boundary merging), which is evidence-driven by
design and doesn't need jurisdiction/relationship framing.

**K-Algorithm presets used**: `convergence` (`amt.rs:1094`, `.read().
default_preset()`) — governs when the boundary-merge loop stops (nothing
grew, nothing pruned, nothing new loaded, or the safety ceiling). Confirms
`CONTRACTS.md`'s claim that `convergence` has a live consumer here.

**Stage**: 5 (Build AMT), one of the two routes `build_amt` dispatches to.

## 3. `build_amt_layer_by_layer` (`amt.rs:1114-2022`) — legacy per-chunk path

**Purpose**: The zero-shot-driven alternative when no real sentence graph
exists to build from — asks a model to propose structure directly, in four
distinct sub-stages, rather than deriving it from grammar evidence like §2.
Confirmed by an earlier fork this session (independently verified) to be the
path every live orchestration request this session actually exercised —
this is the one real requests hit today, not §2's graph-native path.

**Sub-stages, each a distinct real model call**:
| Sub-stage | Line | Registry # | `max_tokens` / `temperature` | Purpose |
|---|---|---|---|---|
| Intent extraction | `amt.rs:1241` | #3 | 500 / 0.2 | Extract new intents not already in this AMT layer |
| Branch generation | `amt.rs:1404` | #4 | 600 / 0.3 | Suggest branches per methodology — **the one sub-stage that receives jurisdiction/relationship context** (`amt.rs:1379-1380`, same pattern as §2) |
| Detail extraction | `amt.rs:1588` | #5 | 700 / 0.3 | Extract concrete details per branch |
| Cross-ref | `amt.rs:1867` | #6 | 150 / 0.2 | "Are these two branches related to each other?" — pairwise judgment |

> **Cross-ref PARALLELIZED (ZCode, 2026-10-09)** — the `EFFICIENCY_GUIDE.md`
> §6 Tier-1 #1 win: up to `max_pairs` (real value 50) independent pairwise
> calls, previously strictly sequential, now dispatched in bounded
> `tokio::task::JoinSet` waves (`amt.rs:1922-2030`) at 1× cores. Results are
> keyed by the real pair identity `(i, j)` in a `HashMap`, not by position
> — a genuinely robust choice: a `HashMap` lookup by the semantic key can
> never suffer the completion-order-vs-spawn-order mismatch the two Tier-0
> bugs had, and a panicked pair's key simply never gets inserted, so the
> lookup at application time correctly falls through to a named
> `"pair result missing (wave join failed)"` error — not a silent drop, not
> a misattribution. **Independently verified correct on that specific
> question** by CC, 2026-10-09.
>
> **A real finding found while verifying it, not a correctness bug — a
> resilience/observability regression, exactly the "efficiency vs.
> degradation" trade-off this project's doctrine warns about.** The
> original sequential loop routed each pair's call through
> `self.metered_execute_resilient(state, input, call_site)`
> (`src/orchestrator/mod.rs:3087+`), which provides, per call: a bounded
> same-model retry on an empty/unusable response; a genuine **fallback
> chain** to a different model if the primary is still unusable after that
> retry (`try_fallback_chain`, `mod.rs:3296`); and a `capture_zero_shot_
> call` entry to `zero_shot_calls.jsonl` (the S11 store) recording the
> outcome, retry count, and whether fallback was used — PLUS a real
> `model_ledger::record`/`record_with_cap` entry with cause attribution.
> The new spawned task (`amt.rs:2001-2018`) only calls bare
> `executor.execute(9, input)` wrapped in the same per-attempt watchdog
> timeout `metered_execute` itself uses — **no retry, no fallback chain, no
> ledger entry, no S11 capture**. `metered_execute_resilient` needs `&mut
> OrchestrationState` (confirmed: specifically for `state.tokens_used_so_
> far` accounting inside `metered_execute`'s retry loop, not for the
> ledger/capture calls themselves, which take owned values or `&
> OrchestrationState` — a narrower blocker than "the whole wrapper needs
> `&mut state`"), which is why it can't run inside a concurrent spawn
> as-is — a real, correctly-identified blocker, just one that was worked
> around by dropping the guarantees rather than narrowing them. Net effect
> going forward: a cross-reference pair that would previously have had two
> real chances (same-model retry + a different-model fallback) to produce
> a usable relatedness judgment now gets exactly one attempt, and neither
> successes nor failures at this call site are visible to the model ledger
> or the S11 capture store anymore — this call site effectively disappears
> from this session's own "providers almost never the cause" style
> analysis and from the catalog's historical-success ranking input, purely
> because of how it's now invoked, not because it stopped mattering.
> **Not fixed — flagged for a decision**, same discipline as every other
> judgment-requiring item this session. **Fix shape, CORRECTED 2026-10-09
> (a parallel spec pass on intent extraction found something better than
> the original recommendation below)**: don't build a new `&mut`-free
> helper — `Self::walk_fallback_chain_standalone` (`mod.rs:4029`) already
> exists, confirmed genuinely `&mut`-free (no `self`, no `state` anywhere
> in its signature), and already provides same-model-retry + a real
> fallback-chain walk + real `model_ledger::record_with_text` coverage (it
> calls `run_one_candidate` internally, confirmed by reading — the same
> function the AMT lanes fix already uses). It's already proven safe under
> concurrent dispatch by that same lanes fix. The one honest gap: it does
> NOT call `capture_zero_shot_call` (S11 capture is specific to `metered_
> execute_resilient`'s own wrapper) — so this fix restores retry/fallback/
> ledger but not S11 visibility specifically; worth deciding if that's an
> acceptable residual gap or needs its own small addition. Simplest real
> fix: swap the cross-reference wave's bare `executor.execute(9, input)`
> call for `Self::walk_fallback_chain_standalone(...)`, reusing the exact
> pattern the lanes fix already uses — not new code, a known-good call
> already one function away. (Original recommendation — inline a new
> `&mut`-free resilient-call helper — kept below, struck through in spirit,
> superseded by the simpler reuse: ~~defer `state.tokens_used_so_far`
> accounting to the existing serial-application loop... and inline a
> `&mut`-free version of the retry + `try_fallback_chain` + `capture_
> zero_shot_call`/`model_ledger::record` sequence inside each spawned
> task~~.)

**Context it receives**: same `jurisdiction_summary`/`file_relationship_
summary` pair as §2, computed once at `amt.rs:1152-1153`, but — confirmed by
direct read, not assumed from symmetry with §2 — **only reaches the branch-
generation sub-stage** (`amt.rs:1379-1380`). Intent extraction, detail
extraction, and cross-ref do not receive it. Same rationale as §2: only the
"what work should exist" decision point was in scope for tonight's fix.

> **FIXED AND INDEPENDENTLY VERIFIED, 2026-10-09** (ZCode built the fix,
> CC verified it directly against source before trusting it — see below).
> Kept in full as the historical record per this doc's additive convention.

> **CRITICAL, FOUND 2026-10-08**: `methodology_branch_discovery_parallel`
> (`amt.rs:2514`, called from both `build_amt_from_graphs` at `amt.rs:1126`
> AND this builder at `amt.rs:1536` — live on both AMT-building routes, not
> one) dispatches each methodology batch as a parallel "lane" via
> `tokio::task::JoinSet` (spawn loop at `amt.rs:2674`, in `lanes`' own
> order), then collects with `while let Some(res) = set.join_next().await
> { results.push(r) }` (`amt.rs:2703-2708`). **`JoinSet::join_next()`
> returns results in COMPLETION order, not spawn order** — standard,
> documented tokio behavior, confirmed by reading the actual collection
> loop, not assumed. But `results[i]` is then treated as corresponding to
> `lanes[i]` in THREE places: the retry-trigger zip (`amt.rs:2735-2748`),
> the retry-replacement index (`amt.rs:2766-2777`), and — most
> consequentially — the **final, state-mutating application**
> (`amt.rs:2784`, `for ((ids, prompt, _, batch_members, _), result) in
> lanes.iter().zip(results.into_iter())`). Since each lane independently
> walks its own full fallback chain (`walk_fallback_chain_standalone`,
> different models, different latencies, different retry depths per lane —
> confirmed by reading the spawn body), out-of-order completion is the
> REALISTIC COMMON CASE, not a rare edge case. Net effect: branches parsed
> from one methodology batch's response can be silently attributed to a
> DIFFERENT batch's `ids`/`parent_intent` context whenever lanes finish out
> of spawn order; "UNCOVERED" warnings can blame the wrong methodologies;
> the retry logic can retry a lane that actually succeeded while leaving a
> genuinely-failed one unretried. This is pre-existing (not introduced by
> today's text-pipeline wave refactor) — the exact same ordering-bug CLASS,
> independently discovered in a different, already-live mechanism. **Not
> fixed this pass** (read-only review) — fix shape: carry each lane's own
> index through the spawn (`set.spawn(async move { (lane_idx, inner.await)
> })`), then write results into a `Vec` pre-sized to `lanes.len()` at the
> carried index instead of pushing in completion order.
>
> **FIX APPLIED (ZCode, 2026-10-08) AND INDEPENDENTLY VERIFIED (CC,
> 2026-10-09) — correct and complete.** Both the main lane `JoinSet`
> (`amt.rs:2673-2740`) and the retry `JoinSet` (`amt.rs:2784-2827` — ZCode
> found and fixed the IDENTICAL bug a second time here: `retried` was
> pushed in completion order but consumed by a submission-order `ri`
> counter) now carry each lane's own index through the spawn and write
> into a `Vec<Option<_>>` pre-sized to `lanes.len()`, keyed by that index —
> not pushed in completion order. **The panic case was verified
> specifically**, since a panicked task's `JoinError` carries no index (it
> never produced its return value, carried index included — there is no
> way to extract "which lane" from a bare panic). The fix handles this
> correctly by elimination, not by guessing: every slot starts `None`;
> every successfully-completing lane fills ONLY its own slot via its
> carried index; after the full drain, any slot still `None` is, by
> construction, exactly the lane(s) that panicked — no other lane could
> have left it unfilled. This is a genuinely correct solution to the
> "a panicking task can't self-report its identity" problem, not a partial
> fix. The downstream code (the retry-trigger zip, the final state-
> mutating application at `amt.rs:2831`) was left completely unchanged —
> it now simply receives a `results` Vec that is genuinely lane-ordered,
> which was the whole point of a minimal-diff fix. Verified via
> `cargo check --release` (clean) independently of ZCode's own 121/121 test
> pass. **Also swept the entire codebase for any other instance of this
> bug class**: grepped every `JoinSet`/`FuturesUnordered`/`join_all` usage
> in `src/` and `assets/pipelines/` — exactly one other `JoinSet` exists
> (`src/orchestrator/mod.rs:4294`, the User-tier parallel-candidate batch
> dispatch), independently re-verified as NOT the same bug class: it's a
> "race for the first usable result" design that never attributes a
> result back to a predetermined index (the carried `profile.identifier`
> is only ever used standalone — logging, and a `batched_ids` set built
> *before* spawning, unaffected by completion order or panics), so a
> panic there just means one fewer racer, not a misattribution. No other
> concurrent-collection primitive exists anywhere in the codebase — this
> bug class is now fully closed, not just patched at the two sites it was
> first found.
>
> **SEPARATE REAL GAP FOUND, 2026-10-09 (correctness was verified above;
> bounding was not — a different question, checked on a second pass).**
> Each lane calls `walk_fallback_chain_standalone`, which can walk a FULL
> fallback chain per lane (`amt.rs:2766-2769`'s own comment: "each lane
> walks the health-reordered fallback chain"), but the spawn body
> (`amt.rs:2770-2786`) wraps it in **no timeout at all** — unlike the
> sequential path (`metered_execute_resilient`, `mod.rs:3100-3119`), which
> deliberately sizes its own outer bound proportionally to chain length
> (`per_attempt_secs * (1 + walk_candidates + pool_extra)`) specifically
> because a multi-candidate walk can legitimately need several attempts'
> worth of time. A single lane whose primary fails and walks several slow
> fallback candidates can occupy a concurrency slot for a correspondingly
> large, unbounded-in-practice multiple of a single attempt's cost, with
> no explicit ceiling matching the sequential path's own. Not a
> correctness bug (the ordering/panic-safety verified above is unaffected)
> — a missing defensive bound, same severity class as "don't let fast
> mean unboundedly slow on one straggler." **Not fixed — flagged for a
> decision.** Fix shape: wrap each lane's `walk_fallback_chain_standalone`
> call in `tokio::time::timeout` sized by the same proportional formula,
> using THIS lane's real candidate count (`models`/`order`, already in
> scope at the spawn site) rather than a flat per-attempt value.

**K-Algorithm presets used**: `convergence` (`amt.rs:1126`, `max_outer_
passes`) — and its own comment records a real prior bug: the outer-pass cap
used to be hardcoded here while the sibling `build_amt_from_graphs` already
respected the configurable preset, so the pass cap was "never actually under
user control despite that system existing" — now both builders read the
same live preset. `pairwise` (`amt.rs:1824`, `max_pairs`) governs the
cross-ref sub-stage's pairwise-comparison discipline (window/cap) — confirms
`CONTRACTS.md`'s claim that `pairwise` has a live consumer here, specifically
in this sub-stage.

**Stage**: 5 (Build AMT), the other route `build_amt` dispatches to.

## 4. `cross_reference_methodologies_for_layer` (`amt.rs:2593-2735`)

**Purpose**: Given the AMT branches built by §2 or §3, identifies which
methodology *domains* are actually needed (not yet-registered methodologies
— this can trigger drafting a brand-new one) and synthesizes a concise draft
when a needed domain has no existing methodology covering it. This is the
mechanism by which AMT building can grow the methodology store mid-request,
not just consume it.

**Zero-shot calls it makes**: registry #7 (`amt.rs:2642`, domain
identification, `max_tokens: 200`) and #8 (`amt.rs:2692`, methodology
synthesis, `max_tokens: 400` — flagged in `ZERO_SHOT_CALL_REGISTRY.md` §6 as
the one site with a token budget in the same order of magnitude as a
*confirmed* truncation failure elsewhere in the codebase, not yet raised).

**Context it receives**: neither `jurisdiction_summary` nor
`file_relationship_summary` — confirmed by grep, no references to either in
this function. Consistent with the design rationale above: this function
identifies/drafts *methodology* domains, not user-facing work items, so
jurisdiction/file-relationship framing is a weaker fit than for §2/§3's
branch generation — not flagging this as a gap, noting it as a deliberate
scope boundary worth being explicit about.

**K-Algorithm presets used**: none found by grep in this function.

**Stage**: 5 (Build AMT), called from within both §2 and §3's per-
methodology loops (not a separate stage of its own).

## 5. `merge_back_to_main` (`amt.rs:284-360`)

**Purpose**: Grafts a fork AMT's verified branches into the project's main,
persistent AMT tree — the mechanism that turns per-request AMT trees into a
project's genuinely growing knowledge structure across multiple prompts,
rather than each request producing a disposable, disconnected tree. This IS
the `Continuation` route `docs/AMT_EXPANSION.md` names, confirmed real and
already landed this session (that doc's "v2, next" label for this was
stale — corrected earlier tonight).

**Zero-shot calls it makes**: none — pure content-string dedup (does a
verified fork branch's content already exist in main?) and a raw
`std::fs::write` of the merged tree. Deliberately deterministic; grafting
already-verified content doesn't need a new judgment call.

**Context it receives**: N/A — operates on already-built `AMTNode` trees,
not on `OrchestrationState` directly.

**K-Algorithm presets used**: none.

**A real, known gap** (already documented this session, repeated here for
completeness of this registry): the graft's own `std::fs::write` (`amt.rs:
358`) bypasses `ZSEI::query`, the system's real graph-write choke point
(`src/zsei/mod.rs:129`) — so a merge-back graft does not itself emit a
ripple event, unlike every other real graph mutation in this codebase. Low
severity (the fork AMT's own creation already rippled once) but a real,
still-open gap, not fixed by tonight's context-wiring work.

**Stage**: called from within `build_amt`'s persistence step (`amt.rs:63`,
inside the `state.amt_validated` branch), so effectively still Stage 5, at
the very end of it.

## 6. `stage_3_blueprint_assignment` (`stages.rs:253-505`)

**Purpose**: Turns the completed AMT into a real, executable step-by-step
plan — the blueprint. This is the single highest-stakes builder in the
system: its output (`step.pipeline_id`, `step.action`, `step.model_override`)
is literally what gets executed later. Confirmed this session as one of only
two sites with genuine `Ok`-but-empty protection (via the new
`metered_execute_resilient` helper, `stages.rs:500`) rather than accepting a
technically-successful-but-empty draft.

**Zero-shot calls it makes**: registry #10 (`stages.rs:500` now — line
number shifted from the registry doc's `:356` because `jurisdiction_summary`/
`file_relationship_summary` were inserted earlier in this same file tonight;
same call, same `max_tokens: 1000`/`temperature: 0.3`, just moved).

**Context it receives**: everything — `jurisdiction_ctx`/`file_
relationships` (`stages.rs:391-392`, interpolated at `:412-415`+), the full
AMT (`AMT ROOT`/`BRANCHES`), available pipelines, available models (for
per-step `model_override`), and real methodology rule text via
`load_methodology_rules_text` (`stages.rs:381`, matching `docs/ZERO_SHOT_
CALL_REGISTRY.md`'s own file-map entry for this function). This is the
richest-context builder in the registry — consistent with it being the
highest-stakes one.

**K-Algorithm presets used**: none found by grep in this function —
blueprint assignment is itself a single zero-shot call with rich context,
not an iterative loop needing a convergence/pairwise discipline.

**Stage**: 6 (Blueprint Assignment) — confirmed real stage order this
session, `stages.rs:9-97`: Jurisdiction(0) → Text Norm(2) → Methodologies(3)
→ File Classification(4a) → Initial Graph Creation(4b) → **Build AMT(5)** →
**Blueprint Assignment(6)** → Zero-Shot Simulation(7) → Consciousness
Gate(8).

## 7. `create_initial_modality_graphs` (`graphs.rs:75-276`) — stage 4b

**Purpose**: Builds modality graphs from the *prompt's own* detected
modality spans (not attached files — see §8 for that), in three passes:
structural creation → semantic enrichment (a `TriggerSemanticHook` call per
graph, `OnInferRelationships`) → cross-modal reference building until 5x
stable (`build_cross_modal_references_until_stable`, `graphs.rs:279`, itself
calling `run_cross_modal_reference_pass`, `graphs.rs:315`, on each pass).

**Zero-shot calls it makes**: none directly in the pass-1 structural loop
shown — pipeline calls are to the modality pipelines themselves (text/code/
math's own `Analyze`/`ParseExpression`+`CreateGraph` actions, real work but
not pipeline-9 zero-shot calls; math's branch here was fixed this session,
`graphs.rs:104-151`, to use `ParseExpression`+the correctly-shaped `analysis`
field rather than the generic `Analyze` action math never had). Not
independently traced whether `run_cross_modal_reference_pass`'s own
relationship-judgment logic makes a zero-shot call — flagged as unverified
in this pass, worth a follow-up read rather than assumed either way.

**Context it receives**: operates over `state.root_modality_list.verified_
modalities` — confirmed elsewhere this session (`ZERO_SHOT_CALL_REGISTRY.md`
§11, `LIVING_GRAPH_STATUS.md`) that for attachment-driven requests this list
is effectively `["text"]` only, because the upstream file-role-
classification call (registry #1) commonly returns an empty response,
leaving `classified_file_graphs` empty. **This builder is real and correctly
implemented, but largely dormant for the file-attachment case that matters
most in practice** — say so plainly rather than overstating its live-path
significance, per this session's own honesty discipline.

**K-Algorithm presets used**: none found by grep in this function or its
two sub-functions.

**Stage**: 4b (Initial Graph Creation).

## 8. `process_modality` (`graphs.rs:555-642`) — the one that actually works

**Purpose**: The per-attached-file graph builder, called from `mod.rs`'s
STEP 0 (stage 2, `prompt_normalization`) — confirmed this session, via a
real 3-file (text/code/math) integration test with independently-verified
on-disk data, to be the mechanism that actually produces the live,
verifiable cross-modal `SimilarTo` relationship edges this whole session's
living-graph work has been about. Where §7 is the modality-graph builder
that's real-but-dormant in practice, this is the one that's real and
*live* — the most consequential single function in this registry for the
"does this system actually build a living graph" question the user has
been asking all session.

**Zero-shot calls it makes**: none of its own (dispatches to the modality
pipeline's own `Analyze`/`ParseExpression` action, same category as §7) —
but it's immediately followed, at its real call site in `mod.rs`'s STEP 0,
by a `CreateGraph` call with `link_to_existing: true` (unlike §7's `false`)
— this is the actual mechanism that produces real relationship edges,
confirmed via `link_related_containers` inside the modality pipelines
themselves (not re-traced in this pass, already established fact from
earlier this session).

**Context it receives**: raw file content (read from disk server-side via
`file_path`, capped at `MAX_FILE_ANALYSIS_BYTES`), not `state`-level
jurisdiction/relationship context — this function runs *before* jurisdiction
or AMT context would even be relevant to it; it's producing the raw
material §2/§3/§6 later consume via `file_relationship_summary`.

**K-Algorithm presets used**: none.

**Stage**: 2 (Text/Prompt Normalization), STEP 0 — before Build AMT (5),
which is exactly why its output is available for `file_relationship_summary`
to consume by the time §2/§3/§6 run.


## 9. `persist_amt_container` (`amt.rs`) — the island builder (added 2026-09-22, zcode)

**Purpose**: Constructs the AMT's persisted IDENTITY — the half of the AMT
lifecycle §1-5 don't cover. Implements the MAIN/FORK island model (first
AMT per project = MAIN with the `amt-main` keyword; every later generation
= FORK with `amt-fork-of:{prior}` + a root-level `Continues` relation to
the prior container), writes the project-scoped content file
(`amt_p{project}_{millis}_{id}.json` — the min-id canonical resolution
CONTEXT_REGISTRY's cleanup depends on), records the re-expansion route
(InitialBuild/Continuation/ThinTree) into the unified candidate store, and
triggers merge-back. Without this builder the per-request trees of §2/§3
would be born disposable.

**Zero-shot calls**: none — persistence + lineage bookkeeping.
**Context**: operates on the built tree + `state.request` (project id).
**K-Algorithms**: none. **Stage**: end of 5.

## 10. The growth phase — `review_amt_candidates_once` + `try_reexpand_one` + `add_children_to_unverified` (`amt_loop.rs`) (added 2026-09-22, zcode)

**Purpose**: The AMT is not built-once. The re-expansion loop GROWS trees
after build: candidates (UnverifiedNode/GraphRipple/Continuation/ThinTree)
are reviewed, a real model call deepens one branch (guidance = the branch's
methodology rules + related-branch content; ThinTree route targets a
childless root), and the deepened children persist to the tree file. The
registry's §1-8 tell a one-shot construction story; this phase is the
living-system half — built → grown → merged (§5).

**Zero-shot calls**: registry #17 (cross-pass escalation via
`meta_fallback.order` — the second fallback idiom, per the registry's own
note). **Context**: container signals (keywords/topics), methodology
guidance, related branches. **K-Algorithms**: none in-call.
**Stage**: background (interval 1800s / ripple-woken).

## 11. The coordination builder — `context_mirror::mirror` (added 2026-09-22, zcode; corrected 2026-09-22, claude-code)

**CORRECTION**: this section originally described a "mirror + `merge_back`"
pair, analogous to AMT's real builder/graft pair. Verified directly against
`src/context_mirror.rs` (full file read): that file contains exactly **one**
public function, `pub async fn mirror` (`context_mirror.rs:132`). There is
no `merge_back` function and no graft-like logic anywhere in the file —
grepped `graft`/`Continues`/`verified.*branch`, zero hits. The "pair" framing
was inaccurate; only the mirror half is real.

**Purpose**: `mirror` constructs real `/SharedContext` structure from a
coordination event (kind/agent/title/body/files/detail — a claim, handoff,
finding, etc.): dedupes claim-kind events to one container per file path
(scans the root's children for an existing `claim:<path>` keyword before
creating a new one), persists the full event body as content, and creates
a real container under `SHARED_CONTEXT_ROOT_ID` scoped with real keywords.
This is the mechanism behind every `note_add`/`file_claim` call this
session's shared-context MCP tools ultimately write through.

**What does NOT exist yet**: a `merge_back`/graft counterpart, analogous to
AMT's real `merge_back_to_main` (`amt.rs:284-360`) — something that takes
verified/related mirrored content and grafts it back together the way
AMT's fork-to-main lifecycle does. `mirror` only ever creates new, mostly
flat sibling containers (dedupe aside); nothing currently consolidates or
relates them to each other after the fact. A real, not-yet-built gap, not
a documentation oversight to wave away.

**Zero-shot calls**: none. **K-Algorithms**: none. **Stage**: event-driven.

## Cross-cutting findings

1. **The context-wiring fix (tonight) targeted exactly one decision point
   per builder — branch/plan generation — not every sub-call.** Intent
   extraction, detail extraction, cross-ref, domain identification, and
   methodology synthesis all remain jurisdiction/relationship-blind by
   design, not oversight; only the calls that decide "what work exists"
   (§2/§3's branch generation, §6's blueprint assignment) were in scope.
   Worth knowing before assuming the fix is broader than it is.
2. **`convergence` and `pairwise` are confirmed live exactly where
   `CONTRACTS.md` says they are** — CORRECTION 2026-09-22 (zcode): the
   "only these two have a live consumer" note in CONTRACTS.md is now
   STALE — the `relevance` family (graph-first / keywords-only presets,
   shared/contracts/k_relevance.rs) has a live consumer: the
   link_related_containers path in text/code/math reads
   OZONE_RELEVANCE_POLICY, exported at boot from KAlgorithms.relevance
   (code/main.rs relevance_policy()). (`amt.rs:1094/1126` and `amt.rs:1824`),
   with one real bug-fix history worth preserving: the outer-pass cap used
   to be hardcoded in the legacy builder while the graph-native builder
   already respected the configurable preset — now both agree.
3. **§7 vs §8 is the most important distinction in this doc.**
   Superficially similar-sounding functions (`create_initial_modality_
   graphs` vs `process_modality`) have opposite real-world significance
   today: one is correctly implemented but practically dormant (blocked by
   an upstream bug), the other is the one actually proven to work live.
   A reader skimming function names alone would not know which is which.

## Files

- `src/orchestrator/amt.rs` — `build_amt` (gateway), `build_amt_from_graphs`,
  `build_amt_layer_by_layer`, `cross_reference_methodologies_for_layer`,
  `merge_back_to_main`, `jurisdiction_summary`/`file_relationship_summary`
  callers (helpers themselves live in `stages.rs`)
- `src/orchestrator/stages.rs` — `stage_3_blueprint_assignment`,
  `jurisdiction_summary` (`:135-206`), `file_relationship_summary`
  (`:208-260`), real stage order (`:9-97`)
- `src/orchestrator/graphs.rs` — `create_initial_modality_graphs`,
  `build_cross_modal_references_until_stable`,
  `run_cross_modal_reference_pass`, `process_modality`
- `docs/ZERO_SHOT_CALL_REGISTRY.md` — every individual model call these
  builders make, cross-referenced by registry number throughout this doc
- `docs/CONTRACTS.md` §3 — the K-Algorithm registry these builders draw
  `convergence`/`pairwise` presets from
- `docs/AMT_EXPANSION.md` — the architectural narrative (gateway/routes/
  candidate store) this doc's §1-5 confirm against current source
- `docs/LIVING_GRAPH_STATUS.md` — the living-graph status this doc's §7/§8
  distinction directly informs
