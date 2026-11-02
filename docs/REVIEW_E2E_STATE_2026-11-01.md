# Full-state review and edit TODO — 2026-11-01 (cc)

Status of this document: REVIEW ONLY. No source edits, builds or tests were run for it. Evidence tags: **[code]** read in source, **[log]** read in `target/release/host.log`, **[fork]** reported by a read-only review fork and spot-checked where marked.

Trigger: the operator's run "Create a clock in Blender" showed `Error: Error invoking remote method 'orchestrate': Error: timeout`, a "review pending — needs human review" feed with no prompt, no thinking/progress in the project chat, and chats that do not survive a restart.

## 0. Ground truth now
- Local host is **not running**. `target/release/ozone-studio` is dated Nov 1 08:04 (rebuilt after the Oct 30 binary the operator was fighting). HEAD `d5678b2`; one uncommitted edit (`ui/src/hooks/useChatSession.ts`, sessionStorage → localStorage) [code].
- Host log (23 MB) shows the two failing runs **finished server-side**: task 941 in 544 s, task 943 in 795 s, both "all 14 stages finished" [log]. The UI said "timeout" while the walk succeeded.

## 1. Root causes of the three reported symptoms

### 1.1 "Error: timeout" — client budget shorter than the walk (HIGH)
- `ui/electron/main.js:423-425`: `/orchestrate` gets 300 000 ms. Real walks take 9–13 min on a rate-limited free pool [log]. The Electron request rejects with the bare string `timeout` [code].
- `MetaPortion.tsx` has **no** timeout recovery (grep: none). The recovery poll exists only in `ui/src/hooks/useChatSession.ts` (project chat) and only works if its 900 ms discovery poll happened to see a "running" task.
- The server is already detached [code `grpc/mod.rs:3148`], so the work continues. The loss is purely client-side.
- Fix shape: (a) raise the `/orchestrate` client budget well above a worst-case walk (≥ 45 min) AND (b) put the same recover-by-task-id path into MetaPortion via one shared function, and never render "Error" for a task that is still running. Better: a submit-then-poll contract so no HTTP request stays open for minutes.

### 1.2 "Confirmation needed" never asks, and the walk proceeds anyway (HIGH)
- `stages.rs:1823-1853`: only `"Decline"` returns an error. `ReviewPending` (and a missing decision, via `unwrap_or("Proceed")`) falls through; stage 8 is recorded `success=true`, the walk continues to step execution and delivery. Log lines confirm: `Stage 8 … Decision: ReviewPending [gate-reported]` followed by `delivery … gate decision = Some("ReviewPending")` [log].
- Why it is pending: `decision_review.rs:120-157` merge — Proceed needs every chunk judged. In the failing run chunks 1 and 2 said proceed, chunk 3 produced no judgment (HTTP 429 `free-models-per-min`, empty/confetti), so the merge is `review_pending` [log 81456, 82585].
- The jurisdiction path is different and correct: an unresolved RequireConfirmation holds the request (`jurisdiction.rs:608`). Only the consciousness gate (stage 8) fails open.
- Nothing surfaces the pending item as a prompt: only a count chip (`WorkspaceStatusStrip` polls `/consciousness/reviews/pending`) and the coordination feed. No resolve UI exists [code grep: only one consumer].
- Doctrine the operator stated: consciousness/meta decisions are made by consciousness/meta, not a human. A failure to judge must not push through. A human is the **last** fallback, and only after the meta path is exhausted.
- Fix shape: for gate kinds `consciousness` and `meta`, re-run only the unjudged chunks through the fallback chain with rate-limit backoff until judged (bounded); if still unjudged, **pause** the task (`awaiting_confirmation`, resumable, visible in Order) — never continue past it. Jurisdiction RequireConfirmation keeps the human path but gets a real modal with Proceed/Decline wired to `/consciousness/reviews/resolve`, and the walk resumes on resolve.

### 1.3 Project chat shows no thinking, progress or coordination (MED)
- `ProjectChatPanel.tsx` is 88 lines. It renders only content. MetaPortion renders, near the transcript and input: `OrchestrationStatusPanel` (stage checklist), `DiscoveryEffects`, `McpActivityIndicator`, `ChatNotificationStream`, the in-transcript "Thinking… m:ss · step n/N" row, the collapsible Thinking log, the attempt Trail with `summarizeTrail`, model-accent bubbles, emotion/consciousness meta [code].
- `useChatSession` already stores `thinkingLog`, `attemptTrail`, `refusals`, `amtSummary` per entry but the panel discards them [code].
- The project chat also hardcodes `consciousness_enabled:false`, `user_id:1`, `device_id:1` (server ignores the first; the ids should come from the auth session).
- Scope of fix (direction: copy missing indicators FROM MetaPortion TO the project chat, never the reverse): extract the shared pieces into `chatRendering.tsx`/shared components and mount them in the project panel with `projectId` as the event filter. See §4 for the item list.

