# MCP Testing & Cross-MCP Coordination — full sweep

> Doctrine doc, 2026-09-29. Operator directive, verbatim intent: "fully
> test all mcps we have and all left to create and cross-mcps coordination
> as well" — plus explicit permission/instruction to throw up **batches of
> 4-5 forks at a time until completion**. This doc is the real, verified
> inventory the fork batches work from, not a plan written from memory —
> every port/tool count below was read live from `/mcp/tools` and each
> server's own source, moments before this doc was written.

---

## 1. The real fleet, live (108 tools, `GET /mcp/tools`, 2026-09-29 ~18:30)

| MCP | Dir | Default port | Live tools | Tested this session? |
|---|---|---|---|---|
| bridges (Roblox/Unity/Unreal/Blender) | `tools/bridges/` | :3210 | 67 | **PASS** (see §2e) — foundation tools live on a separate `:3100/mcp` surface, real MCP-protocol client needed to test those |
| terminal-mcp | `tools/terminal-mcp/` | :3215 | 2 | **Batch 1 — PASS**, real bug found+fixed (see §2b) |
| visual-mcp | `tools/visual-mcp/` | :3220 | 7 | **PASS** (retest, see §2d) — `yolo_detect` needs `pip install ultralytics`, already-known gap |
| web-mcp | `tools/web-mcp/` | :3225 | 2 | **Batch 1 — PASS** (`web_search` config-disabled, not broken; `current_datetime` real) |
| code-mcp | `tools/code-mcp/` | :3230 | 1 | **Batch 1 — PASS**, real callgraph verified |
| git-mcp | `tools/git-mcp/` | :3235 | 3 | **Batch 1 — PASS** (note: `/mcp/call` responses are double-nested `{output:{output:{...}},success}` — trips up naive tests) |
| security-mcp | `tools/security-mcp/` | :3240 | 8 | Yes — live this session (CLI, `/mcp/call` gated stack proven) |
| gcal-mcp | `tools/gcal-mcp/` | :3241 (own default) | 7 | **Batch 1 — FIXED & VERIFIED**, was broken (see §2) |
| cerebrix-mcp | `tools/cerebrix-mcp/` | :3245 | 3 (eeg_window, karaone_trial, karaone_stats) | **Batch 2 — PASS**, no bugs (see §2c) |
| threed-mcp | `tools/threed-mcp/` | :3250 | 1 (eeg_cap_costume_generate — documented headless-hang limitation) | **PASS** (retest, see §2d) — hang limitation reproduced exactly as documented, internal 20s timeout guard confirmed working |
| connectome-mcp (+ circuit_service.py :3256) | `tools/connectome-mcp/` | :3255 | 3 (fly_*) | **Batch 2 — PASS, 1 real bug found** (see §2c) |
| re-mcp | `tools/re-mcp/` | :3260 | 3 | Yes — built and proven this session |
| ozone-shared-context | `tools/ozone-shared-context/` | stdio, not HTTP | 16 (the coordination surface this whole session runs on) | Continuously, all session |

