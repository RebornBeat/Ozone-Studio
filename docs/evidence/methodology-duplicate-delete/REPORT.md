# Duplicate methodology hard-delete (EU, 2026-11-11) — operator approved "hard-delete 40252, 40348 and 40394 with the checks above"

Outcome: **1 of 3 deleted (40348). 40252 and 40394 NOT deleted — the citation check failed for both.** Nothing else touched; no restart, no build, no tests, settlement untouched.

## Checks
1. Before-copies: `before/container_<id>.json` + `before/history_<id>.json` (all six containers: three duplicates + three canonicals) and `before/bodies/` (the six body files). The duplicates have no children and no relations; `DeleteContainer` removes the container, drops its in-memory version history and unlinks it from its parent's child list. It does NOT delete the body file under `zsei_data/methodologies_created/` (still present for 40348). `restore.mjs` re-creates a container from its saved copy (UNTESTED; a restore gets a NEW id).
2. Content coverage (read by hand, rule by rule): 40252 -> 40305: every rule/heuristic is present verbatim or as a restatement (version control/history, approve-before-apply, baseline, link-to-version; automate; drift). 40348 -> 40350: all 3 rules and 2 heuristics are verbatim in the canonical. 40394 -> 40396: the merge of 2026-11-10 (methodology-merge-2) moved the non-restatements; the remaining ones are restated in 40396. All three PASS.
3. Citations: EU data-dir grep (`eu_id_citations_files.txt`), repo grep (no source/UI/bootstrap reference to any of the three ids), and a full scan of every container's local state through the API (`scan_citations.py`, 3,303 containers, `eu_container_citations.json`):
   - **40348: no citation anywhere** (only history: rollback snapshots, hit log, host.log, ledgers) -> PASS.
   - **40252: cited** by Blueprint 40255 "AMT Re-Expansion Initial Build Blueprint" (context.methodologies + step methodology_ids) and Derived 40253 (the AMT) -> FAIL, stopped.
   - **40394: cited** by Blueprint 40397 "AMT Re-expansion Failure Recovery Blueprint for Container 3644" (context.methodologies + step methodology_ids) -> FAIL, stopped. (Also named in capability_gaps.json provenance text; that part is history.)
   The pilot order in the directive (40394 first) could not be followed because 40394 failed; 40348, which passed every check, was done as the single pilot.
4. Delete 40348 and verify: `DeleteContainer` returned Success; `/health` 200, service active, 0 errors/panics in the following 2 minutes of the journal; GetContainer 40348 = not found; canonical 40350 loads (v9); keyword gather for "container management" returns 11 -> 10 containers (only 40348 gone); re-scan of all containers: no dangling reference to 40348 (`after_scan_40348.txt`; the scanned count stayed 3,303 because the live store gained a container meanwhile, not a verification gap — 40348 itself is gone by GetContainer).

## Open for the operator
- To delete 40252 / 40394 the blueprints 40255 / 40397 (and Derived 40253) must first be repointed at 40305 / 40396, or you accept their dangling ids. Not done: that edits blueprints, outside this approval.
- Hit-log lines naming 40348 stay (logs are never rotated); the rollup tolerates a missing container (`keyword rollup: container not updated`, debug).