### 1.4 Chat does not persist across restart
- Persistence key `ozone-chat:{project}:{workspace}` in `useChatSession.ts`; the working-tree edit moves it to `localStorage` (survives restart). MetaPortion's own transcript persistence: not verified this pass — needs a check and the same treatment, keyed by chat context.

## 2. Rate limits (OpenRouter free pool) (HIGH, operational)
- 631 lines match 429/rate-limited in the host log. Message: `HTTP 429: Rate limit exceeded: free-models-per-min` [log 82112].
- Current handling: `mod.rs:3033` treats 429 as `MoveOn` after a 2 s pause. Each candidate burns a request; with parallel lanes, chunked reviews and loops sharing one account-wide per-minute limit, bursts fail repeatedly. `openrouter_quota.rs` tracks the daily count only [code].
- Fix shape: one shared token-bucket for the free pool keyed by the account's per-minute limit, acquired before every OpenRouter attempt by walk, lanes, loops and reviews; honour `Retry-After`/reset headers; queue instead of failing; record waits in the ledger. Pair with §1.2 re-run of unjudged chunks.
- Second failure class seen in the same run: `finish_reason=length` with `content:null` and large `reasoning` (reasoning models spend the cap) [log 82537]. Existing handling labels it `ozone-studio` and moves on; raising per-call output budgets for reasoning models or preferring non-reasoning free models for JSON-only calls is a tuning item.

## 3. Consciousness subsystem (fork audit, [fork] unless noted)
- I-Loop: identifies only. Reads unhandled AMT candidates (zero-cost skip when idle), makes one model call, writes a reflection JSON and one CoordinationEvent. It does not create tasks, edit the AMT or modify code. The guide's "every 60 s" and "observe/reflect/evaluate/integrate/plan" are not what the code does (floor is 1800 s; single reflection prompt).
- Emotional flow in the store is real (task lifecycle → store → assistant digest). The **UI emotion circle is broken**: `MetaPortion.tsx:191` executes pipeline 40 (`GetCurrent`), which has no crate and which `/pipeline/execute` rejects as a consciousness-category id → UI can only show defaults. Needs a read route over the store.
- Stage 12 reports honest failures: pipelines 41/43/47 are registered through aliases but **no binary exists** in the shared build dir.
- Of the 16 pipelines in the guide: 5 aliased (39,41,42,43,47), 11 registered without any crate (40,44–46,48–54); 12 crates exist with no registry id. 39 is intercepted by `DecisionReviewExecutor` so its missing binary is harmless.
- Dead toggles: `identity_system_enabled`, `relationship_system_enabled`, `ethical_system_enabled`, `collective_enabled`, `show_decision_reasoning`, `show_experience_retrieval`, `share_experiences_collective`, `anonymize_shared_data`, `playback_enabled` have no runtime reader. `show_emotional_state` is not read by the UI.
- The Consciousness Guide needs a correction pass (stale counts, interval, pipeline list, "restart required", directory layout).

## 4. Bottom status bar (fork [fork], findings numbered as reported)
1. P2P renders in four places; `StatusBar.tsx:72` shows "Connected" on `peerCount>0` even when no peer is paired (overstates trust). The strip's `PeerState` is the only place that knows paired vs connected.
2. ZSEI count and "Depth: N" read `config.stats.zsei_*`, which no backend field provides → always 0; tooltip claims "traversal depth" falsely. None of the three depths (graph parent-chain, image relative depth, depth modality) is shown.
3. Contribution blocks: honest zeros, but displayed as real; mark "not tracked yet".
4. "I-Loop: Running" is hardcoded from the config flag, not loop state (fabricated state).
5. Task count has two sources (health push and local ±1) that can disagree.
6. Quota/ledger poll `fetch(${OZONE_HOST}/…)` directly, not bridge-first (`hostGet`), so under Electron `file://` they may silently fail.
7. Ledger chip is all-time; "UI Mem %" is heap ratio; uptime title wrong.
8. Task / Gate / CapturePresence / Loop chips and ActivityPulse exist only in `WorkspaceStatusStrip`.
- Plan: the bar owns system-wide state (connection, peers+trust, quota, ledger, tasks, gate, loops, capture); the strip keeps only project-scoped pieces. Move the shared chips into `graphkit/indicators`; one poll per route via the existing `usePolled`.

