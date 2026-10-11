# Context flow — how context actually moves through this system
## (verified against code 2026-10-10; read-only study, no edits/builds/tests)

Operator's questions, answered in order: the full walk broken down; all
walks' current state; what's left to test; how well grounded it is now;
context flow alongside efficiency; whether past context is captured or
should be; loops and simulations; the back-trace/replay; and
model-switch transferability with chunks.

---

## 1. The in-walk flow, stage by stage (what context enters, what persists)

| Stage | Context IN | Context OUT (persisted) | Verified mechanism |
|---|---|---|---|
| 0 Jurisdiction | request text | matched/block/warn (gate result on state) | 42 files/161 rules, word-boundary match (fixed 2026-10-09), rules_loaded counted |
| 1 Input capture | raw prompt | `state.cleaned_prompt` | — |
| 2 Text normalization | cleaned prompt | chunks + sentence graphs + grammar (containers) + intents (wave) | per-chunk grammar WAVE (1× cores, dispatch_bounded at the text pipeline); intent WAVE (walk-resilient, dedup serial in chunk order); chunks_snapshot iterated owned |
| 3 Methodologies | request keywords (≤12 = keywords+topics) | methodologies found + NEW methodologies created + knowledge paths | `search_by_keywords(&search_keywords, "Methodology")` — graph keyword search |
| 4 Graph creation | chunks | chunk graphs as containers (`state.file_graphs` for files) | — |
| 5 AMT build | chunks + methodologies + intents | AMT containers, branch_captures, cross_refs, detail_captures | LANES wave (index-carrying, bounded walk) + cross-ref WAVE (resilient walk, ≤max_pairs) + intent WAVE (resilient walk, serial dedup) |
| 6 Blueprint | AMT + branch→chunk reconciliation | blueprint steps with `source_chunk_indices` | per-branch reconciliation (T-A5) |
| 7 Simulation | AMT render | predictions on state | — |
| 8 Gate | AMT render (window/8-derived, was flat 1200) + jurisdiction + methods(4) + simulation self-critique | gate decision + confirmation reviews | render budget window-derived since 2026-10-09 |
| 8b Jurisdiction full | resolved plan | re-check | — |
| 9 Task | blueprint | task 399-class records | — |
| 10 Steps | **branch-scoped session replay** (see §3) + step context | step results, tokens | `reconstruct_session_context(state, window/8, step.source_chunk_indices)` — scoped to the step's OWN branch chunks, trims recorded as `session_context_trimmed` events |
| 11-12 | step results | experience containers, emotions | — |
| 13 Delivery | results | response ladder T1→T2→T0 | — |

**Capture per call** (shipped this session): ContextRecord (model, window +
provenance, tokens, usable, trims) → context_records.jsonl (699 rows);
attempt_trail per walk (model/outcome/cause/latency/next-step); model_ledger
(outcome, cause, latency, call_site, full prompt/response text via
record_with_text); confetti_rescued/confetti_adjudicated flags;
intent_call_failed/intent_parse_failed/intent_merged/request_fallback/
session_context_trimmed/context_trimmed events; `[pipeline]` stderr lines
streamed live (72 seen in walk #6).

## 2. Cross-walk flow — the graph IS the memory, and the gate is keywords

Everything every walk, loop, or gate produces lands as containers in the
ZSEI graph: AMTs, blueprints, methodologies, experience containers,
reflections, gap drafts, chunk graphs. That is the persistent cross-walk
context store, and it works — verified: walks #3-6 each persisted, the
reclassification pass decoded 54 historical rows, the ledger and capture
surfaces grow monotonically.

**How a later walk REACHES that context — the verified answer**:
`enrich_with_zsei_knowledge` (amt.rs) searches by
`layer_input.keywords + topics` (max 12, request-derived) against
"Methodology" containers, plus related-blueprint and per-modality searches.
**Reachability is keyword-overlap, nothing else.** Walk #6 ("status of this
system") correctly found nothing from walk #5 (backup strategies) — zero
keyword overlap is zero reach. There is no recency feed, no session
continuity, no embedding path: `SemanticSearch` exists as a wire variant
with **no embedding producer anywhere in the orchestrator** (grep
verified). Loops partially bypass this — i_loop reads amt_candidates
directly (not keyword-gated), meta_loop reads its own gap log — so
loop-generated context accumulates for the loops themselves, but a WALK
only sees loop output if keywords overlap.

**Honest assessment**: this is grounding by keyword, not grounding by
relevance. It is predictable (same words → same reach), debuggable (the
search terms are logged), and never fabricates — but it misses paraphrases,
and cross-walk learning only transfers when the operator reuses vocabulary.
The graph has the data; the retrieval is the bottleneck. (Improvement
paths — recency feed, embedding-backed SemanticSearch, tag-based
crosswalk links — are decisions, not bugs.)

## 3. Loops and simulations — does context flow through them?

| Loop | Reads | Writes | Persists back to graph? |
|---|---|---|---|
| i_loop (30 min) | amt_candidates unhandled (direct read, NOT keyword-gated) | reflection containers | YES — reachable by later walks via search |
| meta_loop | own gap log | methodology drafts | YES |
| amt_loop (30 min, ≤12/pass) | amt candidates + AMT containers IN PLACE | deepens existing AMTs | YES — adds to prior walks' trees |
| assistant (30 min) | task order + findings | digest containers | YES |

amt_loop is the strongest cross-walk context mechanism: it deepens PRIOR
walks' AMT trees in place (real context addition to old walks). The
sweep's confirmed wastes here (idle LLM call, unconditional persistence,
no pass bound in meta_loop — pass bound now shipped for meta_loop) are
efficiency, not flow, gaps. Simulations (Stage 7) read the AMT render and
write predictions to state — in-walk only, persisted via experience
containers at Stage 12.

