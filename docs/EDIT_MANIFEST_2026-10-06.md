# Edit manifest — 2026-10-06

Capture of every changed and new file in the working tree, with its author, what changed, and its build status. No builds, tests or edits were run to produce this document. It was assembled from `git status --short`, `git diff --stat`, file mtimes compared with the host binary, and the CHECKLIST entries dated 2026-10-05 and 2026-10-06.

## Method and facts

- Tree: 42 modified tracked files (2137 insertions, 527 deletions), 19 untracked paths.
- Host binary `target/release/ozone-studio`: mtime 07:08:00.
- Sources newer than the host binary: `src/mcp.rs` and `src/orchestrator/mod.rs` only. Both were touched during the tool-summary cap and its revert, and their content was restored to the pre-cap state. Every other Rust, MJS and Python file predates the binary, so the host binary covers them.
- Pipeline crate binaries (`target/pipelines/release`) are not rebuilt by the host build. Every pipeline change is UNVERIFIED by build.
- MJS and Python tool servers are not built. They were checked with `node --check` or syntax checks only where noted.

## Status legend

- **BUILT**: the file predates the host binary (07:08), so the binary covers it. Compile success for the host crate is implied; live behavior is not verified.
- **PARSE-CHECKED**: `rustfmt --check` parse or `node --check` only. Not compiled.
- **UNVERIFIED**: no compile or test covers the current content.
- **REVERTED**: changed and then restored; net content matches the pre-change state.

## Author key

- **CC**: Claude Code (this session).
- **ZC**: ZCode.
- **Mixed**: both edited the file. The CHECKLIST entries do not separate every hunk.
- **Fork**: produced by a fork and applied by CC.
- **Unknown**: no CHECKLIST entry explains the change.

## 1. Pipeline and gate (src/pipeline)

| File | +/- | Author | What changed | Status |
|---|---|---|---|---|
| src/pipeline/gate.rs (new) | 292 lines | ZC built; CC fixed | Ordered admission gate with User, Lane and Loop tiers. CC fixed: cancelled waiters are skipped on release (leak fix); borrow pattern in `release()`; test helper and hanging test rewritten; cancelled-waiter test added. | UNVERIFIED (not built with fixes) |
| src/pipeline/executor.rs | 48 +/- | CC (path + gate wiring) | Shared pipeline target path candidate; reject-at-cap replaced by `gate.admit(current_call_priority())`; `max_concurrent` field replaced by `gate`. | BUILT (binary predates change? see note) |
| src/pipeline/mod.rs | 1 | CC | `pub mod gate;` registered. | BUILT |

Note on executor.rs: it predates the binary, so the binary includes the gate wiring. Its live behavior is unverified.

## 2. Orchestrator (src/orchestrator)

| File | +/- | Author | What changed | Status |
|---|---|---|---|---|
| src/orchestrator/mod.rs | 235 +/- | Mixed; CC for watchdog, progress markers, `chain_candidates`, `capability_summary` store, byte-slice fix, `prefix_at_char_boundary`; ZC for health reorder, context-fit pre-order, C8 preservation, walk | Watchdog budget scales with candidate count, untried candidates listed on expiry; progress tracked via task-local; lane-split routing; capability summary stored (was discarded); char-safe prefix helper. The tool-summary cap was added and reverted; net content is pre-cap. | REVERTED (cap); file mtime is newer than the binary, so re-verify on next build |
| src/orchestrator/amt.rs | 148 +/- | Mixed; CC for lane budget (window-derived), known-branch and methodology guards (window-derived), lane tuple with spec, lane spawns tagged Lane, byte-slice fixes, lane-split hooks; ZC for full-chain lanes, loud warns, batching, marker vocabulary | See CHECKLIST 2026-10-05 and 2026-10-06 entries. | BUILT |
| src/orchestrator/stages.rs | 14 +/- | Mixed; CC for byte-slice fixes; ZC for markers and simulation | Char-safe slices (7 sites) and marker changes. | BUILT |
| src/orchestrator/jurisdiction.rs | 2 +/- | CC | Byte slice at the confirmation prompt replaced with `prefix_at_char_boundary`. | BUILT |
| src/orchestrator/response.rs | 2 +/- | CC | Byte slice at the cleaned prompt replaced with `prefix_at_char_boundary`. | BUILT |
| src/orchestrator/lane_split.rs (new) | 201 lines | Fork (worktree ab3eb311) built; CC applied and fixed | Per-candidate lane splitting: halves members until each part fits the candidate window; part without parsable branches now fails the part. | UNVERIFIED |