## 5. MCP / methodology / tool-call correctness (partial; the rest needs the E2E run)
- Tool routing is default-off (`OZONE_TOOL_ROUTING=1`); the blueprint planner coerces steps to pipeline 9; `capability_ids` is not built, so no orchestrated run can select a browser/farm/android tool today (agents call `/mcp/call` directly). [docs]
- Methodology count and base methodology creation were verified live earlier; per-call correctness of every methodology call site is part of the E2E sweep in §7 (not re-audited in this pass).
- Registered MCP endpoints must be re-registered after the browser/farm port move (3280/3281); verify at first host start.

## 6. Voice box
Read-only fork result [fork]; nothing built or run.
- ASR is real: `assets/pipelines/general/voice/main.rs` (`TranscribeFile`, whisper.cpp CLI and whisper-rs paths, API fallback, 16 kHz ffmpeg); `VoiceConfig` at `src/config/mod.rs:891`; model `~/ozone-models/whisper/ggml-base.en.bin` and ffmpeg present. Compile/run of the whisper-rs binary unverified.
- **TTS is a stub that reports success:** `generate_tts` (`voice/main.rs:571`) returns the literal `"base64_audio_data"`, so `Speak` says `success:true` with fake audio. Same fabricated-success class the repo is removing elsewhere. Fix: real engine or loud `success:false`.
- Voice identity: `src/orchestrator/voice.rs` (52 lines) persists a container only. UI input (mic meter, 16 kHz WAV capture) is real in MetaPortion; output `speakResponse` is gated on consciousness + `output_enabled` and plays the fake string. `ProjectChatPanel` has no voice at all.
- No local TTS/ASR engine installed (piper, espeak-ng, whisper-cli, festival absent; no pyttsx3/faster_whisper). pip reachability for `piper-tts` unconfirmed.
- Consciousness-response record exists only as a text suffix in the assistant digest. V1 (differentiated text) is otherwise complete.
- Minimal V2 slice: piper in a venv (~150 MB RSS) + ~60 MB voice, real `generate_tts`, rate/warmth mapped to length-scale, run behind the single-flight local gate and never concurrent with a walk (7.6 GB box).
- Record `CONSCIOUSNESS_RESPONSE` {response_id, ts, response_type:"consciousness", channel, content_text, audio_ref, voice_identity_id, voice_policy, emotion_state (from ConsciousnessStore), graph_context, self_heard:null}; persisted as an insight-pattern container with the body behind `object_store_path`; WAVs in `zsei_data/voice/`. Written by assistant/I-Loop only, never by the orchestrator reply.
- Shared hook `useVoiceIO` (extracted from `MetaPortion.tsx` :335-520) used by both chats; new read route `GET /consciousness/responses?scope=`.
- Edit order: (1) real `generate_tts`; (2) `src/consciousness/response_record.rs` + assistant/I-Loop writers; (3) responses route; (4) `useVoiceIO`; (5) both chats render consciousness entries; (6) guide updates.
- Test rows VB1–VB8: Speak without engine → `success:false`; Speak with engine → decodable WAV, rate change reflected; whisper round trip; check-up writes the record with store emotion; orchestrator reply has no record; both chats show the same entry; output disabled → no audio call; RAM under the floor with TTS+whisper and no walk.

## 7. E2E test paths to establish (all before any more claims; nothing run yet)
Existing authority: `docs/MASTER_TEST_GUIDE.md` (L0–L13). Additions required by this review:
- T-TIMEOUT: a walk > 5 min (inject delay or use a real heavy prompt) — UI must show recovery, then the final answer; no "Error".
- T-GATE-PENDING: force one chunk unjudged (429 or empty) — consciousness gate re-runs it; if exhausted the task pauses, never delivers.
- T-GATE-JUR: RequireConfirmation prompt — a modal appears, the walk resumes only after resolve; decline stops.
- T-RATE: burst of N parallel lanes against the per-minute limit — zero HTTP 429 reaching the walk; waits visible in the ledger.
- T-PROJ-CHAT-PARITY: run the same prompt in MetaPortion and the project chat — same stage checklist, thinking row, trail, notifications; scope differs only.
- T-CHAT-PERSIST: send, restart the app, both chats still show history (separate keys per context).
- T-STATUSBAR: peers/trust, tasks, gate, loops, quota, ledger, capture present in the bar; no fabricated values.
- T-EMOTION: emotion circle reads the store; value changes after a completed task.
- T-STAGE12: experience/relationship/emotion dispatch no longer reports "binary not built".
- Workspace/tool/MCP/modality sweeps: reuse MASTER_TEST_GUIDE §7, §9, §10, §11 as written; register results per row.

