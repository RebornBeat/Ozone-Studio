# The Personal Assistant — a consciousness pipeline (guide)

Captured 2026-09-30 from the operator's directive and a full read of the live
consciousness stack. Companion docs: `UNIVERSAL_ORDER_GUIDE.md` (the data it
watches), `ACTING_LOOP_GUIDE.md` (the ripple/actor substrate it rides),
`file-beacon-design.md` (one of its senses).

## 1. Identity — what this is

The Personal Assistant is **not a flat tool and not an MCP** (directive #41).
It is an **internal pipeline**, and it belongs to the **consciousness stack**
— the operator's own framing: *this is consciousness, or what is to be the
consciousness itself — monitoring all and responding, in a global or
per-project format, upon review, as things come through the monitor.*

So its identity is: **the consciousness's operator-facing voice.** Every
other part of the stack watches, remembers, reflects, and acts inward. The
assistant is the part that speaks outward — reminders, check-ups, progress,
proposals — scoped global / workspace / project, grounded only in real
captured state.

Two consequences follow, both binding:

1. **Same power switch as consciousness.** It gates on
   `config.toml → [consciousness] enabled`, exactly like the I-Loop already
   does (`src/orchestrator/i_loop.rs` reads `ConsciousnessConfig.enabled`).
   There are not two "is the system self-aware" toggles.
2. **No fabricated anything.** Every assistant statement carries its real
   source (task id, container id, capture row ts). No invented confidence, no
   invented presence, no invented urgency — the architecture-directives
   doctrine applies to prose the same as to numbers.

## 2. The stack it stands on — all verified real in code, 2026-09-30

| Role | What | Where | Status |
|---|---|---|---|
| Senses | Universal ripple choke point — every Create/Update/Delete/Link fires one `GraphEvent` | `src/zsei/mod.rs:191-196` → `graph_events` hub | **live** (file beacon ripples verified firing today) |
| Senses | File changes on registered FileReference containers | `src/file_beacon.rs`, 45s poll, JSONL history + snapshots | **live** (25 real entries, container 40380) |
| Senses | Task lifecycle events (create/complete/fail/coordination-status) | 3 real emit sites in `src/task/mod.rs` (`emit_task_ripple`) | **live** |
| Senses | Model-call capture quartet S10–S13 (decision reviews, host zero-shots, pipeline zero-shots, tool calls) | `{data_dir}/model_calls/*.jsonl` + read routes `/capture/*` | **writes all live; reads partial** (see §5) |
| Order | Universal task order — derived view, buckets, kind/due fields | `/order/global` (`src/grpc/mod.rs get_global_order`), Stage-1 task fields | **live** (64 done, filters verified) |
| Memory | Experience memory + emotional context + decision gate | `src/consciousness/store.rs` (§35-40) | compiled, **gated off** (`consciousness.enabled=false`) |
| Reflection | I-Loop self-reflection (pipeline 9 prompt, reflection JSON persisted) | `src/orchestrator/i_loop.rs` — **built**, 1800s budget-clamped floor | built, **gated off** |
| Review | Consciousness review pass — aged `review-failed` S10 rows + S11 zero-shot failures → creates real follow-up tasks | `src/consciousness/review.rs run_review_pass`, called from `src/grpc/mod.rs:783` (on-demand route) | **live on demand** |
| Planning | AMT re-expansion — event-woken (`Notify`), reviews candidates with a real LLM call each | `src/orchestrator/amt_loop.rs` + `amt_candidates.rs` | **live** |
| Acting | Ripple actors — subscribes the GraphEventHub, matched events wake the same amt_loop consumption | `src/orchestrator/actors.rs spawn_ripple_actor_dispatch`, `ACTING_LOOP_GUIDE.md` §6 | **live dispatch, registry content young** |
| Methodology | Meta-loop review pass | `src/orchestrator/meta_loop.rs`, 1800s | **live** |

The boot-spawn block (`src/lib.rs` ~1020-1100) is the single place all loops
spawn — the assistant joins this block, same convention.

## 3. The actual gap — why the assistant doesn't exist yet

Read every loop's inputs and one thing is missing:

- **amt_loop** watches graph-vs-AMT alignment.
- **review pass** watches aged decision-review/zero-shot failures (S10/S11).
- **meta_loop** watches methodology health.
- **actors** watch ripple matches.

**Nobody watches the Order.** Nothing computes "3 tasks overdue, X is 80%
done and stalled 2 days, meeting in 2h, this follow-up never got created
after that meeting" and says it to the operator. The data is all real and
already derived (`/order/global`); the missing piece is the voice. That voice
is the Personal Assistant.

## 4. Design — the assistant check-up pass

One new boot-spawned loop + one derived read route. No new store (the
`/order/global` discipline applies: **derived views, never copies**).

### 4.1 The loop (`src/consciousness/assistant.rs`)

- `run_assistant_loop(...)` in the lib.rs boot block, gated on
  `consciousness.enabled` (same switch as the I-Loop).
- Interval: **1800s floor, same clamp idiom as i_loop.rs** (the budget
  doctrine: 60s-class intervals are budget-incompatible; never honored
  literally). Event wake on task ripples via the same `tokio::sync::Notify`
  idiom amt_loop uses — a task completing or slipping wakes the pass early.
- Each pass, **free local filter first — zero LLM cost** (adopting
  ACTING_LOOP_GUIDE's central finding). The trigger classes are all
  computable from the order view + ripples + beacon history:

  | Class | Computed from | Free check |
  |---|---|---|
  | Overdue / due-soon | `/order/global` time buckets | bucket != someday and due_at within horizon |
  | Stalled progress | task `progress`/`steps_done` vs last ripple ts | no movement ≥ N hours while status=live |
  | Paused-too-long | coordination status + ts | paused ≥ threshold |
  | Meeting-soon | kind=meeting, due_at | now within reminder window (`remind_at`) |
  | Slippage cascade | ≥ K overdue in one project | count per workspace/project |
  | Check-up due | tasks with `kind=todo` + `remind_at` past | due reminder fired? |
  | Stuck step | blueprint tasks with `next_step` unchanged ≥ threshold | step-index delta |

- Only the classes that fire are batched into **one** LLM call (pipeline 9,
  the same prompt pipeline every loop uses) to compose the prose digest —
  N findings, one call, never one call per finding. If nothing fires, the
  pass is free and silent.

### 4.2 The output — `/assistant/feed` (derived, scoped)

`GET /assistant/feed?scope=global|workspace:<id>|project:<id>` — computes
entries on read from the same real inputs (order view, last-ripple ts,
capture rows), each entry carrying its source refs. Recomputed per request;
nothing is stored twice; entries older than the horizon drop out naturally.
The Order tab (CC's OrderPanel) gains a feed section reading the same route —
one canonical surface per domain.

Actionable entries don't just display: the pass creates real tasks for real
follow-ups through the exact path `run_review_pass` already uses
(`task_manager` enqueue) — so a check-up the operator never answers becomes a
queued item in the Order itself. The Order stays the single store.

### 4.3 What it never does

- Never reorders the operator's plan (proposals only —
  `UNIVERSAL_ORDER_GUIDE.md` §7 non-goals, reaffirmed).
- Never fabricates urgency or completion percentages — only real fields.
- Never makes its own LLM calls per event — free filter, then one batched
  digest, then silence.
- Never duplicates a surface: the monitor (capture routes, OrderPanel) is the
  operator's window INTO the signals; the assistant is the consciousness's
  window OUT. Both read the same derived views.

## 5. The review-extension half (the monitor side of the same mouth)

CC's earlier finding stands and is now this guide's Stage 2: the review pass
reads S10 + S11 but **not S12 (pipeline zero-shots) or S13 (tool calls)** —
no MCP tool's results, including security-mcp findings, reach consciousness
review today, even though `/capture/tool-calls` (the S13 read route) is live.
The assistant is the reason to close it: a security finding is exactly the
kind of thing the consciousness should *say* to the operator. Extension is
mechanical: two more `read_*` fns in `review.rs` matching the existing
S10/S11 discipline, feeding the same aged-failure → task-creation path.

## 6. Build order

1. **Stage 0 — done (this guide §2)**: senses, loops, order view all live.
2. **Stage 1 — assistant check-up loop**: `assistant.rs` free-filter pass +
   `/assistant/feed` + OrderPanel feed section + boot spawn (gated
   `consciousness.enabled`, 1800s floor, task-ripple wake). Smallest real
   unit that makes the assistant *exist*.
3. **Stage 2 — review extension**: S12/S13 readers in `review.rs` (the
   monitor input completes; security findings become speakable).
4. **Stage 3 — scoped surfaces**: workspace/project feed filters wired into
   the day view + project view.
5. **Stage 4 — proposals**: slippage cascades become AMT expansion
   candidates (re-plan proposals surfaced, never auto-applied).
6. **Stage 5 — reflection integration**: I-Loop reflections + ConsciousnessStore
   experience/emotional context inform digest tone and check-up memory
   (requires flipping `consciousness.enabled=true` — the operator's switch).

## 7. Stress-test gate (directive #43 — before "done")

The assistant is on the list of things that must be **fully stress-tested**,
not just path-proven: overdue cascades across projects, progress that moves
backward, paused-then-resumed-then-failed sequences, meeting reminders firing
under clock skew, feed correctness under empty/missing due fields, and the
budget invariant (a fully idle system must cost **zero** LLM calls from this
loop; a maximally noisy day must stay within one batched call per pass).
Universal Order + Acting Loop + I-Loop share this gate.

## 8. Overlay note (directive #42)

The screen overlay is a **mobile-side client feature, not a dependency**.
The assistant must be complete — loops, feed, tasks — with no overlay at
all; a future mobile client renders `GET /assistant/feed` like any other
derived view.
