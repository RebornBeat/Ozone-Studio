> **CORRECTIONS (2026-10-29, same day — see `docs/MASTER_REMAINING_WORK_GUIDE.md` §0):** (1) the MCP registry **is
> persisted** (`{data_dir}/mcp_tool_registry.json`, restored at boot); statements below that it is in-memory / must be
> re-registered after every restart are wrong — re-register only when an endpoint changes. (2) `browser-farm-mcp` has
> **only** been tested against a mock farm I wrote; that is not proof against Browser-Farm. The operator's direction is
> to test the real farm (locally or on the EU VPS) and retire the mock.

# Browser automation in Ozone-Studio — browser-mcp, browser-farm-mcp, DOMorpher

> 2026-10-29 (cc). Operator direction: *"you tested with Chrome by hand — now set that up through Ozone-Studio
> tools and MCPs, do and surpass it; Browser-Farm and DOMorpher are separate projects, DOMorpher may be a module;
> Browser-Farm has a Command Center so Ozone can command it all from there; capture it all."* This guide is that
> capture. Doctrine it follows: `docs/NEW_MCP_GUIDE.md` (Pattern A local MCP, Pattern B absorb/bridge).

## 1. The three pieces and how they fit

```
                         Ozone-Studio host  (/mcp/call: identity · metering · jurisdiction gate · ripple · review)
                                   │
        ┌──────────────────────────┼───────────────────────────────┐
        ▼                          ▼                               ▼
  browser-mcp (:3250)      browser-farm-mcp (:3252)       DOMorpher  (library, not yet buildable)
  local governed browser   commander of a farm            DOM→semantic prep · chunking · extraction · agent loop
  CDP · snapshot/click/…   REST + X-API-Key ─▶ Browser-Farm server(s) ─▶ profiles (Xvfb+proxy+fingerprint)
                                   └─ shared state ◀──▶ COMMAND_CENTER profile script ──▶ worker profiles
```

* **browser-mcp** replaces the hand-written CDP scripts agents were using to verify the UI. Same technique
  (headless Chrome + CDP), now a governed capability.
* **browser-farm-mcp** is the Ozone end of Browser-Farm's Command Center: observe the farm, start/pause/stop
  profiles, steer a profile, pull screenshots into the image modality, and exchange COMMAND/RESULT records with
  the CC profile through the farm's shared state.
* **DOMorpher** is the *intelligence* DOMorpher describes (DOM understanding → decide → act). In Ozone that loop
  is: perception = `browser_snapshot`, decision = Ozone's orchestrator/LLM, action = the click/type/… tools.
  DOMorpher's own crate is the candidate for richer preprocessing/extraction once it compiles (§4).

## 2. browser-mcp — what exists and what was proven

Tools: `browser_status browser_open browser_navigate browser_snapshot browser_click browser_type browser_scroll
browser_screenshot browser_eval browser_console browser_close` (`tools/browser-mcp/`, README there).

**Perception (`browser_snapshot`)** is computed in the page: URL/title/viewport/scroll, a landmark/heading outline,
visible body text (capped), and up to 150 *numbered interactive elements* (tag, role, label, href, value, box). Indexes
stay valid until the next snapshot; click/type take `index`, `selector` or `x,y`. Hidden elements are excluded. This
is DOMorpher's "DOM Preprocessor + semantic outline" in ~40 lines and token-cheap.

**What "surpass" means here** (vs. my hand-run CDP scripts): governed (allowlist, origin guard, metering, gate,
ripple), reusable by every agent, element-index driving instead of pixel guessing, console capture, screenshots
written for `visual_describe`/`visual_ingest` (image modality 102), sessions with TTL/limits, refusal paths tested.

**Security model** (written before capability): locked until `OZONE_BROWSER_ALLOW_HOSTS`; origin guard re-checks the
live page host before *every* interaction (a redirect/link to a foreign host cannot be acted on); `browser_eval` off
by default; `file:`/`javascript:`/`data:` refused; page globals only from `OZONE_BROWSER_ALLOW_GLOBALS`; results of
page content are returned as `untrusted_*`; throw-away profile, no extensions, loopback debug port, max 3 sessions,
10 min idle TTL. `OZONE_BROWSER_NO_SANDBOX=1` is an explicit operator opt-in (see §6.2).