## 8. Edit TODO (ordered; edits only after this capture; no builds/tests until all captured)
**P0 correctness (blocks trust in every run)**
1. Gate semantics: stage 8 must never continue on ReviewPending/missing decision; re-run unjudged chunks with backoff for consciousness/meta kinds; pause-and-resume for exhausted/human cases (`stages.rs`, `decision_review.rs`).
2. Orchestrate client contract: raise/replace the 300 s budget; shared recover-by-task-id used by MetaPortion and the project chat; never display "Error" for a running task (`ui/electron/main.js`, `MetaPortion.tsx`, `useChatSession.ts`).
3. Shared OpenRouter per-minute limiter with Retry-After handling (`src/orchestrator/mod.rs`, `openrouter_quota.rs`).
4. Human-confirmation UI: pending-reviews modal/inbox with Proceed/Decline wired to `/consciousness/reviews/resolve`, resume on resolve.
**P1 parity and honesty**
5. Project chat: mount `OrchestrationStatusPanel`, thinking row (elapsed + step), Thinking log, Trail, `DiscoveryEffects`, `McpActivityIndicator`, `ChatNotificationStream`, coordination activity scoped by `projectId`; take ids from the auth session; shared components extracted from MetaPortion into `chatRendering.tsx`.
6. Chat persistence in MetaPortion (per context) to match the project chat (localStorage).
7. Bottom bar per §4: trust-aware peers, real depth or "—", real I-Loop state, one task source, bridge-first fetches, chips moved from the strip; strip keeps only project-scoped indicators.
8. Emotion UI: new store-backed read route + MetaPortion points at it; honour `show_emotional_state`.
9. Stage 12: route experience/relationship/emotion dispatch to the host store or build the three binaries; remove the "binary not built" class.
10. Consciousness toggles: wire or delete the dead keys; correct the Consciousness Guide.
**P2**
11. Reasoning-model output budgets / model preference for JSON-only calls.
12. Registry/MCP re-registration check at host start; `capability_ids` design remains open.
13. Voice box V1 per the voice fork's plan (below).
**Operator-gated**: local rebuild/relaunch (disk 93%, RAM), EU deploy, commit/push, key rotation, data mutations.

## 9. Open questions for the operator
- Pause semantics: when the meta path is exhausted, should the task pause for a human, or fail with a retry button? (This review assumes pause-and-resume.)
- Rate limiter: one account-wide bucket assumed (20/min free); confirm the plan tier changes the number.
- Which chat owns notification toasts when both are visible?

---
## 10. Awaiting-item doctrine and design (fork result, adopted; operator ruling: meta/consciousness gates are meta-only, wait until judged, never fail open, never ask a human)
Fail-open sites inventoried [fork]: stage 8 (`stages.rs:1823-1853`), stage 14/8b plan check discards confirmation matches (`jurisdiction.rs`), jurisdiction confirmation `ReviewFailed` fails open (`jurisdiction.rs:~819`). Feeding them: decision_review merge never re-runs an unjudged chunk (`decision_review.rs:120-157`); no per-minute limiter; no `awaiting` task state anywhere (`/order/global` buckets live/paused/queued/interrupted/done only); boot reconciliation turns stranded tasks into `interrupted` with no resume. Bounded give-ups with no resume: amt_loop (`failed_structural`, `failed_after_max_attempts`), meta_loop (10 attempts).
Design:
- Record `{await_id, task_id, kind: consciousness_gate|meta_gate|jurisdiction_confirm, resolver: meta|human, stage, plan_ref{blueprint_id, amt_container_id}, unjudged_chunks[], judged{idx:decision}, attempts, next_try_ts, last_error, escalation_level, status: awaiting|resolved|cancelled, created_ts, trace_id}` in append-only `{data_dir}/model_calls/awaiting_items.jsonl`.
- New durable task state `awaiting` (boot reconciliation preserves it; never maps it to `interrupted`).
- Gate returns a typed `Awaiting(await_id)` instead of continuing; judged chunks are saved, only unjudged ones are redone.
- Drain: meta-loop pass plus wake on append; re-runs only unjudged chunks through the fallback chain behind the shared per-minute limiter with escalation; retries forever with backoff ceiling (≈30 min); per-pass bound only defers (an `awaiting_deferred` count stays visible); Decline fails the task with the reason; all judged and Proceed resumes the same task at the gate stage from the persisted AMT/blueprint (never rebuild).
- Human path only for `jurisdiction_confirm` (modal → `/consciousness/reviews/resolve` → resume). Jurisdiction `ReviewFailed` becomes an await. Stage-14 confirmations are retained and awaited.
- Visibility: `/order/global` gains an `awaiting` bucket (kind, resolver, attempts, next try, last error); gate chip counts awaits; timeout recovery treats `awaiting` as running.
Test rows AW1–AW10: one chunk 429 → await, no delivery; drain merges and resumes same task; hours-long outage never drops; restart keeps `awaiting`; Decline during drain fails with reason; meta awaits never create a human prompt while jurisdiction does; `ReviewFailed` awaits; Order bucket fields; UI recovery shows no "Error"; stage-14 confirmation retained.