## 3. ZSEI and integrity (src/zsei, src/integrity)

| File | +/- | Author | What changed | Status |
|---|---|---|---|---|
| src/zsei/mod.rs | 25 +/- | ZC (lock-scope fixes) and CC (stale-cache fix) | `get_container` holds the storage read guard through the cache insert (CC fix for a stale-cache race ZC introduced); ZC's traverse `block_in_place` mitigation; store-container scoping. | BUILT (CC fix predates binary) |
| src/integrity/mod.rs | 65 +/- | CC | `create_snapshot`: version reserved under the lock, file written asynchronously outside it, rollback on failure, stale-file removal outside the lock. | BUILT |

Still open in this area (not changed): `query()` holds write guards across `process().await`; `store_local` does a synchronous write under the storage lock; `traverse` holds the read guard across the traversal.

## 4. Graph persistence and grpc (src/mcp_graph.rs, src/grpc, src/types)

| File | +/- | Author | What changed | Status |
|---|---|---|---|---|
| src/mcp_graph.rs (new) | 349 lines | CC | MCP graph-block persistence: validation before any write, containers for each entity, typed relations (including ImportsFrom and CallsTo), attribute content files. | BUILT |
| src/grpc/mod.rs | 109 +/- | CC | `query_zsei` write audit to `model_calls/zsei_writes.jsonl`, refusal switch `OZONE_ZSEI_REQUIRE_SESSION`; `mcp_call` graph persistence hook and `persisted_graph` response field. | BUILT |
| src/types/container.rs | 14 +/- | CC | Spatial `RelationType` variants (80-84); `DiscoveryMethod::ToolOutput`; `ContainerType::McpResult` (90) and `McpEntity` (91) with display names. | BUILT |

## 5. MCP (src/mcp.rs)

| File | +/- | Author | What changed | Status |
|---|---|---|---|---|
| src/mcp.rs | 0 net | CC | A relevance-capped tool summary was added and then reverted. Net content matches pre-change. | REVERTED (file mtime newer than binary) |

## 6. Lib and config

| File | +/- | Author | What changed | Status |
|---|---|---|---|---|
| src/lib.rs | 17 +/- | CC | `pub mod mcp_graph;`; background loop spawns wrapped with `with_priority(Loop, ...)` (meta, AMT re-expansion, I-loop, assistant). | BUILT |
| Cargo.toml | 10 +/- | CC | `[profile.release]` now `lto = "thin"`, `debug = "line-tables-only"`; new `[profile.dist]` (fat LTO, no debug). | BUILT |
| Cargo.lock | 62 +/- | Unknown | Root lock: dependency change visible (`num_cpus` to `syn`). No CHECKLIST entry explains it. Not from CC per CC's own note. | BUILT (content unexplained) |
| .cargo/config.toml | new, not tracked | CC | lld linker flag for `x86_64-unknown-linux-gnu`. **Gitignored** by the `**/config.toml` rule, so it is not in version control. | BUILT locally only |

## 7. Assets: shared and general pipelines

