# Walk expansion + completion gate (2026-11-11)

Edit only. Parse-checked with rustfmt (no errors) and ts.transpileModule (0 diagnostics). Nothing built, nothing tested, EU untouched.
Closes the two open items of `docs/evidence/tool-creation-lifecycle/REPORT.md` §4.

Operator rules enforced: a task completes only when ALL steps complete; re-expansion can happen while the task is open; completed is FINAL
(a rerun is a new task instance); failure pauses and resumes; gates await, never fail open; no fixed caps; no confidence scores.

## 1. Expansion during a walk

Before: `on_step_complete` (`src/orchestrator/mod.rs`) captured a discovered branch into `state.branch_captures` and its comment pointed at
`execute_all_steps`, which no longer exists — the step queue is built once, so no step, no closure, no work-order node ever followed. (Comment fixed.)

Now:
| Piece | Where |
|---|---|
| Review prompt also asks for `missing_capabilities` the new branches need; each discovered branch is queued in `state.walk_expansion` (`{branch,node_id,step}`), gaps are recorded via `record_capability_gaps` and added to `state.missing_capabilities` | `mod.rs` `on_step_complete` |
| Step loop is a loop of ROUNDS: inner loop stops before its next step when `walk_expansion` is non-empty; `expand_walk` runs; next round takes the remaining + new steps; cancel and "nothing queued" end it as before | `stages.rs` `stage_6_to_8_execute_steps` |
| `expand_walk`: (1) AMT node pushed on `state.amt.children` (`amt_expanded_in_walk`); (2) `close_capability_gaps` once for the named gaps (necessity vote, methodology, tool + publish privacy); (3) step `Address: <branch>` APPENDED (pipeline 9, depends on the discovering step, carries the methodologies drafted for its gaps) — no existing step index/position moves; (4) jurisdiction FULL gate re-run over the plan, a block truncates the new steps and pauses; (5) `judge_rule_applicability` when methodologies were drafted; (6) queue consumed, `walk_steps_synthesized` event, `materialize_work_order` + `work_order_after_replan` | `src/orchestrator/walk_expansion.rs` (new) |
| Failure inside `expand_walk` → `pause_on_step_error` (resumable); queue items carry `closed` + `methodology_ids`, so a retry does only the remaining work; the queue is in the run checkpoint (`RunCheckpoint.walk_expansion`, `#[serde(default)]`) and restored on resume | `run_checkpoint.rs`, `stages.rs` |
| Work-order tree: a step whose description is `Address: <branch>` now hangs under that branch node even without source chunks (also true for blueprint reconciliation's fallback steps) | `work_order.rs` `materialize_work_order` |

## 2. Completion gate

Before: `stage_9_result_collection` called `complete_task` unconditionally; an AMT re-expansion child registered under the walk (`amt_loop.rs`) never gated it.

Now `TaskManager` (`src/task/mod.rs`):
- `blockers_in` / `blocking_children(walk)`: descendants through `parent_task_id`. Blocking = a `wo_*` node not completed (skipped when it has no items and no children — nothing to settle); a non-wo child (AMT re-expansion) not completed; `failed`/`interrupted` children block unless a LATER sibling with the same prompt completed (the actor retries by registering a new task per attempt); `cancelled` (operator decision) does not block. **`wo_capability` subtrees do not gate** (see decisions).
- `complete_or_park(task, outputs, tokens)`: check + park under ONE write lock. Nothing blocks → `complete_task`. Otherwise status `awaiting_children`, outputs/tokens kept, log line naming the blockers, removed from running/queue. The response is still delivered; only the task status waits.
- `release_awaiting(start)`: from `start` up through its ancestors, a parked task with no blockers flips to completed (final) under the write lock (no double completion) and runs the same tail (`after_completion`: logs, consciousness observer, save, ripple). Called from `complete_task`, `complete_item`, and `release_all_awaiting` (every `sync_gap_tasks`, i.e. every walk/gap sync, also covers children that finished before a restart).
- `complete_task` split: status set, then `after_completion` (shared with the release), then `release_awaiting(parent chain)`.
- Boot reconciliation only touches queued/running/awaiting, so `awaiting_children` survives a restart as the record it is.
- `stage_9`: `work_order_walk_done` now runs BEFORE the gate (its `verify` item/event now says "ran all of its steps", not "completed"), then `complete_or_park`; a parked walk records `walk_awaiting_children`.
- UI: `WorkspaceStatusStrip` counts `awaiting*` already; its tooltip now says it can also mean open child work.

## 3. Lifecycle table (updates tool-creation-lifecycle §5)

| Action | Position |
|---|---|
| methodology draft / tool creation / privacy check for a gap | stage 6/7 closure (open walk, before the task record) AND, new, inside `expand_walk` for gaps named while the task is open |
| step for a discovered branch | appended in-walk, work-order node + `wo_step` item under the open task |
| AMT re-expansion child task | gates the walk's completion (task parks `awaiting_children`) |
| refinement action / marketplace accept | unchanged: outside any task |

## 4. Decisions for the operator

1. **`wo_capability` tasks do not gate the walk.** Their `verify` item needs this walk to run (deadlock) and an unfillable gap (`wanted`) would hold every walk that reported it open forever. They stay visible in the tree. Say if you want them to gate (it needs a different `verify` rule).
2. **Pending AMT re-expansion CANDIDATES (not yet registered as tasks) do not gate**; only registered children do. Gating on candidates would park nearly every walk until the background actor runs.
3. A parked walk has no live process: it completes only when the last child completes. A permanently failed child with no later success keeps it `awaiting_children` (visible in the work-order tree and the log) until the child is retried or cancelled — deliberate (no fail-open).

## 5. Not done / unverified (all parse-only)

- Stage-4 simulation is not re-run for new steps; the persisted AMT container is not rewritten (background actor owns it). Noted in the module doc.
- `MAX_CANDIDATES_PER_PASS` in `amt_loop.rs` is a fixed cap (pre-existing, outside this task) and the work-order read has a `depth > 64` cap — both already listed for review by the guardrail audit.
- Duplicate `step_index` values (a drafting model can emit them): the loop removes both from the queue after running one, so the second never runs and its `wo_step` stays open — with the gate this now parks the task visibly instead of completing it silently. Latent bug, not fixed here.
- Compile-class review done by reading: borrow of `step_queue` (refs into the per-round `steps` clone) does not overlap `expand_walk(state)`; `complete_or_park` computes blockers and mutates under one guard with the blocker Vec owned; `release_awaiting` copies `parent` before `get_mut`; `AMTNode::new` is private to `orchestrator`, reachable from the child module `walk_expansion`. Expect normal first-`cargo check` fallout (e.g. unused `AMTNodeType` import if the enum path differs).
- Suggested tests (not written, none run): `blockers_in` (branch with no items/children not blocking; failed-then-superseded child; cancelled child), `complete_or_park` → `complete_item` release, `expand_walk` idempotence (same queue twice adds one step), checkpoint round-trip of `walk_expansion` with an old checkpoint lacking the field.
