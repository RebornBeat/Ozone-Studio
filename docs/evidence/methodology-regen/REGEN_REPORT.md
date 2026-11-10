# Methodology regeneration — rule-less principles-only methodologies (EU)

Run 2026-11-10 by the fork. Authored by the agent directly (rules.py / payloads.json), applied on the EU through `/pipeline/execute` id 12 `Update` (`add_decision_rules` + `add_heuristics`, `requires` empty, no keywords, no principles). No builds, no tests, no source edits, settlement left PAUSED.

Scan (EU local/*.json, 98 methodologies): 38 data-dir principles-only bodies with 0 rule_items = the targets; 5 `{methodology, steps}` app-root bodies = skipped; the other 55 already have rules.

## Result
- targets 38 · applied 38 · failed 0 · skipped 0 (of targets) · 5 out-of-scope skipped (below)
- Canaries first (40302, 40317): body had rules + description/storage/principles unchanged, keywords/keyword_meta/topics unchanged, container reloads via GetContainer, /health healthy, old body file kept. Then the other 36; /health healthy after.
- Post-check of all 38 (before vs after): `storage` block (name/category/principles), `description`, container name, topics, keyword set all identical; the update wrote a NEW versioned file under `methodologies_created/` and the container's `object_store_path` points at it; the old file is kept. Counts below are rule_items by the PromptOrchestrator::rule_items definition, recomputed from the after body.
- Observed side effects of the pipeline (not from this task's input): keywords in the container are re-sorted/deduped; for 40383, 40384, 40388 the host created an empty `keyword_meta` provenance row (`source: created:methodology_create`, validated null, zero hits) for their single keyword where there was none — no existing data lost. Pipeline version counter incremented.

## Table
| id | name | rules / heuristics added | rule_items before → after | new body path | status |
|---|---|---|---|---|---|
| 40252 | Configuration Management | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315125_40252.json` | applied; content preserved |
| 40302 | API Design | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794314983_40302.json` | applied; content preserved |
| 40303 | Authorization | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315125_40303.json` | applied; content preserved |
| 40304 | Boot Sequence | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315126_40304.json` | applied; content preserved |
| 40305 | Configuration Management | 5 / 2 | 0 → 7 | `methodologies_created/methodology_1794315126_40305.json` | applied; content preserved |
| 40307 | Consistency Checking | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315127_40307.json` | applied; content preserved |
| 40308 | Concurrency Control | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315127_40308.json` | applied; content preserved |
| 40309 | Component Restoration | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315128_40309.json` | applied; content preserved |
| 40311 | Dependency Management | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315128_40311.json` | applied; content preserved |
| 40312 | Data Validation | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315128_40312.json` | applied; content preserved |
| 40313 | Deduplication | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315129_40313.json` | applied; content preserved |
| 40314 | Error Handling | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315129_40314.json` | applied; content preserved |
| 40315 | Hierarchy Construction | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315129_40315.json` | applied; content preserved |
| 40316 | HTTP Parsing | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315129_40316.json` | applied; content preserved |
| 40317 | Input Validation | 5 / 2 | 0 → 7 | `methodologies_created/methodology_1794315008_40317.json` | applied; content preserved |
| 40320 | Mapping | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315130_40320.json` | applied; content preserved |
| 40321 | Resilience Engineering | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315130_40321.json` | applied; content preserved |
| 40322 | Software Testing | 5 / 2 | 0 → 7 | `methodologies_created/methodology_1794315131_40322.json` | applied; content preserved |
| 40323 | State Propagation | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315131_40323.json` | applied; content preserved |
| 40324 | Storage Reconciliation | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315132_40324.json` | applied; content preserved |
| 40325 | System Architecture | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315132_40325.json` | applied; content preserved |
| 40326 | Verification | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315133_40326.json` | applied; content preserved |
| 40327 | Workflow Automation | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315133_40327.json` | applied; content preserved |
| 40348 | Container Management | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315134_40348.json` | applied; content preserved |
| 40349 | Capability Declarations | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315134_40349.json` | applied; content preserved |
| 40350 | Container Management | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315134_40350.json` | applied; content preserved |
| 40351 | Registry Management | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315135_40351.json` | applied; content preserved |
| 40352 | Capability Management | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315135_40352.json` | applied; content preserved |
| 40353 | Rollback Strategies | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315136_40353.json` | applied; content preserved |
| 40381 | Error Handling & Resilience Engineering | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315136_40381.json` | applied; content preserved |
| 40382 | Input Validation & Data Integrity | 5 / 2 | 0 → 7 | `methodologies_created/methodology_1794315136_40382.json` | applied; content preserved |
| 40383 | MCP Capability Management | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315137_40383.json` | applied; content preserved (+empty keyword_meta provenance row created by host) |
| 40384 | Canonical Resolution & Deduplication | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315138_40384.json` | applied; content preserved (+empty keyword_meta provenance row created by host) |
| 40385 | Hierarchy Construction & Re-expansion Workflow | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315138_40385.json` | applied; content preserved |
| 40386 | Container Lifecycle Management | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315138_40386.json` | applied; content preserved |
| 40388 | Testing & Verification | 3 / 2 | 0 → 5 | `methodologies_created/methodology_1794315139_40388.json` | applied; content preserved (+empty keyword_meta provenance row created by host) |
| 40389 | HTTP Protocol Error Handling | 5 / 2 | 0 → 7 | `methodologies_created/methodology_1794315139_40389.json` | applied; content preserved |
| 40391 | Registry Access & Provider Management | 4 / 2 | 0 → 6 | `methodologies_created/methodology_1794315139_40391.json` | applied; content preserved |

## Skipped (out of scope, reason)
The 5 `{methodology, steps}` app-root methodologies (and any app-root body): the deployed methodology_create reads the data dir only for Update, so an Update would rewrite them with only the additions. Needs the fixed pipeline (source fixed, NOT rebuilt) — redo after the rebuild. IDs: 1000, 1001, 1002, 1003, 1004.

## Refinement candidates (rules cover the core concern only; principles are broad)
40321 Resilience Engineering, 40352 Capability Management, 40325 System Architecture, 40388 Testing & Verification (thin principles, generic rules). Duplicate-named pairs: Configuration Management 40252/40305, Container Management 40348/40350 (similar content, not merged). Rules are grounded only in each methodology's name/category/principles; no providers (`requires` empty), no legal claims.

## Reversal
`before/` holds each target's original body + container JSON. The old body files are still in the EU store; reverting = UpdateContainer pointing `storage.object_store_path` back at the old path in `before/container_<id>.json`.

## Files
rules.py (authored content), payloads.json (what was sent), apply.mjs (applier run on the EU), before/, after/.
