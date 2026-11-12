# Jurisdiction call-site inventory (phase 3, 2026-11-12)

Every place content leaves the process or a call is executed, read from the code (parse-only; nothing run). "Outbound" = `policy::check_outbound*` (agreements, destination region, personal-data shapes, approvals). "Rule gate" = Stage 0 / 8b / call gate / `/mcp/call` review. Engineering input, not legal advice.

## A. Host (Rust) — content-bearing

| # | Site | Kind | Rule gate | Outbound | Status |
|---|---|---|---|---|---|
| 1 | `orchestrator/adapters.rs:68` `RegistryExecutorAdapter::execute` | every walk / loop / graph / tool-draft pipeline call | Stage 0, 8b (walk) | yes (+ vote), marks task-local `scoped_passed` | ok |
| 2 | `pipeline/mod.rs:249` `PipelineRegistry::execute` | HTTP `/pipeline/execute`, gRPC, direct dispatch | call gate + review (G2) | yes, deterministic | ok; destination now per pipeline (see C) |
| 3 | `pipeline/remote.rs` `RemotePipelines::execute` (POST `execute_url`) | self-registered remote pipeline | n/a | **NEW phase 3** `check_outbound_to_endpoint(execute_url)`; mark NOT honoured | was bypass (adapter mark cleared the model endpoint, not this hop) |
| 4 | `mcp.rs` HTTP/SSE branch (POST `tool.endpoint`) | registered tool call | `/mcp/call` gate, call gate | **NEW phase 3** `check_outbound_to_endpoint`; destination = `tool_egress` row for the tool, else its endpoint | was bypass for remote endpoints and for sidecars that reach external services |
| 5 | `mcp.rs` Agent branch → `agent_requests.rs:142` | agent-provider step | plan-level 8b | yes (`check_outbound_to_agent`, G4) | ok |
| 6 | `mcp.rs` stdio branch | delegated to the owning agent session | call gate | n/a (no content sent by host) | by design |
| 7 | `network/mod.rs` fetch serve (~1540) | peer fetches a body | offer gate on the receiver | **NEW phase 3** `policy::check_p2p_body` (shape scan, hold until operator approves hash) + `publish_kinds` (G5) | was unscanned |
| 8 | `network/mod.rs` announce / listing | manifests (name + hash + keywords) | operator-gated | `publish_kinds` (G5) | ok (manifest only) |
| 9 | `orchestrator/jurisdiction_search.rs` / ozone-shared-context search bridge | query text to answering sessions | n/a | shape hold (G6) | ok |
| 10 | `orchestrator/jurisdiction_search.rs:122` `execute(56, …)` | web search | n/a | through #1; destination = search provider (C) | ok |
| 11 | `orchestrator/graph_on_link.rs`, `graphs.rs`, `i_loop.rs`, `meta_loop.rs`, `chat_store.rs`, `response.rs`, `consciousness/assistant.rs`, `lib.rs:1721` | in-process `executor.execute` | none by design | via #1 / #2 | ok (all reach the adapter or registry) |
| 12 | `grpc/mod.rs` `/pipeline/execute`, `/mcp/call`, `/orchestrate`, `/task/*`, `/marketplace/*` | HTTP entry points | per route | via #1–#4 | ok |
| 13 | `grpc/mod.rs` `post_jurisdiction_outbound_check` (**NEW**) | judgement endpoint for #14 | — | is the gate | new |
| 14 | `assets/pipelines/general/prompt/main.rs` `jurisdiction_clearance` (**NEW**) | provider HTTP request (the place model content actually leaves) | — | asks the host (#13) before every `api` request; unreachable host = not sent | closes the R4 hole: modality / zero-shot pipelines spawn this binary directly and never crossed #1/#2 |
| 15 | `model_ledger.rs:840`, `openrouter_quota.rs:94`, `model_windows.rs:304` | constant probe prompt "Reply with the single word OK.", key status, catalog | n/a | n/a — no user content (the API key is our own credential) | documented, no gate needed |
| 16 | `hardware_region.rs:171` `detect_ip_country` | GET ip-api.com (discloses host IP over plain HTTP) | n/a | **NEW** switch `OZONE_REGION_IP_LOOKUP` (default ON) | finding; opt-out added |
| 17 | `mcp_supervisor.rs` (spawn `node`, port probe, `devices`) | local processes | n/a | n/a (loopback) | ok |
| 18 | `pipeline/executor.rs:915` | spawns builtin / custom pipeline binaries | n/a | spawn is downstream of #1/#2 | custom code is unaudited → C |
| 19 | `cli.rs` | host loopback only | n/a | n/a | ok |
| 20 | WebSocket push (`graph_events`, `/ws`) | events to connected UI/agents | n/a | not gated (operator's own UI) | note |

## B. Pipelines (separate processes)

| Pipeline | Egress | Judged as |
|---|---|---|
| 9 Prompt | provider HTTP (api); local gguf/onnx/bitnet | model endpoint; **plus** #14 inside the binary |
| 16 ZeroShotSimulation | spawns the prompt binary | model endpoint (+ #14) |
| 56 WebSearch | Brave (or `OZONE_WEB_SEARCH_ENDPOINT`) | `Endpoint` = search provider |
| 10 Voice | `api` backend uploads audio to `OZONE_VOICE_API_ENDPOINT`; whisper_rs / whisper_cpp stay local | `api`: `Endpoint` + forced kind `voice_recording`; others none |
| 25 BrowserNavigation | page request to the URL's host (puppeteer) | `Endpoint` = the URL found in the input |
| 1–127 others (url_link, package_link, file_link, modalities, context_aggregation, zsei_*, …) | host loopback only (and the prompt binary through #14) | none |
| ≥128 tool-created / self-registered / imported | unknown code | `Endpoint` label `unaudited-pipeline:<id>` → unlisted → `default_region_unlisted_endpoint` |

## C. Destination classification (the fix behind #2/#3/#4)

Before phase 3 every pipeline was classified by the instance's MODEL endpoint. That judged web search "local" when the model was local (a leak) and held pure-local pipelines when the model was remote (a stall). `policy.rs` `pipeline_egress` now gives each pipeline its own destination; `check_outbound_to_endpoint`/`_to_agent` never honour the adapter's mark (it cleared the model hop, not this one).

## D. Sidecar MCP tools (`tools/*-mcp`)

| Tool prefix | Backing | External destination | Handling |
|---|---|---|---|
| `web_search` | `web-mcp` spawns the web_search pipeline binary directly | Brave | `tool_egress` row |
| `geo_` | `geospatial-mcp` | nominatim.openstreetmap.org, overpass-api.de | row |
| `gcal_` | `gcal-mcp` | www.googleapis.com | row |
| `zoom_` | `zoom-mcp` | api.zoom.us | row |
| `farm_` | `browser-farm-mcp` | operator's farm host(s) | row (`browser-farm` label; operator maps it in `providers`) |
| others (`code`, `dna`, `cerebrix`, `connectome`, `mesh`, `re`, `security`, `visual`, `threed`, `android`, `code-graph`, `p2p-bridge`, `ozone-mcp`, `ozone-shared-context`) | local python/blender/CLI or host loopback | none found | no row (re-audit when adding a tool that calls out) |

`tool_egress` is a per-instance policy key (null = shipped rows; `[]` = operator cleared). An unlisted label takes the unverified default.

## E. `tools/bridges` (git directory inside the repo, NOT edited)

| File:line | Call | Reached by |
|---|---|---|
| `src/control-plane/providers.ts:303` | POST api.openai.com `/v1/responses` | control-plane provider jobs (`runLocalProviderJob`) |
| `src/control-plane/providers.ts:349` | POST api.anthropic.com `/v1/messages` | same |
| `src/control-plane/providers.ts:402` | POST api.z.ai coding paas | same |
| `src/control-plane/providers.ts:454` | POST api.minimax.io | same |
| `src/web/llm-proxy.ts:151` / `:212` / `:261` | Anthropic / OpenAI / Google Generative Language | the bridges web console |
| `src/ozone-bridge.ts` | registers pipelines 200–203 (roblox/unity/unreal/blender engine control) at `127.0.0.1:<port>/execute` | host calls → #3 (loopback = local) |

These direct calls do not cross any host gate. Options (operator decision): **A** patch the submodule so each fetch first POSTs `{pipeline_id: 9, input}` to `/jurisdiction/outbound/check` (one helper, 8 sites; unreachable = not sent); **B** route the console's model calls through the host (`/pipeline/execute` 9); **C** do not run the control-plane/web console on instances whose agreements forbid third-country calls (host-side: nothing registers them as tools except the engine bridges above).

## F. Other tools (`tools/`)

All other JS tools (`ozone-mcp`, `ozone-shared-context`, `zcode-connector`, `browser-plugin`, `register-browser-tools`, `keyword-measure`, `e2e`, `ozone-mobile`) call only the host (`OZONE_HOST`) and so inherit the host's gates.

## G. Still open
Bridges (E, operator choice); WebSocket push is not an outbound hop (own UI); remote operator of the UI is out of scope; the vote model's endpoint must be listed in `providers` with an allowed region or the vote call is itself held (documented in the policy note, not auto-fixed).