## 11. UI/chat parity and awaiting rendering (fork result)
Parity (MetaPortion has, project chat lacks): stage checklist, discovery/MCP/notification components, thinking row, thinking log, attempt trail + `summarizeTrail`, model picker (hook sends no `model_config`), emotion orb, I-Loop widget, voice. MetaPortion lacks the project chat's timeout recovery; MetaPortion persistence is in-memory only (`transcriptsByContext`). Project chat extras to keep: project scoping, `zseiProjectId` handoff, recovery poll.
Extraction plan: `components/OrchestrationPanels.tsx` (status panel, discovery, MCP indicator, notification stream; props `{isRunning, scope}`; events filtered by `proj:<id>`); `ThinkingRow.tsx` / `TranscriptEntryMeta.tsx`; one `useChatSession` engine for both chats (modelConfig, contextKey persistence, recoverTaskResult); `useVoiceIO`; `useEmotionState` over a new store read route.
Awaiting rendering: `TaskStateIcon` has no awaiting state (falls to gray clock). Add `awaiting_meta` (hourglass: waiting on a consciousness/meta judgment, retrying) and `awaiting_human` (jurisdiction confirmations only); Order active group with reason chip + age; TaskDetailPanel reason, pending ids, Resume / Proceed-Decline; chats show "Waiting on <meta judgment | your confirmation>…" never "Error"; gate chip lists awaiting counts.
UI rows U17–U25: chat parity; >5 min walk recovery; awaiting visible everywhere and Resume works; jurisdiction modal Decline/Proceed; reload keeps both chats; each of the nine engine views with a fixture; status bar content with no always-zero fields; emotion orb changes after a task; live node-event pulse in 3D.
Electron vs web: the web harness cannot exercise bridge paths (IPC timeouts, file pickers); Electron has never been driven in tests, so add one real Electron run before closing U-rows. Harness: `ui-harness.mjs` port 8767, `OZONE_BROWSER_ALLOW_HOSTS=localhost`, `OZONE_BROWSER_ALLOW_GLOBALS=OZONE_HOST_URL`. Reachability: only 9 engine lenses exist; other modalities are reachable only through the generic "all modalities" lens.

---
## 12. Living graph / acting chain (fork result; evidence from host.log through ~Nov 1 12:00)
Chain: any write → `ZSEI::query` → `graph_events::emit` → GraphEventHub → {ripple-sync (project/file scope) → GraphRipple candidates; actors (global events) → Security/Coordination/TaskFailure/CodeStaleness/McpFailure candidates; P2P bridge (only if `auto_broadcast_sync`); UI ws}. Sources: file beacon 45 s, task lifecycle, `/mcp/call` mirror, note/claim mirror, sensor tick 900 s. Candidates → `AMT_WAKE` → amt_loop (1 LLM call per candidate, ≤12/pass) → writes → more ripples. Observers without tools: I-Loop, assistant, meta_loop.
Evidence: beacon last real change 10:22 (container 40380; only one beacon container in the log); actors: 54 findings (39 coordination, 14 security, 1 staleness) and **none since Oct 31 00:50**; sensor ticks run (257 files) but the log prints failures only (38 lines, last Oct 29); amt_loop 6 passes since 08:00, candidate file 491 rows with **317 `failed_structural`**, 0 unhandled; I-Loop 5 reflections today and its own text diagnosed the repetitive graph-ripple/initial-AMT signal volume; assistant: every pass since 11:52 "findings unchanged (72) — skipped" so 72 findings sit unacted; peer-sync review 0 lines (no paired peers); context_mirror events have no edges.
Findings: (1) graph-ripple route is a feedback echo producing terminal `failed_structural` rows (HIGH); (2) assistant findings never change and nothing acts on them (MED); (3) actor chain silent since Oct 31, log cannot distinguish "nothing qualified" from a wiring break because sensor success is not logged (MED/LOW); (4) stage 8 fails open (HIGH); (5) mirrored events carry no edges (MED); (6) task-failed actor path never observed (untested).
Test rows: edit a registered file → beacon line ≤45 s + staleness candidate for code; failing tool via `/mcp/call` → one mirrored event, one finding, one candidate (dedupe on repeat), one amt_loop pass; sensor cycle → new `tool_call-sensor-tick-*.json`; failed task → `TaskFailure` candidate; burst of 10 identical failures → one candidate/one paid call; `kind=finding` note → `CoordinationReceived`; unchanged findings → "skipped", changed → one batched call; paired peer + `auto_broadcast_sync` → `peer_sync_candidates.jsonl` entry and a finding container; unscoped write → no ripple-sync candidate.

