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

**Context it receives**: same `jurisdiction_summary`/`file_relationship_
summary` pair as §2, computed once at `amt.rs:1152-1153`, but — confirmed by
direct read, not assumed from symmetry with §2 — **only reaches the branch-
generation sub-stage** (`amt.rs:1379-1380`). Intent extraction, detail
extraction, and cross-ref do not receive it. Same rationale as §2: only the
"what work should exist" decision point was in scope for tonight's fix.

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
