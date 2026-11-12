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