## 13. Modality ingestion and workspace (fork result)
Modality table (file_link detect → dispatch → state → MCP → viewer). Wired through `modality_pipeline_id`: Text, Code, Image, Audio, Video, Math, Chemistry, DNA, EEG, 3D, CAD, Kinematics, Control. Absent from file_link: Biology, Proteomics, Depth, EM, Geospatial, Thermal/IMU/Radar/Sonar/Hyperspectral (path hints host-side only), Haptic, Sound, BCI, Network, UiDocument (html/css classified "Web", metadata only). Data/Config/Web/Document/Archive: metadata only. LinkToModality handlers exist for 11 (`graphs.rs:18`); 16 are skipped. Dedicated UI viewers exist for Text, Code, Math, Image, 3D, CAD, Audio/Video (timeline), Geospatial; everything else only through the generic "all modalities" lens.
Defects re-verified: four extension tables disagree (file_link vs host canonical vs `graphs.rs` path hints vs UI positional `keywords[1]`); pdf/docx are "Document" in file_link (metadata only) but Text in the host table; `ts` is Code and Video (Code wins); `.svg` is Image in file_link and UiDocument in the host; `.sdf` is Chemistry (kinematics SDF unreachable); path hints misclassify paths containing `sar`/`imu`; UiDocument unreachable from file_link; MCP graph blocks tag `Unknown` for dna/chem/geo/mesh/proteomics/cerebrix/connectome/android/security/farm (`mcp_graph.rs:162`); direct persisters (image, geospatial, 3D) bypass the spatial pass; stage-4b passes `link_to_existing:false` while STEP 0 passes true; `Locates`/`DepictsSameEntity`/`RegisteredTo` have no producer.
Fixture matrix: Workspace A "Research"/project "Cell biology": notes.md, analysis.py, proof.tex, caffeine.mol, gene.fasta, scan.png, paper.pdf → expect one FileReference each with the right modality, ModalityGraph for wired types, SimilarTo among text/code/math, `[13.link]` after a walk, folder link reports skipped subdirectories. Workspace B "Build"/project "Clock": clock.blend (export clock.glb), model.step, arm.urdf, page.html + screenshot PNG, data.json, track.wav, demo.mp4, x.zip → 3D/CAD/Kinematics graphs, `Renders` edge screenshot→page root, metadata-only for zip, honest no-pipeline for Document/Data, no leakage between A and B.
Rows W-IN1..W-IN11: per-fixture modality/container assertions; same file in two workspaces → independent graphs; `ts`/`svg`/`sdf`/`pdf` expose table disagreements per entry path; folder skip reporting; text mention of radar refuses honestly; STEP 0 walk produces SimilarTo with provenance; ingest with `renders_container`; each MCP via `/mcp/call` records its container modality tag (expect `Unknown` today); viewers open per modality and MathViewer never renders `.md`; rename/delete → beacon ≤45 s; no link handler → no error, no edge.

## 14. Order, assistant, meta (fork result)
Works (code-read): eight order fields stored and returned; `/order/global` groups, scopes, sorts; assistant filter is free and local (meeting 2 h, stalled 24 h, paused 3 d, overdue escalation, cascade ≥3); executor opt-in with memory gate.
Defects: (1) **recurrence stored, never expanded** (HIGH); (2) **checklist cannot be created** — `enqueue_task` sets it empty, no API input; (3) **gcal/zoom sync not idempotent** (their own output says duplicates until the diff stage lands) and both LOCKED without OAuth, nothing ever ran live (HIGH); (4) due bucket labels anything due within the last 24 h as "today" while the assistant calls it overdue; no `due_at` → "upcoming" though the guide says someday; (5) `priority_rank` constant 0; (6) assistant does not create follow-up tasks from findings (guide §4.2 says it does; code only has proposals); (7) meta_loop floor/dedupe not re-run in this pass.
Rows OR1–OR12: todo with due +2 h in `today`; meeting with link → `meeting-soon` finding + proposal; follow-up child (expected FAIL, no checklist API); note promoted to todo; 3 overdue → one cascade finding; paused >3 d / interrupted groups; scope partition global/workspace/project; executor flag unset/low memory/with memory; digest unchanged-skip; recurrence (expected FAIL); gcal/zoom run twice (duplicates); bucket boundary disagreement.

