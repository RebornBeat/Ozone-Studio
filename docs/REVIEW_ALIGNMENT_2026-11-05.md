# Alignment review — docs, registries, stores, tests vs. what we learnt (2026-11-05)

**Status: CAPTURE ONLY.** Nothing below has been edited yet (operator: "review all first before editing … only capture all, no omission"). Each item: evidence → what is out of line → proposed action. IDs: A docs · B registries · C stores · D tests · E model routing · F methodology/zero-shot expansion · G build/ops · H decisions needed.

Insights this review is aligned against: (1) `openrouter/free` is a router that ROTATES members — the requested id says nothing about who answered; (2) a request must always be reviewed for methodology sufficiency (new / modify), like missing capabilities; (3) provider = step type; methodologies list providers per rule; (4) no caps / chunk to the reading model's window; (5) fabricated output fails honestly, original code kept; (6) a pipeline that never compiled can sit in the tree unnoticed; (7) tests that pass the wrong input shape fail for the wrong reason; (8) host/pipelines/tests need a documented, repeatable launch.

## H. Decisions needed from the operator (nothing below is changed until answered)
- **H1 — CONFIDENCE: RESOLVED (operator, 2026-11-05, final).** State = AS WE HAD IT: yes/no, no confidence. The strength-N yes/no version is in place in zsei/hooks (affirmed relationship = weight 1.0, no `confidence >= 0.7` filter, no `importance`), span_out (class weight only), decision_review (confidence always None), graphs (absence = 0.0), text/math prompts (no confidence requested), UI EdgeProvenance ("Relation: present") and NodeDetailPanel (**Verified: yes/no** displayed). The 521 weight / 272 hotness / 15 confidence literals in modality code are left untouched ("leave as is"). CONTRACTS §5.1/§5a and the memory doctrine are correct. (History: stripped → restored by my misreading → stripped → restored by a second misreading → stripped again; this is the final state.)
- **H2 — api key in `config.toml`.** The file is git-ignored (OK) but holds a plaintext OpenRouter key in 2 files (`config.toml`, `target/release/config.toml`) and the key was printed to this session's terminal. Rotation was already on the operator list — still open.
- **H4 — `auth` / `sync` and the other registry entries with no binary: NO ACTION (operator, 2026-11-05):** "we are not covering or fixing broken pipelines — we work with the ones we have right now, leave others alone." Recorded only; auth/sync keep the two compile fixes already made and nothing else is touched. B2/B2a stay as informational.
- **H3 — fabricated-output arms.** Original bodies are kept below an early honest-failure return (DNA annotate/predict restored that way). Keep, or delete the unreachable bodies later? (They are dead code today.)

## A. Docs
- **A1 (void)** — confidence docs-vs-code: resolved, see H1 (yes/no version in place; docs correct).
- **A2 Stale "first 8 registry entries" floor** still described as current in `METHODOLOGY_BLUEPRINT_TOOLS_REVIEW` (lines 38, 294 — describes the OLD behaviour in review tables; needs a "superseded" tag), `CALL_ROUTE_REGISTRY` (marked superseded at line 123 — OK), `SYSTEM_FLOW_AND_GRAPH_STATE` (line 41 describes it, line 148 says gone — internally inconsistent), `EDIT_MANIFEST_2026-11-04`, `MCP_TOOLS_MODALITY_ARCHITECTURE_REVIEW`.
- **A3 Durable stores undocumented.** `capability_gaps.json`, `tool_health.json`, `tool_binding_proposals.json`, `methodology_gaps.json`, NEW `methodology_reviews.json` appear in no store/contract doc (grep over STATE_CAPTURE, CONTRACTS, SYSTEM_FLOW: no hit). A store registry (path, writer, readers, schema, retention, durable-vs-cache) does not exist.
- **A4 Four overlapping "registry" docs with no index:** ZERO_SHOT_CALL_REGISTRY (66 KB), CALL_ROUTE_REGISTRY, STATIC_LOGIC_REGISTRY, CONTEXT_REGISTRY, plus TOOLS/ACTING_FACTORS/GRAPH_RELATIONSHIP/DESIGNED_NOT_BUILT/BUILDER registries. No document says which is authoritative for what, and no CI check ties any of them to code.
- **A5 Old/dead docs mixed with live ones:** 100 files in `docs/`; ~35 pre-2026-10-20 (HANDOFF_ZCODE, REVIEW_*_2026-10-06, DECISIONS_2026-10-06, OPEN_WORK_2026-10-06, CONTEXT_*…) and `technical_documentation.md` (680 KB, Feb) are not marked superseded; `docs/guides`, `docs/developer`, `docs/philosophy` subfolders unreviewed.
- **A6 Hand-off docs vs. reality:** VOICE_BOX_GUIDE says V2–V5 unbuilt while a `voice` pipeline now builds and is on the test list — status unverified; needs an actual `voice` E2E before the guide is edited.
- **A7 README System Status** last updated 2026-11-01; does not mention provider steps, ripple classes, meta ripple, marketplace, sufficiency review, served-member filter.
- **A8 CHECKLIST.md** is append-only (rule) and now >7,000 lines; no index/TOC of entries by topic → findings are hard to retrieve.
- **A9 Stale worktrees:** 9 `.claude/worktrees/agent-*` with substantial uncommitted diffs from before "VoiceBox" (see CHECKLIST ~line 4734); never reconciled. Do-not-delete rule stands; a reconciliation plan is missing.

