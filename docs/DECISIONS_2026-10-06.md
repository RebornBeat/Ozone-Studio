# Decisions log — 2026-10-06

Captured from the CHECKLIST entries of 2026-10-05 and 2026-10-06 and from the decision sections of RUNTIME_MEMORY_PLAN.md (§5), CONTEXT_BUDGET_SERVICE.md (§4) and CONTEXT_OBJECT_MODEL.md (§8–9). Read-only capture: nothing was built, tested, or edited in source.

Status vocabulary: DONE (verified by reading or live test), PARSE-CHECKED (rustfmt parse only, not compiled), PENDING BUILD (in the tree, needs a build to verify), PENDING OPERATOR (needs a decision), REVERTED, OPEN GAP (known, not fixed).

Who decided: OPERATOR = a directive from the operator; CC = Claude Code; ZCode = the ZCode agent; PENDING = nobody has decided.

## A. Standing directives (operator)

| # | Directive | Who | Status | Where / evidence |
|---|---|---|---|---|
| A1 | Never skip a model or an attempt; reorder, don't drop | OPERATOR | DONE as a rule; cooldown-skip design rejected by CC | mod.rs walk; CHECKLIST 2026-10-05 "ZCode gate/fallback review" |
| A2 | No build until the operator says so | OPERATOR | IN FORCE | this log; last build stopped 2026-10-06 |
| A3 | Do not drop features, including LLM zero-shot calls | OPERATOR | DONE: every zero-shot call retained; dead helpers kept | CHECKLIST 2026-10-05 zero-shot entries |
| A4 | No fixed size caps in context paths; budgets follow the model | OPERATOR | DONE for AMT lanes and guards; OPEN GAP for chunk caps and the rest | amt.rs 2256/2275/2290; CONTEXT_REVIEW §8 |
| A5 | Context is objects in the graph; each point knows its needs and traverses for them | OPERATOR | DESIGN only | CONTEXT_OBJECT_MODEL.md |
| A6 | Tools are retrieved where needed, not injected at all times | OPERATOR | DESIGN; injection still active | mod.rs:1620 still calls global_registry_summary |
| A7 | Production host (50051) is not written to by tests; test on a separate instance | CC (operator-accepted) | DONE | CHECKLIST 2026-10-05 graph-native entry |
| A8 | Pipelines and tools stay separate processes/binaries; shared build caches only | CC + ZCode agreed | DONE for the shared build directory | scripts/build-pipelines.sh |

## B. Runtime, gate and concurrency

