# Methodology merge — EU, 2026-11-10 (fork M). Nothing compiled; no source edited; nothing committed.

All 4 bodies live under the DATA dir (`methodologies_created/…`), so the deployed `methodology_create` Update was safe. Applied through pipeline 12 `Update` (additive only). `/health` 200 after each pair. Settlement still paused.

| pair | canonical | why | moved from the duplicate | rules before→after (canonical) | duplicate |
|---|---|---|---|---|---|
| Configuration Management | **40305** (v5) | richer (5 rules, 6 principles, 20 kw vs 4 / 5 / 14); cited by no live content (the ids found elsewhere are AMT re-expansion artifacts) | rules: baseline-before-change; link deployed state to exact version+change. Heuristics: automate a step repeated by hand; treat divergence from baseline as drift. Principles: Baseline Establishment, Automation (descriptions written from its own rules). Keywords (validated:true in 40252, missing in 40305): baseline management, configuration control, system baseline, system configuration, traceability, automation — marked validated:true on 40305 | rules 5→7, heuristics 2→4, principles 6→8 | 40252 untouched (4 rules, 2 heuristics) |
| Container Management | **40350** (v7) | richer (4 rules, 5 principles) and more observed use (114 returned hits vs 44) | rules: orchestrate not manual; limit privileges/resources/network; define scaling+recovery. Heuristic: privileged user → reduce. Principle: Scalability and resilience. Keywords (validated in 40348): container automation, deployment, orchestration, scalability, security — marked validated:true on 40350 | rules 4→7, heuristics 2→3, principles 5→6 | 40348 untouched (3 rules, 2 heuristics) |

Not moved (restatements of canonical content): version-control / change-record rules, "no resource limits → set them", the near-identical principle names.
Verified: new body has all old rules/heuristics/principles (old `storage.principles` intact; added principles are in the body's top-level `principles`), the previous versioned body files are kept (`methodology_1794315126_40305.json`, `…40350.json`), containers load, only the 4 targets' versions changed (40252/40348 `updated_at` unchanged).
Pre-existing, out of scope: the canonicals still hold the unvalidated "left" terms V1 chose not to decide (11 on 40305, 9 on 40350).

## Retire outcome: NOT RETIRED (no supported reversible verb)
The only removal verb is `DeleteContainer` (`methodology_create` `Delete` → `zsei/query.rs::delete_container`): a HARD, recursive delete that also drops the container's version history. There is no archive/deprecate verb and no restore verb. Per the directive the duplicates were left in place.
Effect today: 40252 and 40348 are still gathered by keyword like any methodology, so a request matching them still pulls both members of a pair. Their rules are now a subset of the canonical's, so the duplicated rules appear twice in the prompt (harmless to correctness, wasteful and the rule gate judges them twice).
Options (operator): (a) keep both (status quo); (b) hard-delete the duplicate with the saved `before/` copy as the only restore path (restore = recreate the container from before/container_<id>.json + body, new container id unavoidable) — not recommended without a supported verb; (c) add a reversible retire verb (e.g. `retired:true` in context that gathering skips) in source later.

## Revert recipe
Point the canonical's `storage.object_store_path` back at its previous body: 40305 → `methodologies_created/methodology_1794315126_40305.json`; 40350 → `methodologies_created/methodology_1794315134_40350.json` (via UpdateContainer), and remove the 11 added keywords (or restore the container JSON from `before/container_<id>.json`).

Files: payloads.json, apply_merge.mjs, mark_validated.mjs, before/ (4 containers + 4 bodies), after/ (4 containers + 4 bodies; 40252/40348 = unchanged copies).
