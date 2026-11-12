# ZCode + Claude Code ↔ Ozone-Studio — integration, setup and verification (2026-11-12)

Status: PARSE-ONLY (`node --check`, JSON parse). Nothing was run against a live host; the host on the EU still runs build8, which
predates `/walk/*`, `/agent/*`, `/history/*` and the jurisdiction routes — those calls return 404 until the new build is deployed.
Evidence and checklist lines: `docs/evidence/agent-integration/`.

## 1. The pieces

| Piece | File | What it is for | Auth to the host |
|---|---|---|---|
| `ozone-shared-context` (MCP, stdio) | `tools/ozone-shared-context/server.js` | Coordination only: presence, file claims, notes/handoffs, task routing, search bridge, CC session log. State in `.ozone-context/` (workspace-local, **git-ignored**). | Same-user **internal token** (`x-oz-internal`, read from `<data_dir>/.internal_token`) + its own device key for `/task/*`. |
| `ozone-mcp` (MCP, stdio) | `tools/ozone-mcp/server.mjs` | Ozone-Studio AS A TOOL: **prompt** (`ozone_orchestrate`), link files, read call graphs/work orders/gaps/refinement/ZSEI/marketplace, **join walks** (`ozone_walk_mine`, `ozone_walk_feed`), **be an agent provider** (`ozone_agent_offer`, `ozone_agent_requests`, `ozone_agent_report`). 13 tools. | **Device-key session only** (never the internal token, never reads the data dir). One process = one agent identity (H1). |
| `connect.js` (ZCode connector) | `tools/zcode-connector/connect.js` | Registry/heartbeat/activity reference; the pipeline-9 *model role* (unfillable for ZCode, see §5); in `watch` it announces agent-provider requests that await ZCode. | Device-key session (new 2026-11-12). |
| `agent-loop.js` | `tools/ozone-shared-context/agent-loop.js` | Presence heartbeat daemon for an agent with no session of its own. | via shared-context. |

Both agents need BOTH MCP servers: shared-context to coordinate, `ozone-mcp` to run things on Ozone. Neither agent calls the 20+
capability MCPs (terminal, web, code, …) directly — the walk routes tools; the agent prompts via `ozone_orchestrate`.

## 2. What was outdated or broken (found in this review)

