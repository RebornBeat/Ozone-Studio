# The Acting Loop — what responds when something is seen

> Twelfth doctrine doc. Operator correction, 2026-09-29, precise: **not**
> "wire security-mcp into a dedicated consciousness review" — consciousness
> is a windows-first observer (it sees through the monitor/heartbeat feed,
> the same surface everything else is tracked through). The real,
> repeated question: **"what is acting on the security MCP findings? what
> is acting on shared coordination? what is acting when coordination is
> received? nothing?"** — and the answer, confirmed by reading the real
> code rather than assumed, is: **nothing, today.**
>
> Second operator refinement, same day: what wakes consciousness/the
> acting loop isn't a bespoke poller per source — **it's the living
> graph's own ripple** (`GraphEventHub`, already real, already firing for
> MCP calls, coordination events, and file-beacon updates today). "You
> awake what you need if more than one, but it will always cause
> consciousness to awake, as this will always create or add or ripple
> through the monitor which consciousness is monitoring — thus the acting
> loop is the living network." Responders to that ripple are **actors**,
> "defined per or as needed" — registered, kind-tagged, matching this
> codebase's own K-registry contract pattern (`docs/CONTRACTS.md`) —
> applied **where applicable, not forced everywhere**. This doc is the
> honest finding plus a real, budget-aware, ripple-native design, reusing
> AMT re-expansion exactly as the operator directed ("ozone-studio should
> be acting again all through AMT-re-expansion or new AMT as applicable")
> — not inventing a parallel mechanism.

---

## 1. The confirmed gap (read directly, not assumed)

**Nothing acts on monitored events today.** Checked exhaustively:
`grep -rn "amt_candidates::append\|record_amt_reexpansion_candidate" src/`
finds candidate-creation ONLY inside `amt.rs`'s own internal build process
(`UnverifiedNode` when a freshly-built AMT has an unsourced node,
`ThinTree` when it has no branches at all) and `amt_loop.rs`'s own
graph-ripple subscriber (`GraphRipple`, scoped to `proj:`/`file:` events).
**No code path from `/mcp/call`'s handler, and no code path from
coordination/`context_mirror` handling, creates a candidate or wakes the
loop.** A real security-mcp finding, a real ZCode handoff — both are
captured (S13, the coordination mirror containers) and both are
*visible* (the monitor feed, `context_summary`) — but neither *does*
anything. They sit, tracked, until a human reads them.

**The I-Loop does not exist.** It is registered as inert metadata only
(`src/bootstrap.rs:782`, pipeline_id 44, `folder_name: "i_loop"`) — the
folder itself is not present anywhere under `assets/pipelines/
consciousness/` (confirmed: `chemistry_sync`, `consciousness_integrity`,
`consciousness_config`, `consciousness_dashboard`,
`collective_consciousness`, `consciousness_query` are the real
directories that exist there; `i_loop` is not among them). Unlike AMT
re-expansion and the methodology meta-loop — both real, running,
boot-spawned, both logging `"... loop starting interval_secs=1800"` at
startup, confirmed live in this session's own restart logs — there is no
I-Loop cycle running anywhere in the system, consciousness enabled or
not.

## 2. The real mechanism already there to reuse

`amt_candidates.rs` + `amt_loop.rs` are a better foundation than a new
mechanism would be, and match the operator's own direction exactly:

- **Recording a candidate is free** — `amt_candidates::append(container_id,
  project_id, route, detail)` is a local JSON-file append, deduplicated
  per container, capped at 500 entries. No model call.
- **The loop is already event-driven, not just interval-polled** — a real
  `tokio::sync::Notify` (`AMT_WAKE`) wakes it immediately on a subscribed
  graph-ripple event (`spawn_graph_ripple_sync`, listening to
  `GraphEventHub::global()`), with the 1800s interval as fallback only —
  "event-driven (instantly) with the interval as fallback, not instead"
  per the code's own comment.
- **The expensive part is real and already proven**: each unhandled
  candidate costs exactly one real LLM call (`PROMPT_PIPELINE_ID = 9`) to
  deepen that branch — this has been running safely all session.

**The one real risk, found by reading the consumption loop itself, not
guessed**: `run_amt_reexpansion_loop` processes `for candidate in
candidates.iter_mut()` — **every** unhandled candidate, every wake, each
one a real LLM call. Today this is safe because the only producers
(`UnverifiedNode`/`ThinTree`/`GraphRipple`) are inherently rate-limited by
how often a project's AMT is actually rebuilt or its graph actually
changes. **If MCP calls or coordination events start appending
candidates directly, that safety property breaks** — `/mcp/call` alone
could fire many times a minute across two active agents, and an
unfiltered wire-through would turn each one into a real LLM call. This is
exactly the "draining the budget" risk the operator flagged, and it is
the central design constraint below, not an afterthought.

## 3. The design: a registered actor on the real ripple bus, free local judgment, existing paid step unchanged

Not a bespoke poller per source — a small, registered set of **actors**
subscribed to the SAME `GraphEventHub` the AMT loop already subscribes
to (`spawn_graph_ripple_sync`'s exact pattern, reused not duplicated).
Every stage before the existing `amt_loop` consumption is free (local
reads only, never a model call):

```
Real ripple sources — ALL of them, confirmed by a full-codebase audit,
converge on ONE universal choke point (src/zsei/mod.rs:191-196): every
Create/Update/Delete/Link ZSEIQuery, regardless of caller, fires
graph_events::emit. This is the literal "living network" the operator
described — not several separate wires, one real nervous system:
  MCP call            → context_mirror::mirror(kind:"tool_call")       ─┐
  coordination note   → context_mirror::mirror(kind:"finding"/"handoff") │
  file beacon delta   → UpdateContainer                                 ├─→ same choke point
  task created/updated → emit_task_ripple (task/mod.rs, 3 real sites)   │
  config change (saved)→ context_mirror::mirror(kind:"config_change")  ─┘
        │
        ▼  GraphEventHub::publish — ALREADY REAL, already firing
[GraphEvent: event, container_id, parent_id, container_type,
             source, scope_keywords, timestamp]
        │
        ▼  new: one subscriber loop, same shape as spawn_graph_ripple_sync
[ActorRegistry — every registered RippleActor.interested_in(&event)]
  cheap, sync, container_type-only check — e.g. skip anything that
  isn't "CoordinationEvent" for free, no fetch at all
        │  (only real matches proceed)
        ▼  ONE real, local GetContainer fetch (not a model call)
[real container JSON — verified live shape, §3.1]
  local_state.context.keywords   → the real kind ("tool_call"/"finding"/...)
  local_state.storage.object_store_path → the content-pointer file,
    read directly for tool-call detail (tool name, success/error)
        │
        ▼  RippleActor.evaluate(&event, &container) — sync, real rule, no LLM
[Some(ActorFinding) on a real match]
        │
        ▼  amt_candidates::append — free, deduped, capped (unchanged)
        ▼
[EXISTING amt_loop — completely unchanged, already the real cost center]
  wakes on the existing AMT_WAKE notify, one real LLM call per candidate
```

### 3.0 The full real source inventory (fork-audited, not assumed)

A dedicated read-only pass across the whole codebase (not just the sites
already suspected) confirmed every one of these is real and currently
firing, not designed-only:

| Source | Real call site | kind / container_type | Status |
|---|---|---|---|
| Every ZSEI write | `src/zsei/mod.rs:191-196` | (the universal choke point itself) | ✓ confirmed — everything below rides this |
| File beacon | `src/file_beacon.rs` → `UpdateContainer` | `"FileReference"` | ✓ confirmed — no separate emit needed, rides the choke point free |
| Task lifecycle | `src/task/mod.rs:464-486` (`emit_task_ripple`), 3 real sites (create, 2× status update) | `"Task"`, `source:"task-system"` | ✓ confirmed — created/completed/failed/paused all real |
| MCP tool calls | `src/grpc/mod.rs:3159-3178` | `kind:"tool_call"` | ✓ confirmed live this session (container 40521) |
| Coordination notes | `src/context_mirror.rs` | `kind:"note"/"finding"/"claim"/"handoff"` | ✓ confirmed — 4 real kinds observed in source |
| Config changes | `src/grpc/mod.rs:1646-1663` | `kind:"config_change"` | ✓ confirmed, real saves only |
| AMT re-expansion's own writes | `src/orchestrator/amt_loop.rs:289` | (rides the choke point too) | ✓ confirmed — self-consistent, an actor could watch AMT growth itself |

**One honest, minor gap found, not papered over**: `"decision"` is a
schema-permitted `kind` value (`note_add`'s own tool schema allows it)
but no real code path in `context_mirror.rs` actually emits it yet —
real-but-unobserved, not a bug, just worth knowing before an actor is
written to key off it.

This means the design below needs **zero new emission code anywhere** —
task lifecycle, file beacon, MCP calls, coordination, and config changes
are ALL already real, already-firing ripple sources today. The only new
code is the subscriber side (§3.2-3.3).

### 3.1 The real container shape (verified live, not assumed)

Triggered a real ripple (`firewall_status` through `/mcp/call`, a tool
that genuinely returns `success:false` on this host) and read the
resulting container directly:

```json
// metadata
{
  "container_type": "CoordinationEvent",
  "materialized_path": "/SharedContext/global/tool_call/mcp-tool-call-firewall-status",
  "name": "MCP tool call: firewall_status",
  "provenance": "actor-design-probe"
}
// storage
{ "object_store_path": "shared_context/tool_call-actor-design-probe-1790720343.json" }
// context.keywords
["tool_call", "actor-design-probe", "scope:global"]
```

Two real, load-bearing facts this confirms:
1. **`GraphEvent.container_type` is always the generic `"CoordinationEvent"`**
   for every kind (tool_call/note/decision/handoff/finding/claim) — an
   actor's cheap pre-filter can only narrow to "is this coordination at
   all," never the specific kind, without one real fetch.
2. **The real kind lives in `context.keywords`** (here: `"tool_call"`),
   already present on the SAME fetch used for the pre-filter narrowing —
   no second round-trip needed to know tool_call vs. finding vs. handoff.
   A THIRD, separate read (the `object_store_path` content-pointer file)
   is only needed when an actor needs the tool-call's specific
   `{tool, success, error}` detail (e.g. the security actor below) — not
   for coordination-note actors that only need the kind itself.

### 3.2 Three concrete real actors (worked examples, not yet built)

**`SecurityFindingActor`** — `interested_in`: `container_type ==
"CoordinationEvent"`. `evaluate`: real keywords contain `"tool_call"` →
read the real content-pointer file → `detail.tool` is one of
`net_connections`/`listening_ports`/`firewall_status`/`process_top` AND
`detail.success == false` → one `SecurityFinding` candidate (deduped by
`amt_candidates::append`'s existing per-container logic, so a
repeatedly-failing `firewall_status` — this host's real, currently-true
state — only ever queues once, not per call).

**`CoordinationFindingActor`** — `interested_in`: same coarse
`"CoordinationEvent"` check. `evaluate`: real keywords contain
`"finding"` (ZCode and this session's own existing convention for "this
is worth someone's attention," not a new signal invented here) → one
`CoordinationReceived` candidate.

**`TaskFailedActor`** (third example, from the fork audit's real
task-lifecycle finding) — `interested_in`: `container_type == "Task"`.
`evaluate`: the fetched container's real status field is `"failed"` (not
`"completed"`/`"cancelled"`) → one `TaskFailure` candidate — closes a
real gap on its own: a failed background task today is only visible if
someone checks `/order/global`; this makes a real failure a real reason
for the project's AMT to get a deepening pass on what went wrong.

All three are real Rust, each does exactly one local fetch, none ever
calls a model. Registered via an `ActorRegistry`, mirroring
`shared/contracts/k_registry.rs`'s exact taxonomy-plus-registration
idiom and `src/zsei/search.rs`'s `SearchStrategy` trait-object registry
convention (sync trait methods; the calling loop does the async I/O, not
the trait itself — matches this codebase's existing pattern rather than
introducing `async_trait` as a new dependency).

### 3.3 "Where applicable, not forced" — real scope discipline

Not every real ripple source needs or gets an actor. A `FileReference`
container's plain create/delete ripple, a `ModalityGraph` write from
ordinary orchestration — these already flow through `GraphRipple`
candidates via the EXISTING project-scoped mechanism (§2) when they're
project-anchored; adding a redundant actor for them would just double-
process the same signal through two paths. Actors are for sources that
`amt_loop`'s existing `GraphRipple` route doesn't already cover: global
(non-project) tool-call findings and coordination notes specifically —
named narrowly here, not "every event gets an actor by default."

**Explicitly NOT actor criteria**: anything requiring semantic judgment
("does this finding sound important") stays OUT of `evaluate()` — that
is exactly the decision this design defers to the existing, already-paid
LLM step once a real candidate is queued. An actor is a gate, not a
reviewer.

### 3.4 Budget accounting (the real constraint, done honestly)

At ~1,000 calls/day total budget, and given `/orchestrate`, real chat
usage, and the existing AMT/meta loops already draw from the same pool:
a conservative allocation for this NEW acting path should be a small
fraction of that — e.g. **no more than ~20-30 candidates/day** from
monitored events combined, leaving the overwhelming majority of the
budget for the operator's own actual work. Stage A's rule strictness is
the lever that enforces this, not a hardcoded call-counter (a counter
would silently start dropping real findings once hit; a strict, honest
filter simply doesn't generate noise in the first place). If real usage
shows Stage A still lets through too much, the fix is tightening the
rules (e.g. raising the CPU threshold, requiring N consecutive high
readings), never a silent cap that could hide a real finding.

## 4. What "acting" produces (once a candidate is real)

Reusing `amt_loop`'s existing behavior exactly — a real LLM call deepens
the relevant project's AMT with concrete detail about the finding,
persisted as a real branch. This is genuinely different from just
logging: the finding becomes part of the project's actual living
knowledge structure, discoverable by every downstream consumer that
already reads the AMT (blueprint assignment, consciousness review when
enabled, the UI's `amtLineage.ts`). For a **global** (not project-scoped)
signal — e.g. a host-wide security finding with no obvious project —
route to a **new project-less AMT candidate type** is explicitly a real
open design question (`amt.rs`'s current model is project-anchored
throughout); not resolved here, flagged honestly rather than forced.

## 5. The I-Loop — real gap, and why 60s-as-documented can't be built as-is

Building the I-Loop to the letter of the Consciousness Guide (60,000ms
interval, a real LLM reflection every cycle) would generate 1,440 real
calls/day **on its own** — more than the entire stated budget, before
counting anything else. Two honest paths, not decided here:

1. **A much longer real interval** (mirroring `amt_loop`/`meta_loop`'s
   own real 1800s convention rather than the guide's 60000ms) — simplest,
   smallest change from the documented design, real but infrequent
   reflection.
2. **Event-driven + Stage-A-style gating**, same shape as §3 above: the
   I-Loop wakes on real signals (a project's AMT just grew, a real
   candidate was just acted on, a real user session just ended) rather
   than a fixed clock, and only spends a real call when there's
   something genuinely new to reflect on — mirroring the acting loop's
   own budget discipline rather than introducing a second, inconsistent
   one.

Either way: **the I-Loop needs actual implementation** (a real
`assets/pipelines/consciousness/i_loop/main.rs`, currently nonexistent)
before either option is buildable — this is real, scoped, not-yet-started
work, not a config flip.

## 6. Build order (queued, not started)

1. Add `SecurityFinding`/`CoordinationReceived`/`TaskFailure` routes to
   `amt_candidates.rs`'s taxonomy (additive, matches the existing
   3-route pattern exactly).
2. `src/orchestrator/actors.rs` — the `RippleActor` trait (sync methods,
   matching `SearchStrategy`'s convention, no `async_trait` dependency),
   `ActorFinding`, and an `ActorRegistry` mirroring
   `shared/contracts/k_registry.rs`'s taxonomy-plus-registration idiom.
3. `SecurityFindingActor`/`CoordinationFindingActor`/`TaskFailedActor`
   (§3.2) as the first 3 real registered actors — real thresholds named
   as constants, per this project's own K-registry/named-preset doctrine.
4. One new boot-spawned subscriber loop (`spawn_ripple_actor_dispatch`,
   same file/shape as `amt_loop.rs::spawn_graph_ripple_sync`, wired next
   to it in `src/lib.rs`) — subscribes `GraphEventHub::global()` once,
   fans out to every registered actor's `interested_in`/`evaluate`, calls
   `amt_candidates::append` + `AMT_WAKE`'s notify on a real match (reuse,
   not reimplement).
5. Global (project-less) candidate handling — the one real open design
   question from §4 — resolved before step 4 actually fires for
   non-project-scoped findings (a global `firewall_status` finding has
   no obvious project to deepen).
6. I-Loop real implementation, once the interval/trigger-model question
   (§5) has an operator answer.

## 7. Non-goals (stated, matching this doc-family's own convention)

- NOT a semantic "is this important" LLM pre-filter — that defeats the
  entire budget-safety point of having a free Stage A at all.
- NOT a per-event real-time LLM call — the existing loop's batched,
  woken-not-polled consumption is the only place real cost is spent,
  unchanged.
- NOT resolving what "acting" means beyond AMT deepening in this pass —
  e.g. auto-remediating a firewall issue is a categorically different,
  much higher-stakes kind of "acting" than knowledge-graph deepening,
  and is explicitly out of scope here.