## 15. Gate matrix (fork result)
Correct: pipeline-id coercion to 9 with consciousness-category refusal and SECURITY warn; `model_override` strips executable/endpoint fields and caps `max_iterations`; blueprint body path traversal refused; methodology compliance via resilient path; proposals whitelist = browser_open, browser_navigate, browser_screenshot, farm_screenshot (farm_command removed); `/pipeline/execute` refuses consciousness ids.
Defects: stage 8 fails open (HIGH); plan gate discards its own RequireConfirmation matches (MED); no jurisdiction/consciousness re-check per step (MED); `/mcp/call` and `rerun_step` accept absent/self-declared callers, role allowlists key on an unverified agent string (MED); tool-routing results labelled untrusted but not gated per step (MED); modality binaries (21 files) spawn the prompt binary directly, bypassing admission gate, ledger and quota (MED); `rerun_step` skips blueprint coercion (LOW-MED); unverified: `/orchestrate` session validation.
Rows TD-G1..TD-G12: stage-8 unjudged chunk re-run then pause; jurisdiction confirmation hold/resume/decline; plan gate hold; blueprint coercion with SECURITY log; reuse with missing body falls through; `/mcp/call` no-token and forged agent; tool routing untrusted block, stdio never selected; per-step Block rule stops; `rerun_step` with edited pipeline/model rejected; nested modality call leaves S12 row with trace and queues under pressure; loop calls show Loop tier; tampered proposal refused (already live-verified).

