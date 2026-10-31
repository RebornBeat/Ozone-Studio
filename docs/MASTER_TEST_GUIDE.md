# Master Test Guide — everything, end to end (2026-10-29, cc)

> Operator ask: capture one finalized guide that tests **everything** — CLI, host routes, orchestration, graphs and context
> flow, every modality and file type, workspaces (different workspaces, different files), every MCP and cross-MCP capability
> (including Blender 3D creation and handling), the acting loop and watchdogs, UI, mobile, P2P, security, EU/VPS — "all
> connections, interconnections, cross-features, no omissions."
> This guide **consolidates and extends** `GRAPH_TEST_PLAN.md` (T-ids), `TOP_DOWN_REVIEW_GUIDE.md` (TD-1..3, bp_17),
> `MCP_TESTING_AND_COORDINATION_GUIDE.md` (per-MCP contract), `REVIEW_WATCHDOG_2026-10-07.md`, `NEW_MCP_GUIDE.md` §6,
> `ACTING_FACTORS_REGISTRY.md` (what acts) and `MASTER_REMAINING_WORK_GUIDE.md` §W7 (workspace plan). It does not replace them;
> where a T-id already exists it is referenced, not copied.
> **Nothing here was run as part of writing it** except the rows marked ✔ with their evidence. Every other row is ☐ (pending).

Legend: ✔ verified (date + evidence) · ☐ pending · ⚠ known defect to reproduce · SKIP(reason) · `[R]` needs the real substrate (no mocks).

## 0. Principles (binding)
1. **Real substrate only** (methodology 49): real host, real Chrome/UI, real adb/emulator, real farm/VPS. A mock may exist only
   as a labelled contract test; it never closes an acceptance row. Absent prerequisite ⇒ `SKIP(<what is missing>)`, never PASS.
2. **Captured facts only**: assertions read real registrations, edges, persisted bytes, ledger rows, capture lines — never a
   narrated result. Numbers are copied from the run, not remembered.
3. **Every MCP row has 5 checks** (NEW_MCP_GUIDE §6): positive · refusal (clean, HTTP 200 `success:false` + reason) · ripple
   (a CoordinationEvent container appears) · S13 capture row (`/capture/tool-calls?tool=`) · registry endpoint == real port.
   All through `/mcp/call`, never only direct.
4. **Resource-safe order** (box: 7.6 GB RAM, ~13 GB disk): never two cargo jobs; heavy MCPs (connectome, cerebrix, visual,
   threed, emulator) one at a time with `free -m` before/after; stop the emulator when idle; builds need ≥10 GB free disk.
5. **Process hygiene**: kill by PID from `ps -eo pid,comm,args | awk '$2=="node"…'` — never `pkill -f`/`pgrep -f` patterns that
   match your own shell (twice cost a shell). No background child of an agent session is durable: start long-lived MCPs from the
   operator shell or the launchers (`target/release/launch-ozone.sh`, `launch-browser-stack.sh`, `launch-ops-mcps.sh`).
6. **No credentials in artifacts**: scrub `config.toml`, tokens, `.internal_token`, API keys from any evidence file.
7. **Evidence**: every run writes `docs/evidence/test-runs/<YYYY-MM-DD>-<suite>.md` (command, exit code, counts, paths to
   screenshots/ledger rows). A suite is green only with that file.

## 1. Environment preflight (L0) — run first, record all
| ID | Check | Command / source | Expected |
|---|---|---|---|
| P1 | CLI health | `ozone-studio doctor` | config, data dir, pipelines, port all OK |
| P2 | Host up | `curl :50051/health` | `healthy:true`, version, peer_id |
| P3 | Resources | `free -m`, `df -h /` | ≥2 GB available, ≥10 GB free before any build |
| P4 | Strict-session posture | `OZONE_REQUIRE_SESSION`; `GET /mcp/tools` w/o token | 401 on PROTECTED routes; `/mcp/call` & `/pipeline/*` reachable |
| P5 | Internal token | `<data_dir>/.internal_token` exists; registrar works with it | register OK (needs the rebuilt binary) |
| P6 | Binary currency | `strings` for `x-oz-internal`, force-sync, I-Loop budget | all present, else deploy (W0) |
| P7 | Port map | `facts_for_port` vs `tools/*/server.mjs` defaults vs `TOOLS_REGISTRY.md` | no duplicates (browser 3280, farm 3281, code-graph 3279 absent from facts ⚠) |
| P8 | Registry persistence | restart host, `GET /mcp/tools` | same tool set, same endpoints (⚠ 24 browser/farm endpoints were patched on disk; re-register + verify) |
| P9 | Shared context | `context_summary` | presence/claims/notes readable; `host_events.available:true` when host up |

Port table (source of truth = `src/mcp_supervisor.rs`): bridges 3210 · terminal 3215 · visual 3220 · web 3225 · code 3230 · git 3235 ·
security 3240 · gcal 3241 · cerebrix 3245 · threed 3250 · connectome 3255(+3256) · re 3260 · zoom 3265 · dna 3271 · chemistry 3272 ·
proteomics 3273 · geospatial 3274 · mesh 3276 · android 3277 · p2p-bridge 3278 · code-graph 3279 · **browser 3280 · browser-farm 3281**.

## 2. Build and unit/regression layer (L1)
| ID | Test | Command | Status |
|---|---|---|---|
| B1 | `cargo check` clean on HEAD | `cargo check` | ✔ 2026-10-29 (zcode: exit 0 on 18607ae); re-run after `bootstrap.rs` methodology edit ☐ |
| B2 | Library tests | `cargo test --lib` (155 `#[test]` in src; GRAPH_TEST_PLAN reports 59→63+ green) | ☐ record the real count |
| B3 | Modality crates | `cargo test` in `assets/pipelines/modalities/{text,code,math}` (T-T3 10/10, T-C3 7/7, T-M2 4/4 recorded 2026-09-19) | ☐ re-run |
| B4 | UI typecheck + build | `cd ui && npx tsc --noEmit && npx vite build` | ☐ (1 legacy tsc error in chatRendering noted by zcode) |
| B5 | MCP selftests | `node tools/<mcp>/selftest.mjs` | see §7 — security 5/5 ✔, terminal 18/18 ✔, android 22/22 ✔ (emulator), browser 18/18 ✔ |
| B6 | Methodology load | boot, `GET`/ZSEI shows methodologies 45–50 | ☐ (index edit uncompiled) |
| B7 | Jurisdiction content honesty | T-J4 (132 files / 648 rules; `national/` all `Log`; global.json exempt) | ☐ re-run on expanded content |

