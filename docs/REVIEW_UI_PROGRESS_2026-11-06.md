# UI review — everything that should be visible while a prompt runs: steps, markers, animations (2026-11-06)

Operator: "full UI review to ensure all is captured to display steps or markers … review markers … ensure all steps, all animations in the UI during prompting." **REVIEW ONLY except L23/L23b (below).** Evidence = code read this pass (host emit sites, `/ws` handler, UI consumers) + docs/UI_ANIMATION_DESIGN_2026-11-03.md (verified there, re-checked here). Status: LIVE · PARTIAL · MISSING (host does not emit) · NOT RENDERED (host emits, UI ignores).

## 1. What a prompt does vs. what the user can see
A walk is 15 stages (0–14). Measured: Stage 2 alone is 6 min–30 min+ for a one-sentence request (≥34 sequential model calls; see REVIEW_PROMPT_LATENCY). During that time the UI can show only: the elapsed timer + "Thinking… m:ss · working — no task record yet (early stages)" (`ThinkingRow`), the panel's "Stage 2 running" (inferred as "next after the last completed stage"), and `pipeline_progress` "text Running". **For tens of minutes the user sees no evidence of what is happening or that anything is progressing** — the host knows (it has the calls, heartbeats, retries); nothing carries it to the UI.

## 2. Signal inventory (what the host emits → transport → UI)
| Signal | Host producer | Transport | UI consumer | Status |
|---|---|---|---|---|
| Stage completion (15 stages) | `record_stage[_timed]` → `orchestration_events::emit` | `/ws` frame `orchestration_stage` {stage, name, success, summary, duration_ms, project/workspace} | `OrchestrationStatusPanel`, `DiscoveryEffects` | LIVE — **completion only; there is no stage-START frame** (current stage is inferred) |
| Sub-stage markers | `emit_marker` (vocabulary `marker_info`: 15 codes) arrives as stage-5 rows "AMT — <name>" | same frame | panel rows | PARTIAL — see §4 |
| Pipeline invocation | executor progress map, polled every 500 ms | `pipeline_progress` {status Queued/Running/Completed/Failed/Cancelled, progress_percent 0 or 100, task_id, step_index, tokens} | panel chips, `TaskDetailPanel` | LIVE — 0/100 only (honestly not a smooth bar) |
| Graph writes | `GraphEventHub` (universal choke point) | `graph_event` {created/updated/deleted/linked/context_trimmed/session_context_trimmed, container_id, type, keywords} | `CreationActivity` (chips + glow), 3D/graph views | LIVE; **`context_trimmed` types exist in `graphEventClient.ts` but NO component renders a trim** — NOT RENDERED |
| Node/edge construction | modalities `POST /orchestration/node-event` | `node_event` | GraphWorld3D construction pulses, views | LIVE — **25 modality pipelines post it** (the 11-03 design doc said 3D only: stale; E6 is done) |
| Task steps | `/task/get` poll (5 s, `useChatSession`) | HTTP poll | `ThinkingRow` ("step k/N — last action"), `StepExecutionList` | PARTIAL — exists only after Stage 9 creates the task |
| Pause / awaiting | `PAUSED:`/`AWAITING:` markers, `/task/pause_info`, `/task/resume_run` | poll | `PausedBanner`, `AwaitingBadge`, thinking row labels | LIVE (poll, no WS frame) |
| Final answer extras | `/orchestrate` response: `thinking_log`, `attempt_trail`, `refusals`, `amt_summary`, `model_used` | response | `TranscriptEntryMeta`, `chatRendering.tsx`, `TaskDetailPanel` | LIVE — **post-hoc only** (not while running) |
| File/object cursor | `pipeline_location` (`[[PROGRESS file= op=]]`) | **host never forwards it** (`grep -c pipeline_location src/grpc/mod.rs` = 0) | UI type only | MISSING |
| Span-out | `tracing::info!` only | none | none | MISSING (design E1) |
| Run state (running/paused/awaiting/resumed) | polling only | none | — | MISSING as a frame (design E3) |