**Evidence (real runs, 2026-10-29)**
| Test | Result |
|---|---|
| `selftest.mjs` direct against the MCP | **18/18** (5 refusal paths: locked-by-default, off-allowlist, `javascript:`, `file:`, unknown session; origin guard blocks snapshot **and** eval after an in-page link to a non-allowlisted host; eval disabled by default) |
| Same suite **through a host's `/mcp/call`** (scratch host :50099, 24 tools registered) | **18/18**; usage ledger recorded 19 rows (`/mcp/usage`), each response carried the `jurisdiction_gate` + `review` envelope |
| Real Ozone web UI, perceived + driven by the MCP (`ui-harness.mjs` + `demo_ozone_ui.mjs`) | snapshot saw the real app (24 interactive elements by name: Workspace, Files, Order, Tasks, …); click Workspace → the pipeline-delivered Workspace tab rendered |
Screenshots: `docs/evidence/browser-mcp/01…03`.

**Not done / honest limits:** no Electron driving (this drives web mode; Electron needs its own CDP attach); the
host forwards no token header, so `OZONE_BROWSER_TOKEN` only protects *direct* callers; host→tool timeout is 30 s;
(correction: the registry persists; re-register only if an endpoint changes). The registrar
could only be proven against a **scratch host** (fresh data dir ⇒ first-run exemption): the operator's real host
is not running and its binary predates the `x-oz-internal` token code, so registration there will 401 until the
rebuilt binary is deployed.

## 3. Browser-Farm — assessment and the Command Center hook

`~/Projects/Browser-Farm` (git, 13 commits; working tree has uncommitted edits — README, client GhostCursor/mouse
batching removed, Monitoring page). Server: FastAPI (`server/src/server.py`, 833 lines + context manager, stealth,
Xvfb, XTest input, FFmpeg recorder, memory monitor, PostgreSQL models), Electron/React client, versions 0.3.5–0.3.7 on
dist. API: `X-API-Key` (generated at server start), `/health`, `/profiles[/{id}]`, `/profiles/{id}/{start|pause|stop|
navigate?url=|refresh|go_back|screenshot|metrics|recording/*}`, WebSocket `/stream` + `/control`, `/state/{key}`
shared state, `/proxies`, `/history/record`. **Command Center** = a profile with `mode=command_center` (singleton per
server) whose script orchestrates other profiles via shared state.

**Could not run it here:** the farm needs PostgreSQL, Playwright, FastAPI, Xvfb and (ideally) Google Chrome; this
machine has Xvfb/ffmpeg only. So `browser-farm-mcp` is proven by a **contract test against a mock farm** built from
the real route shapes (20/20) — *not* a live farm. A live acceptance run is an open step (§7).

**Ozone → farm (what the MCP exposes):** `farm_status` (per-farm health), `farm_profiles`/`farm_profile` (secrets
stripped), `farm_profile_start|pause|stop`, `farm_navigate` (allowlist), `farm_screenshot` (downloaded to a local file for
the image modality), `farm_metrics`, `farm_state_get|set`, and the **CC channel**: `farm_command` writes
`ozone~commands~<target>` (`/` is stored as `~` because FastAPI path params cannot carry `/`), the CC profile runs
`cc_script_ozone_bridge.py` (ping | echo | relay, everything else refused), and `farm_result` reads
`ozone~results~<target>`.

**Deliberately outside the boundary:** script creation/execution (arbitrary code on the farm), the account vault,
proxy registration/credentials, profile delete. Farm profiles carry anti-detection fingerprints and proxies; *where* an
agent may point them is an operator decision (`OZONE_FARM_ALLOW_HOSTS`, `OZONE_FARM_READONLY=1`). Browser-Farm's own
disclaimer (legitimate automation/testing only) applies and Ozone's jurisdiction gate runs on every `/mcp/call`.

## 4. DOMorpher — which project is more complete, and the state of the code

Two copies exist:

| | `~/Projects/domorpher` (lowercase) | `~/Projects/DOMorpher` (upper) |
|---|---|---|
| Shape | one crate: `Cargo.toml`, `Cargo.lock`, `src/` | `domorpher-rust/src` only, README, TECHNICAL_DOCUMENTATION, LICENSE (no `Cargo.toml`) |
| Size | **125 `.rs` files, ~146k lines** | 22 files, ~21k lines |
| Modules | agent, api, browser, chunking, dom, enterprise, execution, extension, extraction, forms, instruction, interactive, llm, monitoring, parallel, pipeline, reconciliation, schema, storage, utils, visual | chunking, dom, llm (+prompt) , lib, main |
| Relation | **strict superset**: every file the upper copy has is byte-identical here; upper's `llm/prompt/` has the same contents as lower's `llm/prompts/` (`diff -rq` clean — only the directory name differs) | subset, 25 files tracked in git |

