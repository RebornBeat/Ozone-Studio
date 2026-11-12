# tools/bridges → host: capability inventory (2026-11-12, parse-only)

Read in full: `tools/bridges/src/control-plane/{config,providers,server}.ts`, `src/local.ts`, `src/index.ts`, `src/ozone-bridge.ts`, `src/web/{server,llm-proxy,jobs,activity}.ts` (dashboard.html only by its fetch calls), `scripts/register-tools-to-ozone.mjs`, tests names; host side `assets/pipelines/general/prompt/main.rs`, `src/config/mod.rs` (ModelConfig / AvailableModel / fallback), `src/grpc/mod.rs` (set_config, paid_policy_refusal), `src/mcp_supervisor.rs`, `src/orchestrator/jurisdiction/policy.rs`, `ui/src/components/SettingsPanel.tsx`.

## What tools/bridges actually is (three things, two entry points)
| Part | Entry | What it does | Used by an Ozone walk? |
|---|---|---|---|
| Engine bridge (`ozone-bridge.ts`, `connectors/*`, `tools/*`, `server/create-server.ts`) | `dist/index.js` | 4 engine connectors; remote pipelines 200–203 on `/execute`; 67 engine tools on `/call`; stdio MCP surface | YES (tool routing → `/mcp/call` → `:3210/call`; pipelines 200–203) |
| Provider console (`control-plane/*`) | `dist/local.js` (Fastify, :3210 via `LOCAL_CONTROL_PORT`) | `/api/status`, `/api/jobs`: one prompt to openai / anthropic / z.ai / minimax APIs or codex / claude / zcode CLIs, own keys, `.roblox-mcp/control-plane.json` skills+MCP registry appended to the prompt | NO |
| Web dashboard + LLM proxy (`web/*`) | `dist/index.js` → `startDashboardServer` (:3100) | engine status panel, SSE activity, static tool list, `/api/chat` = anthropic/openai/google tool-use loop calling the engine tools, key typed into the browser | NO |

**Existing defect found:** the host supervisor's :3210 entry was `tools/bridges dist/local.js` (the provider console). The console answers neither `/call` nor `/execute`, so a supervised host had no working engine route; `/call` only worked when `dist/index.js` was started by hand.

## Capability → host equivalent
| # | Capability in the console / proxy | Host equivalent today | Status after this change |
|---|---|---|---|
| 1 | OpenAI API (Responses API, `gpt-5.4`) | pipeline 9 `chat_completions` wire; `AvailableModel` rows | **Added** vendor preset `openai` (chat completions; the Responses-API-only features are not carried — tool use is the walk's job). Paid class: needs `allow_paid_models` |
| 2 | Anthropic API (Messages, `max_tokens` fixed 1024 in the console) | pipeline 9 `anthropic` wire (model's real output ceiling, not 1024) | **Added** preset `anthropic`; the console's hidden 1024 cap is NOT carried (no-caps doctrine) |
| 3 | z.ai API (coding endpoint) | `chat_completions` | **Added** preset `zai` (same endpoint the console used) |
| 4 | MiniMax API | `chat_completions` | **Added** preset `minimax` (same endpoint) |
| 5 | Google Gemini (web proxy, native generateContent + function calling) | `chat_completions` against Google's OpenAI-compatible endpoint | **Added** preset `google`; native function calling not carried (the walk routes tools) |
| 6 | codex CLI | none | **Added** `cli` transport (`cli:codex`) in pipeline 9 |
| 7 | claude-code CLI | none | **Added** (`cli:claude`) |
| 8 | zcode CLI (console flagged the desktop AppImage as non-headless) | none | **Added** (`cli:zcode`); the command/args are operator-overridable (`OZONE_CLI_ZCODE_COMMAND/_ARGS`) because the headless flags are not verified |
| 9 | Provider availability/status (key present, command installed) | UI Settings shows registered models | **Partial**: added a Provider column + add/remove; a per-provider "is the key/CLI present" probe is NOT built (a CLI that is missing returns a precise error at call time) |
| 10 | Skills + MCP registry selected per job (`.roblox-mcp/control-plane.json`) appended to the prompt | methodologies (rules) + tool rows; tool routing selects tools per rule | **Superseded** (no file of this name exists on this machine; nothing to import). If the operator has one elsewhere, its skills are methodology candidates |
| 11 | Job history (`jobs.jsonl`, 200 records, 16 KB output tail, review pending/approved/changes-requested) | task store + work orders + awaiting items + hit/judgment logs (never rotated, no caps); decision review (`/capture/decision-reviews`) | **Superseded** (strictly more, uncapped). The 3-state job review is the awaiting/decision-review flow |
| 12 | Engine status panel (roblox/luau/unity/unreal/blender detection + manifest) | none | **Added** `bridge_status` tool on `/call` (registered by the refreshed script), reachable via `/mcp/call` |
| 13 | Dashboard SSE activity / log stream | host `/ws`, `/monitor/activity`, walk feed | **Superseded** |
| 14 | Static tool list (`/api/tools`, 67) | `/mcp/tools` registry (67 rows already registered at :3210) | **Superseded** |
| 15 | Browser-typed API key per chat request | host config (`api_key`, never echoed; redacted by `/config/get`) | **Superseded** (keys live in the host only) |
| 16 | AI chat console with engine tool loop (10 iterations) | the walk: steps → tool routing → `/mcp/call` (gated) | **Superseded** |
| 17 | Auth | none (loopback, no token; `/execute`/`/call` ran Luau/Python in the engine for any local process) | **Fixed**: engine bridge now enforces the same `x-oz-internal` loopback guard on a strict host and sends it when registering |
| 18 | Jurisdiction | none (direct vendor fetches, ungated) | all model access now crosses the host gate (adapter seam / prompt-binary clearance) |

## Caps in the old code NOT carried over
Anthropic `max_tokens: 1024` (console) / `4096` (proxy); 64 KiB provider output limit; 200 job records; 16 KiB job output tail; 10 tool iterations; 20 SSE clients; 500 activity events.
