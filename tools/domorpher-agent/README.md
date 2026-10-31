# domorpher-agent — the DOMorpher conversion's Layer 3 (enforcement)

`docs/DOMORPHER_CONVERSION_GUIDE.md` is the doctrine. This MCP is the
enforcement point over the `domorpher-core` library (Layer 1) and its
tool surface (Layer 2): loopback-only `/call`, `{tool, agent, input,
context}` in, `{success, output|error}` out.

## STATUS: UNUSED / UNREGISTERED (operator, 2026-10-31)

The Browser-Farm is driven through the Command Center via the `farm_*` tools
(browser-farm-mcp :3281); Ozone-Studio and consciousness make the decisions.
This MCP is kept as the general enforcement skeleton for later reuse — code and
all 16 verbs stay — but it is **not started by the launcher** (opt-in:
`OZONE_START_DOMORPHER_AGENT=1`) and **not registered by the registrar's `all`
target** (explicit: `node tools/register-browser-tools.mjs domorpher_agent`).
Stale pre-rename `agent_*` rows are cleared with
`node tools/register-browser-tools.mjs --prune agent_` (live host route
`POST /mcp/tools/unregister {name}`), or, with the host STOPPED, by a backed-up
edit of `{data_dir}/mcp_tool_registry.json`.

## State

**Skeleton (declared-but-honest).** The 16-tool table is live in
`/status`; every capability tool answers
`harvest pending: domorpher-core is not wired` until the core crate is
mounted (`OZONE_DOMORPHER_CORE=1` + the dispatch route in `server.mjs`).
No mock successes — the selftest ASSERTS the honest refusals.

## Tools (the ActionType vocabulary; `Custom(String)` deliberately absent)

| Tool | Class | Extra gate |
|---|---|---|
| domorpher_click / hover / scroll / key / drag | interaction | session |
| domorpher_input / select / check / upload | input | session |
| domorpher_navigate | navigation | `OZONE_DOMORPHER_ALLOW_HOSTS` (unset = LOCKED) |
| domorpher_wait / switch_frame / status | safe | — |
| domorpher_extract | read | — |
| domorpher_dialog | dialog | confirm:true |
| domorpher_exec_script | script | `OZONE_DOMORPHER_ALLOW_EVAL=1` (the ExecuteScript gate) |

## Env

```
OZONE_DOMORPHER_PORT=3283                 # loopback only
OZONE_DOMORPHER_BACKEND=farm|local        # AG7: farm = PRIMARY (profiles +
                                      # Command Center); local = isolated
                                      # browser-mcp CDP. Config, never a bypass.
OZONE_DOMORPHER_ALLOW_HOSTS=host1,host2   # navigation allowlist (unset = LOCKED)
OZONE_DOMORPHER_ALLOW_EVAL=1              # unlock domorpher_exec_script
OZONE_DOMORPHER_READONLY=1                # read tools only
OZONE_DOMORPHER_TOKEN=...                 # optional shared secret (x-ozone-agent-token)
OZONE_DOMORPHER_CORE=1                    # core crate wired (when harvest lands)
```

## Launch (persistent-process doctrine)

Agent sessions reap background children — launch from the operator's
shell, a supervisor, or the launcher:
`node tools/domorpher-agent/server.mjs` (append to
`target/release/launch-browser-stack.sh` when it goes live).

## Selftest

`node tools/domorpher-agent/selftest.mjs` — asserts LOCKED-by-default, the
nav-host refusal, the eval-gate refusal, the harvest-pending honest
error, and status shape. Refusal paths are tested FIRST (doctrine §6).
