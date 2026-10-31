# Review — Ozone-Studio state, tool/MCP guide conformance, and the google.com DOMorpher test (2026-10-31)

Review and capture only. No source edits, builds or tests run. Facts below
were read live from the local host (:50051, uptime ~15.5 h, healthy), the EU
VPS (ozone-studio, browser-farm, ozone-farm-mcp, ozone-security-mcp all
`active`; 11 GB RAM, 9.4 GB available), the repos, and the guides.

Operator direction applied: `domorpher-agent` is UNUSED for now. The
Browser-Farm is driven through the Command Center (CC). Ozone-Studio and
consciousness handle decisions, internal or external or both. The farm tools
are the execution door today.

## 1. Taxonomy check (docs/TOOLS_PIPELINES_MCP_GUIDE.md §1, NEW_MCP_GUIDE §0-6)

The guide has exactly three tiers: pipeline (internal), tool (external, via
`/mcp/call`), bridge (long-lived external). It has no "seam" tier.
`domorpher-core` is none of the three: it is a compiled library that a host
(the farm) loads. So:

- **Do we need a file named `seams.rs`? No.** Verified: nothing in the core or
  the reference tree uses `PageDriver`, `StateStore` or `SeamError`. The only
  mentions are three comments. The traits are speculative. `PageCommand` and
  `PageResult` are useful as plain data types; keep them (rename the file to
  `types.rs`/`command.rs` or fold into the action module that already has
  `ActionType`/`ActionResult`) and delete both traits. Persistence is the
  farm's profile feature, so a `StateStore` trait in the library is not needed.
  The farm executes; the library describes and transforms.
- "Seam" survives only as a doc word for "where the library ends". It should
  not be a code concept. This also closes the review's M5: with the agent
  unused, the farm tools are the single enforcement edge.

## 2. NEW_MCP_GUIDE §6 checklist, applied to what ZCode built

| item | browser-farm-mcp (:3281) | domorpher-agent (:3283) |
|---|---|---|
| security model before execution | yes (nav allowlist, locked default) | yes (skeleton) |
| locked by default | yes (verified) | yes |
| refusal path tested | yes (selftest + live) | selftest refusals green (not re-run by me) |
| platform/version tags | registered | registered |
| registered with live-derived list | **EU 13/13 on 3281 now** (earlier 3252 drift is fixed: EU shows 13 on 3281) | **local registry holds 32 tools: the old `agent_*` x16 AND the new `domorpher_*` x16** (rename never removed the old rows) |
| called through `/mcp/call` only | yes | no calls (unused) |
| ripple + capture verified | yes on EU (screenshots, ingest) | none, unused |
| persistent launch story | local launcher script; EU systemd | local launcher only; **not on EU** |
| ZSEI contract container if new doctrine | not recorded | not recorded |

Verdict: the farm path follows the guide. The agent is a declared skeleton
that is registered but unused, which adds 16 stale and 16 live tool rows to
the planner's surface for nothing.

### Drift found

1. **Stale `agent_*` tools (16) remain registered locally** next to
   `domorpher_*`. Remove them (re-register in replace mode or delete the
   persisted rows in `zsei_data/mcp_tool_registry.json`, with a backup).
2. While the agent is unused, do not register it at all, or register it
   read-only/locked. Fewer tools means a cleaner planner and cleaner
   consciousness windows.
3. `MASTER_TEST_GUIDE.md` §7c rows AG2/AG4/AG7 still say `agent_navigate` and
   `OZONE_AGENT_BACKEND`; the code uses `domorpher_*` and
   `OZONE_DOMORPHER_BACKEND`.
4. Nothing in the library, the guide, or the parity manifest records that the
   model calls are removed (see REVIEW_DOMORPHER_LIBRARY_BOUNDARY_2026-10-30).
5. EU has no `domorpher-agent`, which is fine while unused. It must not be
   deployed before it is wired (a registered-but-dead tool is worse than none).

## 3. Ozone-Studio first — what it needs for the farm test (the picture)

The chain under test (all existing doors, nothing new):

```
CC command ─► /mcp/call (strict gate, jurisdiction, ledger, ripple)
          ─► farm_* tools (browser-farm-mcp :3281, nav allowlist)
          ─► Browser-Farm (EU): command-center profile, real Chromium/Xvfb
          ─► screenshot / page state back as UNTRUSTED data
          ─► /capture/ingest_image ─► entry gate ─► pipeline 102 (image) ─► graph
          ─► consciousness windows observe: S10 review rows, S13 tool_calls,
             assistant feed (notification / awaiting clarification)
```

What is verified live already: farm real on EU; `farm_command` ping->pong in
0.4 s; google.com navigation and a 199 KB screenshot through the farm
(`docs/evidence/browser-farm-eu/google-com-through-farm.png`); ingest route
live on EU end-to-end with real image analysis.

What Ozone still needs before the test is a full pass (ranked):

| # | Need | Why it matters to this test | Owner |
|---|---|---|---|
| 1 | Ingest hooks send `x-oz-internal` (env `OZONE_*_INGEST_TOKEN`) | today the hook gets a 401 with an empty body, so screenshots reach the graph only by a manual call | zcode (queued) |
| 2 | `/mcp/call` records `context.workspace_id/project_id` | farm/browser activity is stored with global scope and cannot be attributed to a workspace | next edit window |
| 3 | An actor for failed farm/browser/android/terminal calls | a failed farm action currently produces no finding, so consciousness has nothing to propose or ask about | next edit window |
| 4 | `graph` block from farm/page results | page structure never becomes graph nodes; the 3D world cannot show a web page | next edit window |
| 5 | Proposals: cooldown, idempotent Proceed, notification kind for reminders, UI Inbox | "notify and await clarification" is the common case; with no inbox the human never sees it | REVIEW_PROPOSALS R1-R4 |
| 6 | Stale-tool cleanup (section 2) | planner and windows are noisy | quick |
| 7 | systemd watchdog for the local host | it was stopped externally several times | quick |
| 8 | Secrets hygiene: OpenRouter key sits in git-ignored config.toml | rotate | operator |