| # | Decision | Who | Status | Where / evidence |
|---|---|---|---|---|
| B1 | Pipeline calls queue behind a gate; nothing rejected | OPERATOR (never skipping), ZCode built, CC wired | PARSE-CHECKED, PENDING BUILD | gate.rs; executor.rs admit; old reject at executor.rs:112 removed |
| B2 | Gate capacity = config `max_concurrent_pipelines` (default 10) | CC wired; the default value is PENDING | PENDING OPERATOR for the default | config/mod.rs:380 still 10 |
| B3 | Gate size 3 on this 7.6G machine (D1) | CC proposed, ZCode agreed | PENDING OPERATOR | RUNTIME_MEMORY_PLAN §5 D1 |
| B4 | Tier order: User, then Lane, then Loop; FIFO within a tier | OPERATOR (universal order) | PARSE-CHECKED, PENDING BUILD | gate.rs release(); lib.rs Loop tags; amt.rs Lane tags |
| B5 | Loop starvation under sustained User traffic: needs an aging rule or accepted starvation | CC flagged | PENDING OPERATOR | CHECKLIST 2026-10-06 R1 review |
| B6 | Task-manager loops (src/task/mod.rs ~1066, ~1896) tagged Loop | CC flagged | OPEN GAP (default User) | CHECKLIST 2026-10-05 gate-wired entry |
| B7 | Cancelled queued callers do not consume a slot | CC fixed | DONE in source; test added | gate.rs release() skips closed waiters |
| B8 | Remote pipeline dispatch that calls back into the host while holding a slot can deadlock at full capacity | CC flagged | OPEN GAP; fix is to bypass admission for nested or remote dispatch | executor.rs ~250–262 |
| B9 | Keep-warm local model server default ON when a local model is configured (D3) | CC proposed, ZCode agreed | PENDING OPERATOR | RUNTIME_MEMORY_PLAN §5 D3 |
| B10 | GGUF gets a server path (no server path today) | CC and ZCode agreed | OPEN GAP | RUNTIME_MEMORY_PLAN §3.4 |
| B11 | Heavy-worker idle unload 10 minutes (D2) | CC proposed, ZCode agreed | PENDING OPERATOR | RUNTIME_MEMORY_PLAN §5 D2 |
| B12 | One pipeline Cargo workspace with one lockfile (D4) | CC proposed, ZCode agreed | PENDING OPERATOR | RUNTIME_MEMORY_PLAN §5 D4 |
| B13 | Daily dependency upkeep, gated on test coverage and never during an active walk (D5) | CC proposed, ZCode agreed | PENDING OPERATOR | RUNTIME_MEMORY_PLAN §5 D5 |
| B14 | Shared pipeline target dir (`target/pipelines`) | CC | DONE script and executor path; OPEN GAP: 82 binaries of unverified provenance; 41 per-crate target dirs kept until verified | scripts/build-pipelines.sh; executor.rs candidate path |
| B15 | Release profile thin LTO with line tables; `dist` profile keeps fat LTO | CC | DONE in Cargo.toml; PENDING BUILD | Cargo.toml |
| B16 | lld linker | CC | DONE in .cargo/config.toml; PENDING BUILD | .cargo/config.toml |
| B17 | `target/debug` removed (about 7G) | CC | DONE | regenerable; no further deletion decided |
| B18 | Pipeline RAM figures measured | nobody | OPEN GAP: no live measurement | RUNTIME_MEMORY_PLAN §7 |

## C. Model fallback walk

| # | Decision | Who | Status | Where / evidence |
|---|---|---|---|---|
| C1 | Health reorder: models with recent consecutive failures move to the back, still attempted | CC proposed; ZCode built; operator's no-skip rule | DONE in tree; PENDING BUILD | mod.rs `model_health_map()` ~3054 |
| C2 | Cooldown skip of failing models | ZCode proposed; CC rejected (conflicts with A1) | REVERTED to reorder-only design | CHECKLIST 2026-10-05 "fallback review" |
| C3 | Context-fit pre-order: models that cannot hold input plus output defer to the back | ZCode | DONE in walk and text rotation; PENDING BUILD | mod.rs ~3061–3074; text/main.rs ~4081–4099 |
| C4 | C8: keep the first usable response when later candidates fail | ZCode | DONE in tree; PENDING BUILD | mod.rs ~3081–3175 |
| C5 | Per-candidate output budget re-derived from each candidate's window | ZCode | DONE in tree; PENDING BUILD | mod.rs ~2754–2761 |
| C6 | Watchdog budget per attempt times (1 + candidates), finite backstop, untried candidates listed in the error | CC (latency fork), hand-merged by CC | APPLIED in tree; PENDING BUILD | mod.rs watchdog ~2741–2790; planned/started/finished markers |
| C7 | Trade-off: a hung walk can now hold up to the longer budget before failing (freeze guard looser) | CC recorded | PENDING OPERATOR accept or tighten | CHECKLIST 2026-10-06 watchdog entry |
| C8 | Per-attempt HTTP bound 120s unchanged | CC verified | DONE (no change) | prompt/main.rs ~422 |
| C9 | User chain order: openrouter/free, then openrouter/auto, then bitnet-i2_s. Prefer paid OpenRouter for user calls and drop BitNet from the user chain? | CC asked; operator said routing is config (user decides) | PENDING OPERATOR (config only; no code change made) | target/release/config.toml [models.fallback] |
| C10 | Meta chain is BitNet first, free-only OpenRouter second (local-first for background work) | operator directive | DONE (config as directed) | [models.meta_fallback] |
| C11 | Watchdog cut of the walk was a silent skip | CC found, confirmed | DONE via C6 | — |

## D. Zero-shot relation validation

