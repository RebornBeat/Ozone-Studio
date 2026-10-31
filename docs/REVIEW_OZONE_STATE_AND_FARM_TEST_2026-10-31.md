# Review — Ozone-Studio state, tool/MCP guide conformance, and the Browser-Farm test (2026-10-31)

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

## 4. The test (proposed, runs on the VPS first, then local)

**T-FARM-CC — a CC-driven farm round trip observed by Ozone.** Real
substrate only; no mock farm.

1. Preconditions: EU farm status healthy, command-center profile running,
   `google.com` and `example.com` on the farm nav allowlist, strict session
   on, ingest URL set in the farm-mcp unit.
2. CC issues `farm_command` ping, expects pong < 2 s (baseline already 0.4 s).
3. CC issues `farm_navigate` to an allowlisted host; expects `success:true`
   and a screenshot file under the host data dir.
4. Negative: navigate to a non-allowlisted host; expects `success:false`,
   HTTP 200, a reason, and a ledger row. (Locked-by-default proof.)
5. Ingest: the screenshot is ingested (hook once #1 lands, manual call
   until then); expects analysis output and a graph container.
6. Observation: within one consciousness cycle a tool_calls row (S13) exists
   for steps 2-4; the failed step 4 is visible as data. Today it will NOT
   produce a finding (#3); record that as the expected-to-fail row and keep it
   red until the actor exists.
7. Scope: the call row carries the workspace/project that CC passed. Expected
   to fail today (#2).
8. Notification path: a reminder-style item reaches `/assistant/feed` and
   stays awaiting clarification; resolving it as Acknowledge sets
   `acknowledged` once and does not reappear on the next check-up (R2).
9. Persistence: restart the host; the farm tool set and endpoints are
   unchanged (P8).

Pass bar: steps 2-5 and 9 must pass today. Steps 6-8 are listed with their
known current result so a red is information, not surprise.

## 5. Consciousness vs internal/external (the "either, or, both" point)

Consciousness is a windows-first observer and the assistant is a derived
feed. Neither calls tools. A farm action is executed by the CC or by the
orchestrator through `/mcp/call`, whichever "internal" or "external" source
issued it, and each leaves the same trail (ledger, ripple, S13). So the design
already supports both without a second path; the missing pieces are the
failure actor (#3) and attribution (#2). No new agent layer is required.

## 6. Do next (all captured; nothing dropped)

1. Operator decision: delete `seams.rs` traits (keep the data types) and
   confirm `domorpher-agent` stays unregistered while unused.
2. Remove the 16 stale `agent_*` rows; correct MASTER_TEST_GUIDE AG rows.
3. Hook auth edit (#1), then run T-FARM-CC on EU.
4. Then #2, #3, #4 in one reviewed edit window.
5. The model-call removal plan in REVIEW_DOMORPHER_LIBRARY_BOUNDARY stands.
