# Bridges incorporation — report (2026-11-12)

Commit 61f0ed5 (parse-only: rustfmt, ts.transpileModule, node --check; nothing built or run; EU untouched). Written by the parent from the fork's output. Details: CAPABILITY_INVENTORY.md, DECISION.md, CHECKLIST_LINES.md (L240–L246).

## Defect found
The supervisor's :3210 entry was `tools/bridges/dist/local.js` (the provider console), which answers neither `/call` nor `/execute`: a supervised host had no working engine route; `/call` only worked when `dist/index.js` was started by hand. The entry now runs the vendored engine bridge.

## Capability inventory (read in full except dashboard.html fetch calls)
Console: OpenAI, Anthropic, z.ai, MiniMax, Codex CLI, Claude Code CLI, ZCode CLI. Web proxy adds Google and the engine-tool chat loop.
- Missing, now added: vendor APIs as `config::VENDOR_PRESETS`; the three CLI agents as a `cli` transport; engine status as a `bridge_status` tool on `/call`.
- Superseded: skill/MCP selection, job history/review, dashboard activity stream, static tool list, browser-typed keys, chat loop.
- Fixed on the way: the engine bridge's `/execute` and `/call` ran Luau/Python in the engine for any local process; they now need the host internal token on a strict host.
- Not carried over (they were caps): Anthropic max_tokens 1024/4096, 64 KiB output limit, 200 job records, 10 tool iterations, 20 SSE clients / 500 activity events.

## Host provider layer
- Vendors are data (host already speaks Anthropic + chat-completions wire protocols). `set_config` now sets endpoint + wire protocol for every named provider (before: only OpenRouter; choosing Anthropic/OpenAI/Google in the UI left the old endpoint). `add_model` fills the endpoint for an entry naming only its provider.
- Paid refusal covers z.ai, MiniMax, the CLI agents and `cli:` endpoints: selectable only with `allow_paid_models`; nothing auto-added to a chain.
- CLI transport: `model_type "cli"`, endpoint `cli:<codex|claude|zcode>`; prompt on stdin, stdout is the answer; empty answer / non-zero exit = error; child runs in a scratch dir with a cleared environment (only that CLI's own credentials pass); an arbitrary `cli:` name is never spawned unless `OZONE_CLI_<NAME>_COMMAND` is set; cleared by the same jurisdiction gate before the spawn (CLI whose vendor is unlisted = held).
- Orchestrator treats `cli` as remote in the local-last ordering. Shipped policy `providers` rows for each vendor host and each `cli:` destination.
- UI (Settings): new model type/provider options, Provider column + Remove for registered models, "Add a provider model" card, fallback-chain editor (reorder, remove, add-from-registered, free-only).

## Break-off decision
`tools/bridges` is a gitlink with no `.gitmodules` entry. The engine bridge is vendored into `tools/engine-bridge/` (AGPL licence kept); the console and web proxy are not. Provider id stays `bridges` (stored bindings and the 67 registry rows keep resolving). The supervisor sets `OZONE_SUPERVISED=1` so the bridge skips its stdio surface. NOT run: `git rm --cached tools/bridges` and removing the directory — operator commands are in DECISION.md.

## Check first at the first compile
CLI transport in the prompt crate (`cli_spec` tuple match, `run_bounded_child_stdin`); preset constants used from grpc/mod.rs; local-rank edit in orchestrator/mod.rs; supervisor entry/env/provider-id alias.

## Open
Nothing compiled. Vendored bridge needs `npm install && npm run build` per host (no lockfile copied: the old one pinned fastify); registry refresh script after that. Not verified/built: ZCode headless flags, per-provider availability probe, bridge tests. Not carried over: OpenAI Responses-API features, Google native function calling. An instance with a saved jurisdiction policy won't pick up the new `providers` rows by itself.
