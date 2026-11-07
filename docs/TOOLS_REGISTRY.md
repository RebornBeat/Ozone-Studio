# Ozone-Studio Tool Registry — every registered tool, MCP, and detection model

> Live capture of the tool/MCP surface (2026-09-28). Registry persistence:
> `/mcp/tools` survives restarts via `zsei_data/mcp_tool_registry.json`
> (verified: "restored 70/73 persisted tool(s)" across two restarts). Every
> tool is invocable through ONE surface — `/mcp/call` — metered per
> agent/day, jurisdiction-gated, graph-rippled, captured to S13
> (`tool_calls.jsonl`), and wrapped in the Phase-1 insight envelope.

## Registry totals (live)

| Family | server_version | Tools | Modality scope | Platform |
|---|---|---|---|---|
| Bridges (gamedev absorbed) | bridges-0.1.0 | **67** | roblox/unity/unreal/blender (4 engines) | linux/x64 (engines cross-platform) |
| Terminal MCP | terminal-0.2.0 | 2 | system (allowlist-secured) | linux/x64 |
| Visual MCP | visual-0.2.0 | 2 (+1 ingest) | image (modality 102) | linux/x64 |
| Visual + detection | visual-0.2.0-yolo | 1 | image (modality 102, §11.6) | linux/x64 |
| Shared context | (ozone-shared-context server) | 16 | coordination (all scopes) | any (Node) |
| **Total registered** | | **73+** | | |

## The tool families — what each IS in the taxonomy

### 1. Bridges (67 tools) — `tools/bridges/`
The absorbed gamedev-all-in-one package. 4 engine connectors register as
Ozone remote pipelines (Roblox 200, Unity 201, Unreal 202, Blender 203);
every tool call dispatches to the engine's own command functions.
Platform: bridges cross-platform; engines require their editor/runtime.
Per-modality: these tools FEED image/3D modality graphs (Blender scene →
3D 109 graphs; UI screenshots → image 102).

### 2. Terminal (2 tools) — `tools/terminal-mcp/`
`terminal_exec`, `terminal_status`. Allowlist-required (unset = locked),
shell-operator rejection, timeout/output caps, optional shared secret.
Serves system-level capability: git/cargo/build tools — the agent's hands.

### 3. Visual (3 tools) — `tools/visual-mcp/`
`visual_describe`, `visual_graph`, `visual_ingest`, `yolo_detect`.
Modality: **image (102)** — every tool drives the image pipeline's real
Analyze/CreateGraph. Capture layering (operator-corrected): the MCP never
execs screen-grabbers; sanctioned sources are the Ozone Electron app's own
desktopCapturer (POSTing to visual_ingest), the xdg portal with explicit
user approval, or explicit X11 opt-in.

### 4. Detection-model registry (§11.6 — the YOLO vision)
`zsei_data/detection_models/registry.json` — data, not code:
| Model | Backend | Classes | Enabled |
|---|---|---|---|
| yolov8n | ultralytics | coco80 | ✓ (needs `pip install ultralytics`) |
| yolov5n | torch-hub | coco80 | off (weights not cached) |
`yolo_detect` runs registry models through the ultralytics python backend
(torch 2.2.2+cpu present), returns detections + **F.3 spatial relations
computed from real bounding boxes** (Above/LeftOf/RightOf/NearTo/Supports),
and is the first consumer of the parallel-expandable design: add a model =
add a registry entry; N models run in parallel on the same image.

### 5. Shared context (16 tools) — `tools/ozone-shared-context/`
`context_summary`, `note_add`, `file_claim/release/claims`, `task_create/update`,
`presence_heartbeat/list`, `search_request/bridge`, `cc_sessions`, `mcp_usage`.
The agent coordination surface — now routing mirrors through `/mcp/call`
when `OZONE_THROUGH_HOST=1` (the ordered layer).

