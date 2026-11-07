# Ripple — how it is used today, its categories, and the META ripple (2026-11-04)

Operator direction: the meta loops (I-Loop, methodology meta-loop, assistant, AMT re-expansion) run on a 30-minute
**timer**; they should be **event-driven** like the ripple. Ripple began for file-beacon + modality load/edit/graph
update, now also drives fan-out. Identify every way ripple is used, categorize it, and define a **meta ripple**: when
a meta-tagged event fires, all the meta loops fire automatically.

## 1. How ripple works today (verified in code)

| Piece | Where | What it does |
|---|---|---|
| Hub | `src/graph_events.rs` (`GraphEventHub`, broadcast ring 1024, never blocks a write) | one process-wide channel of `GraphEvent{event: created|updated|deleted|linked, container_id, parent_id, container_type, source, scope_keywords, timestamp, trace_id, span_id}` |
| Single producer choke point | `src/zsei/mod.rs:228,232` — every successful `ZSEI::query` write emits (source `"zsei"`) | content, structure, consciousness records, chat, tasks created via the store — all ripple |
| Other direct producers | `src/task/mod.rs:315` (source `task-system`), `src/orchestrator/amt_loop.rs:459` (`amt-expansion`), `src/context_budget.rs:284` (view-trim records) | task lifecycle, the AMT loop's own deepen writes, context trims |
| File beacon | `src/file_beacon.rs` — real file change → `update_container` → ordinary `updated` ripple | modality/code graphs notice edits; `CodeStaleness` actor reacts |
| Consumers (push) | `src/grpc/mod.rs:4123,4436` — every `/ws` socket gets `graph_event` frames | UI + agents |
| Consumers (actors, free/no-LLM) | `src/orchestrator/actors.rs` — 5 `RippleActor`s: `security-finding`, `coordination-finding`, `task-failed`, `code-staleness`, `mcp-failure` | append to the `amt_candidates` log (deduped) |
| Consumer (acting) | `src/orchestrator/amt_loop.rs:174,298` — `process_graph_event` + `AMT_WAKE` Notify; **also an interval** | one paid call per candidate, woken instantly by ripple |
| Consumer (wake) | `src/consciousness/assistant.rs:54-62,851-856` — `ASSISTANT_WAKE` from the task ripple site; **also an interval** | order check-up |
| Echo filter | `amt_loop.rs:203-215` NON_DEEPENABLE: ChatSession, ChatTurn, AMTExpansion, Task, CoordinationEvent, … | the loop's own writes never re-trigger it (11-01 fix: 317/491 candidates had ended `failed_structural`) |

**Timer-only today:** I-Loop (`i_loop.rs:91` `sleep(interval)`, floor 1800 s) and methodology meta-loop (`meta_loop.rs:57`
`sleep(interval)`). **Timer + wake:** AMT loop (`amt_loop.rs:295-298`), assistant (`assistant.rs:851-856`).
The I-Loop learns about ripple only second-hand, through the candidate log the AMT loop already consumed.

## 2. What actually flows (measured)

*Live `/ws` tally, 150 s of real work (workspace/project create, file link, code analysis, task create, resumed run):*
`created|Workspace`, `created|Project`, `created|ModalityGraph` ×3, `created|Derived` ×4, `created|FileReference`,
`updated|FileReference`, `created|Task`/`updated|Task` (source `task-system`), `updated|AMTExpansion` (source
`amt-expansion`), `created|Experience`.

*Candidate log (498 rows since 2026-10-20) by route:* GraphRipple 358 (of which **356 = "graph-ripple: created
CoordinationEvent"** — the multi-agent shared-context mirror, i.e. 71 % of all ripple-derived candidates is
coordination bookkeeping, now echo-filtered), InitialBuild 76, SecurityFinding 23, CoordinationReceived 17,
Continuation 11, CodeStaleness 11, TaskFailure 3. Ripple actors: 68 findings queued, 24 deduped as identical.

## 3. Ripple categories (proposed taxonomy — computable from `(event, container_type, source)`)

