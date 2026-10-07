# ZSEI review — ZCode's storage and query changes (2026-10-06)

Scope: read-only review of `src/zsei/storage.rs` (write-behind), `src/zsei/query.rs` (read/write split), `src/zsei/mod.rs` (`query`, `get_container`, `traverse`). No source edits, no builds, no tests. Every finding below was checked by reading the code at the cited lines.

Context: ZCode's handoffs (`muwu2d3tvyp3` for the full-chain run, `muwoptnd8fxy` for the lock fixes) say these changes were built and ran a full chain 0→13 with zero freezes. So these are live findings, not hypotheticals. Whether any write has actually been lost is not known: `target/release/host.log` ends 2026-09-29 and contains no write-behind messages, but it does not cover the current binary.

## Verdict

The write-behind removes the storage-guard hold, which was a real freeze contributor. It does so by trading away durability and error reporting, and that breaks the standing rule "never silently drop." The read/write split and lock ordering are sound. Four write-behind findings are HIGH and should be fixed before the next full chain is trusted.

## Findings

### F1 — HIGH: inline fallback can reorder writes (storage.rs:645–668)

`store_local` takes the inline path when `pending >= LOCAL_WRITE_MAX_PENDING` (10,000). The inline path writes straight to disk and does not wait for the queue.

Failure sequence:
1. Queue holds `X@v1` (not yet written).
2. Pending count reaches 10,000.
3. Caller stores `X@v2`; inline `fs::write` lands `v2` on disk.
4. Writer thread later writes `X@v1` over it.

Result: cache holds `v2`, disk holds `v1`. The divergence is silent in-process and appears only after restart, when `load_local_cache` reads disk.

Fix: never bypass the queue. Use a bounded `sync_channel` so backpressure blocks the sender in order, or flush/await the queue before the inline write.

### F2 — HIGH: write failures are reported after the caller has returned Ok (storage.rs:30–44, 650)

`store_local` returns `Ok(())` once the pair is queued. If `fs::write` later fails, the writer thread does `eprintln!` and moves on. There is no retry, no record, and no way for the caller or `sync()` to learn of it. Before this change, the same failure returned `Err` to the caller.

Fix: keep a failure ledger (container id, error, time). Retry with backoff. Surface non-empty ledgers through `sync()` and the health path.

### F3 — HIGH: nothing drains the queue on sync or exit (storage.rs:741–746; grep shows `LOCAL_WRITE_PENDING` only at lines 42 and 645)

- `sync()` flushes only the global mmap. It does not wait for queued local writes.
- The `Sender` lives in a `static OnceLock`, so the channel never closes and the thread never drains on normal process exit.
- On `SIGTERM`/`SIGKILL`/abort, up to 10,000 queued local-state writes are lost. The global index (mmap, synchronous) can survive while the JSON does not.

Fix: have `sync()` block until pending reaches zero (with a timeout that reports the remaining count). Register a shutdown hook that drains.

### F4 — HIGH: a missing local file is read as an empty state with no error (storage.rs:404)

`load()` does `self.load_local(id)?.unwrap_or_default()`. When the global record exists but the local JSON is missing, the caller gets an empty `LocalState` (empty metadata, context, and relations) and no error. Combined with F3, a lost write becomes an empty container with no signal.

This line predates the write-behind. The write-behind makes the condition reachable after a crash.

Fix: if the global record says local state exists and the file is absent, return an error. Only `unwrap_or_default` when the container was never given local state.

### F5 — MEDIUM: delete can race a queued write and recreate the file (storage.rs:697–735)

`delete()` calls `fs::remove_file` synchronously (line 734). A `store_local` for the same id may still be queued, and the writer then recreates the file.

- `load()` is safe: the global record is zeroed and checked first, so the container does not reappear through `load()`.
- The orphan JSON is loaded into `local_cache` at boot by `load_local_cache`, which reads every `*.json` regardless of the index.

Fix: route deletion through the same queue as a `Remove` op so ordering is preserved.

### F6 — MEDIUM: local-state writes are not atomic (storage.rs:657–668, the `fs::write` calls)

`fs::write` truncates, then writes. A crash mid-write leaves a truncated file. `load_local_cache` skips unparsable files silently (`if let Ok`), and `load_local` returns a parse error. This predates the write-behind; the queue widens the window.

Fix: write to `<id>.json.tmp` and `rename` into place.

### F7 — OK (with a note): `query()` read/write split (mod.rs:129–190; query.rs:205–222, 229–346)

- All 13 variants in `is_read_query` have arms in `process_read`. The catch-all at the end of the match handles only the write variants.
- `process_read` takes `&ContainerStorage`, so the borrow checker enforces read-only access. A read arm cannot mutate storage without a compile error.
- Lock order is `query_processor` then `storage` in both branches. `cache.write()` happens after the storage guard is released. No cycle found.
- Not verified here: that each read arm's body is line-equivalent to the matching `process()` arm. I checked arm presence and the borrow constraint, not body diffs. ZCode says the bodies delegate to shared helpers.

### F8 — MEDIUM: `GetContainerContent` does blocking disk IO under the storage read guard (query.rs:78–110)

It calls `std::fs::read_to_string` inside an async fn while holding `storage.read()`. This blocks a tokio worker for the file's read time, and because the lock is write-preferring, a waiting writer then blocks new readers too.

Fix: copy the path out, drop the guard, then read with `tokio::fs` or `spawn_blocking`.

### F9 — LOW/MEDIUM: `traverse` holds the storage read guard across a blocking traversal (mod.rs:335–342)

There is no `.await` under the guard, so this cannot deadlock. But `block_in_place` plus `futures::executor::block_on` blocks the worker for the whole traversal, and `block_in_place` panics on a current-thread runtime. The snapshot-then-traverse fix is still open (ZCode: StorageView per-hop guards, designed, not implemented).

### Checked, no finding

- `get_container` (mod.rs ~266–290): cache read released before storage read; storage read held through cache insert; order storage→cache matches `store_container`. Stale-cache race as fixed by CC stays fixed.
- `pre_write_snapshot` and the ripple cache invalidation run outside the storage guard.

## Net

| # | Sev | Where | Status |
|---|---|---|---|
| F1 | HIGH | storage.rs:645–668 | open |
| F2 | HIGH | storage.rs:30–44 | open |
| F3 | HIGH | storage.rs:741–746 | open |
| F4 | HIGH | storage.rs:404 | open (pre-existing, reachable after F3) |
| F5 | MED | storage.rs:697–735 | open |
| F6 | MED | storage.rs write sites | open (pre-existing) |
| F7 | OK | mod.rs:129–190, query.rs | verified; body-equivalence not diffed |
| F8 | MED | query.rs:78–110 | open |
| F9 | LOW/MED | mod.rs:335–342 | open (designed) |

Recommended order: F1 and F3 together (ordering plus a drain point), then F2 and F4 (no silent failure or empty default), then F5 and F6, then F8.

Decision needed from the operator: whether to keep write-behind with these fixes, or revert `store_local` to synchronous writes and accept the guard hold. Reverting is the smaller change and restores durability immediately; the freeze work then needs another route.

No build or test was run for this review.
