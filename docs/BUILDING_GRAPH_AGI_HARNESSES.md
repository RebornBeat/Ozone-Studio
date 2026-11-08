# Building Harnesses for Graph-Based AGI — the educational guide (2026-11-01)

> Written for students. Everything here was learned by building one — Ozone-Studio's
> orchestration harness — and measuring it: 15 real walks traced stage by stage, a
> completion funnel honestly quantified at 7%, and the edit waves that targeted the
> root causes. The system is the worked example; the numbers are real (sources:
> `walk_metrics.jsonl`, `tools/walk-scorecard/scorecard.mjs`, `docs/WALK_COMPLETION_GUIDE.md`,
> `docs/MASTER_TEST_GUIDE.md`).
>
> **Snapshot note (2026-11-08).** The funnel figures below (15 walks, Nov 3–7: stage 2 median 19 min, step-execution reach 7%) are a
> dated SNAPSHOT, kept because they are what drove the fixes. The live source is the scorecard (`tools/walk-scorecard/scorecard.mjs`):
> on 17 walks it now reads stage 2 median 12.8 min and step-execution reach 12%, and the walks run after the fixes cleared stage 2 in
> 0.4 s and 107 s — the old stage-2 kills were a wall-clock watchdog, since made progress-aware. The costs that now dominate are stage 4
> (a sequential store-linking loop), stage 5 (AMT branch count) and plan size. That the numbers moved is the lesson: a finding drove a
> fix and the funnel moved. Re-run the scorecard before quoting any figure.

---

## 1. What a harness is (and is not)

A harness is the **skeleton around the intelligence**: the fixed machinery that takes a
request in, walks it through defined stages, captures everything, and delivers a result —
regardless of which model is plugged in.

The inversion that makes a graph-based harness different: **models are clients of the
harness, not its core.** Traditional AI applications put the model call at the center and
wrap scaffolding around it. A graph-based AGI harness inverts the stack:

- **Data is structured first.** Every input becomes typed nodes and edges in a graph store.
- **Intelligence emerges from traversal.** The reasoning reads the graph — relationships,
  provenance, prior work — not a context window stuffed with text.
- **Generation is the final step, not the foundation.** The model is summoned to do the one
  thing the graph cannot do: judge, name, create novel structure. Then its output goes back
  INTO the graph.

Three properties follow, and everything in this guide serves them:

1. **Nothing judges blind.** Every decision-point prompt carries the traversed context of
   what it judges — relationships, provenance, prior work — not a summary string.
2. **Nothing bypasses review.** Every write, call, and decision funnels through one choke
   point where capture, gating, and ripple happen.
3. **Nothing is uncaptured.** Failures are data. An absent value ships with its reason. A
   walk that stalls at stage 2 of 14 is MEASURED, not hidden.

## 2. The store: one graph, one choke point, everything ripples

The single most important architectural decision: **one graph store, one write path.**

Every create, update, delete, and link — from every pipeline, tool, and actor — funnels
through ONE function. That choke point is where the system:

1. **Persists** the typed container (nodes carry their kind, attributes, provenance,
   geometry, keywords).
2. **Emits a ripple event** (typed, scope-keyworded, trace-stamped) that every observer
   — actors, loops, the UI — receives.
3. **Validates** (jurisdiction gates, per-edge rejection, schema checks).

Why one path: a bypass means an unrippled write (the UI and the actors never see it), an
uncaptured write (no provenance), and an ungated write (no policy). The tool that writes
around the choke point trains the system to ignore the law.

**Student exercise 1**: take any pipeline in your own system and trace where its output is
persisted. If there is more than one write path, you have found your first bug — the
second path will silently diverge.

## 3. The walk: the fixed skeleton around model calls

The walk is the ordered stage sequence every request passes. Ozone's has 14 stages:
jurisdiction gate (0), input capture (1), text normalization (2), methodology gather (3),
initial graph creation (4), AMT build (5), blueprint assignment (6), zero-shot simulation
(7), consciousness gate (8), jurisdiction full gate (14), task creation (9), step
execution (10), response synthesis (11/13), consciousness post-hook (12).

