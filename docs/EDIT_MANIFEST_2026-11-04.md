# Edit manifest — 2026-11-03 → 2026-11-04 (E2E + fan-out + meta ripple + providers + marketplace)

Status of everything below: **written; the edits marked ⚠ UNCOMPILED / UNTESTED were made under the operator instruction "no build, no test"**.
Items marked ✅ were built and tested earlier in the same wave (see `CHECKLIST.md` + `docs/evidence/test-runs/`). Nothing here is deployed to EU except
where stated. `git status` is the authoritative file list (132 entries at the time of writing); this file explains the *why*.

## A. Host (Rust) — behavior changes

| File | Edit | State |
|---|---|---|
| `src/pipeline/executor.rs` | inputs > 100 KB staged in a temp file, child gets `--input @path` (E2BIG fix) | ✅ built, deployed to EU |
| `assets/pipelines/**/main.rs` (≈72 files) + `general/prompt` | every pipeline resolves `--input @path` (`ozone_input_arg`); prompt: `-f <file>` for oversize local prompts (BitNet/llama E2BIG #2), request-proportional HTTP timeout (45 s + 1 s/10 tokens, cap 600 s) instead of a flat 120 s | ✅ built live locally; EU has `@path` (first wave), NOT the timeout/`-f` |
| `assets/pipelines/modalities/code/main.rs` | indentation-aware block end, innermost-caller attribution, declaration lines not self-calls, **cross-file calls** (imported callee → external Function node + Calls edge, `FunctionCall.module`) | ✅ built + tested locally (3 unit tests) |
| `src/orchestrator/graph_fanout.rs` (+ `graph_fanout_edge_names.rs`) | per-modality fan-out: edge classes over all 27 modalities (562 names), entries, flow sequence, dated digest | ✅ built + tested (5 unit tests) |
| `src/orchestrator/span_out.rs` | opens modality graph FILES, `fanout{}` evidence per item; `relation_weight` → classifier | ✅ |
| `src/grpc/mod.rs` | `POST /context/span_out`, `GET /ripple/meta/status`, `GET /marketplace/overview` (providers, wanted, health, proposals, methodologies+rules, blueprints), PROTECTED `/context/ /ripple/ /marketplace/`; `/mcp/call` stamps `called_from` | route code ⚠ UNCOMPILED (marketplace/overview rewritten for providers); span_out/ripple status ✅ |
| `src/orchestrator/ripple_class.rs` | meta-ripple: `classify`, `LoopGate` (wake/heartbeat/min-gap/coalescing), own-pass echo guard, dispatcher, `emit_finding`/`emit_methodology_gap`/`emit_capability_gap`, `status_json` | ✅ built + 5 tests; `CapabilityGap` additions ⚠ UNCOMPILED |
| `src/orchestrator/{i_loop,meta_loop}.rs`, `src/consciousness/assistant.rs` | event-driven (meta ripple + startup catch-up + heartbeat + min gap); assistant also next-deadline sleep; prompts carry CURRENT TIME; I-Loop signals DATED (absolute timestamp in the change-detection fingerprint) | ✅ built; stale doc comments replaced ⚠ |
| `src/orchestrator/run_spans.rs` | `now_line()`, `when()` + test | ✅ |
| `src/orchestrator/amt.rs`, `actors.rs` | gap/finding ripples; amt test state fields | ✅/⚠ |
| `src/orchestrator/stages.rs` | `unexecuted_tool_markup` (no fake success); **provider STEP TYPE**: prompt "RULE → REQUIRED PROVIDERS" + step types, `reconcile_tool_bindings` (validate, insert provider steps before carriers, renumber + remap deps, capture gaps), `execute_provider_step` + dispatch, provider steps exempt from pipeline coercion/compliance, simulation: host-computed provider checks + provider-aware step lines | markup check ✅; everything provider-related ⚠ UNCOMPILED / UNTESTED |
| `src/orchestrator/mod.rs` | `BlueprintStep { rule_refs, step_type, provider, tool, args, optional }` + `is_provider_step`; `OrchestrationState.tool_manifest/missing_capabilities`; **Stage 3 rewritten: bindings first, search fallback, the "first 8 registry entries" floor REMOVED**; stale `applicable_tools` doc replaced | ⚠ UNCOMPILED |
| `src/orchestrator/tool_binding.rs` | provider bindings (`gather_bindings`, `evaluate`, `providers_of`, `manifest_block`), **capability-gap capture** (durable + ripple + methodology gap, `close_satisfied_gaps`), provider health ledger, binding proposals, `provider_of_tool` + 2 tests | ⚠ UNCOMPILED / UNTESTED |
| `src/orchestrator/tool_routing.rs` | **no fixed text/size/count caps** (200-char query, 2,000/4,000-char results, ≤2 tools removed); search fallback skips bound providers; relevance-group selection; `call_provider_for_step` + `choose_provider_tools` (tool chosen at call time) + health + proposals; caller fits the block to the window | ⚠ UNCOMPILED / UNTESTED (1 new test) |
| `src/orchestrator/run_checkpoint.rs` | persists `tool_manifest` / `missing_capabilities` (serde-default) | ⚠ |
| `src/mcp.rs` | post-call hook (`install_post_call_hook`), **jurisdiction call gate now inspects the WHOLE text (was first 4,000 chars)** | ⚠ UNCOMPILED |
| `src/mcp_graph.rs` | `CalledFrom` provenance (`call:*` keywords) + `persist_from_output_called` | ⚠ |
| `src/mcp_supervisor.rs` | `provider_for_endpoint` (provider id from the port table) | ⚠ |
| `src/lib.rs` | spawns the meta-ripple dispatcher; installs the post-call hook | ⚠ |
| `src/types/methodology.rs` | `ToolRequirement` + `DecisionRule.requires[]` (legacy-safe) + test | ⚠ |
| `assets/pipelines/general/methodology_create/main.rs` | `DecisionRuleInput.requires`, stored on the canonical rule; Propose message states what is missing | ⚠ |
| `src/orchestrator/meta_loop.rs` (prompt) | drafting prompt asks for per-rule `requires`; capability-gap keywords | ⚠ |

| (doctrine audit batch, 2026-11-04 — ALL ⚠ UNCOMPILED) | `orchestrator/mod.rs` (all-keyword methodology search; whole-file analysis, NO size ceiling; `parse_json_object` deleted), `amt.rs` (full rule text; enrichment counts removed), `stages.rs` (keyword/blueprint/step floors removed; awaiting subject whole), `graphs.rs`, `awaiting.rs`, `amt_candidates.rs`, `actors.rs`, `context_needs.rs`, `zsei/query.rs` (every seed/neighbour), `zsei/hooks.rs` (**no confidence**: prompts, threshold, field, importance removed; every node summarised whole), `consciousness/store.rs` (legacy migrations skip instead of inventing), `network/mod.rs` (over-long dialogue turn CHUNKED into numbered parts `part/parts`, sent whole; one budget unit per logical turn), `mcp.rs` (gate sees whole text), `span_out.rs`/`graph_fanout.rs` (no confidence term), `meta_loop.rs` (no heuristic confidence), `assets/pipelines/general/prompt` (Anthropic ceiling, window-derived local defaults, prompt-size timeout), `context_aggregation` (token budget instead of 4/8/1600/12 counts), 16 modality pipelines (`infer_semantic_relationships_validated` → overlapping windows over EVERY node + `shared/semantic_relations::windows`), 25 modality/text keyword `truncate` removals, `browser_navigation` (whole page), `uidocument` (per-node text/names whole), 14 MCP servers (512 MiB transport guard, env-tunable; 413 JSON beyond), `browser-mcp` (typed text whole, snapshot `truncated`), `terminal-mcp` (`*_truncated`), `git-mcp` (all diff lines), `ui UnifiedSearch` (every id/hit) | ⚠ |


| (fabricated-output batch, 2026-11-05 — ⚠ UNCOMPILED; every file parses under rustfmt) | 16 modality `Analyze` arms + 14 `ExportProduct` arms → honest failures; **DNA**: real ORF scan / motif search / translation, other canned actions → errors (+ tests); BCI `CalibrateDecoder`, proteomics alignments, depth Register/ReconstructMesh/FuseFrames/ComputeNormals, geospatial `TransformCoordinates`, 3D `RunSimulation`/`BakePhysics`, biology `BuildRegulatoryNetwork`, thermal `FuseWithVisible`, network `DiscoverNetwork`/`AnalyzeCapture`, kinematics recorded-source gait/biomechanics + imports/export, EEG `extract_bands` → honest failures; `general/sync` (no fake peer id; host writes `<data_dir>/p2p/peer_id`), `src/integrity` (derived `data_loss_risk`), `collective_consciousness` (`extract_pattern`), `zsei/hooks.rs` (`process_batch` Err) | ⚠ |

## B. Tools / MCP
`tools/threed-mcp/server.mjs` (+ `clock_blender.py`, `clock_cad.py`): `clock_generate`, `cad_clock_generate`; rat model documented as a standalone 3D asset. `tools/register-browser-tools.mjs`: `threed` family.
`~/.venvs/cad` (cadquery) installed locally (not in repo).

## C. UI
`ui/src/components/MarketplacePanel.tsx` (Catalog / Installed / Relations / Contribute & Review; providers; wanted; health; proposals), `ui/src/ozoneClient.ts` types + `fetchMarketplaceOverview`, `ui/src/pipeline-ui.tsx` tab 1012. ✅ typechecked + built once; the provider/wanted rewrite ⚠ type-checked only (`tsc`, no bundle).

## D. E2E harness (`tools/e2e/`)
`fixtures.mjs`, `mcp-contract.mjs` ✅ 43/0/2, `workspaces.mjs` ✅ 5/5, `modalities.mjs` ✅ 46/46, `jurisdiction.mjs` ✅ 9/9, `fanout.mjs` ✅ 13/13, `fanout-coverage.mjs` ✅, `pause-resume.mjs` (manual half ✅), `clock.mjs` (Ozone-driven; walk #2 paused at stage 2 before the timeout fix), **written, not run:** `meta-ripple.mjs`, `marketplace.mjs` (updated for providers), `provider-step.mjs`.

## E. Docs
New: `PER_MODALITY_FANOUT_GUIDE`, `RIPPLE_CATEGORIES_AND_META_RIPPLE`, `UI_ANIMATION_DESIGN`, `METHODOLOGY_BLUEPRINT_TOOLS_REVIEW` (+ operator direction verbatim, App. A/B), `MCP_TOOLS_MODALITY_ARCHITECTURE_REVIEW`, this manifest.
Updated (stale content replaced, dated): `SYSTEM_FLOW_AND_GRAPH_STATE_2026-11-02` (stage 3, tools, loops + addendum), `CALL_ROUTE_REGISTRY` (R7 + §6 R13–R18), `ACTING_FACTORS_REGISTRY`, `ACTING_LOOP_GUIDE`, `PERSONAL_ASSISTANT_GUIDE`, `AMT_EXPANSION`, `EFFICIENCY_GUIDE`, `P2P_AND_SHARED_MCP_COMMUNICATION_GUIDE` (stale §7 note), `MCP_TESTING_AND_COORDINATION_GUIDE`, `TOOLS_REGISTRY`, `STATIC_LOGIC_REGISTRY`, `MASTER_TEST_GUIDE` (§26g–§26i), `CHECKLIST.md`.
Not yet swept (older, still contain pre-2026-11-04 statements): `FINAL_REVIEW_2026-10-31`, `REVIEW_E2E_STATE_2026-11-01`, `STATE_CAPTURE`, `ZERO_SHOT_EXPANSION_GUIDE`, `MODALITY_TOOL_AFFINITY_GUIDE`, `CONTEXT_FLOW_*`, `README.md` status block.

---

# FULL LIST OF EVERYTHING LEFT TO WORK ON (nothing omitted)

## 1. Blocked on the build/test gate (the operator said "no build, no test" — all of this is waiting)
1. **First compile of all ⚠ edits** — expect fixes (closure typing in `lib.rs` hook, borrow shapes in `reconcile_tool_bindings`, struct literals of `OrchestrationState`/`BlueprintStep` elsewhere, `serde_json::Value` indexing).
2. **Unit tests to run:** `tool_binding` (2), `tool_routing` (3 incl. `choose_provider_tools`), `types::methodology::requires_tests`, `ripple_class` (5), `graph_fanout` (5), `code::callgraph_tests` (3), `run_spans::time_tests`, `stages::tool_markup_tests`.
3. **E2E to run:** `meta-ripple.mjs` (host with `OZONE_META_MIN_GAP_SECS=15 OZONE_ASSISTANT_MIN_GAP_SECS=15`), `marketplace.mjs`, `provider-step.mjs`, `fanout*.mjs`, `jurisdiction.mjs`, `mcp-contract.mjs`, then the **Ozone-driven clock project** (`clock.mjs`; host must run with `OZONE_TOOL_ROUTING=1` only for the search fallback — provider steps do not need it) with Blender and CAD one at a time; Roblox is deferred.
4. **Local host restart** on the new binary; resume paused tasks (994 completed without doing the work; 1000 paused at stage 2) after the timeout fix.
5. **EU:** diff EU↔local, sync everything since the 2026-11-03 23:19 deploy (code-pipeline fixes, prompt timeout + `-f`, fan-out, meta ripple, marketplace, provider steps, mcp.rs gate fix), rebuild on EU (`PATH=$HOME/.cargo/bin`), swap with backups, verify.

## 2. Edits not yet made (design settled, code not written)
6. **Call-time argument derivation**: provider steps send `{"query": step text}` unless the step has `args`; a tool like `clock_generate` needs `{time, radius_m}`. Add registry fields `input_schema`, `timeout_ms`, `provider`, `modality`, `capability_ids` (optional) and derive args from the step context + schema (host injects workspace/project scope).
7. **Unify the two call paths** (partly DONE 2026-11-04, ⚠ uncompiled): the post-call hook now persists the graph block (with `called_from`) AND writes the S13 `tool_calls.jsonl` row for every `call_global` call; still missing: the context-mirror coordination ripple that `/mcp/call` writes.
8. **Modality-first affinity** (doctrine "modality first, tools second"): `modality:` tags / affinity table; decide the 3D owner (3 overlapping providers: bridges Blender, `threed-mcp`, `mesh-mcp`); ~10 modalities have no provider (audio, video, math, thermal, …).
9. **"Learn once, bind forever" consumer**: proposals are recorded (`tool_binding_proposals.json`) but nothing promotes them to declared `requires[]`; the meta-loop pass is not written.
10. **Annotate real methodologies — reviewed 2026-11-04, deliberately NONE annotated:** the 44 runtime / 15 created / 58 asset methodologies are policy/process rules (api design, TDD, verification discipline, MCP-done criteria …); none of their rules genuinely requires a provider, so adding `requires` would FABRICATE a need. Real bindings come from methodologies written for tool-using work: authored by the operator, or drafted by the meta-loop from a captured capability gap (its prompt now asks for per-rule `requires`).
11. **Reused blueprints**: `reconcile_tool_bindings` adapts a reused plan in memory only (not persisted back).
12. **Provider-step tool choice** for multi-tool providers with no text match fails honestly; consider a model-assisted pick.
13. **Marketplace protocol (build-plan items 8–11):** signed manifest-only `Catalog` exchange between paired peers, quarantine → auto-review → human approval → install (autostart off), `Propose` implementation + `created_by`/`consensus_status`, the Review queue UI wired to real offers.
14. **UI animations** (docs/UI_ANIMATION_DESIGN_2026-11-03.md, 11 items): Fan-out Lens on `GraphWorld3D`, blueprint-finale convergence, AMT growth, gate/pause states, `prefers-reduced-motion` guard; needs new wire events (`span_out`, `run_state`, `trace_id` on frames, unemitted markers `[6.tools] [10.step] [7.0] [8.x] [13.0]`, `pipeline_location` forwarding).
15. **Per-step AMT expansion** (only the background loop deepens the AMT today) — operator decision pending.
16. **Doctrine audit fix list** (`docs/DOCTRINE_VIOLATIONS_AUDIT_2026-11-04.md`, 37 items): 34 APPLIED (uncompiled) — see its §6 status table. **Still open:** #34 (text/math `max_tokens` literals are hints; documented, not removed), #37 (UI list display slices — class C; add "show more"), the confidence retirement (item 23), and the audit's "not verified" list (Stage-0 normalisation before gating, every window-budget caller, Python workers, `bridges` TypeScript, UI display slices).
17. **Sparse-capture modalities** (zero typed edges): audio, control, electromagnetic, haptic, hyperspectral, network, proteomics, radar, video (+54/176 text graphs).
18. **Capture gaps:** cross-file calls for **Rust/JS** imports (only Python-style `from x import y` item lists are resolved so far); methodology-gap/capability-gap drafting loop is untested end to end.
19. **Stage 2**: the text pipeline makes many tiny prompt calls for a one-sentence prompt (the timeout fix bounds waiting, not the call count).
20. **Docs not yet swept** (list in §E above) + README status block + MASTER_TEST_GUIDE rows for the new E2Es.


23. **Retire confidence from the data model (operator doctrine 2026-11-04: "confidence is false confidence — it is yes or no, that's why we have the loop").** Swept already: `zsei/hooks.rs` (prompts no longer ask for confidence/importance; the `≥ 0.7` threshold and `confidence` field removed; affirmed relations weigh 1.0), span-out and fan-out (no confidence term), meta-loop draft prompt (no heuristic confidence), math-parse absence = 0.0, legacy gate/emotion/perception migrations skip instead of inventing values. **Still to do:** `Relation.confidence` (src/types/container.rs + every constructor; D3 "0.5 convention" — decide: replace with a `verified`/strength-N count), `GateDecisionRecord.confidence` + the decision-gate / decision_review `confidence` fields and prompts, `HeuristicInput.confidence`, modality `*_validated` relation paths that still ask for / store a score, `consciousness/*` `confidence` fields, `ContextRef` ranking by confidence, UI displays of "confidence". Replace each with the binary verdict + strength-N loop result (or drop).

28. **Fabricated-output audit leftovers** (docs/FABRICATED_OUTPUT_AUDIT_2026-11-04.md §5): `HeadlessProcess` sub-ops that still push claiming version notes (BCI, biology, depth, kinematics URDF/MJCF, EM, network, sound, thermal, geospatial, 3D); CAD unwritten export/BOM paths; refinement-daemon TODO bodies; ≈600 literal edge weights / `hotness_score` literals (pair with confidence retirement); `UnmixPixel` / ML-traversal renames; the 2 `ExportProduct` arms not located; modalities the audit did not verify (`control` beyond Analyze, `chemistry`, non-Analyze actions of radar/sonar/sound/thermal/hyperspectral/haptic/IMU, EM beyond headless). **Regression risk to check at first compile + run:** the E2E `modalities` suite links with `analyze:false` (unaffected), but any flow that calls `analyze:true` on those 16 modalities now gets an honest analysis failure (file_ref `analyzed:false`) instead of a root-only graph.

29. **Modality status review** — `docs/MODALITY_STATUS_2026-11-05.md` lists every modality/action with its state (unmarked / fails honestly / stub-simplified), unit-test counts and open items; the ~800 literal edge-weight/hotness/confidence values (521 + 272 + 15) await the operator's decision (not removed); 16 modalities still have no unit tests (add real-input tests alongside any implementation).

## 3. Data / hygiene (operator decisions — NOT touched)
24. 30 `stress-test item` fixtures (34–50 days old) drive 25+ assistant "resolve overdue" follow-ups; 21 `interrupted` tasks (9–52 days old).
25. 30 orphan graph files without an owning container (25 code); 63 old malformed `model_ledger.jsonl` rows (pre-fix, skipped with a warning).
26. `Ozone-Studio (Copy)` snapshot and `.claude/worktrees` (operator decisions; `tools/*/data` must never be deleted).

## 4. Operator actions
27. EU SSH hardening command (76,388 failed logins / 5,762 bans so far); rotate the OpenRouter key that appeared in chat; decide `auto_broadcast_sync`; answer the open decision lists in the review docs (rule-level vs provider-level binding is now decided: PROVIDERS per rule; still open: who approves new capability ids, marketplace shares providers vs tools, paired-only vs open discovery).

## 5. TEST BACKLOG from the full E2E plan (MASTER_TEST_GUIDE) — none of these has been run yet
- **Done (real host):** workspaces 5/5, modalities 46/46 (file handling), MCP contract 43/0/2, jurisdiction 9/9 (+J8 written), fan-out 13/13, coverage, pause/resume (manual: pause at step 0 + step-level resume proven; scripted PR1 timed out pre-fix).
- **Not run:** O-series orchestration (jurisdiction Block/Warn/RequireConfirmation inside a real walk, awaiting/gate resolve → resume, empty/confetti handling); chat persistence **T-CHAT-1..8**; span-out provenance in step context; Order/tasks/assistant E2E; ripple/living-graph E2E (file-beacon edit → graph update → AMT re-expansion); P2P local↔EU bidirectional (pairing is live; sync policy undecided); **UI E2E** (served at :4173; no browser tool in this session — use browser-mcp/farm or record SKIP) incl. the new Marketplace tab; the **Ozone-driven clock** (Blender then CAD, one at a time — Roblox deferred); jurisdiction J8; `meta-ripple`, `marketplace`, `provider-step`.
- **Regression to re-run after the first compile:** `cargo test --lib` (all), pipelines build (`scripts/build-pipelines.sh` — 59 binaries), `tsc` + `vite build`.

## Addendum 2026-11-06 — see docs/REVIEW_WINDOWING_2026-11-06.md §3 (complete edit ledger since 2026-11-05: routing filter, methodology sufficiency + Update fix, yes/no instruction fix, [optimization] config, persistent host log, 29 data-path cap removals) and docs/REVIEW_ALIGNMENT_2026-11-05.md (open items). Windowing findings W1–W11 are captured, not edited.