## 3. Signals the host HAS but the UI never sees (found by this review — these are what "all steps" means)
| # | Hidden activity | Where it lives today | Why it matters in the UI |
|---|---|---|---|
| H1 | **Stage 2 sub-progress** (section/paragraph/sentence/modality events, each with 5× order + 5× boundary votes) | text pipeline loops; only stderr/ledger | the 6–30 min "black box" |
| H2 | **Model-call lifecycle**: start, streaming heartbeat (30 s, new), retry/"unusable", served model (router member), proven-model pick, fallback attempt trail, skipped/quota | prompt binary stderr (not surfaced on success), `WALK_PROGRESS` (task-local; only the watchdog/final response read it), ledger | shows the system is working; explains slowness; makes timeouts visible |
| H3 | **Liveness**: `model_calls/model_activity.stamp` (new) — "last model progress N s ago" | file mtime read by the watchdogs | the exact signal that now keeps long walks alive; trivially renderable |
| H4 | Quota / limiter waits / daily free budget | `openrouter_quota.json`, limiter bucket | "you have 131/1000 free calls left today" |
| H5 | Context trims and (future) chunked calls "part k of n for this model's window" | `context_trimmed` graph events, ContextRecord | model-agnostic chunking should be visible: *"this call was split into 4 parts for a 4k model"* |
| H6 | Analysis reuse: output carries `analysis_reused: <key>` (new) | text output JSON | "reused stored analysis — no model calls" |
| H7 | Cross-modal passes (now once per pair): pass number, new links | `graphs.rs` loop | step visibility for Stage 4b |
| H8 | Provider steps / tool outcomes / health | `execute_provider_step`, `tool_health.json`, `capability_gaps.json` | provider = a step type; failures/gaps should appear in the run |
| H9 | Methodology sufficiency review: queued → judged → new/modify gaps raised → methodology updated | `methodology_reviews.json`, meta loop logs | the meta layer's work is invisible |
| H10 | Span-out (E1) — items, scores, hops, fan-out lens | `span_out.rs` | the per-modality fan-out animation |
| H11 | AMT growth (E5), blueprint "forge" finale, step wave | various | designed, not built |

## 4. Markers — what exists, what fires, what the UI does
Vocabulary (15, `marker_info`): `[5.enrich] [5.xref.1] [5.xref.2] [5.xref.3] [5.xref.4] [5.lanes] [5.think] [6.tools] [7.0] [8.1] [8.2] [8.3] [8b] [10.step] [13.0]`.
**Emitted (call sites):** `[5.enrich]` (amt.rs:3306, 3611), `[5.xref.1]` (amt.rs:3891), `[5.xref.4]` (amt.rs:4026), `[5.think]` (mod.rs:5175), one more at graphs.rs:1001. **Defined but NEVER emitted:** `[5.xref.2]`, `[5.xref.3]`, `[6.tools]`, `[7.0]`, `[8.1]`, `[8.2]`, `[8.3]`, `[8b]`, `[10.step]`, `[13.0]` (grep: 1–2 hits each = the vocabulary entry only) — i.e. blueprint/tools, simulation, gate sub-steps, per-step start and delivery have NO markers. UI: markers arrive as ordinary stage-5 frames ("AMT — name") and render as extra rows in `OrchestrationStatusPanel`; there is no marker-specific component. Marker semantics are sound (named + described, never a bare number) — the gap is coverage.

## 5. Animations — inventory vs design
**Exist (verified by grep):** thinking dot + panel `pulse` (1.2 s), `StepExecutionList` active-step pulse + color/opacity transitions, `CreationActivity` fade-slide chips with glow, `DiscoveryEffects` celebration flourishes (from stage summaries), GraphWorld3D construction pulses/camera fly-to, LivingNetworkView, ThreeDEngine, `graphkit/indicators`, 145 declarations in `App.css`.
**Designed in UI_ANIMATION_DESIGN (11-03) and NOT built (host/UI grep = 0):** E1 `span_out` frame + trace/step ids on frames (E2), Fan-out Lens (per-modality wavefront), E3 `run_state`, E4 missing markers + `BlueprintForge` finale, E5 AMT growth + `AmtGrowth` living tree, E7 `pipeline_location` forwarding, step wave from `pipeline_progress`, `prefers-reduced-motion` guard (**0 occurrences in `ui/src` — no reduced-motion handling at all**; an accessibility gap independent of the design).

## 6. Proposal — one honest live feed for the whole walk (design; not built)
New host frame family on the existing `/ws`, fire-and-forget (same hub pattern, never blocks the walk), plus ONE route for nested processes to post into it (the existing `POST /orchestration/node-event` pattern):
- `stage_start {stage, name}` (completes the stage timeline; kills the inferred-"current" guess)
- `sub_progress {stage, label, done, total?, detail}` — posted by the text pipeline per validated event (H1), by Stage 4b per pass (H7), by span-out (E1), by the sufficiency review (H9)
- `model_call {call_site, requested, served?, state: start|streaming|retry|failed|done, elapsed_ms, tokens?, part?: {k, n}}` — the host wraps pipeline-9 execution (it already has `WALK_PROGRESS`/ledger fields); the prompt binary's heartbeat can be forwarded by the host's child-stderr drain (it already forwards complete stderr lines to tracing and parses `[[PROGRESS …]]`)
- `liveness {last_model_activity_ms_ago, quota_remaining?}` every few seconds (H3, H4)
- `run_state` (E3) and `span_out` (E1) exactly as designed.
UI: a compact "what is it doing right now" line under the thinking row (current stage + sub-progress + last model call + liveness), the panel gets a stage-start state and a live sub-progress bar where `total` is real (never a fake smooth bar — keep the honesty rule), `context_trimmed`/chunk parts rendered as chips, `analysis_reused` shown as a chip, a reduced-motion guard, markers for the 10 un-emitted codes.

