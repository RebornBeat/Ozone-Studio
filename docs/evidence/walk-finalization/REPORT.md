# Walk finalization — 2026-11-11 (edit only; parse-checked with rustfmt; NOT compiled, NOT tested, EU untouched)

Operator rulings implemented: AMT new version after expansion; unique step_index; no fixed caps; memory floor pauses interactive walks; no minted graph ids; stage-4 re-simulation for new steps; operator retry/cancel of a failed child.
NOTE: the first six items were swept into the operator's commit `86145d9` (a whole-tree commit); the graph-failure items (5) are uncommitted until this fork's commit.

| # | Change | Where |
|---|---|---|
| 1 | AMT container rewritten as a NEW VERSION after every expansion round: `persist_amt_version` writes `amt_p{project}_{ts}_{id}_v{n}.json` (old body file kept; `update_container` bumps version + history snapshot; `object_store_path` repointed). Runs inside `expand_walk` BEFORE the queue is consumed, so a failed write pauses with the queue intact (resume writes it). | `amt.rs` (before `persist_amt_container`), `walk_expansion.rs` step 5b |
| 2 | `duplicate_step_indices` + `stage_3_validated`: a plan with a repeated `step_index` is invalid, replanned fresh (no stored-blueprint reuse), and fails (→ pause) if still invalid. Never renumbered, never silently skipped. Wraps all 3 stage-3 call sites. | `stages.rs` (`stage_3_validated`) |
| 3 | Caps removed: `MAX_CANDIDATES_PER_PASS` (amt_loop), `MAX_GAPS_PER_PASS` (meta_loop), work-order tree `depth > 64` (→ visited-set), `graph_terms_max` 256 / `rollup_terms_max` 512, span-out 400 floor (→ budget/50, at least 1), P2P name 512 / keywords 500 / chunk 2×30,000 guards, created-tool buffer 1024 (→ unbounded single-consumer queue). Registry updated (section "fixed caps REMOVED"). | `amt_loop.rs`, `meta_loop.rs`, `task/mod.rs` (`work_order`), `keyword_settle.rs`, `span_out.rs`, `network/mod.rs`, `STATIC_LOGIC_REGISTRY.md` |
| 3a | **The 2×30,000 answer:** `FETCH_CHUNK_BYTES` = 30,000 is TRANSPORT fragmentation (gossipsub's 64 KiB max message; now `GOSSIPSUB_MAX_TRANSMIT_BYTES` set explicitly in the config). It is not a model-window chunk and not a limit — a body of any size is split into as many fragments as needed. Kept, documented as a protocol constant. The "chunk ≤ 2×" refusal is removed. | `network/mod.rs` |
| 4 | `crate::wait_for_memory_floor` (OZONE_MIN_AVAIL_MB, default 1536): PAUSE then resume — at the start of every `/orchestrate` walk, before each step, before each AMT re-expansion candidate and meta-loop gap, and in graph-on-link (now shares it). Never skips/stops/fails; unreadable meminfo = nothing to wait for. | `lib.rs`, `orchestrator/mod.rs`, `stages.rs`, `amt_loop.rs`, `meta_loop.rs`, `graph_on_link.rs` |
| 5 | No minted ids: modality-graph site (`graphs.rs`) and file site (`graph_attached_file_build`) now fail instead of `generate_id_static()`; the text pipeline returns `success:false` on a persist failure (no local id). The walk records a failure (`state.graph_failures`, checkpointed, `graph_creation_failed` intent event) and CONTINUES from the chunked text. A FILE failure opens a `wo_graph_retry` task under the walk (gate waits); the graph-on-link worker re-queues the file every 60 s while one is open and ticks the item when the graph exists (also on the early `complete` path). | `text/main.rs`, `graphs.rs` (`record_graph_failure`), `orchestrator/mod.rs` STEP 0, `run_checkpoint.rs`, `work_order.rs`, `graph_on_link.rs`, `task/mod.rs` (`open_work_keys`, `complete_work_items_by_key_suffix`) |
| 6 | After a round that added steps, the stage-4 simulation is re-run on the new plan (result replaces the earlier one; recorded `walk_expansion_resimulated`; failure pauses). | `stages.rs` step loop |
| 7 | Operator actions on a failed child: `POST /task/retry {task_id}` (new route, wires the existing `retry_task`; the retry is a NEW task under the SAME parent so the gate waits on it) and `POST /task/cancel` now accepts a `failed`/`interrupted` CHILD (operator decision; releases the parked walk). A cancelled `wo_*` node no longer blocks. | `task/mod.rs`, `grpc/mod.rs` |

## Review-like-a-compiler notes (still to confirm at `cargo check`)
* `select!` arm with `, if tool_open` and an `async` block borrowing `tool_rx` (amt_loop bridge); `subscribe_created_tools` now returns `Option<UnboundedReceiver<…>>`.
* `retry_task`: partial move of `task.inputs` after cloning `parent`/`kind` first.
* `cancel_task`: the new `failed_child` flag is read inside the write lock; `release_awaiting(parent)` runs after the lock scope.
* `persist_amt_version` passes `"context": null` (as the existing `ContainerUpdate` callers do for `metadata`/`storage`).
* `graph_on_link::spawn` clones `tasks` for the retry loop before the worker moves it.

## Left for review / gaps (not hidden)
* **Modality-graph (prompt text) failures cannot gate yet**: recorded (`graph_failures`, event) and the walk continues from the chunked text, but there is no rebuilder/ticker for them, so no `wo_graph_retry` item is opened (an item nothing can tick would park the walk forever). Option: a targeted single-modality rebuild + tick before the completion gate.
* The retry loop matches a walk's `file_path` to the link job's path by string equality; a relative-vs-absolute mismatch would leave the item open (operator cancel releases it).
* `stages.rs` step loop `max_iterations = steps.len()*2` is unchanged (unreachable: each iteration completes ≥1 step).
* `keyword_settle` Pass A has no memory-floor wait (synchronous function); the other term caps are gone.
* The per-container fail count / parked backoff in `amt_loop` stays (retry policy for broken candidates, deferred + logged, never dropped).
* `/task/retry` and the UI: no button added (UI files are another fork's).
* Unverified: everything above (parse-only). New tests: none written.
