# Manual keyword validation — EU Methodology containers (fork V1, 2026-11-10)

Scope: every UNVALIDATED non-marker keyword on EU Methodology containers — **92 containers, 1,785 terms** — judged by reading each container's name, topics and real body (data dir, then app root). Settlement stays paused (`OZONE_KEYWORD_SETTLE=off`); Ozone's own loop was not used. No source file was edited, nothing was built or tested, nothing was committed.

## Result
| | terms |
|---|---|
| validated (`validated:true`) | **1012** |
| removed from `keywords` and recorded `validated:false` (never re-proposed) | **443** |
| left untouched (still unvalidated: plausible but not supported by the body) | **330** |

Post-apply re-scan of the 92 containers: 330 unvalidated remain (= the LEFT terms), 0 removed terms still stored, 0 LEFT terms changed. Host `/health` = 200 after every container; service `active`.

Method: YES needs support in the body/name/topics (ledger records the support flag: S = every term token occurs there, P = some, N = none; every N-flag YES was reviewed by hand: acronyms of the name such as TDD/EDA/dedup, topic words, or the practice the body describes); NO needs a clear reason (not the content's language per KEYWORD_RULES, wrong sense, an id/number, malformed token, generic fragment, templated "<phrase> + generic noun" variants, jumbled word order, a different named concept); anything I could neither support nor call wrong was LEFT (asymmetric — never removed on doubt). No scores anywhere, yes/no only. Markers were never touched. The two pilots (30019, 40322, then 30005 for the removal path) were re-read through GetContainer before the rest were applied in batches; every container was re-read after its update and checked for: removed terms gone, YES terms stored with `validated:true`, keyword order otherwise unchanged, LEFT terms unchanged, `object_store_path` and name unchanged — 0 problems in all 92.

## What the judging found
- The zero-shot expansion of **30005 Security Awareness** misread the title: the body is secure-coding awareness (validate input, no secret literals, safe error messages) but 13 expansion terms were about employee security-awareness training (phishing, culture, human firewall, campaigns, months, metrics, platform). All removed as wrong sense.
- **Foreign-language expansions** (Spanish, French, German, Italian, Portuguese, Chinese, Japanese) were stored on English content (30001, 30002, 30023, 30032, 30037, 40323, 40353): removed, KEYWORD_RULES requires the content's own language.
- **30021 (111 terms), 30031 (81), 40350 (37), 40349 (40), 40327 (28), 40268 (39)** carried large families of templated recombinations ("host lifecycle governance", "content privacy", "verify live defaults", "container management pod", "workflow automation toolkit"): the distinct concept was kept, the recombinations removed or left.
- **Ids/numbers as keywords** ("container 40211", "container-40211", "container 3644") removed — KEYWORD_RULES excludes ids and numbers.
- Malformed tokens with trailing punctuation (`behavior,`, `depth,`, `active,`, `artifact,`) and a possessive fragment (`loader's`) removed.
- The 38 regenerated methodologies' keywords were not changed by the regeneration (only re-sorted/deduped); their terms were judged against the NEW bodies (rules).

## Regenerated methodologies (docs/evidence/methodology-regen) — judged against their new rule bodies
34 of the 38 had unvalidated terms (the other 4 had none): validated 231, removed 105, left 176. Their ids are marked `regen` in the table.

## Not done / could not verify
- Only Methodology containers (fork V2 owns the other types).
- Hit/judgment logs: the host's own `keywords_changed` events were produced by the UpdateContainer calls; I did not inspect `keyword_hits.jsonl` to confirm them.
- `validated:true` here means manually reviewed by me, not the strength-N gate; TermMeta has no field for the validator, so the ledger (`decisions.jsonl`) is the record of who/why.
- The LEFT terms (330) stay pending; strict search remains OFF so they still match.
- Local store untouched (host down) — same containers on the local store are not validated.

## Restore
- A removed term back as pending: `node tools/keyword-measure/restore_term.mjs <container_id> "<term>"` (on the EU: `/opt/ozone-build/tools/keyword-measure/restore_term.mjs`); it restores `validated:null` with the old hit counters.
- Whole container back: `before/container_<id>.json` holds the full pre-change container; re-apply its `local_state.context` through `UpdateContainer` (as `backlog_cleanup.mjs` does).
- A validated term back to pending: set its `keyword_meta[term].validated` to null in the same way.

## Files
`decisions.jsonl` (1,785 lines: container, name, term, decision yes/no/left, reason, previous meta) · `before/` (92) · `after/` (92) · `results/` (apply logs) · this file.

## Per container
| id | name | yes | no | left | regen |
|---|---|---|---|---|---|
| 1001 | Django/React Web App Deployment on Ubuntu VPS (nginx + gunic | 6 | 0 | 0 |  |
| 1003 | Ozone-Studio Self-Deployment on Ubuntu VPS (tarball + system | 22 | 0 | 1 |  |
| 1004 | Security Monitoring via Scoped-Capability MCP (read-only, un | 20 | 0 | 1 |  |
| 30001 | Conversational Response | 17 | 8 | 8 |  |
| 30002 | Contextual Awareness | 8 | 3 | 0 |  |
| 30003 | Clean Code Principles | 26 | 3 | 4 |  |
| 30004 | Code Review Best Practices | 14 | 0 | 4 |  |
| 30005 | Security Awareness | 8 | 14 | 2 |  |
| 30006 | Impact-First Development | 6 | 0 | 0 |  |
| 30007 | Test-Driven Development | 9 | 0 | 0 |  |
| 30008 | Incremental Development | 13 | 5 | 3 |  |
| 30009 | Data Analysis Methodology | 11 | 2 | 2 |  |
| 30010 | API Design Principles | 12 | 1 | 4 |  |
| 30011 | Documentation Standards | 8 | 2 | 5 |  |
| 30012 | Technical Writing | 9 | 1 | 5 |  |
| 30013 | Exploratory Analysis | 12 | 0 | 1 |  |
| 30014 | Formal Proof Methods | 11 | 1 | 8 |  |
| 30015 | Mathematical Rigor | 7 | 1 | 9 |  |
| 30016 | Host Lifecycle Management | 11 | 13 | 0 |  |
| 30017 | Memory-Pressure-Aware Build Sequencing | 6 | 0 | 0 |  |
| 30018 | CWD-Independent Path Resolution | 14 | 1 | 6 |  |
| 30019 | Retry Escalation Must Cycle | 12 | 0 | 2 |  |
| 30020 | Cross-Agent Claim Verification | 15 | 5 | 1 |  |
| 30021 | Jurisdiction Content Sourcing Standard | 21 | 72 | 18 |  |
| 30022 | Persistent-vs-Cached Field Parity | 16 | 1 | 0 |  |
| 30023 | Idempotency Checks Must Read Durable State | 11 | 4 | 3 |  |
| 30024 | Incremental Layering Requires Reconciliation | 14 | 3 | 5 |  |
| 30025 | Trace the Full Write-to-Read Path | 9 | 1 | 3 |  |
| 30026 | Attribution Requires a Capability Check | 15 | 4 | 2 |  |
| 30027 | Broad Relevance Discovery Pattern | 13 | 7 | 4 |  |
| 30028 | Extract for Testability | 19 | 6 | 1 |  |
| 30029 | Verify Test Assumptions Against Documented Exceptions Before | 10 | 7 | 0 |  |
| 30030 | Enforcement Actions Need Real Behavior, Not Just Labels | 15 | 7 | 0 |  |
| 30031 | Verify What's Actually Active, Not Just What's Built | 24 | 49 | 5 |  |
| 30032 | Self-Assessment Prompts Need a Calibration Bar | 9 | 10 | 2 |  |
| 30033 | Verify the Artifact, Not the Status | 12 | 8 | 5 |  |
| 30035 | Rescue JSON From Model Noise | 11 | 6 | 0 |  |
| 30036 | Persist Gates Must Measure Content Depth, Not Element Count | 9 | 9 | 6 |  |
| 30037 | Canonical Container Resolution Must Be Order-Independent | 12 | 8 | 0 |  |
| 30038 | Tune From Measured Metrics, Never Preemptive Caps | 15 | 6 | 0 |  |
| 30039 | A Guard Held Across a Forever-Loop Is a Permanent Lock | 11 | 14 | 10 |  |
| 30040 | Persisted Shape Must Match the Loader's Parse Shape | 11 | 7 | 1 |  |
| 30041 | New Nodes Link Through Relationship Paths First | 15 | 9 | 2 |  |
| 30042 | Planning Artifacts Outlive the Process That Made Them | 20 | 6 | 5 |  |
| 30043 | Every Failure Path Marks State Truthfully | 16 | 7 | 1 |  |
| 30044 | Session Token Metering | 14 | 3 | 6 |  |
| 40084 | MCP Tool Failure & HTTP Response Recovery | 20 | 0 | 1 |  |
| 40125 | AMT Initial Build & Re-expansion Procedure | 4 | 2 | 0 |  |
| 40126 | AMT Re-Expansion Initial Build Methodology | 25 | 2 | 0 |  |
| 40185 | MCP Capability Declaration Verification | 8 | 0 | 0 |  |
| 40222 | AMT Re-expansion MCP Tool Failure Handling | 14 | 0 | 0 |  |
| 40226 | amt-reexpansion-profile-resolution | 22 | 2 | 2 |  |
| 40252 | Configuration Management | 11 | 0 | 3 | regen |
| 40262 | AMT Re-expansion Initial Build Container 40211 — Registry Ac | 23 | 1 | 0 |  |
| 40264 | AMT Hierarchy Initial Build via Registered Tools | 18 | 1 | 2 |  |
| 40268 | AMT Re-expansion Rollback Methodology | 22 | 15 | 1 |  |
| 40302 | API Design | 13 | 4 | 9 | regen |
| 40303 | Authorization | 8 | 0 | 3 | regen |
| 40304 | Boot Sequence | 16 | 0 | 3 | regen |
| 40305 | Configuration Management | 9 | 1 | 11 | regen |
| 40307 | Consistency Checking | 7 | 1 | 3 | regen |
| 40308 | Concurrency Control | 8 | 0 | 7 | regen |
| 40309 | Component Restoration | 12 | 0 | 9 | regen |
| 40311 | Dependency Management | 4 | 0 | 4 | regen |
| 40312 | Data Validation | 10 | 10 | 5 | regen |
| 40313 | Deduplication | 15 | 0 | 6 | regen |
| 40314 | Error Handling | 1 | 0 | 0 | regen |
| 40315 | Hierarchy Construction | 6 | 2 | 3 | regen |
| 40316 | HTTP Parsing | 1 | 0 | 0 | regen |
| 40317 | Input Validation | 19 | 1 | 12 | regen |
| 40320 | Mapping | 7 | 3 | 1 | regen |
| 40321 | Resilience Engineering | 1 | 0 | 0 | regen |
| 40322 | Software Testing | 1 | 0 | 5 | regen |
| 40323 | State Propagation | 4 | 3 | 6 | regen |
| 40324 | Storage Reconciliation | 6 | 4 | 6 | regen |
| 40325 | System Architecture | 3 | 0 | 4 | regen |
| 40326 | Verification | 13 | 1 | 13 | regen |
| 40327 | Workflow Automation | 5 | 17 | 6 | regen |
| 40348 | Container Management | 6 | 0 | 10 | regen |
| 40349 | Capability Declarations | 13 | 15 | 12 | regen |
| 40350 | Container Management | 5 | 23 | 9 | regen |
| 40351 | Registry Management | 3 | 5 | 17 | regen |
| 40352 | Capability Management | 10 | 5 | 3 | regen |
| 40353 | Rollback Strategies | 9 | 10 | 6 | regen |
| 40383 | MCP Capability Management | 1 | 0 | 0 | regen |
| 40384 | Canonical Resolution & Deduplication | 1 | 0 | 0 | regen |
| 40386 | Container Lifecycle Management | 1 | 0 | 0 | regen |
| 40388 | Testing & Verification | 1 | 0 | 0 | regen |
| 40389 | HTTP Protocol Error Handling | 1 | 0 | 0 | regen |
| 40394 | Farm Profile Start Failure Recovery | 8 | 1 | 1 |  |
| 40395 | Registered Farm Start Fallback | 7 | 0 | 1 |  |
| 40396 | farm-profile-start-failure-recovery | 11 | 0 | 1 |  |