## B. Registries (code)
- **B1 ZERO_SHOT_CALL_REGISTRY is described by name, not keyed on the ledger's call-site labels (CORRECTED).** Ledger labels recorded (counts): `walk:pipeline-9` 3,310, amt_branch_generation 608, assistant_check_up 362, amt_reexpansion 291, methodology_compliance_check 216, confetti_adjudication 163, i_loop_reflection 154, amt_detail_extraction 122, meta_loop_draft 108, methodology_synthesis 94, response_graph_render 88, blueprint_assignment 52, amt_branch_graph_native 50, zero_shot_simulation 48, methodology_domain_id 46, amt_intent_extraction 12, amt_cross_ref 10. 11 of the 17 labels do not appear verbatim in the registry, BUT 6 of those are documented under descriptive wording (compliance check, confetti, synthesis, domain id, cross-ref …). **5 have no mention at all**: `amt_branch_generation` (608 calls), `amt_detail_extraction` (122), `response_graph_render` (88), `amt_branch_graph_native` (50), `amt_intent_extraction` (12). `walk:pipeline-9` (57% of calls) is a catch-all label: the real site is not recorded. → rows need a stable `call_site` key; a test must diff code labels ⟷ ledger labels ⟷ registry.
- **B2 Pipeline registry (`src/pipeline/registry.rs`, 55 entries) vs reality.** 17 entries have NO crate (ids 35 home_return, 36 task_viewer, 39–51 consciousness_* variants, 53 voice_identity, 54 meta_portion_consciousness). 35 of 55 have NO built binary (incl. 2 theme_loader, 3 zsei_query, 4 zsei_write, 11 methodology_fetch, 13 blueprint_search, 15 pipeline_creation, 18 code_analysis, 19 package_context, 20 text_analysis, 22 graph_visualization, 24 reordering, 26 integrity_check, 27 consensus, 28 external_reference, 29 package_relationship, 34/37/38). 40 crates (all 27 modalities + several consciousness crates e.g. `decision_gate`, `experience_memory`, `reflection`, `relationship`, `self_model`) are NOT in this "single source of truth" (names differ: registry `consciousness_decision_gate` vs crate `decision_gate`). → what happens when a blueprint selects a registered pipeline that has no binary? Needs a defined honest failure + a registry⟷crate⟷binary consistency test.
- **B3 Tool registry (`mcp_tool_registry.json`, 184 tools, 23 ports)** vs 23 `tools/*-mcp` dirs vs `docs/TOOLS_REGISTRY.md` (52 table rows): counts do not reconcile; 67 tools sit behind one port (3210). A `.bak-ports-…` file sits next to it.
- **B4 `model_windows` non-chat list is name-fragment based** (guard/moderation/shield/safety/classif/lyria/embed/rerank/whisper/tts/image/imagen/veo). Duplicated in the prompt binary (now) — two copies can drift. The live catalog carries `architecture.output_modalities` (the 2 lyria models are the only non-text free models today) but `model_catalog.json` stores only id/context_length/max_completion_tokens/is_free.
- **B5 `k_registry.rs`** (K algorithm families) and `STATIC_LOGIC_REGISTRY` — not re-checked this session against the new sufficiency/served-filter logic (no K family for "sufficiency" or "served-member").
- **B2a `auth` (id 1) and `sync` (id 33) — wired?** Registered in `registry.rs` and `bootstrap.rs` (category general), but NOTHING calls them by id: not the host, not the UI, not the E2E suites. The host does both natively (`src/auth/mod.rs` — same ed25519 verify; `src/network/mod.rs` — P2P/sync routes). They are reachable only through the generic `/pipeline/execute` or a blueprint that selects them. `tools/PIPELINE_TRIAGE.md` already listed both as broken/excluded, i.e. they were known-dead crates. No doc records them as deprecated. Today they were made to compile (sync: GetStatus field names; auth: ed25519-dalek dep) — nothing else changed. **Not deleted, not declared deprecated — decision needed (H4).** `zero_shot_simulation` (id 16) is NOT in this class: Stage 4 uses it on every walk.
- **B6 Provider/capability vocabulary:** capability ids have no approver and no registry (open operator question since 2026-11-04).

