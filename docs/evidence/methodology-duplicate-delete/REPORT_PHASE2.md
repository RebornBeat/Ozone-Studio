# Phase 2 — duplicate repoint + hard delete on the EU (2026-11-12 01:54–01:56Z)

Scope: run `tools/methodology-duplicate-repoint/repoint_and_delete.mjs` on the EU. No build, no tests, settlement untouched, no service restart, API key never printed.

## Inspection first (the earlier attempt was stopped)
EU `ozone-studio` active, `/health` 200, binary unchanged (Nov 10 06:44). 40394/40396/40252/40305/40397/40255/40253 all present; no partial scripts, run dirs or backups on the EU. Nothing from the stopped attempt had applied.

## Scripts
Staged at `/root/ozone-dupfix/` (same layout as the repo; sha256 identical to the repo copies: repoint_and_delete.mjs 470300fb…, restore_repoint.mjs 1a27bb70…) with the 2026-11-10 before-copies. Run output was copied back to `docs/evidence/methodology-duplicate-repoint/run-*`.

## Dry run (run-20261112T015446Z-dry): clean for both pairs
All stop-conditions held: both canonicals keep every saved rule (40396: 13/13, 40305: 11/11); live citers exactly as expected; no stray body refs; no numeric reference would remain; both duplicates have 0 children.

## Prerequisite for 40394 (plan §12 "40396 must be updated first")
Checked rather than assumed: the CURRENT 40396 body holds a restatement of every decision rule and heuristic of 40394 (including the "use farm_state_get, farm_profile, farm_state_set, farm_profile_start in sequence", the registry-capability rule and the "avoid blind retries" heuristic). Deleting 40394 first loses no content. The rule-fix apply (P3) addresses 40396 by id and is unaffected. The ordering note is therefore a precaution, not a blocker; it did not block.

## Applied
| Pair | Run | Result |
|---|---|---|
| 40394→40396 (pilot) | run-20261112T015520Z | 40397 repointed (v4→v5, context + body `blueprints/walk_1794298398587540751.json`); 40394 DELETED; 40396 loads (v7); /health 200; journal 0 ERROR/panic |
| 40252→40305 | run-20261112T015553Z | 40253 (v7→v8) and 40255 (v12→v13) repointed (contexts + bodies `amt/amt_1794258230_22.json`, `blueprints/walk_1794258481774727006.json`); 40252 DELETED; 40305 loads (v7); /health 200; journal 0 ERROR/panic |

Independent verification (not from the script's own output): grep of `local/*.json`, `blueprints/*`, `amt/*`, `methodologies_created/*` for 40394 and for 40252 → no hits; 40255 and 40253 now list `[40126, 40305]`; 40397's methodology list contains 40396; service active, /health 200, journal 0 error lines in the last 5 min.

History is not rewritten (runs/, capability_gaps.json, hit/judgment logs keep the old ids).

## Facts for the later 7-draft delete (plan §12 step 2b)
Blueprint 40397 also cites other redundant drafts (40226, 40222, 40268, 40262, 40126, 40264, 40395, 40084): those citers must be repointed to 40396/40125 before each of the seven is deleted. Blueprints 40255/40253 still cite 40126.

## Restore
- Repoint undo: `node tools/methodology-duplicate-repoint/restore_repoint.mjs <run-dir> --data /opt/ozone-studio/zsei_data --apply` (run dirs are under `docs/evidence/methodology-duplicate-repoint/`; before-copies of the duplicates, citers, bodies and version histories are in each run's `before/`).
- Deleted duplicate: `docs/evidence/methodology-duplicate-delete/restore.mjs <id>`; it gets a NEW id, then `restore_repoint.mjs … --rebind <newId>`. These restore paths were not exercised.
- EU scratch copy: `/root/ozone-dupfix/` (can be removed; evidence is in the repo).

Unverified: gather counts and keyword search after the deletes were not re-measured (only /health, canonical load, reference scans, journal).
