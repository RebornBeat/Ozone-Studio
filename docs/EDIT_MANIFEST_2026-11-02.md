# Edit manifest — the 2026-11-01/02 edit wave (generated from `git status`, 133 paths)

Every path below is in the working tree, **uncommitted, uncompiled and untested** (build + test pass is operator-gated). Authoritative narrative: `CHECKLIST.md` (chronological), `SPAN_OUT_AND_RESUME_2026-11-02.md`, `SYSTEM_FLOW_AND_GRAPH_STATE_2026-11-02.md`, `CALL_ROUTE_REGISTRY.md`, `REVIEW_E2E_STATE_2026-11-01.md` (findings + TODO). Re-generate with `git status --porcelain -uall`.

| Path | State | What changed |
|---|---|---|
| `CHECKLIST.md` | modified | living status record — every wave appended |
| `README.md` | modified | System Status 2026-11-02 block, doc map |
| `assets/pipelines/general/context_aggregation/main.rs` | modified | span_out_context layer, no string chop |
| `assets/pipelines/general/file_link/main.rs` | modified | shared table, doc/ui dispatch |
| `assets/pipelines/general/prompt/main.rs` | modified | cross-process limiter, NO response cap (effective_api_cap) |
| `assets/pipelines/general/voice/main.rs` | modified | honest TTS engines |
| `assets/pipelines/general/zero_shot_simulation/main.rs` | modified | routed through prompt binary (no direct provider POST) |
| `assets/pipelines/modalities/3D/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/BCI/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/CAD/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/IMU/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/biology/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/control/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/depth/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/electromagnetic/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/geospatial/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/haptic/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/hyperspectral/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/kinematics/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/network/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/proteomics/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/radar/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/sonar/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/sound/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/modalities/text/Cargo.toml` | modified | no pdf/zip deps |
| `assets/pipelines/modalities/text/main.rs` | modified | document MCP client, ExtractionOutcome::Failed, no silent {} helpers |
| `assets/pipelines/modalities/thermal/main.rs` | modified | LinkToModality handler + prompt-binary env/system_prompt/no-silent-{} fixes |
| `assets/pipelines/shared/link_handler.rs` | new | NEW shared LinkToModality writer |
| `assets/pipelines/shared/modality_ext.rs` | new | NEW single extension→modality table |
| `docs/ACTING_FACTORS_REGISTRY.md` | modified | documentation (registry / guide / review) |
| `docs/AMT_EXPANSION.md` | modified | documentation (registry / guide / review) |
| `docs/CALL_ROUTE_REGISTRY.md` | new | documentation (registry / guide / review) |
| `docs/CONSCIOUSNESS_RESPONSE_INTEGRATION.md` | modified | documentation (registry / guide / review) |
| `docs/CONTEXT_REGISTRY.md` | modified | documentation (registry / guide / review) |
| `docs/DESIGNED_NOT_BUILT_REGISTRY.md` | modified | documentation (registry / guide / review) |
| `docs/GRAPH_RELATIONSHIP_REGISTRY.md` | modified | documentation (registry / guide / review) |
| `docs/LIVING_GRAPH_STATUS.md` | modified | documentation (registry / guide / review) |
| `docs/MASTER_TEST_GUIDE.md` | modified | documentation (registry / guide / review) |
| `docs/PERSONAL_ASSISTANT_GUIDE.md` | modified | documentation (registry / guide / review) |
| `docs/REVIEW_E2E_STATE_2026-11-01.md` | new | documentation (registry / guide / review) |
| `docs/SPAN_OUT_AND_RESUME_2026-11-02.md` | new | documentation (registry / guide / review) |
| `docs/STATE_CAPTURE.md` | modified | documentation (registry / guide / review) |
| `docs/SYSTEM_FLOW_AND_GRAPH_STATE_2026-11-02.md` | new | documentation (registry / guide / review) |
| `docs/TOOLS_REGISTRY.md` | modified | documentation (registry / guide / review) |
| `docs/UNIVERSAL_ORDER_GUIDE.md` | modified | documentation (registry / guide / review) |
| `docs/VOICE_BOX_GUIDE.md` | modified | documentation (registry / guide / review) |
| `docs/ZERO_SHOT_CALL_REGISTRY.md` | modified | documentation (registry / guide / review) |
| `docs/guides/CONSCIOUSNESS_GUIDE.md` | modified | documentation (registry / guide / review) |
| `src/consciousness/assistant.rs` | modified | follow-up tasks, response records |
| `src/consciousness/response_record.rs` | new | NEW structured CONSCIOUSNESS_RESPONSE records |
| `src/consciousness/store.rs` | modified | toggles wired, experience_count_for_task, state snapshot |
| `src/context_mirror.rs` | modified | mirror edges |
| `src/grpc/mod.rs` | modified | awaiting/order/consciousness routes, /chat/turns, /task/resume*, /task/pause_info, /consciousness/responses, external_ref idempotency, pipeline-execute gate, session gate list, internal_token, tests |
| `src/lib.rs` | modified | awaiting driver, step-gate + call-gate install, resume_run, loops call-indexed, pipeline-9 path check |
| `src/mcp.rs` | modified | call-gate (check_call_gate), x-oz-internal header to tools |
| `src/mcp_graph.rs` | modified | modality tag prefixes |
| `src/mcp_supervisor.rs` | modified | port 3282, token env to spawned MCPs |
| `src/model_ledger.rs` | modified | atomic row writes + salvage |
| `src/model_windows.rs` | modified | non-chat models excluded from pool |
| `src/orchestrator/actors.rs` | modified | sensor rows, findings |
| `src/orchestrator/adapters.rs` | modified | flat-JSON container normalizer (+tests) |
| `src/orchestrator/amt.rs` | modified | no-silent-{} recording at intent/detail/cross-ref/domain/synthesis sites |
| `src/orchestrator/amt_candidates.rs` | modified | unified candidate store tweaks |
| `src/orchestrator/amt_loop.rs` | modified | echo filter (+ChatSession/ChatTurn), span-out context, no-JSON = failed attempt, starvation fixes |
| `src/orchestrator/awaiting.rs` | new | NEW durable awaiting items (meta-only gate doctrine) |
| `src/orchestrator/chat_store.rs` | new | NEW scoped chat persistence (ChatSession/ChatTurn, full text, response graph, chat memory) + tests |
| `src/orchestrator/decision_review.rs` | modified | chunked multi-pass review, safety-first merge, awaiting |
| `src/orchestrator/entry_gate.rs` | modified | require_identity / gated_checked |
| `src/orchestrator/graphs.rs` | modified | link-handler allowlist, shared extension table, blank-response parse |
| `src/orchestrator/i_loop.rs` | modified | span-out section, CONSCIOUSNESS_RESPONSE record |
| `src/orchestrator/jurisdiction.rs` | modified | step gate install, jurisdiction_confirm awaiting items, hold-not-pass |
| `src/orchestrator/meta_loop.rs` | modified | blank draft = failure, awaiting drain |
| `src/orchestrator/mod.rs` | modified | init_state extraction, response_text/parse_json_object_strict, no-cap host side, early-failure chat persist, alignment-parse recording, 429/404/reasoning handling, module wiring |
| `src/orchestrator/response.rs` | modified | identity toggle, dashboard dispatch removed |
| `src/orchestrator/run_checkpoint.rs` | new | NEW run checkpoints, failure pause, resume_run |
| `src/orchestrator/span_out.rs` | new | NEW context span-out algorithm + step seeding + tests |
| `src/orchestrator/stages.rs` | modified | awaiting gate loop, step pause/checkpoint/resume loop, span-out per step, stage-12 verify-store, strict blueprint/simulation parse, task id to tool routing, sub-step/web-search failure doctrine |
| `src/orchestrator/tool_routing.rs` | modified | per-step gate + task-id variant |
| `src/orchestrator/voice.rs` | modified | voice identity persistence fix |
| `src/pipeline/executor.rs` | modified | builtin candidates, limiter dir, internal token env |
| `src/pipeline/registry.rs` | modified | consciousness binary_alias extended |
| `src/task/mod.rs` | modified | awaiting + pause_for_error/resume_paused/paused_step_of, boot reconcile |
| `src/types/container.rs` | modified | ChatSession/ChatTurn/VoiceIdentity container types |
| `src/types/modality.rs` | modified | shared extension table |
| `tools/android-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/browser-farm-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/browser-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/cerebrix-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/chemistry-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/code-graph-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/code-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/connectome-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/dna-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/document-mcp/server.mjs` | new | loopback guard (guardOk) |
| `tools/gcal-mcp/server.mjs` | modified | idempotent order sync |
| `tools/geospatial-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/git-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/mesh-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/p2p-bridge-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/proteomics-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/re-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/register-browser-tools.mjs` | modified | document family |
| `tools/security-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/shared/guard.mjs` | new |  |
| `tools/terminal-mcp/server.mjs` | modified | roles keyed on agent_trust |
| `tools/threed-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/visual-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/web-mcp/server.mjs` | modified | loopback guard (guardOk) |
| `tools/zoom-mcp/server.mjs` | modified | idempotent order sync |
| `ui/electron/main.js` | modified | 45-min slow endpoint timeout, client-timeout text |
| `ui/src/App.tsx` | modified | UI |
| `ui/src/components/AwaitingBadge.tsx` | new | UI component (new or updated this wave) |
| `ui/src/components/MetaPortion.tsx` | modified | shared engine, PausedBanner, voice errors, toggles, orb route |
| `ui/src/components/OrchestrationPanels.tsx` | new | UI component (new or updated this wave) |
| `ui/src/components/OrchestrationStatusPanel.tsx` | modified | scope filter, awaiting stage |
| `ui/src/components/OrderPanel.tsx` | modified | awaiting rows, Resume on failure-paused |
| `ui/src/components/PausedBanner.tsx` | new | UI component (new or updated this wave) |
| `ui/src/components/PendingReviewModal.tsx` | new | UI component (new or updated this wave) |
| `ui/src/components/ProjectChatPanel.tsx` | modified | shared engine + indicators + PausedBanner |
| `ui/src/components/StatusBar.tsx` | modified | P2P/depth ownership, honest "—" |
| `ui/src/components/TaskDetailPanel.tsx` | modified | awaiting + pause banner/resume |
| `ui/src/components/ThinkingRow.tsx` | new | UI component (new or updated this wave) |
| `ui/src/components/TranscriptEntryMeta.tsx` | new | UI component (new or updated this wave) |
| `ui/src/components/WorkspaceStatusStrip.tsx` | modified | UI component (new or updated this wave) |
| `ui/src/data/workspaceData.ts` | modified | UI |
| `ui/src/graphkit/graphProvider.ts` | modified | indicators / modality reads |
| `ui/src/graphkit/index.ts` | modified | indicators / modality reads |
| `ui/src/graphkit/indicators.tsx` | modified | indicators / modality reads |
| `ui/src/hooks/useChatSession.ts` | modified | shared chat engine, persistence, host hydration, awaiting+pause recovery |
| `ui/src/hooks/useConsciousnessToggles.ts` | new | UI hook (new) |
| `ui/src/hooks/useVoiceIO.ts` | new | UI hook (new) |
| `ui/src/ozoneClient.ts` | modified | hostGet/Post, awaiting, resumeRun, pause_info, parsePause |
| `ui/src/services/store.ts` | modified | UI |

## Operator-gated (not done, by instruction)
Build + test pass; local relaunch; EU deploy (new host binary and `tools/shared/guard.mjs` must ship together before enabling the MCP loopback guard on a strict host); commit/push; registering `document-mcp` (`node tools/register-browser-tools.mjs document`); key rotation; data mutations (FileRef backfill, dedupe, worktree deletion); first build of the consciousness crates (`OZONE_BUILD_NEW=… scripts/build-pipelines.sh`).