## C. Stores
- **C1 `model_ledger.jsonl` is corrupted by concurrent appends:** 63 of 5,797 lines unparseable (truncated JSON + blank lines, interleaved writers — host + every prompt subprocess append large `message` strings >PIPE_BUF). The reader skips them (one warn at boot), so rankings silently ignore the worst failures. → single-writer append (or file lock/ O_APPEND with line < PIPE_BUF + message truncation into a side file), plus a repair/compaction tool.
- **C2 Ledger records only SERVED-model rows for successes; failures are keyed to the requested id** — an `openrouter/free` failure cannot be attributed to a member (384 empties + 215 other errors + 26 HTTP + 10 timeouts = ~49% of its 1,245 calls unattributable). Empty-response diagnostics (`served=` in the prompt binary stderr) are printed but not stored.
- **C3 `walk:pipeline-9` call_site** hides the true site for 57% of rows; ledger rows have no `trace_id/span_id` join for many paths.
- **C4 In-memory vs durable:** statics without a proven durable twin: `run_spans::RUNS`, `ripple_class` LoopGate counters (status only), `orchestrator::HEALTH` (model health map, mod.rs:3937 — consecutive-failure counter, reset on restart), `i_loop::LAST_REFLECTED_FINGERPRINT`, `jurisdiction_search::attempted_regions`, `model_windows::POOL/WINDOWS`. Check each for restart behaviour (several are intentionally ephemeral; none is documented as such).
- **C5 No retention/compaction policy** for any append-only store (ledger 5.8k lines, `zero_shot_calls.jsonl`, `tool_calls.jsonl`, `sensor_ticks.jsonl`, `methodology_gaps.json`, `methodology_reviews.json`). No-cap doctrine says do not drop — a compaction (summarise old rows, keep raw archive) design is missing.
- **C6 `runs/` checkpoints:** task 1049/1073 "walk failed before task creation — PAUSED"; resume re-runs the walk from the START (not from the failed stage). Stage-level resume is not implemented.
- **C7 stress-test fixtures / orphan graph files / interrupted tasks** hygiene still open (operator list).