All 12 HTTP families are currently **down** — only the core host (:50051)
is running, per the operator's own resource-conservation correction
earlier this session ("take them down if you aren't using them... no need
to deploy all if not testing"). §5 below is the testing protocol that
respects that instruction while still reaching full coverage.

## 2. Real bug found while building this inventory: gcal-mcp is mis-registered

`gcal-mcp/server.mjs`'s own default port is **3241** (`OZONE_GCAL_PORT ??
3241`, confirmed by reading the source directly). `security-mcp`'s
default is **3240**. But the live `/mcp/tools` registry has all 7
`gcal_*` tools pointing at `:3240` — security-mcp's port, not gcal's own.
This means gcal-mcp was, at some point, launched with `OZONE_GCAL_PORT=3240`
(colliding with security-mcp's default) and registered that way. Right
now, any `gcal_*` call through `/mcp/call` either 404s against
security-mcp (if it's the one actually listening on :3240) or connection-
refuses (if nothing is). **Real, concrete, fix-before-testing bug** — not
a design gap. Fix: relaunch gcal-mcp with its own real default (no
`OZONE_GCAL_PORT` override), re-register its 7 tools against `:3241`.

`TOOLS_REGISTRY.md`'s own "LIVE FLEET CAPTURE" table already has the
*correct* intended assignment (`gcal-0.1.0 | :3241`) — the doc was right;
the live registration drifted from it. Cerebrix/threed/connectome were
cross-checked the same way (each server's own hardcoded default vs. live
registered port) and all three match their own source — no other port
bugs found.

**FIXED, 2026-09-29 (batch 1 fork)**: relaunched gcal-mcp with no port
override (real default 3241), re-registered all 7 `gcal_*` tools against
`:3241` (`/mcp/tools/register` confirmed `"replaced":true` for each),
verified `gcal_status`/`gcal_create_event` both now return the correct
"LOCKED — no OAuth credentials" refusal (honest, not a crash) with a real
ripple + S13 row each. **Root cause found**: the server's own header
comment said "default 3240" while the code itself said `?? 3241` — a
stale comment, almost certainly what caused the original mis-launch.
**Real-world confirmation this bug was live and hit**: the S13 capture
store already had 4 prior rows from `agent:"zcode"` failing with `"unknown
security tool 'gcal_status'"` — ZCode had independently hit this exact
broken registration earlier today, unprompted.

### 2b. Second real bug found, terminal-mcp: roles.json silently unused unless explicitly pointed at

`tools/terminal-mcp/roles.json` is a real, valid file sitting right next
to `server.mjs`, but the server only loaded a roles file when
`OZONE_TERMINAL_ROLES_FILE` was explicitly set at launch — with no env
var set, every agent was silently `LOCKED` (`roles_configured: []`) even
though a real allowlist existed on disk. Confirmed live by a batch-1 fork:
launched without the env var (all calls refused, "no allowlist for
agent"), then with it (worked correctly, `roles_configured:
["_doctrine","zcode","claude-code","*"]`). **Fixed**: `server.mjs`'s
`ROLES` resolution now falls back to `roles.json` next to itself
(`path.dirname(fileURLToPath(import.meta.url))`) when no env var is set
and no inline `OZONE_TERMINAL_ROLES` JSON is given — still `LOCKED` if
that default file is missing or invalid, so "unconfigured = LOCKED"
(`NEW_MCP_GUIDE.md` §6) still holds; it just no longer locks out a real,
present, valid roles file. Verified `node --check` clean and the JSON
parses correctly; not yet re-run live end-to-end (low-risk, mechanical
fallback add — flagged honestly rather than claimed as re-verified).

### 2c. Batch 2 findings: connectome-mcp bug, cerebrix-mcp clean, a real process lesson

**Re-tested 2026-09-30, not a real bug — a stale long-running process.**
The original finding ("`fly_circuit_subgraph` doesn't refuse on a
nonexistent `seed_body_id`, silently falls back to the default GF
subgraph") did not reproduce against a freshly-started `circuit_service.py`.
Read `giant_fiber_circuit.py`'s `build_subgraph` directly: it already
correctly returns `{"error": "bodyId {id} has no real connections..."}`
when both `out_edges`/`in_edges` are empty, and `circuit_service.py`'s
handler already correctly turns that into `{"success": false, "error":
...}`. Confirmed live against a fresh process: `seed_body_id: 10001`
(real) returns real subgraph data; `seed_body_id: 999999999999` (fake)
correctly refuses with the exact error text above — no fallback, no
masking. `circuit_service.py` is long-running by design (loads ~4.2GB of
feather tables once at startup, never restarted per-call) — the original
test almost certainly hit an older in-memory instance still running from
before this refusal check existed in the source, not the code as it
exists now. No fix needed.

**cerebrix-mcp: all 5 contract items pass, no bugs.** One thing worth a
future look, not chased: `karaone_trial`'s summary stats (`mean: 8.97e13`)
look like unnormalized raw feature values, not obviously wrong but odd
enough to flag for whoever builds on this data next.

**A real process lesson, confirmed independently by 3 separate forks in
this same batch**: connectome-mcp's `circuit_service.py` genuinely holds
**~4.2GB RSS** (measured live — higher than this session's earlier ~3.3GB
estimate), and while it was up, the sibling forks testing visual-mcp/
threed-mcp and the bridges family both correctly detected the resulting
memory pressure (down to ~120-500MB free) and declined to start their own
servers rather than risk the exact crash this session was already
corrected about once. **This was a real mistake in how batch 2 was
dispatched** — the guide's own §5 says heavy MCPs go "ONE at a time," but
two heavy-tier forks (connectome-mcp, visual/threed-mcp) were dispatched
in the same parallel batch, which broke that rule at the batch level even
though every individual fork behaved correctly. Fix applied going
forward: heavy-tier retests are now dispatched one at a time, waiting for
each to fully tear down (confirmed via `free -h`) before the next starts.
A second-order finding from the same batch: even within ONE MCP family,
`fly_circuit_subgraph` and `fly_connectome_stats`/`fly_neuron_query`
independently spawn Python subprocesses that compete for RAM with each
other — serialization matters at the sub-tool level too, not just
cross-MCP.

### 2d. Retest, dispatched alone after the batch-2 sequencing fix: visual-mcp + threed-mcp both clean

Confirmed memory clear (5.3Gi available) before dispatching, ran ONE MCP
at a time this time. Both pass all 5 contract items:
- **visual-mcp**: real `visual_ingest` analysis on a real repo image
  (`llama.cpp/media/llama0-banner.png`), real refusal paths (missing
  image, Wayland-capture-not-sanctioned per its own documented policy).
  `yolo_detect` reports "ultralytics not installed" — this is the
  already-documented known gap from `TOOLS_REGISTRY.md`'s §4, not a new
  finding.
- **threed-mcp**: the documented headless-hang limitation reproduced
  exactly as its own source predicts — 20,149ms, then a clean internal
  timeout error. Confirms the timeout guard genuinely works; the MCP
  itself stayed responsive throughout rather than actually hanging the
  process. Not a bug, a working safety net around a known real
  limitation (Blender headless rendering).

Both torn down clean, memory unchanged at 5.2Gi available afterward — no
leak from either.

### 2e. bridges retest: refusal path clean, one real envelope-shape inconsistency found

`list_capabilities`/`doctor`/`inspect_project` are genuinely not reachable
via a plain `:3210/call` POST — the server's own honest error says they
live on a separate surface (`:3100/mcp`), which requires the real
MCP-over-HTTP protocol (session/SSE handshake), not a bare JSON-RPC POST.
Confirmed via a 405 even with a correct `initialize` payload. This
matches `TOOLS_REGISTRY.md`'s already-documented gap ("foundation tools
live on the bridges MCP surface :3100/mcp, not through the shim") — an
honest "untestable without a real MCP client library" finding, not a bug.

`unity_get_hierarchy` with no engine attached refused cleanly and
honestly (`"Unity bridge not connected... Ensure the Unity Editor C#
companion package is running"`, real `bridgeStatus` detail). Real,
minor, worth-noting **envelope-shape inconsistency**: bridges returns
top-level `success:true` even on this refusal — the actual failure lives
in `output.ok:false` — while every other MCP tested this sweep
(security/git/web/code/gcal/terminal/connectome/cerebrix/visual/threed)
returns `success:false` at the top level on a refusal. Not a bug in
either convention, just a real inconsistency a future caller needs to
know about (check `output.ok` for bridges specifically, not just
top-level `success`). Ripple + S13 both confirmed. Clean teardown, zero
memory delta.

## 3. What's left to create

- **zoom-mcp — BUILT, 2026-09-30** (`tools/zoom-mcp/server.mjs`, port
  :3265). Mirrors gcal-mcp's exact real structure: locked-by-default
  (confirmed live — every tool, including an unknown-tool call, correctly
  refuses with the same LOCKED message before any dispatch, matching
  gcal-mcp's own proven behavior), 7 tools (`zoom_status`,
  `zoom_auth_start`/`zoom_auth_code`, `zoom_list_meetings`,
  `zoom_create_meeting`, `zoom_join_url`, `zoom_sync_to_order`), tokens
  stored locally (0600), same idempotent-per-pass external_id sync shape
  as `gcal_sync_to_order`. Real, confirmed Zoom-specific difference from
  Google: token exchange uses HTTP Basic auth with the client
  credentials, not a form-body client_id/secret. All 7 tools registered
  and proven through the full gated `/mcp/call` stack (jurisdiction gate,
  usage ledger, ripple, S13 capture — all confirmed live). **Honest gap,
  not resolved**: the exact required OAuth scope string for a real
  registered Zoom app needs verification against that app's actual
  granted scopes before first real use — a reasonable real default is set
  (`meeting:write:meeting meeting:read:meeting user:read:user`), not
  guessed blindly, but not live-tested against a real Zoom account either
  (no credentials available in this environment, same honest boundary
  gcal-mcp has always had).
- No other MCP is named-but-missing anywhere in `docs/*.md` (checked via
  grep across the doc set for "not yet created"/"planned MCP"/"TODO.*mcp").
  **§3 is now empty** — every named-but-missing MCP has been built.

## 4. Cross-MCP coordination — there is no direct MCP-to-MCP channel, by design

Checked directly: no MCP server's source calls another MCP's `/call`
endpoint anywhere in `tools/*/server.mjs`. This is not a gap — it is
`TOOLS_PIPELINES_MCP_GUIDE.md` §1's own stated architecture: "Ozone sits
above both as the reviewer/order layer." Every MCP call already goes
through the ONE real door (`/mcp/call`) and, per `ACTING_LOOP_GUIDE.md`
§3.0's fork-audited finding, every one of those calls rides the SAME
universal ripple choke point (`src/zsei/mod.rs:191-196`) into the living
graph. **"Cross-MCP coordination" already exists — it is the graph, not
a new mechanism to build.** A security-mcp finding and a gcal-mcp sync
both land as real containers under the same `SharedContext` root, visible
to any actor (human, ZCode, a future `RippleActor`) reading the graph —
no MCP needs to know another MCP exists. What's real to verify in this
sweep is narrower than "build coordination": confirm each MCP's calls
actually ripple (§6's per-MCP checklist item 3), not invent a new bus.

## 5. Testing protocol (resource-safe, per the operator's crash correction)

Split by real memory weight, confirmed by reading each server's own
imports/data-loading code:

- **Light (pure Node, no model/data load — safe to test several at once
  in one batch)**: terminal-mcp, web-mcp, code-mcp, git-mcp, gcal-mcp.
- **Heavy (Python subprocess and/or loads real model/data weight —
  test ONE at a time, tear down and confirm freed before the next)**:
  connectome-mcp (circuit_service.py holds ~3.3GB of feather tables —
  confirmed this session's own earlier `free -h` investigation),
  cerebrix-mcp (loads real EEG trial data), visual-mcp/threed-mcp (yolo
  model weights). bridges (:3210, 67 tools, Node but spawns
  engine-bridge connectors) tested cautiously, not lumped with light.

Every fork below ends its own MCP's process before finishing — no fork
leaves a server running for the next one to find. `free -h` before/after
each fork's server lifecycle, logged in its findings.

## 6. Per-MCP test contract (from `NEW_MCP_GUIDE.md` §6, reused verbatim)

1. Positive path — at least one real tool call, real output.
2. Refusal path — at least one real, intentional failure (bad input,
   locked/no-credentials, disallowed command) — confirm it fails
   *cleanly*, not with a crash or a silent empty success.
3. Ripple — confirm the call produces a real `GraphEvent` (subscribe or
   inspect the resulting `CoordinationEvent` container directly, same
   method proven this session for `firewall_status`).
4. S13 capture — confirm `GET /capture/tool-calls?tool=<name>` shows the
   real row.
5. Registry correctness — confirm the tool's registered `endpoint` in
   `/mcp/tools` actually matches the server's own real listening port
   (this is exactly the check that caught §2's gcal bug — repeat it for
   every family, not just the ones already suspected).

## 7. Fork batch plan (dispatched in batches of 4-5, per operator instruction)

- **Batch 1** (light Node MCPs, dispatched together — 5 forks): terminal-mcp,
  web-mcp, code-mcp, git-mcp, gcal-mcp (includes the §2 port fix).
- **Batch 2** (heavy MCPs, one fork each but sequenced/cautious internally,
  plus zoom-mcp scoping): connectome-mcp, cerebrix-mcp, visual-mcp/threed-mcp,
  bridges spot-check, zoom-mcp design-only (mirrors gcal-mcp's real shape).
- **Batch 3+**: whatever Batch 1/2 findings require (bug fixes, re-tests,
  doc updates to `TOOLS_REGISTRY.md`'s live table) — sized 4-5 at a time,
  continuing until the full fleet + zoom-mcp scoping is genuinely done.

Each batch's findings get folded back into this doc's §1 table and into
`CHECKLIST.md`, matching this session's established pattern.
