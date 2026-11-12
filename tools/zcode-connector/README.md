# Ozone-Studio Agent Connector

Reference implementation of the Ozone-Studio agent wire contract — zero
dependencies, one file. Connect anything to a running Ozone-Studio host the
same way the browser plugin and ZCode do.

This is a standalone Ozone-Studio tool. It ships separately from the backend
and the UIs so others can build their own connectors on the same contract.

## The contract

| Endpoint                    | Method | Body                                        |
| --------------------------- | ------ | ------------------------------------------- |
| `/pipelines/register`       | POST   | `{pipeline_id, name, execute_url, roles?}`  |
| `/pipelines/unregister`     | POST   | `{pipeline_id}`                             |
| `/agent/requests?agent=`    | GET    | — (agent-provider steps awaiting this agent)|
| `/jurisdiction/outbound/check` | POST | `{pipeline_id, input}` → `{allow, reason}` (read-only, no session) |
| `/monitor/activity`         | POST   | `{kind, level, source, message}`            |
| `/monitor/summary`          | GET    | — (agents + activity feed)                  |
| `/pipelines/remote`         | GET    | — (registry listing)                        |
| `/mcp/tools`                | GET    | — (registered MCP tools)                    |
| `/mcp/tools/register`       | POST   | `{name, transport, endpoint, capabilities?}`|

- `pipeline_id` — stable numeric id you choose for your agent (e.g. 9002).
- `execute_url` — where the host dispatches `PipelineInput` work to your
  agent. Point it at your own endpoint to receive work; a monitor URL means
  observe-only.
- `roles` — what your connection serves: `agent` (task dispatch),
  `model` (pipeline-9 model calls), `observer` (feed only). Absent =
  `["agent"]`.
- Activity `kind`: `log | agent | tool | job | bridge | external`;
  `level`: `info | ok | warn | error`.
- Requests are JSON (`Content-Type: application/json`); responses are JSON.
- Registration persists until unregistered or the host restarts; re-register
  any time (idempotent — refreshes the entry, which the dashboard reads as
  the heartbeat/last-seen).

## Authentication (strict hosts)

The host runs with `OZONE_REQUIRE_SESSION` on by default. `/pipelines/register`, `/agent/*`, `/walk/*`, `/jurisdiction/*` and
`/history/*` need a session; `connect.js` authenticates with a device key (Ed25519 challenge, same as `tools/ozone-cli`):
`ZCODE_KEY_PATH`, else `~/.ozone-cli/agents/<ZCODE_AGENT_NAME>/device-key.pem`, else `~/.ozone-cli/device-key.pem`.
`node connect.js whoami` shows which (never the key). Without any key it tries unauthenticated and a strict host answers 401.

## Jurisdiction

Content this connector forwards to ZCode's model provider (Z.ai) leaves the machine, so `connect.js` asks
`POST /jurisdiction/outbound/check` (destination `ZCODE_PROVIDER_ENDPOINT`, default `https://api.z.ai/`) before calling
`handleModelCall()` and does NOT forward a held or unjudgeable call (HTTP 503 `awaiting_operator`). List the provider in the
instance policy's `providers` (region) and accept the agreements in the UI (Marketplace → Jurisdiction), or the call stays held.

## ZCode is a desktop app — which integration to use

ZCode has no headless CLI, so `handleModelCall()` (the pipeline-9 model role) has nothing to spawn and stays an honest error.
The supported route is MCP: ZCode connects `tools/ozone-mcp` (prompt with `ozone_orchestrate`, follow walks, offer itself with
`ozone_agent_offer`, answer with `ozone_agent_report`). `node connect.js watch` then announces on the monitor feed when
agent-provider requests await ZCode. Setup: `docs/AGENT_INTEGRATION_ZCODE_CC.md`.

## Multi-device pairing (phones included)

Devices pair through `src/pairing.rs` — the phone is the authenticator:

| Endpoint              | Method | Body / Query                            |
| --------------------- | ------ | --------------------------------------- |
| `/pairing/start`      | POST   | `{device_hint?}` → QR payload + code    |
| `/pairing/status`     | GET    | `?pairing_id=…` → token when approved   |
| `/pairing/approve`    | POST   | `{code, device_name?}` (the phone)      |
| `/pair/{code}`        | GET    | one-tap approve page (scanned by phone) |
| `/devices`            | GET    | every paired device on the host         |

Pairing creates a REAL session in the same store Ed25519 logins use; paired
devices send `Authorization: Bearer <hex token>` on calls.

## Usage

```bash
node connect.js watch                # register + heartbeat loop (Ctrl-C leaves cleanly)
node connect.js register             # register once
node connect.js activity "message"   # push one activity event
node connect.js unregister           # leave the registry
node connect.js requests             # agent-provider requests awaiting this agent
node connect.js whoami               # identity, key mode, jurisdiction check (no secrets)
```

### Environment

| Var                | Default                    |
| ------------------ | -------------------------- |
| `OZONE_HOST`       | `http://127.0.0.1:50051`   |
| `ZCODE_AGENT_ID`   | `9002`                     |
| `ZCODE_AGENT_NAME` | `zcode`                    |
| `HEARTBEAT_SECS`   | `30`                       |
| `ZCODE_KEY_PATH`   | per-agent key if present, else `~/.ozone-cli/device-key.pem` |
| `ZCODE_PROVIDER_ENDPOINT` | `https://api.z.ai/` (destination judged by the jurisdiction check) |
| `ZCODE_JURISDICTION_CHECK` | `off` only for a standalone connector with no host |

## Building your own connector

The script is the documentation in motion: register, heartbeat by
re-registering, push real activity for everything worth observing, and
unregister on shutdown. Any language with an HTTP client can do the same —
that's the whole surface.