## D. Tests
- **D1 MASTER_TEST_GUIDE (62 KB, 35 sections) does not mention** `fanout-coverage`, `pause-resume`, `methodology-sufficiency` (new), and mentions `clock`, `marketplace`, `mcp-contract`, `meta-ripple`, `provider-step` once each. §26g–i are the only 2026-11-04 rows. No run-ledger line for today's results (46/46, 43/2 skip, 10/10, 16/16, 13/13, 11/1 skip).
- **D2 Walk-dependent tests are non-deterministic** because they ride `openrouter/free`: provider-step PS3/PS4 failed twice with Stage 2 watchdog (1800 s) while a tiny prompt answered in 3–5 s. Missing: a walk-readiness preflight (probe N calls, record served members) and a per-test model pin for LOCAL runs.
- **D3 Test input-shape bugs:** provider-step sent `{action:{type:"Create",…}}` to pipeline 12 (needs flat `{action:"Create",…}`) — it passed review because tests were never run; PS5b crashed on `undefined.slice`. → generate request fixtures from the pipeline input types or add a contract test per pipeline (L1 of §2).
- **D4 L1 compile gate misses never-built crates:** `scripts/build-pipelines.sh` only builds crates that already have a binary → `sync` (wrong field names) and `auth` (missing dep) were broken at HEAD unnoticed. → `cargo check` every crate in `assets/pipelines/*/*` in L1. Script is also not executable (nohup "Permission denied").
- **D5 No unit tests in 16 of 27 modalities** (that is how fabricated output shipped); `tool_routing` had a wrong-by-design tie-break test failure only caught by running tests.
- **D6 No test for:** served-member filter (host + prompt binary), proven-in-slot ordering, `Update` merge (new methodology-sufficiency.mjs, not yet run), ledger concurrent-append integrity, registry⟷crate⟷binary consistency, call-site label ⟷ registry diff.
- **D7 Skipped rows:** mcp-contract 2 skips and MR5 (needs a gap-forcing request) — MR5 is now answerable: with sufficiency review, ANY request queues a review (M3 in the new test).
- **D8 Per-modality analysis E2E not run** for the completed set (audio, video, image, text, code, math, chemistry, DNA real parts, 3D AnalyzeScene); clock (Blender → CAD) and pause-resume not run.

## E. Model routing — where it can be made more intelligent
- **E1 Per-call-site × served-member scoring.** Success rates today are per model id globally (nemotron-ultra 77%, openrouter/free 49%). A JSON-strict call (methodology draft, entity extraction) and a prose call (reflection) have different failure profiles; score on (call_site, served_model).
- **E2 Use catalog capability metadata:** store `architecture.output_modalities/input_modalities`, `supported_parameters` (response_format / structured_outputs / tools / reasoning), `top_provider.max_completion_tokens`, `pricing`; filter non-text by metadata (not names) and require `structured_outputs` for JSON-demanding call sites.
- **E3 Attribute router failures:** on empty/HTTP-error replies read `x-generation-id`/body `id` and look up `GET /generation?id=` (served model + provider) to credit the failure to the member.
- **E4 Slow-member awareness:** probe latencies show 3–5 s for small prompts but 12–30 s for entity extraction on rotating members (dots-3-note-preview 26–30 s, flint 26 s). Add per-member latency to ranking and a per-call soft deadline that re-calls (the router re-picks) before the 90 s+ request timeout is burnt.
- **E5 Stage 2 cost:** TextAnalysis makes many sequential internal prompt calls for a ONE-sentence request (26 min once). Candidates: batch independent extractors in parallel behind the shared rate limiter; skip modality extractors whose output the next stage does not consume; per-extractor outcome recorded.
- **E6 Rate limiter** (`openrouter_bucket`): the "waited 1 ms for a slot" lines come from a per-minute bucket shared by all processes — expose its state in `/health` and add it to the walk-readiness preflight.
- **E7 Proven-in-slot** is a first rule; extend with: time-of-day/quota awareness (`openrouter_quota.json`), per-member cooldown after consecutive failures, and promotion/demotion events written to the ripple so the assistant can tell the operator.

- **E8 Host-log review for limits (2026-11-06):** across host.log … host4.log there is NO 429 / Retry-After / "free-models-per-min" error. The local cross-process limiter (`OZONE_OPENROUTER_RPM`, default 20/min, shared bucket file) never blocked ("waited 1 ms"). Daily free quota (`openrouter_quota.json`): 131 / 1000 used on 2026-11-06, remaining 869 — so a 1,000/day ceiling EXISTS and is the real budget: one ONE-sentence request cost ≥34 Stage-2 calls (≥18 detect + 14 validate + 2 section) and a full walk likely 100–300 → only ~3–10 walks/day fit. Provider-side errors seen: HTTP 503 "Provider returned error" from nemotron-ultra (4×, falls back to the router), HTTP 404 "provider/model endpoint" for the content-safety model, one quota-check connect failure. → expose remaining daily quota in the walk-readiness preflight and in `/health`; budget per walk by call count; consider batching the 5× yes/no votes into ONE call returning N independent verdicts (cuts both latency and quota).
- **E9 Stage 2 5× votes now concurrent** (text pipeline `confirm_times`): first vote gates, remaining 4 run concurrently; same all-5-must-agree rule; same total calls on a pass, ≤3 extra on a late fail.

