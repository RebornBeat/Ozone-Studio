# Break-off decision and operator commands (2026-11-12)

**State found:** `tools/bridges` is a gitlink (mode 160000, commit 5f10948, origin github.com/dmae97/gamedev-all-in-one-mcp) with NO `.gitmodules` entry — a git directory inside the repo whose contents are not tracked by Ozone's history. `git submodule status` fails ("no submodule mapping").

**Decision:** VENDOR the engine part, UNREFERENCE the rest. The engine bridge (connectors, 67 tools, remote pipelines 200–203, stdio MCP) now lives as ordinary files in `tools/engine-bridge/` (AGPL-3.0 kept: `LICENSE`; header comments say so; package.json `ozone-engine-bridge`). The provider console (`control-plane/`) and the web dashboard + LLM proxy (`web/`) are NOT vendored: their capabilities are mapped to the host in CAPABILITY_INVENTORY.md. The host supervisor's :3210 entry now runs `tools/engine-bridge/dist/index.js`; nothing in Ozone references `tools/bridges` any more except docs that say it is retired.

**Not done (destructive or outside this task — run when satisfied):**
1. Stop tracking the gitlink (index only; the directory and its own repo stay): `git rm --cached tools/bridges && git commit -m "untrack the retired tools/bridges gitlink"`.
2. Optionally archive then remove the directory: `mv tools/bridges ~/gamedev-all-in-one-mcp.retired` (its own `.git` keeps its history; the upstream remote is unchanged).
3. Build the vendored bridge on each host: `cd tools/engine-bridge && npm install && npm run build` (no lockfile was copied: the old one pinned fastify).
4. Refresh the registry rows (idempotent, same endpoint `http://127.0.0.1:3210/call`; adds `bridge_status`): `node tools/engine-bridge/scripts/register-tools-to-ozone.mjs` with the host running.
5. If an instance still runs the OLD supervised console, stop it (it holds :3210): `systemctl`/process list → it is `node .../tools/bridges/dist/local.js`.

**Provider id** of the vendored bridge stays `bridges` (`mcp_supervisor::provider_for_endpoint`), so stored methodology/blueprint bindings and the 67 registry rows keep resolving.