| Category | Matches | Who legitimately reacts | Meta-significant? |
|---|---|---|---|
| **CONTENT** | `FileReference`, `ModalityGraph`, `Derived`, `Document*`, `Chunk*`, `Link*`, file-beacon `updated` | modality graph refresh, AMT re-expansion (project-scoped), **fan-out re-digest**, `CodeStaleness` | only when it signals a *change in understanding* (new graph, stale code) |
| **STRUCTURE** | `Workspace`, `Project`, `Directory`, `Package`, `URL*` | scoping, UI, AMT initial build | low |
| **TASK / ORDER** | `Task` created/updated, source `task-system`; paused/failed/awaiting transitions | assistant, `task-failed` actor, UI Order | **yes** for failed / paused / overdue-edge transitions |
| **COORDINATION** | `CoordinationEvent`, shared-context notes/claims/presence | `coordination-finding` actor, other agents | no (echo-filtered) — only *findings* (handoffs, decisions) |
| **SECURITY / MCP** | security-mcp findings, `mcp-failure` | actors → candidates | **yes** (posture change) |
| **CONSCIOUSNESS / EXPERIENCE** | `Experience`, reflections, reviews, awaiting items, response records, decisions | I-Loop, consciousness dashboards | **yes** (a new experience/review is what reflection is *about*) |
| **METHODOLOGY** | methodology store writes + `methodology_gaps.json` appends (`amt.rs:2635`) | methodology meta-loop | **yes** — the gap append is the meta-loop's real trigger and currently emits no ripple |
| **LOOP-INTERNAL (echo)** | `AMTExpansion`, run-pass spans, view-trim records, `ChatSession`, `ChatTurn` | nobody (must NOT re-trigger) | never |

## 4. The META ripple (design)

**Definition.** A ripple event is a *meta ripple* when it is in a meta-significant category (§3: TASK transitions
failed/paused/overdue, SECURITY/MCP, CONSCIOUSNESS/EXPERIENCE, METHODOLOGY gap, CONTENT that created or invalidated a
graph) and is not LOOP-INTERNAL echo. Classification is a pure function
`classify(&GraphEvent) -> RippleClass{category, meta: bool, scope}` (no producer changes needed; producers may add an
explicit `meta:<kind>` scope keyword to force/clarify — the event already carries `scope_keywords`).

**Fan-out.** One `MetaRipple` dispatcher subscribes to the hub (as the actors do) and, on a meta event, wakes **all**
meta consumers through their existing `Notify` pattern (`amt_loop::notify_wake`, `assistant::notify_wake`, plus new
`i_loop::notify_wake`, `meta_loop::notify_wake`). Each loop then runs its normal pass; the passes already carry
**change detection** (I-Loop skips an unchanged signal set; assistant skips unchanged findings), so a burst of meta
events costs one pass, not N.

**Budget safety replaces the timer.** The 1800 s clamp existed to cap paid calls at ~1,000/day. Event-driven keeps that
guarantee with a **minimum gap between paid passes per loop** (coalescing: events inside the gap collapse into one
pending wake) instead of a fixed schedule. A long **heartbeat** (hours, not 30 minutes) can remain purely as a
liveness safety net — decision for the operator (§6).

**Time-based exceptions that are genuinely time events.** "Due soon / overdue" has no write to ripple on. The assistant
should sleep until the *next real deadline* (`min(due_at, remind_at)` over the Order) rather than poll — a scheduled
event, not an interval.

## 5. Implementation plan (nothing in this section is built yet)

1. `src/orchestrator/ripple_class.rs`: `RippleCategory`, `RippleClass`, `classify()` + unit tests over the real
   container-type/source vocabulary (and a test that every `ContainerType` display name lands in a category).
2. `MetaRipple` dispatcher (one subscriber) + `notify_wake()` for I-Loop and meta-loop; loops switch `sleep(interval)`
   → `select!{ wake, heartbeat }` with a per-loop minimum gap.
3. Emit a `created|MethodologyGap` ripple when `amt.rs:2635` appends a gap (the meta-loop's real trigger).
4. Assistant: next-deadline sleep.
5. E2E (real host): fire each meta category (fail a task, post a security finding, add a gap, add an Experience) and
   assert (a) the right loops woke within seconds, (b) echo categories wake nothing, (c) a burst of 20 events yields one
   paid pass per loop (min-gap coalescing), (d) change detection still skips an unchanged signal set.
6. Docs: `CONTEXT_REGISTRY` / `CALL_ROUTE_REGISTRY` rows; UI event/animation hooks (see
   `docs/UI_ANIMATION_DESIGN_2026-11-03.md`: `span_out` / `run_state` events).

## 6. Decisions needed from the operator

1. Which categories count as meta — accept §3's "yes" set, or add/remove (e.g. should a new `ModalityGraph` be meta)?
2. Minimum gap between paid passes per loop once event-driven (suggest: I-Loop 10 min, meta-loop 10 min, assistant
   5 min, AMT loop unchanged), and whether a long heartbeat (suggest 6 h) stays as a liveness net.
3. Should a meta ripple wake **all four** loops every time, or each loop only for the categories it is about (I-Loop:
   task/security/experience; meta-loop: methodology gap; assistant: task/order; AMT: content)? §4 as written wakes all
   and lets change-detection decide; the targeted form is cheaper.