### 6. Browser (11 tools) — `tools/browser-mcp/`  *(2026-10-29)*
`browser_status/open/navigate/snapshot/click/type/scroll/screenshot/eval/console/close`. Pattern A. Locked until
`OZONE_BROWSER_ALLOW_HOSTS`; origin guard before every interaction; eval off by default; page content returned as
`untrusted_*`. Feeds the **image modality (102)** via `browser_screenshot` paths. Guide:
`docs/BROWSER_AUTOMATION_GUIDE.md`. Register with `tools/register-browser-tools.mjs`.

### 7. Browser-Farm bridge (13 tools) — `tools/browser-farm-mcp/`  *(2026-10-29)*
`farm_status/profiles/profile/profile_start|pause|stop/navigate/screenshot/metrics/state_get|state_set/command/result`.
Pattern B bridge to `~/Projects/Browser-Farm` (Command Center channel via shared state). Locked until a farm + API key
are configured; no script/vault/proxy-credential tools by design. Proven against a mock farm only; live acceptance open.

## Registration rules (every new tool, no exceptions)

1. Security model written before the first execution path; locked-by-default.
2. Refusal path tested, not just the happy path.
3. Platform + modality + version tags on registration.
4. Registered into `/mcp/tools` (persisted across restarts).
5. Invoked ONLY through `/mcp/call`.
6. Ripple + S13 capture verified (one call = one ledger row + one ripple + one S13 row).
7. Persistent-process launch story documented.
8. Contract container in ZSEI if it introduces doctrine.

## Known gaps (honest)

- Detection backend needs `pip install ultralytics` (torch present, one command).
- Brides' 67 tools: dispatch mapping (tool name → engine method) is live for
  the prefix families; foundation tools (doctor/inspect) live on the bridges
  MCP surface :3100/mcp, not through the shim.
- No version-range dispatch yet (registry stores versions; enforcement designed).
- Platform is capability tags today; native registry field designed.


---

## LIVE FLEET CAPTURE (2026-09-29, GET /mcp/tools) — 102 tools / 15 families

| Family | server_version | Tools | Port | Notes |
|---|---|---|---|---|
| bridges-0.1.0 | 67 | :3210 | engines (Roblox 200/Unity 201/Unreal 202/Blender 203) |
| security-0.1.0 | 8 | :3240 | CC's guardrail layer (net/firewall/process) |
| gcal-0.1.0 | 7 | :3241 | Google Calendar connector (Universal Order Stage 3) — locked until OAuth credentials |
| git-0.1.0 | 3 | :3235 | native file history over the file beacon |
| shared-context 0.3.0 | 1 | — | coordination (16 tools on the stdio surface) |
| terminal-0.2.0 | 2 | :3215 | per-role allowlisted exec |
| visual family | 7 | :3220 | ingest/describe/graph/yolo_detect/yolo_graph/depth_estimate/depth_graph |
| web-0.1.0 | 2 | :3225 | Brave search + datetime (wraps pipeline 56) |
| code-0.1.0 | 1 | :3230 | code_callgraph (wraps pipeline 101) |
| cerebrix-0.1.0 | 1 | :3245 | CC's EEG window |
| connectome-0.1.0 | 2 | :3250 | CC's fly connectome circuits |
| threed-0.1.0 | 1 | :3255 | CC's 3D expansion |

**Persistence verified**: restored across restarts from mcp_tool_registry.json (3x observed: 70/73/75/84→102 growth path all clean).

**Registration contract recap** (full rules in NEW_MCP_GUIDE §6): security model first, locked-by-default, refusal path tested, platform+version tags, /mcp/tools registration, /mcp/call only, ripple+S13 verified, launch story documented, contract container if doctrine.


---

## Corrections and full port table (2026-11-02, from each server's source default)

The "LIVE FLEET CAPTURE" table above is a 2026-09-29 snapshot: its header has five columns but the rows have four (the `server_version` column is missing from the rows, so read each row as Family | Tools | Port | Notes), and it omits every server added since. Ports below are the defaults in each `tools/*/server.mjs` (all bind 127.0.0.1; each is overridable by its `OZONE_*_PORT` env var):

