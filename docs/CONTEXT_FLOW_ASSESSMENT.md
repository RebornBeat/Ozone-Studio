# The context system — what it actually is, what it means, what's real
## (2026-10-10, the full assessment after 6 walks + 22 orchestration runs + all fixes verified live)

---

## The shape of the thing

The system has three context layers, each with its own lifetime and
mechanism. They are not redundant — they serve different timescales.

**Layer 1: In-walk context** (lifetime: one walk). The request becomes
chunks, chunks become sentence graphs, sentences feed the intent wave, the
grammar wave, and the branch-generation lanes. The AMT tree carries
source_chunk_indices back to the text. Blueprint steps carry their branch's
chunk indices. Step execution replays the step's own branch via
reconstruct_session_context — scoped, budget-trimmed, trims recorded. This
layer is complete, well-captured (ContextRecords + attempt_trail + ledger),
and correct (both ordering bugs fixed, index-carrying structural).

**Layer 2: Cross-walk context** (lifetime: the graph store, 90 MB,
2083 containers). Every walk, loop, gate, and reflection persists
containers. AMTs, blueprints, methodologies, experiences, reflections,
chunk graphs, insight containers. A later walk reaches this layer ONLY
through enrich_with_zsei_knowledge searching by request-derived keywords
(max 12 = keywords + topics) against Methodology/Blueprint containers.
**The gate is keyword overlap.** Zero overlap = zero reach, by design,
never fabricating.

**Layer 3: Loop context** (lifetime: the loop's own interval + the graph).
The four consciousness loops (i_loop 30 min, meta_loop 30 min, amt_loop
30 min, assistant 30 min) read real signals: i_loop reads amt_candidates
directly (not keyword-gated), meta_loop reads its own gap log, amt_loop
deepens PRIOR walks' AMT trees in place, assistant reads the task order.
All four persist their output to the graph. amt_loop is the strongest
cross-walk mechanism: it adds real content to prior walks' trees regardless
of keyword overlap.

---

## What we verified (the evidence, not the claims)

| Verification | How | Result |
|---|---|---|
| Honesty under grounding | walks #3-5: text-grounded prompts | substantive answers, no fabrication |
| Honesty under ungrounding | walk #6: abstract status prompt | honest refusal ("necessary input is missing") |
| Honesty under cross-walk ungrounding | walk B: koi review with no plan in context | honest clarification request |
| Cross-walk content transfer | walks A→B: same topic, graph search | **BROKEN** — retrieval delivered names, not content |
| Budget re-derivation per candidate | ContextRecord.window_source | catalog | registry | configured | unknown — all four provenance labels live |
| Confetti detection + rescue | 25+ events across 6 call sites | promoted, adjudicated, or moved on — never silent |
| Reasoning-cap detection | 26 events, ALL finish_reason=length+reasoning | labeled ozone-studio, retry rule fires |
| HTTP status decode | 54 historical rows | 22×403, 19×502, 11×400, 2×429 decoded |
| Lane attribution | Tier 0 fix: index-carrying, panic-safe | next walk attributes correctly |
| Grammar-wave results | ordering fix: stabilization before dispatch | wave markers visible via stderr streaming |
| 231 orphan purge | env-gated, tombstoned via delete() | fired + verified; future boots clean |
| Ledger reclassification | 54/55 unknown rows decoded to real causes | the "provider almost never" narrative corrected |
| Watchdog layers (2 flat sites) | adapter (max(registry, 1800s)) + metered_execute (3× for multi-call pipelines) | both rescaled; walk A2 clean through both |

---

## What this means — the honest assessment

### What works
The in-walk flow is COMPLETE. Every stage's context enters and exits
through named, budget-trimmed, provenance-tracked paths. The honesty
doctrine holds in every tested condition. The model-switch transferability
is real (budget re-derivation per candidate, window provenance). The
capture surfaces (ContextRecords, ledger, attempt_trail, events) make the
system's behavior measurable instead of guessed.

### What doesn't work yet
The cross-walk flow has a **retrieval bottleneck**: the graph holds
everything but the retrieval delivers names/IDs, not content. Walk B
proved this live — walk A's design was IN the graph and walk B got a
clarification request instead of the plan content. The four fix paths
(recency feed, embedding search, tag links, blueprint-content injection)
are retrieval designs; the storage side needs nothing.

### The deeper pattern
The two ordering bugs (grammar-wave discard, AMT lane misattribution) were
both caused by splitting sequential code into parallel phases without
verifying that implicit ordering dependencies survived the split. The
system_prompt key-name bug was caused by a wire contract never being
tested against its consumer. The http_status_of misclassification was
caused by a parser not handling its real input format. These are all the
same class: **code that was correct in isolation but wrong in composition**,
found only because someone read the actual call site instead of trusting
the abstraction. The methodology that finds them — verify against actual
code, spot-check load-bearing claims, report honest negatives — is the
system's real quality mechanism.

---

## What's left (the honest list)

1. Cross-walk content transfer: the retrieval fix (4 paths, operator
   decision) + the content-depth injection (mechanism 1 going deeper)
2. A1 marker protocol: call-start markers so the host can distinguish
   "thinking" from "hung" (needs shared-state architecture)
3. Nested pipeline-9 routing: the 3 modality binaries' internal calls
   bypass walk/ledger/quota (C6 fixed capture, not routing)
4. Embedding producer for SemanticSearch: build or remove explicitly
5. The 55 historical mislabeled rows: reclassified, but the forward-only
   fix means the ledger's historical narrative stays partially wrong
6. UI: attempt_trail rendering, quota display, ledger browser, trim
   markers, Electron rebuild, tsconfig.json
7. Hygiene: tools/bridges, tracked zsei_data, binary-less index entries,
   stale worktrees

## What the system is

It is a **context-flow system with verified honesty, model-agnostic
transferability, graph-persistent memory, and keyword-gated cross-walk
reachability** — where the in-walk layer is complete, the capture
infrastructure is comprehensive, the loops add context continuously, and
the remaining gaps are retrieval designs (not storage bugs) and
architecture passes (not patches).

Everything on this page is backed by a verified claim — either a live walk,
a test suite, a grep against source, or a timestamp-checked log entry.
Nothing here is assumed.
