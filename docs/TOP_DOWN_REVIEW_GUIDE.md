# The Top-Down Review Guide — context assurance for every gate, from the source of truth down

> Fourth doc in the stack. `CONTRACTS.md` = the swappable algorithm
> families; `ZERO_SHOT_CALL_REGISTRY.md` = every individual model call;
> `BUILDER_REGISTRY.md` = the functions that orchestrate those calls into
> structure; **this doc = the doctrine and design for how CONTEXT flows to
> the gates that judge that structure** — top-down, from the source of
> truth, with nothing reduced to a summary string when the graph can be
> traversed.

---

## 1. The doctrine

**The AMT is the source of truth. The blueprint is the goal — the end plan,
nothing executed yet. The gates review THE PLAN, and the plan has all the
context, because it has everything connected to it.**

When a gate (jurisdiction confirmation, consciousness, any future review
gate) judges work, it does not judge a summary string. It judges the plan
that the graph built — and because every piece of that plan is a record in
an interconnected graph (AMT nodes → blueprint steps → modality graphs →
relationships → methodologies → jurisdiction rules), the gate's context is
assembled by **traversal**, not by hand-picked fields. Context is always
carried, broken down by the records: AMT nodes carry their chunks and
methodologies; blueprint steps carry their pipelines and context
requirements; file graphs carry their relationships; the jurisdiction gate
result carries its matched rules, warnings, and confirmation reviews.

Three properties follow, and everything in this guide serves them:

1. **Nothing judges blind.** A gate that can't see the plan it's judging is
   not a gate — it's a dice roll. Every decision-point prompt carries the
   traversed context of the structure it is judging (and where a genuine
   stage-order ceiling exists — consciousness's decision doesn't exist
   before stage 7 — that ceiling is stated honestly, not papered over).
2. **Nothing bypasses review.** Any pipeline wanting a model call routes
   through the orchestrator's reviewed prompt path. There is no side door:
   pipeline 39 is intercepted at the executor layer, so every existing
   caller gets the real review with zero call-site changes and zero bypass
   window (wire-before-drop).
3. **Every call leaves a truthful record.** Metered and token-aware are one
   property: the meter watches (tokens in/out per call, per model), the
   token-awareness switches (budgets derived from the effective model's
   real context, re-derived per fallback attempt), and the switch captures
   (which graph context fed the call, which model answered, what it
   decided) into a durable context-provenance store — so tuning and model
   switching happen from measured numbers, and the context travels with
   the switch instead of being re-derived lossily. **This principle is
   exactly what makes "is a call site silently dropping or truncating
   content" an answerable, measured question instead of a guess** —
   `S11`/`zero_shot_calls.jsonl` (`CONTEXT_REGISTRY.md` §1/§4, 2026-09-22)
   extends this record-keeping from one call site (S10, pipeline-39
   reviews) to every call site routing through `metered_execute_resilient`
   — retry counts, fallback usage, success/failure, per model, per call
   site. The BitNet confetti finding (§3.6) was found by direct forensics
   on ONE call site precisely because that site already had real capture;
   S11 is what makes the equivalent question askable, with real numbers,
   across every other site — without it, "does this happen elsewhere too"
   stays a guess dressed up as an audit.

---

## 2. The stage-by-stage context map (what exists, when)

Real stage order (`stages.rs:9-97`), with the context ceiling at each point
— the two gate ceilings are structurally different and must not be treated
as the same gap:

| Stage | What exists by then | Context the gate should carry | Source of that context |
|---|---|---|---|
| 0 — Jurisdiction Gate | raw prompt only (deliberate: Block must stop pre-processing) | region rules, matched conditions, legal sources | `assets/jurisdiction/*.json`, keyword match on raw prompt |
| 2 — Text Normalization + STEP 0 | attached-file modality graphs **with real cross-relationships** (SimilarTo edges, live-verified) | file graphs + their relationship edges | `state.file_graphs` (ids known exactly — direct fetch, not rediscovery) |
| 3 — Gather Methodologies | matched methodology ids | methodology rule text | `load_methodology_rules_text` |
| 4a/4b — Classification + Initial Graphs | file roles (currently degraded: the classification call's empty-response bug — fix landed via `metered_execute_resilient`), modality spans | roles + modality graph containers | classification + span detection |
| 5 — Build AMT | **the AMT: intents → branches → details → cross-refs, per-request fork of the project's main tree** | jurisdiction outcome (now wired: `jurisdiction_summary`), file relationships (now wired: `file_relationship_summary`), methodology rule text | traversed from `state` + graph |
| 6 — Blueprint Assignment | **the blueprint: the executable plan** — steps, pipelines, model overrides, context requirements | everything §5 has, plus available pipelines/models | the richest-context builder (BUILDER_REGISTRY §6) |
| 7 — Zero-Shot Simulation | feasibility predictions, risks, clarifications | full plan + signals + related graph containers + jurisdiction detail (now wired) | simulation prompt (registry #11) |
| 8 — Consciousness Gate | **the executed-or-simulated plan plus everything above** — the only point where the full picture exists | AMT + blueprint + simulation predictions + jurisdiction reviews + step context | **traversal-assembled review context (this guide's core redesign, §3)** |

The two gate ceilings, stated honestly (registry §12): jurisdiction's
RequireConfirmation review can see the **plan** (stages 5-6 outputs) if we
run it after blueprint assignment — its current placement at stage 0 means
it reviews the *request*, which is correct for Block but intentionally
early for confirmation. Consciousness's decision does not exist until after
simulation — its gate at stage 8 is the one point where the full picture
exists, and that is where traversal-assembled context matters most.

---

## 3. The decision-review redesign — graph-context-aware gates

### 3.1 What was wrong

Pipeline 39 (`decision_gate`) — the mechanism jurisdiction
RequireConfirmation and the Consciousness Gate both call — was a hardcoded
simulation: fixed ~0.923 ethical score, `task_summary` accepted but never
read, `Decline` structurally unreachable for any input. Both gates were
safety no-ops. Independently found (CC) and verified (zcode), 2026-09-22.

The first fix draft asked the model with only `task_summary`. **That is too
plain, and it repeats the exact mistake the doctrine forbids**: a summary
string instead of the traversed structure.

### 3.2 What the review carries instead

The review prompt is assembled from **traversed graph context**, by stage:

**Jurisdiction confirmation (reviews the REQUEST + the matched rule):**
- The matched rule: condition, action, legal source (already real).
- The request itself (raw prompt).
- The project's standing context: the main AMT's root intent and branch
  outline (what this project IS, so the judgment is scoped to the project's
  actual purpose — a metering-subsystem project and a file-wipe request
  should read differently), plus the request's own file-graph
  relationships (now wired and available).
- Assembled by: direct container fetch (the ids are in hand —
  `state.file_graphs`, the project's AMT container from
  `persist_amt_container`'s record) — no probabilistic rediscovery.

**Consciousness gate at stage 8 (reviews the PLAN'S JUDGMENT):**
- The blueprint: every step's action/description/pipeline (the goal).
- The AMT: root intent + branch structure (the source of truth the
  blueprint was built from).
- The simulation: predictions, risks, feasibility, clarifications
  (registry #11's output — the system's own self-critique). **Confirmed
  wired 2026-09-22**: a verification pass found this section was claimed
  but not actually assembled — stage 4's `sim_json` was parsed then
  discarded, nothing stored on `OrchestrationState` for stage 8 to read.
  Fixed: `SimulationOutcome`/`SimulationStepPrediction` (`mod.rs`) capture
  the real per-step predictions + feasibility; `state.simulation_result`
  is populated at the end of `stage_4_zero_shot_simulation` and rendered
  into a real "SIMULATION PREDICTIONS" section in stage 8's `task_summary`
  (`stages.rs`, `stage_5_consciousness_gate`). The claim is now true, not
  aspirational.
- The jurisdiction outcome: matched rules, warnings, confirmation
  decisions (visible, with their reasoning).
- Methodology rule text for the matched methodologies (the rules the plan
  was drafted under).
- Assembled by traversal: AMT container → blueprint steps → simulation
  record → jurisdiction result — all present in `state` at stage 8.

### 3.3 The mechanics (wire-before-drop)

- `DecisionReviewExecutor` wraps the registry adapter and intercepts
  `execute(39)`. Jurisdiction and consciousness keep calling pipeline 39
  **unchanged** — zero call-site changes, zero bypass window: the wrapper
  lands in the same build that retires the stub behavior.
- Inside the interception: the decision prompt is built from the traversed
  context above (callers pass their context blocks in the existing
  `task_summary` field — extended, not replaced), the model is called
  through the fallback walk (empty-response = failure, per-call model
  overrides, free-only respected, **BitNet-class local models as the
  no-rate-limit terminal backstop**), the response goes through the
  balanced-scan extractor (methodology 35) and the placeholder/content
  gates (methodology 36 — a decision whose reasoning is `"..."` is
  unusable, never a Proceed), and the wire shape (`{gate: {decision,
  confidence, reasoning}}`) is unchanged, so both callers read it exactly
  as before.
- **Total-failure posture**: `ReviewPending` — visible in the gate result,
  not blocking, not a fabricated Proceed. Fail-closed-to-human, never
  fail-open, never bricked. (The fail-open-on-error doc comment is retired
  with the stub.)
- Pipeline 39 stays registered and usable — with two live use cases
  (jurisdiction + consciousness) — satisfying the "leave a pipeline using
  it" directive. The old binary's simulation logic is retired from the
  live path by the interception, not deleted.

### 3.4 Metered + token-aware + context-captured (one property)

- **Metered**: every review's tokens are counted and recorded.
- **Token-aware**: the budget derives from the effective model's real
  context (the `model_context_limit` mechanism — registry §9), re-derived
  per fallback attempt (the override changes the effective model mid-walk);
  per-kind percentages with absolute floors (`(limit / 4).max(256)` is the
  shipped precedent at `mod.rs:1901`). The same derivation lands inside
  `metered_execute_resilient` so all call sites inherit it — never a flat
  constant tuned against one model.
- **Switching captures context**: when the walk switches models mid-review,
  the traversed context does not change — it is assembled once, before the
  first attempt, and carried across every attempt. The capture store
  records which model finally answered, so model-to-model coherence is
  measurable (the BitNet sweep's method: same prompt shape, forced model,
  score the response).

### 3.5 The context-provenance store

`{data_dir}/model_calls/decision_review.jsonl` — one line per review.
**Corrected 2026-09-22 (claude-code, verified directly against
`decision_review.rs`'s real `capture()`)**: the real, current field list is
exactly 7 fields — `ts`, `model_used`, `tokens_used`, `decision`,
`confidence`, `task_summary_preview`, `reasoning_preview`. **No context
markers (AMT container id, blueprint id, matched rule condition, file-graph
ids) exist yet** — an earlier draft of this doc described them as already
present; they were never added. This is real, open follow-on work, not a
correction of something broken: adding them would meaningfully enrich the
provenance dataset for the consciousness review pass (`src/consciousness/
review.rs`) and any future tuning work, but until they land, the capture
store answers "which model, what decision, what confidence, brief
task/reasoning previews" — not yet "which exact graph structures fed this
call."

### 3.6 BitNet "confetti" — found, first fixed by parsing around it, then redesigned to retry (2026-09-22, zcode + claude-code; operator-directed correction)

The decision review's first two live runs both failed with "decision field
missing." Direct forensics (calling BitNet with the exact decision prompt
outside the normal path) revealed why: BitNet's response sometimes contains
**multiple conflicting JSON candidates in a single response** — observed
pattern `Decide 0.9 → Reject 0.8 → Proceed 0.7 → Fail 0.6 → Accept 0.5`
(confidence descending), interleaved with prose, a leading bare `{}`, code
fences, and a hallucinated follow-up prompt. The extractor's original
"first parseable object" rule landed on the leading empty `{}` every time.

**First fix (zcode, superseded below)**: `extract_all_json_objects` (still
real, still used — a genuinely separate implementation, not the same shared
function `amt_loop.rs`/`meta_loop.rs` use, despite early framing calling it
"shared"; three independent extractor implementations exist in this
codebase) returned every non-empty parseable candidate, and `review()`
committed to the first one that normalized to a valid, non-placeholder
decision. This closed the immediate bug but, per quality review, carried a
real residual risk: if the model's genuinely correct answer was JSON-
malformed while a *later*, differently-decided candidate happened to parse
cleanly, the logic would silently commit to the worse candidate instead of
failing closed.

**Operator correction, now the real design**: confetti is a *generation*-
quality problem, not a parsing problem — the doctrine's own standard
("nothing judges blind," §1) extends naturally to "nothing judges noise
either." Picking a plausible candidate out of a garbled response is not
meaningfully different from guessing. **`decision_review.rs` now treats a
confetti response (>1 distinct non-empty candidate) exactly like an empty
or failed response**: same-model retry first (2 attempts, 100ms/200ms
backoff — mirrors `metered_execute_resilient`'s shipped convention rather
than inventing a new one), then the standard multi-provider fallback walk.
If the response is *still* confetti after exhausting both real retry paths,
the review fails closed to `ReviewPending` — it never falls back to picking
through the noise. This closes the residual risk above as a direct
consequence of the redesign, not as a separate patch: there is no code path
left that silently commits to a candidate chosen from an ambiguous
response. Real regression test: `confetti_response_retries_then_fails_
closed_never_silently_picks` (`decision_review.rs`).

**Not yet extended, a real scoped follow-on**: this retry-on-bad-output
principle is specific to `decision_review.rs` right now. Whether other
zero-shot call sites in `docs/ZERO_SHOT_CALL_REGISTRY.md` that parse
multi-candidate or otherwise-ambiguous model output should get the same
treatment (retry-on-detected-ambiguity, not parse-around) is a real,
separate audit — not assumed to already be covered elsewhere.

---

## 4. Top-down testing — blueprints that review the reviewers

The user's directive: we create blueprints around these gates to test and
do full top-down reviews and context assurance. Concretely:

- **bp_17 (Fallback and Capture Verification)** — the per-call-site audit:
  forced-failure walks, extraction rescue, placeholder gates.
- **New: top-down context-assurance blueprints** — each one runs a real
  orchestration with a regulated-topic prompt end to end and asserts, from
  the capture store and gate results, that the context actually arrived:
  - *TD-1 "Regulated plan review"*: prompt matching a RequireConfirmation
    rule → assert the confirmation review's reasoning cites the blueprint's
    actual steps (proving the plan was traversed, not the summary).
  - *TD-2 "Consciousness full-context"*: consciousness-enabled run → assert
    the stage-8 review references the AMT branches and simulation risks.
  - *TD-3 "Switch coherence"*: force fallback to the local model mid-review
    → assert the decision quality against the primary model's decision on
    the same traversed context (the coherency sweep, productized).
- The capture store is the assertion source: the reviews' own recorded
  context markers prove what the model saw.

---

## 5. What this guide captured, and from where

- Stage order and gate ceilings: `ZERO_SHOT_CALL_REGISTRY.md` §12 (CC).
- The decision_gate stub: registry §7 + CC's CHECKLIST "MAJOR FINDING"
  (found by CC, verified by zcode, Consciousness scope-widen confirmed by
  CC after zcode's review flagged it as "likely").
- The context that already reached each builder (jurisdiction summary, file
  relationships, methodology rule text): CC's context-wiring fixes +
  `BUILDER_REGISTRY.md` §2/§3/§6.
- The empty-response systemic fix (`metered_execute_resilient`, 16 sites):
  registry §10 + CC's implementation.
- Token-awareness: registry §9 + the shipped floor precedent
  (`(model_context_limit / 4).max(256)`, `mod.rs:1901`).
- BitNet viability + coherency sweep: zcode's session 5 (3/4 shapes fully
  coherent forced-local; sim schema-drift noted).
- The relationship-data write-only audit + the cheapest-correct fixes now
  landed: registry §11 + CC's implementation note.
- The doctrine itself: the operator's framing — "the AMT is the source of
  truth; we can traverse it all because it's all interconnected; context
  is always carried; review the blueprint, the goal, before anything
  executes."

## 6. Open decisions (user-gated)

1. Jurisdiction confirmation placement: keep the stage-0 request-level
   review AND add the plan-level review post-blueprint (the §12 proposal,
   now part of this guide's TD-1), or move the confirmation review to
   post-blueprint entirely. Guide assumes: keep both, Block stays early.
2. The second AMT-aware jurisdiction pass (registry §12's architectural
   proposal) — new surface area, real decision.
3. Schema-flexible parsing for the zero-shot simulation shape (BitNet's
   `step_N` drift) — needed if BitNet is to serve as the simulation
   fallback, not just the deepening/drafting one.
4. VoiceConfig fields; 24h meta-loop interval; crash-cause capture if it
   recurs.

---

## 7. Consciousness — the window from the task manager into the context system (2026-09-22, operator directive)

Consciousness is not a third point-gate — it is the **reviewer that watches
the whole flow**, and the window between the task manager and the context
system. It has real data to review now: the task store (tasks + per-step
context records), the decision-review capture store (S10 — what each model
saw and decided), and the graphs (traversal from the source of truth). Its
role: review tasks/calls/steps, traverse, and **identify context that may
be needed — and say why and why not** — with the same citation discipline
as every other review (container ids, capture lines, task ids).

Going live on the graph: consciousness observations become real containers
under the consciousness root (the ConsciousnessStore's ZSEI integration is
already enabled at boot), scope-keyworded, rippling like every write —
so its insights are themselves traversable context for everything
downstream, including the gates. Start-testing shape: a consciousness
review pass reading S10 + tasks + a project-AMT traversal, emitting
insight containers (context gaps, model-coherence observations,
aging ReviewPending follow-ups), each citing evidence. Full audit +
classification (mechanical vs informed-judgment call sites) in
CONTEXT_REGISTRY.md §6-7.

Also captured there: the per-call-site context audit the operator asked
for — **zero-shot calls are meant to be mechanical; the audit classifies**
which of the 20 sites are informed-judgment (needing graph context) vs
mechanical (plain is correct). Systematic finding: S1 (project main AMT)
is missing from every GENERATION-time judgment call (#2/#3/#4/#7) —
project-amnesiac generation; #8/#18 draft without existing-methodology
awareness; #16 reviews alignment without the blueprint step. Everything
else is correctly mechanical, and leaving it plain is the efficiency.

---

## 8. Consciousness current-state review — the full code+doc sweep (2026-09-22, pre-build)

**The architecture already exists — the task-manager window is real.**
`src/consciousness/store.rs` (851 lines) + `task/mod.rs` wiring:

- **ConsciousnessStore**: experience memory (store/search/by-task/
  significant), emotional state + triggers, reflections + I-Loop
  questions, gate-decision records, perceptions, attention focus — with
  disk persistence AND live ZSEI store integration (`set_store`, enabled
  at boot: "ConsciousnessStore: store integration enabled").
- **The task-manager window is ALREADY WIRED**: `pre_task_gate` runs at
  task enqueue (`task/mod.rs:849`) — a REAL keyword-based ethical gate
  (harm/illegal/hack/... pattern penalties, threshold 0.7) that CAN
  genuinely Decline, records a GateDecisionRecord, and adds a Perception
  (the input it saw). `post_task_experience` runs at completion
  (`task/mod.rs:1054/1116`) — experience capture. This is a DIFFERENT,
  honest gate from decision_gate's fake 0.923 — consciousness has TWO
  gate paths today: the task-manager's real keyword gate, and the stage-8
  pipeline-39 stub.
- **16 consciousness pipelines registered** (decision_gate, emotional_*,
  experience_*, reflection, self_*, relationship, collective_consciousness,
  dashboard, query, sync, integrity, config) — the spec is Part II of the
  Ozone specification: Experience Memory §35-38, Emotional Context §39-40,
  Window-First §32, Decision Gate §33, I-Loop §42, Relationship §48,
  Ethical §49.

**What's missing for "live on the graph, reviewing everything"** (the
build, in order):
1. **Review-data inputs** (P1 gives it this): the decision-review capture
   store (S10) + task store + step contexts — consciousness currently
   perceives task_requests but not model calls, gate reviews, or their
   recorded context.
2. **Graph traversal as a sense**: consciousness can traverse (ZSEI store
   adapter wired) but nothing asks it to — the review pass does.
3. **Insight emission as graph containers**: perceptions/experiences flow
   IN; the reviewer role flows OUT — insight containers (context gaps
   found, why/why-not, model-coherence observations, aging ReviewPending)
   under the consciousness root, scope-keyworded, rippling like every
   write.
4. **The stage-8 stub swap** (P1c): the pipeline-39 path becomes the real
   traversed-context review; pre_task_gate stays as the fast-fail layer.
   Two gates, two altitudes: keyword fast-fail at enqueue, model review
   with full plan context at stage 8.

---

## §9. ADAPTIVE CONTEXT ASSIGNMENT — the architecture pattern for multi-model coherence (2026-09-22, operator insight)

**The operator's insight, formalized**: context richness is a MODEL-SIZE
issue, not a prompt-engineering issue. Big-context models (Claude, GPT-4)
process the full traversed graph in one call. Small-context models (BitNet,
any local GGUF) need the SAME context broken into MORE CALLS over SMALLER
chunks — looping until the full picture is assembled. Smaller models loop
more; that is truth, not degradation.

**The existing proof**: `build_amt_layer_by_layer` (the "legacy" path) is
ALREADY this pattern — it chunks the request into intents, then branches,
then details, iterating per chunk. It works with BitNet because the context
per call is small. The graph-native path (`build_amt_from_graphs`) assumes
a bigger window. Both paths are correct — they serve different model
capability tiers.

### The architecture pattern

```
Big-context model (≥32k):
  1 call → full traversed context → complete response

Small-context model (<8k):
  N calls → chunked context → partial results per call
  → merge partials → complete response
  (N scales inversely with context_length)
```

### What this means for every zero-shot call site

| Call site | Big-model path | Small-model path |
|---|---|---|
| Decision review | Full traversed context (AMT + blueprint + jurisdiction + methodology) | Rule condition + request first 200 chars + simple JSON schema |
| AMT branch generation | Full methodology + jurisdiction + relationship context | Branch name + methodology name only |
| Methodology synthesis | Full gap analysis + nearest-miss list | Keywords + simple schema |
| Simulation | Full AMT tree + blueprint + related containers | Root intent + branch list only |
| Deepening | Methodology guidance + related branches | Branch content only |

### The implementation

1. **Per-model context profile** — each `AvailableModel` already has
   `context_length`. The budget derivation uses this. Extend it: also
   derive a PROMPT RICHNESS level (full / medium / minimal) from the same
   context_length.
2. **Adaptive prompt assembly** — when the fallback walk switches to a
   model with smaller context, the NEXT attempt uses a simplified prompt
   (fewer context blocks, simpler JSON schema). Not just max_tokens —
   the prompt itself adapts.
3. **Multi-pass merge** — for calls that need the full picture but can't
   get it in one call: split the context into chunks, call per chunk,
   merge the partial results. The AMT's own layer-by-layer builder already
   proves this pattern works.

### The context registry connection

CONTEXT_REGISTRY's S1-S10 sources each have a SIZE (how many tokens of
prompt they consume). The adaptive assembler picks which sources to
include based on the available budget:

| Richness | Sources included | For models with |
|---|---|---|
| **Full** | S1-S7 + related containers + methodology rules + jurisdiction detail | ≥32k context |
| **Medium** | S2 (AMT outline) + S3 (blueprint steps) + S5 (jurisdiction) | 8k-32k |
| **Minimal** | S5 (matched rule + request excerpt only) | <8k |

The `relevance` K-alg already has the graph-first/keywords-only toggle —
this extends it to a three-tier richness system driven by the effective
model's context_length.

### What to build

1. **Context budget profiler** — given a model's `context_length`, return
   the richness level + max prompt tokens + max output tokens
2. **Adaptive prompt assembler** — takes richness level + context sources,
   returns the assembled prompt (rich = all blocks, medium = key blocks
   only, minimal = task + rule only)
3. **Per-attempt prompt simplification** — when the walk switches models,
   the next attempt uses the new model's richness level (the context
   doesn't change — which SOURCES are included does)
4. **Stress test per richness level** — T1-T6 mapped to richness levels,
   providing the measured data for which models work at which level

---

## §10. ADAPTIVE CONTEXT CHUNKING — the multi-pass doctrine (2026-09-22, operator correction)

**OPERATOR CORRECTION — this is NOT prompt simplification.** The previous
§9 framing ("adaptive prompt simplification for local models") was wrong.
The correct architecture: **chunk the traversed context into model-sized
pieces, make multiple model calls (one per chunk), and merge the partial
results.** The full context is always delivered — just in portions the
model can handle. No shortcuts, no dumbing down.

### The doctrine

The chunk-based AMT builder (`build_amt_layer_by_layer`) and the
graph-native builder (`build_amt_from_graphs`) are not two tiers — they are
two views of the same process. The chunk path breaks context into
model-sized pieces and iterates; the graph path delivers richer context in
fewer calls. Both should be MORE INTERTWINED, not separate. The zero-shot
calls are NOT replaced — they are logged, registered, and cataloged.

### Context assignment per step

Context is not rigidly assigned to a block. It is **identified per step**
through the close-burst relationship neighborhood: the traversed neighbors,
cross-relationships, and hot paths around each AMT branch / blueprint step.
Over time, hot paths are logged and identified. The live graph and ripple
keep everything connected.

### The multi-pass pattern

```
Model with 200k context:
  1 call → full traversed context → complete response

Model with 4k context:
  N calls → chunk i of the traversed context → partial judgment per call
  → merge partials → final decision
  (N scales with total_context / model_context — smaller model, more calls,
  same full context eventually processed)
```

### What changes in the code

1. **Context chunker** — given the assembled context blocks and the
   effective model's context_length, split into self-contained chunks.
   Each chunk carries: the task statement (always included) + one context
   block. The model judges the SAME concern against EACH context block.
2. **Multi-pass merger** — collects partial judgments from all chunks and
   merges them. For a safety gate: any Decline = Decline (conservative).
   All Proceed = Proceed. Mixed = the reasoning from the Decline wins
   (safety-first merge).
3. **Registration** — every chunk call is registered in the zero-shot
   call registry with its chunk index, total chunks, and the context
   source markers. No undocumented calls.

### Why NOT simplify

Simplifying the prompt for small models is a shortcut that limits the
system. The full context is the source of truth — reducing it means the
model makes a less-informed decision. Instead: deliver the full context in
portions the model CAN handle. More calls, same information, same judgment
quality. The cost is latency, not accuracy.

### The zero-shot registry connection

Every chunk call is a real zero-shot call — logged, registered, and
captured with full metrics. The chunk index and total are recorded so the
merge knows how many partials to expect. The call_site label includes the
chunk info: `"decision_review[chunk 1/3]"`.