**Verdict: `~/Projects/domorpher` (lowercase) is the complete one — use it.** Upper adds nothing (its `llm/prompt/` is identical to lower's `llm/prompts/`), so it is safe to archive.

**Neither builds today.** The lowercase `Cargo.toml` declares **no dependencies** although the code uses tokio, serde,
reqwest, html5ever, scraper, sqlx, hyper, warp, image, handlebars, rayon, regex, uuid, chrono, lru, … ; and `cargo
check` stops at the lexer with 27 errors, **all in `src/agent/action.rs`** (raw strings like `r"with text ['\"]…"` —
`\"` does not escape inside `r"…"`; needs `r#"…"#`). Behind that, 146k lines have never been type-checked. Zero
`todo!()`/`unimplemented!()`, so the code is *written*, but "complete" ≠ "compiles".

**Plan (not started — needs the operator's go, it is a multi-session campaign):** (1) fix the `action.rs` raw strings
and declare deps; (2) compile a **core** behind cargo features first — `dom` (preprocessor, semantic markdown),
`chunking`, `llm`, `extraction`, `schema`, `reconciliation` — and defer enterprise/extension/monitoring/distributed;
(3) expose the core to Ozone either as a small JSON CLI wrapped by an MCP (`dom_to_markdown`, `extract`) or in-process;
(4) until then DOMorpher's *role* is already served by `browser_snapshot` + Ozone's orchestrator. Its Python API in the
README (`pip install domorpher`, `AutonomousAgent`) is not this repo and was not verified to exist on PyPI — do not
assume it is importable inside Browser-Farm scripts.

## 5. Files added / changed

`tools/browser-mcp/{server.mjs,selftest.mjs,ui-harness.mjs,demo_ozone_ui.mjs,README.md}`,
`tools/browser-farm-mcp/{server.mjs,selftest.mjs,cc_script_ozone_bridge.py,README.md}`,
`tools/register-browser-tools.mjs`, `docs/evidence/browser-mcp/*.png`, this guide, TOOLS_REGISTRY §6–7.
UI (found *by driving the real UI through the new MCP*): `ui/src/pipeline-ui.tsx` — web-mode HTTP fallback for
pipeline registry / UI component / execute (previously every pipeline-delivered tab — Workspace, Tasks, Library,
Settings — said "No UI module found" outside Electron; execute still needs a paired device's session token);
`ui/src/App.css` — the floating search/bell overlay was hiding the right end of the tab strip and the strip showed a
white scrollbar.

## 6. Findings worth knowing

1. **Ubuntu `chromium-browser` is a snap stub** — cannot expose a debugging port from a throw-away profile; discovery
   now prefers google-chrome → Playwright Chromium.
2. **AppArmor userns restriction** (Ubuntu 23.10+) makes Chrome's sandbox unusable; the MCP reports exactly that and
   needs `OZONE_BROWSER_NO_SANDBOX=1`. Run it only with a tight host allowlist.
3. **CDP clicks need `mouseMoved` first and `buttons` on press/release**, or they are silently dropped (applied).
4. Web-mode Ozone UI could not host any pipeline-delivered tab (fixed, §5). Web-mode execution without a paired
   device session is refused by the host — correct, and the Workspace tab then shows its empty state rather than data.
5. (Corrected: the registry persists.) The strict-session host 401s registration without `x-oz-internal` — the registrar
   reads the token from the usual places and says so when it cannot.

## 7. Open items

* Live Browser-Farm acceptance (install deps/PostgreSQL, run the farm, register a CC profile with
  `cc_script_ozone_bridge.py`, run `selftest` shapes against it).
* **Ozone UI for the farm**: a graph-native "Farm" lens (profiles as nodes, CC as hub, live status/metrics, screenshots
  as image-modality nodes, command history) — the natural home for "command it all from Ozone-Studio". Not built.
* Electron driving (CDP attach to the app) so the same MCP can verify the desktop build.
* Feed `browser_screenshot` paths into `visual_ingest` automatically; persist snapshots as text-modality graphs.
* DOMorpher compile campaign (§4) and operator decision on the two copies.
* Operator: start both MCPs from the operator shell (not an agent session); rebuild/deploy the host binary so the
  registrar works on the strict-session host; decide `OZONE_BROWSER_NO_SANDBOX` for this machine.
