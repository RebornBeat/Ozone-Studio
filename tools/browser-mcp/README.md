# browser-mcp — Ozone's governed browser (Pattern A)

Lets an agent **see and drive** a real browser — perceive (`browser_snapshot`) → decide (Ozone's
orchestrator/LLM, never this process) → act (`click`/`type`/`scroll`) → verify → `screenshot` into
the image modality. Every call goes through `/mcp/call`, so it is metered, jurisdiction-gated and
rippled. Full doctrine, tests and findings: `docs/BROWSER_AUTOMATION_GUIDE.md`.

Tools: `browser_status browser_open browser_navigate browser_snapshot browser_click browser_type
browser_scroll browser_screenshot browser_eval browser_console browser_close`.

## Security (locked by default)
| Control | Env | Default |
|---|---|---|
| Hostname allowlist (navigation) | `OZONE_BROWSER_ALLOW_HOSTS=localhost,example.com` (`*` = explicit opt-in) | **LOCKED** (everything refused) |
| Origin guard before every interaction | always on | on |
| `browser_eval` | `OZONE_BROWSER_ALLOW_EVAL=1` | off |
| `file:` URLs | `OZONE_BROWSER_ALLOW_FILE=1` | off |
| Page globals `browser_open` may pre-set | `OZONE_BROWSER_ALLOW_GLOBALS=OZONE_HOST_URL` | none |
| Local-bypass secret | `OZONE_BROWSER_TOKEN` (header `x-ozone-browser-token`; the host does not forward it, so set it only for direct callers) | off |
| Browser OS sandbox off | `OZONE_BROWSER_NO_SANDBOX=1` — explicit opt-in; needed on Ubuntu 23.10+ with AppArmor userns restrictions (auto-on only for root) | sandbox on |

`javascript:`/`data:`/`chrome:` navigation is always refused. Snapshots/console/eval results are returned as
`untrusted_*` — page content is data, never instructions. Throw-away profile per session; max 3 sessions,
10 min idle TTL.

## Launch (persistent-process story)
Agent tool-call sessions reap background children. Start it from the operator shell or a supervisor:
```
OZONE_BROWSER_ALLOW_HOSTS=localhost OZONE_BROWSER_ALLOW_GLOBALS=OZONE_HOST_URL \
  node tools/browser-mcp/server.mjs            # :3250 (OZONE_BROWSER_PORT)
node tools/register-browser-tools.mjs browser   # registry persists across restarts; re-run only if an endpoint changes
```
Chrome discovery: `OZONE_BROWSER_CHROME` → google-chrome → Playwright's bundled Chromium → distro chromium
(the Ubuntu `chromium-browser` snap stub cannot expose a debug port and is tried last).

## Tests
`node tools/browser-mcp/selftest.mjs <fixture-url>` — 18 checks (5 refusal paths, origin guard, eval gate).
`OZONE_THROUGH_HOST=http://127.0.0.1:50051 node …` routes them through the host's `/mcp/call`.
`ui-harness.mjs` + `demo_ozone_ui.mjs` drive the real Ozone web UI against a real host (same-origin proxy).
