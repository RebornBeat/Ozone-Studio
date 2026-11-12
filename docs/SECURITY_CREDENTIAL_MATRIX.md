# Credential matrix — who presents what to the host (2026-11-12; parse-only, never run)

Strict-session mode (`OZONE_REQUIRE_SESSION`, default ON; stands down only while first-run setup is incomplete) protects the route families below.
A request is accepted with EITHER credential:
- **device session** — ed25519 device-key challenge (`POST /auth/challenge` + `/auth/authenticate`) → `session_token`, presented as
  `Authorization: Bearer <hex>` (or `x-oz-session`). Used by anything acting as a paired device/agent.
- **internal token** — `<data_dir>/.internal_token` (0600, 32 random bytes hex, reused across restarts), presented as `x-oz-internal`. A
  FILE-PERMISSION boundary: a browser page cannot read it, a same-user local process can. Used by processes the host spawned or that live on the
  same machine and cannot pair (tool servers, spawned pipelines, the shared-context server).

## Protected prefixes (grpc/mod.rs `session_gate`)
`/config/set` `/mcp/tools` `/mcp/call` `/chat/` `/context/` `/ripple/` `/marketplace/` `/network/` `/task/` `/consciousness/reviews/resolve`
`/capture/ingest_image` `/capture/ingest_uidocument` `/pipelines/register` `/walk/` `/agent/` `/jurisdiction/` `/history/` and, added in the credentials
pass: `/pipelines/unregister` `/orchestration/` `/consciousness/review_pass` `/pipeline/cancel`; mutating-only (non-GET): `/monitor/activity`.
Exempt: `/jurisdiction/outbound/check` (spawned prompt binary has no session; read-only check).
Handler-level: `/orchestrate` now REQUIRES a credential on a strict host — a valid body `session_token`, OR a valid Bearer/`x-oz-session` header, OR
`x-oz-internal` (a missing credential is refused; an invalid body token is still refused). `/pipeline/execute` and `/zsei/query` still validate their body
token themselves (unchanged).

## Who uses what
| Caller | Credential | Notes |
|---|---|---|
| Electron UI main process (`ui/electron/main.js`) | device session, as body `session_token` AND `Authorization: Bearer` on every non-public call | `PUBLIC_PATHS` = /health, /auth/, /pairing/, /pair/ |
| Browser/renderer fetch (`ozoneClient.ts`) | pairing session token (`Authorization: Bearer`) | from QR pairing |
| `tools/ozone-cli` | device session (body + header via `areq`) | chat/capture/order/mcp now send the header |
| `tools/e2e`, `tools/third-party-verify` | device session (`http()` auto-adds the Bearer once a session exists; `orchestrate()` sends body token) | `/auth/*` and `/health` excluded |
| `tools/ozone-mcp` (agents) | per-agent device-key session; never the internal token | read tools + orchestrate/link/agent tools; history/held-call READ tools added |
| Spawned modality pipelines (24) → `/orchestration/node-event` | `x-oz-internal` from `OZONE_INTERNAL_TOKEN` (set by `pipeline/executor.rs`) | `.header("x-oz-internal", ...)` added at all 24 sites |
| Spawned prompt binary → `/jurisdiction/outbound/check` | none (exempt route) | host unreachable = provider request not sent |
| Tool servers (git-mcp restore, others) → host | `x-oz-internal` via `tools/shared/guard.mjs::internalHeaders()` | token lookup: `OZONE_INTERNAL_TOKEN`, `OZONE_INTERNAL_TOKEN_FILE`, `$OZONE_DATA_DIR/.internal_token`, repo `target/release/zsei_data/.internal_token`, `zsei_data/.internal_token` |
| `tools/zcode-connector` (register/unregister/activity) | device-key session (rewritten by the ZCode/CC integration review, commit b8b3b22) | was sending nothing: failed on strict hosts; my interim internal-token hunk was superseded by that rewrite |
| `tools/ozone-shared-context` | `x-oz-internal` (own reader) | unchanged |
| Browser plugin (`tools/browser-plugin`) | NONE yet | a browser extension cannot read the token file; needs the pairing flow — OPEN |
| Host-internal loops | in-process (no HTTP) | not affected |

## Open
- `/pipeline/execute`, `/zsei/query`, `/orchestrate/status`, `/order/*`, `/callgraph/*`, `/capability/gaps`, `/methodology/refinement`, `/config/get`,
  `/quota`, `/model-ledger-summary` are outside the protected list (read paths or body-token validated). The EU host binds 127.0.0.1 and CORS is a
  predicate, so exposure = same-machine processes. Extend the list only with the same caller trace done here.
- Browser plugin pairing; `tools/walk-scorecard` makes no HTTP calls (reads files).
