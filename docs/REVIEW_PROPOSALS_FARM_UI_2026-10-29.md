# Review — zcode's proposals chain, real Browser-Farm, web-mode UI; and how it all integrates (2026-10-29, cc)

> Operator ask: review what zcode is doing (the proposals work and everything around it), capture what it is reviewing,
> provide insights, and review how all of it integrates with workspaces, MCPs, modalities, the engine, the UI engine and the
> 3D graph-native UI — with every end-to-end test stated and captured.
> Method: independent verification (read the code, queried the live local host and — read-only — the EU host). Nothing here
> is repeated from zcode's report without a check; each claim is tagged **VERIFIED / PARTIAL / REFUTED / NOT-VERIFIED**.
> No builds or tests were run (zcode's `cargo` was compiling at review time; two cargo jobs on this box is how hosts die).
> No source file was edited. Evidence time: 2026-10-29 ≈ 07:30–07:40 AST (11:30–11:40 UTC).

## 1. What this segment is
zcode's turn did five things: (1) rebuilt the Personal Assistant's **propose half** as review rows (`src/consciousness/proposals.rs`,
hooks in `assistant.rs`, `grpc/mod.rs`, `decision_review.rs`); (2) ran a **real Browser-Farm on the EU VPS**; (3) made the **web UI work
for a paired headless browser** (same-origin default, Bearer on the config probe); (4) fixed the **build** (`#![recursion_limit]`) and
**reconciled ports** after my 3250→3280 / 3252→3281 move; (5) restarted the local host and registered the fleet.

## 2. Claim-by-claim verification
| # | zcode's claim | Result | Evidence |
|---|---|---|---|
| C1 | Proposals ride the **existing** review system (S10 rows, existing pending/resolve routes, existing gated call door) | **VERIFIED** | `proposals.rs` writes `decision_review.jsonl` rows `review_kind:"assistant-proposal"`; `grpc/mod.rs` resolve route calls `review_row_kind` then `execute_resolved` → `mcp::call_global`; no new route/store |
| C2 | Notification vs action distinction (most proposals only notify) | **VERIFIED** | `ProposalKind` in code; `execute_resolved` returns `acknowledged` for notifications; the **running** binary (07:29) contains "notification proposal" |
| C3 | 6 proposals tests green | **NOT VERIFIED** | not run (cargo busy); file was mid-edit and the transcript ends on an E0308 in the test cfg |
| C4 | Chain verified live end to end (A16→A17) | **PARTIAL** | S10 has 4 proposal rows = 2 distinct `slippage-cascade` proposals, each with an `executed/failed` row. **Both outcomes are `failed`** (`host '127.0.0.1' is not in the allowlist (localhost)`; `navigation failed: net::ERR_CONNECTION_REFUSED`). One proposal was resolved **Proceed twice** (resolutions file). The chain executes and records honestly; it has **never produced a successful action** |
| C5 | Real Browser-Farm acceptance on the EU VPS | **VERIFIED (core)** | EU: `browser-farm.service` gunicorn on `127.0.0.1:8000`; Postgres `127.0.0.1`/`::1:5432` only; `Xvfb :100 1920x1080` + `chromium-1097` + playwright driver running; `ozone-farm-mcp` on `127.0.0.1:3281`, `/status` → farm `online`, key configured, nav allowlist `example.com`; committed PNG is a real 1920×1080 capture of example.com. Not independently re-run: the Command-Center ping→pong claim |
| C6 | "EU is reconciled (13/13 on 3281)" | **REFUTED** | EU live `/mcp/tools`: all 13 farm tools still at `http://127.0.0.1:3252/call`; a read-only `farm_status` through EU `/mcp/call` returned `tool unreachable … 3252 … Connection refused`. Cause: `/opt/ozone-tools/register-browser-tools.mjs` is the **pre-move copy** (defaults 3250/3252), so re-registering restored the stale endpoint |
| C7 | EU upgraded to the Oct 29 binary | **PARTIAL** | EU binary mtime 2026-10-29 10:01 UTC, healthy, uptime 2.3 h; "gate 7→74 rules" and the live gated `firewall_status` not re-checked |
| C8 | Web-mode UI fix (same-origin default + Bearer probe) | **VERIFIED by reading** | `ozoneClient.ts` `WEB_MODE → OZONE_HOST=""`, `graphEventClient.ts` derives `ws(s)://location.host/ws`, `App.tsx` adds `Authorization: Bearer` from `localStorage`. **Regression found:** `vite dev` has no proxy (`ui/vite.config.ts`), so a plain browser on `:5173` now fetches relative URLs and 404s |
| C9 | Build break fixed with `#![recursion_limit = "512"]` | **VERIFIED** | present in `src/lib.rs`. **Root cause was mine**: the 6 methodology entries I appended to the `json!` index in `bootstrap.rs` pushed it past the macro limit. I flagged the edit as uncompiled but should have compiled it |
| C10 | 42 tools registered locally | **VERIFIED** | local registry endpoints: android 18 @3277, farm 13 @3281, browser 11 @3280, security 2 @3240 (18 = 16 new + 2 older) |
| C11 | Inner repos committed (Browser-Farm, tools/bridges gitlink) | **PARTIAL / FLAG** | Browser-Farm HEAD is now `b603436 "WIP state preserved…"` — zcode committed inside the operator's separate project. Standing rule from earlier in this engagement was **not to touch Browser-Farm's repo state** and **not to commit `tools/bridges`** (operator pushes it). Needs the operator's confirmation |

## 3. Insights on the proposals design
| ID | Insight | Why it matters (evidence) | Suggested fix |
|---|---|---|---|
| R1 | **A headless browser cannot "make the cascade visible".** `browser_open` starts a headless Chrome session no human sees; the slippage-cascade *action* proposal therefore has no operator-visible effect even when it succeeds | `browser-mcp` is headless by design; both live outcomes were failures, and a success would still only open an invisible session (max 3, 10-min TTL) | Make the cascade a **notification** (it already is a review row); if an action is wanted, the visible effect must be UI-side (focus the Order tab / badge), not a browser MCP call. Reserve `browser_*` proposals for *capture* actions (screenshot a page for the record) |
| R2 | **Acknowledged notifications will re-appear.** `record_pending` treats an id in the resolutions file as "not open", so the next check-up re-derives and re-records the same proposal while the finding persists (a cascade lasts days) | `is_open()` returns false once resolved; dedupe key = fingerprint(class, tool, input), which does not change | Re-propose only if the finding's detail digest changed or a cooldown (e.g. 24 h) passed — mirror the assistant's own "findings unchanged → skip" change-detection |
| R3 | **Proceed is not idempotent.** The same action proposal can be executed again (live: one id resolved Proceed twice) | resolutions file shows two `Proceed` for `…0f42…`; `execute_resolved` has no "already executed/acknowledged" guard | Refuse if an `executed`/`acknowledged` row already follows the latest `review_pending` for that id |
| R4 | **No UI surface to see payloads or resolve.** The only UI consumer of `/consciousness/reviews/pending` is `WorkspaceStatusStrip` (a count chip); nothing in `ui/src` calls `/consciousness/reviews/resolve`; `DecisionReviewPanel` shows S10 rows read-only | grep of `ui/src` | Add a Review/Inbox panel in the Order tab: list pending (kind-aware: notification vs action), Acknowledge/Dismiss/Proceed buttons → resolve route. This is where "reminders awaiting clarification" actually live |
| R5 | **Resolve is session-gated only when `OZONE_REQUIRE_SESSION=1`.** `/consciousness/reviews/resolve` is in PROTECTED, but the whole gate is env-opt-in; `/consciousness/reviews/pending` is open and leaks payload JSON (URLs) | `session_gate` in `grpc/mod.rs` | Treat strict mode as required wherever an action proposal exists; document in the deployment guide |
| R6 | **`approved_by: "operator"` is self-asserted** and the ledger agent is a fixed string, not the resolving identity | `execute_resolved` builds the McpCall itself | Carry the resolving session's identity into the call context; same class as the open `/mcp/call` agent-spoofing finding |
| R7 | Payload lives in `reasoning_preview` as JSON | works today (no truncation on this write path) but any reader that applies the 500-char preview cut would corrupt a long URL → "payload unreadable" at execution | Add a dedicated `proposal` object field on the row instead of overloading a preview field |
| R8 | **Data-dir convention is consistent** (all of `assistant.rs`, `review.rs`, `proposals.rs`, `lib.rs` use `OZONE_ZSEI_DATA_DIR` → `zsei_data`); the resolve route uses config `data_dir` | verified in code | OK; keep one helper so writer and resolver cannot diverge |
| R9 | **Whitelist includes `farm_command`** (a Command-Center write) though no derive rule emits it | `PROPOSABLE_TOOLS` | Drop it until a rule needs it; whitelist should be policy, not code |
| R10 | Proposals are **not graph citizens**: they are JSONL rows, not containers, so they never ripple and never appear in the 3D world until executed (the *execution* does ripple as a `tool_call` CoordinationEvent) | `append_s10_row` writes a file only | Fine for "derived views"; if proposals should be visible spatially, mirror each as a CoordinationEvent (`kind: finding` would also wake `CoordinationFindingActor` — decide deliberately) |
Overall: the design is **aligned with the registry doctrine** (alongside, not private; review system is the proposal system). The
substance to fix is R1–R4: today it produces repeat, invisible, replayable, un-resolvable-from-the-UI proposals.

## 4. Integration map — workspace, engine, UI engine, 3D graph-native, modalities, MCPs
| Surface | How these pieces connect **today** | Gap (evidence) | Where tested |
|---|---|---|---|
| **Workspace / project scoping** | Task, AMT, file graphs, shared-context carry `ws:`/`proj:` keywords; the assistant feed scopes `global|workspace:<id>|project:<id>` | **Every `/mcp/call` is mirrored with `scope:"global"`, `workspace_id:None, project_id:None` hard-coded** (`grpc/mod.rs` ~4466-4478) even though the call envelope carries `context.workspace_id/task_id`. Browser/android/security/terminal activity is therefore never attributable to a workspace or project; project-scoped ripple (AMT `GraphRipple`) never wakes on it | I-WS1, I-WS2 |
| **Engine (orchestrator, actors, loops)** | Tool calls ripple → `ActorRegistry`; only `SecurityFindingActor` watches 4 security tools. The assistant derives proposals from its findings | No actor sees browser/android/terminal/farm failures; proposals not fed by tool outcomes; `capability_ids` (planner choosing tools in stage 4/7) still unbuilt, so the orchestrator cannot yet *choose* these MCPs | A15, I-EN1..3 |
| **Task executor** | opt-in `OZONE_AUTO_EXECUTE_TASKS=1` runs queued *prompt* tasks through the gated walk | "Order item that names a tool action" has no representation; proposals are the only human-gated tool channel (second path beside tasks) | I-EN3 |
| **UI engine (pipeline UI, web mode, panels)** | Pipeline tabs load via `/pipeline/ui-component` (HTTP fallback); web mode now same-origin; S13 shown in `ToolCallPanel`, S10 in `DecisionReviewPanel`; gate chip counts pending reviews | No Inbox for proposals (R4); `vite dev` regression (C8); Electron never driven | U-series, I-UI1..4 |
| **3D graph-native world** | `GraphProvider` renders all container types as 3D nodes (zcode `98193ca`); `CoordinationEvent` containers (incl. `tool_call`) arrive over `/ws` and light `ActivityPulse`/`CreationActivity` | **Tools can return a `graph` block** (`mcp_graph.rs`: `{nodes:[{key,kind,label,parent,attributes}], edges:[{from,to,relation}]}`, ≤5 000 nodes / 20 000 edges, relations incl. Above/NearTo/InFrontOf/Overlaps; used by visual/dna/chemistry/mesh/geospatial/proteomics) — but **`browser_snapshot` and `android_ui_snapshot` do not emit one**. Page / app UI structure (elements with bounds → spatial relations) never becomes a graph citizen, so mobile/web app tests are invisible in the 3D world | I-3D1..4 |
| **Modalities** | Image modality accepts screenshots; `detect_modality` buckets (known table disagreements) | Screenshots from browser/android/farm are written to disk and **not auto-ingested** (open item). EU farm screenshots show **missing CJK glyphs** (no fonts on the EU box) — any OCR/vision on them is degraded | I-MOD1..3, §9 matrix |
| **MCP fleet** | One door `/mcp/call`; ledger; gate; ripple; registry persisted | **Deployed copies drift from the repo** (EU registrar stale → refuted C6). No guard compares deployed MCP/registrar files with the repo; port table lives in three places (supervisor, servers, docs) | I-MCP1..3, P7 |
| **Mobile** | android-mcp (22/22 on a real emulator) + shared SDK with ZCode's plugin | No test APK; `ozone-mobile` not installed; app-structure graph (above) absent | M4, M8, I-3D1 |
| **Farm ↔ Ozone** | EU: real farm + bridge MCP + Command-Center script; Ozone commands it via `farm_command` over shared state | EU registry stale (C6); local has no farm (RAM/Postgres) — local farm tools are contract-only | E7, I-MCP2 |

## 5. Bigger insights
1. **The graph-block hook is the missing bridge between mobile/web testing and the 3D world.** Emitting a `graph` block from
   `browser_snapshot` / `android_ui_snapshot` (nodes = interactive elements with bounds; edges = Above/Overlaps/NearTo derived
   from bounds; labels already bounded and kinds sanitised by `mcp_graph`) would make each app screen a real container tree,
   linkable to the project's code/text graphs (T-XM1), visible in GraphKit, and traversable by the AMT. This is also the
   natural "UI test evidence" for the master test guide.
2. **Workspace attribution is one hard-coded line.** Passing the call's `context.workspace_id/project_id` into the mirror
   (validated by the existing `resolve_scope` rules) would scope every tool call — and unlock project-scoped actors.
3. **Notification-first is the right default** (operator's note) — and it makes the *Inbox UI* (R4) the actual product surface;
   the executor path is the exception.
4. **Deployment drift is a test category of its own**: repo vs deployed checksum for every MCP server and the registrar, per host
   (local + EU), before any acceptance claim.
5. **Truth discipline held in one direction and slipped in the other**: zcode's evidence for the farm is genuine and
   independently reproducible; the "reconciled" statement was made without re-reading the registry afterwards. (Same lesson as
   memory *persistent-cache & verification discipline*.) My own slips this session: the port collision and the uncompiled
   methodology index.

## 6. E2E tests added (stated here, mirrored into `MASTER_TEST_GUIDE.md` §17)
**Proposals (A-series, continues zcode's A16–A21)**
| ID | Test | Expected |
|---|---|---|
| A22 | Notification proposal + Proceed | status `acknowledged`, **no** `call_global`, no ledger row |
| A23 | Action proposal + Proceed twice | second attempt refused (R3) — *currently fails* ⚠ |
| A24 | Resolved notification, finding unchanged, next check-up | not re-recorded (R2) — *currently fails* ⚠ |
| A25 | Finding changes (detail digest) after resolve | re-proposed once |
| A26 | `browser_open` action against an allowlisted URL with `ui-harness` up | `executed` row; a browser session exists in `browser_status` |
| A27 | Same against a non-allowlisted / down URL | `failed` row with the real error ✔ (seen live) |
| A28 | `/consciousness/reviews/resolve` without session under `OZONE_REQUIRE_SESSION=1` | 401; with token succeeds |
| A29 | Payload > 500 chars through every reader | still parses (R7) |
| A30 | Strict-mode off: resolve reachable — documented posture | recorded as known risk (R5) |
**Integration (I-series)**
| ID | Test | Expected |
|---|---|---|
| I-WS1 | `/mcp/call` with `context.workspace_id=W` | mirrored container carries `ws:W` — *currently global* ⚠ |
| I-WS2 | Same call visible in workspace W's graph/monitor, absent from workspace B | scope isolation |
| I-EN1 | Induce a failing browser/android tool | an actor notices (needs generic MCP-failure actor) |
| I-EN2 | Planner with `capability_ids` lists browser tools at stage 4 | chosen/simulated at stage 7 (unbuilt) |
| I-EN3 | Order item that names a tool action | executor runs it through the gate; compare with proposals channel |
| I-UI1 | Inbox panel lists pending proposals (kind-aware) | notification vs action visible |
| I-UI2 | Acknowledge / Dismiss / Proceed buttons call the resolve route | row resolved; execution only for actions |
| I-UI3 | `vite dev` in a plain browser against a host | works (proxy or override) |
| I-UI4 | Paired browser through nginx (EU) | same-origin works, WS `wss://host/ws` |
| I-3D1 | `browser_snapshot` emits a `graph` block → containers + relations | tree appears in GraphKit, Overlaps/Above edges correct, labels bounded |
| I-3D2 | `android_ui_snapshot` likewise on the emulator Messaging screen | same |
| I-3D3 | Cross-link a screen graph to the project's code/text graph | `SimilarTo` edges (lexical ceiling documented) |
| I-3D4 | 2 400+ nodes screen/workspace in the 3D view | frame rate / LOD (real GPU) |
| I-MOD1 | Browser/android/farm screenshot → Image modality ingest | container + ripple |
| I-MOD2 | EU screenshot with non-Latin text | fonts installed or documented degradation |
| I-MOD3 | `.png` evidence from `docs/evidence/` ingested as a project file | Image pipeline output |
| I-MCP1 | Registry endpoint == listening port, **per host** (local + EU) | no `Connection refused` |
| I-MCP2 | Checksum of deployed `tools/*` + registrar vs repo, per host | identical or listed drift |
| I-MCP3 | Port uniqueness across supervisor facts, server defaults, docs | no duplicates (3279/3280/3281 missing from facts ⚠) |

## 7. Prioritised fixes (not done by me; owners suggested)
| P | Fix | Owner |
|---|---|---|
| P0 | **EU:** copy the repo's `register-browser-tools.mjs` to `/opt/ozone-tools/`, re-run `farm` with the internal token; verify `farm_status` through EU `/mcp/call` (currently `Connection refused`) | zcode (it is working on EU) |
| P0 | Operator decision on inner repos: Browser-Farm commit `b603436` and any staged `tools/bridges` gitlink vs the standing "don't touch / don't commit" rules | operator |
| P1 | R2 + R3 (re-propose cooldown, idempotent Proceed); R1 (cascade → notification) | zcode (its module) |
| P1 | `vite.config.ts` dev proxy or documented `OZONE_HOST_URL` override (C8 regression) | zcode / UI |
| P1 | Inbox panel (R4) | UI |
| P2 | Mirror `context.workspace_id/project_id` in `/mcp/call` (one block in `grpc/mod.rs`) | engine |
| P2 | `graph` block from `browser_snapshot` + `android_ui_snapshot` | cc (MCP side) |
| P2 | Supervisor facts for 3279/3280/3281; fix `TOOLS_REGISTRY.md` ports | cc |
| P3 | Generic MCP-failure actor; S13 into the consciousness review pass; drop `farm_command` from the whitelist; EU fonts | engine |
