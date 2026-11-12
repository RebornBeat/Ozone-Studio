# ZCode + Claude Code integration review — report (2026-11-12)

Commit b8b3b22 (parse-only: node --check, JSON parse; nothing run; EU untouched). Written by the parent from the fork's output. Setup doc: docs/AGENT_INTEGRATION_ZCODE_CC.md. Checklist lines: CHECKLIST_LINES.md (L260–L262).

## Outdated / broken, and fixed
- `.zcode/config.json` had only ozone-shared-context: ZCode could coordinate but not prompt Ozone, follow walks or offer itself → ozone-mcp added (OZONE_AGENT_NAME=zcode).
- Committed `.claude/settings.json` enabled only shared-context (ozone-mcp only in local settings) → both enabled.
- Both configs relied on the working directory for the context dir and link roots → OZONE_CONTEXT_DIR and OZONE_MCP_LINK_ROOTS set explicitly in `.mcp.json` and `.zcode/config.json`.
- `connect.js` sent no credentials; `/pipelines/register` is session-protected (strict default ON) → device-key auth, one re-auth on 401.
- `connect.js` forwarded model calls to ZCode's provider (Z.ai) with no jurisdiction check (host hop gate sees localhost = local) → asks `/jurisdiction/outbound/check` first; held/unreachable = HTTP 503, not forwarded.
- Nothing told ZCode a walk step awaited its report → `connect.js watch` polls `/agent/requests` and posts a monitor-feed warning; new `requests`, `whoami`.
- Shared-context device key written 0664 → 0600 (existing key chmod'ed).
- `file_claims` showed silent agents' claims as live → rows carry `holder_live` and `stale`.
- README/AGENTS.md called `.ozone-context/` git-visible; it is git-ignored → corrected; AGENTS.md gains an ozone-mcp section.
- No per-agent key helper → `ozone-mcp --init-key` (prints the public key only).
Also: version strings aligned, connector README rewritten, check P5 (claims carry holder_live) added to `m1-coordination.mjs`.

Claude Code wiring was already current (both servers in `.mcp.json`, bound to `claude-code`).
ZCode's last handoff: 2026-11-08 18:49 (takeover after CC's weekly limit; captured keyword-program work, wave 2, tests). Operator-gated queue it left: push, key rotation, flash-1 floor, EU measurement run, encoder weights, stale-task cleanup. Silent since; its connector/config were last touched in September.

## What would block each agent
1. Prompting: no ozone-mcp entry (fixed), no device key / unreachable host, OZONE_MCP_READ_ONLY=1, files outside link roots or refused by the link guard, first-boot jurisdiction hold until the three agreements are accepted.
2. Joining walks: same context dir for all agents, valid agent names, new host build (EU build8 lacks /walk/* and /agent/*).
3. Offering as agent provider: a live session must answer (a session is not a daemon); an unlisted agent is `third_country_unverified`, so the step is held — the operator must add `agent:claude-code` and `agent:zcode` to policy `providers` and accept the agreements (not pre-listed; no regions invented).
4. Shared context: different context dirs, or a strict host without the internal token. Stale claims never block.

## Stale claims
Four `claude-code-fork-sensor` claims (~614 min, silent holder) were dropped by the shared-context tool's own expiry at the first heartbeat; none of those files had uncommitted changes; sensor raw-input stays deferred.

## Findings NOT fixed
- **Auto-enrolment (security):** `src/auth/mod.rs authenticate` creates a user + device for ANY unknown ed25519 key on first use. The strict-session gate therefore proves "a key signed", not "the operator approved". A per-agent key = a different host user. Recommend the shared key for operator-owned agents until the host has key approval.
- `handleModelCall()` stays unfilled (ZCode is a desktop app, no headless CLI as of 2026-09-06; MCP is the supported route).
- "MCP as a model" generic chain entry not built; agent providers are the only form.
- `/pipelines/unregister` and `/monitor/activity` (POST) still outside the strict-session prefixes (connect.js authenticates every call, so it works either way). The credentials fork may have changed this.