---
## 16. Logs, MCP logs, shared context, EU (fork result; local log from 10-29 on, host down since 11-01 12:10)
1. Stage 8 returns `review_pending` in 21 of 34 merges (62%); EU 1. Ten pending merges on the last day; every walk continued past them. Cause: unjudged chunks (#2).
2. Free pool: 38 429-lines on 11-01; 55 fallback failures = 42 http-error + 13 empty; 27 `HTTP 404: Provider returned error`, 11 `HTTP 429: Provider returned error`, 6 `free-models-per-min`, 2 503. **`nvidia/nemotron-3.5-content-safety:free` (a moderation model) failed 28 times as a fallback candidate** — exclude non-generation models from the chain. Ledger has 32 unreadable rows (first bad line 505, trailing characters) — a torn-write/format defect in `model_ledger.jsonl`.
3. amt_loop is wedged on container 40560 (the global findings AMT): 43 of 45 retry lines; only 5 "deepened" in 3 days; all 8 actor findings target 40560, so findings starve and nothing else benefits.
4. Whole-task failures: 32 `Task N failed: prompt pipeline returned an empty response`, 11 confetti refusals (2/3/4/8 candidates), 15 reasoning-cap warnings.
5. Consciousness post-execution dead: binaries missing for core_memory_formation, emotional_baseline_update, relationship_development, narrative_construction, i_loop, meta_portion_consciousness; stage 12 records ~0 ms.
6. `Pipeline 9 (prompt) binary not found — OZONE_PROMPT_PIPELINE_PATH left unset` logged at three boots though pipelines ran: misleading warning.
7. 38 jurisdiction files differ from shipped and were not overwritten (no manifest, no `OZONE_JURISDICTION_FORCE_SYNC=1`), logged 9 times: the expanded content and alias matching have not reached the live data dir.
8. I-Loop/assistant intervals clamped to the 1800 s floor (9 warnings each); I-Loop reflections themselves describe finding 3.
9. Restarts on 10-30 09:13 and 11-01 08:58 have no preceding shutdown line (unclean exit); gaps of 337/503/1603 min.
10. Sensor: only failures logged (9 local); EU shows 0 tick lines in 3 days, so EU sensor is unverified from the journal.
11. MCP side quiet: 3 `/mcp/call` lines; no security/browser/farm/android file logs locally; EU `ozone-security-mcp` has no journal entries; only `/tmp/wa-gateway.log` exists (Manejard WhatsApp gateway, unrelated).
Timing (two walks): stage 5 Build AMT 204 s / 503 s (largest variable), step execution 159 s / 232 s, gate 29 s / 13 s, delivery 53 s / 30 s; totals 544 s / 795 s — both over the 300 s client budget.
EU: four units active, health OK, 0 peers, 12 GB RAM (7.6 GB free), disk 32%, one orchestration completed, 14 429-lines in 3 days.
Shared context: 486 notes; **105 stale claims** — 12 from dead cc-fork-ordering/loops/orch forks (230+ h), 51 jurisdiction-batch claims (110–122 h), 43 `claude-code` claims including modality-fork and uidocument paths; dead presence records for cc-fork-w73, linkparent, task-recommendation, orchestrate-timeout, wizard-jurisdiction. Contradiction: the 10-31 "matcher weak" note vs later verified findings.
Rows: T-GATE-PENDING, T-RATE (+ chain excludes moderation models), T-AMT-DEDUPE (distinct findings → distinct candidates; a failing container never starves others), T-CONFETTI, T-REASONING-CAP, T-STAGE12, T-JUR-SYNC, T-RESTART-RECOVERY (kill -9 mid-walk), T-SENSOR (success rows as data), per-MCP file logging, T-CLAIMS (stale claims/presence expire or are released), ledger line integrity.

---
## 17. FINAL CONSOLIDATED EDIT TODO (supersedes §8; ordered; nothing started)
**Doctrine:** consciousness/meta gates are meta-only: unjudged → AWAIT (durable state) → meta re-judges only unjudged chunks → resume same task; never fail open, never a human for meta. Humans only for jurisdiction confirmations (modal, resume).
**P0 – correctness and trust**
1. Awaiting-item mechanism (§10): `awaiting` task state + `awaiting_items.jsonl`; stage 8 returns Awaiting; meta drain re-runs unjudged chunks with backoff/escalation; resume same task at the gate; boot reconciliation preserves it; `/order/global` awaiting bucket.
2. Close the other fail-open sites: stage 8b/14 plan check retains and awaits RequireConfirmation matches; jurisdiction `ReviewFailed` awaits; per-step jurisdiction/consciousness check for tool-routing and model-authored steps.
3. Shared OpenRouter per-minute limiter with Retry-After; exclude moderation/non-generation models from the fallback and pool; fix `HTTP 404 Provider returned error` handling (record model in the log line); reasoning-model output budgets.
4. Orchestrate client contract: long budget or submit-then-poll; shared recover-by-task-id used by MetaPortion and project chat; `awaiting` treated as running; never "Error" for a live task.
5. amt_loop starvation: distinct finding keys per finding, no single global container absorbing all actor findings, per-container failure isolation; fix the graph-ripple echo producing `failed_structural` (filter producer, not cap).
6. Jurisdiction content sync: ship manifest + one-time force sync so the 132 files/aliases reach the live data dir.
7. Ledger integrity: find and fix the writer producing unreadable rows.
**P1 – parity, honesty, modalities**
8. Project chat parity (§1.3/§11): shared `OrchestrationPanels`, `ThinkingRow`, `TranscriptEntryMeta`, one `useChatSession` engine (modelConfig, per-context persistence, recovery), ids from the session; MetaPortion persistence per context (localStorage).
9. UI awaiting states (`awaiting_meta`, `awaiting_human`), Order bucket, TaskDetailPanel Resume, jurisdiction modal wired to `/consciousness/reviews/resolve`.
10. Bottom bar per §4; chips moved from the strip; real depth or "—"; real I-Loop state; trust-aware peers.
11. Emotion read route + UI; stage 12 dispatch to the host store or build the binaries; dead consciousness toggles wired or removed; Consciousness Guide corrected.
12. Extension table unification (one table for host/file_link/UI); PDF/DOCX → Text with extraction; `.svg`/`ts`/`sdf` collisions; UiDocument reachable from file_link; MCP modality tags beyond the prefix table; link handlers for the remaining 16 or an explicit honest skip; path-hint false positives (`sar`, `imu`).
13. Order: recurrence expansion; checklist creation API; gcal/zoom idempotent sync by `external_id`; unify today/overdue boundary; assistant follow-up tasks or correct the guide.
14. Gate hygiene: `/mcp/call` and `rerun_step` session requirement; role checks on verified identity; `rerun_step` coercion; nested modality binaries through the admission gate/ledger.
15. Logging hygiene: sensor success rows; per-MCP file logs; correct the Pipeline-9 path warning; release stale claims/presence.
**P2** voice V2 (real TTS or loud failure now), reasoning-cap tuning, `capability_ids` design, GraphKit leftovers.
**Operator-gated:** local rebuild/relaunch (RAM, disk 93%), EU deploy, commit/push (one uncommitted UI edit), key rotation, data mutations (FileRef backfill, dedupe, worktrees).
Order of work: P0.1→P0.4 first (they remove the symptoms you reported), then P0.3/5/6/7, then P1.
