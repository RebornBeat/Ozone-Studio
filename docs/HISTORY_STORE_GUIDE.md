# Native history store (src/history) — one git-like history for files AND containers

Status 2026-11-12: PARSE-ONLY (rustfmt / node --check). Never compiled, run or deployed.

## Why
Two stores existed: the file beacon's `file_history/` (workspace files; full copy per change) and the integrity monitor's
`integrity/rollback/` (container records; full copy per mutation, in-memory index lost on restart, silent 100-version cap, and
`rollback()` had NO caller). Operator: "reconstruct via diff, takes less space than complete backups, make both work together".

## Layout (`<OZONE_ZSEI_DATA_DIR|zsei_data>/history`)
- `objects/<hh>/<blake3 hex>` — one content-defined chunk (gear-hash CDC, ~8 KiB avg, 2 KiB min, 64 KiB forced boundary), stored ONCE.
- `versions/<scope>/<id>.jsonl` — append-only rows `{v, ts, reason, path?, size, hash, chunks[]}`, one per DISTINCT content.
  scope `file` = workspace file (id = file container id); scope `container` = ZSEI record (id = container id).
- A new version adds only the chunks that changed (that is the diff-based saving); a version is rebuilt by concatenating its chunks, each
  chunk and the whole blake3-verified. No delta chains, no keyframes. Differences are computed on demand by git-mcp.
- Never pruned or capped (no-caps doctrine). `IntegrityConfig.max_versions` is ignored. Chunk sizes are chunking parameters, not caps.
- Durable: temp + fsync + rename for objects, append + fsync for rows. Failures are returned; writers retry (integrity `retry_until_ok`,
  beacon forgets its baseline so the next poll retries).

## Writers
- File beacon (`src/file_beacon.rs`): records a BASELINE on first sighting each boot (captures changes made while down and gives the first
  change a "before"), then a version on every detected change.
- Integrity monitor (`src/integrity/mod.rs`) via `ZSEI::pre_write_snapshot`: the container's OLD content (or its raw bytes when it does not
  load, snapshot option A) before every update/delete. NOTE: each row is the state BEFORE a write; the live record is the newest state.

## Readers / actions
- git-mcp (`tools/git-mcp`, :3235): `history`, `status`, `diff` (Myers line diff with 3-line-context hunks; binary = chunk counts),
  `read_version`, `restore` (calls the host), legacy aliases `file_history/file_status/file_diff`. Ids restricted to `[A-Za-z0-9_-]`.
- Host: `POST /history/list {scope,id}`; `POST /history/restore {scope,id,version,confirm:"RESTORE"}` — operator action; records the
  current state first (`pre-restore`), audit row in `history_restores.jsonl`. Container restore = `ZSEI::restore_container_version`
  (store_container; check at cargo check that the keyword index/cache stay consistent — it re-stores the record, it does not re-index).
  File restore writes the recorded path atomically. This is the first caller of rollback data (`IntegrityMonitor::rollback` still returns bytes).

## Migration
`history::open_shared` (default root only) runs `migrate_legacy` once: imports `integrity/rollback/*.snapshot` (scope container, reason
`legacy-integrity`) and `file_history/` snapshots (scope file, reason `legacy-beacon`), ORIGINALS KEPT. Idempotent; marker
`history/.legacy_imported_v1` written only on full success (failure = retried next boot, loud log).

## Jurisdiction
History is append-only, so personal-data erasure must reach it (open gap G10, jurisdiction phase 3). `History::scopes/list/read_version`
give the erasure planner the enumeration it needs; a chunk shared by several versions needs reference-aware removal.

## Check first at cargo check
`History::record_with` lock held across blocking fs (fine: called from spawn_blocking / beacon); `integrity` spawn_blocking closures own
their data; `run_check` assigns `containers_checked` declared `let mut`; `zsei/mod.rs` `restore_container_version` uses
`IntegrityMonitor::history_handle`; grpc handlers `post_history_*`; `config::IntegrityConfig` default path changed to `zsei_data/history`
(lib.rs rewrites default values to `history::default_root()`); tests in `src/history/mod.rs` and rewritten integrity tests.


## Integration (2026-11-12, creds-history-ui; parse-only)
- **Read routes:** `POST /history/status | /history/list | /history/diff | /history/read` (chunked offset/length, no size refusal) — all protected; host-side Myers diff in `src/history/diff.rs` (same hunks as git-mcp). ozone-mcp read tools: `ozone_history_status`, `ozone_history`, `ozone_history_diff`, `ozone_history_read`, plus `ozone_held_calls` — so an agent in an MCP call can see WHAT CHANGED.
- **Restore (operator only, `confirm:"RESTORE"`):** containers restore through `ZSEI::restore_container_version` → the NORMAL `UpdateContainer` path (pre-write snapshot of the current record, write-time keyword gate and `record_keywords_changed`, cache invalidation, version bump, graph ripple). Restored parts: metadata, context (keywords/scope), storage pointers, hints. NOT restored: structure, type-specific contexts, integrity data; a deleted container is not re-created (read it via `/history/read`). Files: the recorded path is rewritten atomically and the beacon then records the restored content (consistent ripple for linked files).
- **What is recorded automatically:** (1) linked file containers (beacon: baseline + every change; the container ripple fires on change); (2) container records (before each update/delete); (3) WATCH ROOTS — `OZONE_HISTORY_WATCH` (":"-separated; default existing of `docs`, `CHECKLIST.md`, `AGENTS.md`, `.ozone-context`) text-like files (`OZONE_HISTORY_WATCH_EXT`) as scope `file`, id `p<24 hex of blake3(path)>`: history-only (no container, no ripple). Checklists, step files, evidence docs and shared-context state are therefore versioned and visible to agents via the MCP tools.
- **Walk context:** the step context gets a "[Recent changes to the attached files (history store)]" block (`History::changes_block`: newest version, reason, +/- lines versus the previous). Read-only text; walk control flow untouched. Only attached files are covered (linked-but-unattached files are reachable through the MCP tools / History tab).
- **Discoverable as graph resources:** a linked file's history is keyed by its container id (`scope file`, id = container id), so the file container IS the graph resource; `/history/status` lists the recorded path for every item. Content is never duplicated into the graph.
- **UI:** `HistoryPanel` (versions, diff, RESTORE with typed confirm) embedded in Files → History (file + container record) and the new **History** tab (every item, filter, select). Old beacon section (MCP `file_history`) removed from `VersionHistory.tsx`.
- **Check first at cargo check:** `History::by_path` mutex use in `record_with` (the `latest` guard `g` is held while `by_path` is locked: lock order latest → by_path, `id_for_path` takes only by_path then `read_rows` — no inversion), `diff.rs` borrow lifetimes in `myers`, `post_history_*` closures, `restore_container_version` (`self.query` re-entrancy: no locks held), `stages.rs` `.await` inside the context block, `file_beacon::poll_watch` (spawn_blocking take/return of the cache).