| # | Decision | Who | Status | Where / evidence |
|---|---|---|---|---|
| D1 | Three verification modes: verbatim (`validate`), mapped (`validate_mapped`), structural (`validate_structural`) | CC | DONE; PARSE-CHECKED, PENDING BUILD | assets/pipelines/shared/semantic_relations.rs |
| D2 | Verification level recorded on every accepted relation | CC | DONE | AcceptedRelation.verification |
| D3 | Structural relations stored at confidence 0.5 (convention, not a probability) | CC | PENDING OPERATOR accept the convention | semantic_relations.rs zsei_relation_json_for |
| D4 | Domain labels mapped to ZSEI names; original kept in domain_relation; unmapped labels rejected with raw text | CC | DONE; PENDING OPERATOR for the extra mappings | map_domain_relation() |
| D5 | Extra mappings TemporallyPrecedes → Precedes and PrecedesInSequence → Precedes | CC proposed | PENDING OPERATOR | CHECKLIST 2026-10-05 wiring entry |
| D6 | Unmapped vocabularies (sound 8, proteomics 9, radar 2, thermal 1) stay rejected until decided | CC | PENDING OPERATOR | CHECKLIST 2026-10-05 second-pass entry |
| D7 | Group B switched to validate_mapped (strictly more accepted; verbatim check kept) | CC | DONE; PARSE-CHECKED | six modality call sites |
| D8 | Node-ID endpoints must be in the node list the model saw; relations never build the entity list | CC (after ZCode's circular-check finding) | DONE for groups A2, B, C2; text fix uses grammar nodes of the same parse | text/main.rs; modality call sites |
| D9 | Text legacy per-sentence path accepts nothing until an independent entity list is wired | CC | OPEN GAP | text/main.rs legacy path ~4752 |
| D10 | Update-path calls (BCI, 3D ×3, biology, CAD, depth) still use legacy wrappers | CC (R2) | OPEN GAP | CHECKLIST 2026-10-06 R2 entry |
| D11 | Accepted zero-shot relations stored in graph JSON only, not ZSEI containers | CC | OPEN GAP | CHECKLIST 2026-10-05 wiring entry |
| D12 | Wiring prompts changes to every modality spends LLM calls and changes model behavior | OPERATOR approved expansions; per-modality test not run | PENDING BUILD and a live test with approved budget | — |
| D13 | Hyperspectral literal fixed (compile error E0063) | CC (R2) | DONE; PARSE-CHECKED | hyperspectral main.rs graph literal |

## E. Context handling (chunking, budgets, lanes, caps)

| # | Decision | Who | Status | Where / evidence |
|---|---|---|---|---|
| E1 | Orchestrate-path text chunk size = model_context_limit / 4 (dynamic) | existing code, confirmed | DONE (no change) | mod.rs ~2000 |
| E2 | Fixed 2000-token text chunk default is a fallback only (decision 1 of CONTEXT_BUDGET §4) | CC | PENDING OPERATOR confirm fallback or make window-sized opt-in | text/main.rs ~1405 |
| E3 | AMT lane budget = 3/4 of model_context_limit (was fixed 20 000) | CC | DONE in tree; PARSE-CHECKED, PENDING BUILD; behavior change: larger lanes on large windows, needs a live check | amt.rs ~2256 |
| E4 | Known-branches guard = window_chars / 4 (was 24 000 chars) | CC | DONE in tree; PARSE-CHECKED | amt.rs ~2275–2290 |
| E5 | Methodology guard = window_chars / 12 (was 8 000 chars) | CC | DONE in tree; PARSE-CHECKED | amt.rs ~2290 |
| E6 | Lane call-time split: halve members per candidate until each part fits; merge in member order | CC (applied from fork patch); CC review fix | APPLIED; PARSE-CHECKED, PENDING BUILD | src/orchestrator/lane_split.rs; amt.rs; mod.rs |
| E7 | Lane split is all-or-nothing per candidate; a single member that fits nowhere fails the candidate with its id | CC accepted limits | PENDING OPERATOR accept | lane_split.rs run_candidate |
| E8 | Retry lanes do not split | CC | OPEN GAP | CHECKLIST lane-split entry |
| E9 | Window 0 (unknown) treated as fitting | CC | OPEN GAP (unknown-window gap) | lane_split.rs run_candidate |
| E10 | Lane estimate len/4+1 not calibrated per model | CC | OPEN GAP | amt.rs; lane_split.rs |
| E11 | Chunk-size policy: fixed default or window-sized opt-in per call site (decision 1) | CC | PENDING OPERATOR | CONTEXT_BUDGET §4 |
| E12 | Section priorities for the ContextBudget service (decision 2) | CC proposed | PENDING OPERATOR | CONTEXT_BUDGET §2.7 |
| E13 | Lane budget policy (decision 3) | answered in practice by E6 | PENDING OPERATOR confirm | CONTEXT_BUDGET §4 |
| E14 | Grpc pipeline token_budget flat 100 000 (decision 5) | CC | PENDING OPERATOR | grpc/mod.rs ~1893 |
| E15 | Model override does not carry the per-candidate window to the prompt pipeline (ModelOverrideConfig has no context_length) | CC verified | OPEN GAP; affects llama-cli and BitNet candidates | general/prompt/main.rs ModelOverrideConfig |
| E16 | Prompt pipeline token_budget is set by three callers and read by nothing | CC (modality inventory) | OPEN GAP | lib.rs; grpc; stages.rs ~1917 |
| E17 | Silent caps that cut without a trace: intent and detail chunks (1500 chars); structure loops (8000 / 6000 / 3000 / 1200 bytes); decision review (1500); branch pruning deletes with a count only | CC inventories | OPEN GAP: trim records required | CONTEXT_REVIEW §8; capture-text.md; capture-amt.md |
| E18 | Consciousness gate render 1200 chars; blueprint and simulation renders | CC inventory | OPEN GAP | stages.rs ~1400 |
| E19 | Step first attempt not fit-checked; bypasses metered path | CC inventory | OPEN GAP | stages.rs 2152–2212 |
| E20 | Fixed max_tokens on prompt builders and AMT calls (blueprint 1000, simulation 800, decomposition 200, compliance 150; AMT 150–900) | CC inventory | OPEN GAP | CHECKLIST 2026-10-06 capture entries |
| E21 | Stage 10 memory and relationship write results discarded | CC inventory | OPEN GAP | stages.rs (`let _ =`) |
| E22 | Silent `context_truncated` flag: reports truncation although the prompt is sent whole | CC inventory | OPEN GAP | general/prompt/main.rs ~193 |

## F. Byte-slice safety

| # | Decision | Who | Status | Where / evidence |
|---|---|---|---|---|
| F1 | No raw byte-index slice on model or user text; use a char-boundary-safe prefix | CC | DONE in the host (13 sites) and six pipeline crates (13 sites); PARSE-CHECKED, PENDING BUILD | orchestrator prefix_at_char_boundary; pipeline prefix_chars_safe |
| F2 | Text pipeline lines 4337 and 6476 | CC | DONE (part of F1) | text/main.rs |

## G. Tool summary and context objects

| # | Decision | Who | Status | Where / evidence |
|---|---|---|---|---|
| G1 | Tool summary was computed and then discarded (state set to None) | CC found, ZCode concurred | FIXED then REVERTED: the fix put all 121–134 tools into every prompt, which the operator rejected | mod.rs ~1618–1620; mcp.rs:89 |
| G2 | Cap the tool summary to top N by relevance | CC proposed; operator rejected cap | REVERTED; the cap was removed from mcp.rs and mod.rs | CHECKLIST 2026-10-06 "cap REVERTED" |
| G3 | Retrieval of tools by need through a ContextNeed query, with the injected list removed in step with it | operator direction | PENDING BUILD of the design; OPEN GAP: the list is still injected | CONTEXT_OBJECT_MODEL §8 step 2 |
| G4 | Context as graph objects: ContextNeed, ContextRef, aggregates, per-call ContextRecord | operator direction | DESIGN ONLY, nothing built | CONTEXT_OBJECT_MODEL.md |
| G5 | Order of migration: ContextRecord first, then tool retrieval, ContextRef, aggregates, trim records, per-model windows | CC | PENDING OPERATOR confirm | CONTEXT_OBJECT_MODEL §8 |
| G6 | Open decisions: relevance ranking, traversal budgets per stage, persist aggregates always or on request, tool retrieval paired with removal | CC | PENDING OPERATOR | CONTEXT_OBJECT_MODEL §9 |
| G7 | ContextBudget service (one estimator, one assembler, trims recorded) | CC design; ZCode plan | DESIGN ONLY; superseded in part by G4 | CONTEXT_BUDGET_SERVICE.md |
| G8 | Five or more token estimators disagree | CC inventory | OPEN GAP; one estimator to be chosen | CONTEXT_BUDGET §1a |

## H. Graph-native MCP persistence and write audit

| # | Decision | Who | Status | Where / evidence |
|---|---|---|---|---|
| H1 | MCP output carrying a `graph` block is persisted as real ZSEI containers (parent containment, typed edges) | CC, operator-directed | DONE; VERIFIED on a second host instance | src/mcp_graph.rs; grpc/mod.rs mcp_call hook |
| H2 | Spatial relation types (Above, Below, NearTo, InFrontOf, Overlaps) and DiscoveryMethod::ToolOutput | CC | DONE; PARSE-CHECKED | types/container.rs |
| H3 | Relation names ImportsFrom and CallsTo accepted by the host (ZCode's type system had them; the fork had said otherwise) | CC corrected the fork | PARSE-CHECKED | mcp_graph.rs relation_from_name |
| H4 | Validation before any write (keys, parents, cycles, edges, relation names) | CC | DONE; VERIFIED (refused graph writes nothing) | mcp_graph.rs |
| H5 | Tools emitting graph blocks: scene_graph, chem_analyze, dna_find_gene, geo_features_near verified; pose, segmentation, shapes, protein_function, mesh bodies, code graph built and partly tested | CC | PARTLY VERIFIED; the rest PENDING live check | CHECKLIST 2026-10-05 and 2026-10-06 entries |
| H6 | Subtree cascade delete (deleting a parent orphans children) | CC requested from ZCode | OPEN GAP; ZCode's file | zsei/mod.rs |
| H7 | Write audit for ZSEI write variants: model_calls/zsei_writes.jsonl, refusals included (refused:true) | CC | PARSE-CHECKED, PENDING BUILD and live check | grpc/mod.rs query_zsei ~1091–1170 |
| H8 | Refusal switch OZONE_ZSEI_REQUIRE_SESSION=1, default OFF | CC; default OFF because pipelines and UI write without tokens | PENDING OPERATOR: when to turn it on | grpc/mod.rs ~1156 |
| H9 | ZSEI write variant list must be maintained by hand | CC (R3) | OPEN GAP for any new mutating variant | grpc/mod.rs ZSEI_WRITE_VARIANTS |
| H10 | Rollback, LinkFile, LinkURL, LinkPackage, UnlinkFile are write variants with no handler; they return "Unsupported query type" | CC (R3) | OPEN GAP: implement or document | zsei process(); GRAPH_AUTHORING_GUIDE §4 corrected |
| H11 | Tool-call capture stores a 300-char input preview and no output | CC inventory | OPEN GAP: output content pointer | grpc/mod.rs tool-call row |
| H12 | Android: shell requires confirm:true; apk path confined and symlink-checked; only devices and device_info registered | CC | DONE; shell and install deliberately NOT registered | tools/android-mcp/server.mjs |

## I. Freeze and locks (ZCode territory, reviewed by CC)

| # | Decision | Who | Status | Where / evidence |
|---|---|---|---|---|
| I1 | get_container: release the storage guard before the cache read; keep it through the cache insert (no stale cache) | ZCode, corrected by CC | DONE; PARSE-CHECKED | zsei/mod.rs ~262–292 |
| I2 | Leading hypothesis: a lock holder parked on an await, not a lost wakeup | CC accepted by ZCode | PENDING: a task dump decides | CHECKLIST 2026-10-05 "gdb verdict" |
| I3 | Diagnostic: Handle::dump under tokio_unstable, or timed tracing spans around lock acquisition (cheaper than a full tokio-console build) | CC | PENDING OPERATOR (a rebuild with tokio_unstable is needed) | CHECKLIST 2026-10-06 R3 |
| I4 | query() holds write guards across process().await for reads too | CC (R3) | OPEN GAP, largest contention source | zsei/mod.rs ~164–166 |
| I5 | traverse holds the read guard across the traversal; fix is snapshot-then-traverse | CC, accepted by ZCode | OPEN GAP | zsei/mod.rs ~321–326 |
| I6 | store_local synchronous write under the storage write lock | CC (verified) | OPEN GAP; needs async storage writes | zsei/storage.rs ~593, ~391 |
| I7 | Integrity snapshot: version reserved under lock, file written outside it, rollback on failure, stale removal outside the lock | CC | DONE; PARSE-CHECKED | integrity/mod.rs ~168–216 |
| I8 | Pre-write snapshot holds the integrity read guard across a disk write | CC (R3 suspect) | OPEN GAP (low) | zsei/mod.rs ~106–111 |
| I9 | Gate and executor: stale-cache race and cancelled-waiter leak fixed | CC | DONE in source | gate.rs; zsei/mod.rs |

## J. Build and release state

| # | Decision | Who | Status | Where / evidence |
|---|---|---|---|---|
| J1 | Binary built by ZCode at 07:08; the tree has CC edits since | ZCode built; CC edits after | PENDING BUILD of the merged tree | target/release/ozone-studio; CHECKLIST 2026-10-06 |
| J2 | Final release build started, then stopped by PID before completion | CC (mistake, recorded) | STOPPED | CHECKLIST 2026-10-06 "build STOPPED" |
| J3 | Compile questions to resolve on the next build: task-local in spawn, Send bounds, deref coercions in render_branch_lane_prompt | CC | PENDING BUILD | CHECKLIST lane-split entry |
| J4 | Production host is up (health green, 134 tools registered); no writes made | operator reported up; CC checked read-only | DONE (read-only checks) | — |
| J5 | Verification walk on the production host (LLM calls and writes to ZCode's store) | operator choice pending | PENDING OPERATOR (A, B or C) | CHECKLIST 2026-10-06 |
| J6 | Write-audit and refusal test on a separate instance | CC option C | PENDING OPERATOR go | — |

## K. Open gaps not yet decided

| # | Gap | Status |
|---|---|---|
| K1 | Stage-4b/5 freeze recurrence, if it happens | OPEN until a task dump runs |
| K2 | Object_store_path retry loop for AMT re-expansion (ZCode: mark unexpansible candidates handled, loud) | OPEN GAP |
| K3 | Emotional-system derivation (near-default valence after successes) | ZCode: design question, PENDING OPERATOR |
| K4 | OCR (tesseract binary, needs sudo) and faces (no Haar cascades) | OPEN GAP (reported in detection_backend_notes) |
| K5 | Connectome TensorFlow (1.9G) unused; visual CUDA wheels (about 4.5G) unused on a no-GPU box | PENDING OPERATOR (prune) |
| K6 | CAD graph container is a stub (placeholder path, zeroed hash) | OPEN GAP |
| K7 | STEP assembly hierarchy untested (no real STEP file) | OPEN GAP |
| K8 | Proteomics name fallback ("insulin human" resolves to a Conus peptide) | OPEN GAP |
| K9 | DNA translate uses the standard code; Mycoplasma TGA = Trp | OPEN GAP |
| K10 | Root Cargo.lock has 62 changed lines not made by CC | OPEN GAP: check before the next build |
| K11 | loadDetectionRegistry returns an empty list on any parse error | OPEN GAP |
| K12 | ZCode's 67-of-68 registry enrichment claim | UNVERIFIED |

## L. Summary counts

- DONE (verified by reading or live test): about 35 rows.
- PARSE-CHECKED or PENDING BUILD: about 30 rows (every source change of 2026-10-05 and 2026-10-06 is unbuilt).
- PENDING OPERATOR: about 25 rows, led by gate size, idle unload, local server default, workspace, upkeep, chunk policy, section priorities, lane budget confirm, grpc default, user-chain config, watchdog trade-off, refusal switch default, verification walk choice.
- OPEN GAP: about 40 rows, led by the fixed chunk and output caps, trim records, the tool injection removal, query() locks, store_local, cascade delete, and the Rollback handlers.
- REVERTED: the tool-summary cap and the cooldown-skip design.