Design rules that matter:

- **Stages are numbered, typed, and traced.** Each stage emits one event with its trace id
  into `walk_metrics.jsonl` — one line per stage event. From that alone the harness
  computes: reach %, median time, p90, per-stage health, the funnel.
- **Some stages are gates, not steps.** They can refuse. A harness where nothing can
  refuse is not a harness — it is a pipeline with opinions.
- **Two stages are consciousness stages** — reported, not required for completion. Mark
  which stages are core and which are advisory, and keep the completion definition
  honest.
- **The walk is detached from the client.** The HTTP call returns immediately; the walk
  runs on its own task; the client subscribes to progress. A walk that dies with the
  browser tab that started it is a demo, not a harness.

**The funnel is the harness's EKG.** Ozone's real numbers (15 walks, Nov 3–7):

| stage | name | reach % | median |
|---|---|---|---|
| 1 | Input Capture | 100% | — |
| 0 | Jurisdiction Gate | 100% | — |
| 2 | Text Normalization | 53% | **19.0 min** |
| 3 | Gather Methodologies | 53% | 0.1 s |
| 4 | Initial Graph Creation | 53% | — |
| 5 | Build AMT | 47% | 7.0 min |
| 6 | Blueprint Assignment | 33% | 58.7 s |
| 7 | Zero-Shot Simulation | 33% | 47.1 s |
| 10 | Step Execution | **7%** | 14.7 min |

Read it like a clinician: the reach % *drops* where walks die or stall. Everything
downstream of the biggest drop inherits it. In Ozone's funnel, stage 2 (text
normalization, median 19 minutes — one dual-vote loop costing ≥34 sequential model calls
per sentence) is the flood gate: walks that stall there never reach stage 5, so 5's 47%
and 10's 7% are downstream shadows, not seven independent problems.

**Student exercise 2**: instrument your own walk with one metric line per stage event.
Run five real requests. Compute the funnel. The first stage with reach < 80% is where
your harness actually lives — everything before it is working.

## 4. Measuring: the scorecard discipline

You cannot improve what you have not quantified. The scorecard turns raw stage events
into four numbers per stage (reach %, median, p90, max) plus derived findings:

- **Walks**: total / fully complete / stopped / running — completion % is completed core
  stages ÷ core stages.
- **The funnel table** (above) — the EKG.
- **Findings**, each with a target: methodology floor used in 1 of 1 walks (target 0% —
  the floor firing means keyword search found nothing); keyword quality 1 of 1 walks with
  ≥3 usable keywords (target 100%); simulation feasibility LOW in 1 of 5 (target 0);
  blueprint missing-capabilities in 4 of 5, each judged + closed on the spot (target 0
  unresolved).

The discipline rules:

1. **Every metric line is trace-attributed.** One trace id per walk, stamped on every
   stage event — so the funnel is per-WALK attributable, not global averages that hide
   the distribution.
2. **The scorecard runs from real data** (the metrics file + the host log for older
   walks), never from intentions.
3. **Each finding names its target** ("floor used: target 0%") — a finding without a
   target is a complaint, not a finding.

## 5. The findings that moved the funnel — and the ones that looked like bugs but were the design

Real examples from Ozone's scorecard, each teaching a different lesson:

**Finding: "the cascade matched only 2 of 66 rules."** Looked like the weakest matcher
failure. Root cause: the cascade only CONSUMES six visibility-relevant properties
(display, visibility, position, z-index, opacity, pointer-events) — by design; the rest
are paint. Two fixes were needed together: a selector-match probe (`matches_any`) so a
paint-only rule still counts as MATCHED when its selector hits an element, and a lossless
payload so unmatched rules are summarized, not dropped (D23: data loss dressed as
summarization is still data loss). **Lesson: a filter doing its job will look like a
matcher bug until you separate "the selector matched" from "the cascade consumed it."**