| File | +/- | Author | What changed | Status |
|---|---|---|---|---|
| assets/pipelines/shared/semantic_relations.rs (new) | 313 lines | CC (fork-derived) | Zero-shot relation validator: `validate` (verbatim), `validate_mapped` (domain labels mapped to ZSEI names, original kept), `validate_structural` (node IDs in the node list); `zsei_relation_json_for`; `AcceptedRelation` gained `verification` and `domain_relation`. | UNVERIFIED (not compiled in any pipeline) |
| assets/pipelines/general/text_analysis/main.rs | 13 +/- | CC | Two raw byte slices made char-safe with a local `prefix_chars_safe`. | UNVERIFIED |
| assets/pipelines/consciousness/collective_consciousness/main.rs | 11 +/- | CC | One raw byte slice made char-safe. | UNVERIFIED |

## 8. Assets: modality pipelines

Zero-shot relation wiring and byte-slice fixes. Each file also has a `#[path]` include of the shared validator.

| Group | Files | Author | What changed | Status |
|---|---|---|---|---|
| Group B | electromagnetic, geospatial, haptic, hyperspectral, IMU, kinematics | Fork B applied by CC; literal fix and `validate_mapped` switch by CC | Wrapper keeps original signature; relations request added; `zero_shot_relations` and `zero_shot_rejected` serde-defaulted fields; hyperspectral literal now sets the two new fields. Byte slices made char-safe in geospatial and kinematics. | UNVERIFIED |
| A2 | 3D, BCI, biology, CAD, depth | Fork A2 applied by CC | Structural zero-shot wiring on create paths; update paths still use legacy wrappers (known gap). 3D also has char-safe slices. | UNVERIFIED |
| C2 | network, proteomics, radar, sonar, sound, thermal | Fork C2 applied by CC | Structural zero-shot wiring; unmapped labels rejected with raw text kept. | UNVERIFIED |
| Text | text/main.rs (+137) | Mixed: ZC for rotation pre-order and grammar; CC for zero-shot validator, char-safe slices (2), and the entity-source fix | Relations wired at the grammar sites; legacy per-sentence path accepts nothing (no independent entity list); char-safe slices. | UNVERIFIED |

## 9. Tools (tools/)

| Server | +/- | Author | What changed | Status |
|---|---|---|---|---|
| tools/visual-mcp/server.mjs | 269 +/- | CC | Shape, pose (ultralytics yolov8n-pose), segmentation and scene-graph tools; argv-based image paths; `parseBackend` (crashes surface as errors); `detection_backend_notes`; graph blocks; header updated. | PARSE-CHECKED (node --check); live-verified earlier on test images |
| tools/chemistry-mcp/server.mjs | 19 +/- | CC | `chemGraph` graph block for `chem_analyze`. | PARSE-CHECKED |
| tools/dna-mcp/server.mjs | 23 +/- | Fork (dna) and CC | Coordinates corrected; `dnaGeneGraph` graph block. | PARSE-CHECKED |
| tools/dna-mcp/dna_worker.py | 3 +/- | Fork (dna) | `find_gene` reports 1-based coordinates. | UNVERIFIED (syntax only) |
| tools/geospatial-mcp/server.mjs | 14 +/- | Fork (geo) and CC | `geoGraph` graph block. | PARSE-CHECKED |
| tools/proteomics-mcp/server.mjs | 16 +/- | Fork (proteomics) and CC | `proteinGraph` graph block. | PARSE-CHECKED |
| tools/android-mcp/ (new) | server.mjs 265 lines; .gitignore | CC | Android control via adb: devices, device info, shell (requires confirm), screenshot, input, install (apk confined and realpath-checked), logcat. | PARSE-CHECKED |
| tools/mesh-mcp/ (new) | server.mjs 267 lines; requirements.txt | Fork (mesh), CC applied | trimesh analysis, conversion, STEP info, and `meshGraph` for connected components. | PARSE-CHECKED |
| tools/code-graph-mcp/ (new) | server.mjs 106; code_worker.py 339 | Fork (code-graph), CC corrected relations | Python static call and import graph via `ast`; edges use ImportsFrom and CallsTo. | PARSE-CHECKED |
| tools/code-graph-mcp/__pycache__/ | pyc file | Generated | Bytecode cache committed by accident into the working tree. Should be ignored. | Hygiene issue |
| tools/bridges | submodule | Not ours | Dirty submodule (`package-lock.json`, `src/control-plane/*.ts` modified). Not touched in this work. | Not ours |