## F. Methodology & zero-shot expansion
- **F1 Sufficiency review now exists** (calls #19/#20, pipeline 12 Update fixed). Missing around it: (a) a **run-outcome signal** — failed/paused steps, simulation `feasibility`, consciousness-gate declines and `Stage 8` findings should raise `modify` reasons, not only the request text; (b) **dedupe vs. in-flight gaps** by semantic (not keyword) overlap; (c) a **usage ledger per methodology** (times gathered / times its rules led to a successful step) feeding retire/merge proposals; (d) UI: a Methodologies view of open `new`/`modify` gaps and their reviews (Marketplace shows providers; gaps are not shown).
- **F2 Same symmetric review for blueprints and pipelines:** "is a new blueprint/pipeline needed, or should an existing one change" — blueprint_search/pipeline_creation (ids 13/15) have no binary today.
- **F3 Same for modalities:** unknown extension / low-fidelity modality → capture "missing modality capability" (sparse-capture modalities are on the remaining list).
- **F4 Capability-gap loop closure:** `capability_gaps.json` → binding proposals → "learn once, bind forever" consumer still unbuilt; marketplace peer catalog + install-from-peer unbuilt.
- **F5 Call-time argument derivation** for provider steps (registry `input_schema`, `timeout_ms`, `provider`, `modality`, `capability_ids`) still open.
- **F6 Context-mirror ripple for `call_global`; modality-first affinity / 3D owner** still open.
- **F7 Zero-shot call registry as code:** generate the registry table from a `const CALL_SITES: &[CallSite{label, file, detached, inputs, output_schema}]` so docs/tests/UI read one source (kills B1).

## G. Build / ops
- **G1 No documented host start/stop.** Host runs from `target/release` with `OZONE_TOOL_ROUTING=1 OZONE_TOOL_ROUTING_REQUIRE_GATE=1`; learnt the hard way that `pkill -f <pattern>` can kill the caller's own shell. Needs `scripts/run-host.sh` (+ pid file, env, log path) and a restart note in MASTER_TEST_GUIDE §1.
- **G2 Release build is ~13–15 min; the pipeline fleet ~1 h** — a documented "which crate to rebuild after editing X" map would avoid full rebuilds.
- **G3 Host ⟷ pipeline binary version skew:** pipelines launch fresh per call (pick up rebuilds immediately); the host does not — a version/stamp endpoint (host build id, per-pipeline binary mtime/hash) would make skew visible in `/health`.
- **G4 EU deploy** pending: diff EU vs local, ship host + tools/shared; everything since 2026-11-03 23:19 deploy is undeployed (incl. today's served-member filter, sufficiency review, sync/auth fixes).
- **G5 SSH hardening command and key rotation** still operator-side (fail2ban 76,388 failures).

## Suggested order once approved (not started)
1. Answer H1–H3. 2. C1 ledger single-writer + repair (it corrupts the data every routing decision reads). 3. D4 (cargo-check all crates in L1) + D3 contract fixtures. 4. B1/F7 call-site registry as code + diff test. 5. B2 registry⟷crate⟷binary consistency + honest failure for missing binaries. 6. E2/E3 catalog metadata + failure attribution. 7. A3/A4 store registry + doc index; update A1/A2/A7. 8. F1 run-outcome signals. 9. Walk-readiness preflight (D2), then the pending E2Es (D8), then EU deploy (G4).

## 2026-11-06 — "nothing should be dropped" sweep (operator) — what was fixed, what is captured

**Fixed now (edits):**
1. `shared/contracts/k_validation.rs::yes_no_input` — the strict YES/NO instruction was sent as `system_context`, a key `PromptInput` does not have (serde dropped it): EVERY K-validation vote (text pipeline's 5x checks and the host's AMT validation) went out WITHOUT its "answer with one JSON object" instruction. It is now sent as `system_prompt` AND appended to the prompt (backends that ignore system prompts still get it). Test `yes_no_input_uses_keys_the_prompt_pipeline_reads` fails if any key is not a `PromptInput` field.
2. Vote temperature stays **0.0** (operator ruling 2026-11-06: it is a yes/no — a correct YES should repeat; noise only creates spurious NOs and re-proposals). A 0.5 setting was tried and reverted. REVIEW_OPTIMIZATION item 8 ("votes stop being independent") is therefore NOT acted on: near-identical answers are expected; the check's value is consistency + provider-failure tolerance, not independent sampling.
3. First-N cuts removed from DATA PATHS (29 sites): graph-persistence loops (haptic shear vectors 50 / deformation maps 10; CAD drawing views 30 / BOM 100; network flow paths 50; thermal contours 20; 3D VSE strips 50; biology regulatory network 100 / metabolites 50; kinematics trajectories 20; sound units 200 / echolocation pulses 100 / events 100; proteomics expression 200 / MS ids 100; hyperspectral anomaly pixels 50 / absorption features 5 / signature→chemical links 10; EM snapshot links 3), the **network export previews** (nodes 500/200, edges 1000/500 in JSON/Cytoscape/DOT exports — exported files were silently incomplete), and `span_out` / `/context` keyword seeds (take(40) + a 12-keyword floor — span-out is token-budgeted, the cut was extra).
   Left on purpose: `code` header-line scan (6 lines, detection not data), `actors.rs` 80-char error fingerprint (dedupe key), short display labels (`chars().take(25..60)` on node labels).

**Captured, NOT edited (needs per-site windowing, not blind removal):** ~75 prompt-side `.iter().take(N)` lists and 63 `chars().take(N)` in prompt text/node content across the modalities (biology 12, BCI 8, thermal/sound/haptic/EM/CAD 4 each, proteomics/image/hyperspectral/geospatial/browser_navigation 3 each, …). The prompt pipeline only FLAGS an over-window prompt (`context_truncated`), it does not cut or chunk, so deleting these caps would turn a silent cut into an over-window failure on small local models (BitNet 4096). Correct fix = the relation-window pattern (`shared/semantic_relations::windows`): chunk the list to the READING model's window, one call per window, merge — per site. Sites (file:line):
```
consciousness/emotional_state/main.rs:171:        let recent: Vec<_> = self.history.iter().rev().take(5).collect();
consciousness/emotional_calibration/main.rs:78:        let recent: Vec<_> = self.snapshots.iter().rev().take(100).cloned().collect();
modalities/kinematics/main.rs:1160:        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
modalities/kinematics/main.rs:1217:            "joint_types": analysis.joints.iter().take(5).map(|j| format!("{:?}", j.joint_type)).collect:
modalities/depth/main.rs:876:        let list: Vec<serde_json::Value> = clusters.iter().take(20).map(|c| serde_json::json!({
modalities/depth/main.rs:999:        let shown: Vec<&DepthGraphNode> = nodes.iter().take(25).collect();
modalities/haptic/main.rs:1086:        let list: Vec<serde_json::Value> = events.iter().take(20).map(|ev| serde_json::json!({
modalities/haptic/main.rs:1223:        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
modalities/haptic/main.rs:1706:    let pressure_node_ids: Vec<(u64, u64)> = analysis.pressure_maps.iter().take(20).enumerate().map(|(i, pm)|
modalities/haptic/main.rs:2144:                    top_candidates: lib.iter().take(3).map(|t| (t.material_name.clone(), 0.7)).collect(),
modalities/image/main.rs:1619:                    let key: String = d[..z].iter().map(|c| *c as char).take(80).collect();
modalities/image/main.rs:1620:                    let val: String = d[z + 1..].iter().map(|c| *c as char).take(200).collect();
modalities/image/main.rs:1695:        let s: String = raw.iter().take_while(|c| **c != 0).map(|c| *c as char).take(120).collect();
modalities/CAD/main.rs:1036:        let list: Vec<serde_json::Value> = features.iter().take(30).map(|f| serde_json::json!({
modalities/CAD/main.rs:1101:        let shown: Vec<&CADGraphNode> = nodes.iter().take(25).collect();
modalities/CAD/main.rs:1163:            "sample_part_names": analysis.parts.iter().take(5).map(|p| &p.name).collect::<Vec<_>>(),
modalities/CAD/main.rs:1536:        let param_summary: Vec<String> = feat.parameters.iter().take(3).map(|(k, v)| format!("{}={:.2}", k, v)).
modalities/geospatial/main.rs:1048:        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
modalities/geospatial/main.rs:1111:        let list: Vec<serde_json::Value> = regions.iter().take(20).map(|r| serde_json::json!({
modalities/geospatial/main.rs:2161:                    let nearest: Vec<_> = with_dist.iter().take(20).map(|(n, d)| serde_json::json!({"node
modalities/uidocument/src/rules.rs:538:            return n.attrs.get(name).map(|v| json!(v.split_whitespace().take(50).collect::<Vec<_>>())
modalities/uidocument/src/svg.rs:836:    let c: String = raw.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c ==
modalities/uidocument/src/a11y_json.rs:574:        .map(|f| f.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(*c, '-' | '_' | ':' |
modalities/network/main.rs:999:        let host_list: Vec<serde_json::Value> = hosts.iter().take(25).map(|h| serde_json::json!({
modalities/network/main.rs:1112:        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
modalities/electromagnetic/main.rs:876:        let list: Vec<serde_json::Value> = signals.iter().take(20).map(|s| serde_json::json!({
modalities/electromagnetic/main.rs:913:        let list: Vec<serde_json::Value> = emitters.iter().take(15).map(|e| serde_json::json!({
modalities/electromagnetic/main.rs:964:        let shown: Vec<&EMGraphNode> = nodes.iter().take(25).collect();
modalities/electromagnetic/main.rs:1209:    let snap_node_ids: Vec<(u64, u64)> = analysis.spectrum_snapshots.iter().take(50).map(|snap| {
modalities/thermal/main.rs:881:        let list: Vec<serde_json::Value> = hotspots.iter().take(20).map(|h| serde_json::json!({
modalities/thermal/main.rs:917:        let list: Vec<serde_json::Value> = zones.iter().take(20).map(|z| serde_json::json!({
modalities/thermal/main.rs:980:        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
modalities/thermal/main.rs:1033:        let list: Vec<serde_json::Value> = sigs.iter().take(10).map(|s| serde_json::json!({
modalities/biology/main.rs:1151:        let pw_list: Vec<serde_json::Value> = pathways.iter().take(20).map(|p| serde_json::json!({
modalities/biology/main.rs:1183:            "top_markers": c.marker_genes.iter().take(5).collect::<Vec<_>>(),
modalities/biology/main.rs:1212:        let pw_names: Vec<&str> = pathways.iter().take(10).map(|p| p.name.as_str()).collect();
modalities/biology/main.rs:1213:        let dis_names: Vec<&str> = diseases.iter().take(10).map(|d| d.disease_name.as_str()).collect();
modalities/biology/main.rs:1265:        let shown: Vec<&BiologyGraphNode> = nodes.iter().take(25).collect();
modalities/biology/main.rs:1321:        let pw_info: Vec<serde_json::Value> = pathways.iter().take(30).map(|p| serde_json::json!({
modalities/biology/main.rs:1323:            "pathway_genes": p.gene_symbols.iter().take(20).collect::<Vec<_>>(),
modalities/biology/main.rs:1341:            gene_list.iter().take(30).map(|g| g.as_str()).collect::<Vec<_>>().join(", "),
modalities/biology/main.rs:1526:            keywords: { let mut kw = vec!["pathway".into(), pw.database.to_lowercase(), format!("{:?}", pw.c
modalities/biology/main.rs:1615:            keywords: { let mut kw = vec!["cell-type".into(), ct.name.to_lowercase()]; kw.extend(ct.marker_g
modalities/biology/main.rs:1638:            keywords: { let mut kw = vec!["cluster".into()]; kw.extend(cluster.marker_genes.iter().take(3).m
modalities/biology/main.rs:2200:                CellCluster { cluster_id: *cid, cluster_name: format!("Cluster_{}", cid), cell_count: cells.
modalities/sonar/main.rs:1014:        let node_list: Vec<serde_json::Value> = nodes.iter().take(30).map(|n| serde_json::json!({
modalities/3D/main.rs:1616:        let shown: Vec<&ThreeDGraphNode> = objects.iter().take(20).copied().collect();
modalities/3D/main.rs:1678:            "object_names_sample": analysis.objects.iter().take(5).map(|o| &o.name).collect::<Vec<_>>(),
modalities/3D/readers.rs:326:                let c: Vec<f64> = it.take(3).filter_map(|t| t.parse::<f64>().ok()).collect();
modalities/3D/readers.rs:471:            let c: Vec<f64> = l.split_whitespace().skip(1).take(3).filter_map(|t| t.parse::<f64>().ok()).collec
modalities/IMU/main.rs:1268:        let list: Vec<serde_json::Value> = features.iter().take(20).map(|f| serde_json::json!({
modalities/IMU/main.rs:1350:        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
modalities/BCI/main.rs:1044:        let sig_list: Vec<serde_json::Value> = signals.iter().take(20).map(|s| serde_json::json!({
modalities/BCI/main.rs:1081:        let epoch_list: Vec<serde_json::Value> = epochs.iter().take(20).map(|e| serde_json::json!({
modalities/BCI/main.rs:1087:            "csp_features": e.features.csp_features.iter().take(4).collect::<Vec<_>>(),
modalities/BCI/main.rs:1124:        let feat_list: Vec<serde_json::Value> = features_list.iter().enumerate().take(20).map(|(i, f)| serde_jso
modalities/BCI/main.rs:1196:        let shown: Vec<&BCIGraphNode> = nodes.iter().take(25).collect();
modalities/BCI/main.rs:2399:                    decoded_state.continuous_output.iter().take(2).cloned().collect()
modalities/BCI/main.rs:2402:                    decoded_state.continuous_output.iter().take(3).cloned().collect()
modalities/BCI/main.rs:2407:                        .take(6).collect()
modalities/control/main.rs:1176:        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
modalities/sound/main.rs:986:        let voc_list: Vec<serde_json::Value> = vocalizations.iter().take(15).map(|v| serde_json::json!({
modalities/sound/main.rs:1046:        let voc_list: Vec<serde_json::Value> = vocalizations.iter().take(20).map(|v| serde_json::json!({
modalities/sound/main.rs:1120:        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
modalities/sound/main.rs:1183:        let voc_list: Vec<serde_json::Value> = vocalizations.iter().take(30).map(|v| serde_json::json!({
modalities/hyperspectral/main.rs:953:                let sig_list: Vec<serde_json::Value> = sigs.iter().take(20).map(|s| serde_json::json!({
modalities/hyperspectral/main.rs:1107:                let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json
modalities/hyperspectral/main.rs:1579:            let sig_node_ids: Vec<(u64, u64)> = analysis.spectral_signatures.iter().take(200).map(|sig
modalities/proteomics/main.rs:1256:        let list: Vec<serde_json::Value> = proteins.iter().take(20).map(|p| serde_json::json!({
modalities/proteomics/main.rs:1299:        let list: Vec<serde_json::Value> = ptms.iter().take(30).map(|ptm| {
modalities/proteomics/main.rs:1355:        let node_list: Vec<serde_json::Value> = nodes.iter().take(25).map(|n| serde_json::json!({
general/browser_navigation/main.rs:306:    let keywords: Vec<String> = sorted.iter().take(30).map(|(w, _)| w.clone()).collect();
general/browser_navigation/main.rs:307:    let key_concepts: Vec<String> = keywords.iter().take(10).cloned().collect();
general/browser_navigation/main.rs:312:        summary, key_concepts: key_concepts.clone(), keywords, topics: key_concepts.iter().take(5).cl
general/context_aggregation/main.rs:493:        .map(|obj| obj.values().take(3).filter_map(|v| v.get("summary").and_then(|s| s.as_str()).map
src/orchestrator/span_out.rs:360:                sequence_head: d.sequence.iter().take(12).cloned().collect(),
src/orchestrator/actors.rs:1200:                    "keys": out.as_object().map(|o| o.keys().take(8).cloned().collect::<Vec<_>>()),
src/grpc/mod.rs:1019:        let words: std::collections::BTreeSet<String> = text.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >
src/grpc/mod.rs:1028:        scored.into_iter().take(3).map(|(_, n)| n.clone()).collect()
src/mcp_graph.rs:631:        let shown = notes.iter().take(20).cloned().collect::<Vec<_>>().join("; ");
```
**Operator decision recorded:** tiny-input fast path (§A6 of REVIEW_OPTIMIZATION) — ALLOWED as an opt-in config knob but NOT implemented; never-skip doctrine stands (nothing cut from important info).
