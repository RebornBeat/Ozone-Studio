# Tools, MCPs, pipelines and libraries — identified by language (2026-10-31)

Operator direction: Ozone-Studio already allows tools in different languages
(TOOLS_PIPELINES_MCP_GUIDE §1: a tool is an "external process / server (any
language)"). The Browser-Farm is Python and domorpher-core is Rust, so the
need is a **Python library for domorpher**, and both languages must be
captured and identified by language. Capture only; no build or test run.

## 1. What exists today (read from the repo, 2026-10-31)

| Class | Language | Count / where | Door into Ozone |
|---|---|---|---|
| MCP servers (the HTTP `/call` wrapper) | **Node** (`.mjs`) | every `tools/*-mcp` (security, terminal, android, browser, farm, git, code, web, visual, gcal, zoom, dna, chemistry, proteomics, geospatial, mesh, re, p2p-bridge, threed, connectome, cerebrix, code-graph, domorpher-agent) + ozone-cli, ozone-mobile, ozone-shared-context | `/mcp/tools/register` -> `/mcp/call` |
| Heavy-compute workers behind a Node wrapper | **Python** | cerebrix (2), connectome (3), dna, threed, code-graph, browser-farm-mcp (1 helper) | the Node MCP spawns/calls them; never registered directly |
| Pipelines | **Rust** | 82 independent crates under `assets/pipelines/` | registry gate, `{data,context}` envelope |
| Bridges | Node + engine plugins (Luau/Python/C#) | `tools/bridges/` (operator pushes it; untouched) | `/pipelines/register` |
| Browser-Farm itself | **Python** package (`pyproject.toml`, FastAPI + Playwright) | `~/Projects/Browser-Farm`, EU VPS | `farm_*` tools through browser-farm-mcp :3281 |
| domorpher-core | **Rust** crate | `~/Projects/domorpher/core` | not a tool; not a pipeline; a library (see 3) |

Gap found: the registry has **no language field**. `McpToolRegisterRequest`
carries `name, transport, endpoint, capabilities, server_version`. Platform is
already carried as a capability tag (`platform:linux/x64`, TOOLS guide §13).
Language was implicit in the file extension, so a Python or Rust tool could
not be identified, filtered, or health-checked by language.

## 2. Identification rule (cheap, no host change)

Use the same mechanism as platform: capability tags, plus an eventual native
field.

- `lang:<node|python|rust|luau|csharp|shell>` — the language of the
  process that answers the call
- `kind:<mcp|pipeline|bridge|library|worker>` — the class
- `runtime:<requirement>` — optional, e.g. `runtime:python>=3.11`,
  `runtime:node>=20`

Done in this pass: `tools/register-browser-tools.mjs` now tags every
registered tool `lang:node` and `kind:mcp`. Next: the other registrars, and a
`GET /mcp/tools?lang=python` filter + supervisor facts. Native field
(`language`, `runtime`) later, same as the `platform` field plan.

Libraries are not registered as tools. They are **recorded** (this document
plus a ZSEI contract container) and surfaced through the tool that hosts
them, e.g. `farm_*` tools list `lib:domorpher-py@x.y` in capabilities when the
farm reports the library loaded.

## 3. domorpher: one canonical crate, one Python library, one host

```
domorpher-core   (Rust crate, canonical logic, pure: no model, no sockets)
      │
      ├── domorpher-py   (Python wheel via PyO3 + maturin; thin bindings)
      │         └── imported by the Browser-Farm CC bridge script inside the
      │             farm venv  (farm = host; one-way optional dependency)
      │
      └── domorpher-cli  (optional thin binary: stdin JSON -> stdout JSON;
                          usable as an Ozone pipeline or for offline tests)
```

Why a Python library and not only a binary:
- The farm is Python; its CC bridge script (`cc_script_ozone_bridge.py`) can
  call the library in-process with the live DOM, no extra process hop.
- One-way dependency: the library knows nothing about the farm. The
  Browser-Farm repo is not edited by this design; the wheel is installed
  into its venv and the (already external) CC bridge script imports it.
- The crate stays canonical: Python is bindings, not a port. No logic fork.
- domorpher-cli keeps the offline path (D2 in the google.com test) and
  matches the pipeline tier, so Ozone can also call it directly.

Python API (pure functions, JSON in / JSON out; decisions arrive as data):
- `dom_to_field(html: str, opts: dict) -> dict`  traversable field
- `plan_actions(field: dict, decision: dict) -> list[dict]`  action sequence
  (human-like typing, pauses) from a decision supplied by Ozone
- `extract(html: str, spec: dict) -> dict`  items from the DOM
- `needs_judgment(...) -> dict | None`  the outward signal
- `version() -> str`

Rules (carried from the doctrine): no model client, no sockets, no
persistence in the library; every call that touches the web is executed by
the farm and goes through `/mcp/call` for the CC/Ozone side.

## 4. Capture and tests to wire (MASTER_TEST_GUIDE §7c / §17 follow-up)

| id | check | expected |
|---|---|---|
| LANG1 | registry listing after `register ... all` | every tool carries `lang:node` and `kind:mcp` (post-gate; EU registrar copy must be refreshed first) |
| LANG2 | grep gate: no tool answers without a `lang:` tag | zero untagged |
| LANG3 | `maturin develop` builds domorpher-py from domorpher-core | wheel imports in the farm venv |
| LANG4 | `dom_to_field` on the captured google.com HTML (T-DOM-GOOGLE D1/D2) | field has search input + buttons |
| LANG5 | `import domorpher` in a venv that has no Rust toolchain (wheel only) | works |
| LANG6 | dependency direction: Browser-Farm repo has no import of the crate; only the CC bridge script imports `domorpher` | grep clean |
| LANG7 | AG6 on the wheel: no network/model symbols | `cargo tree` clean |

## 5. Decision recorded (supersedes 4b option 1 as primary)

Primary: **domorpher-py (PyO3)** for the farm, **domorpher-cli** as the
offline/pipeline path, both built from the one crate. This is operator-led
("we should build a python library"); the earlier recommendation of a binary
only is demoted to the secondary path.