**Finding: "rule_hits stayed empty."** Looked like an aggregation bug. Root cause: the
reader parsed the wrong level of the response (`graph.rule_hits` vs
`graph.bounds.rule_hits` — the real data held 429 hits across 37 rules). **Lesson:
before fixing an aggregation, print the container at the level you are reading from.**

**Finding: "walks stall at stage 2, median 19 minutes."** Root cause: the keyword
extraction produced unusable keywords → the methodology floor fired EVERY methodology →
61 branches, 12-step plans, 9 invented gaps → the walk drowned. The fix was two-part:
keyword hygiene (stopword-only items dropped, short expansions to content words) + a
relevance-first floor (floor only when keyword search genuinely finds nothing).
**Lesson: garbage in the discovery stage does not stay in the discovery stage — it
multiplies through every downstream stage, and the fix is at the DISCOVERY, not at the
symptom.**

**Finding: "review pending — needs human review," three models voting, one returning no
judgment.** The vote loop costs N sequential model calls per yes/no decision. The fixes:
ONE vote primitive with a configurable strength ladder (paranoid-7 / strict-5 / lenient-3 /
light-2 / flash-1 — one function, `configured_strength()`, one env export, every call site
inherits), temperature 0.0 (a yes/no vote on the same event must not flip), and the
strict one-word JSON instruction delivered as the system prompt (small models otherwise
write prose that parses as non-affirming). **Lesson: an assurance primitive is one
function; twelve call sites hand-rolling the loop means twelve drift points.**

**Finding: "review pending" with the walk parked forever.** Root cause: the resolver only
handled reviews in one of the two places they can sit (rows vs awaiting). **Lesson: every
state your harness can hold needs a named exit, and the exit needs a test.**

## 6. The honesty rule — the one law that cannot be violated

**A pipeline computes a value from its input, or reports the value ABSENT with the reason.
No constants. No placeholder objects. No fabricated structures.**

The lived example: Ozone's image pipeline returned a fixed 800×600 / "example_object"
analysis — discarding the pixels. It looked like a working pipeline and poisoned
everything downstream (the graph stored a fabricated result, and an "ingest milestone"
was celebrated that had analyzed nothing). The fix taught three things:

1. Parse what the bytes DECIDE (headers: format, dimensions) — honest by construction.
2. Every semantic layer not computed ships an explicit ABSENT reason ("objects: absent —
   detection is visual-mcp's surface").
3. The absence reason is not an apology; it is the contract that lets the caller route
   around honestly.

Corollary: **an unverifiable claim of success is worse than an honest failure.** The
ingest milestone that was constants was celebrated before it was decoded. Test with real
bytes, generated fixtures, self-consistent round-trips — never with hand-written
"expected" byte tables (the encoding tests caught that the hand-written Shift_JIS byte
pair was itself wrong; the encoder knows, the guesswork does not).

## 7. The gates: the compile is not the finish line

Ozone runs its gates in order; each unlocks the next:

| gate | what it proves |
|---|---|
| 0 — compile | the whole tree (every agent's work) builds on the target machine |
| 1 — honesty | the semantic layers either compute or report absent with reasons; no constants |
| 2 — relations | the spatial/graph relations are computed from structure, not LLM-inferred |
| 3 — the real page | the pipeline ingests REAL production content at scale, provenance on every item |
| 4 — cross-graph links | the deterministic producers create the cross-graph edges (Renders, Locates) |
| 5 — values | every typed-shell modality carries real values |

Plus the standing test suites per crate, run per machine, and the protocol: **capture
first, build second** — every edit wave is committed and documented BEFORE the build that
verifies it, because a build that fails mid-edit-wave must not destroy the wave.

**The sync discipline** (learned from a real one-sided drift): edits happen LOCALLY
first; in-place fixes on the remote build tree must be mirrored back and checksum-verified
BEFORE any deleting sync; every sync ends with a two-way checksum audit. The audit that
found the drift compared every source file on both trees (307 files, 0 differences after
the fix) — the guarantee is checksums, not memory.

## 8. The modality rules: how everything joins the graph

- **Run-vs-tools**: the modality RUNS ITSELF on the content first (the owning analysis →
  the base graph); tools run ON TOP as guests extending that graph. A tool that replaces
  the modality's run is a bypass.
- **The taxonomy test**: before absorbing or writing any module, ask "is this Ozone's job
  already?" (Ozone's chunking was retired from the harvest — it duplicated a pipeline;
  grinding its compile errors was noise.) The filter runs BEFORE the compile work.
- **Rules are data.** One rule pack per flavor: each rule carries when / emit /
  confidence (certain, derived, heuristic) / source / rationale. If the facts are
  missing, the rule emits nothing rather than guessing.
- **Control selections are captured and shared**: chat surfaces wired to one store state —
  both surfaces reflect one selection; captured via the config round-trip.

## 9. Building your own: the ordered recipe

1. **One store, one write path, ripples.** Typed containers; every write emits; every
   observer subscribes. (No side writes, ever.)
2. **The walk**: numbered stages, one metric line per stage event, trace-attributed,
   detached from the client. Mark the core stages; define completion honestly.
3. **The scorecard**: funnel + per-stage health + findings with targets. Run it on real
   requests, not fixtures.
4. **The gates**: stages that can refuse, ordered — policy first (cheapest to evaluate,
   cheapest to be wrong about later is nothing), then structure, then semantics, then
   simulation, then consciousness/review, then execution.
5. **The assurance primitive**: ONE yes/no vote function, configured strength, a ladder
   of presets, temperature 0.0, the strict instruction as the system prompt. Every
   decision-point call site inherits it.
6. **The honesty pass**: parse what the bytes decide; every absent value ships its
   reason; test with generated real fixtures and self-consistent round-trips.
7. **The fix order**: discovery first (garbage multiplies downstream), then the vote
   cost, then the exits. The scorecard re-runs after each — the funnel is the referee.
8. **Capture first, build second.** The edit wave is committed before the build that
   verifies it. A failed build must not destroy the wave.
9. **The taxonomy test before absorbing**: "is this the platform's job already?" —
   filters out duplicate work before the compile grind starts.
10. **The modality rules** (§8 of the affinity guide): the owning run first, tools as
    guests, rules as data, control selections captured and shared.

## 10. The reading list (this system's sources, in teaching order)

- `docs/WALK_COMPLETION_GUIDE.md` — the completion metric and the funnel.
- `tools/walk-scorecard/scorecard.mjs` — the scorecard implementation (real, ~140 lines).
- `docs/SPATIAL_GEOMETRY_LAYER.md` — frames, primitives, relations, cross-graph links.
- `docs/MODALITY_TOOL_AFFINITY_GUIDE.md` — run-vs-tools, the affinity table, the
  auto-run rules.
- `docs/TOOLS_PIPELINES_MCP_GUIDE.md` — the three-tier taxonomy and the ordered shared
  context.
- `docs/CREATING_NEW_MCPS_GUIDE.md` — the three patterns (A: local MCP, B: absorb,
  C: clean RE) and the checklist every new MCP must complete.
- `docs/EDIT_CAPTURE_2026-11-07.md` — a real edit-capture document from a real
  multi-agent week (the good, the incomplete, the open defects — all recorded).
- `docs/STATE_CAPTURE.md` — the living state capture: every arc, newest first, nothing
  hidden.
- `CHECKLIST.md` — the operator's living record; additive, never rewritten.

## 11. The final teaching point

The harness is not the thing that makes the system intelligent. The graph is not the
thing that makes it intelligent. The models are not the thing that makes it intelligent.

**The discipline is what makes it intelligent**: one write path, one measurement file,
one vote primitive, one honesty rule, one capture protocol — enforced structurally
(checksums, journals, typed stores) rather than by memory or good intentions. The
intelligence emerges from traversal over disciplined structure. Every shortcut around the
discipline is a place where the system will silently lie to you later — and it will lie
in production, on the walk you did not instrument, at the stage you did not gate.

Build the discipline first. The intelligence has nowhere else to live.
