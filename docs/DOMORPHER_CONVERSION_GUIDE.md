# DOMorpher → Ozone-Studio conversion guide (2026-10-29, operator directive)

> "This was PRE our Ozone-Studio connection — we are totally rebuilding this
> as an Ozone-Studio tool with MCP use, reasoning it through Ozone-Studio."
> The compile campaign continues ONLY as review-enablement (make the source
> readable enough to harvest); the DELIVERABLE is the conversion below, not
> a standalone domorpher binary.

## 0. Provenance + license boundary (Pattern B step 1 — OPEN ITEM)

- `~/Projects/domorpher` (lowercase, 125+ .rs files, ~146k lines) is the
  complete copy; `~/Projects/DOMorpher/domorpher-rust` holds a PARTIAL
  original (no agent/ dir — its action.rs exists nowhere else, and BOTH
  known copies of several files were truncated mid-function: action.rs,
  perception.rs, reasoning.rs all ended mid-expression; `api/rpc` and
  `chunking/utils` modules are declared but their files were never
  captured).
- **NO LICENSE FILE exists in either copy.** Before any absorb-into-tree
  step (Pattern B), the operator confirms provenance (own code? fetched
  from where, under what terms?). Until then domorpher stays OUTSIDE the
  Ozone tree (it is: `~/Projects/`, sibling to Ozone-Studio).

## 1. What the compile campaign actually was (and now is)

Started as "make it build" (operator greenlight). Landed so far, all
verified by cargo check iterations (every fix proven by the error moving
past it):
1. Cargo.toml `[dependencies]` restored from EMPTY — ~50 crates derived
   from the use-scan; crate-name corrections (actix-web, async-trait,
   crossbeam-channel, ego-tree, lazy_static, once_cell, derive_builder,
   tiktoken-rs are the real crates.io spellings); playwright version
   found live via `cargo search` (0.0.20) then REMOVED — its build.rs
   downloads a driver binary that fails here; the driver's call sites
   (mod.rs, headless.rs, agent.rs) now route Chromium/Firefox/Webkit to
   the self-contained CDP PuppeteerDriver (firefox/webkit fail honestly
   at launch if no local binary).
2. 29+ broken raw-string literals fixed (`r"...\"..."` — the `\"` is
   literal backslash-quote in a raw string, so the first `"` terminated
   the literal; rewritten `r#"..."#` with plain-quote regex classes).
3. Four files were TRUNCATED MID-FUNCTION by whatever captured them —
   action.rs (ended at `.with_url(` inside execute_switch_frame),
   perception.rs (ended mid `level: if in_nav { 1 } else`),
   reasoning.rs (ended mid `match config.fallback_strategy.default_action
   .as_str`) — each completed from its own sibling-function patterns
   using only constructors verified to exist (ActionTarget::css,
   ReasoningResult::fallback, ActionType::Scroll/Back);
   NavigationLink/NavigationStructure types (referenced only by the
   truncated tail) defined at their point of use. semantic_markdown.rs
   is the fourth (unclosed delimiter at 2454 — NOT yet completed).
4. Two modules declared but never captured: `api/rpc` (mod.rs:16) and
   `chunking/utils` (mod.rs:34) — E0583 file-not-found. Open.
5. Known latent wave BEHIND the parse layer (name resolution): ~11 call
   sites use `ActionTarget::Selector(...)` / `ActionTarget::None`
   against a STRUCT (no such associated items) — will surface when
   parsing completes; mechanical fix = associated constructors + sed.

**Campaign status: parse layer nearly clean (3 items open: 2 missing
modules, 1 truncated file). The name-resolution + type waves are queued
and BOUNDED by the new goal — we go only as deep as harvesting needs.**

## 2. The seam — library/crate vs tool vs MCP vs Ozone-Studio (THE identification)

**The diagnosis (operator, 2026-10-29): domorpher does not separate these
correctly today.** One binary mixes four layers: the pure capability logic
sits entangled with model clients (llm/anthropic, llm/local), a web server
(api/: actix/graphql/rest/websocket), persistence (storage/), monitoring
(monitoring/), enterprise surfaces (enterprise/), and its own browser
drivers (browser/puppeteer, headless). None of that infrastructure should
exist in a capability library — Ozone-Studio already owns every piece of
it, better hardened. The rebuild is the CUT at the correct seam:

```
┌───────────────────────────────────────────────────────────────┐
│ OZONE-STUDIO (owns ALL infrastructure — none of it re-ported) │
│  model calls (pipeline 9 + fallback) · storage/graph ·        │
│  monitoring · task system · API host · metering/gating        │
└───────────────▲───────────────────────────────▲───────────────┘
                │ injected impls                │ /mcp/call
        ┌───────┴────────┐              ┌───────┴────────┐
        │ LAYER 1        │              │ LAYER 3        │
        │ domorpher-core │◄─────────────│ THE MCP        │
        │ (LIBRARY CRATE)│  in-process  │ (enforcement)  │
        │ pure capability│  calls       │ loopback /call │
│ no llm, no http,│              │ allowlists,    │
│ no storage, no  │              │ caps, locks,   │
│ server — only   │              │ token, tags    │
│ trait seams     │              └───────▲────────┘
└───────▲─────────┘                      │ schema + handler
        │ calls                          │
┌───────┴────────────────────────────────┴───────┐
│ LAYER 2 — THE TOOL SURFACE (1 verb = 1 tool,   │
│ JSON schema, lives in the MCP process)         │
└────────────────────────────────────────────────┘
```

### 2.0a Layer 1 — `domorpher-core` (the library crate)

Pure capability, compiles WITHOUT any infrastructure. Cargo deps: serde,
regex, chrono, thiserror-class errors — nothing that opens a socket,
spawns a server, or talks to a model.

