# Ozone-Studio Shared Context — 3-way agent coordination

ZCode + Claude Code + Ozone-Studio share one coordination state. The
canonical store is `.ozone-context/` in this repo (git-visible); events
mirror live into Ozone-Studio's monitor feed (`:50051`). The MCP server is
zero-dependency Node: `tools/ozone-shared-context/server.js`.

## The protocol (both agents, every session)

1. **Start of work** → call `context_summary`. One call: who's live, what
   files are claimed, latest handoffs, recent Claude Code sessions.
2. **Before editing a file** → `file_claim` with your agent name. It REFUSES
   if another live agent holds the claim — that's the no-overwrite rule.
   Coordinate via notes, then the other agent releases.
3. **While working** → `presence_heartbeat` whenever focus changes (files,
   task). Silent >5 min = you drop out of "live" (claims of silent agents
   expire automatically).
4. **Done with files** → `file_release`. `file_claims` marks a claim `stale:true` when its holder has gone silent (>5 min): it
   blocks nobody and the next `presence_heartbeat` by anyone drops it.
5. **Ending a session** → `note_add` kind=`handoff`: what you did, what's
   next, gotchas. `finding` for discoveries, `decision` for choices made.

## Wiring

| Agent  | File                    | Scope    | Auto-connect |
| ------ | ----------------------- | -------- | ------------ |
| ZCode  | `.zcode/config.json`    | workspace | yes, session start (new sessions only) |
| Claude | `.mcp.json` (+ `.claude/settings.json` `enabledMcpjsonServers`) | project | yes (approve on first use) |

Both point at the same server; `OZONE_AGENT_NAME` distinguishes them. BOTH files also carry the `ozone-mcp` entry (the
walk/prompt surface, below) since 2026-11-12 — ZCode had only this coordination server before. Exact snippets, identity/key
setup and a verification checklist: `docs/AGENT_INTEGRATION_ZCODE_CC.md`.

`.ozone-context/` is git-IGNORED (`.gitignore`): notes, claims and the device key stay on the machine. (Earlier text calling it
"git-visible" was stale.) The server's own device key is written 0600.

## Claude Code session logs

`cc_sessions` reads `~/.claude/projects/-home-rebornbeat-Projects-Ozone-Studio/*.jsonl`
tail-only (read-only, never written). Override the dir with
`OZONE_CC_PROJECTS`.

## Ozone-Studio side

- Tool registered in the host MCP registry: `GET /mcp/tools`
- Coordination events appear in `GET /monitor/summary` (Monitor tab)
- Mirroring is built (`src/context_mirror.rs`, `POST /context/mirror`): notes
  and claims become `CoordinationEvent` containers under the SharedContext
  root (id 8), and non-claim events carry a `ForkOf` edge to the previous
  event of the same kind and agent. `context_summary` reads the host's
  recent `tool_call` / `config_change` events back as `host_events`.
  Still open: an edge from an event to the container it touched (the mirror
  request carries no subject id), and cross-host sharing (P2P carries only a
  finding signal today — this server stays local, stdio-only, by design).
- Tool servers the host starts log to `<zsei_data>/logs/mcp-<tool>.log`; a
  tool already listening on its port (systemd on the server) is recognized
  as running and is not re-spawned. The host forwards the caller's validated
  identity to HTTP tools in the call envelope (`identity_validated`,
  `session_user_id`) plus a `trace` object.

## Scoping (defined 2026-09-15 — the coordination graph is scoped, never flat)

The coordination graph mirrors Ozone-Studio's own scoping spine:
Session.active_workspace/active_project → OrchestrateRequest
.workspace_id/.project_id → context_aggregation's project-scoped +
separate-layer doctrine.

| Scope | Meaning | Examples |
| ----- | ------- | -------- |
| `global` | host-wide, visible to every workspace | presence, host-ops decisions, cross-workspace findings |
| `workspace` | bound to one workspace (DEFAULT) | notes/decisions/claims made in this repo |
| `project` | bound to one project inside a workspace | per-project coordination (requires workspace_id + project_id) |

Rules:
- Declared at session start: env `OZONE_SCOPE` / `OZONE_WORKSPACE_ID` /
  `OZONE_PROJECT_ID` (context_summary echoes the binding).
- Per-event override: `note_add` accepts `scope`/`workspace_id`/`project_id`.
- **Claims are always at least workspace-scoped** — file paths belong to a
  repo; a "global" claim is coerced.
- **Cross-workspace visibility is explicit only**: promote an event to
  `global` deliberately, or another workspace calls global items in.
  Never silent bleed.
- Graph encoding (what AMT/context queries filter on): keywords
  `scope:global` | `ws:<id>` | `proj:<id>`; materialized path
  `/SharedContext/<scope>/<kind>/<slug>`.
- Task 43 wires the AMT/context-aggregation pull: global + current
  workspace (+ project) events enter stage context as their OWN layer —
  never mixed into project-scoped container context.

## Using Ozone-Studio itself as an MCP client (2026-11-11)

`ozone-shared-context` is coordination only (presence, claims, notes, task routing). To RUN things on Ozone-Studio —
a walk, file/folder links, call graphs, work orders, gaps, the refinement report, ZSEI reads, marketplace views — an
agent also connects the sibling server `tools/ozone-mcp/server.mjs` (`.mcp.json` entry `ozone-mcp`; for ZCode add the
same command with `OZONE_AGENT_NAME=zcode`). Both servers share ONE state dir (`OZONE_CONTEXT_DIR`): every mutating
`ozone-mcp` call (`ozone_orchestrate`, `ozone_link`) appends a note there (agent, tool, content hash, lengths — never the
prompt or file text). It talks only to the host's public HTTP surface with the device-key session, never reads the data dir.
Design, hardening review and options: `docs/evidence/mcp-surface/REPORT.md`; drafted (not run) suite:
`tools/third-party-verify/mcp/`.
