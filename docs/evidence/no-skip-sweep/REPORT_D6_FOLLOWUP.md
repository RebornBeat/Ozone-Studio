# No-skip sweep — D.6 follow-up (operator "you decide" items)

2026-11-11. Edit-only; parse-checked with rustfmt (`--emit stdout`) and `ts.transpileModule`; nothing compiled, tested or run; EU untouched.
Builds on `REPORT_D6_COMPLETE.md` (commit 278b928).

## 1. `file_beacon` snapshot eviction (fixed cap 10) — REMOVED
- `src/file_beacon.rs::record_history`: the "evict oldest beyond 10" block is gone. Every snapshot is kept; history is never deleted silently.
- The early `return` on a failing `create_dir_all` is gone too: `durable_write::replace` creates parents on every attempt, so a failing mkdir is a retried, visible failure (not a silently lost history row).
- Guard instead of a cap: a full disk shows up as a stalled durable write (`GET /durable_writes`, `durable_write_stalled` event) and retries until space exists. Reclaiming space is an explicit operator act.
- Known consequence: unbounded disk growth for frequently changing files. No fixed number replaces it, per the no-fixed-limits ruling; a disk-space floor (like `OZONE_MIN_AVAIL_MB` for memory) is the natural option if wanted — not added (no disk-space reader exists in the crate).

## 2. `zsei/storage.rs` `LOCAL_WRITE_MAX_PENDING = 10_000` — REPLACED by a memory guard
- Constant removed. `enqueue_local_op` now waits (condvar, 500 ms timeout so a MemAvailable recovery releases it) only while `crate::mem_below_floor()` AND writes are still queued; it never drops, never writes inline (ordering), and never waits on an empty queue (nothing would release it).
- New helper `crate::mem_below_floor()` in `src/lib.rs` (next to `wait_for_memory_floor`; unreadable `/proc/meminfo` = not below).
- Dated reviews (docs/REVIEW_ZSEI_2026-10-06.md, REVIEW_OPTIMIZATION_2026-11-06.md) still name the constant: they are the record, left unedited.

