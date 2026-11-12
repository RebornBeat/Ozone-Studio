# No-skip sweep — D.6 complete (best-effort bookkeeping writes)

Operator 2026-11-11: "all should be completed" — the ~40 "best-effort" writes classified in REPORT.md §D.6 follow the no-skip doctrine.
Parse-only (rustfmt); nothing compiled, nothing run, EU untouched.

## The helper — `src/durable_write.rs` (new, `pub mod durable_write` in lib.rs)
- `append` / `append_json` / `replace` (whole-file snapshot, temp + rename) / `read_to_string` / `retry_until_ok` (async, pauses the caller until a store/IO call succeeds) / `pending` / `stats` / `flush_for`.
- ONE synchronous attempt on the caller (no added hot-path latency, same single `write` call O_APPEND semantics); a failure queues the job per path and a single worker thread retries it IN ORDER until it succeeds (2 s, doubling, held 60 s: spacing, never a give-up). No cap, no drop, no rotation; only host memory bounds the queue.
- Visible: `tracing::warn!` on first failure + recovery, graph events `durable_write_stalled` / `durable_write_recovered`, counters + per-path last error at `GET /durable_writes` (grpc/mod.rs).
- Partial writes: a retried append after a failed attempt starts with `\n` so a torn tail can't glue to the next row (no rollback-truncate: shared files are appended to by other processes).
- `replace` snapshots carry a sequence number: an older queued snapshot is discarded only if a NEWER one of the same path was already applied; `read_to_string` returns the newest not-yet-applied snapshot so read-modify-write callers don't work from stale disk bytes.
- Shutdown: `lib.rs` calls `flush_for(30 s)` after the ZSEI drain and logs what the disk still refused.
- Limits (stated, not hidden): the queue is in memory — a process that dies while a write is still failing loses it (a dead disk can't hold its own journal).

## Per-site table (old → new)
| Site | Old | New |
|---|---|---|
| orchestration_events.rs `append_walk_metric` (walk_metrics.jsonl) | `if let Ok(f) … let _ = write_all` | `durable_write::append_json` |
| context_budget.rs `record_call` (context_records.jsonl) | first failure DISABLED the sink for the whole process (`SINK_DISABLED`) | durable append; sink never disabled |
| model_ledger.rs `append_line` (ledger, source of truth) | warn + row lost | durable append (one write per row kept) |
| model_ledger.rs stats snapshot | `write().is_ok()` then rename, errors ignored | `durable_write::replace` |
| model_ledger.rs reclassify rewrite | silent | loud error, returns 0, idempotent → retried next start (rewrite-from-snapshot must not be queued behind appends) |
| network/mod.rs inbox / rejected / peer_sync_candidates | warn / `let _` | durable append |
| network/mod.rs paired_peers.json | warn, approval lost on restart | durable replace |
| network/mod.rs peer_id, marketplace listing + body index, review offset, listing versions + exchange offset | `let _ = fs::write` / warn | durable replace; the read-modify-write reads go through `durable_write::read_to_string` |
| meta_loop.rs refinement candidates / uncovered / `append_jsonl` | warn | durable append |
| meta_loop.rs refinement report; methodology_gaps/reviews writes | warn / `?` | durable replace; reads via durable read |
| amt.rs `record_methodology_gap_ex`, `record_methodology_review` | write `.is_ok()` | durable replace (+ ripple always emitted) |
| amt.rs `merge_back_to_main` | read/parse failure skipped; write ignored | read retried; write durable; unparsable main AMT = ERROR for the operator (no retry fixes corrupt data) |
| amt.rs enrich methodology search | `unwrap_or_default()` (store error ⇒ "zero matched" ⇒ BOGUS gap) | `retry_until_ok` |
| amt_candidates.rs (candidate log, archive) | `let _ = fs::write` ×2, archive `let _` | durable replace/append; reads durable |
| run_checkpoint.rs `write_checkpoint` / `load_checkpoint` | error logged, run not resumable | durable replace; load sees pending snapshot |
| run_spans.rs persist, `write_body`, Run root + span `create_container` | ignored / `None` | durable replace; `retry_until_ok` for the creates |
| graphs.rs `link_walk_content` (partner search, container load, edge write) | "skipped loudly" | `retry_until_ok` each (idempotent edges) |
| instrumented.rs / entry_gate.rs `call_spans.jsonl` | ignored | durable append |
| orchestrator/mod.rs `zero_shot_calls.jsonl` ×2 | `let Ok(f) … else return`, `let _ = writeln!` | durable append |
| decision_review.rs chunk cache + `decision_review.jsonl` | ignored | durable append |
| consciousness/proposals.rs `append_s10_row` | returned false on failure | durable append |
| consciousness/store.rs (9 store files) | `let _ = fs::write` | durable replace |
| chat_store.rs `persist_chat_turn` | content file failure ⇒ turn lost; session/turn/links/graph failures logged+skipped; failed session SEARCH ⇒ duplicate session | content file durable; every store step retried in a background task (answer never delayed); returns `()` |
| keyword_program.rs `append_line` (hit/judgment/event logs, never rotated) | `let _ = write_all` | durable append |
| keyword_program.rs rollup | failed container ⇒ counts LOST, offset advanced | per-container `retry_until_ok`; offset durable |
| keyword_program.rs `register_model_containers`, train_state | `if let Ok` / `let _` | `retry_until_ok` / durable replace |
| zsei/mod.rs `pre_write_snapshot` | failed snapshot never blocked the mutation (no rollback copy) | load + snapshot `retry_until_ok`: the MUTATION PAUSES until its rollback copy exists |
| jurisdiction_search.rs | a failed search marked the region attempted for good | region un-marked when a search failed (retried at next request); file durable; container `retry_until_ok` |
| lib.rs `.bak` before overwrite | `let _ = fs::write` then overwrite anyway | `?` — nothing overwritten without its backup |
| lib.rs manifest, tool_calls.jsonl; mcp.rs persist; file_beacon snapshot + history; actors sensor_ticks; tool_binding (gaps, health, proposals ×8); peer_import maps + offset; offer_review reviews; keyword_settle marker; walk_feed rows; grpc (done map, pairing code, mcp/call log) | `let _` | durable append/replace |

## Exceptions (not converted) and why
- Operator-gated / operator-kill-switched: none touched.
- Host log (`lib.rs` PersistentLogWriter): it IS the tracing sink — routing it through the queue would recurse; stdout is the second sink. (It still rotates at 256 MiB at start; host.log is not a hit/judgment log.)
- Internal token file (grpc): 0600 mode; durable_write would write 0644. Now logs loudly if the write fails.
- `agent_requests.rs`, `awaiting.rs`, `response_record.rs`, grpc offer-decide write: already RETURN the error to a caller that handles it.
- Reads / derivations: `stages.rs` best-effort read, `executor.rs` model-name derivation, `mcp_supervisor` log tail, grpc `/node-event` republish (in-memory hub), readiness wait before an honest failure.
- `consciousness::retire_hook_file` rename: failure logged; the hook file stays, so the idempotent migration re-runs next boot.

## For operator review (changes behaviour or contradicts a ruling)
1. `pre_write_snapshot` now PAUSES container mutations while the integrity snapshot can't be taken; a persistently corrupt container would hold every update to it.
2. `link_walk_content` now pauses the walk at stage 13 on a store failure (the answer waits).
3. `file_beacon.rs` still evicts snapshots beyond 10 (a fixed cap that deletes history) — left for your ruling.
4. `zsei/storage.rs` local-write queue has `LOCAL_WRITE_MAX_PENDING = 10_000` (blocks writers, drops nothing) — a fixed number, left for your ruling.
5. `decision_review.rs` rows still carry a `confidence` field (legacy; contradicts the no-confidence doctrine) — not touched here.
6. Unprocessed-but-unlabeled sites not converted: `peer_import.rs:325/467/601` (error returned/reported), `actors.rs` direct `fs::write` at 479/592/876/903/1110 (not reviewed in this pass).

## Unverified (parse-only) — check first at `cargo check`
- `durable_write.rs`: worker loop borrow shapes (`ready` / `g` moves into `Condvar::wait`), `Duration * 2`, `Arc<Vec<u8>>` bodies; 3 new unit tests (retry-in-order, supersede, immediate write) never run.
- Closures passed to `retry_until_ok` capture `ctx.clone()` / `container.clone()` — each must be `FnMut` returning a future that borrows only locals (graphs.rs edge write, chat_store, jurisdiction_search, zsei/mod.rs `pre_write_snapshot` with `self.storage.read().await`).
- `persist_chat_turn` now returns `()` (callers do `let _ =`); `ChatScope` moved into the spawned task after its last use.
- `OzoneError: Display` assumed for `retry_until_ok` in zsei/mod.rs.
