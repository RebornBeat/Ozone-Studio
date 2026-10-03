# Coordination / Ripple Gaps (R5)

Full findings from the read-only review fork dispatched to deep-dive the
coordination/ripple mechanism — what mirrors into the live graph
correctly today, what doesn't, and independent re-verification of this
session's own MCP jurisdiction+ripple fix. Back to [README](README.md) ·
[master guide](living-graph-field-guide.md).

## This session's own fix, independently re-confirmed

`src/grpc/mod.rs`'s `mcp_call` handler and `src/orchestrator/jurisdiction.rs`'s
newly-`pub(crate)` functions were re-checked directly against source, not
taken on the strength of the coordinator's own self-report. Confirmed
genuinely real and correct:

- Real adapters constructed in the handler (`RegistryExecutorAdapter` →
  `DecisionReviewExecutor` → `executor_adapter`; `ZseiStoreAdapter` →
  `zsei_adapter`) — the exact same pattern `AppRuntime::orchestrate()`
  uses (`src/lib.rs:1234`), not a shortcut duplicate.
- A real `PromptOrchestrator` instance is built from those adapters, and
  `load_jurisdiction_rules` / `categorize_jurisdiction_matches` /
  `resolve_confirmation_reviews` are the real functions, genuinely called —
  not stubs.
- `context_mirror::mirror(kind: "tool_call")` is a real call into the same
  `mirror()` function already used for note/decision/handoff/finding/claim
  kinds, and it genuinely goes through `ZSEI::query`'s `CreateContainer`
  choke point — so it genuinely ripples, not a fabricated claim.

**Update, 2026-09-29 — the honest limitation above is now closed (ZCode).**
`McpCall` gained a real `session_token` field; `mcp_call` validates it
against the same `AuthSystem` `/orchestrate` uses, and the resulting
`identity_validated` flag now travels with both the insight envelope and
the S13 `tool_calls.jsonl` row. Per-call identity is real provenance now,
not a hardcoded zero. (`blueprint_id: 0` for MCP calls remains — MCP calls
genuinely have no blueprint context, which is a real absence, not a gap to
close.)

## What's confirmed fine as-is (not a gap)

- `fileClaim` / `fileRelease` / `noteAdd` in
  `tools/ozone-shared-context/server.js` all correctly call
  `mirrorContext`. The earlier `file_release` mirror fix (this session) is
  confirmed still in place at `server.js:297`.
- `presenceHeartbeat` (`server.js:219`) never mirrors — but this looks
  intentional, not a gap: mirroring every heartbeat would flood the graph.
  This is exactly why a dedicated `state.json` read route exists instead
  (referred to elsewhere as "B10"), specifically to serve presence data
  without needing every heartbeat to ripple.

## Update, 2026-09-29 — gap #1 CLOSED by ZCode, verified

**Task lifecycle ripple is now live.** ZCode added `emit_task_ripple()` in
`src/task/mod.rs` (binary 00:09, restart-verified): real scoped graph
events (`ws:`/`proj:`/`global` vocabulary) now fire on task creation
(`enqueue_task` → "created"), completion (`complete_task` → "updated"),
failure (`fail_task` → "updated"), and every coordination status
transition (`update_coordination_status` → "updated" — paused/
interrupted/queued/running all move the graph now). This was the single
highest-priority item in this document; it's done. Consumers — WS
subscribers, the monitor feed, AMT ripple-sync (a completed task can now
wake its project's AMT), the I1 coordination feed, `/order/global` — all
light up automatically since they already read the same event vocabulary.

Not re-derived from ZCode's own note alone — independently worth noting
this closes part of R1's H5/H6 finding for *live viewing* (a status change
is now observable as it happens), though it does **not** persist
jurisdiction/simulation results onto the durable task record itself —
that's still a separate, unresolved gap (see
[living-graph-field-guide.md](living-graph-field-guide.md#master-punch-list)
item 18).

## Update, 2026-09-29 — gap #1 (`/config/set`) also CLOSED, verified live

`set_config` now fires a real `context_mirror::mirror(kind:"config_change")`
on every successful save, claiming only the real top-level section names
present in the request (`sections_touched`, derived directly from
`req.updates`'s own keys — never a fabricated per-field diff). Live-verified
end to end, not just built: sent a real `/config/set` call while watching
`/ws`, got back a real `CoordinationEvent` container (id 40376) with
`scope_keywords:["config_change","host-config","scope:global"]`. "When did
this host's config last change, and to what" is now a real, answerable
graph query.

## Remaining gap: `taskCreate` in `tools/ozone-shared-context/server.js:467`

Re-checked directly: still posts to `/task/create` and calls
`pushActivity` only — no `mirrorContext` call. Now that the Rust-side task
lifecycle genuinely ripples (above), this is a much cheaper fix than
originally scoped: it doesn't need new plumbing, just a `mirrorContext`
call added alongside the existing `pushActivity`, matching the pattern
`fileClaim`/`fileRelease`/`noteAdd` already use in the same file.

## Bonus finding: an earlier bug is already fixed, credit elsewhere

The `persist_insight` untyped-extra-JSON-keys bug (found earlier this
session — insight container 40194's `content`/`citations` fields were
silently dropped by typed deserialization) is **already fixed** as of this
check: `src/consciousness/review.rs:273-301` now uses a real
content-pointer convention, matching the AMT/modality-graph pattern used
elsewhere. This fix was not made by this session's own work — worth
confirming with ZCode (the coordination agent) whose fix this was, since
it landed independently and in parallel.

## What's left

Just `taskCreate` in `server.js` — a small, mechanical add (`mirrorContext`
alongside the existing `pushActivity`), matching the pattern
`fileClaim`/`fileRelease`/`noteAdd` already use in the same file.

See also [zcode-state-and-remaining-work.md](zcode-state-and-remaining-work.md)
for the full current backend picture (modality revivals, MCP tool
ecosystem, auth) beyond just ripple coverage.