## 3. CLI (L2)
| ID | Test | Expected |
|---|---|---|
| C1 | `ozone-studio version` | prints version |
| C2 | `ozone-studio doctor [--config f]` incl. bad config | reports real failure, non-zero exit |
| C3 | `serve --port N --data-dir D --host 127.0.0.1` | binds override; second host on other port/data-dir coexists |
| C4 | `mcp list` against running host | tools with endpoints; equals `/mcp/tools` |
| C5 | `mcp autostart <tool> on|off`, `mcp start/stop <tool>` | process starts/stops; supervisor state matches; **device-gated (android) refuses without device** |
| C6 | `--host` / `$OZONE_HOST` targeting (local vs EU) | commands hit the chosen host |
| C7 | Bare `ozone-studio` = `serve` | same behaviour |
| C8 | Refusal paths: unknown subcommand, missing arg, host down | clean message, non-zero exit, no panic |

## 4. Host HTTP surface (L3) — every route, positive + refusal + auth
58 routes in `src/grpc/mod.rs`. Run each with: no session, bad session, valid session, internal token; malformed body; oversized
body. Group and expectations:
- **Health/auth/pairing**: `/health`, `/auth/challenge`, `/auth/authenticate`, `/pairing/start`, `/pairing/status`, `/pairing/approve`,
  `/pair/:id`, `/devices` — full pairing round-trip (code read from `pairing_pending/<id>.code`, token issued, Bearer works,
  revoked token fails). ✔ pairing flow exercised 2026-10-29 by zcode (device 1).
- **Pipelines**: `/pipeline/registry`, `/pipeline/ui-component`, `/pipeline/execute`, `/pipeline/progress`, `/pipeline/cancel`,
  `/pipelines/register|unregister|remote` — execute needs a session token; ui-component does not.
- **Orchestration**: `/orchestrate` (⚠ no session validation per MASTER W9), `/orchestration/node-event`.
- **Tasks/Order**: `/task/create|update|get|list|cancel`, `/task/step/rerun`, `/order/global`, `/assistant/feed`.
- **Consciousness**: `/consciousness/reviews/pending|resolve`, `/consciousness/review_pass`.
- **Graph/store**: `/zsei/query` (create/get/update/delete/link/search), `/cross-modal-pairs`, `/ws` (graph_event frames — T-G4).
- **Capture**: `/capture/decision-reviews` (S10), `/capture/zero-shot-calls` (S11), `/capture/tool-calls` (S13), `/monitor/activity|summary`.
- **Network**: `/network/connect-peer|approve-peer|revoke-peer|paired-peers`.
- **MCP**: `/mcp/tools`, `/mcp/tools/register|unregister`, `/mcp/tools/:name/autostart|start|stop`, `/mcp/usage`, `/mcp/call`
  (⚠ trusts self-declared `agent`), `/context/mirror`, `/coordination/presence|claims`.
- **Config/quota**: `/config/get|set`, `/quota`, `/model-ledger-summary`.
Pass criteria: documented status codes, no 5xx on malformed input, secrets never echoed, session gate exactly as in P4.

## 5. Orchestration and gates (L4) — the 14-stage walk
| ID | Test | Expected evidence |
|---|---|---|
| O1 | Plain chat prompt end-to-end | all stages summarized; response; S11/S12 rows with run/trace ids |
| O2 | Multi-intent prompt | one AMT step per branch, no collapse (T-A1) |
| O3 | Jurisdiction **Block** | stage-0 early stop; `jurisdiction_gate.blocked:true` in API response |
| O4 | **Warn** | warning surfaced in response |
| O5 | **RequireConfirmation** (DO/EU personal-data rules) | real decision_review; Decline blocks; Proceed passes; failed call ⇒ `ReviewFailed` fail-open visible; total failure ⇒ `ReviewPending` |
| O6 | Consciousness gate stage 8 | review prompt contains AMT + blueprint + SIMULATION + jurisdiction (TD-2) |
| O7 | Confetti/empty model output | same-model retry ×2 then fallback walk then `ReviewPending`, never silent pick |
| O8 | Model fallback walk + free-only + BitNet backstop | capture rows show which model answered |
| O9 | Adaptive context chunking (§10 of TOP_DOWN) | chunk index/total in call_site label; Decline-wins merge |
| O10 | Strict session on `/orchestrate` ⚠ | document current behaviour; decide |
| O11 | Memory gate: run with low MemAvailable | walk refuses loudly, host survives |
Blueprints to run: bp_17 (Fallback & Capture Verification); **TD-1** regulated plan review, **TD-2** consciousness full-context,
**TD-3** switch coherence (cloud vs BitNet on identical traversed context).

## 6. Graphs and context flow (L5)
All `GRAPH_TEST_PLAN.md` ids stay authoritative. Status snapshot from that doc (2026-09-22) and what to (re)run:
- ✔ T-J1..J5, T-A2..A5, T-T1/T3, T-C1/C3, T-M2, T-I1..I3, T-CO1..5, T-G1..3, T-U1..4, T-X1 (codified; re-run for regressions).
- ☐ T-A6 lineage (3+ generation fork chain), T-T2/T4/T5/T6/T7, T-XM1..3, T-C2/C4/C5, T-M1/M3, T-G4, T-X2/X3, T-I4..I6.
- ☐ Deep stress **T-S1..S7** (code/math/text density; full-project cross-modal triangle; multi-generation fork; confetti across all
  modalities; S10↔S11 consistency).
**Graphing the context flow (new, make it a repeatable procedure)**
| ID | Procedure | Pass |
|---|---|---|
| G-CF1 | Run one orchestration with a regulated, multi-file prompt. Collect: S10/S11/S12/S13 rows (by `trace_id`/`run_id`), `call_spans`, the AMT container, blueprint, file-graph ids | every captured call has a trace id; every model call row names its context sources |
| G-CF2 | Build the DAG: prompt → stage → call → model → context blocks (S1–S9 from `CONTEXT_REGISTRY.md`) → result → ripple → actor → candidate/task | no orphan node; no call without a stage; counts match across stores (T-XM3, T-S7) |
| G-CF3 | Render it in the 3D world (GraphKit) and in the call-graph lens (when built, W5) | nodes/edges equal the DAG; honest absence where data is missing |
| G-CF4 | Trim visibility (`CONTEXT_OBJECT_MODEL`: "every cut is visible") | any truncated context shows a trim marker |
| G-CF5 | Budget per model (token-aware, per fallback attempt) | derived from the effective model's context length |
⚠ `context_records` is writer-only (no reader) per zcode — add a read route before G-CF2 can use it.