**IN the crate** (harvest list, by module):
- `dom/` (preprocessor, semantic_markdown) — DOM → semantic snapshot.
- `agent/action.rs` types (ActionType/ActionTarget/ActionResult +
  builders) — the command vocabulary as plain data.
- `agent/perception.rs` logic — element extraction, navigation structure.
- `agent/reasoning.rs` STRATEGIES as pure functions: `(perception,
  instruction, config, decision_source) → ActionDecision` — the regex
  fallback strategies stay deterministic in-crate; MODEL judgment arrives
  only through the injected trait below.
- `agent/goals.rs`, `agent/memory.rs` as data + pure ops.
- `forms/`, `extraction/`, `instruction/` (NL instruction → action
  sequence, pure parse), `interactive/detector.rs`, `execution/`
  multi_pass + recovery LOGIC, `browser/session.rs + cookies.rs +
  auth.rs` as state transforms, `chunking/`, `schema/`,
  `reconciliation/`.

**The trait seams** (defined BY the crate, implemented BY Ozone/the MCP
host — the actual "point where" the library ends):
- `DecisionSource` — model judgment. Ozone impl: pipeline 9 + fallback
  chain. (Replaces llm/anthropic + llm/local entirely.)
- `PageDriver` — launch/navigate/click/type/screenshot/eval. Ozone impl:
  browser-mcp's hardened CDP layer, or the farm's. (Replaces
  browser/puppeteer + headless + javascript.)
- `StateStore` — session/cookie/profile persistence. Ozone impl: farm
  profiles / data_dir. (Replaces storage/.)

**OUT of the crate entirely** (Ozone already owns; never ported): `llm/`,
`api/`, `enterprise/`, `monitoring/`, `storage/`, `pipeline/`,
`parallel/`, browser driver internals, `extension/` (unless in-page
assistance is later wanted), `main.rs`/`lib.rs` wiring.

**Compile-campaign scope, restated by the seam**: the campaign no longer
aims at "all 146k lines build standalone" — it aims at **the IN-list
building as `domorpher-core`**. The OUT-list is not carried. The missing
modules resolve by seam: `api/rpc` = OUT (drop the mod line);
`chunking/utils` = IN-list support (stub or recreate from use-sites);
`semantic_markdown.rs` tail = IN (complete it).

### 2.0b Layer 2 — the tool surface

One verb = one tool with a JSON schema, a thin handler that calls
`domorpher-core` in-process. The full table is §2.2 below. No logic lives
here — a tool handler is: validate input → call library → wrap result
(untrusted labels for page-derived data) → answer the standardized shape.

### 2.0c Layer 3 — the MCP (enforcement point)

`tools/agent-mcp/` following the terminal-mcp hardening pattern exactly:
loopback-only `/call` accepting `{tool, agent, input, context}`;
allowlists REQUIRED (unconfigured = LOCKED); per-tool locks
(`agent_exec_script` off without OZONE_AGENT_ALLOW_EVAL=1); navigation
host allowlist; timeout/output caps; optional shared secret; platform +
version tags on registration. The MCP is the only thing that holds a
PageDriver and runs the library — nothing calls the library around the
door.

### 2.0d Layer 4 — Ozone-Studio

Everything above the MCP: orchestration, model calls, jurisdiction gate,
metering ledger, graph capture/ripple, task Order, the proposals chain
(any autonomous multi-step agent run is an action-proposal, approval-gated),
the UI. Reasoning it through Ozone-Studio means: domorpher's agent loop
never self-directs at this layer — it is one of the actors, and its acts
are gated calls like everything else's.

### 2.1 Tier mapping of the ORIGINAL tree (what each module was)

## 3. Build order (dependency-first, seam-first)

1. **License/provenance confirmation** (operator; blocks absorb-into-tree,
   NOT reading/harvest work).
2. **Crate extraction**: create `domorpher-core` (or `~/Projects/domorpher
   /core/`) with ONLY the IN-list modules + the three trait seams; the
   compile campaign now targets THIS crate's `cargo check` (each fix
   proven by the error moving). OUT-list modules are simply not included.
3. **Seam impls** (in the MCP host process): DecisionSource over pipeline
   9 + fallback; PageDriver over browser-mcp's CDP layer; StateStore over
   farm profiles/data_dir.
4. **tools/agent-mcp/** (Pattern A): server skeleton → the 16-tool table
   → selftest WITH refusal paths → registrar → one positive + one refusal
   through `/mcp/call`.
5. **Harvest implementations** land incrementally behind each tool
   (session/cookie aging first — the farm wants it; semantic DOM second;
   multi-pass/recovery with the farm runner).
6. **Retirements** recorded (llm/, api/, enterprise/, monitoring/,
   storage/, pipeline/, parallel/, puppeteer, extension) — nothing is
   deleted from ~/Projects/domorpher (it stays the reference copy).

## 4. Acceptance rows (wired into MASTER_TEST_GUIDE)

- **AG1**: agent-mcp selftest N/N including a refusal per dangerous
  class; registration carries platform + version tags.
- **AG2**: `agent_navigate` to a non-allowlisted host refused (live
  precedent: farm_navigate's lockout).
- **AG3**: `agent_exec_script` refused while OZONE_AGENT_ALLOW_EVAL unset.
- **AG4**: session/cookie aging: a profile restarted keeps its cookies
  with correct relative ages (farm profile persistence).
- **AG5**: agent_extract output lands as an untrusted-labeled tool result
  (X2 rule — never an instruction).
- **AG6**: domorpher-core builds WITHOUT the OUT-list (its Cargo tree
  contains no http-server, llm-client, or storage-engine crates) — the
  seam is enforced by the dependency graph itself, not by discipline.