## 3. `confidence` on decision-review rows — REMOVED (tolerating legacy rows)
- `decision_review.rs`: `ChunkJudgment` is `(decision, reasoning)`; `merge_judgments` returns `(decision, reasoning)`; `DecisionReview` (orchestrator/mod.rs) has no `confidence`; `capture`/`capture_meta` take no confidence and rows no longer carry the key; the gate JSON (`review()`, operator-resolved and review-failed branches) no longer emits `"confidence"`; chunk cache rows no longer write it and ignore it on read (legacy rows load, nothing recomputed). Unit test rewritten (`merge_requires_every_chunk_judged_and_carries_no_confidence`).
- `consciousness/review.rs::CaptureRow` and `consciousness/proposals.rs` rows: field/`"confidence": null` removed (serde ignores the legacy key on read).
- `stages.rs` stage 8: no longer reads `gate.confidence`, no `(NN%)` text; `GateResult.confidence` is now `#[serde(default, skip_serializing)]` (never populated, never sent to the UI, legacy persisted rows still deserialize).
- UI: `captureData.ts` (no field), `captureStatus.tsx` (merged-row "fixed confidence" note and per-chunk confidence text removed), `ConfettiViewer.tsx` (candidate confidence display removed), `DecisionReviewPanel.tsx` / `ConsciousnessGateSection.tsx` (doc + copy), `JurisdictionGateSection.tsx` (confidence badge + type field removed). `docs/CONTEXT_REGISTRY.md` S8/S10 rows updated.
- **Follow-up edits left (files held by the jurisdiction fork, uncommitted work in progress):** `src/grpc/mod.rs:2643-2644` (`DecisionReviewRow.confidence` — `/capture/decision-reviews` still serves a legacy row's value); `src/orchestrator/jurisdiction.rs` ~972, ~1006-1009 (reads `gate.confidence`; always None now) and the test at ~1668 (asserts `Some(0.9)` from a fixture); then delete `GateResult.confidence` in `orchestrator/mod.rs`. Also `graphs.rs` ~1016 still writes a constant `"confidence": 1.0` on `RelatedTo` edges (schema field) — a relationship-schema decision, not changed.

## 4. `actors.rs` direct writes
- Production writes (2): `create_global_amt` body and `resolve_or_create_finding_target` finding body — were `fs::write … ?` (error returned to the dispatch loop, event effectively skipped). Now `durable_write::retry_until_ok`: mkdir+write retried until it succeeds, so the container created next never points at an unwritten body; the actor pauses visibly.
- `sensor-tick` mirror: `let _ = context_mirror::mirror(..)` (failure discarded) → `retry_until_ok` (the tick's report pauses until mirrored).
- The other 3 `std::fs::write` hits (lines ~876/903/1110) are `#[cfg(test)]` fixtures — legitimate, left.
- Left alone: `read_content_pointer` (`.ok()?` read; a missing pointer is a data state, the matcher returns no match) and `container_keywords` (`unwrap_or_default` on a missing key).

## 5. `durable_write` queue now survives process death
- New on-disk journal `<data_dir>/durable_write_journal.jsonl`, enabled by `durable_write::init_journal(&config.general.data_dir)` at the start of `OzoneRuntime::new` (src/lib.rs). Append-only `put` lines for every job that has to wait (first attempt failed, or it queued behind one that did; whole-file snapshots base64) and `done` lines when applied or superseded.
- Boot: `journal_load_pending` (every `put` without a `done`, in order; a torn tail line is skipped) → compact file to the pending set (atomic rename; failure keeps the old file, ids are kept so `done` lines still match) → re-queue in per-path order (replayed appends start with a newline).
- Idle: when every queue is idle and every `done` is written the journal is truncated. Ordering, per-path queueing and snapshot-supersede semantics are unchanged (a superseded older snapshot gets its `done`).
- Journal failing: its lines wait in memory (`unwritten`), are retried every worker turn (the worker no longer parks while a journal line is unwritten), and a loud `tracing::error!` + `durable_write_journal_failing` graph event say the queue is memory-only until it is writable. `GET /durable_writes` now includes a `journal` block (enabled, failing, unwritten_lines, lines_on_disk, replayed_at_boot).
- Semantics are AT-LEAST-ONCE: a crash between a job being applied and its `done` line reaching disk replays that job once more (an appended row may then appear twice; a snapshot is idempotent).
- The one case left (stated in the module doc): a disk that refuses both the file and the journal has nowhere to keep a retry record.
- Lock order is queues → journal; the journal lock never takes the queues lock. `graph_events::emit` is a broadcast `send` (non-blocking), so emitting under those locks cannot re-enter `durable_write`.
- New test `journal_pending_is_every_put_without_a_done_in_order` (pure function; never run).

## 6. The three flagged behaviours — deadlock / starvation review (read, not run)
**(a) `pre_write_snapshot` pausing container mutations (`zsei/mod.rs:98`)**
- No lock cycle found. `query()` calls it before taking `query_processor`/`storage` write guards; the retry closure takes `storage.read()` for one `load` and releases it. The outer `RwLock<IntegrityMonitor>` is only ever READ-locked (`start_monitoring` holds a read guard by design — the 2026-09-19 deadlock fix, lib.rs ~1138; nothing takes the outer write lock), and `create_snapshot` uses its own inner locks.
- STALL RISK 1 (real, left for operator): a container whose `load` fails deterministically (unparseable local-state JSON) makes every `UpdateContainer`/`DeleteContainer` on it wait forever — including the DeleteContainer an operator would use to remove the corrupt container. Retrying cannot fix corrupt data. Options: (A) snapshot the RAW bytes of the corrupt file (needs a raw-read API on `ContainerStorage`) so the rollback copy exists and the mutation proceeds [recommended]; (B) allow DeleteContainer through with a loud event + operator-visible record; (C) leave (visible stall, repair by hand on disk).
- STALL RISK 2: a persistently failing rollback directory (full/permission) pauses ALL container updates system-wide. It is the doctrine (no mutation without a rollback copy) and visible (`/durable_writes`, event), but it is a single point; option: a second rollback location.
- Not a deadlock: write amplification (a snapshot file per update; `integrity.max_versions` evicts old rollback copies — a CONFIG value, operator-set, not hard-coded).

**(b) stage-13 `link_walk_content` pausing the walk (`graphs.rs:871`, `stages.rs:415, 3033`)**
- No lock held across it: the walk runs on an `orchestrator` value prepared by the `/orchestrate` handler (grpc/mod.rs ~4464/4564); no `runtime` guard is held across `orchestrate()` at those sites. Pause blocks only that walk.
- STARVATION RISK (answer latency): one persistently failing edge write holds that walk's ANSWER indefinitely, although the link pass is bookkeeping for the NEXT walk. Option [recommended]: deliver the answer first and run the pass as a post-delivery child work-order item that the completion gate waits on (completed stays final, the answer is never delayed, the failure stays visible and retried). Not applied: it changes stage order and the operator's "walk answers when done" expectation.

**(c) shutdown drain (`lib.rs` ~1645)** — two real defects FIXED:
- `zsei::drain_local_writes(..)?` returned BEFORE `durable_write::flush_for` ran: a ZSEI drain timeout or failed write starved the durable queue (its rows lost). Now the ZSEI result is held, the durable drain always runs, and the ZSEI error is propagated afterwards.
- `flush_for` (std `Condvar`, up to 30 s) ran directly on an async worker thread; it now runs under `tokio::task::spawn_blocking`.
- The "lost with this process" message was wrong once the journal exists: pending rows stay in the journal and replay at the next boot; only a failing journal makes them memory-only.
- No deadlock found: the worker is a plain thread, `flush_for` releases the queue lock while waiting, and it is bounded by its timeout.

## Unverified (parse-only) — check first at `cargo check`
- `durable_write.rs`: `journal_flush_locked(&mut j)` with `j: MutexGuard` (deref coercion), the `Job.jid` field at every construction site (`submit` ×2, `push`, `init_journal`), `base64::Engine` import in tests, `Body` not `Debug` (the new test avoids formatting it).
- `actors.rs`: the `retry_until_ok` closures (clone-then-`async move` shape) and the `mirror` closure borrowing `z`/`data_dir`/`req`.
- `lib.rs`: `spawn_blocking(|| durable_write::flush_for(..))` closure type, `zsei_drain?` placement inside the `select!` branch, `crate::mem_below_floor()` visibility from `zsei::storage` (private `mem_available_mb` is reachable from a descendant module).
- `decision_review.rs`: all 4 `capture_meta` call sites updated to the 8-argument form (counted by hand); `capture()` has no caller (pre-existing dead code, not removed here).
- None of the new/changed unit tests has run.