## 4. Model-switch transferability — verified this session

- **Budget re-derivation per candidate**: fixed this session —
  `_budget_fraction` rides into walk_fallback_chain_standalone, which
  re-derives max_tokens from each real candidate's context_length.
- **Window provenance per call**: ContextRecord.window_source = catalog |
  registry | configured | unknown (the live catalog supplies real windows;
  54 historical rows reclassified to their true HTTP causes by the B7
  pass, fired live).
- **Chunk carryover across model switches**: chunks are model-independent
  (text + ids); the grammar wave, intent wave, and cross-ref wave all
  dispatch chunk/pair work through the resilient walk with the
  _budget_fraction re-derivation, so any chunk extracted under model A
  stays valid under model B. Checkpointing (B-design) would extend this
  from "valid" to "resumable" — deferred by decision.
- **Single-flight**: local-model calls are gated one-at-a-time (shipped),
  so a mid-walk model switch never contends with a background loop's
  BitNet call.
- **Wave width**: hardware-scaled (1× cores), not model-scaled — correct,
  because the width bounds I/O concurrency, not context size; context size
  is handled per-call by the budget re-derivation.

## 5. Back-trace / replay — what exists

- **In-walk replay**: reconstruct_session_context — replays the session's
  own text graph (chunks or sentences) branch-scoped to the step's
  source_chunk_indices, budget window/8, dropped-unit count recorded as a
  session_context_trimmed event. Falls back to full-walk when the step has
  no branch scope. Verified: called from the step-context path (stages.rs
  ~2070) and recursively for the unscoped fallback.
- **Cross-walk replay**: does not exist. The graph holds prior context but
  nothing reconstructs a prior walk's context view. The
  `context_mirror`/`file_beacon` modules are change-ripple, not replay.
- **Related**: `context_compaction` call site exists (stages.rs ~2240) for
  lossless-compaction under budget — same budget discipline.

## 6. How well grounded is it now? (the assessment)

- **Text-grounded prompts**: fully grounded end-to-end. Chunks carry
  provenance (source_chunk_indices → chunk graphs → sentence ids stable
  across retries since the node_id fix), steps read their own branch's
  replay, the delivery ladder renders from results. Walk #4's response
  answered the question with a real table; walk #6 honestly refused an
  ungroundable one. Both directions of the honesty doctrine verified live.
- **Cross-walk grounding**: graph-mediated, keyword-gated (§2). Works when
  vocabulary overlaps; silent when it doesn't — by design, never
  fabricating reach.
- **Loop-grounded**: the loops read real signals (candidates, gaps, task
  order) and their output persists — but walks reach it only via keywords.
- **Model-switch grounding**: transferable per candidate (verified
  mechanisms above); the walk never loses context because the model
  changed — only the budget/window math changes, per candidate.
- **The misattribution era is closed**: lanes, intent wave, and cross-ref
  pairs all carry indices/pair-keys; slots are pre-sized; panics are loud.
  The two ordering bugs found by the sweep are structurally impossible in
  the shipped code.

## 7. What's left to test / evaluate (the honest list)

1. **Two-walk chained experiment**: walk A on topic X → walk B referencing
   X by vocabulary → verify enrich surfaces A's containers. The single
   most informative missing test for cross-walk grounding.
2. **Wave-marker visibility with real content**: walk #6's grammar pass had
   no work (short prompt); a long-document walk would show
   `wave of N sentences, width W` lines now that stderr streams.
3. **served_window_exceeded**: armed, never exercised — needs a
   small-window model + big-input walk.
4. **The 8 remaining hand-rolled cap sites** (efficiency guide §6): actual
   behavior at scale.
5. **SemanticSearch**: confirm no embedding producer (grep-verified) and
   decide build-vs-remove explicitly.
6. **Per-stage context-size markers** (recommended, needs an edit pass):
   one marker per stage logging context bytes/tokens in-vs-out would make
   this entire document measurable instead of read-verified. Explicitly
   NOT added this round (no edits).

## 8. Relation to existing docs

This doc grounds the design docs in verified flow: CONTEXT_OBJECT_MODEL.md
(the doctrine), CONTEXT_BUDGET_SERVICE.md (the assembler), §6a of
EFFICIENCY_GUIDE.md (the sweep), CONTEXT_FLOW_COVERAGE_2026-10-06.md (the
coverage audit), CONTEXT_HANDLING_AUDIT + CONTEXT_REVIEW (the cap
inventory). Where those docs describe intent, this doc describes what the
shipped code actually does per stage, with the verification standing
behind each claim.