## 10. Docs

| File | Author | Status |
|---|---|---|
| docs/RUNTIME_MEMORY_PLAN.md (new) | CC | Plan for review |
| docs/GRAPH_AUTHORING_GUIDE.md (new) | CC | Guide; Rollback claim corrected to "no handler" |
| docs/CONTEXT_HANDLING_AUDIT.md (new) | ZC | Audit (ZCode's claims partially corrected in CC review) |
| docs/CONTEXT_REVIEW_2026-10-06.md (new) | CC | Review of ZCode's context sites; completed coverage section |
| docs/CONTEXT_BUDGET_SERVICE.md (new) | CC | Design for the budget service; chunker history corrected |
| docs/CONTEXT_OBJECT_MODEL.md (new) | CC | Design: context as objects in the graph |
| docs/LATENCY_AUDIT.md (new) | Fork (latency), captured by CC | Audit of timeouts and latency paths |
| CHECKLIST.md | CC and ZC (appended) | Status record; additive entries only |

## 11. Scripts

| File | Author | Status |
|---|---|---|
| scripts/build-pipelines.sh (new) | CC | Builds pipelines with a release binary into `target/pipelines` with one job. Its last run was not captured (its log was lost), so the shared-dir contents are UNVERIFIED. |

## 12. Hygiene issues found while building this manifest

1. `.cargo/config.toml` is gitignored by `**/config.toml`. The lld linker setting will not survive a clean checkout. Fix: add a negation rule for `.cargo/config.toml` or move the setting to a tracked file.
2. `Cargo.lock` changed (`num_cpus` to `syn`) with no owner in the CHECKLIST. Confirm its origin before committing.
3. `tools/code-graph-mcp/__pycache__/` is committed-state content; add `__pycache__/` to ignore rules.
4. `tools/bridges` is a dirty submodule with changes that are not part of this work.
5. `src/mcp.rs` and `src/orchestrator/mod.rs` are newer than the host binary. Their content matches the pre-cap state, but the next build must include them.

## 13. Open items still unfixed (see CHECKLIST for full detail)

- `query()` write guard held across `process().await`; `traverse` read guard across traversal; `store_local` synchronous write under the storage lock.
- Lane splitting: retry lanes do not split; all-or-nothing per candidate; unknown window (0) treated as fitting.
- `ModelOverrideConfig` carries no `context_length`, so per-candidate windows do not reach the prompt pipeline.
- Silent caps (1,500-character chunk cuts in intent and detail prompts; 8,000, 6,000, 3,000 and 1,200 byte cuts in text structure and validators); branch pruning without records.
- The tool list is still precomputed and injected (the cap was reverted; retrieval by need is the next step).
- Pipeline binaries not rebuilt; every zero-shot wiring change is unverified until a build and a live run.

## 14. Summary by status

- BUILT (host binary covers the file): executor.rs, pipeline/mod.rs, amt.rs, stages.rs, jurisdiction.rs, response.rs, zsei/mod.rs, integrity/mod.rs, mcp_graph.rs, grpc/mod.rs, types/container.rs, lib.rs, Cargo.toml, Cargo.lock (content unexplained).
- REVERTED (binary predates the file mtime): mcp.rs (net zero), orchestrator/mod.rs.
- UNVERIFIED: gate.rs, lane_split.rs, shared validator, all pipeline crates' changes, text analysis, collective consciousness.
- PARSE-CHECKED: all tool servers and the new Python worker.
