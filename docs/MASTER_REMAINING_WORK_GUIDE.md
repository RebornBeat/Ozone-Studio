# Master remaining-work guide — full state as of 2026-10-29 (cc)

> Operator ask (2026-10-29): *capture everything left before any more edits — no builds, no tests; review all of
> what is left; tie the browser stack to the Personal Assistant and the Command Center; is the MCP index/registry
> guide findable; can Browser-Farm's account vault be extracted/isolated/extended into an Ozone vault (for
> accounts first, but notes/anything), and how could it be internal and secure; why does `.md` show as a math
> modality; where are the 3D indicators and more controls; view call graphs in the code modality; test the
> workspace environment with different modalities.* This document is the review result. **Nothing in it was
> built or tested this turn** — every claim is from reading code/data, and each is marked *verified-by-reading*,
> *verified-by-running (earlier)*, or *unverified*.

## 0. Corrections to what I said earlier (read first)

| Earlier claim (mine) | Truth after review |
|---|---|
| "The MCP registry is in-memory; re-run the registrar after every restart" (BROWSER_AUTOMATION_GUIDE §2/§6, NEW_MCP_GUIDE §5 still says it too) | **Wrong for the current code.** `McpRegistry::with_persistence` (`src/grpc/mod.rs:3090`) persists every register/unregister to `{data_dir}/mcp_tool_registry.json` and restores at boot (`TOOLS_PIPELINES_MCP_GUIDE §11.5`, laned 2026-09-28). Verified by reading the scratch host's data dir: 25 entries (24 browser/farm + p2p-bridge) were on disk. The repo's own `zsei_data/mcp_tool_registry.json` (1 entry, Sep 29) is just an old host's file. Re-registration is only needed when the *endpoint/port* changes. |
| "`browser-farm-mcp` proven (20/20)" | Proven **only against a mock I wrote**. Operator is right: a mock proves my reading of the API, not the farm. It must be replaced by a live run (§3.W1). The mock selftest stays only as a fast regression guard until then, then should be retired or clearly renamed. |
| "Ozone has no secret handling to extend" (implied by leaving vault out) | Review found real gaps (§3.W3): a **live OpenRouter key sits in plaintext in `config.toml`** (git-ignored, never committed — `git log -S` clean — but on disk; it also scrolled through this session's tool output, so rotating it is prudent). The Electron device private key is a plain `device-key.pem`. No symmetric-crypto crate in `Cargo.toml`. |

## 1. Ground truth (state of the world)

**Repo** `main`, HEAD `a346d38`, 3 commits ahead of `origin/main` (operator pushes). Only `tools/bridges` (nested
upstream repo, gitlink with local changes) is uncommitted — still an operator decision (vendor vs fork).

**Host/binary.** The local host is **not running**. `target/release/ozone-studio` is from Oct 28 09:24 and
**predates** the jurisdiction code (`aliases`, `rule_match_term`), `sync_jurisdiction_file`, the `x-oz-internal`
internal-token code, and zcode's I-Loop budget fix. None of that has been compiled into a deployed binary, so: new
jurisdiction rules load without alias matching, no manifest sync, strict-session routes (registration, `/task/*`,
`/context/mirror`) reject the shared-context MCP and my registrar. **One build + deploy is the gate for most of
what follows (§3.W0).** Constraints on this machine: 7.6 GB RAM (2.7 GB available at review time; two silent host
deaths under task-executor load → `OZONE_AUTO_EXECUTE_TASKS` held off), disk 18 GB free (93%; build rule ≥10 GB).
EU VPS (12 GB) runs the Oct 28 01:57 binary with all earlier fixes, all four loops, consciousness enabled.

**Built and committed (my side, this and the previous turns):** graph-native 3D workspace (one component, two
mounts), indicator rail (tasks/gate/peers/heartbeat), Workspaces-tab restyle, `browser-mcp` (11 tools),
`browser-farm-mcp` (13 tools), registrar, web-mode pipeline-UI fallback, tab-strip fix, guides, evidence screenshots.
**zcode's side** (from its FULL-STATUS): sensor ticks live, koi + valence acceptances closed, both hosts on the same
binary (pre-jurisdiction), tombstone pass II clean, I-Loop budget now window-proportional.

## 2. Answers to the operator's questions (from review)

### 2.1 "Did you find the guide on MCP index/registry possibilities?" — yes
`docs/TOOLS_PIPELINES_MCP_GUIDE.md`: **§11.1** every `/mcp/call` is an **AMT expansion** (rides GraphRipple into the
project's AMT, modality-tagged); **§11.2** tools/MCPs become **capability call sites in blueprint stage 4 and
simulation stage 7** (`BlueprintStep.capability_ids`, planner lists name+description+platform tags; stage 7 simulates
capability calls incl. *creating* tools/MCPs); **§11.3** the **edge-identification K-registry** (per-modality strategy
presets, parallel fan-out, single graph choke point); **§11.5** registry persistence; **§11.6** detection models as
parallel graph-expanding tools; **§12** security model; **§13** versioning/platform gaps (no version-constrained
dispatch, no native `platform` field, last-write-wins registration). Also `NEW_MCP_GUIDE` (Patterns A/B/C),
`MODALITY_EXPANSION_GUIDE` (the contract, per-modality localization, MCP roadmap), `TOOLS_REGISTRY` (now incl. §6–7).
**Consequence for the browser stack:** the capability index is *how the planner finds it*. Today stage 4 still coerces
steps to pipeline 9 (+ web-search carve-out); `capability_ids` (§11.2) is **not built** — until it is, no orchestrated
run can *select* `browser_*`; agents call them directly through `/mcp/call`. That is the single biggest wire for
"Ozone navigates for me and for itself".

### 2.2 DOMorpher is "indexable"
Yes — that is `NEW_MCP_GUIDE` **Pattern B** (download → graph → index → traverse): link `~/Projects/domorpher` (and
`~/Projects/Browser-Farm`) as projects, run the code modality over them, traverse the result in the 3D world (§2.6),
write the architecture into the guide stack, and derive its capability list from its own CLI/API instead of copying.
DOMorpher's seven components map onto existing Ozone pieces: DOM Preprocessor/semantic markdown ≈ `browser_snapshot`
(+ a future `dom` edge-identification strategy: elements/landmarks/forms as a graph so a page *is* a graph in the
text/structure family); Chunking Engine ≈ Ozone chunk graphs + context budget; LLM Integration ≈ the model router;
Instruction Parser/Adaptive Execution/Reconciliation ≈ AMT + blueprint/simulation + multi-pass review; Schema
Enforcement ≈ typed pipeline envelopes. So DOMorpher is best treated as (a) an indexed reference + (b) an optional
extraction engine behind an MCP once it compiles (§3.W4) — not as a second agent loop competing with Ozone's.

### 2.3 Personal Assistant × browser × Command Center
Directive #41: the Personal Assistant is **not an MCP and not a flat tool** — an internal consciousness-stack
pipeline (`src/consciousness/assistant.rs`, `/assistant/feed`), gated by the same `[consciousness] enabled` switch,
**proposals only, never reorders the operator's plan, no fabricated urgency, one batched digest not per-event
calls**. Therefore "the assistant navigates for me" must be modeled as: the assistant **proposes** a browser
capability call (as an Order item / blueprint step), the operator (or a standing per-project policy) approves, and
the *execution* is a normal `/mcp/call` to `browser_*` / `farm_*` — metered, gated, rippled. Never an assistant that
silently drives a browser. "Itself": the I-Loop/AMT loops may *propose* verification navigations (e.g. check a
deployed page) under the same rule. The **Command Center** is the Browser-Farm-side orchestrator profile; Ozone is the
commander *above* it (`farm_command`/`farm_result`); the CC profile never decides policy — allowlists, read-only
mode and confirmation live in Ozone (and in `browser-farm-mcp` env). Safe-by-construction today because **no proxies
and no real accounts are in use** (operator-confirmed): navigation tests target allowlisted hosts only.
Needed wiring (not built): `capability_ids` (§2.1), an "assistant proposes browser action" kind in the Order guide
(`UNIVERSAL_ORDER_GUIDE`), RequireConfirmation rules for: first visit to a new host, any form submit, any
`vault_use` (§W3), anything on a farm profile that has a proxy attached.

### 2.4 The account vault — can it be extracted, isolated, extended to Ozone, for anything, internal and secure?
**What Browser-Farm actually has (read from code):** accounts live in the Electron *client's* `electron-store`
(localStorage fallback) as **plaintext**; they are sent in the profile-start request, held in the server's
`accounts_cache` (memory) and injected as a plain dict into scripts via `get_account()`. The PostgreSQL
`accounts`/`proxies` tables store passwords in **plaintext** (`# Ideally encrypted at rest`). What *is* valuable and
should be kept: the **proxy-history compliance rule** ("1 account per website per proxy", unique constraint) and the
"fetch by id inside the script" ergonomics. So: the *idea* extracts well; the *implementation* must not be copied.

**Yes, a general Ozone vault is feasible and fits the architecture. Proposed design (not built):**
* **Typed items, secret fields flagged:** `credential` (platform, url, username, **password**, email, phone, TOTP
  seed), `api_key`, `oauth_token`, `note` (secure note), `ssh_key`, `file` (encrypted blob), `other`. Metadata
  (label, kind, tags, scope global/workspace/project, owner user id, created/rotated/last-used, policy) is plain;
  only fields flagged secret are encrypted. "A vault for anything, mainly accounts" = same engine, different `kind`.
* **Isolation rule (the core of "secure"):** **secrets never enter the graph, capture stores, logs, `/mcp/call`
  envelopes, or any LLM prompt.** The graph may hold *handles* (ZSEI `VaultItem` container: label/kind/tags/policy/
  last_used — so the 3D world can show a Vault hub) but never ciphertext or plaintext. Ciphertext lives in a
  dedicated store `{data_dir}/vault/` outside ZSEI/AMT/P2P-broadcast.
* **Crypto:** per-item data key (XChaCha20-Poly1305) wrapped by a key-encryption key derived with Argon2id from an
  operator passphrase, optionally combined with a device-bound secret (derived from the existing Ed25519 device key
  / Electron `safeStorage` on desktop; headless VPS: passphrase via systemd credential or 0600 key file). Master
  key lives in memory only inside a **vault session** with TTL; zeroized on lock. Needs new crates
  (`chacha20poly1305`, `argon2`, `zeroize`) — none are in `Cargo.toml` today. Recovery key printed once at creation;
  no recovery without it (stated plainly).
* **Use by reference, not by reading:** agents and models see `vault://credential/<id>` handles only. A **vault
  broker** route (`/vault/lease`) grants a **short-lived, single-purpose lease** to a *named MCP/tool* after policy
  checks (agent role, tool allowlist, per-item use limits, optional **human confirmation through the existing
  RequireConfirmation/decision-review machinery**). The MCP resolves the secret *inside its own process* and
  fills the field (`browser_type {vault_ref}`; Browser-Farm scripts call a lease instead of reading a plaintext dict).
  Snapshots already omit password values; screenshots of filled forms and `browser_eval` are the leak paths to close
  (mask on screenshot, refuse eval when a lease is active).
* **Adding/revealing is human-only:** create/edit/reveal/export happen in the UI or CLI under a vault session, never
  through an agent-callable tool. Agent-callable surface = `vault_list` (metadata only) and `vault_request_use`
  (asks; the human/policy answers).
* **Audit:** every lease/use writes who/what/why/trace-id/outcome (no value) into a capture store and the ripple.
* **Multi-user, "others safely":** items are namespaced by the *validated* session user id (the Ed25519 identity the
  host already issues); cross-user sharing is an explicit per-item re-wrap to the peer's public key, never via
  auto-broadcast sync. P2P `auto_broadcast_sync` must exclude the vault store by construction.
* **Browser-Farm integration:** the farm stops holding passwords; profile start passes `account_refs`, the script
  asks the broker for a lease (farm gets a Python client). The farm-local proxy-history table stays farm policy.
* **First migrations it would absorb:** `config.toml` API keys (→ handles + env/vault resolution), gcal/zoom OAuth
  tokens, the MCP shared secrets (`OZONE_TERMINAL_TOKEN`, `OZONE_BROWSER_TOKEN`, farm API keys).
* **Honest threat model:** protects at rest, from the graph/LLM/logs, and from over-broad agents; does **not**
  protect against malware running as the same user with debugger access to the host process, nor against a
  compromised unlocked session. Say that in the UI.
Phased build in §3.W3.

### 2.5 Why does `.md` show as "math"? — not reproduced; four real defects found that explain "wrong modality"
Verified by reading: at link time `file_link` (`assets/pipelines/general/file_link/main.rs::detect_modality`)
classifies `.md` as **Text** (live data: `README.md → Text`, `analyzed:false`, no graph container), and
`graphs.rs`/`types/modality.rs` also say text. So the label "math" is not coming from link-time classification. The
candidates, in likelihood order — **all to be confirmed by the §4 test script, not assumed**:
1. **`MathViewer` ignores modality.** It renders the *selected file's raw content* through a LaTeX/ASCII converter
   whatever the file is; `CodeViewer` has a `classify()` that redirects text/math, `MathViewer` has none. Selecting
   `README.md` and opening **Files → Math** shows an `.md` "as math". Likely the observed symptom.
2. **Per-file modality is read from `context.keywords[1]`** (`ui/src/data/workspaceData.ts`) — a positional contract on
   an array that any keyword-extraction change can reorder; `metadata.modality` is always the literal "Unknown".
3. **Four extension→modality tables that disagree** (a systemic defect, certain): `src/types/modality.rs`
   (`tex`,`pdf`,`doc` → Text), `src/orchestrator/graphs.rs` (`tex/latex/nb` → math; json/yaml/xml/toml → code; svg → image), `file_link::detect_modality` (adds non-modality buckets **Data, Config, Web,
   Document, Archive**; `.m`, `.ipynb`, `.rmd`, `.qmd` → Math; `ts` listed under both Code and Video — Code wins, so `.ts` video is unreachable; `.m` is also Objective-C),
   `assets/pipelines/modalities/mod.rs` (Text includes csv/json/xml/html).
4. **`pdf/doc/docx/xls/ppt` → "Document", which has no pipeline** (`modality_pipeline_id` returns `None`), so **PDFs
   are never analyzed** — directly blocks the jurisdiction law-book plan (§3.W8). `Data/Config/Web/Archive` likewise
   get a file node but no graph.
Fix direction (after test): one shared extension table (Rust crate/JSON in `assets/`) consumed by host, `file_link`,
pipelines and UI; add a real **Document→Text (with PDF text extraction)** route; make `MathViewer` modality-aware.

### 2.6 Call graphs in the code modality
**What exists:** the code pipeline emits `Calls` edges (function→function) — **same-file resolution only**, regex-based
(`extract_function_calls` matches only callees present in that file's own function list), plus `Contains`/`Imports`/
`Extends`/`Implements`; `Standard` depth now includes calls (Surface skips). `tools/code-graph-mcp` does a
**cross-file static call graph for Python only** (conservative: unique callees only; unresolved reported, never
guessed; port 3279). UI: `codeEdges.ts` already styles `Calls` (orange dashed, arrow). **Gaps:** no cross-file call
resolution for Rust/JS/TS (so DOMorpher, 146k lines of Rust, would show almost no calls); no call-graph *lens* in the
3D world; the 3D layout is modality-cluster-by-golden-angle, not layered caller→callee; edges are not drawn with
direction; no depth-limited expansion, entry-point rooting, or path-finding. Design in §3.W5.

### 2.7 3D world — "where are the indicators, more controls"
**What exists now:** in the **header rail** (outside the world): tasks (running/queued/failed + recent list), gate
(pending reviews / last decision), peers, graph heartbeat. In the world: node pulses (real construction events),
selection ring, hub shells, edge classes (real/derived/inferred/cross-modal), legend (click = solo modality),
controls (reset, focus, labels, auto-rotate, fullscreen, dock), find-in-graph, lens bar. **What's missing (the
operator's point):** indicators *as objects in the space* — task nodes orbiting their project hub with `StepPips`
rings, gate "doors" on stage edges, capture-presence (S10–S13) halos per view, loop heartbeats at hubs, peer/vault
presence — plus richer controls: layout mode (cluster / layered / force / radial-by-depth), edge-type + edge-class
toggles, node filters (type, size, age), depth/hops slider, path-finding A→B, saved views/deep links, time scrubber,
minimap (GraphCanvas 2D), screenshot/export, camera presets per lens, keyboard help overlay, performance mode
(LOD) and honest node-cap controls. Several need substrate data that does not exist yet (task rows in the graph —
`GKTaskRef` is typed but unpopulated; capture-presence needs a per-view S10–S13 mapping). Order in §3.W5.

## 3. Workstreams — everything left, in dependency order

Legend: **[G]** operator gate/decision, **[B]** needs a build, **[V]** needs live verification.

### W0 — Deploy gate [B][G]
Build once (check `free -h`, no concurrent cargo; disk ≥10 GB) → deploy to local and EU → restart with
`OZONE_JURISDICTION_FORCE_SYNC=1` once (first manifest sync) → restart the shared-context MCP → run
`tools/sync_jurisdiction_content.py` dry-run review → verify strict-session registration via the internal token.
Unblocks: jurisdiction aliases/sync, `x-oz-internal` for MCPs, registrar on the strict host, I-Loop budget.
*Acceptance:* alias rule matches in a gate probe; `register-browser-tools.mjs` succeeds on the real host; the
shared-context MCP posts `/context/mirror` without 401.

### W1 — Browser stack live (no more mocks) [V][G]
1. **Start `browser-mcp` from the operator shell** (agent sessions reap children): allowlist `localhost` (+ chosen
   hosts), `OZONE_BROWSER_NO_SANDBOX=1` (this box's AppArmor), `ALLOW_GLOBALS=OZONE_HOST_URL`; register; re-run
   `selftest` through the real host.
2. **Live Browser-Farm, locally first.** Needs: Python venv + `pip install -e server` (PyPI reachable), Playwright
   (Chromium already cached), Xvfb (installed), **PostgreSQL — not installed and no sudo/docker here**. User-space
   options: a pip-bundled Postgres (e.g. `pgserver`), or an operator-run `sudo apt install postgresql`. Memory:
   farm + Postgres + Chrome on a 2.7 GB-available box is tight → single profile, no recording, `memory_threshold_mb`
   low. If local is too heavy: **EU VPS** (12 GB) — requires the operator's SSH access/consent for this host; prefer an
   SSH tunnel and never expose the farm port publicly (the farm's default bind is `0.0.0.0`; API key is the only
   gate — bind loopback and tunnel).
3. Create a CC profile + one worker (no proxy), run `cc_script_ozone_bridge.py`, then drive: `farm_status` →
   `farm_profile_start` → `farm_navigate` (allowlisted) → `farm_screenshot` → `farm_command ping` → `farm_result`.
   Record real outputs + screenshots in `docs/evidence/`. Then **delete or demote the mock selftest**.
4. Note Browser-Farm's own repo has 15 uncommitted changes (server input/context/script/xvfb, client cursor
   overlay; `GhostCursor.js` and `useMouseBatcher.js` deleted while the README still advertises mouse coalescing) —
   confirm what is intended before running it; do not commit in that repo from here.
5. Decide: keep `browser-farm-mcp` as a bridge (current) vs vendoring Browser-Farm under `tools/` like `bridges`
   (license: MIT, own repo — a bridge is cleaner).

### W2 — Capability wiring: assistant/planner/CC [B][G]
`BlueprintStep.capability_ids` + stage-4 capability listing + stage-7 capability simulation (guide §11.2); capture each
capability call as a registered call site (S11-family + usage ledger); Order item kind "browser action proposal";
RequireConfirmation rules (new host, form submit, vault use, proxied profile); per-project browser allowlist stored
as a project setting (graph-visible); assistant digest includes "proposed navigations awaiting you". Acceptance: a
planned task selects `browser_snapshot`→`browser_click` on an allowlisted fixture, the run shows in Order, the
operator approves, the call appears in the usage ledger and the project's AMT expansion.

### W3 — Ozone Vault [B][G] (design in §2.4)
Phases: **P0** decisions (passphrase vs device-bound, desktop `safeStorage` vs headless, recovery policy, where the
store lives); **P1** crate + CLI (`ozone-vault`: items, Argon2id/XChaCha20, lock/unlock, tests incl. tamper/rotation);
**P2** host routes (`/vault/*`, session-user namespacing, audit, **redaction guard in capture/ledger/ripple**, exclude
from P2P sync); **P3** broker leases + RequireConfirmation integration; **P4** UI Vault panel/graph-native hub (metadata
only, human-only reveal) ; **P5** consumers: `browser_type {vault_ref}`, farm lease client, `config.toml` key
migration, MCP shared secrets; **P6** security review/red-team (leak paths: screenshot, eval, logs, crash dumps,
swap). **Immediate hygiene independent of the vault:** rotate the OpenRouter key (it appeared in session output),
keep `config.toml` out of any copy/scratch dir (I deleted my scratch copy), `chmod 600` the device key and config.

### W4 — DOMorpher [G]
(1) Operator picks the lowercase copy as canonical and archives the upper one (contents are a strict subset).
(2) **Index it** (Pattern B): link as a project, run code analysis, read the architecture graph (needs W5/W6 for
useful call graphs). (3) Compile campaign, staged: fix 27 lexer errors (`agent/action.rs` raw strings), declare deps,
compile **core** only (`dom`, `chunking`, `llm`, `extraction`, `schema`, `reconciliation`) behind features; defer
enterprise/extension/monitoring/distributed/visual. (4) Expose as a JSON CLI + MCP (`dom_to_markdown`, `extract`) —
only where it beats `browser_snapshot` + existing pipelines. (5) Do not assume its Python API exists on PyPI.
*Acceptance:* `cargo check` clean on the core feature set; one extraction run reproduced against a fixture page.

### W5 — 3D native world: indicators, controls, call graphs [B]
Order by value: (a) **populate task substrate** (task rows → `GKTaskRef`) so tasks exist as world objects; (b) in-world
indicators (task orbit + StepPips, gate doors, capture-presence, loop heartbeats, vault/peer presence); (c) **call-graph
lens**: filter `Calls`/`Imports`, directed edges with arrowheads, **layered caller→callee layout** rooted at entry
points, depth slider, expand/collapse a function's neighbours, path-find A→B, highlight blast radius of the selected
function; (d) layout modes + edge/node filters + hops slider; (e) saved views, deep links, time scrubber, minimap,
export; (f) keyboard help; (g) LOD/perf mode + real-GPU measurement at 2400+ nodes; (h) Electron verification;
(i) port the 9 dedicated engine bodies into lens renders. Prereq for useful call graphs: cross-file call resolution
for Rust/JS/TS (extend the code pipeline or add a `code-graph-mcp` language worker; Python already cross-file there).

### W6 — Modality classification truth [B]
Single extension table (data file in `assets/`, loaded by host/file_link/pipelines/UI); remove positional
`keywords[1]` reliance (store per-file modality as a first-class field); Document→Text route with **PDF/DOCX text
extraction** (so law-book PDFs are ingestible); decide what Data/Config/Web/Archive are (new modalities or
subtypes mapped to code/text); make `MathViewer` modality-aware; add regression tests for the whole table
(including the `ts` Code/Video collision and `.m`/`.v`/`.d` ambiguity).

### W7 — Workspace environment test plan [V] (captured now; run after W0/W1, using `browser-mcp`)
Run order and what each proves — **no edits before running; record failures as findings first**:
1. Fresh data dir host (first-run exemption) + UI via `ui-harness.mjs`; create workspace/project (needs a session —
   pair a device or run with the internal token in the harness).
2. Link one real file per modality: `.md`, `.txt`, `.pdf`, `.tex`, `.rs`, `.py`, `.json`, `.toml`, `.png`, `.mp3`,
   `.mp4`, `.glb`, `.csv`. For each record: link-time modality, `analyzed`, graph container id, what the 3D world draws,
   what Files→{Code,Math,Editor} show. **This reproduces or refutes the `.md → math` report (§2.5).**
3. Link `~/Projects/Browser-Farm` and `~/Projects/domorpher` folders; analyze; view the code graph and call graph
   (§2.6); record node/edge counts, cross-file call coverage, 3D frame rate.
4. Exercise lenses (Graph/Files/Fabric/Hierarchy/Engines) and the indicator rail with live tasks (needs the task
   executor or `/task/create`).
5. Capture screenshots via `browser_screenshot` into `docs/evidence/`.
Pass criteria are per-step observations, not "no errors".

### W8 — Jurisdiction polish [B][G] (queued behind W0/W6)
Per-country constitution/legal-code **book (PDF)** → text modality → rules sourced to ONE authoritative book, not
blogs; fetch-verify the search-summary-only rules first (DO, EU, US, live regions); fill ~60 thin scopes; elevations
remain the operator's per-rule act (`ELEVATION_CANDIDATES.md`); keep `OZONE_JURISDICTION_FORCE_SYNC` semantics
(`.bak`). **Hard dependency:** W6's PDF extraction route.

### W9 — Other open items (collected)
Chat modify/retry/branch **buttons** (types exist, panel doesn't render them); CapturePresence indicator;
`tools/bridges` decision; EU: security-mcp boot self-registration so sensor ticks run there; data mutations
(FileRef backfill, dedupe passes, worktrees) — operator-gated; push the 3 local commits; `/orchestrate` has no session
validation and `/mcp/call` trusts the self-declared `agent` (guide §12 honest gaps — close before any exposure);
host→tool timeout 30 s and no header forwarding (so `OZONE_*_TOKEN` only protects direct callers); `capability`
platform/version fields still tags only; amt_reexpansion failure rate (65%, free-tier drops) is an ops concern, not code.

## 4. Decisions needed from the operator
1. **W0:** go for one build+deploy? (and EU restart with `OZONE_JURISDICTION_FORCE_SYNC=1`)
2. **W1:** where to run the live farm — local (user-space Postgres, tight RAM) or **EU VPS** (needs your SSH go-ahead)?
3. **W3:** vault unlock model (passphrase / device-bound / both) and whether the vault may ever sync to paired peers.
4. **W4:** canonical DOMorpher copy; authorize the compile campaign.
5. **W2/W3:** default confirmation policy for new hosts, form submits, vault use.
6. Rotate the OpenRouter key now? (recommended)
7. `tools/bridges`: vendor vs fork.
