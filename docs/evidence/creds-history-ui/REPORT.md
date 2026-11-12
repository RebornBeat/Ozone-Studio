# Credentials, route protection, history integration, UI — report (2026-11-12)

Commit f7cd424 (parse-only: rustfmt, node --check, ts.transpileModule; nothing compiled or run; EU untouched). Written by the parent from the fork's output. Checklist lines: CHECKLIST_LINES.md (L220–L224). Matrix: docs/SECURITY_CREDENTIAL_MATRIX.md; CALL_ROUTE_REGISTRY R49–R54.

## 1. Credentials and tokens
- Token path already existed (`<data_dir>/.internal_token`, accepted as `x-oz-internal`); every caller traced before protecting anything. `tools/shared/guard.mjs` exports `internalTokenValue()` / `internalHeaders()`; git-mcp restore sends `x-oz-internal`.
- Strict-session PROTECTED list (grpc/mod.rs) gains `/pipelines/unregister`, `/orchestration/`, `/consciousness/review_pass`, `/pipeline/cancel`, `/history/`; `/monitor/activity` protected for non-GET only (UI polls it openly).
- `/orchestrate` on a strict host (OZONE_REQUIRE_SESSION on, setup complete) now refuses a request with no credential: accepts a valid body token, Bearer or `x-oz-session` header, or the internal token; an invalid body token is still refused; non-strict hosts keep the old behaviour.
- Callers fixed: Electron `main.js` (never sent an Authorization header; already failed on strict hosts); `ozone-cli` (chat, capture, order, mcp); `tools/e2e` and `third-party-verify` (Bearer once a session exists; orchestrate()/mcp() get a session first); 24 modality pipelines posting `/orchestration/node-event` send `x-oz-internal` from `OZONE_INTERNAL_TOKEN`. zcode-connector: the integration review's device-key rewrite (b8b3b22) stands.
- Still open: browser plugin has no credential path (needs a pairing flow); `/pipeline/execute` and `/zsei/query` check only a body token.

## 2. History integration
- New protected read routes `/history/status`, `/history/diff` (Myers, `src/history/diff.rs`; too-large diffs reported as whole-region replaced = memory guard, not a content cap), `/history/read` (chunked, no size refusal); `/history/list` + `/history/restore` existed. Off-by-one at `d == max` fixed in both the Rust diff and git-mcp.
- ozone-mcp tools: ozone_history_status, ozone_history, ozone_history_diff, ozone_history_read, ozone_held_calls (committed with the agent-integration commit).
- Container restore now goes through the normal `UpdateContainer` path (pre-write snapshot, keyword gate + keyword-program bookkeeping, cache invalidation, version bump, graph ripple): reversible and re-indexed. Restored: metadata, context, storage pointers, hints. NOT restored: structure, type-specific contexts, integrity data; a deleted container is not re-created (the error says so). File restore rewrites the recorded path; the beacon then records the result.
- Evidence/checklist files: the beacon polls watch roots (`OZONE_HISTORY_WATCH`, default = whichever of `docs`, `CHECKLIST.md`, `AGENTS.md`, `.ozone-context` exist); text-like files recorded as scope `file`, id `p<24 hex>` — history only, no container, no ripple, walk behaviour unchanged.
- Walk context: step context gains a read-only "Recent changes to the attached files" block (newest version, reason, +/- lines vs previous). No walk control flow changed. A linked file's history is keyed by its container id, so the file container is the graph resource; no content duplicated.

## 3. UI
HistoryPanel (versions, diff, restore with typed `RESTORE`) in Files → History for file and container record, plus a History tab (`history-view`, pipeline id 1013); old MCP beacon section removed. HeldCallsPanel (Marketplace → Jurisdiction) and HeldCallsChip (status bar) read new `GET /jurisdiction/held` (age, attempts, next retry, unaccepted requirements, recent held decisions). WorkOrderTree AgentWaiting banner reads new `GET /agent/open` (agent, step, tool, age, "no timeout"; `open_requests()` in agent_requests.rs). Client functions appended to ozoneClient.ts.

## 4. Probe
`tools/third-party-verify/09-creds-history.mjs` (read-only; in run-all.sh; not run): anonymous callers refused on strict hosts; history / held-calls / open-agent shapes.

## Check first at cargo check / UI type check
`orchestrate`: `header_credential_ok` and extractor order; session gate `PROTECTED_MUTATING` method condition; `post_history_*` closures and `base64::Engine` import; `diff.rs` lifetimes in `myers`; `ZSEI::restore_container_version` through `self.query`; `.await` in the stages.rs context block; beacon `spawn_blocking` take-and-return of the cache in `poll_watch`; UI prop types incl. `selectedFile.containerId` possibly null and the tab registration.
Notes: HeldCallsChip and the History tab are not deep-linked; walk-scorecard makes no HTTP calls.