## 7. Edits made in this review (UI/host plumbing for the cap question) — L23b
Operator: the status route should return the full result ("why doesn't it? it should"). **Done (uncompiled/untested):** `GET /orchestrate/status` now returns `result` = the same object the blocking `POST /orchestrate` returns (`orchestrate_response_from_output`, `src/grpc/mod.rs`); Electron's `orchestrate` IPC now SUBMITS (`submit_only`) and POLLS `/orchestrate/status` every 2 s with short requests (10 s each), returning `s.result` — so no HTTP request stays open for the walk and there is no wall-clock cap; a lost run record or ~60 s unreachable host rejects the promise and the chat's existing task-recovery poll takes over (its ceiling is now a sliding stall limit, L23). `node --check` + `tsc --noEmit` pass; needs a host rebuild + Electron restart.

## 8. Tests to write (real host, no mocks)
`/ws` contract test: run a real `/orchestrate`, assert `stage_start`, `sub_progress` (Stage 2), `model_call` and `liveness` frames arrive in order; UI node-script replay of recorded real frames (`docs/evidence/ws-captures/*.ndjson`); screenshot via the browser-farm profile at t=0 / Stage 2 mid / done; reduced-motion emulation; poll-path test: a walk >1 min returns the full response object through submit+poll identical to the blocking call.

## 9. Open decisions for the operator
1. Build the unified live feed (§6) before the Fan-out Lens animations, or together? 2. Should model-call detail (served member, retries) be shown to every user or only in an expandable "details" row? 3. Show the daily free-quota counter in the status bar? 4. Is `prefers-reduced-motion` support a must for the animations wave (recommended)? 5. Stage-2 sub-progress granularity: per validated event (can be hundreds) or per paragraph/section roll-up?

## 10. EDITS MADE AFTER THIS REVIEW (operator: "why would it reject only after 60 s and no retry? continue with all edits all") — all UNCOMPILED (host) / `node --check` + `tsc --noEmit` pass (UI)
| # | Edit | Files |
|---|---|---|
| L24 | **Poll/submit are retries, not give-ups.** Submit: a connection failure BEFORE the host could have received the request (refused / unreachable / DNS) is retried with backoff (up to 10 tries ≈ the host's start-up); an ambiguous failure (timeout/reset after sending) is NOT retried blindly (could duplicate a walk) → rejects and the chat's task-discovery recovers. Poll: an unreachable host is retried FOREVER with 2 s→15 s backoff (waiting is not a failure while the host is away). A LIVE host answering `found:false` (restarted, lost the in-memory run record) can never produce the run again → reject IMMEDIATELY so the chat recovers from the durable task record (the old "~60 s of misses" rule is gone). | `ui/electron/main.js` |
| L25 | **`liveness` frame** every 3 s on `/ws`: `{last_model_activity_ms_ago, free_quota_remaining, timestamp}` (H3, H4); UI type + `onLiveness` listener; `ThinkingRow` subscribes and shows "· last model activity N s ago" (turns amber "no model activity for N s" after 120 s) and "· N free calls left today". | `src/grpc/mod.rs`, `ui/src/graphEventClient.ts`, `ui/src/components/ThinkingRow.tsx` |
| L26 | **Ten missing markers now emitted** via `emit_marker` (they surface as stage-5 style named rows): `[5.xref.2]` (synthesis, amt.rs), `[5.xref.3]` (persisting, amt.rs), `[6.tools]` (with applicable tool count), `[7.0]` (simulation start), `[8.1]` (gate assembling), `[8.2]` (gate deciding, per attempt), `[8.3]` (gate decided + decision), `[8b]` (jurisdiction full gate), `[10.step]` (per step: index + pipeline), `[13.0]` (delivery). | `src/orchestrator/stages.rs`, `src/orchestrator/amt.rs` |
| L27 | Boot-time `model_ledger::init()` so `model_stats.json` exists from the start | `src/model_ledger.rs`, `src/lib.rs` |
Marker coverage after L26: all 15 vocabulary codes have an emit site except `[5.xref.1]`/`[5.xref.4]`/`[5.enrich]`/`[5.think]`/`[5.lanes]` which already had them. NOT built (still design/captured): `stage_start`, `sub_progress` (Stage 2 K-algorithm), `model_call` lifecycle frames, `run_state`, `span_out`, forwarding `pipeline_location`, reduced-motion guard, Fan-out Lens/AMT-growth/BlueprintForge animations — §6 and §9.