## 4. The test — CORRECTED by the operator: it is the google.com DOMORPHER test, not a screenshot test

My first draft (T-FARM-CC) tested only navigate + screenshot. That is the
farm's own capability and is already proven (google.com navigation + 199 KB
screenshot, `docs/evidence/browser-farm-eu/google-com-through-farm.png`). The
test that was defined is: **prove domorpher works on google.com** — the page's
DOM becomes a DOM-traversable field that the library then acts on, through
the CC / farm / Ozone chain.

**Honest status, read live from the EU farm (read-only `farm_state_get`):**
`dom-field/google` holds only a label
`{by:"zcode+CC", captured_at:"2026-10-30", note:"DOM traversable-field capture
for domorpher-core harvest", url:"https://www.google.com"}`. No DOM, no field,
no extracted element exists. So the domorpher-on-google test has NOT run; the
marker only reserves the place. Also `domorpher-core` does not compile yet
(last reported 145 errors) and the guide's statement "Browser-Farm loads the
crate" has no mechanism: the farm is Python (FastAPI/Playwright), the library
is Rust. A decision is needed (section 4b).

### T-DOM-GOOGLE (stages; each has a pass bar and a current expectation)

| stage | action | pass bar | today |
|---|---|---|---|
| D0 | library gate: `cargo check` on domorpher-core, `cargo tree` shows no http/llm/storage crate (AG6) | 0 errors; no reqwest/tiktoken/jsonwebtoken | blocked: model-call removal edits just landed uncompiled; census unknown |
| D1 | capture real DOM: farm `navigate` google.com on the running CC profile; CC pulls the rendered HTML/DOM (not only a PNG) into `dom-field/google` (or a file under the host data dir) | stored HTML is the real page, hash recorded, size > 0 | not done: marker only |
| D2 | OFFLINE field build: run domorpher-core on the D1 HTML (no browser, no model) | traversable field with the search input, the Search / I'm Feeling Lucky buttons, language and footer links; each element has role, text, stable selector, bounding info if supplied | not done |
| D3 | field -> Ozone graph: field nodes/edges ingested through the `graph` block path | containers visible under the workspace; ripple event emitted | not done (graph block hook exists; farm result has none) |
| D4 | ACT: Ozone (not the library) supplies the decision as data: "type this query"; library produces the human-like action sequence (typing cadence, pauses); farm executes via CC channel on the CC profile | query text appears in the search box; screenshot proves it; allowlist still holds | not done; typing sequences exist only in the unported reference |
| D5 | EXTRACT: after Enter, library extracts the result list from the new DOM | N results with title/url/snippet, matching the screenshot | not done |
| D6 | trail: every step went through `/mcp/call` | ledger rows, S13 tool_calls rows, ripple per step; a forced failure (non-allowlisted host) is refused as data with HTTP 200 | partly proven for navigate/screenshot only |
| D7 | consciousness: the run is observable; the failure in D6 yields a finding (needs the failure actor from fork 6) and a reminder-style item awaits clarification once | one finding, no repeats | depends on uncompiled edits |
| D8 | repeat on EU, then local; no proxies, normal setup (as defined) | same results | pending |

Order and honesty rule: D1 and D2 can be proven BEFORE the farm integration
exists (real DOM from the real farm, library run offline). That gives an
early, real, non-mock signal for "domorpher works" the moment D0 passes.
D4-D5 need the host mechanism below.

### 4b. How the farm "loads" the crate — decision needed
Options (the taxonomy in TOOLS_PIPELINES_MCP_GUIDE §1 decides which is right):
1. **A thin compiled binary wrapping the crate, run as an Ozone pipeline**:
   reads `{data:{html|decision}, context}` on stdin, prints the field /
   action sequence as JSON. Fits the pipeline tier exactly (own crate,
   own workspace root, envelope contract, gated by the registry). The farm
   or CC calls it through `/mcp/call` -> pipeline, or the farm bridge script
   shells out to it. Recommended: no new tier, no Python-Rust FFI, testable
   offline.
2. A `cdylib`/PyO3 module imported by the farm's Python: tighter but couples
   the Browser-Farm repo (which we are not to alter) to Rust builds.
3. A serve-mode bridge process (tier 3) the farm talks to over HTTP.
`domorpher-agent` stays unused in all three; the verbs are the vocabulary the
decision data uses.

## 5. Consciousness vs internal/external (the "either, or, both" point)

Consciousness is a windows-first observer and the assistant is a derived
feed. Neither calls tools. A farm action is executed by the CC or by the
orchestrator through `/mcp/call`, whichever "internal" or "external" source
issued it, and each leaves the same trail (ledger, ripple, S13). So the design
already supports both without a second path; the missing pieces are the
failure actor (#3) and attribution (#2). No new agent layer is required.

## 6. Do next (all captured; nothing dropped)

0. Operator decision 4b (how the farm runs the crate); recommended option 1.
1. Operator decision: delete `seams.rs` traits (keep the data types) and
   confirm `domorpher-agent` stays unregistered while unused.
2. Remove the 16 stale `agent_*` rows; correct MASTER_TEST_GUIDE AG rows.
3. Hook auth edit (#1), then run T-FARM-CC on EU.
4. Then #2, #3, #4 in one reviewed edit window.
5. The model-call removal plan in REVIEW_DOMORPHER_LIBRARY_BOUNDARY stands.
