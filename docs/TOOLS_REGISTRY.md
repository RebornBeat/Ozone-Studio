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