| # | Finding | Effect | Fixed |
|---|---|---|---|
| A1 | `.zcode/config.json` had **only** `ozone-shared-context`. No `ozone-mcp`. | ZCode could coordinate but could not prompt Ozone, follow walks or offer itself. | Added `ozone-mcp` (`OZONE_AGENT_NAME=zcode`). |
| A2 | `.claude/settings.json` (committed) enabled only `ozone-shared-context`; `ozone-mcp` was enabled only in `settings.local.json`. | A fresh checkout would not load ozone-mcp. | Both enabled in `settings.json`. |
| A3 | `connect.js` sent **no credentials**. `/pipelines/register` is session-protected and the host's `OZONE_REQUIRE_SESSION` is ON by default. | Registration/heartbeat would 401 on any strict host. | Device-key session, re-auth on 401, clear hint. |
| A4 | `connect.js` forwarded model calls to ZCode's provider with **no jurisdiction check** (its endpoint is localhost, so the host's hop gate classed it local; the real destination is Z.ai). | Content leaving the machine unjudged. | Asks `POST /jurisdiction/outbound/check` (destination `ZCODE_PROVIDER_ENDPOINT`) before forwarding; held/unreachable = not forwarded (503). |
| A5 | `connect.js` README/comments described a contract with no auth, no `/agent/*`, no walk participation. | Misleading. | README + header rewritten. |
| A6 | Nothing told ZCode when an agent-provider step awaited its report (a session is not a daemon). | Walks parked until someone happened to call `ozone_agent_requests`. | `connect.js watch` polls `/agent/requests` and pushes a monitor-feed warning when the count changes; `node connect.js requests`. |
| A7 | `shared-context` wrote its device key `.ozone-context/device-key.pem` with default mode **0664**. | Private key readable by group. | Written 0600, re-chmod on load (existing file chmod'ed). |
| A8 | `file_claims`/`context_summary` listed claims of silent agents as live (four `claude-code-fork-sensor` claims, 10 h old). | Looked like blocking work; they block nobody but mislead. | `file_claims` rows carry `holder_live` / `stale`; the next `presence_heartbeat` drops them (it did, this pass — §7). |
| A9 | README/AGENTS.md said `.ozone-context/` is "git-visible"; `.gitignore` ignores it (and `walks/`, `device-key.pem`). | Wrong expectation of what is shared via git. | Corrected. |
| A10 | `serverInfo.version` mismatched the registered `server_version` in both servers. | Cosmetic. | Aligned (0.3.0 / 0.2.0). |
| A11 | No way to create a per-agent key for H1 without hand-rolled openssl. | `ozone_whoami` would say `shared` forever. | `node tools/ozone-mcp/server.mjs --init-key` (prints the PUBLIC key only). |

Claude Code: its wiring was current (both servers in `.mcp.json`, bound `OZONE_AGENT_NAME=claude-code`); the gaps were A2, the
explicit context dir / link roots (now set so the server does not depend on its working directory), and A7–A9.

## 3. Configuration (exact snippets)

### Claude Code — `.mcp.json` (project) + `.claude/settings.json`
```json
{
  "mcpServers": {
    "ozone-shared-context": {
      "command": "node",
      "args": ["/home/rebornbeat/Projects/Ozone-Studio/tools/ozone-shared-context/server.js"],
      "env": {
        "OZONE_HOST": "http://127.0.0.1:50051",
        "OZONE_AGENT_NAME": "claude-code",
        "OZONE_CONTEXT_DIR": "/home/rebornbeat/Projects/Ozone-Studio/.ozone-context"
      }
    },
    "ozone-mcp": {
      "command": "node",
      "args": ["/home/rebornbeat/Projects/Ozone-Studio/tools/ozone-mcp/server.mjs"],
      "env": {
        "OZONE_HOST": "http://127.0.0.1:50051",
        "OZONE_AGENT_NAME": "claude-code",
        "OZONE_CONTEXT_DIR": "/home/rebornbeat/Projects/Ozone-Studio/.ozone-context",
        "OZONE_MCP_LINK_ROOTS": "/home/rebornbeat/Projects/Ozone-Studio"
      }
    }
  }
}
```
`.claude/settings.json`: `{"enabledMcpjsonServers": ["ozone-shared-context", "ozone-mcp"]}`. Claude Code asks for approval on first use.
Forks of one Claude Code session speak through shared-context under their own names (`claude-code-fork-…`, allowed because
`OZONE_CONTEXT_STRICT_AGENT` is off); through `ozone-mcp` they always act as `claude-code` (H1) — omit the `agent` argument.

### ZCode — `.zcode/config.json` (workspace scope; user scope `~/.zcode/cli/config.json` would override it — none exists today)
```json
{
  "mcp": {
    "servers": {
      "ozone-shared-context": {
        "command": "node",
        "args": ["/home/rebornbeat/Projects/Ozone-Studio/tools/ozone-shared-context/server.js"],
        "env": { "OZONE_HOST": "http://127.0.0.1:50051", "OZONE_AGENT_NAME": "zcode",
                 "OZONE_CONTEXT_DIR": "/home/rebornbeat/Projects/Ozone-Studio/.ozone-context" }
      },
      "ozone-mcp": {
        "command": "node",
        "args": ["/home/rebornbeat/Projects/Ozone-Studio/tools/ozone-mcp/server.mjs"],
        "env": { "OZONE_HOST": "http://127.0.0.1:50051", "OZONE_AGENT_NAME": "zcode",
                 "OZONE_CONTEXT_DIR": "/home/rebornbeat/Projects/Ozone-Studio/.ozone-context",
                 "OZONE_MCP_LINK_ROOTS": "/home/rebornbeat/Projects/Ozone-Studio" }
      }
    }
  }
}
```
ZCode (desktop app v3.11.x per `~/.agents/memory/harness.md`, verified 2026-09-06 — re-verify) connects workspace servers at
**session start**: restart/open a new ZCode session after any config edit. It has no headless CLI.

### Environment (both servers)
| Var | Server | Meaning |
|---|---|---|
| `OZONE_HOST` | both | host base URL; `ozone-mcp` refuses a non-loopback host unless `OZONE_MCP_ALLOW_REMOTE_HOST=1` |
| `OZONE_AGENT_NAME` | both | identity; **required** on `ozone-mcp` (binds the process, H1) |
| `OZONE_CONTEXT_DIR` | both | the ONE shared state dir (default `./.ozone-context` — set it explicitly) |
| `OZONE_MCP_LINK_ROOTS` | ozone-mcp | ":"-separated roots `ozone_link` / `attached_files` may read (default = cwd) |
| `OZONE_AGENT_KEY_PATH` | ozone-mcp | explicit device key; else `~/.ozone-cli/agents/<name>/device-key.pem`, else the shared key |
| `OZONE_MCP_READ_ONLY=1` | ozone-mcp | refuse mutating tools (orchestrate, link, agent report/offer) |
| `OZONE_CONTEXT_STRICT_AGENT=1` | shared-context | opt-in H1 binding (off: one session may speak as several named forks) |
| `OZONE_INTERNAL_TOKEN[_FILE]` | shared-context | same-user token for strict hosts (found automatically in the data dir) |
| `OZONE_THROUGH_HOST=1` | shared-context | mirror events through `/mcp/call` so the host reviews them |
| `OZONE_OPERATOR_RELEASE=1` | shared-context | **operator only**: lets `search_release` run; never set it for an agent |
| `OZONE_SCOPE` / `OZONE_WORKSPACE_ID` / `OZONE_PROJECT_ID` | shared-context | coordination scope binding |

## 4. Identity, keys and strict sessions

* **Sessions.** With `OZONE_REQUIRE_SESSION` on (default), the host's gate requires a session for `/config/set`, `/mcp/tools*`,
  `/mcp/call`, `/chat/`, `/context/`, `/ripple/`, `/marketplace/`, `/network/`, `/task/`, `/walk/`, `/agent/`, `/jurisdiction/`,
  `/history/`, `/pipelines/register`, … (exempt: `/jurisdiction/outbound/check`). `ozone-mcp` and `connect.js` authenticate with
  the Ed25519 device-key challenge (`/auth/challenge` → `/auth/authenticate`); the token is cached, re-fetched once on 401, never
  printed (errors are scrubbed). `shared-context` uses the internal token instead (same OS user, 0600 file) and a device key for
  `/task/*`.
* **Shared key vs per-agent key.** Default: every agent uses `~/.ozone-cli/device-key.pem` (`ozone_whoami` → `key_mode: shared`).
  A per-agent key (`~/.ozone-cli/agents/<name>/device-key.pem`) gives each agent its own device identity: create it with
  `OZONE_AGENT_NAME=zcode node tools/ozone-mcp/server.mjs --init-key` (prints the public key only; the file is 0600), restart
  the agent session, check `ozone_whoami` → `per-agent`.
  **Read before enabling:** the host does not approve keys. `/auth/authenticate` does get-or-create — an unknown key is
  enrolled on first use as a NEW host user and device (`src/auth/mod.rs`). So (a) a per-agent key is a different *user* from
  the operator's (`user_id` ≠ 1; `ozone-mcp` still sends `USER_ID`/`DEVICE_ID` from env, default 1, in `/orchestrate`), and
  (b) the strict-session gate proves "a key signed", not "the operator approved this agent". Until the host gains key approval,
  prefer the shared key for operator-owned agents, and rely on H1 (process-bound name) + `OZONE_MCP_READ_ONLY` for restraint.
  This is a host design gap, not fixed here (src/ is outside this task).
* The host reads the walk caller from the request **body** `agent`; the `x-ozone-agent` header `ozone-mcp` also sends is ignored
  by the host (harmless).

## 5. The four things each agent must be able to do

### (1) Prompt Ozone-Studio — `ozone_orchestrate`
Tool: `ozone_orchestrate {prompt, workspace_id?, project_id?, attached_files?, participants?}` → `run_token`; poll `ozone_run_status`,
then `ozone_task_get` / `ozone_work_order` / `ozone_callgraph_live`. What stops it:
* ZCode: no `ozone-mcp` entry (fixed, A1). Claude Code: not approved in the UI / `enabledMcpjsonServers` (fixed in settings.json).
* No usable device key / host not reachable / `OZONE_MCP_READ_ONLY=1`.
* `attached_files` outside `OZONE_MCP_LINK_ROOTS`, or refused by the host link guard (secrets, data dir, `~/.ssh`, `/etc`, …).
* **Jurisdiction hold:** the first boot of the new build holds every call to a third-country/unverified provider (OpenRouter) until
  the operator accepts the three agreements (UI → Marketplace → Jurisdiction). The walk waits visibly (awaiting item); it does not fail.
* The EU host (build8) lacks the new routes (`/walk/*`, `/agent/*`); `/orchestrate` exists there, but the `agent`/`participants` body fields are unverified against that older binary.

### (2) Participate in walks — `ozone_orchestrate{participants:[…]}`, `ozone_walk_mine`, `ozone_walk_feed`
The caller is the first participant; named participants read the same feed (host `GET /walk/feed`, mirrored into
`.ozone-context/walks/`, git-ignored). Names must match `[A-Za-z0-9._-]{1,64}` and be the agents' real `OZONE_AGENT_NAME`s
(`claude-code`, `zcode`). Both agents must use the same `OZONE_CONTEXT_DIR`. Needs the new host build (`/walk/*`).

### (3) Offer itself as an agent provider — `ozone_agent_offer`, `ozone_agent_requests`, `ozone_agent_report`
`ozone_agent_offer {capabilities:[…]}` registers a tool row (`transport: agent`, endpoint `agent:<name>`); a walk step that picks
it creates a durable request and **waits with no timeout** for `ozone_agent_report {id, result}`; completed is final. What stops it:
* The agent must be in a live session that checks `ozone_agent_requests` (a session is not a daemon). `connect.js watch` (ZCode) or
  `agent-loop.js` + the monitor feed tell the operator/agent; Claude Code can poll from its session.
* **Jurisdiction (G4):** content handed to an agent is judged like any outbound call; the destination is the agent endpoint matched
  against the instance policy's `providers` rows, and an **unlisted agent is `third_country_unverified`** — held until the operator
  lists it and accepts the agreements. `claude-code` forwards to Anthropic and `zcode` to Z.ai: the operator chooses the region label
  for each row (`agent:claude-code`, `agent:zcode`) in UI → Marketplace → Jurisdiction → providers. Nothing is pre-listed.
* `agent:*` steps can be answered by any participant of that walk.
* **MCP as a model:** an agent provider is the only built form today. A generic "MCP tool as a selectable model in the model chain"
  entry is NOT built (decision pending with the operator); when it is, it goes through the same adapter seam, so the same check applies.

### (4) Coordinate through shared context
Protocol (`AGENTS.md`): `context_summary` first → `file_claim` before editing → `presence_heartbeat` while working → `file_release` →
`note_add` handoff. What stops it: host unreachable (local mode continues), `OZONE_CONTEXT_DIR` pointing at different dirs for
different agents (they will not see each other), a live holder's claim (refusal = coordinate), a strict host without the internal
token (`/task/*`, `/context/mirror` 401; the server finds `<data_dir>/.internal_token` by itself). Stale claims never block.

## 6. What ZCode was last doing
ZCode's last handoff is **2026-11-08 18:49** ("ZCode takeover: CC's arc captured+verified…"): it took over after Claude Code's
weekly limit, captured CC's keyword-program arc (d7127c1), landed wave 2 (cb050d9), verified SW1c / keyword E2E 9/0/1 / lib 257/257,
resolved the "literal ruling" (confidence gone from production, hotness kept), and left an operator-gated queue: push (13 ahead
then), key rotation, flash-1 drop-gate floor, EU measurement run, pretrained-encoder weights, stale-task cleanup. Claude Code
resumed the same evening; ZCode has not written to shared context since (4 days) and its presence is silent. Its last work item
was not an integration refactor: the integration pieces (`connect.js`, `.zcode/config.json`) were last touched in September, before
ozone-mcp, walk participants, strict sessions and the jurisdiction gate existed — that is what this pass brings up to date.

## 7. Stale-claims cleanup
At the start of this pass `file_claims` listed 4 claims held by `claude-code-fork-sensor` ("sensor raw-input path", ~614 min old, holder
silent): `assets/pipelines/shared/modality_ext.rs`, `src/orchestrator/file_reader.rs`,
`tools/third-party-verify/mcp/m3-link-graphs.mjs`, `docs/evidence/sensor-raw-input`. The sensor raw-input path is DEFERRED by the
operator ("ensure captured") and none of those files had uncommitted changes. The tool's own expiry removed them when this pass sent
its first `presence_heartbeat` (claims of silent agents are dropped by the next heartbeat) — no force-release was used. Re-claim
them when the sensor work resumes. `file_claims` now flags such rows `stale:true`.

## 8. Verification checklist (run after the new host build is deployed; all drafted, none run)
1. `node --check tools/ozone-mcp/server.mjs tools/ozone-shared-context/server.js tools/zcode-connector/connect.js` — done, passes.
2. Host up: `curl -s http://127.0.0.1:50051/health`.
3. Each agent session: `ozone_whoami` (agent name, `key_mode`, `read_only: false`, link_roots); `context_summary`.
4. ZCode: `node tools/zcode-connector/connect.js whoami`, then `watch` — register succeeds on a strict host (no 401); monitor feed shows it.
5. `node tools/third-party-verify/mcp/m1-coordination.mjs` — three agents, surface, claims (P1–P5 incl. `holder_live`), identity binding (I2/I3).
6. `node tools/third-party-verify/mcp/m4-participants.mjs` — Part A (no model calls): offer → request waits → only the target sees it → wrong agent refused → target's report completes → second report refused. Part B (`OZONE_TPV_WALK=1 … "<prompt>"`): walk with participants, identical feeds.
7. `OZONE_TPV_WALK=1 node tools/third-party-verify/mcp/m2-walk.mjs "<prompt>"` — A starts, B/C observe, completion gate, live call graph (the first real walk).
8. `node tools/third-party-verify/jurisdiction/j1…j6` first (read-only) — confirm held/allowed behaviour for `agent:zcode` / `agent:claude-code` rows.
9. Operator-run once: `node tools/ozone-mcp/server.mjs --register` (registers the ozone-mcp row with the host, H10).
Expect shape mismatches on the first live run: fix the script from the real payload, never loosen a check.

## 9. Not done / needs the operator
* Per-agent keys vs shared key (§4) — and a host-side key-approval step (src/auth/mod.rs get-or-create) if per-agent identity should mean something.
* Region labels for `agent:claude-code` / `agent:zcode` in the jurisdiction policy; agreements acceptance.
* `/pipelines/unregister`, `/monitor/activity` (POST) are outside the strict-session prefixes today (another fork is adding internal-token paths); `connect.js` already authenticates all its calls so it works either way.
* `handleModelCall()` for the pipeline-9 model role stays unfilled: ZCode has no headless invocation (verify against current ZCode docs before filling).
* A UI/launcher for `connect.js watch` and `agent-loop.js` (they are manual commands).