| Server | Default port | Env var |
|---|---|---|
| terminal-mcp | 3215 | OZONE_TERMINAL_PORT |
| visual-mcp | 3220 | OZONE_VISUAL_PORT |
| web-mcp | 3225 | OZONE_WEB_PORT |
| code-mcp | 3230 | OZONE_CODE_PORT |
| git-mcp | 3235 | OZONE_GIT_PORT |
| security-mcp | 3240 | OZONE_SECURITY_PORT |
| gcal-mcp | 3241 | OZONE_GCAL_PORT |
| cerebrix-mcp | 3245 | OZONE_CEREBRIX_PORT |
| threed-mcp | 3250 | OZONE_THREED_PORT | tools: eeg_cap_costume_generate (rat 3D asset — historical name, not EEG), clock_generate, cad_clock_generate; PROVIDER id `threed-mcp` |
| connectome-mcp | 3255 | OZONE_CONNECTOME_PORT (circuit service 3256: OZONE_CIRCUIT_SERVICE_PORT) |
| re-mcp | 3260 | OZONE_RE_PORT |
| zoom-mcp | 3265 | OZONE_ZOOM_PORT |
| dna-mcp | 3271 | OZONE_DNA_PORT |
| chemistry-mcp | 3272 | OZONE_CHEM_PORT |
| proteomics-mcp | 3273 | OZONE_PROTEOMICS_PORT |
| geospatial-mcp | 3274 | OZONE_GEOSPATIAL_PORT |
| mesh-mcp | 3276 | OZONE_MESH_PORT |
| android-mcp | 3277 | OZONE_ANDROID_PORT |
| p2p-bridge-mcp | 3278 | OZONE_P2P_BRIDGE_PORT |
| code-graph-mcp | 3279 | OZONE_CODE_GRAPH_PORT |
| document-mcp | 3282 | OZONE_DOCUMENT_PORT |
| browser-mcp | 3280 | OZONE_BROWSER_PORT |
| browser-farm-mcp | 3281 | OZONE_FARM_PORT |
| bridges (engines) | 3210 (MCP surface :3100/mcp for foundation tools) | — |

Notes: zoom-mcp (3265) was missing from the fleet table; like gcal it is LOCKED until OAuth credentials exist. The 102-tool / 15-family count is the 2026-09-29 capture and is stale — query `GET /mcp/tools` for the live number. gcal/zoom order sync is now idempotent (UNIVERSAL_ORDER_GUIDE corrections). Ports 3290/3292 in selftests are test-only.


### Document (4 tools) — `tools/document-mcp/` (:3282, added 2026-11-02)
`document_info` (format/size/PDF pages+metadata), `document_extract` (real text from pdf/docx/odt/pptx/xlsx/rtf; legacy doc/ppt/xls via LibreOffice; scanned PDFs need tesseract else honest error), `document_images` (PDF embedded-image list), `document_formats` (which backends are installed). The text modality (pipeline 100) calls `document_extract` through `/mcp/call`. Registered by `node tools/register-browser-tools.mjs document` (operator-run data mutation).


### Gate coverage note (2026-11-02)
Every MCP server binds 127.0.0.1 and trusts the host's `/mcp/call` for G1 (session/internal token), G5 (jurisdiction) and G10 (usage ledger); in-process callers (`call_global`) now pass the same jurisdiction snapshot via `mcp::set_call_gate`. A tool port called directly on loopback skips all of that — `terminal-mcp` has `OZONE_TERMINAL_TOKEN`; the others have no equivalent yet (see `CALL_ROUTE_REGISTRY.md` §3). Role-gated tools must key on the envelope's `agent_trust` (`validated` | `declared`); `terminal-mcp` now does.


## Terminology 2026-11-04
A **tool** is a registry call contract (name + capabilities + endpoint); the **provider** is the MCP server process behind one or more tools (id = the `tools/<dir>` name in the supervisor port table, e.g. `threed-mcp`) — the unit that is installed, started, trusted and shared. Methodologies declare PROVIDERS per rule; blueprint creation turns each into a provider STEP. See docs/MCP_TOOLS_MODALITY_ARCHITECTURE_REVIEW_2026-11-04.md.
