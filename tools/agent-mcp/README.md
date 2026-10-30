# agent-mcp — the DOMorpher conversion's Layer 3 (enforcement)

`docs/DOMORPHER_CONVERSION_GUIDE.md` is the doctrine. This MCP is the
enforcement point over the `domorpher-core` library (Layer 1) and its
tool surface (Layer 2): loopback-only `/call`, `{tool, agent, input,
context}` in, `{success, output|error}` out.

## State

**Skeleton (declared-but-honest).** The 16-tool table is live in
`/status`; every capability tool answers
`harvest pending: domorpher-core is not wired` until the core crate is
mounted (`OZONE_AGENT_CORE=1` + the dispatch route in `server.mjs`).
No mock successes — the selftest ASSERTS the honest refusals.

## Tools (the ActionType vocabulary; `Custom(String)` deliberately absent)

| Tool | Class | Extra gate |
|---|---|---|
| agent_click / hover / scroll / key / drag | interaction | session |
| agent_input / select / check / upload | input | session |
| agent_navigate | navigation | `OZONE_AGENT_ALLOW_HOSTS` (unset = LOCKED) |
| agent_wait / switch_frame / status | safe | — |
| agent_extract | read | — |
| agent_dialog | dialog | confirm:true |
| agent_exec_script | script | `OZONE_AGENT_ALLOW_EVAL=1` (the ExecuteScript gate) |

## Env

```
OZONE_AGENT_PORT=3283                 # loopback only
OZONE_AGENT_BACKEND=farm|local        # AG7: farm = PRIMARY (profiles +
                                      # Command Center); local = isolated
                                      # browser-mcp CDP. Config, never a bypass.
OZONE_AGENT_ALLOW_HOSTS=host1,host2   # navigation allowlist (unset = LOCKED)
OZONE_AGENT_ALLOW_EVAL=1              # unlock agent_exec_script
OZONE_AGENT_READONLY=1                # read tools only
OZONE_AGENT_TOKEN=...                 # optional shared secret (x-ozone-agent-token)
OZONE_AGENT_CORE=1                    # core crate wired (when harvest lands)
```

## Launch (persistent-process doctrine)

Agent sessions reap background children — launch from the operator's
shell, a supervisor, or the launcher:
`node tools/agent-mcp/server.mjs` (append to
`target/release/launch-browser-stack.sh` when it goes live).

## Selftest

`node tools/agent-mcp/selftest.mjs` — asserts LOCKED-by-default, the
nav-host refusal, the eval-gate refusal, the harvest-pending honest
error, and status shape. Refusal paths are tested FIRST (doctrine §6).
