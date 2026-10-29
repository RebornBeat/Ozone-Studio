# OZONE STUDIO — COMPREHENSIVE STATE CAPTURE
## 2026-09-17 · ZCode + Claude Code joint session · living graph operational

> **LATEST 2026-10-29 (cc, FULL-STATE CAPTURE)** — Docs-only review (no code edits, builds or tests). Master guide: `docs/MASTER_REMAINING_WORK_GUIDE.md`. Headlines: deployed binary predates jurisdiction aliases/sync + internal-token + I-Loop fixes (one build+deploy gates most work; local host down; 7.6G RAM / 18G disk); browser-farm-mcp is mock-tested only → real-farm test (local or EU VPS) is next; MCP registry persists (my earlier "in-memory" claim was wrong); Browser-Farm's vault is plaintext → Ozone Vault design captured (never in graph/LLM/logs; broker leases; human-only reveal); a live OpenRouter key sits in plaintext `config.toml` (never committed; rotate); `.md→math` not reproduced but four disagreeing modality tables, a PDF→"Document" dead end, and a modality-blind MathViewer found; code call graphs are same-file only and have no 3D lens; in-world indicators + controls designed, not built.
>
> **LATEST 2026-10-29 (cc, BROWSER AUTOMATION)** — Ozone now has a governed browser (`browser-mcp`: snapshot/click/type/screenshot through `/mcp/call`, locked-by-default, origin guard; 18/18 direct and through a host) and a Browser-Farm Command-Center bridge (`browser-farm-mcp`; 20/20 against a MOCK farm only — no live farm yet). DOMorpher: lowercase `~/Projects/domorpher` is the complete copy (superset) but neither copy compiles (no deps declared; 27 lexer errors in `agent/action.rs`). Driving the real web UI through the MCP found and fixed two UI defects (web-mode pipeline tabs; tab strip hidden under search). Guide: `docs/BROWSER_AUTOMATION_GUIDE.md`; CHECKLIST "BROWSER AUTOMATION".
>
> **LATEST 2026-10-28 (cc, INDICATOR RAIL + WORKSPACES TAB POLISH)** — The workspace header now carries a live indicator rail: tasks (real /task/list counts + recent-task popover), consciousness gate (pending reviews / last decision), peers, graph heartbeat — on every mount of the graph-native workspace. The Workspaces tab (landing + workspace/project list) is restyled to the same visual language by CSS only. Rendered from static DOM mocks; not yet seen in Electron. See CHECKLIST "INDICATOR RAIL + WORKSPACES TAB POLISH".
>
> **LATEST 2026-10-28 (cc, GRAPH-NATIVE WORKSPACE POLISH) — the 3D world is now THE workspace, not a tile above the classic view.** One full-size world (root -> graph hubs -> node clusters; files and other containers on outer shells; cross-modal links in magenta) with the classic Files/Fabric/Hierarchy/Engines views **docked inside it** (resizable Inspector | Detail dock) and a lens bar whose lenses really highlight/dim and re-frame the camera; click/double-click/find-in-graph/keys traversal; real construction pulses; WebGL-fallback; lazy 3D chunk (bundle 1.62 MB -> 766 KB). The same component now backs BOTH the Workspace tab and the global Files & Code / Fabric / Hierarchy / Engines tabs (Files gained Math/Editor/History). Provider leaks fixed; `WorkspaceStatusStrip` now reaches the host under Electron. `tsc` 0 errors, bundle OK, and the UI was **rendered and driven in headless Chrome with WebGL** against synthetic AND the live host's real data (one Fabric-lens counting bug found and fixed that way). Not verified: inside Electron, real-GPU scale. Details: `docs/GRAPH_NATIVE_UIUX_GUIDE.md` §9, CHECKLIST "2026-10-28 (cc) — GRAPH-NATIVE WORKSPACE POLISH". Next: jurisdiction ingest-from-law-books.
>
> **LATEST 2026-10-28 (cc, JURISDICTION EXPANSION) — jurisdiction content went from 41 files / 161 rules to 132 files / 648 rules** (UN layer 7 -> 41; all 27 EU member states; 131 national/local files incl. 8 sub-national; 308 rules with local-language aliases). Every added rule is `Log`; only the operator's own DO/EU elevations and the 7 baseline global rules act. Gathered by live web search — mostly search summaries, a minority from fetched statute text (disclaimers and snippets say which); ~220 rules not `official_source`; ~60 scopes still thin. Engine: rule `aliases` (multilingual match), upgrade-safe content sync (`OZONE_JURISDICTION_FORCE_SYNC=1` once after the next build), local internal token so the strict session gate no longer 401s the shared-context MCP (found live). All code UNCOMPILED. How-to + elevation suggestions: `docs/JURISDICTION_SETUP.md`, `assets/jurisdiction/ELEVATION_CANDIDATES.md`. CHECKLIST "2026-10-28 (cc) — JURISDICTION EXPANSION". `tools/bridges` nested upstream repo still uncommitted (operator decision).
>
> **LATEST 2026-10-27 (zcode, INDICATOR-RETROFIT) — GraphKit indicators wired into the real UI panels: TaskStateIcon + StepPips in TaskDetailPanel, ActivityPulse in WorkspaceStatusStrip. The indicators are no longer "built but unwired."** Still TODO: GateState in OrchestrationStatusPanel, LoopPulse (loop cadence data to UI), CapturePresence, the full workspace shell revamp (guide §4.7). tsc clean in all edited files. CHECKLIST "2026-10-27 (zcode, INDICATOR-RETROFIT)".
>
> **LATEST 2026-10-27 (zcode, GRAPH-NATIVE-WORKSPACE) — Tasks removed from workspace tabs; GraphNativeSection component created and wired into ALL workspace sections.** Every tab now renders the SAME 3D GraphWorld3D substrate from the GraphProvider — switching tabs is a lens/filter change on one persistent 3D world, not a page swap. The substrate loads once per project; switching is instant. CrossLinkStrip on selection shows cross-links as clickable chips. The existing engine views become detail panels alongside the 3D world. Guide §8 has the full 3D world spec. **tsc: 1 pre-existing legacy error (chatRendering markdown renderer). No builds, no tests.** CHECKLIST "2026-10-27 (zcode, GRAPH-NATIVE-WORKSPACE)".
>
> **LATEST 2026-10-27 (zcode, FINAL-CONTINUE) — jurisdiction elevations APPLIED (DO: personal data→RequireConfirmation; EU: data breach + personal data→RequireConfirmation — operator green-lit); GraphProvider renders ALL container types as 3D nodes (text-only projects no longer empty); chatRendering fontSize cast; GraphWorld3D EdgeClass dash fix; three.js deps installed. **tsc: 1 pre-existing legacy error.** CHECKLIST "2026-10-27 (zcode, FINAL-CONTINUE)".
>
> **LATEST 2026-10-28 (zcode, ACCEPTANCES-CLOSED) — valence 1.0 ✅, cross-walk blueprint reuse ✅, EU on the absolute latest binary ✅.** The emotion path is LIVE for the first time (emotional_state.json shows satisfaction/valence 1.0 from a real task_success trigger — the C1 consolidation works). Cross-walk blueprint reuse verified (walk 2 found walk 1's blueprint at 100% match — the blueprint persistence repair is functional). EU redeployed on the absolute latest binary (md5-verified, all fixes). Remaining: CC's jurisdiction expansion (assignment posted), jurisdiction elevation queue (operator review), data mutations (operator-gated), GraphKit UI polish waves. CHECKLIST "2026-10-28 (zcode, ACCEPTANCES-CLOSED)".
>
> **LATEST 2026-10-28 (zcode, KOI ACCEPTANCE) — both koi walks COMPLETED on the new binary.** Task 848 (walk 1): 1 step, 55k tokens. Task 853 (walk 2): 3 steps, 131k tokens. Full 14-stage pipeline verified: jurisdiction loaded 22 rules from graph, consciousness gate reviewed the plan, stage-12 truth fix live (honestly reporting 3 consciousness pipeline failures), fallback chain worked (nemotron → poolside/laguna after openrouter/free truncation), response delivered through the ladder. npm install done; UI tsc down to 1 pre-existing legacy error (chatRendering markdown renderer). **Cross-walk content transfer PARTIAL** — walks completed but the keyword-gated reachability of walk 1's containers to walk 2 needs S11 analysis. CHECKLIST "2026-10-28 (zcode, KOI ACCEPTANCE)".
>
> **LATEST 2026-10-27 (zcode, CONTINUE-ALL) — tombstone pass II CLEAN (0 additional torn; the earlier fixes covered it), EU redeployed on the LATEST binary (all fixes live, all 5 env vars, 12G box), three.js + GraphWorld3D landed (deps in package.json; tsc errors EXPECTED until npm install), chat modify/retry types added, jurisdiction elevation queue written (DO+EU proposals, PENDING-OPERATOR-REVIEW), the I-Loop's failed_structural echo confirmed as the system's first genuine self-observed defect (tombstones broke the echo at source). NO in-tree edit items remain. The remaining events are: npm install (for three.js), the operator's elevation review, koi + valence acceptances, and any future waves.** CHECKLIST "2026-10-27 (zcode, CONTINUE-ALL)".
>
> **LATEST 2026-10-28 (zcode, FULL-STATUS) — comprehensive review of CC's latest work, both hosts, all logs, chat history, capture stores.** CC committed 3 new commits: jurisdiction expansion (132 files, 648 rules, UN 41, all 27 EU, aliases — UNCOMPILED), graph-native workspace polish, indicator rail. Local host: sensor tick LIVE (27 events), task executor 31 launches (task 44 completed), meta on OpenRouter (ledger-verified), amt_reexpansion tombstone echo BROKEN (pass II clean), chat persistence confirmed, Tasks tab removed. EU: latest binary, all 5 env vars, all 4 loops on schedule, p2p-bridge registered, 0 sensor events (MCP registration gap on EU). Capture stores: S11 2,614+, S12 2,319+, call_spans 2,051+, amt_candidates 451 (448 handled, 3 tombstoned). Chat persistence: sessionStorage confirmed in code. **NO edits this pass — read-only review.** Full detail: CHECKLIST "2026-10-28 (zcode, FULL-STATUS)".
>
> **LATEST 2026-10-27 (zcode, INDICATORS-PHASE2) — GateState wired into OrchestrationStatusPanel (stage-8 + stage-14 rows); LoopChips wired into WorkspaceStatusStrip (real WS activity heartbeat from GraphKit substrate).** The indicator set is now substantially visible across the running UI. Remaining wiring: CapturePresence (needs per-view S10-S13 mapping — minor). All tsc clean. CHECKLIST "2026-10-27 (zcode, INDICATORS-PHASE2)".
>
> **LATEST 2026-10-27 (zcode, FAILURE-AUDIT+TODO) — every failure from the live captures diagnosed; the I-Loop's own insight CONFIRMED as a real systemic defect; Tasks tab + chat persistence LANDED; 3D graph-native world spec captured.** amt_reexpansion 65% fail = 42 empty-model (OpenRouter drops + BitNet GPU-less) + permanent structural (torn 42xxx containers) — the walk's fallback is working, the torn containers are the root. **The I-Loop's "failed_structural echo" insight is the system's first genuine self-observed defect**: each terminal structural failure CREATES a new graph event the I-Loop then reflects on — the fix is the tombstone pass at source. Tasks tab added to workspace (OrderPanel); chat transcript now sessionStorage-persisted (tab-switch survival); 3D graph-native world spec appended to the GraphKit guide (§8 — three.js leaf renderer, camera-as-lens-selector, raycast navigation). **Remaining TODO (7 items in CHECKLIST §6): tombstone pass II, three.js dep, chat modify/retry/branch, koi+valence, EU redeploy, CC jurisdiction, engine ports + shell.** NO builds or tests run. CHECKLIST "2026-10-27 (zcode, FAILURE-AUDIT+TODO)".
>
> **LATEST 2026-10-27 (zcode, WIZARD+EU-FINAL) — the wizard complaints root-caused and fixed; 42209 tombstoned; both hosts on the final binary.** (1) The wizard re-showed because the strict-session middleware 401'd its empty-token /config/get (isFirstLaunch catch → true). (2) The confirm errored for the same reason on /config/set. (3) FIXED: /config/get split out of PROTECTED (safe — redacted), FIRST-RUN EXEMPTION added (gate stands down while setup_complete/user_setup_complete are false — the wizard must be able to write config pre-pairing; completing setup arms the gate). (4) The jurisdiction question is REMOVED from the wizard per operator ("we never ask — auto-detect only"); display-only summary line; server-side hardware detection unchanged. (5) 42209 reviewed: torn CreateContainer from the task-44 walk during a silent-death kill; content unrecoverable but REGENERABLE (task 44 still queued); tombstoned 42209+42210+42211 (search path unblocked, verified success:true). (6) EU redeployed with the session_gate fix (skip_seed + first-run exemption + jurisdiction removal); EU config/get with session → api_key [redacted] ×3; /pair page serving. CC's jurisdiction-expansion assignment written (mvr5cdvrx935). **The only remaining events are: koi + valence acceptances (system-side), EU national-rule content (CC's assignment), and any data-mutation passes the operator authorizes.** CHECKLIST "2026-10-27 (zcode, WIZARD+EU-FINAL)".
>
> **LATEST 2026-10-27 (zcode, META+EU+ENVELOPE) — meta on OpenRouter (ledger-verified), memory gate in code, EU DEPLOYED, 3 methodologies seeded, 1 data defect captured.** meta_fallback now leads openrouter/free (local + EU config; ledger shows amt_reexpansion on the free pool; BitNet backstop). Task executor memory gate: MemAvailable < OZONE_MIN_AVAIL_MB (default 1536) skips the walk loudly. EU: new binary + 6 pipelines + meta switch + systemd drop-in (all switches ON; 12G box) — active, health 200, boot evidence clean. Log review both hosts: expected actions confirmed (task 44 walked the gated walk locally; sensor tick + p2p-bridge + consciousness config on both). **DATA DEFECT (operator-gated fix): container 42209 global-without-local (torn write from a silent death) blocks keyword-search scans; persists across boots.** 3 methodologies seeded (memory-envelope, out-of-band-approval, reserved-id-override) — files + EU shipped + local-store registered. Detail: CHECKLIST "2026-10-27 (zcode, META+EU+ENVELOPE)".
>
> **LATEST 2026-10-25/26 (zcode, BUILD+TEST+RUN) — the gate opened and the cycle CLOSED: disk freed (15G) → switches flipped (launcher) → cargo build --release PASSED (28m47s + 25m31s incremental; 12 first-pass errors fixed live; from_fn Send lesson: own strings before awaits) → **151 tests passed, 0 failed** → 6 pipelines rebuilt serially 0 errors → host RELAUNCHED + LIVE-VERIFIED: 4 switches in environ, consciousness config applied (emotion path fed FIRST TIME), out-of-band pairing E2E (no code in API; host-console code TKQTMQT5 → approve → session), config redaction ([redacted] live), reserved-id reconciliation both directions live, strict-session 401s live, /pair page serving, InstrumentedExecutor streaming call_spans.jsonl, task executor fired (queued task 44 walked the full gated walk), sensor tick + p2p-bridge registered, pipeline-9 dispatch restored post-restart. TWO silent host deaths during task-executor walks on the 7.6G box → **OZONE_AUTO_EXECUTE_TASKS HELD OFF in the launcher** (envelope finding, not a code defect; the other 3 switches ON). Full serial log + acceptances (koi + valence pending with exact preconditions; EU redeploy unblocked): CHECKLIST "2026-10-25/26 (zcode, BUILD+TEST+RUN)".
>
> **LATEST 2026-10-23 (zcode, RULINGS-FINALIZE) — the open rulings closed to their edit-safe maximum.** #46 tool routing WIRED default-off (orchestrator/tool_routing.rs: applicable-tools selection, HTTP-only, call_global-metered, 2 tools/2k chars/4k block caps, <<<UNTRUSTED_DATA>>> everywhere, injected at execute_step; **OZONE_TOOL_ROUTING=1 is the ruling switch**). Peer dialogue primitives + caps landed (PeerDialogueTurn, thread-key derivation, hop 3 / 20-per-day / 4k chars, kill switch OZONE_PEER_DIALOGUE=1, paired-only; nothing auto-spawns). National-rules elevation documented as the concrete per-rule how-to (the policy act stays yours). Data mutations stay gated on the build (destructive + unverifiable). Wave-7 additive part landed (WorkspaceStatusStrip: real PeerState in the workspace header); full §4.7 layout held for the live-DOM pass. **NO edit items remain on any list — the next event is the build.** CHECKLIST "2026-10-23 (zcode, RULINGS-FINALIZE)".
>
> **LATEST 2026-10-23 (zcode, UI-FINALIZE) — GraphKit 6+8 UI-side done; /pair page live.** GraphCanvas renders inside every generic lens (18 modalities get real selectable canvases); EnginesPanel "All modalities" tab (the fleet reachable at the Engines surface); `/pair/:pairing_id` host-served code-entry page completes the out-of-band pairing loop; WorkspaceBrowser rows carry `graph →` chips; field-guide status notice. **Only open UI wave: 7 (workspace shell revamp — deliberate, needs view migrations + your look at the layout).** Everything else: operator chain + rulings. CHECKLIST "2026-10-23 (zcode, UI-FINALIZE)".
>
> **LATEST 2026-10-23 (zcode, REMAINING-FINALIZE) — the designed-not-built list executed (6 items):** GraphEvent carries trace_id/span_id (emit attaches the current scope — actor causality closes); peer_sync_review drains paired-peer envelopes into real finding containers (offset-resumable, bookkeeping-only); p2p-bridge-mcp registered at boot when P2P is on (auto_start false, supervisor gate intact); pairing is OUT-OF-BAND (no code in the API response — host console + pairing_pending/<id>.code; QR encodes only /pair/<pairing_id>); GraphCanvas landed in the kit (grid/radial/tree layouts, single canvas component); stale-doc corrections on 3 guides. **Post-finalize map: in-tree = ONE finalized uncompiled whole; remaining = the operator chain (disk→push→build→acceptances→EU redeploy), open rulings (#46 safe to wire), UI polish (GraphKit 7–8, /pair code-entry page).** CHECKLIST "2026-10-23 (zcode, REMAINING-FINALIZE)".
>
> **LATEST 2026-10-23 (zcode, FINALIZATION) — THE 12 IN-TREE ITEMS COMPLETE; TREE FINALIZED FOR THIS CYCLE.** Resolve+list routes (ReviewPending has its door); stage-12 truth (per-pipeline outcomes, no blanket success); server-side consciousness_enabled (request field ignored — chat cannot skip the gate); model_override whitelist (executable/secret fields stripped at coercion; loop cap 1,000; body-path traversal guard); untrusted labels on coordination injections; InstrumentedExecutor wrapped around the whole walk chain (every call indexed in call_spans.jsonl); sensor tick (15-min posture calls via call_global, mirrored → actors finally see real posture); approval-gated task executor (OZONE_AUTO_EXECUTE_TASKS=1 opt-in, full gated walk, non-prompt tasks stay records); CORS loopback predicate + OZONE_REQUIRE_SESSION strict header gate on mutating routes. `_` matcher + digest change-detection verified ALREADY LANDED (stale review claims corrected). Cross-link audit in CHECKLIST §2 (every item → doctrine doc → capture surface → downstream). **No builds/tests — compile-clean is the build pass's verdict** (89 files balance-clean, imports/icons/types verified per edit). Remaining: designed-not-built list (pairing UI, GraphKit 6–8, GraphEvent trace field, p2p-bridge registration) + operator rulings (#46 now SAFE to wire — labels+whitelist landed). Full detail: CHECKLIST "2026-10-23 (zcode, FINALIZATION)".
>
> **LATEST 2026-10-23 (zcode, TAKEOVER) — CC weekly-limited (resets Oct 25 7pm); operator transferred CC's work. DONE THIS PASS:** (1) **Blocker #1 was MY FALSE ALARM** — a proper Rust scanner (raw-strings/escapes/comments) shows orchestrator/mod.rs BALANCED (CC's fork repaired the BlueprintStep region before its limit) and **all 89 Rust files balance-clean**; raw brace counts on prompt-heavy files are not evidence. (2) **Blocker #2 FIXED: src/orchestrator/entry_gate.rs WRITTEN** (gated-entry decorator: trace root or walk-joined child + with_trace + one external-pipeline span row in call_spans.jsonl; tests included) and **WIRED at both external dispatch points** (OzoneRuntime::execute_pipeline + grpc rerun_step). (3) **Reserved-id reconciliation CLOSED**: /pipelines/register → register_guarded; built-in override requires allow_builtin_override + valid session (the deliberate operator feature CC preserved, the hijack closed). (4) CC's three finished forks spot-verified true (supervisor probe/logging/tools_dir; consciousness single-impl w/ observer task-manager + no loop-emotion + queue fixes; trace-core envelope stamping + sink fields + the 8-modality alias table; run_spans wired in assistant+meta_loop). (5) README gained an honest System Status section + doc map. **THE CONSOLIDATED TODO (single list, supersedes all): CHECKLIST entry "2026-10-23 (zcode, TAKEOVER)" §3** — operator-gated chain (disk → push → ONE build pass → acceptances → EU redeploy) + 12 in-tree edit items (InstrumentedExecutor adoption, resolve route, stages-side semantics, sensor tick, task executor+approval, override whitelist, perimeter rest, `_` matcher, untrusted labels, p2p-bridge-mcp, GraphKit 6–8, digest change-detection, doc sweep) + open rulings (tool routing #46 w/ preconditions, national rules elevation, data mutations).
>
> **LATEST 2026-10-23 (zcode, GraphKit wave) — UI GO EXECUTED (edits only, no builds/tests): GraphKit steps 1–2 COMPLETE + core library LANDED.** Discriminator single-sourced (graphRenderers/defaults.ts, derived from MODALITY_COLOR's 27 keys — all prefixes verified per backend; graphViewData's 6-prefix table AND crossModal's private 3-prefix copy replaced; unknownModalityGraphs counted never dropped; **GeospatialEngine unblocked**). /cross-modal-pairs consumed twice (C8 overlay fail-soft merge via zseiQuery; provider substrate). `ui/src/graphkit/` landed (6 files): graphProvider (one substrate fetch + one WS subscription + both-direction cross-link index incl. file refs), primitives (NodeCard/**CrossLinkStrip**/ModalityWheel/HonestState), indicators (TaskStateIcon/StepPips/GateState/PeerState/EdgeRealism — icon names verified against installed lucide), lens registry + generic lens (all 27 covered day-one). Existing views untouched (same conversion path, zero behavioral break). Guide §5 now a status table; §3.5 "two registries" CORRECTED (PWV already imports ENGINES — single-source since 10-18). Compile-clean UNPROVEN until the build pass. CHECKLIST "2026-10-23 (zcode, GraphKit wave)".
>
> **LATEST 2026-10-23 (zcode, UI pass) — GRAPH-NATIVE UI/UX GUIDE written: docs/GRAPH_NATIVE_UIUX_GUIDE.md (Ozone GraphKit).** Full read-only review of the real UI tree, every claim evidence-cited. The operator's diagnosis verified: engines are fetch silos (each calls loadGraphData({modality}) privately); the modality discriminator sees only 6 of 27 modalities (GeospatialEngine structurally broken — filters a modality the discriminator can never return); the real C8 cross-modal overlay renders only in GraphView; /cross-modal-pairs has zero UI consumers; files are a tab silo with no bidirectional graph links; two hand-mirrored engine registries; no unified indicator language. Strengths kept: WS singleton, classifier plugins, MODALITY_COLOR (all 27), navigation spine, honest states, no graph deps (everything hand-rolled — ours by necessity AND design). **GraphKit spec (guide §4): five layers over existing bones** — GraphProvider (one substrate, discriminator fixed, selectors), primitives (GraphCanvas/NodeCard/**CrossLinkStrip**/ModalityWheel/HonestState), navigation extended, the indicator set (TaskStateIcon+StepPips, ConstructionPulse, LoopPulse, PeerState, GateState — real captured values only), engine LENSES for all 27 + workspace shell revamp (lens tabs over one substrate). Migration: 8 edit-only-compatible steps, first two tiny (discriminator fix unblocks 21 modalities). NO UI edits made; awaiting operator go. CHECKLIST entry "2026-10-23 (zcode, UI pass)".
>
> **LATEST 2026-10-23 (zcode, latest) — WAVE 4 EXECUTED + WIP COMMIT `b08cfd8`** (edits only, no builds/tests; "nothing uncommitted" honored): zcode took the ONLY unclaimed file (src/grpc/mod.rs) and completed its lane — byte-slice panic fix, /config/get api_key redaction with set-side sentinel (round-trip safe), reserved-id guard on /pipelines/register (remote can no longer shadow built-ins). cc's 6 edit-forks ALL died at rate-limit mid-flight; their partials are reviewed, coherent, and committed: trace.rs written, mcp_graph bounding complete, supervisor code-graph facts + tools_dir order, call_global no-panic, C1 consolidation started. **TWO COMPILE BLOCKERS in cc's lanes (do not build until cleared)**: orchestrator/mod.rs BlueprintStep region scrambled + 5 unclosed braces (~925–1010), and 3 declared-but-unwritten modules (instrumented/entry_gate/run_spans). Full state + remaining-wave list: CHECKLIST entry "2026-10-23 (zcode, latest)".
>
> **LATEST 2026-10-23 (zcode, later) — FINALIZED DESIGN REVIEW captured (CHECKLIST entry "2026-10-23 (zcode, later)"): the "everything reruns from the orchestrator + gets an AMT?" question answered — trace spine, not AMT-per-call, not orchestrate-rerouting.** Verified: nested-call capture = 2,319 S12 rows with NO run/parent id (measured — the untraceability is real); 45% of all host call rows are AMT-building (AMT-per-call cost-infeasible); file_link re-enters /pipeline/execute over HTTP from inside a walk (rerouting would recurse); no env_clear anywhere → env-passed OZONE_TRACE_ID reaches all 19 modality binaries + their nested spawns with ZERO per-call-site edits; task_local precedent exists (gate.rs:425 CALL_PRIORITY); CONSCIOUSNESS_ROOT_ID=5. Design: TraceCtx task-local minted at the 5 roots (orchestrate, loop passes, pipeline-execute, mcp-call, rerun_step); gated-entry decorator (DecisionReviewExecutor pattern) on the 3 external entries = jurisdiction without the walk; coarse Run/span containers (per blueprint step, per loop pass — NOT per-call: ~3x graph cost, and spans must not carry project keywords or they trigger ripple sync); NO global consciousness AMT (catch-all anti-pattern already proven by the Global Findings dedupe bug). The AMT stays the semantic structure of requests; trace_id stamped on AMT/blueprint/task containers + every capture row = two-way call↔run lookup. Consolidated 8-layer build order in the entry; nothing applied (read-only).
>
> **LATEST 2026-10-23 (zcode) — PERIMETER + GATE-SEMANTICS REVIEW captured (CHECKLIST entry "2026-10-23 (zcode)"): cc's completed 7-fork security sweep spot-verified. VERIFIED critical: /pairing/start returns its approval code to its own unauthenticated caller (sessions self-issuable — /pipeline/execute's session gate is guarded by a door anyone can mint); CORS allow_origin(Any) makes every loopback route browser-reachable; remote pipeline registration shadows built-ins (remote-first dispatch, no reserved-id guard — registering id 9 redirects every prompt call); blueprint model_override merges api_endpoint/bitnet_cli_path onto pipeline 9 and bitnet_cli_path RUNS as an executable (model text → arbitrary binary); UI chat hardcodes consciousness_enabled:false; a multi-chunk review passes on ANY single judged chunk (unjudged chunks fail open; all-unjudged → ReviewPending, which has NO resolve route); grpc:1174 byte-200 slice = live panic site the orchestrator's char-boundary fix missed; stage-14 discards its own RequireConfirmation matches; /config/get leaks api_key (no skip_serializing anywhere). What held: pipeline-execute session+category rejection, the interception (stub dead), safety-first merge, bidirectional P2P gate (unbuilt tree). NOTHING applied (no-edit ruling) — the endorsed first edit-wave when the ruling lifts: perimeter (CORS/pairing/reserved-ids/config redaction/session on mutating routes) → model_override whitelist → gate semantics. Full evidence + fork-reported-unverified list in the CHECKLIST entry.
>
> **LATEST 2026-10-22 (zcode, late) — GATES + GUARDRAIL-BYPASS + ACTING-LOOP review written (CHECKLIST entry "2026-10-22 (zcode, late)"), done directly because cc's 7 review forks all rate-limited. CORRECTION to the entries below: the consciousness `apply_config` copy is necessary but NOT sufficient — task/mod.rs carries its OWN file-based pre_task_gate/post_task_experience, gated on `TaskQueueConfig.consciousness_enabled` (default FALSE, task/mod.rs:75) reading a nonexistent config.json, and nothing calls the store's trigger methods — so the assistant's valence stays 0.00 even after apply_config; the real fix is an operator decision (task lifecycle calls the store, or one of the two copies dies). Also verified: start_queue_processor has ZERO callers (queued tasks never execute — a guardrail by accident); global findings are silently dedupe-dropped (one shared "Global Findings AMT" container); I-Loop starves by design collision (amt_loop handles first); stage-12 logs success over `let _ =` drops; /pipeline/execute DOES enforce session + consciousness-category rejection (not a bypass); the EU instance (active, binary 10-21) runs all loops on schedule but has ZERO candidates ever, no actor lines, security-mcp live-but-uncalled with a 30s supervisor spawn-churn, P2P up-but-unpaired — monitors, never acts, for four compounding causes. The full acting-path wiring map (8 connections) + consolidated priority order are in the CHECKLIST entry. No edits, no builds, no tests.
>
> **UPDATE 2026-10-22 (cc, latest) — EDIT-ONLY FIX WAVE APPLIED, UNBUILT/UNTESTED:** consciousness TOML→store config copy (emotion path goes live on first boot — config.toml already enables it); blueprint persistence repaired (root cause: flat JSON could never deserialize into Container, so blueprints were never persisted; now body-file + typed container, zero-step reuse falls through, prior-design reader + pipeline-21 `prior_design_context` field wired — koi re-test is the acceptance); Jurisdiction Full Gate is now stage 14 (UI STAGE_ORDER updated); P2P trust gate now bidirectional + batch-sync driver + `peer_sync_candidates.jsonl` bookkeeping; `due_at` in /order/global and queue tie-break; shared-context `context_summary` shows host events + `ForkOf` edges on mirrored events; 23 modality placeholder `object_store_path`→null, control fabricated p-value removed, chrono+capture added to control/electromagnetic/geospatial. Full detail and the remaining-open list: `CHECKLIST.md` entry "2026-10-22 (cc) EDIT-ONLY FIX WAVE". **Nothing compiled; the corrections block below is superseded for those items once the build pass confirms them.**
>
> **VERIFIED 2026-10-22 (zcode) — the EDIT-ONLY FIX WAVE above was diff-verified fix-by-fix this pass (CHECKLIST entry "2026-10-22 (zcode) VERIFICATION PASS"):** all six areas real and correctly wired; every compile-risk spot cc flagged eye-checked, none a blocker (manual `impl Default` blocks exist at types/container.rs:160/188/218 — my own derive-list concern was a false alarm; TOML config struct field names match `apply_config` 1:1; `BlueprintStep` already in stages.rs scope; `queued_due_at` computed before the inputs move; the p-value has exactly one construction site, updated). Residuals carried unchanged: blueprints parent to 0; unused `state` param; raw `broadcast(topic,..)` ungated but dormant (zero callers); `OZONE_ZSEI_DATA_DIR`/cwd-relative body paths; `let _ =` updates leave honest-null silently; depth/hyperspectral/IMU fabricated-value reviews open. **Still nothing compiled** — the build+test+fleet pass remains the acceptance gate for BOTH waves in the tree; then the koi re-test (blueprint acceptance) + the first non-0.00-valence digest (consciousness acceptance).
>
> **UPDATE 2026-10-22 (zcode) — read this first; everything below, including cc's same-day entries, is prior.** Full detail: `CHECKLIST.md` entries "2026-10-22 (cc) FULL STATUS CAPTURE" + its ADDENDUM, and "2026-10-22 (zcode) FULL STATE CAPTURE v2" (the verification pass — every claim below is grounded there). Summary:
> - **Now**: local host DOWN (only the shared-context MCP node processes run); disk 98% (5.7G free — below the ≥10G build rule, so NO build was run); last commit `960902a`; **40 uncommitted/untracked entries carrying one coherent, unbuilt feature wave**; EU VPS status unverifiable (loopback-only binding is by design; no SSH tunnel running).
> - **The uncommitted tree, verified file-by-file this pass** (previously "+1662 lines, content unexamined"): node-event producers in all 24 modality pipelines (fire-and-forget → `/orchestration/node-event`); the complete P2P wave in `src/network/mod.rs` (durable default-deny `paired_peers.json` trust gate, loopback `listen_on`, bounded Swarm event loop, 86400s idle timeout with empirical justification, gossipsub subscribe + paired-gated Message handler); 4 `/network/*` routes + the node-event route + **on-demand MCP start in `/mcp/call`** (widens WHEN an opted-in tool starts, never WHETHER) in `src/grpc/mod.rs`; supervisor facts for `p2p-bridge-mcp` (port 3278, gated on `enable_p2p`); `auto_broadcast_sync` config (default false) + the boot-gated sync bridge in `lib.rs` + the `Arc<RwLock<>>` signature fix in `amt_loop.rs`; UI: orchestration timeout-recovery poll (recovers real results the client timed out on, e.g. the 585s task), ThreeDEngine as first node-event consumer, SetupWizard P2P disclosure screen; `tools/p2p-bridge-mcp/server.mjs` (locked-by-default SSH-bridge MCP, unregistered in the MCP index, untested).
> - **Compile status: UNVERIFIED — the tree has never been built as a whole.** One build+test+fleet pass lights up the entire wave in one boot; it stays operator-gated behind disk + commit + relaunch (exact order in the CHECKLIST entry §6).
> - **Wiring map (how it all works together)**: one write choke point (`ZSEI::query` → `graph_events::emit`); two event streams — graph events (the reflex nervous system: ripple actors, AMT wake, I-Loop, assistant, P2P bridge, 12 UI consumers) and orchestration events (stage transitions + per-node construction events + pipeline locations; ThreeDEngine consumes node events); four real thinking/consciousness capture layers — in-walk `ThinkingEntry` raw outputs on the task record, S10–S13 audit stores (consciousness review reads them), the background loops' persisted candidate/insight stores, and the shared-context coordination record. Orchestration reads top-down (jurisdiction → methodology → AMT → blueprint → sim → gates → steps), traversal reads bottom-up (keyword seed + provenance-labelled one-hop + prior-design injection + session replay); `[13.link]` supplies edges; **content-level cross-walk transfer is still the depth gap**.
> - **UNVERIFIED → resolved this pass**: `find_methodologies_by_keywords` hop expansion present in tree (query.rs ~613–670, 12-neighbor cap); wave-barrier redundancy CLOSED (zero `.chunks(` sites remain in the orchestrator); node-event consumer count precise (ThreeDEngine only — `graphEventClient.ts` serves 13 files but two distinct streams).
> - **Redundancies**: unchanged ten-item ledger minus #7 (closed above); new instance of the "two books" pattern — `p2p-bridge-mcp` known to the supervisor, absent from the MCP registry index.
> - **Still not wired (honest list)**: P2P content replication + content-review gate (guide §7.3 designed, not built), auto-redial, `known_peers` persistence, disconnect route; `p2p-bridge-mcp` registration/test; clarification generation (no constructor); `PipelineLocationEvent` producers (receivers exist); `depth` producer (no reqwest dep); ordering policy (opt-in vs forced `/mcp/call` envelope); raw-data modality live tests; FileRef legacy backfill; duplicate math↔text edge dedupe (operator-gated).
>
> **CORRECTIONS 2026-10-22 (zcode, later same day — after independently verifying cc's 12-fork review; full evidence in the CHECKLIST entry "2026-10-22 (zcode) CROSS-CHECK CAPTURE"):** three lines above are superseded:
> - ~~"content-level cross-walk transfer is still the depth gap"~~ → **structurally DEAD, one root cause, three symptoms**: blueprint persist drops `storage.steps` through `serde_json::from_value::<Container>` (adapters.rs:208, unknown-field default), `load_prior_design_context` (stages.rs:314) therefore finds only step-less containers, and pipeline 21 has no `prior_design` field at all (0 hits) so anything sent is dropped — the same defect also explains "Using existing blueprint" succeeding on zero loaded steps and the koi test's missing plan text. It is a bug to fix (persist shape + reader + pipeline-21 field + koi re-test), not a retrieval design gap.
> - ~~thinking/consciousness capture "four real layers ... none fabricated"~~ → the capture infrastructure stands, but the **emotion trigger layer is inert on this host**: `CONSCIOUSNESS_STORE` initializes `enabled:false` and nothing copies TOML `consciousness` config into it at boot (only setter: store.rs:168, never called at boot; gates at store.rs:481/488/567 early-return). The 2026-10-04 "emotion near-default = derivation tuning?" question is answered: the gate was never fed. Fix ≈2 lines at lib.rs:877 next to `set_store`. PRIORITY 1.
> - P2P additions: the trust gate is **receive-only** — `broadcast_high_priority` (network/mod.rs:696) sends to every `known_peers` entry with no paired check, and `process_batch_sync` has zero callers (batched Normal-priority findings queue forever). Add an outbound paired-only filter before content ever rides the wire.
> - Also confirmed this pass: stage 8/8b collision (jurisdiction.rs:431 + stages.rs:95 both record stage 8 — UI rows overwrite); dual tool-selection (`applicable_tools` built, only length logged; prompts use `capability_summary`; zero orchestrator MCP-call consumers — decision per directive #46); placeholder `object_store_path` ×5 (biology/CAD/audio/BCI/control) + a hardcoded placeholder statistic (control:2901); decision-review writes no ledger row (S11-only; by-design-or-gap undecided); hygiene: 2 unpushed commits, `tools/bridges` gitlink with no `.gitmodules`, `target/debug` regrown under pipeline crates. Recommended order: config copy → disk+commit+relaunch+one build pass → 8b fix → P2P outbound filter → blueprint repair wave + koi re-test → tool-routing ruling → due_at ordering (CHECKLIST §7).

> **ADDENDUM 2026-10-22 (cc):** see the second `CHECKLIST.md` entry "ADDENDUM to the FULL STATUS CAPTURE" for the doctrine-corpus coverage map, interlock rules and carried open items (E-series expansions, clarification generation, P2P content gate, redundancies).

> **UPDATE 2026-10-22 (cc) — read this first; the 2026-09 material below is history.** Authoritative detail: `CHECKLIST.md` entry "2026-10-22 (cc) FULL STATUS CAPTURE". Summary:
> - **Now**: local host DOWN (relaunch from `target/release/`); disk 98% (5.7G free); ~37 files / +1662 lines UNCOMMITTED and unbuilt since the P2P/trust-gate/node-event work.
> - **Orchestration**: 14-stage walk (0→13, 8b jurisdiction full gate), detached from client, never-zero methodologies, tools gathered at Stage 3, `[13.link]` edge supply, seed→hop + prior-design injection. Cross-walk is structurally connected; content-level transfer still partial.
> - **Traversal**: single write choke point (`ZSEI::query`→`graph_events::emit`) drives loops, ripple actors, AMT wake, node-events, UI. Reads: keyword seed + one hop + blueprint-content injection + session replay.
> - **P2P**: local↔EU proven live; trust gate (`paired_peers`, approve/revoke) built and A/B-verified; sync carries a *finding signal only* (no content replication); auto-broadcast opt-in; no redial/persistence/unpair; `tools/p2p-bridge-mcp` untested.
> - **Shared context**: stdio MCP is agent-local; host mirrors into `/SharedContext` but cannot call the MCP; ordering (insight envelope) is opt-in; CoordinationEvents carry no relationship edges.
> - **Redundancies to resolve**: dual config.toml, dual zsei_data islands, dual task systems, dual I-Loop, triplicated linker, stale docs (list in the CHECKLIST entry §4).
> - **Open wiring**: clarification generation, node-event consumers (only 3D), live code-edit producers, legacy FileRef backfill, nested pipeline-9 routing, raw-data modality live tests.

> **UPDATE 2026-09-20 (zcode, session 4-5)** — read the 2026-09-17 base below, then these deltas:
> - **Jurisdiction**: root 7 deduplicated (231 scaffolds deleted, user-approved); canonical wiring = MIN container id (child_ids order reshuffles across boots — only min-id is stable); convergence proven (wired 21 → 0).
> - **Knowledge base**: 44 methodologies (35-44 new, real session lessons incl. method_44 Session Token Metering), 17 blueprints (8-17, real executable plans with pipeline ids + methodology configs). Registered in bootstrap defaults + both stores.
> - **BitNet coherency** (forced through pipeline 9): deepening / meta-loop draft / keyword extraction fully coherent (60-84s per call, correct schema, no placeholders); zero-shot sim returns valid JSON with wrong schema (step_N keys) — schema-flexible stage-7 parsing is the open follow-up.
> - **Loops**: both meta-loops have balanced-scan JSON capture, placeholder-content gates, fallback escalation (all call sites audited), 10-attempt honest caps. The 3 metering gaps are open and drafting under these gates.
> - **Known numbers**: linking keep-alive 80x (13.1s→199ms); AMT deepening arc completed live (7 attempts, BitNet fallback); one unexplained host crash 2026-09-20 ~07:40 AST (no panic captured — watch for it).
> - **Open, user-gated**: VoiceConfig fields; 24h meta-loop interval decision; crash-cause terminal capture if it recurs.


> This is the authoritative roadmap. Every item is verified by a test, a live
> check, or a code read — never a claim. Cross-referenced with
> CHECKLIST.md (CC's historical detail), docs/SESSION_HANDOFF.md (CC's
> handoff), docs/GRAPH_TEST_PLAN.md (test contract), and
> docs/LIVING_GRAPH_STATUS.md (graph doctrine-vs-reality).

---

## WHAT'S LIVE AND VERIFIED

### Core infrastructure
| System | Status | Proof |
| ------ | ------ | ----- |
| Host binary (release) | Running on newest build, all fixes | Boot log + /health |
| ZSEI graph (mmap + plain-file) | Both modes byte-identical, cross-mode proven | storage::tests (3) |
| Cache coherency | Writes invalidate target + parent cache | Fixed + tested |
| child_ids persistence | rebuild_child_ids_cache at boot | Fixed after 299-duplicate finding |
| Structural-root id floor 1000+ | Dynamic ids never overwrite roots | Confirmed across boots |
| Jurisdiction graph | 41 scopes, 37 relationship edges, re-wired every boot | Boot log + live query |
| Methodology store | 34 entries, ALL with content files, 0 phantoms | Index + disk audit |
| Blueprint index | 7 entries registered | Boot log |
| Pipeline registry | 82 pipelines loaded (55 compile-time + 27 seeded) | Boot log |
| Graph ripple | Every write publishes scoped events | Live: ws + monitor feed |
| Context objects | Per-step context_assembled + context_sources on task records | Task 72 verified |
| Task lifecycle | /task/create + /task/update, source-tagged reconciliation | Live: 56/58/59/62 closed via API |
| Coordination graph | /SharedContext: 80 children, both agents mirroring | Live count |
| MCP surface | /mcp/call standardized + usage ledger + gate | Live: delegated + metered |
| Shared-context tool | v0.3.0, 12 tools, self-registers on boot | Live: registered + calls |
| 3-way coordination | ZCode + CC + Ozone-Studio sharing scoped graph | Live: both agents present |
| 12 bootstrap methodologies | Real content, deployed to both stores | Disk audit |

### Verified live this session
- Section-3 re-test: auto-route ✓, graph creation ✓, main generation walked
  fallback chain (6 distinct real models) ✓, needs_clarification false ✓
- Model fallback chain: 6 distinct real models across one request,
  model_used reports true routing (not a static value)
- Cache coherency: write→immediate read returns new state (was stale before)
- Plain-file storage: round-trip + in-place update + cross-mode proven
- Coordination task preservation: all coordination tasks survive restarts
  as queued (source-tagged reconciliation fix)
- Methodology phantom resolution: 12 bootstrap entries filled with real
  content, deployed to both stores

---

## ALL BLUEPRINTS NEEDED

The zero-shot simulation gathers methodology rules → identifies pipelines →
creates blueprint steps. Currently everything coerces to pipeline 9 (the
prompt pipeline). The system needs real multi-step blueprints for each
workflow class:

### Existing (old schema — needs migration)
| # | Name | Real content? | Target schema ready? |
| - | ---- | ------------- | -------------------- |
| 1 | General Assistant | YES (rich) | Needs BlueprintStep migration |
| 2 | Code Review | YES (rich) | Needs migration |
| 3 | Documentation | YES (rich) | Needs migration |
| 4 | Code Creation | YES (rich) | Needs migration |
| 6 | Data Analysis | YES (rich) | Needs migration |
| 7 | Mathematical Proof Verification | YES (rich) | Needs migration |

### New blueprints needed
| Name | Pipeline chain | Status |
| ---- | ------------- | ------ |
| Modality Processing | text(100)/code(101)/math(102) → context_aggregation(21) → link | Not started |
| Jurisdiction Compliance Check | jurisdiction rule load → gate check → report | Gate exists, blueprint needed |
| Research + Synthesize | web_search(56) → prompt(9) → context_aggregation(21) | Search pipeline built, no key |
| Multi-Model Pipeline | classify → route to bitnet/api → aggregate | Fallback chain works, blueprint needed |
| Cross-Modal Analysis | text(100) + code(101) → cross-link → aggregate | Both pipelines exist |
| Coordination Sync | mirror → AMT candidate → expansion | Ripple sync built |

### Blueprint schema migration
Old: `{step_id, name, description, pipeline_id, action, outputs}`
New: `{step_index, action, description, pipeline_id, context_requirements,
       depends_on, max_retries, wait_for_graph_update, model_override,
       source_chunk_indices, methodology_ids_applied}`

The 100%-match reuse path (stages.rs:171-182) is real but unreachable
because: (1) old-schema content doesn't deserialize, (2) assets/blueprints/
doesn't exist so fresh instances never receive them.

---

## PIPELINE IDENTIFICATION (zero-shot simulation)

### Current state
Stage 4 (zero-shot simulation) gathers methodology decision rules and
identifies pipelines for each blueprint step. But:
- ALL steps are coerced to pipeline_id 9 (prompt pipeline) — CC's fix
- Exception: pipeline 56 (web search) is carved out
- Modality pipelines (100 text, 101 code, 102 math) exist and work
  when called directly, but are never selected during blueprint creation
- The LLM authors blueprint steps but only pipeline 9's schema matches

### What's needed
1. The blueprint step schema should support modality pipeline selection
   when the step's content matches a modality (text/code/math)
2. The coercion to pipeline 9 is correct for the general case but
   should be extendable: a step tagged `pipeline_id: 100` should
   dispatch to the text modality pipeline when the step's input
   carries text analysis data
3. The AMT branch reconciliation already populates
   `source_chunk_indices` and `methodology_ids_applied` — these should
   inform pipeline selection for modality-specific branches

---

## JURISDICTION — user configuration needed

The jurisdiction gate is real and always-on: 41 scopes, 37 relationship
edges (EU/regional hierarchy), real enforcement with Block /
RequireConfirmation / Warn actions.

### What exists
- global.json: 7 baseline rules (UDHR/CRC cited)
- ~54 national files: real search-and-cite content across ~26 countries
- Source registry: SOURCE_REGISTRY.json/md aggregating all citations
- Region auto-detection: timezone/locale/IP 3-signal cross-check
  (confirmed: DO via explicit config + hardware detection both work)
- Enforcement: Block (real), RequireConfirmation (runs decision_gate
  pipeline 39), Warn (pushes real surfaced warning)
- `OrchestrationResponse.jurisdiction_gate` now reaches the API caller

### What the user needs to configure
1. **Region**: config.toml `jurisdiction.instance_region` — currently
   "DO" (Dominican Republic) via explicit config
2. **Enforcement actions**: which rules Block vs Warn vs
   RequireConfirmation — the global baseline is configured, national
   content is all Log-only pending user review
3. **National rules for DO**: Dominican Republic national content
   exists but is Log-only — user should review and elevate actions
4. **Scope rules**: global vs regional vs national — the graph edges
   define inheritance (national → regional → global baseline)

### Remaining jurisdiction work
- [ ] User review of DO national rules — elevate from Log to real actions
- [ ] Remaining thin countries (ar bd do ec id ke ma my ng no pe ph pk se th vn)
- [ ] Source registry maintenance protocol (CC's fork output needs verification)
- [ ] Jurisdiction content for new scopes as the user's needs expand

---

## ALL REMAINING WORK (routed through the task system)

### ZCode queue (mine)
| Task | What | Blocked on |
| ---- | ---- | ---------- |
| 69 | Fix steps-0: context objects through /orchestrate (Stage 7 hook fires before step_contexts exists; move to execute_step) — **fix built, rides next restart** | Restart |
| 70 | Re-run section-3 test verifying ModalityGraphs populate AND context objects persist | 69 |
| 43 | Coordination layer into ForStep: verify coordination_context appears in real orchestrate step context | 69 + restart |
| 45 | First real stage McpCall consumer | A real tool need |
| 60 | Cross-process + integration test batch | 57/68 |
| — | Merge-back v2: fork grafts into main AMT | Designed, ready to build |
| — | Deploy url/package binaries (done, verify) | — |

### CC queue (on return)
| Task | What | Blocked on |
| ---- | ---- | ---------- |
| 68 | Nested pipeline-9 extraction → fallback walk + graceful degradation — **the section-3 fix** | Nothing — ready to build |
| 44 | CHECKLIST retirement post context-transfer | Transfer proven |
| 46 | Host-ops methodology verification on boot | Methodology 16 (live) |
| 57 | link_to_existing completion | Task 68's fallback walk pattern |
| 47 | Text/code cross-process graph retrieval | Math reference fix |
| 65 | Code modality dependency graph + provisional nodes | Math reference pattern |

### Shared
| Item | What |
| ---- | ---- |
| Merge-back v2 | Graft substantive fork branches into main AMT (designed in docs/AMT_EXPANSION.md) |
| Task 64 | file/url/package links onto real ZSEI graph (FileReference/URLReference/PackageReference containers + Relation edges) |
| Task 65 | Code modality dependency graph + provisional-node lifecycle |

### Open engineering (not yet tasked)
- mmap:false cross-mode index rebuild (deferred w/ test — layouts identical)
- ContextSource provenance field (traversal vs keyword)
- TraversalRequest keyword_filter/topic_filter dead fields
- Network hooks fed from GraphEventHub
- T-G4: ws frames integration test
- Root children count display lag
- StatusBar fabricated fields (CC finding)
- Zero-shot simulation over-asking (fixed, needs re-test under load)
- Blueprint Assignment stage ~580s pattern (root-caused? not yet)

---

## EVERYTHING CAPTURED THIS SESSION

### Built and verified
1. Unified AMT expansion candidate store (amt_candidates.rs) — 3 routes,
   path-injected, deduped, 500-entry cap
2. Main/fork island model — amt-main / amt-fork-of:<prior> keywords +
   Continues relation, sequential lifecycle test
3. Graph ripple — every write publishes scoped events to ws + monitor + AMT
4. GraphEventHub — process-global, broadcast ring 1024
5. Coordination layer in ForStep — workspace_id + include_coordination,
   separate-layer doctrine
6. Context objects — per-step context_assembled + context_sources on task
   records, provenance = keyword-scan today / traversal:<mode> later
7. Task 43 coordination layer — /SharedContext scoped pull into step context
8. Task 45 global McpCall — install_global + call_global + usage metering
9. Task lifecycle — /task/create + /task/update, source-tagged
   reconciliation, restart-preservation proven
10. Storage plain-file branch — mmap:false no longer silent data loss
11. Cross-mode byte compatibility — plain-write → mmap-read proven
12. Task 68 — extraction fallback walk + deterministic tier (CC verified
    the approach; I completed the implementation after the limit)
13. 12 bootstrap methodologies — real content, deployed both stores
14. 3-way coordination — shared-context MCP, scoped graph, both agents
15. Docs sweep — deep dive addendum, CONTRACTS §6, test plan, expansion
    architecture, living graph status

### Bugs found and fixed (this arc)
1. ZSEI hot-cache incoherency — writes bypassed cache → stale reads
2. Absolute object_store_path garbage-join (amt_loop, jurisdiction, amt)
3. Stage 7 hook fired before step_contexts existed → steps-0
4. mmap:false silently no-opped store_global writes → data loss
5. Plain-file write branch wrong stride (32 vs 24 bytes) → misalignment
6. Plain-file init missing header → cross-mode rejection
7. Missing `discovered_via` field on Relation literals (CC found, race)
8. k_registry tests: RwLock-wrapped fields accessed without lock
9. remote_dispatch/executor_l2 tests: missing roles parameter
10. text modality create_graph: orphaned code block from wire-protocol edit

### Bugs found and filed (not fixed)
1. Task 68: nested pipeline-9 extraction lacks fallback walk (CC building)
2. mmap:false cross-mode index rebuild (deferred, layouts identical)
3. ContextSource lacks traversal-vs-keyword provenance
4. TraversalRequest keyword_filter/topic_filter dead fields
5. Root children count display lag after restart
6. StatusBar fabricated placeholder values (CC finding, documented not hidden)
7. /orchestrate unauthenticated (CC finding)
8. Zero-shot simulation inefficiency: executes all steps before
   discarding on clarification

---

## HOW TO CONTINUE

1. **Restart onto newest binary** (all fixes live in one boot)
2. **Run task 70**: orchestrate with attachments → verify ModalityGraphs
   populate + context objects persist + coordination layer appears
3. **CC returns** → picks up 68/44/46/57/47/65 from the tracker
4. **I continue** → merge-back v2, task 45 consumers, task 60
5. **User configures jurisdiction** → reviews DO rules, elevates actions
6. **Iterate**: each cycle lands fixes, closes tasks through the API,
   updates docs additively, and the living graph captures everything

Everything routes through the task system. Everything is observable
through the monitor. Everything is scoped through the graph. Everything
is documented in the docs. Nothing is claimed without a test.


## 2026-10-29 (cc) — MOBILE/ANDROID + SECURITY/TERMINAL MCP ENRICHMENT (guide: `docs/MOBILE_ANDROID_GUIDE.md`)
Operator: review security MCP + logs local & EU, identify mobile MCP, terminal-like Android, ZCode's emulator -> Ozone MCP, enrich security/terminal/mobile, test.
- **Reviewed:** local host.log + security-mcp log; EU over SSH (read-only): EU sensor tick HEALTHY (133 events, all success, EU unit has CAP_NET_ADMIN, real ufw rules) — zcode's "0 sensor events" was a log-only-failures artifact. Local `firewall_status` failed every 15 min (unprivileged) feeding the I-Loop.
- **security-mcp:** unprivileged firewall = success + `inspectable:false`; new `exposure_report`, `security_status`. Selftest 5/5.
- **terminal-mcp:** closed interpreter/arg bypass of the first-token allowlist (node -e, python3 -c, find -exec, git -c, npm exec, systemctl restart…). Selftest 18/18 (new file).
- **android-mcp:** 7 -> 18 tools; shell locked-by-default allowlist + deny verbs + token; preflight; AVD/emulator (KVM-gated, opt-in); ui_snapshot/tap_element; term_* sessions; forward/Termux; refusals now HTTP 200 + reason. Selftest 17/17 + 1 SKIP (no device) against REAL adb 37.0.1 (installed ~/Android/platform-tools).
- **Registered on the real local host:** 16 android + 2 security tools; verified through `/mcp/call` (gate not blocked, ledger counted). New launcher `target/release/launch-ops-mcps.sh`.
- **ZCode android-emulator plugin:** ZCode-private (22 tools, needs SDK/JDK/Gradle); capability list absorbed, plugin not wrapped.
- **NOT done / unverified:** no emulator or phone run (no SDK/JDK; RAM tight; /dev/kvm IS usable); EU deploy of the new security/terminal code; next 15-min sensor tick to prove the I-Loop echo stops.

- **EMULATOR RUN (same day):** user-space SDK+JDK17+AVD via `tools/android-mcp/provision_sdk.sh` (4.8 GB); KVM headless boot ~65 s; started through `/mcp/call`. `selftest.mjs emulator-5554` **22/22, 0 skipped**; live perceive->tap->verify on the Messaging app; screenshots `docs/evidence/android-mcp/`. Fixed `termux_status` (pm path exit code). ZCode plugin relationship: separate layers sharing one SDK (guide §3b). Emulator stopped afterwards to free RAM.