## 7. MCP fleet (L6) — contract per MCP, then cross-MCP
Run each row's five checks (§0.3). Light MCPs may be batched; heavy ones alone.
| MCP | Port | Weight | Selftest | Status / notes |
|---|---|---|---|---|
| terminal | 3215 | light | `tools/terminal-mcp/selftest.mjs` (18) | ✔ 2026-10-29 18/18 (arg-bypass classes); also through host ☐ |
| security | 3240 | light | `tools/security-mcp/selftest.mjs` (5) | ✔ 5/5; through `/mcp/call`: firewall_status unprivileged ⇒ `inspectable:false` ✔, exposure_report ✔; EU privileged rules ✔ (direct) |
| android | 3277 | device | `tools/android-mcp/selftest.mjs [serial]` (22) | ✔ 22/22 on real emulator ozone_api30 (KVM, boot ~65 s); live perceive→tap→verify ✔; Termux/phone/mobile-app ☐ |
| browser | 3280 | medium | `tools/browser-mcp/selftest.mjs <fixture>` (18) | ✔ 18/18 direct + via host (scratch host 2026-10-28); ✔ **real host 2026-10-29 (zcode)**: paired-browser drive of the real UI through `/mcp/call` — open→snapshot→screenshot, metered ledger rows, gate evaluated; web-mode same-origin fix + paired Bearer boot probe landed (UI connected: Consciousness ON, I-Loop 179 insights visible) |
| browser-farm | 3281 | light | `tools/browser-farm-mcp/selftest.mjs` | ✔ **live farm acceptance 2026-10-29 (zcode, EU VPS)**: real farm deployed (systemd + Postgres, loopback-only); command-center profile RUNNING with the bridge script in real Chromium/Xvfb; `farm_command` ping → bridge consumed in 0.4 s → `farm_result` pong; `farm_navigate` (allowlisted example.com) → real 87 KB screenshot pulled as evidence (`docs/evidence/browser-farm-eu/`); locked-by-default nav verified; MCP `farm_profile_start` 422 bug found by the REAL farm (missing `accounts` body) — fixed |
| web | 3225 | light | – | ✔ earlier (web_search config-disabled ≠ broken); re-run ☐ |
| code | 3230 | light | – | ✔ earlier (callgraph); re-run ☐ |
| git | 3235 | light | – | ✔ earlier (note double-nested `{output:{output}}`); re-run ☐ |
| gcal | 3241 | light | – | ✔ earlier after port fix; OAuth path `[R]` ☐ |
| zoom | 3265 | light | – | design-only; ☐ |
| visual | 3220 | heavy | – | ✔ earlier (`yolo_detect` needs `pip install ultralytics`) ☐ re-run alone |
| threed | 3250 | heavy | – | ✔ earlier (headless-hang limitation, 20 s guard) ☐ |
| cerebrix | 3245 | heavy | – | ✔ earlier ☐ |
| connectome (+3256) | 3255 | heavy (~3.3 GB) | – | ✔ earlier; 1 bug found ☐ re-verify fix |
| re | 3260 | light | – | ✔ earlier ☐ |
| dna / chemistry / proteomics / geospatial / mesh | 3271-3276 | varies | – | ☐ never recorded in this guide's sources — run contract |
| code-graph | 3279 | light | – | ☐ (Python-only cross-file; no supervisor facts ⚠) |
| p2p-bridge | 3278 | light | – | ☐ with two paired hosts |
| bridges (Roblox/Unity/Unreal/**Blender**) | 3210 | medium | – | ✔ refusal path earlier; **Blender live ☐ (see §7a)** |
| ozone-shared-context | stdio | light | – | ✔ continuous; mirror degraded only when host down |

### 7a. Blender 3D — creation and handling, end to end `[R]` (`/usr/bin/blender` is installed)
| ID | Step | Expected |
|---|---|---|
| BL1 | Via bridges (:3210): `blender_get_scene` then `blender_create_object` (cube) → `blender_set_transform` → `blender_set_material` | success; object in scene |
| BL2 | `blender_setup_rigid_body`, `blender_set_gravity`, `blender_bake_physics` | baked frames exist |
| BL3 | `blender_export` (glb/obj/stl) to a workspace folder | file written, size > 0 |
| BL4 | Attach the exported file to a project (file_link) | modality **3D** (glb/obj/stl/blend per `detect_modality`); FileReference container; ripple event |
| BL5 | Run the 3D modality pipeline on it | modality graph container; ModalityBadge `3D`; GraphKit node |
| BL6 | Cross-modal link: 3D export + its README.md + a Python generator script in one project | `SimilarTo` edges across 3D/Text/Code (T-XM1 shape); triangle connected (T-S4) |
| BL7 | `blender_run_python` refusal/allow policy | arbitrary Python is gated (compare terminal-mcp: `blender --python-expr/--python-console` denied) |
| BL8 | terminal-mcp `blender` role: `blender -b --python <repo script>` | runs a file in the repo; `--python-expr` refused |
| BL9 | Render a PNG with Blender, ingest as Image | modality Image; image pipeline output |
| BL10 | Open the 3D file in the UI viewer; screenshot via browser-mcp | viewer loads; evidence PNG |
| BL11 | Bridges not running ⇒ tools absent/clean error, not a crash | honest unavailable |

### 7b. Cross-MCP capability scenarios (via the graph, never MCP→MCP) [doc MCP_TESTING §4]
| ID | Scenario | Pass |
|---|---|---|
| X1 | browser_screenshot of the Ozone UI → image modality ingest | Image container with the PNG; ripple |
| X1a | Auto-ingest hook (DESIGNED_NOT_BUILT_REGISTRY §1): `POST /capture/ingest_image` + `OZONE_*_INGEST_URL` on both browser MCPs (default OFF) | ingest success returns container_id; ingest FAILURE leaves the screenshot on disk + `ingest_error` (capture and ingest are separate facts); route is strict-session; pipeline 102 runs through the entry gate + jurisdiction + S12 like every walk |
| X2 | android_screenshot → image modality; ui_snapshot labels stored as untrusted text only | no label ever treated as an instruction |
| X3 | security finding (induced `success:false` on a real security tool) → sensor/actor → `SecurityFinding` candidate → amt_loop deepening (1 call) | candidate once (dedupe), one paid call, branch persisted |
| X4 | terminal refusal and git status in one task | both rows in S13; refusal is data, not crash |
| X5 | code-mcp callgraph + code-graph-mcp (Python) on the same repo | same-file edges vs cross-file; document the boundary (T-C4) |
| X6 | web_search + web fetch → text modality → keyword links (T-XM1 lexical ceiling) | SimilarTo edges, honest about lexical-only |
| X7 | Blender export + visual yolo_detect on a render | detection JSON linked to the render's Image container |
| X8 | Two hosts: local + EU, paired; a finding on one appears on the other only if paired (default-deny both directions) | trust gate holds |
| X9 | Registry replacement: re-register a tool at a new endpoint ⇒ `server_version` bumps, old endpoint gone | no stale endpoints (port-move regression) |
| X10 | Metering: N calls by agent A and B | ledger separated per agent/day/tool; over-limit refusal is counted |

### 7c. agent-mcp — the DOMorpher conversion (docs/DOMORPHER_CONVERSION_GUIDE.md; seam: library crate `domorpher-core` → tool surface → MCP enforcement → Ozone above)
| ID | Test | Pass |
|---|---|---|
| AG1 (retired, fresh start) | domorpher-agent selftest (agent is UNUSED/unregistered while the farm tools are the door) incl. a refusal per dangerous class; registration carries platform + version tags | locked-by-default; refusals are data |
| AG2 (retired, fresh start) | `domorpher_navigate` to a non-allowlisted host | refused (farm_navigate precedent) |
| AG3 (retired, fresh start) | `domorpher_exec_script` with OZONE_DOMORPHER_ALLOW_EVAL unset | refused (browser-mcp eval-gate precedent) |
| AG4 (retired, fresh start) | session/cookie aging: profile restart keeps cookies with correct relative ages | FARM-NATIVE (persistent profiles are a Browser-Farm feature — StateStore seam maps to farm profiles) |
| AG5 (retired, fresh start) | `domorpher_extract` output | untrusted-labeled tool result (X2 rule — never an instruction) |
| AG6 (retired, fresh start) | `domorpher-core` dependency tree | NO http-server / llm-client / storage-engine / browser-driver crates — the seam enforced by the dependency graph itself |
| AG7 (retired, fresh start) | Backend selection: `OZONE_DOMORPHER_BACKEND=farm` (default) vs `local` | farm backend executes via farm tools/Command Center (live-verified round-trip); local via browser-mcp CDP for isolated use; SAME gate/metering/ripple on both — backend is config, never a bypass |

## 8. Acting loop and watchdogs (L7) — see ACTING_FACTORS_REGISTRY
| ID | Test | Expected |
|---|---|---|
| A1 | Sensor tick (every 900 s; or wait one cycle) with firewall unprivileged | `inspectable:false` ⇒ no `SecurityFinding`; I-Loop stops echoing (**verify at next tick** ☐) |
| A2 | Force a real failing security tool | one candidate (dedupe on container+route+detail digest) |
| A3 | `CoordinationFindingActor`: `note_add(kind=finding)` | `CoordinationReceived` candidate |
| A4 | `TaskFailedActor`: fail a task | `TaskFailure` candidate |
| A5 | `CodeStalenessActor`: edit a registered FileReference | only real `updated` events, not baselines |
| A6 | File beacon: create/modify/delete a registered file | 45 s poll ripple, JSONL history, snapshot |
| A7 | AMT loop budget: burst of events | one LLM call per candidate; ≤ ~20–30/day target holds; no silent cap |
| A8 | Assistant check-up with `consciousness.enabled=true` | free filter; unchanged findings skip paid call; changed ⇒ one batched call; `/assistant/feed` entries carry source refs |
| A9 | Assistant follow-up task creation | real Order task; never reorders the plan |
| A10 | I-Loop 1800 s floor; disabled when `consciousness.enabled=false` | no calls when off |
| A11 | Review pass (`/consciousness/review_pass`) | tasks from aged S10/S11 failures; S12/S13 extension ☐ |
| A12 | Task executor **opt-in**: `OZONE_AUTO_EXECUTE_TASKS=1`; queued prompt task runs full gated walk; non-prompt stays a record | one per pass; refuses below `OZONE_MIN_AVAIL_MB` |
| A13 | **Watchdog** (REVIEW_WATCHDOG): kill-on-silence not kill-on-age; retry/escalation cycles through all fallbacks | stuck call detected by silence; state marked truthfully (method 43); host never silently dies |
| A14 | Memory-envelope soak: executor + emulator + browser simultaneously | host survives; gate skips are loud |
| A15 | Generic MCP-failure actor (proposed) | n/a until built ⚠ |

### 8a. Assistant proposals — the acting chain's propose half (built 2026-10-29, zcode, per the operator's ACTING-FACTORS correction)
The Personal Assistant works ALONGSIDE the system: it never calls tools. Proposals are REVIEW rows in the EXISTING S10 store
(`review_kind: assistant-proposal`), listed by the EXISTING `/consciousness/reviews/pending`; the operator's "Proceed" at the
EXISTING `/consciousness/reviews/resolve` executes through the EXISTING gated `call_global` door (metered, jurisdiction-gated,
rippling) and appends an honest `executed`/`failed` row. Derivation is deterministic (no LLM): meeting-soon with a real URL →
`browser_open` it; slippage-cascade → open the Order board; overdue(≥3) with an external link → open it; a finding without
concrete data proposes NOTHING. Deduped while unresolved (stable kind-scoped ids); after a Decline, the next check-up may
re-propose. Whitelist (`PROPOSABLE_TOOLS`) bounds what a tampered row can reach — closes the A8/A9 open risk.
| ID | Test | Expected |
|---|---|---|
| A16 | Check-up pass with a meeting holding a real `meeting_url` inside the 2 h window | one final S10 row, kind `assistant-proposal`, decision `review_pending`, payload JSON in `reasoning_preview`; appears at `/consciousness/reviews/pending` |
| A17 | Resolve A16's row `Proceed` | `proposal_execution` in the response; tool ran via `/mcp/call` (ledger row, gate verdict); follow-up S10 row `executed` with real output — or `failed` with the real error (never fabricated) |
| A18 | Resolve `Decline` | resolution recorded; NO execution; next check-up may re-propose (re-derivable) |
| A19 | Tampered row naming a non-whitelisted tool + Proceed | refused (`not on the proposable whitelist`), `failed` row recorded, nothing executed |
| A20 | Unchanged findings on the next pass | no duplicate proposal while unresolved (stable id dedupe) |
| A21 | Proposal for an unregistered tool + Proceed | honest `failed` row ("mcp global handles"/unreachable), never a silent no-op |

## 9. Modalities and file types (L8) — fixture matrix `[R]`
Pipelines: 28 modality dirs (3D, audio, BCI, biology, CAD, chemistry, code, control, depth, dna, eeg, electromagnetic,
geospatial, haptic, hyperspectral, image, IMU, kinematics, math, network, proteomics, radar, sonar, sound, text, thermal, video)
+ 38 general + 17 consciousness + 4 shared. For each fixture: attach to a project → record (a) `detect_modality`, (b) FileReference
container, (c) pipeline run result or honest "no pipeline", (d) viewer in the UI, (e) cross-links.
| Group | Fixtures (extension) | Known issue to assert |
|---|---|---|
| Text | `.txt .md .rst .adoc .org` | ✔ `.md` → Text (live file refs); ⚠ viewer must not render as Math (MathViewer is modality-blind) |
| Text-PDF | `.pdf .doc .docx` (DESIGNED_NOT_BUILT_REGISTRY §6) | text pipeline 100 gains real pdf/docx extraction (pdf-extract / zip+xml) BEFORE plain-text read; per-PAGE provenance in chunk meta; scanned-PDF → honest "no text layer" error (OCR out of scope, recorded); encrypted PDF → honest error; `.tex` → **Math** (canonical table, post-unification) |
| Code | `.rs .py .js .ts .tsx .java .go .c .cpp .sql .sh .v .fs .ml` | ⚠ `.ts` is Code (MPEG-TS never reached); `.v/.d/.fs/.ml` ambiguity; T-C4/C5 |
| Data | `.json .yaml .toml .xml .csv` | Data has **no pipeline** ⚠ |
| Config / Web | `.conf .ini .env`, `.html .css` | Config & Web have no pipeline ⚠ |
| Document | `.pdf .docx .xlsx .pptx .odt .rtf` | ⚠ "Document" has **no pipeline** — PDFs never analyzed (blocks law-book ingestion) |
| Image | `.png .jpg .svg .webp .heic .psd` | `.raw` ambiguity |
| Audio / Video | `.mp3 .wav .flac .mid`, `.mp4 .mkv .webm .mov` | |
| Math | `.tex .ipynb .nb .m .wxm .rmd` | ⚠ `.m` (Objective-C/MATLAB) and `.ipynb` mixed content |
| Chemistry / DNA | `.mol .sdf .pdb .xyz .cif .smi`, `.fasta .fastq .gbk .vcf .bam .gff` | ⚠ `.sdf` (also Kinematics), `.pdb` (protein vs chemistry) |
| EEG / BCI | `.edf .bdf .fif .vhdr .set` | `.set` ambiguity |
| 3D / CAD / Kinematics / Control | `.glb .gltf .obj .stl .fbx .ply .usd .blend`, `.step .iges .brep .f3d`, `.urdf .mjcf .bvh`, `.slx` | `.blend` → 3D; BL1–BL11 |
| Geospatial / sensors | `.geojson .shp .tif`, radar/sonar/thermal/hyperspectral/depth/IMU samples | classifier has **no** bucket for several — expect `Unknown`, document |
| Archive | `.zip .tar .gz .7z` | Archive has no pipeline ⚠ |
| Unknown | `.xyzabc`, no extension | `Unknown`, no crash |
| Hostile | zero-byte, 2 GB sparse, binary-as-text, UTF-16, huge single line, symlink loop, path with spaces/unicode | clean refusal/degradation; no panic; no host OOM |
Also assert the **four disagreeing extension→modality tables** (file_link, `orchestrator/graphs.rs`, `types/modality.rs`, UI
`workspaceData.ts`) agree for every fixture — any mismatch is a defect row. Per-file modality must not depend on positional
`context.keywords[1]` (add a round-trip test).

## 10. Workspaces (L9) — different workspaces, different files
| ID | Test | Expected |
|---|---|---|
| W1 | Create workspace A and B, each with a project; attach different file sets | containers parented to the right project; no cross-bleed |
| W2 | Scope isolation: shared-context `scope` global/workspace/project (T-CO1/CO3) | keywords `ws:<id>`/`proj:<id>`; coercions never silent-global |
| W3 | Graph/AMT project scoping | AMT parents to its project (T-A2); text/code/math parent to project (T-T2/C2) |
| W4 | Same file attached to two workspaces | two references, one source of truth, independent graphs |
| W5 | Delete/rename file on disk | beacon ripple; reference marked stale; no orphan edges |
| W6 | Workspace tab (pipeline-delivered) in web mode and Electron | loads via HTTP fallback / bridge; actions need session |
| W7 | Workspaces list + "Test Workspace" page (the two pasted views) | graph-native look, indicators visible |
| W8 | Multi-user/device: paired phone sees only permitted scope | per-user namespacing (vault design) ☐ |
| W9 | Large workspace (2400+ nodes) in 3D | frame rate, LOD, no crash (real GPU `[R]` untested) |
| W10 | Cross-workspace relationships | only via explicit link; jurisdiction scope edges typed (T-I5) |

## 11. UI (L10) — drive it with browser-mcp, screenshot every tab `[R]`
Method: `ui-harness.mjs` (same-origin proxy) + `browser_open/snapshot/click/type/screenshot` through `/mcp/call`; paired session
token injected into `localStorage.ozone_session_token`. ⚠ zcode found the web build boots "Awaiting connection" because the bundle
targets absolute `http://127.0.0.1:50051` (cross-origin) — in-flight fix: same-origin default + Bearer on the config probe
(`ui/src/ozoneClient.ts`, `App.tsx`, `graphEventClient.ts`, uncommitted by zcode).
| ID | Surface | Check |
|---|---|---|
| U1 | Boot/connection/setup wizard | connects; first-run exemption only while setup incomplete |
| U2 | Dashboard tab strip | no overlap with search; scrollbar styled (fixed a346d38) ✔ |
| U3 | Workspace / Files / Order / Library / Settings / Monitor / Tools / Devices / Graph tabs | each loads; "No UI module found" never appears ✔ for pipeline tabs (HTTP fallback) |
| U4 | Chat: send, retry/branch buttons ☐ (W9), persistence (sessionStorage) | |
| U5 | Order panel + assistant feed section | reads `/order/global`, `/assistant/feed` |
| U6 | Task detail: TaskStateIcon, StepPips, step context | real fields only |
| U7 | OrchestrationStatusPanel: GateState (stage 8/14) | ⚖ cleared / Proceed / Decline |
| U8 | WorkspaceStatusStrip indicators: tasks, gate, peer, activity, loops | chip hides when its route is silent |
| U9 | 3D GraphNative world: nodes by modality, cross-modal magenta edges, picking, orbit; in-world indicators ☐ (W5); call-graph lens ☐ | |
| U10 | File viewers per modality (Code/Math/Text/Image/…) | modality-aware (⚠ MathViewer) |
| U11 | MonitoringPanel / McpActivityIndicator / ConnectedAgents / PairingPanel | live events for browser/android calls ✔ visible as host events |
| U12 | Electron app (bridge path) | `[R]` never driven yet ☐ |
| U13 | Phone via QR pairing (`ozone-mobile` / mobile browser) | pairs, token stored, calls work |
| U14 | Theming/localization, empty states, error states | no raw stack traces |
| U15 | Farm lens (DESIGNED_NOT_BUILT_REGISTRY §2): EU farm configured → workspace Farm lens | Command Center + workers as infra nodes, statuses verbatim from `farm_status`; no farm ⇒ lens ABSENT (not an empty shell); MCP `untrusted_*` labels display-only (X2 rule); starting/stopping profiles is an acting-chain act, never a lens click |
| U16 | In-world indicators (registry §4): task nodes carry TaskStateIcon sprites, capture pillar bars match CaptureChip numbers exactly, right-click menu = pin/hide/copy-id only | absent data = no sprite; no "execute from world" affordance anywhere |

## 12. Mobile (L11) `[R]`
Emulator `ozone_api30` provisioned (`tools/android-mcp/provision_sdk.sh`); stop it when idle.
| ID | Test | Status |
|---|---|---|
| M1 | preflight/avd_list/emulator_start through `/mcp/call` | ✔ |
| M2 | device_info, ui_snapshot, tap_element, term_open/exec, screenshot, logcat, forward, termux_status | ✔ (22/22) |
| M3 | Shell policy: `su`, `rm -rf`, pipes, off-allowlist, `settings put`, `pm uninstall` refused; locked-by-default | ✔ |
| M4 | Install the `ozone-mobile` Expo app (needs a built APK) and run the QR pairing against a host | ☐ |
| M5 | Termux in the emulator/phone: `pkg install openssh`, `sshd`, `android_forward remote=8022`, ssh from a terminal role | ☐ |
| M6 | Real phone over USB/Wireless debugging; RSA prompt; `unauthorized` state reported | ☐ SKIP(no phone) |
| M7 | Wireless/tablet/low-RAM AVD variants; API 34/35 image | ☐ |
| M8 | App test loop: install APK → launch → perceive → act → verify → screenshot → logcat on crash | ☐ (needs a test APK; ZCode's build/run tools would help) |
| M9 | Emulator + host + browser simultaneously (RAM envelope) | ☐ (available RAM drops to ~1.5–2 GB with the emulator up) |

## 13. P2P / multi-host / EU (L12) `[R]`
| ID | Test | Expected |
|---|---|---|
| E1 | Local + EU pairing (`/network/connect-peer`, `approve-peer`, `revoke-peer`) | default-deny both directions; revoke cuts traffic |
| E2 | Peer-sync review turns received signals into finding containers | bookkeeping only, no model calls |
| E3 | `auto_broadcast_sync` off by default ⇒ no outbound; on ⇒ methodology/blueprint reach peer | per config |
| E4 | EU deploy: build (≥10 GB free), scp binary, restart unit, `/health`, logs | ✔ EU currently on Oct 28 01:57 binary; redeploy ☐ |
| E5 | EU MCPs under systemd (`ozone-security-mcp.service` with CAP_NET_ADMIN ✔); deploy new security/terminal code ☐ | |
| E6 | EU sensor tick evidence (`shared_context/tool_call-sensor-tick-*`) | ✔ 133 events, all success, newest minutes old (2026-10-29) |
| E7 | Live Browser-Farm on EU (Postgres + Docker present from Manejard) | `[R]` ☐ — then retire the mock |
| E8 | Reverse proxy/TLS exposure; default bind 127.0.0.1 | only via proxy/firewall |

## 14. Security and abuse (L13)
| ID | Test |
|---|---|
| S1 | Locked-by-default for every MCP with no config (terminal, android shell, browser allowlist, farm) |
| S2 | Terminal bypass classes (node -e, python3 -c, find -exec, git -c, npm exec, systemctl restart, operators, newline, `nodemon`) ✔ 18/18 |
| S3 | Browser: off-allowlist host, `javascript:`, `file:`, redirect to foreign origin, eval gate, injected page text ✔ 18/18 |
| S4 | Android: token required, forward port whitelist, emulator_stop refuses physical serials ✔ |
| S5 | Session gate matrix (P4); internal-token scope; `/mcp/call` agent spoofing ⚠ |
| S6 | Secrets hygiene: `config.toml` key (rotate!), no tokens in logs/captures/prompts/evidence; vault design (not built) |
| S7 | Jurisdiction: Block/Warn/Confirm; elevations only by operator act |
| S8 | Prompt injection through page/app/file content reaching a model prompt (untrusted_* wrapping) |
| S9 | Resource abuse: output caps, timeouts, max sessions, idle TTL |
| S10 | Supply chain: downloaded SDK/JDK are untrusted data; SHA check of platform-tools/cmdline-tools |
| S11 | Vault (DESIGNED_NOT_BUILT_REGISTRY §5): set a known secret → grep the whole {data_dir} for it ⇒ only ciphertext bytes (never in graph containers, prompts, logs, ledger rows); `vault_lease` TTL ≤ 300 s, expired lease refused at the consumer; `/vault/reveal` over HTTP REFUSED (interactive CLI + master passphrase only); every set/lease/reveal/unlock attempt in `vault/audit.jsonl` with zero secret bytes |
| S12 | Proposal whitelist replay (live-verified 2026-10-29 ✔): tampered S10 row naming a non-whitelisted tool + Proceed ⇒ refused + honest `failed` row (PROPOSABLE_TOOLS); resolved proposal id re-derivation held by the 6 h cooldown (no duplicate-row flood) |

## 15. Regression catalog (every defect found so far gets a permanent test)
| Defect (date) | Test |
|---|---|
| firewall_status echo → I-Loop (10-29) | A1 |
| terminal first-token bypass (10-29) | S2 |
| android refusals hidden as HTTP 400 (10-29) | M3 asserts reason visible through `/mcp/call` |
| browser port collision 3250 (10-29) | P7/X9 |
| `termux_status` exit code (10-29) | M2 |
| Pipeline tabs Electron-only (10-28) | U3 |
| Tab strip hidden under search (10-28) | U2 |
| Web UI cross-origin boot (10-29, zcode) | U1 |
| `.md` shown as Math ⚠ not reproduced | §9 Text row + viewer check |
| PDFs have no pipeline | §9 Document row |
| Four modality tables disagree | §9 closing note |
| Registry persistence misreported in-memory (10-29) | P8 |
| mmap-disabled `store_global` silent no-op | storage unit test ☐ |
| Absolute-path garbage-join (3 sites) | T-62 family ✔ |

## 16. Run order and ledger
1. P1–P9 → B1–B7 → C1–C8 (cheap, no host load)
2. L3 routes → O1–O11 → G-CF1..5
3. MCP light batch → heavy one-by-one → §7a Blender → §7b cross-MCP
4. A1–A15 (needs a wait of ≥1 sensor cycle; run overnight on a quiet box)
5. §9 file matrix → §10 workspaces → §11 UI (browser-mcp) → §12 mobile (emulator) → E1–E8 → S1–S10
6. Record each suite in `docs/evidence/test-runs/<date>-<suite>.md`; update the ✔/☐ marks here **in the same commit** as the evidence.
Operator decisions that gate sections: host rebuild/deploy (W0) → P5/P6, C5, A8, B6; EU consent → E4–E7; test APK → M4/M8;
Browser-Farm location → §7; GPU/Electron run → W9/U12.

## 17. Review additions (cc, 2026-10-29) — proposals hardening and cross-surface integration
Source and evidence: `docs/REVIEW_PROPOSALS_FARM_UI_2026-10-29.md` (claim-by-claim verification; insights R1–R10).
**Annotations on rows above that carry ✔ from the same day:**
- *browser-farm (§7)* — the farm itself is verified real on EU (services, loopback bindings, Xvfb+Chromium, real screenshot).
  **But** EU's registry still maps the 13 farm tools to `:3252` while farm-mcp listens on `:3281`; `farm_status` through EU
  `/mcp/call` returns `Connection refused` (2026-10-29 11:34 UTC). Not a pass for registry correctness (I-MCP1) until re-registered
  from an up-to-date registrar.
- *A16/A17* — live data shows two `slippage-cascade` action proposals, both resolved Proceed (one twice), both outcomes `failed`
  (allowlist / connection refused). The chain records honestly; no successful action has been observed yet.
**New rows:** A22–A30 (proposals: notification acknowledge, idempotent Proceed, no re-nag, detail-change re-propose, allowlisted
success, real failure, session gate, payload length, strict-mode-off posture) and I-WS1..2, I-EN1..3, I-UI1..4, I-3D1..4,
I-MOD1..3, I-MCP1..3 (workspace attribution, engine actors/planner/executor, Inbox UI, graph-block from UI snapshots, modality
ingest of screenshots, per-host endpoint and checksum drift). Tables are in §6 of the review document; run them in L4/L7/L10/L6.
⚠ currently expected to FAIL: A23, A24, I-WS1, I-EN1, I-EN2, I-3D1, I-3D2, I-UI1/UI2 (not built), I-UI3 (vite dev), I-MCP1 on EU.

## §18 Library boundary, language identification, google.com DOMorpher test, host edits (2026-10-31, cc) — all UNCOMPILED/UNRUN until the gate
Source docs: REVIEW_DOMORPHER_LIBRARY_BOUNDARY_2026-10-30, REVIEW_OZONE_STATE_AND_FARM_TEST_2026-10-31, TOOLS_BY_LANGUAGE.
| id | check | expected |
|---|---|---|
| AG6a | `cargo tree` on domorpher-core | no reqwest/jsonwebtoken/tiktoken-rs/wasm-bindgen/http/llm/storage crate |
| AG6b | grep public API of core for `Llm`/`LLM`/`DecisionSource`/`PageDriver`/`StateStore` | zero hits (LlmZeroShot->ExternalJudgment, LLMError->JudgmentError done) |
| AG8 (retired, fresh start) | RETIRED roles each have a named Ozone owner (reasoning->pipeline 9+AMT, traversal->traversal_ml/context_aggregation, multi-pass->decision_review+consensus, recovery->TaskFailed/McpFailure actors, extraction->text/code pipelines+S11, instruction->pipeline 9+task_recommendation) | table in REVIEW_FORK_ALIGNMENT correction; no library code for them |
| AG9 (retired, fresh start) | MOOT after lean core: library emits no judgment requests | grep core for NeedsJudgment/EscalationRequest = zero (after census) |
| T-DOM-GOOGLE D0-D8 | see REVIEW_OZONE_STATE_AND_FARM_TEST §4 | D1/D2 first (real google.com DOM from the CC farm profile -> library offline -> field) |
| LANG1-7 | see TOOLS_BY_LANGUAGE §4 | tags `lang:`/`kind:`; domorpher-py wheel builds and imports in the farm venv |
| SC1-SC4 | `/mcp/call` with context workspace/project | S13 row + ripple carry scope (global when absent) |
| MF1-MF6 | failed browser_/farm_/android_/terminal_/domorpher_ call | one CoordinationReceived finding, cooldown 600 s, expected refusals ignored, project AMT routing |
| PR1 | acknowledged/executed/failed proposal re-draft within cooldown (6 h, `OZONE_PROPOSAL_COOLDOWN_SECS`) | not re-recorded |
| PR2 | Proceed twice on one review | one execution; second returns `already_resolved:true`, no duplicate resolution row |
| PR3 | resolved id re-recorded after cooldown | visible again in `/consciousness/reviews/pending` (row newer than resolution) |
| PR4 | resolve-route execution error | HTTP 200 `{success:false,error}` |
| HK1 | ingest hooks send `x-oz-internal` from `OZONE_*_INGEST_TOKEN`; unset = off + one log line, token never logged | screenshot lands in graph without a manual call |
| GB1 | `browser_snapshot` / `android_ui_snapshot` return a `graph` block (<=2000 edges, bounds stated) | containers visible under the workspace |
| RG1 | `register-browser-tools.mjs all` excludes domorpher_agent; `--prune agent_` clears the 16 stale rows via `POST /mcp/tools/unregister` | registry has no `agent_*` |

## §19 UiDocument modality (design 2026-10-31, docs/DOMORPHER_CLEAN_SLATE.md §9) — rows UD1-UD12
UD1 detect `.html/.css` != Unknown · UD2 pipeline 127 on captured google.com HTML · UD3 live DOMSnapshot adds visible + spatial edges · UD4 standalone CSS graph · UD5 cascade visibility/specificity · UD6 Android uiautomator dump same shape · UD7 hostile page text stays untrusted · UD8 script text -> Code modality, never run · UD9 semantic_markdown on Document node · UD10 Python wheel == pipeline output · UD11 AG6 cargo tree clean · UD12 JS graph-block stopgap removed. (Full table in the design doc; status: all NOT BUILT.)

> **§7c AG1-AG9 and §18 AG/LANG rows tied to domorpher-core/domorpher-agent are RETIRED (2026-10-31, fresh start).** Replaced by §19 UD1-UD12 plus one row per rule family (`docs/UIDOCUMENT_RULES_GUIDE.md` §3). LANG1-2 (language tags) stand.
UD13 live DOM: a mutation (e.g. cookie banner closes) on the real google.com page via the CC farm profile -> one graph delta, node identity stable, ripple emitted · UD14 observer is read-only (no click/type/eval path) and never reports password-field values · UD15 mutation burst -> one summarized delta with a count · UD16 `actionable` false for an element covered by another (`elementFromPoint`) · UD17 geometry rows absent until requested (no guessed spatial edges). (§19 additions; NOT BUILT.)

## §20 GEO series (docs/GEOMETRY_GUIDE.md §5) — GEO1-GEO15, all NOT RUN
GEO1 real screenshot into pipeline 102 gives file-derived width/height/format/colour, not the 800x600 PNG constants (RED today) · GEO2 no hard-coded `example_object` path in `analyze_image` · GEO3 graph block with LeftOf/Touches/Behind accepted or only the bad edge rejected (RED today: whole block rejected) · GEO4 graph-block modality tag correct (not `Unknown`) for dna/chemistry/geospatial/mesh/browser · GEO5 one relation engine across two modalities in a linked frame · GEO6 depth back-projection with intrinsics · GEO7 3D transforms from the Blender bridge · GEO8 STEP faces/edges · GEO9 chemistry coordinates from RDKit not fabricated · GEO10 PDF blocks + reading order · GEO11 thermal<->image `RegisteredTo` · GEO12 sound localization filled or honestly absent · GEO13 URDF/MJCF/SDF unambiguous · GEO14 no cross-modal self-loops · GEO15 video frames/tracks.
> **EVIDENCE CORRECTION (2026-10-31):** earlier entries (CHECKLIST/STATE_CAPTURE, EU ingest milestone f973fa0) call the `/capture/ingest_image` output "real image analysis". Verified in code: `analyze_image` discards the pixels and returns constants (800x600 PNG, sRGB, 8-bit, `example_object`). The ROUTE/gate/dispatch evidence stands; the ANALYSIS CONTENT was not derived from the screenshot. Rows that cited it (E-series ingest) must be re-run after GEO1/GEO2.

## §21 UiDocument host wiring (B3, uncompiled) — rows from the fork report
UD-REG: after boot on an existing data dir `index.json` contains 127 and the registry gate loads it · UD-ING: `/capture/ingest_uidocument` refuses unknown flavor / zero or two sources / path outside data_dir+cwd / inline >5 MB; unbuilt binary => `success:false` "binary not built"; built pipeline => graph persisted with `ws:`/`proj:` keywords; strict mode without session => 401 · SG-PERSIST: graph block with `attributes.geom` yields stored `Within`/`Encloses`; unknown relation still refused · SG5-lite: nodes in different frames never get a spatial edge · UD-UI: `graphs/uidocument_<id>.json` resolves to modality `uidocument`; `tsc` type-check passes · pipeline binary + package both named `uidocument`; EU unit needs WorkingDirectory + `pipelines/modalities/uidocument/uidocument`. Known B3 gaps: InFrontOf/Behind (need z-order in geom), Touches, Aligned, Supports not computed; cross-frame relations not computed; huge boxes (>1024 grid cells) compared against everything; `mcp_call` still uses unscoped `persist_from_output`.

## §22 UiDocument engine (B1, uncompiled) — contract and rows
Engine output node fields follow the HOST (`key`, `attributes`, `parent`; `id` alias; geometry under `attributes.geom`+`frame`); relation names are mapped to host-known ones by default (`relation_mode:"host"`: LabelFor/FormOwns/Controls -> RelatedTo; LinksTo/Embeds/Includes/StyledBy -> References; ChildOf via `parent`; original name kept in the edge's `semantic`) and emitted as-is with `"relation_mode":"semantic"` once the host accepts them. `normalize_input` accepts engine shape, the ingest-route shape (`action:{type:Analyze,flavor,url,document_data:{Html|Batch|FilePath}}`) and the orchestrator's generic `{type:Analyze,text|content|path}`. Rows: UE1 route shape -> same graph as engine shape · UE2 generic Analyze with `.css` path -> flavor css · UE3 `relation_mode` host vs semantic · UE4 74 html + 11 css rules load, each emitted item carries rule id + confidence · UE5 dom-live full batch then delta (`delta{added,changed,removed}`) · UE6 android-xml/a11y-json answer honest not-implemented · UE7 `cargo tree` no network/model/async crates · UE8 `scraper` 0.20 API compile (risk) · UE9 py wheel `analyze` == CLI output for the same input · UE10 observer batch accepted by `dom-live` incl. extra fields. NOT RUN.
